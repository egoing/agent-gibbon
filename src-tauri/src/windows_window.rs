//! Native operations on this app's HWNDs, called only on Tauri's main thread.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::WebviewWindow;

const GWL_EXSTYLE: i32 = -20;
const WS_EX_LAYERED: i32 = 0x0008_0000;
const WS_EX_NOACTIVATE: i32 = 0x0800_0000;
const LWA_ALPHA: u32 = 2;
const WM_SIZING: u32 = 0x0214;
const WM_NCDESTROY: u32 = 0x0082;
const RESIZE_OBSERVER: usize = 1;

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
        unsafe { (&*(data as *const AtomicBool)).store(true, Ordering::Release) };
    } else if message == WM_NCDESTROY {
        // Remove before releasing reference data. Do not borrow it across the
        // next handler: DefSubclassProc can synchronously reenter window code.
        unsafe {
            RemoveWindowSubclass(window, Some(observe_resize), id);
            drop(Box::from_raw(data as *mut AtomicBool));
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
    let data = Box::into_raw(Box::new(AtomicBool::new(false)));
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
        unsafe { (&*(data as *const AtomicBool)).swap(false, Ordering::AcqRel) }
    })
}

pub fn take_native_resize(window: &WebviewWindow) -> Result<bool, String> {
    Ok(take_native_resize_for_handle(
        window.hwnd().map_err(|e| e.to_string())?.0 as isize,
    ))
}

#[link(name = "user32")]
extern "system" {
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
    shown.and(restored)
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
        fn GetWindowRect(window: isize, rect: *mut Rect) -> i32;
        fn GetClientRect(window: isize, rect: *mut Rect) -> i32;
    }

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
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
