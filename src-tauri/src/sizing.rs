use std::{collections::BTreeMap, sync::Mutex};
#[cfg(target_os = "macos")]
use tauri::Emitter;
use tauri::{LogicalSize, Manager, State, WebviewWindow};

#[derive(Default)]
struct SizeState {
    manual: bool,
    wheel_anchor: Option<WheelAnchor>,
    image_size: Option<LogicalSize<f64>>,
    expanded: Option<LogicalSize<f64>>,
    contact: crate::warp::Contact,
    placed: bool,
    pending: bool,
    presentation: crate::entrance::Presentation,
}
#[derive(Default)]
pub struct Sizes(Mutex<BTreeMap<String, SizeState>>);

#[derive(Clone, Copy)]
struct WheelAnchor {
    position: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
    center: (f64, f64),
}

// The native constraint applies during live dragging, without resize-event corrections.
pub fn constrain_image(window: &WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let size = LogicalSize::new(width, height);
    set_image_ratio(window, Some(size))?;
    window
        .state::<Sizes>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().into())
        .or_default()
        .image_size = Some(size);
    Ok(())
}

fn set_image_ratio(window: &WebviewWindow, size: Option<LogicalSize<f64>>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: creation and synchronous fold commands execute on the main thread.
        let native = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        if let Some(size) = size {
            let mut ratio = native.contentAspectRatio();
            ratio.width = size.width;
            ratio.height = size.height;
            native.setContentAspectRatio(ratio);
        } else {
            // A zero aspect ratio can leave AppKit with invalid geometry and
            // SIGTRAP on a later resize or display reconfiguration. Resize
            // increments and aspect ratio are mutually exclusive constraints;
            // unit increments restore unconstrained resizing.
            let mut increments = native.contentResizeIncrements();
            increments.width = 1.0;
            increments.height = 1.0;
            native.setContentResizeIncrements(increments);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, size);
    Ok(())
}

pub fn prepare(
    window: &WebviewWindow,
    presentation: crate::entrance::Presentation,
) -> Result<(), String> {
    let sizes = window.state::<Sizes>();
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    state.pending = true;
    state.presentation = presentation;
    Ok(())
}
pub fn cancel(window: &WebviewWindow) {
    if let Ok(mut states) = window.state::<Sizes>().0.lock() {
        if let Some(state) = states.get_mut(window.label()) {
            state.pending = false;
        }
    }
}

// macOS may handle the native resize border before the WebView's grip handler.
// Record only live user resizing, never programmatic fit/fold/placement changes.
#[cfg(target_os = "macos")]
pub fn native_resize(window: &WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    // SAFETY: the window event callback runs on the main thread and Tauri owns it.
    let live = unsafe { (&*(ptr as *const objc2_app_kit::NSWindow)).inLiveResize() };
    if live {
        manual_resize(window.clone(), window.state::<Sizes>())?;
        window
            .emit_to(window.label(), "manual-resize", window.label())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn manual_resize(window: WebviewWindow, sizes: State<Sizes>) -> Result<(), String> {
    crate::entrance::cancel(&window)?;
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    if state.expanded.is_some() {
        return Err("접힌 메모는 펼친 뒤 크기를 조절하세요".into());
    }
    state.manual = true;
    Ok(())
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeAxis {
    Both,
    Horizontal,
    Vertical,
}

fn wheel_size(
    width: f64,
    height: f64,
    delta: f64,
    axis: ResizeAxis,
    min: (f64, f64),
) -> (f64, f64) {
    let factor = (-delta.clamp(-240.0, 240.0) * 0.002).exp();
    match axis {
        ResizeAxis::Both => {
            let factor = factor.max(min.0 / width).max(min.1 / height);
            (width * factor, height * factor)
        }
        ResizeAxis::Horizontal => ((width * factor).max(min.0), height),
        ResizeAxis::Vertical => (width, (height * factor).max(min.1)),
    }
}

#[tauri::command]
pub fn wheel_resize(
    window: WebviewWindow,
    sizes: State<Sizes>,
    delta: f64,
    axis: ResizeAxis,
) -> Result<(), String> {
    if !delta.is_finite() {
        return Err("유효하지 않은 휠 값입니다".into());
    }
    if delta == 0.0 || !window.is_visible().map_err(|e| e.to_string())? {
        return Ok(());
    }
    manual_resize(window.clone(), sizes.clone())?;
    let image = sizes
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(window.label())
        .and_then(|s| s.image_size);
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let before = window.outer_size().map_err(|e| e.to_string())?;
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let center = sizes
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(window.label())
        .and_then(|s| s.wheel_anchor)
        .filter(|a| a.position == position && a.size == before)
        .map(|a| a.center)
        .unwrap_or((
            f64::from(position.x) + f64::from(before.width) / 2.0,
            f64::from(position.y) + f64::from(before.height) / 2.0,
        ));
    let size = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let min = if image.is_some() {
        (2.0, 2.0)
    } else {
        (144.0, 32.0)
    };
    let (width, height) = wheel_size(size.width, size.height, delta, axis, min);
    // Axis-only resizing is explicit even for image memos. Restore their native
    // drag constraint afterwards; proportional wheel resizing uses the current ratio.
    if image.is_some() {
        set_image_ratio(&window, None)?;
    }
    #[cfg(target_os = "macos")]
    {
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: synchronous Tauri commands execute on the main thread. Set
        // the complete native frame atomically; Tao's set_size is deferred.
        let native = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        let mut frame = native.frame();
        let new_width = width + f64::from(before.width) / scale - size.width;
        let new_height = height + f64::from(before.height) / scale - size.height;
        frame.origin.x += (frame.size.width - new_width) / 2.0
            + (center.0 - f64::from(position.x) - f64::from(before.width) / 2.0) / scale;
        frame.origin.y += (frame.size.height - new_height) / 2.0
            - (center.1 - f64::from(position.y) - f64::from(before.height) / 2.0) / scale;
        frame.size.width = new_width;
        frame.size.height = new_height;
        native.setFrame_display(frame, true);
        if image.is_some() {
            set_image_ratio(&window, image)?;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let (position, after) = {
        window
            .set_size(LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        // Size changes can be queued by the platform. Use the requested physical
        // dimensions including the existing frame inset, not a stale size read.
        let target_width = (width * scale).round() + f64::from(before.width) - size.width * scale;
        let target_height =
            (height * scale).round() + f64::from(before.height) - size.height * scale;
        let position = tauri::PhysicalPosition::new(
            (center.0 - target_width / 2.0).round() as i32,
            (center.1 - target_height / 2.0).round() as i32,
        );
        window.set_position(position).map_err(|e| e.to_string())?;
        (
            position,
            tauri::PhysicalSize::new(target_width as u32, target_height as u32),
        )
    };
    #[cfg(target_os = "macos")]
    let (position, after) = (
        window.outer_position().map_err(|e| e.to_string())?,
        window.outer_size().map_err(|e| e.to_string())?,
    );
    sizes
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().into())
        .or_default()
        .wheel_anchor = Some(WheelAnchor {
        position,
        size: after,
        center,
    });
    Ok(())
}

#[tauri::command]
pub fn fit_note(
    window: WebviewWindow,
    sizes: State<Sizes>,
    width: f64,
    height: f64,
) -> Result<bool, String> {
    if !width.is_finite() || !height.is_finite() {
        return Err("유효하지 않은 패널 크기입니다".into());
    }
    if crate::entrance::active(&window) {
        return Ok(false);
    }
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    if state.manual || state.expanded.is_some() {
        return Ok(true);
    }
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("패널의 화면을 찾을 수 없습니다")?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let size = LogicalSize::new(
        300.0_f64.min((f64::from(work.size.width) / scale - 16.0).max(144.0)),
        height
            .clamp(48.0, 420.0)
            .min((f64::from(work.size.height) / scale - 16.0).max(32.0)),
    );
    if !state.placed {
        let destination = crate::place_new_window(window.app_handle(), &window, Some(size))?;
        state.placed = true;
        if state.pending {
            let next = window.clone();
            std::thread::spawn(move || {
                // Let native move/resize notifications settle before the first show.
                std::thread::sleep(std::time::Duration::from_millis(20));
                let app = next.app_handle().clone();
                let _ = app.run_on_main_thread(move || {
                    let should_show = if let Ok(mut states) = next.state::<Sizes>().0.lock() {
                        let state = states.entry(next.label().into()).or_default();
                        std::mem::take(&mut state.pending).then_some(state.presentation)
                    } else {
                        None
                    };
                    if let Some(presentation) = should_show {
                        if let Err(error) =
                            crate::entrance::show(&next, Some(destination), presentation)
                        {
                            eprintln!("패널 표시 실패: {error}");
                        }
                    }
                });
            });
        }
    } else {
        let position = window.outer_position().map_err(|e| e.to_string())?;
        let width_px = (size.width * scale).round() as i32;
        let height_px = (size.height * scale).round() as i32;
        let x = position.x.clamp(
            work.position.x,
            work.position.x + (work.size.width as i32 - width_px).max(0),
        );
        let y = position.y.clamp(
            work.position.y,
            work.position.y + (work.size.height as i32 - height_px).max(0),
        );
        window.set_size(size).map_err(|e| e.to_string())?;
        if (x, y) != (position.x, position.y) {
            window
                .set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(true)
}

pub fn folded(window: &WebviewWindow) -> bool {
    window
        .state::<Sizes>()
        .0
        .lock()
        .ok()
        .and_then(|states| states.get(window.label()).map(|s| s.expanded.is_some()))
        .unwrap_or(false)
}

fn toggle(window: &WebviewWindow, state: &mut SizeState) -> Result<(), String> {
    crate::entrance::cancel(window)?;
    if let Some(size) = state.expanded {
        window.set_size(size).map_err(|e| e.to_string())?;
        window
            .set_min_size(Some(if state.image_size.is_some() {
                LogicalSize::new(2.0, 2.0)
            } else {
                LogicalSize::new(144.0, 32.0)
            }))
            .map_err(|e| e.to_string())?;
        set_image_ratio(window, state.image_size)?;
        state.expanded = None;
    } else {
        let size = window
            .inner_size()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
        set_image_ratio(window, None)?;
        window
            .set_min_size(Some(LogicalSize::new(
                if state.image_size.is_some() {
                    2.0
                } else {
                    144.0
                },
                12.0,
            )))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(size.width, 12.0))
            .map_err(|e| e.to_string())?;
        state.expanded = Some(size);
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_fold(window: WebviewWindow, sizes: State<Sizes>) -> Result<bool, String> {
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    let (cursor, _) = contact_geometry(&window)?;
    state.contact.block(cursor);
    toggle(&window, state)?;
    Ok(state.expanded.is_some())
}

#[tauri::command]
pub fn poll_fold(window: WebviewWindow, sizes: State<Sizes>) -> Result<bool, String> {
    if crate::capture::active(window.app_handle()) {
        return Ok(folded(&window));
    }
    #[cfg(target_os = "macos")]
    let control = objc2_app_kit::NSEvent::modifierFlags_class()
        .contains(objc2_app_kit::NSEventModifierFlags::Control)
        && !crate::warp::close_shortcut_pressed();
    #[cfg(not(target_os = "macos"))]
    let control = false;
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    if !window.is_visible().map_err(|e| e.to_string())? {
        state.contact = crate::warp::Contact::default();
        return Ok(state.expanded.is_some());
    }
    let (cursor, rect) = contact_geometry(&window)?;
    let inside = contains_contact(rect, cursor, state.expanded.is_some());
    if state.contact.trigger(
        control,
        inside,
        window.is_focused().map_err(|e| e.to_string())?,
        cursor,
    ) {
        toggle(&window, state)?;
    }
    Ok(state.expanded.is_some())
}

// All values share desktop logical coordinates, including mixed-DPI displays.
type ContactGeometry = ((f64, f64), (f64, f64, f64, f64));
fn contact_geometry(window: &WebviewWindow) -> Result<ContactGeometry, String> {
    #[cfg(target_os = "macos")]
    {
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: synchronous commands and event monitors run on the main thread;
        // Tauri owns the window throughout this borrow.
        let frame = unsafe { (&*(ptr as *const objc2_app_kit::NSWindow)).frame() };
        let cursor = objc2_app_kit::NSEvent::mouseLocation();
        Ok((
            (cursor.x, cursor.y),
            (
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
            ),
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let c = window.cursor_position().map_err(|e| e.to_string())?;
        let p = window.outer_position().map_err(|e| e.to_string())?;
        let s = window.outer_size().map_err(|e| e.to_string())?;
        Ok((
            (c.x, c.y),
            (p.x as f64, p.y as f64, s.width as f64, s.height as f64),
        ))
    }
}

fn contains_contact(
    (x, y, w, h): (f64, f64, f64, f64),
    (cx, cy): (f64, f64),
    folded: bool,
) -> bool {
    let margin = if folded { 4.0 } else { 0.0 };
    cx >= x - margin && cy >= y - margin && cx < x + w + margin && cy < y + h + margin
}

#[cfg(target_os = "macos")]
pub fn install_contact_monitor(app: &tauri::AppHandle) {
    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::AnyObject};
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::{cell::RefCell, ptr::NonNull};
    // Monitor tokens remain owned on the main thread for the app lifetime.
    struct Monitor(Retained<AnyObject>);
    impl Drop for Monitor {
        fn drop(&mut self) {
            // SAFETY: this token came from an NSEvent monitor registration.
            unsafe { NSEvent::removeMonitor(&self.0) };
        }
    }
    thread_local! { static MONITORS: RefCell<Vec<Monitor>> = const { RefCell::new(Vec::new()) }; }
    fn sample(app: &tauri::AppHandle) {
        if crate::capture::active(app) {
            return;
        }
        for window in app.webview_windows().into_values() {
            if crate::feedback_window::is_surface(window.label()) {
                continue;
            }
            if let Err(error) = poll_fold(window.clone(), window.state::<Sizes>()) {
                eprintln!("접촉 감지 실패: {error}");
            }
            if let Err(error) = crate::sync_opacity(window) {
                eprintln!("롤오버 표시 실패: {error}");
            }
        }
    }
    let mask = NSEventMask::MouseMoved
        | NSEventMask::LeftMouseDragged
        | NSEventMask::RightMouseDragged
        | NSEventMask::OtherMouseDragged
        | NSEventMask::FlagsChanged;
    let handle = app.clone();
    let global = RcBlock::new(move |_: NonNull<NSEvent>| sample(&handle));
    let handle = app.clone();
    let local = RcBlock::new(move |event: NonNull<NSEvent>| {
        sample(&handle);
        event.as_ptr()
    });
    let global = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global);
    // SAFETY: the local handler returns the original valid event unchanged.
    let local = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) };
    MONITORS.with(|tokens| {
        tokens
            .borrow_mut()
            .extend(global.into_iter().chain(local).map(Monitor));
    });
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    #[test]
    fn folded_margin_handles_small_title_and_negative_coordinates() {
        let rect = (-300., -100., 300., 12.);
        assert!(contains_contact(rect, (-150., -103.), true));
        assert!(!contains_contact(rect, (-150., -103.), false));
        assert!(!contains_contact(rect, (-150., -105.), true));
        assert!(!contains_contact(rect, (5., -95.), true));
    }
    #[test]
    fn rapid_exit_and_reentry_rearms_without_bouncing_after_resize() {
        let mut contact = crate::warp::Contact::default();
        assert!(!contact.trigger(true, false, false, (100., 80.)));
        assert!(contact.trigger(true, true, false, (100., 100.)));
        assert!(!contact.trigger(true, false, false, (100., 100.)));
        assert!(!contact.trigger(true, false, false, (100., 80.)));
        assert!(contact.trigger(true, true, false, (100., 100.)));
        assert!(!contact.trigger(true, true, false, (101., 100.)));
        assert!(!contact.trigger(false, true, false, (101., 100.)));
        assert!(!contact.trigger(true, true, false, (101., 100.)));
    }
}

#[cfg(test)]
mod wheel_tests {
    use super::*;
    #[test]
    fn proportional_resize_preserves_ratio_at_minimum() {
        let (w, h) = wheel_size(150.0, 300.0, 240.0, ResizeAxis::Both, (144.0, 32.0));
        assert_eq!(w, 144.0);
        assert_eq!(h, 288.0);
    }
    #[test]
    fn independent_axes_and_direction() {
        let (w, h) = wheel_size(300.0, 200.0, -100.0, ResizeAxis::Horizontal, (144.0, 32.0));
        assert!(w > 300.0);
        assert_eq!(h, 200.0);
        let (w, h) = wheel_size(300.0, 40.0, 240.0, ResizeAxis::Vertical, (144.0, 32.0));
        assert_eq!((w, h), (300.0, 32.0));
    }
}
