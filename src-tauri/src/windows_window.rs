//! Native operations on this app's HWNDs, called only on Tauri's main thread.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::WebviewWindow;

const GWL_EXSTYLE: i32 = -20;
const WS_EX_LAYERED: i32 = 0x0008_0000;
const WS_EX_NOACTIVATE: i32 = 0x0800_0000;
const LWA_ALPHA: u32 = 2;
const WM_SIZING: u32 = 0x0214;
const WM_NCDESTROY: u32 = 0x0082;
const RESIZE_OBSERVER: usize = 1;

#[derive(Default)]
struct ResizeObserver {
    resized: AtomicBool,
    // Capture notes use client coordinates; ordinary notes retain outer anchors.
    preserve_client_rect: AtomicBool,
    // Zero disables the image constraint. Access never locks application state.
    ratio: AtomicU64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}

// Correct the proposed outer rectangle before Windows applies it. The image
// fills the client area, so invisible resize borders must not enter the ratio.
fn constrain_sizing_rect(
    rect: &mut Rect,
    edge: usize,
    client: (f64, f64),
    frame: (f64, f64),
    ratio: f64,
    minimum: f64,
) -> bool {
    if !(1..=8).contains(&edge) || !ratio.is_finite() || ratio <= 0.0 {
        return false;
    }
    let width = (f64::from(rect.right) - f64::from(rect.left) - frame.0).max(minimum);
    let height = (f64::from(rect.bottom) - f64::from(rect.top) - frame.1).max(minimum);
    let width_drives = match edge {
        1 | 2 => true,  // left/right
        3 | 6 => false, // top/bottom
        // Corners follow the axis with the larger proportional user movement.
        _ => (width - client.0).abs() * client.1 >= (height - client.1).abs() * client.0,
    };
    let (width, height) = if width_drives {
        let width = width.max(minimum * ratio);
        (width, (width / ratio).round())
    } else {
        let height = height.max(minimum / ratio);
        ((height * ratio).round(), height)
    };
    let (width, height) = ((width + frame.0).round(), (height + frame.1).round());
    if !width.is_finite()
        || !height.is_finite()
        || width > f64::from(i32::MAX)
        || height > f64::from(i32::MAX)
    {
        return false;
    }
    let (width, height) = (width as i32, height as i32);
    // Keep the opposite edges anchored for all eight native drag directions.
    if matches!(edge, 1 | 4 | 7) {
        rect.left = rect.right.saturating_sub(width);
    } else {
        rect.right = rect.left.saturating_add(width);
    }
    if matches!(edge, 3..=5) {
        rect.top = rect.bottom.saturating_sub(height);
    } else {
        rect.bottom = rect.top.saturating_add(height);
    }
    true
}

type SubclassProc = unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(
        window: isize,
        callback: Option<SubclassProc>,
        id: usize,
        data: usize,
    ) -> i32;
    fn GetWindowSubclass(
        window: isize,
        callback: Option<SubclassProc>,
        id: usize,
        data: *mut usize,
    ) -> i32;
    fn RemoveWindowSubclass(window: isize, callback: Option<SubclassProc>, id: usize) -> i32;
    fn DefSubclassProc(window: isize, message: u32, wparam: usize, lparam: isize) -> isize;
}

unsafe extern "system" fn observe_resize(
    window: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    id: usize,
    data: usize,
) -> isize {
    if message == WM_SIZING {
        // WM_SIZING is user-only; WM_SIZE also fires for fit/fold/placement.
        // Never call Tauri or lock app state from this reentrant native callback.
        // SAFETY: installation transfers this Box to the HWND until WM_NCDESTROY.
        let ratio = unsafe {
            let observer = &*(data as *const ResizeObserver);
            observer.resized.store(true, Ordering::Release);
            f64::from_bits(observer.ratio.load(Ordering::Acquire))
        };
        // No reference to observer survives the next handler, which may destroy
        // the HWND. Only native geometry queries and the caller's RECT follow.
        let result = unsafe { DefSubclassProc(window, message, wparam, lparam) };
        if ratio > 0.0 && lparam != 0 {
            let (mut outer, mut client) = (Rect::default(), Rect::default());
            if unsafe { GetWindowRect(window, &mut outer) } != 0
                && unsafe { GetClientRect(window, &mut client) } != 0
            {
                let width = f64::from(client.right) - f64::from(client.left);
                let height = f64::from(client.bottom) - f64::from(client.top);
                let frame = (
                    (f64::from(outer.right) - f64::from(outer.left) - width).max(0.0),
                    (f64::from(outer.bottom) - f64::from(outer.top) - height).max(0.0),
                );
                let dpi = unsafe { GetDpiForWindow(window) };
                let dpi = if dpi == 0 { 96 } else { dpi };
                // Image notes have a 2-DIP client minimum on both axes.
                let minimum = (2.0 * f64::from(dpi) / 96.0).ceil();
                // SAFETY: WM_SIZING supplies a live writable RECT until return.
                if constrain_sizing_rect(
                    unsafe { &mut *(lparam as *mut Rect) },
                    wparam,
                    (width, height),
                    frame,
                    ratio,
                    minimum,
                ) {
                    return 1; // TRUE: the proposed drag rectangle was processed.
                }
            }
        }
        return result;
    } else if message == WM_NCDESTROY {
        // Remove before releasing reference data. Do not borrow it across the
        // next handler: DefSubclassProc can synchronously reenter window code.
        unsafe {
            RemoveWindowSubclass(window, Some(observe_resize), id);
            drop(Box::from_raw(data as *mut ResizeObserver));
        }
    }
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

fn resize_observer(window: isize) -> Option<usize> {
    let mut data = 0;
    // All callers run on the HWND owner thread; its data cannot be destroyed
    // between this lookup and the immediately following atomic operation.
    let found =
        unsafe { GetWindowSubclass(window, Some(observe_resize), RESIZE_OBSERVER, &mut data) };
    (found != 0).then_some(data)
}

fn install_resize_observer_for_handle(window: isize) -> Result<(), String> {
    if resize_observer(window).is_some() {
        return Ok(());
    }
    let data = Box::into_raw(Box::new(ResizeObserver::default()));
    // SAFETY: same-thread installation; successful registration owns data until
    // WM_NCDESTROY, and failed registration returns ownership immediately.
    if unsafe { SetWindowSubclass(window, Some(observe_resize), RESIZE_OBSERVER, data as usize) }
        == 0
    {
        unsafe { drop(Box::from_raw(data)) };
        return Err("네이티브 크기 조절 감시 등록 실패".into());
    }
    Ok(())
}

pub fn install_resize_observer(window: &WebviewWindow) -> Result<(), String> {
    install_resize_observer_for_handle(window.hwnd().map_err(|e| e.to_string())?.0 as isize)
}

fn take_native_resize_for_handle(window: isize) -> bool {
    resize_observer(window).is_some_and(|data| {
        // Keep the signal until consumed even after the native drag loop exits;
        // Tauri can deliver the final Resized event after WM_EXITSIZEMOVE.
        unsafe {
            (&*(data as *const ResizeObserver))
                .resized
                .swap(false, Ordering::AcqRel)
        }
    })
}

fn set_image_ratio_for_handle(
    window: isize,
    size: Option<tauri::LogicalSize<f64>>,
) -> Result<(), String> {
    let ratio = match size {
        Some(size)
            if size.width.is_finite()
                && size.height.is_finite()
                && size.width > 0.0
                && size.height > 0.0 =>
        {
            size.width / size.height
        }
        Some(_) => return Err("유효하지 않은 이미지 비율입니다".into()),
        None => 0.0,
    };
    if !ratio.is_finite() || (size.is_some() && ratio == 0.0) {
        return Err("유효하지 않은 이미지 비율입니다".into());
    }
    install_resize_observer_for_handle(window)?;
    let data = resize_observer(window).ok_or("크기 조절 감시를 찾을 수 없습니다")?;
    // SAFETY: same-thread HWND data remains live for this atomic store.
    unsafe {
        (&*(data as *const ResizeObserver))
            .ratio
            .store(ratio.to_bits(), Ordering::Release)
    };
    Ok(())
}

pub fn set_image_ratio(
    window: &WebviewWindow,
    size: Option<tauri::LogicalSize<f64>>,
) -> Result<(), String> {
    set_image_ratio_for_handle(window.hwnd().map_err(|e| e.to_string())?.0 as isize, size)
}

pub fn take_native_resize(window: &WebviewWindow) -> Result<bool, String> {
    Ok(take_native_resize_for_handle(
        window.hwnd().map_err(|e| e.to_string())?.0 as isize,
    ))
}

#[link(name = "user32")]
extern "system" {
    fn GetWindowRect(window: isize, rect: *mut Rect) -> i32;
    fn GetClientRect(window: isize, rect: *mut Rect) -> i32;
    fn ClientToScreen(window: isize, point: *mut Point) -> i32;
    fn GetDpiForWindow(window: isize) -> u32;
    fn SetWindowPos(
        window: isize,
        after: isize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn GetWindowLongW(window: isize, index: i32) -> i32;
    fn SetWindowLongW(window: isize, index: i32, value: i32) -> i32;
    fn SetLayeredWindowAttributes(window: isize, key: u32, alpha: u8, flags: u32) -> i32;
    fn GetLayeredWindowAttributes(
        window: isize,
        key: *mut u32,
        alpha: *mut u8,
        flags: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn SetLastError(error: u32);
}

fn extended_style(handle: isize) -> Result<i32, String> {
    // GWL_EXSTYLE is a 32-bit value on both x86 and x64. Zero is valid.
    unsafe {
        SetLastError(0);
        let style = GetWindowLongW(handle, GWL_EXSTYLE);
        let error = std::io::Error::last_os_error();
        if style == 0 && error.raw_os_error() != Some(0) {
            return Err(format!("창 스타일 조회 실패: {error}"));
        }
        Ok(style)
    }
}

pub fn show_without_focus(window: &WebviewWindow) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let desired_client = client_size_for_handle(handle)?;
    let client_origin = if resize_observer(handle).is_some_and(|data| unsafe {
        (&*(data as *const ResizeObserver))
            .preserve_client_rect
            .load(Ordering::Acquire)
    }) {
        Some(client_origin_for_handle(handle)?)
    } else {
        None
    };
    let focusable = extended_style(handle)? & WS_EX_NOACTIVATE == 0;
    // Keep Tao's visibility state in sync so repeated hide/show keeps working.
    // Disabling activation during show also protects against a future change to
    // Tao's focus:false/SW_SHOWNOACTIVATE implementation. Clicks work afterwards.
    if focusable {
        window.set_focusable(false).map_err(|e| e.to_string())?;
    }
    let shown = window.show().map_err(|e| e.to_string());
    let restored = if focusable {
        window.set_focusable(true).map_err(|e| e.to_string())
    } else {
        Ok(())
    };
    shown.and(restored)?;
    // The first hidden HWND can still report its pre-show caption insets. Tao
    // recalculates them while toggling focusability/showing, which otherwise
    // adds that caption height to the WebView. Preserve the requested client.
    if let Some(origin) = client_origin {
        set_client_rect_for_handle(handle, origin, desired_client)
    } else {
        restore_client_size_for_handle(handle, desired_client)
    }
}

fn client_origin_for_handle(handle: isize) -> Result<tauri::PhysicalPosition<i32>, String> {
    let mut origin = Point::default();
    if unsafe { ClientToScreen(handle, &mut origin) } == 0 {
        return Err(format!(
            "창 내부 원점 조회 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(tauri::PhysicalPosition::new(origin.x, origin.y))
}

fn client_size_for_handle(handle: isize) -> Result<tauri::PhysicalSize<u32>, String> {
    let mut client = Rect::default();
    if unsafe { GetClientRect(handle, &mut client) } == 0 {
        return Err(format!(
            "창 내부 크기 조회 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(tauri::PhysicalSize::new(
        client.right.saturating_sub(client.left).max(0) as u32,
        client.bottom.saturating_sub(client.top).max(0) as u32,
    ))
}

fn restore_client_size_for_handle(
    handle: isize,
    desired: tauri::PhysicalSize<u32>,
) -> Result<(), String> {
    let current = client_size_for_handle(handle)?;
    if current == desired {
        return Ok(());
    }
    let mut outer = Rect::default();
    if unsafe { GetWindowRect(handle, &mut outer) } == 0 {
        return Err(format!(
            "창 외곽 크기 조회 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    let width = i64::from(outer.right) - i64::from(outer.left) - i64::from(current.width)
        + i64::from(desired.width);
    let height = i64::from(outer.bottom) - i64::from(outer.top) - i64::from(current.height)
        + i64::from(desired.height);
    if !(1..=i64::from(i32::MAX)).contains(&width) || !(1..=i64::from(i32::MAX)).contains(&height) {
        return Err("창 외곽 크기가 허용 범위를 벗어났습니다".into());
    }
    // Only the outer dimensions change. Position, z-order and activation stay
    // intact; the callback records only WM_SIZING, never this programmatic size.
    set_outer_size_for_handle(
        handle,
        tauri::PhysicalSize::new(width as u32, height as u32),
    )
}

pub fn set_client_rect(
    window: &WebviewWindow,
    position: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
) -> Result<(), String> {
    set_client_rect_for_handle(
        window.hwnd().map_err(|e| e.to_string())?.0 as isize,
        position,
        size,
    )
}

fn set_client_rect_for_handle(
    handle: isize,
    position: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
) -> Result<(), String> {
    // Moving across monitors or the first hidden resize can change frame/DPI.
    // Re-read the actual client geometry after each bounded native correction.
    for _ in 0..3 {
        let current_origin = client_origin_for_handle(handle)?;
        let current_size = client_size_for_handle(handle)?;
        if current_origin == position && current_size == size {
            preserve_client_rect_for_handle(handle);
            return Ok(());
        }
        let mut outer = Rect::default();
        if unsafe { GetWindowRect(handle, &mut outer) } == 0 {
            return Err(format!(
                "창 외곽 크기 조회 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
        let x = i64::from(position.x) - (i64::from(current_origin.x) - i64::from(outer.left));
        let y = i64::from(position.y) - (i64::from(current_origin.y) - i64::from(outer.top));
        let width = i64::from(outer.right) - i64::from(outer.left) - i64::from(current_size.width)
            + i64::from(size.width);
        let height =
            i64::from(outer.bottom) - i64::from(outer.top) - i64::from(current_size.height)
                + i64::from(size.height);
        let coordinate = i64::from(i32::MIN)..=i64::from(i32::MAX);
        let dimension = 1..=i64::from(i32::MAX);
        if !coordinate.contains(&x)
            || !coordinate.contains(&y)
            || !dimension.contains(&width)
            || !dimension.contains(&height)
        {
            return Err("창 내부 영역이 허용 범위를 벗어났습니다".into());
        }
        // SWP_NOZORDER | SWP_NOACTIVATE: never show, activate, or reorder a note.
        if unsafe {
            SetWindowPos(
                handle,
                0,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                0x14,
            )
        } == 0
        {
            return Err(format!(
                "창 내부 영역 설정 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    if client_origin_for_handle(handle)? == position && client_size_for_handle(handle)? == size {
        // Verify after the final correction as well.
        preserve_client_rect_for_handle(handle);
        return Ok(());
    }
    Err("요청한 창 내부 영역을 적용하지 못했습니다".into())
}

fn preserve_client_rect_for_handle(handle: isize) {
    if let Some(data) = resize_observer(handle) {
        // Reopen preserves the user's latest geometry, not the original crop.
        unsafe {
            (&*(data as *const ResizeObserver))
                .preserve_client_rect
                .store(true, Ordering::Release)
        };
    }
}

pub fn set_outer_size(
    window: &WebviewWindow,
    size: tauri::PhysicalSize<u32>,
) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    set_outer_size_for_handle(handle, size)
}

fn set_outer_size_for_handle(handle: isize, size: tauri::PhysicalSize<u32>) -> Result<(), String> {
    // Placement rectangles include Windows' invisible resize frame. Tauri's
    // set_size accepts the client size and would add that frame on every reopen.
    // SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE; WM_SIZE keeps Tao informed.
    if unsafe { SetWindowPos(handle, 0, 0, 0, size.width as i32, size.height as i32, 0x16) } == 0 {
        return Err(format!(
            "창 외곽 크기 설정 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

pub fn set_opacity(window: &WebviewWindow, alpha: f64) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let style = extended_style(handle)?;
    let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    // Tao can replace extended styles when toggling pin/focusability. Recheck
    // instead of caching this bit, preserving all unrelated native styles.
    unsafe {
        if style & WS_EX_LAYERED == 0 {
            SetLastError(0);
            let previous = SetWindowLongW(handle, GWL_EXSTYLE, style | WS_EX_LAYERED);
            let error = std::io::Error::last_os_error();
            if previous == 0 && error.raw_os_error() != Some(0) {
                return Err(format!("창 반투명 스타일 설정 실패: {error}"));
            }
        } else {
            let (mut key, mut current, mut flags) = (0, 0, 0);
            if GetLayeredWindowAttributes(handle, &mut key, &mut current, &mut flags) != 0
                && current == alpha
                && flags == LWA_ALPHA
            {
                return Ok(());
            }
        }
        // System redirection blends the whole WebView2 host, including controls.
        // No color key or input-transparent style: normal hit testing is retained.
        if SetLayeredWindowAttributes(handle, 0, alpha, LWA_ALPHA) == 0 {
            return Err(format!(
                "창 반투명도 설정 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(
            extended: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            parameter: *mut std::ffi::c_void,
        ) -> isize;
        fn DestroyWindow(window: isize) -> i32;
        fn SendMessageW(window: isize, message: u32, wparam: usize, lparam: isize) -> isize;
    }

    struct TestWindow(isize);

    impl TestWindow {
        fn new() -> Self {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let window = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    0x00cf_0000,
                    -400,
                    -200,
                    336,
                    191,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                )
            };
            assert_ne!(window, 0);
            install_resize_observer_for_handle(window).unwrap();
            Self(window)
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe { DestroyWindow(self.0) };
        }
    }

    fn dimensions(rect: Rect) -> (i32, i32) {
        (rect.right - rect.left, rect.bottom - rect.top)
    }

    #[test]
    fn exact_client_rect_handles_negative_monitors_and_frame_changes_without_activation() {
        let window = TestWindow::new();
        let target = tauri::PhysicalPosition::new(-1200, -450);
        let size = tauri::PhysicalSize::new(640, 360);
        set_client_rect_for_handle(window.0, target, size).unwrap();
        assert_eq!(client_origin_for_handle(window.0).unwrap(), target);
        assert_eq!(client_size_for_handle(window.0).unwrap(), size);
        assert!(!take_native_resize_for_handle(window.0));
        let data = resize_observer(window.0).unwrap();
        assert!(unsafe {
            (&*(data as *const ResizeObserver))
                .preserve_client_rect
                .load(Ordering::Acquire)
        });
        unsafe {
            let style = GetWindowLongW(window.0, -16);
            SetWindowLongW(window.0, -16, style & !0x00c0_0000);
            assert_ne!(SetWindowPos(window.0, 0, 0, 0, 0, 0, 0x37), 0);
        }
        assert_ne!(client_origin_for_handle(window.0).unwrap(), target);
        set_client_rect_for_handle(window.0, target, size).unwrap();
        assert_eq!(client_origin_for_handle(window.0).unwrap(), target);
        assert_eq!(client_size_for_handle(window.0).unwrap(), size);
        assert!(!take_native_resize_for_handle(window.0));
        // A later user placement becomes the next preserved snapshot; the API
        // never retains and replays the original capture bounds.
        let moved = tauri::PhysicalPosition::new(500, 300);
        let resized = tauri::PhysicalSize::new(800, 450);
        set_client_rect_for_handle(window.0, moved, resized).unwrap();
        assert_eq!(client_origin_for_handle(window.0).unwrap(), moved);
        assert_eq!(client_size_for_handle(window.0).unwrap(), resized);
    }

    #[test]
    fn client_size_survives_native_frame_recalculation_without_moving_or_marking_manual() {
        let window = TestWindow::new();
        let desired = client_size_for_handle(window.0).unwrap();
        let mut before = Rect::default();
        assert_ne!(unsafe { GetWindowRect(window.0, &mut before) }, 0);
        unsafe {
            // Simulate the caption/invisible-frame change on first Tauri show.
            let style = GetWindowLongW(window.0, -16);
            SetWindowLongW(window.0, -16, style & !0x00c0_0000);
            assert_ne!(SetWindowPos(window.0, 0, 0, 0, 0, 0, 0x37), 0);
        }
        assert_ne!(client_size_for_handle(window.0).unwrap(), desired);
        restore_client_size_for_handle(window.0, desired).unwrap();
        assert_eq!(client_size_for_handle(window.0).unwrap(), desired);
        let mut after = Rect::default();
        assert_ne!(unsafe { GetWindowRect(window.0, &mut after) }, 0);
        assert_eq!((after.left, after.top), (before.left, before.top));
        assert!(!take_native_resize_for_handle(window.0));
        restore_client_size_for_handle(window.0, desired).unwrap();
        assert_eq!(client_size_for_handle(window.0).unwrap(), desired);
    }

    #[test]
    fn image_native_resize_constrains_all_edges_and_restores_after_fold_without_touching_text() {
        let image = TestWindow::new();
        let text = TestWindow::new();
        let (mut outer, mut client) = (Rect::default(), Rect::default());
        assert_ne!(unsafe { GetWindowRect(image.0, &mut outer) }, 0);
        assert_ne!(unsafe { GetClientRect(image.0, &mut client) }, 0);
        let frame = (
            dimensions(outer).0 - client.right,
            dimensions(outer).1 - client.bottom,
        );
        for ratio in [2.0, 0.5] {
            set_image_ratio_for_handle(image.0, Some(tauri::LogicalSize::new(ratio * 100., 100.)))
                .unwrap();
            for edge in 1..=8 {
                let proposal = Rect {
                    left: -500,
                    top: -300,
                    right: -500 + 450 + frame.0,
                    bottom: -300 + 230 + frame.1,
                };
                let mut adjusted = proposal;
                assert_eq!(
                    unsafe {
                        SendMessageW(image.0, WM_SIZING, edge, &mut adjusted as *mut _ as isize)
                    },
                    1
                );
                let (width, height) = dimensions(adjusted);
                let (width, height) = (f64::from(width - frame.0), f64::from(height - frame.1));
                // A nearest physical pixel in the dependent axis is unavoidable.
                assert!(
                    (width - height * ratio).abs() <= ratio.max(1.0),
                    "{edge}: {adjusted:?}"
                );
                assert!(width > 0. && height > 0.);
                if matches!(edge, 1 | 4 | 7) {
                    assert_eq!(adjusted.right, proposal.right);
                } else {
                    assert_eq!(adjusted.left, proposal.left);
                }
                if matches!(edge, 3..=5) {
                    assert_eq!(adjusted.bottom, proposal.bottom);
                } else {
                    assert_eq!(adjusted.top, proposal.top);
                }
                let mut unconstrained = proposal;
                unsafe {
                    SendMessageW(
                        text.0,
                        WM_SIZING,
                        edge,
                        &mut unconstrained as *mut _ as isize,
                    )
                };
                assert_eq!(unconstrained, proposal, "text note changed: {edge}");
            }
        }
        // Folding disables the native constraint before a programmatic resize;
        // unfolding restores it through the same setter used by sizing::toggle.
        set_image_ratio_for_handle(image.0, None).unwrap();
        let proposal = Rect {
            left: -500,
            top: -300,
            right: 10,
            bottom: -250,
        };
        let mut folded = proposal;
        unsafe { SendMessageW(image.0, WM_SIZING, 8, &mut folded as *mut _ as isize) };
        assert_eq!(folded, proposal);
        set_image_ratio_for_handle(image.0, Some(tauri::LogicalSize::new(200., 100.))).unwrap();
        let mut restored = proposal;
        unsafe { SendMessageW(image.0, WM_SIZING, 8, &mut restored as *mut _ as isize) };
        assert_ne!(restored, proposal);
        assert!(
            set_image_ratio_for_handle(image.0, Some(tauri::LogicalSize::new(f64::NAN, 100.)))
                .is_err()
        );
        // Programmatic sizes (including fold) stay free of ratio corrections.
        set_outer_size_for_handle(image.0, tauri::PhysicalSize::new(336, 191)).unwrap();
        assert_ne!(unsafe { GetWindowRect(image.0, &mut outer) }, 0);
        assert_eq!(dimensions(outer), (336, 191));
    }

    #[test]
    fn image_ratio_keeps_two_dip_minimum_on_both_axes() {
        for ratio in [0.1, 0.5, 2.0, 10.0] {
            for edge in 1..=8 {
                let mut rect = Rect {
                    left: -100,
                    top: -100,
                    right: -81,
                    bottom: -81,
                };
                assert!(constrain_sizing_rect(
                    &mut rect,
                    edge,
                    (400., 200.),
                    (18., 18.),
                    ratio,
                    3.
                ));
                let (width, height) = dimensions(rect);
                assert!(
                    width - 18 >= 3 && height - 18 >= 3,
                    "{ratio}: {edge}: {rect:?}"
                );
                assert!(
                    (f64::from(width - 18) - f64::from(height - 18) * ratio).abs()
                        <= ratio.max(1.0)
                );
            }
        }
    }

    #[test]
    fn native_user_resize_survives_drag_exit_but_programmatic_size_and_move_do_not_latch() {
        // A hidden, same-thread system-class HWND exercises the real comctl32
        // subclass chain without showing a window or changing user focus.
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0x00cf_0000, // WS_OVERLAPPEDWINDOW, hidden: includes a native frame
                0,
                0,
                100,
                100,
                0,
                0,
                0,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(window, 0);
        install_resize_observer_for_handle(window).unwrap();
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 160,
            bottom: 140,
        };
        set_outer_size_for_handle(window, tauri::PhysicalSize::new(336, 191)).unwrap();
        assert_ne!(unsafe { GetWindowRect(window, &mut rect) }, 0);
        assert_eq!((rect.right - rect.left, rect.bottom - rect.top), (336, 191));
        assert_ne!(unsafe { GetClientRect(window, &mut rect) }, 0);
        assert!(rect.right < 336 && rect.bottom < 191);
        unsafe {
            SendMessageW(window, 0x0005, 0, 0); // WM_SIZE: automatic fit/fold
            SendMessageW(window, 0x0216, 0, &mut rect as *mut _ as isize); // WM_MOVING
        }
        assert!(!take_native_resize_for_handle(window));
        unsafe {
            SendMessageW(window, WM_SIZING, 8, &mut rect as *mut _ as isize);
            SendMessageW(window, 0x0232, 0, 0); // WM_EXITSIZEMOVE before Tauri event
        }
        // Repeated installation must not replace/leak data or lose the signal.
        install_resize_observer_for_handle(window).unwrap();
        assert!(take_native_resize_for_handle(window));
        assert!(!take_native_resize_for_handle(window));
        assert_ne!(unsafe { DestroyWindow(window) }, 0);
        assert!(resize_observer(window).is_none());
    }
}
