use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, LogicalSize, Manager, State, WebviewWindow};

#[derive(Default)]
struct SizeState {
    content_height: Option<f64>,
    pending: Option<crate::entrance::Presentation>,
    image_size: Option<LogicalSize<f64>>,
    expanded: Option<LogicalSize<f64>>,
    contact: crate::warp::Contact,
}
#[derive(Default)]
pub struct Sizes(Mutex<BTreeMap<String, SizeState>>);

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
    #[cfg(windows)]
    crate::windows_window::set_image_ratio(window, size)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = (window, size);
    Ok(())
}

pub fn prepare(
    window: &WebviewWindow,
    presentation: crate::entrance::Presentation,
) -> Result<(), String> {
    if crate::stash::active(window.app_handle()) {
        crate::stash::release(window)?;
        return crate::show_without_focus(window);
    }
    let visible = window.is_visible().map_err(|e| e.to_string())?;
    crate::layout::register(window, visible || pending(window));
    if visible || image_size(window).is_some() || folded(window) {
        return crate::entrance::show(window, None, presentation);
    }
    window
        .state::<Sizes>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().into())
        .or_default()
        .pending = Some(presentation);
    window.emit("layout-measure", ()).map_err(|e| e.to_string())
}
pub fn pending(window: &WebviewWindow) -> bool {
    window
        .state::<Sizes>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.get(window.label()).map(|s| s.pending.is_some()))
        .unwrap_or(false)
}
pub fn cancel(window: &WebviewWindow) {
    if let Ok(mut states) = window.state::<Sizes>().0.lock() {
        if let Some(s) = states.get_mut(window.label()) {
            s.pending = None;
        }
    }
}
pub fn content_height(window: &WebviewWindow) -> f64 {
    window
        .state::<Sizes>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.get(window.label()).and_then(|s| s.content_height))
        .unwrap_or(48.0)
}
pub fn image_size(window: &WebviewWindow) -> Option<LogicalSize<f64>> {
    window
        .state::<Sizes>()
        .0
        .lock()
        .ok()?
        .get(window.label())?
        .image_size
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
    crate::layout::mark(&window, true);
    Ok(())
}

#[cfg(windows)]
pub fn native_resize(window: &WebviewWindow) -> Result<(), String> {
    let sizes = window.state::<Sizes>();
    // A programmatic frame update may reenter Resized while fold/fit owns Sizes.
    // Keep WM_SIZING latched until a later native event, measurement, or frame.
    let Ok(states) = sizes.0.try_lock() else {
        return Ok(());
    };
    if !crate::windows_window::take_native_resize(window)? {
        return Ok(());
    }
    let folded = states
        .get(window.label())
        .is_some_and(|s| s.expanded.is_some());
    drop(states);
    crate::entrance::cancel(window)?;
    if folded {
        return Err("접힌 메모는 펼친 뒤 크기를 조절하세요".into());
    }
    // Latest main stores user intent in layout exclusions, not Sizes.manual.
    crate::layout::mark(window, true);
    Ok(())
}

#[tauri::command]
pub fn fit_note(
    window: WebviewWindow,
    width: f64,
    height: f64,
    revision: u64,
    local_flow: Option<u64>,
) -> Result<bool, String> {
    let _scope = crate::local_trace::webview_scope(&window, local_flow);
    let _timing = crate::telemetry::Timing::new("window.fit");
    #[cfg(windows)]
    native_resize(&window)?;
    if !width.is_finite() || !height.is_finite() {
        return Err("유효하지 않은 패널 크기입니다".into());
    }
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    if window
        .state::<crate::AppNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .get(id)
        .is_none_or(|note| note.revision != revision)
    {
        return Ok(false);
    }
    let presentation = {
        let state = window.state::<Sizes>();
        let mut states = state.0.lock().map_err(|e| e.to_string())?;
        let s = states.entry(window.label().into()).or_default();
        s.content_height = Some(height.ceil().clamp(48.0, 420.0));
        s.pending.take()
    };
    // Measuring visible content only refreshes its cap for the next open.
    if let Some(presentation) = presentation {
        crate::entrance::show(&window, None, presentation)?;
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

pub(crate) fn expanded_size(window: &WebviewWindow) -> Option<LogicalSize<f64>> {
    window
        .state::<Sizes>()
        .0
        .lock()
        .ok()?
        .get(window.label())?
        .expanded
}
pub(crate) fn restore_fold(
    window: &WebviewWindow,
    expanded: Option<LogicalSize<f64>>,
) -> Result<(), String> {
    if let Some(size) = expanded {
        window.set_size(size).map_err(|e| e.to_string())?;
        let sizes = window.state::<Sizes>();
        let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
        let state = states.entry(window.label().into()).or_default();
        state.expanded = None;
        toggle(window, state)?;
    }
    Ok(())
}

fn toggle(window: &WebviewWindow, state: &mut SizeState) -> Result<(), String> {
    crate::entrance::cancel(window)?;
    crate::layout::mark(window, true);
    if let Some(size) = state.expanded {
        window.set_size(size).map_err(|e| e.to_string())?;
        window
            .set_min_size(Some(if state.image_size.is_some() {
                LogicalSize::new(2.0, 2.0)
            } else if window.label().starts_with("webview-") {
                LogicalSize::new(240.0, 160.0)
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
                24.0,
            )))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(size.width, 24.0))
            .map_err(|e| e.to_string())?;
        state.expanded = Some(size);
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_fold(window: WebviewWindow, sizes: State<Sizes>) -> Result<bool, String> {
    if crate::stash::owns(&window) {
        return Ok(folded(&window));
    }
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    let (cursor, _) = contact_geometry(&window)?;
    state.contact.block(cursor);
    toggle(&window, state)?;
    Ok(state.expanded.is_some())
}

#[tauri::command]
pub fn chrome_fold(view: tauri::Webview, sizes: State<Sizes>) -> Result<bool, String> {
    let window = crate::web::managed_window(view.app_handle(), view.window().label())
        .ok_or("메모 창을 찾을 수 없습니다")?;
    let collapsed = toggle_fold(window, sizes)?;
    if let Some(id) = view.label().strip_prefix("webview-") {
        if let Some(content) = view.app_handle().get_webview(&format!("webpage-{id}")) {
            if collapsed {
                content.hide()
            } else {
                content.show()
            }
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(collapsed)
}

#[tauri::command]
pub fn poll_fold(window: WebviewWindow, sizes: State<Sizes>) -> Result<bool, String> {
    if !fold_contact_enabled(window.label()) {
        return Ok(false);
    }
    if crate::capture::active(window.app_handle())
        || crate::stash::owns(&window)
        || crate::stash::chord_busy(window.app_handle())
    {
        let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
        let state = states.entry(window.label().into()).or_default();
        // Entry observed before capture/stash must not trigger later inside.
        state.contact = crate::warp::Contact::default();
        return Ok(state.expanded.is_some());
    }
    #[cfg(target_os = "macos")]
    let control = control_only_macos(objc2_app_kit::NSEvent::modifierFlags_class());
    #[cfg(windows)]
    let control = {
        let modifiers = crate::windows_contact::modifiers();
        modifiers.control_only()
    };
    #[cfg(not(any(target_os = "macos", windows)))]
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

#[cfg(target_os = "macos")]
fn control_only_macos(flags: objc2_app_kit::NSEventModifierFlags) -> bool {
    use objc2_app_kit::NSEventModifierFlags as Flags;
    crate::warp::only_modifier_macos(flags, Flags::Control)
}

fn fold_contact_enabled(label: &str) -> bool {
    label != crate::updater::NOTICE_LABEL
}

// Convert non-macOS physical values together so the folded margin stays 4 DIP.
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
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        Ok((
            (c.x / scale, c.y / scale),
            (
                p.x as f64 / scale,
                p.y as f64 / scale,
                s.width as f64 / scale,
                s.height as f64 / scale,
            ),
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
        for window in crate::web::managed_windows(app).into_values() {
            if window.label() == crate::manager::LABEL
                || !fold_contact_enabled(window.label())
                || window.label().starts_with("agent-")
            {
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
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_contact_folding_accepts_only_control() {
        use objc2_app_kit::NSEventModifierFlags as Flags;
        let others = [
            Flags::Shift,
            Flags::Option,
            Flags::Command,
            Flags::Function,
            Flags::CapsLock,
            Flags::Help,
            Flags::NumericPad,
        ];
        for bits in 0..(1 << others.len()) {
            let extra = others
                .iter()
                .enumerate()
                .fold(Flags::empty(), |flags, (i, flag)| {
                    if bits & (1 << i) != 0 {
                        flags | *flag
                    } else {
                        flags
                    }
                });
            assert!(!control_only_macos(extra));
            let eligible = control_only_macos(Flags::Control | extra);
            assert_eq!(eligible, bits == 0, "modifier mask {bits}");
            let mut contact = crate::warp::Contact::default();
            assert!(!contact.trigger(eligible, false, false, (0., 0.)));
            assert_eq!(
                contact.trigger(eligible, true, false, (10., 10.)),
                bits == 0
            );
            // Releasing an extra key while already inside is not a new gesture.
            assert!(!contact.trigger(true, true, false, (11., 10.)));
            assert!(!contact.trigger(true, false, false, (0., 0.)));
            assert!(contact.trigger(true, true, false, (10., 10.)));
        }
        // Device-specific low bits do not represent additional modifier keys.
        assert!(control_only_macos(
            Flags::Control | Flags::from_bits_retain(1)
        ));
    }

    #[test]
    fn update_notice_is_excluded_while_memo_windows_keep_contact_folding() {
        assert!(!fold_contact_enabled("updater-notice"));
        assert!(fold_contact_enabled("main"));
        assert!(fold_contact_enabled("note-work"));
    }

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
