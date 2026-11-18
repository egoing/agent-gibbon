//! Frozen screen selection; only app-created PNGs are exposed to note windows.
mod visibility;
use crate::sizing::TOOLBAR_ROW_HEIGHT;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Cursor,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{Manager, WebviewWindow};
use xcap::image::{self, RgbaImage};

#[derive(Clone, Serialize, Deserialize)]
pub struct ImageNote {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Image,
    Text,
    Terminal,
    Web,
}
struct Screen {
    generation: u64,
    mode: SelectionMode,
    ready: bool,
    image: RgbaImage,
    bounds: ImageNote,
}
#[derive(Default)]
pub struct Captures(Mutex<BTreeMap<String, Screen>>);
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub(crate) fn png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(err)?;
    Ok(bytes.into_inner())
}
fn data(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}

// Local development diagnostics contain only fixed lifecycle stages and timing.
fn trace(stage: &str) {
    #[cfg(debug_assertions)]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        eprintln!(
            "capture +{}ms {stage}",
            START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_millis()
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
}

pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    app.manage(visibility::Visibility::default());
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    app.plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .map_err(err)?;
    // Another running instance may own the shortcut. Keep the menu usable.
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut("Ctrl+Shift+1", |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                trigger(app, SelectionMode::Image);
            }
        })
    {
        eprintln!("캡처 단축키 등록 실패 (메뉴에서 실행 가능): {error}");
    }
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut("Ctrl+Shift+2", |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                trigger(app, SelectionMode::Text);
            }
        })
    {
        eprintln!("영역 메모 단축키 등록 실패 (메뉴에서 실행 가능): {error}");
    }
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut("Ctrl+Shift+3", |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                trigger(app, SelectionMode::Terminal);
            }
        })
    {
        eprintln!("터미널 단축키 등록 실패 (메뉴에서 실행 가능): {error}");
    }
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut("Ctrl+Shift+4", |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                trigger(app, SelectionMode::Web);
            }
        })
    {
        eprintln!("웹 단축키 등록 실패 (메뉴에서 실행 가능): {error}");
    }
    Ok(())
}
static LAUNCH_QUEUED: AtomicBool = AtomicBool::new(false);
static NEXT_SELECTION: AtomicU64 = AtomicU64::new(1);
// Windows lifecycle work runs on blocking workers only. Never acquire this
// mutex from capture_ready, a native event, or another UI-thread callback.
#[cfg(windows)]
static LIFECYCLE: Mutex<()> = Mutex::new(());
#[cfg(windows)]
async fn lifecycle(
    operation: &'static str,
    work: impl FnOnce() -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let sentry_parent = crate::telemetry::TimingContext::request("capture.request");
    tauri::async_runtime::spawn_blocking(move || {
        let _sentry_scope = sentry_parent.enter();
        let _root = crate::telemetry::Timing::new(operation);
        let _operation = {
            let _wait = crate::telemetry::Timing::new("capture.lifecycle.wait");
            LIFECYCLE.lock().map_err(err)?
        };
        work()
    })
    .await
    .map_err(err)?
}

pub fn trigger(app: &tauri::AppHandle, mode: SelectionMode) {
    if crate::updater::frozen(app) || crate::privacy::onboarding_required(app) {
        return;
    }
    trace("trigger");
    if LAUNCH_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    // Leave the native shortcut/menu callback before building WebViews. Sending
    // from another thread wakes the event loop even when the mouse stays still.
    let sentry_parent = crate::telemetry::TimingContext::request("capture.request");
    let app = app.clone();
    std::thread::spawn(move || {
        let _sentry_scope = sentry_parent.clone().enter();
        let generation = NEXT_SELECTION.fetch_add(1, Ordering::Relaxed);
        #[cfg(windows)]
        {
            let _root = crate::telemetry::Timing::new("capture.prepare");
            let operation = {
                let _wait = crate::telemetry::Timing::new("capture.lifecycle.wait");
                LIFECYCLE.lock()
            };
            if let Ok(_operation) = operation {
                launch(&app, mode, generation);
            }
            LAUNCH_QUEUED.store(false, Ordering::Release);
        }
        #[cfg(not(windows))]
        {
            // Finish hiding overview in a separate UI dispatch before acquiring
            // the frozen preview. Keeping both in begin blocks the event loop
            // throughout screenshot acquisition and WebView construction.
            if !active(&app) {
                if let Err(error) = visibility::set(&app, generation, true) {
                    let _ = visibility::set(&app, generation, false);
                    eprintln!("모아보기 캡처 준비 실패: {error}");
                    LAUNCH_QUEUED.store(false, Ordering::Release);
                    return;
                }
            }
            let handle = app.clone();
            if app
                .run_on_main_thread(move || {
                    let _sentry_scope = sentry_parent.enter();
                    launch(&handle, mode, generation);
                    LAUNCH_QUEUED.store(false, Ordering::Release);
                })
                .is_err()
            {
                let _ = visibility::set(&app, generation, false);
                LAUNCH_QUEUED.store(false, Ordering::Release);
            }
        }
    });
}
fn launch(app: &tauri::AppHandle, mode: SelectionMode, generation: u64) {
    if let Err(error) = begin(app, mode, generation) {
        let _ = cleanup_before_create(app);
        if let Err(e) = visibility::set(app, generation, false) {
            eprintln!("모아보기 재표시 실패: {e}");
        }
        if let Some(tray) = app.tray_by_id("agent-gibbon") {
            let _ = tray.set_tooltip(Some(format!("화면 캡처 실패: {error}")));
        }
        eprintln!("화면 캡처 실패: {error}");
    }
}
pub fn update_blocked(app: &tauri::AppHandle) -> bool {
    LAUNCH_QUEUED.load(Ordering::Acquire)
        || app
            .state::<Captures>()
            .0
            .lock()
            .map(|s| !s.is_empty())
            .unwrap_or(true)
}
pub fn active(app: &tauri::AppHandle) -> bool {
    app.state::<Captures>()
        .0
        .lock()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}
#[cfg(target_os = "macos")]
pub(crate) fn show_permission_guide(app: &tauri::AppHandle) -> Result<(), String> {
    show_permission_scope(app, false)
}

#[cfg(target_os = "macos")]
pub(crate) fn show_shortcut_permission_guide(app: &tauri::AppHandle) -> Result<(), String> {
    show_permission_scope(app, true)
}

#[cfg(target_os = "macos")]
fn show_permission_scope(app: &tauri::AppHandle, shortcut: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("permission-guide") {
        // Do not interrupt a settings flow or repeatedly raise an open guide.
        if shortcut && window.is_visible().map_err(err)? {
            return Ok(());
        }
        window
            .eval(format!("window.setPermissionScope({shortcut})"))
            .map_err(err)?;
        return window.show().map_err(err);
    }
    tauri::WebviewWindowBuilder::new(
        app,
        "permission-guide",
        tauri::WebviewUrl::App(if shortcut {
            "permission.html?shortcut=1".into()
        } else {
            "permission.html".into()
        }),
    )
    .title("Agent Gibbon 권한 설정")
    .inner_size(600.0, 450.0)
    .min_inner_size(380.0, 320.0)
    .focused(false)
    .always_on_top(true)
    .build()
    .map(|_| ())
    .map_err(err)
}

#[cfg(target_os = "macos")]
pub(crate) fn accessibility_allowed() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(target_os = "macos")]
pub(crate) fn permission_status() -> (bool, bool) {
    (
        unsafe { CGPreflightScreenCaptureAccess() },
        accessibility_allowed(),
    )
}

#[tauri::command]
pub fn permission_guide(
    window: tauri::WebviewWindow,
    action: String,
) -> Result<(bool, bool, String, bool), String> {
    if window.label() != "permission-guide" {
        return Err("권한 안내 창에서 실행하세요".into());
    }
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn gibbon_show_permission_panel(screen: bool, overview: *mut std::ffi::c_void);
            fn gibbon_hide_permission_panel();
        }
        let pane = match action.as_str() {
            "screen" => Some("Privacy_ScreenCapture"),
            "access" => Some("Privacy_Accessibility"),
            "status" | "observe-screen" | "observe-access" => None,
            _ => return Err("알 수 없는 동작입니다".into()),
        };
        if let Some(pane) = pane {
            let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
            if !std::process::Command::new("/usr/bin/open")
                .arg(url)
                .status()
                .map_err(err)?
                .success()
            {
                return Err("시스템 설정을 열지 못했습니다".into());
            }
            let overview = window.clone();
            let screen = action == "screen";
            window
                .run_on_main_thread(move || {
                    if let Ok(pointer) = overview.ns_window() {
                        unsafe {
                            gibbon_show_permission_panel(screen, pointer);
                        }
                    }
                })
                .map_err(err)?;
        }
        let (screen, access) = permission_status();
        if access && !crate::stash::input_owned(window.app_handle()) {
            let handle = window.app_handle().clone();
            window
                .run_on_main_thread(move || {
                    if let Err(error) = crate::stash::refresh_input(&handle) {
                        eprintln!("단축키 입력 감시 재설치 실패: {error}");
                    }
                })
                .map_err(err)?;
        }
        if let Err(error) =
            crate::tray::sync_permission_item(window.app_handle(), screen && access, false)
        {
            eprintln!("권한 메뉴 갱신 실패: {error}");
        }
        if action == "status"
            || (action == "observe-screen" && screen)
            || (action == "observe-access" && access)
        {
            window
                .run_on_main_thread(|| unsafe { gibbon_hide_permission_panel() })
                .map_err(err)?;
        }
        Ok((
            screen,
            access,
            std::env::current_exe().map_err(err)?.display().to_string(),
            crate::stash::input_owned(window.app_handle()),
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = action;
        Err("macOS에서 지원합니다".into())
    }
}

fn begin(app: &tauri::AppHandle, mode: SelectionMode, generation: u64) -> Result<(), String> {
    let _update_activity = crate::updater::activity(app)?;
    #[cfg(not(windows))]
    let _root = crate::telemetry::Timing::new("capture.prepare");
    trace("begin");
    #[cfg(target_os = "macos")]
    // Called only by an explicit capture action. Never capture a wallpaper-only
    // image when macOS has denied access to other applications' windows.
    if unsafe { !CGPreflightScreenCaptureAccess() } {
        // The OS request may display a dialog or require a restart before returning.
        // Record the blocked user action before handing control to macOS.
        if let Err(error) = crate::tray::sync_permission_item(app, false, true) {
            eprintln!("권한 메뉴 갱신 실패: {error}");
        }
        if !unsafe { CGRequestScreenCaptureAccess() } {
            show_permission_guide(app)?;
            return Err("현재 앱의 화면 캡처 접근을 확인하지 못했습니다".into());
        }
        let (screen, access) = permission_status();
        if let Err(error) = crate::tray::sync_permission_item(app, screen && access, false) {
            eprintln!("권한 메뉴 갱신 실패: {error}");
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(guide) = app.get_webview_window("permission-guide") {
        guide.close().map_err(err)?;
    }
    if !app.state::<Captures>().0.lock().map_err(err)?.is_empty() {
        return Ok(());
    }
    // Capture every display before creating any overlays, so dimming never enters the PNG.
    #[cfg(windows)]
    visibility::set(app, generation, true)?;
    let mut screens = BTreeMap::new();
    let monitors = {
        let _timing = crate::telemetry::Timing::new("capture.screen.enumerate");
        xcap::Monitor::all().map_err(err)?
    };
    if monitors.is_empty() {
        return Err("캡처할 화면을 찾을 수 없습니다".into());
    }
    for (index, monitor) in monitors.into_iter().enumerate() {
        let scale = if cfg!(target_os = "macos") {
            1.0
        } else {
            f64::from(monitor.scale_factor().map_err(err)?)
        };
        screens.insert(
            format!("capture-{generation}-{index}"),
            Screen {
                generation,
                mode,
                ready: false,
                image: {
                    let _timing = crate::telemetry::Timing::new("capture.screen.capture");
                    monitor.capture_image().map_err(err)?
                },
                bounds: ImageNote {
                    x: f64::from(monitor.x().map_err(err)?),
                    y: f64::from(monitor.y().map_err(err)?),
                    width: f64::from(monitor.width().map_err(err)?) / scale,
                    height: f64::from(monitor.height().map_err(err)?) / scale,
                },
            },
        );
    }
    trace("screens captured");
    let bounds: Vec<_> = screens
        .iter()
        .map(|(label, s)| (label.clone(), s.bounds.clone()))
        .collect();
    *app.state::<Captures>().0.lock().map_err(err)? = screens;
    for (label, b) in bounds {
        let _timing = crate::telemetry::Timing::new("capture.overlay.create");
        crate::focus::remember(app, &label);
        let win = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App("capture.html".into()),
        )
        .title(match mode {
            SelectionMode::Image => "영역 캡처 — Escape로 취소",
            SelectionMode::Text => "빈 메모 영역 선택 — Escape로 취소",
            SelectionMode::Terminal => "터미널 영역 선택 — Escape로 취소",
            SelectionMode::Web => "웹 창 영역 선택 — Escape로 취소",
        })
        .decorations(false)
        .transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .accept_first_mouse(true)
        // A hidden WKWebView must finish decoding without waiting for activation.
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .inner_size(b.width, b.height)
        .build()
        .map_err(err)?;
        set_position(&win, b.x, b.y)?;
        trace("webview created");
        // Keep the window hidden until every display image has decoded.
    }
    Ok(())
}
fn selection_position(x: f64, y: f64) -> tauri::Position {
    #[cfg(target_os = "macos")]
    let position = tauri::Position::Logical(tauri::LogicalPosition::new(x, y));
    #[cfg(not(target_os = "macos"))]
    let position = tauri::Position::Physical(tauri::PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    ));
    position
}
fn set_position(win: &WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    win.set_position(selection_position(x, y)).map_err(err)
}
fn cleanup(app: &tauri::AppHandle) {
    let _timing = crate::telemetry::Timing::new("capture.cleanup.request");
    crate::telemetry::checkpoint("capture.cleanup.request");
    for win in crate::web::managed_windows(app)
        .values()
        .filter(|w| w.label().starts_with("capture-"))
    {
        // Remove the native surface before its WebView backing store is dismantled.
        let _ = win.hide();
        let _ = crate::stash::release(win);
        crate::focus::restore(win);
        let _ = win.destroy();
    }
    if let Ok(mut screens) = app.state::<Captures>().0.lock() {
        screens.clear();
    }
}
// destroy() only queues native destruction. WebView2 creation pumps window
// messages. A stall recurred after the first async change, so also order old
// overlay destruction before the next build. Destroyed listeners run before
// WebviewWrapper::drop; the worker queues the next creation event, which Tao
// handles after completing destruction. No UI callback waits on this barrier.
#[cfg(windows)]
fn cleanup_before_create(app: &tauri::AppHandle) -> Result<(), String> {
    let mut pending = Vec::new();
    for window in crate::web::managed_windows(app)
        .into_values()
        .filter(|w| w.label().starts_with("capture-"))
    {
        let (tx, rx) = std::sync::mpsc::channel();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = tx.send(());
            }
        });
        pending.push(rx);
    }
    cleanup(app);
    wait_for_cleanup(pending, std::time::Duration::from_secs(5))
}
#[cfg(any(windows, test))]
fn wait_for_cleanup(
    pending: Vec<std::sync::mpsc::Receiver<()>>,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let _timing = crate::telemetry::Timing::new("capture.cleanup.wait");
    crate::telemetry::checkpoint("capture.cleanup.wait");
    let deadline = std::time::Instant::now() + timeout;
    for rx in pending {
        rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .map_err(|_| "영역 선택 창 종료를 확인하지 못했습니다. 다시 시도하세요".to_string())?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn cleanup_before_create(app: &tauri::AppHandle) -> Result<(), String> {
    cleanup(app);
    Ok(())
}
fn cancel_selection(window: WebviewWindow) -> Result<(), String> {
    if !window.label().starts_with("capture-") {
        return Err("캡처 창이 아닙니다".into());
    }
    // Delayed cancel/failure from a prior generation must not erase a newer
    // selection. Every overlay label includes its generation.
    let selection = window
        .state::<Captures>()
        .0
        .lock()
        .map_err(err)?
        .get(window.label())
        .map(|screen| screen.generation);
    if let Some(generation) = selection {
        end_selection(window.app_handle(), generation)?;
    }
    Ok(())
}

fn end_selection(app: &tauri::AppHandle, generation: u64) -> Result<(), String> {
    let cleanup = cleanup_before_create(app);
    visibility::set(app, generation, false)?;
    cleanup
}
#[cfg(windows)]
#[tauri::command]
pub async fn cancel_capture(window: WebviewWindow) -> Result<(), String> {
    lifecycle("capture.cancel", move || cancel_selection(window)).await
}
#[cfg(not(windows))]
#[tauri::command]
pub fn cancel_capture(window: WebviewWindow) -> Result<(), String> {
    let _root = crate::telemetry::Timing::new("capture.cancel");
    cancel_selection(window)
}
#[tauri::command]
pub fn capture_ready(window: WebviewWindow) -> Result<(), String> {
    trace("image decoded");
    let app = window.app_handle();
    let labels = {
        let state = app.state::<Captures>();
        let mut screens = state.0.lock().map_err(err)?;
        screens
            .get_mut(window.label())
            .ok_or("캡처가 종료되었습니다")?
            .ready = true;
        if !screens.values().all(|s| s.ready) {
            return Ok(());
        }
        screens.keys().cloned().collect::<Vec<_>>()
    };
    let mut focus_target = None;
    for label in labels {
        let overlay = app.get_webview_window(&label).ok_or("캡처 창이 없습니다")?;
        #[cfg(target_os = "macos")]
        let under_pointer = {
            let ptr = overlay.ns_window().map_err(err)?;
            // This command runs on the main thread. Order the prepared window
            // regardless of app activation, then activate only once below.
            unsafe {
                let native = &*(ptr as *const objc2_app_kit::NSWindow);
                native.orderFrontRegardless();
                let frame = native.frame();
                let point = objc2_app_kit::NSEvent::mouseLocation();
                (frame.origin.x..frame.origin.x + frame.size.width).contains(&point.x)
                    && (frame.origin.y..frame.origin.y + frame.size.height).contains(&point.y)
            }
        };
        #[cfg(not(target_os = "macos"))]
        let under_pointer = {
            overlay.show().map_err(err)?;
            true
        };
        overlay
            .eval("window.dispatchEvent(new Event('capture-shown'))")
            .map_err(err)?;
        if focus_target.is_none() || under_pointer {
            focus_target = Some(overlay);
        }
    }
    trace("window ordered front");
    if let Some(overlay) = focus_target {
        #[cfg(target_os = "macos")]
        {
            let ptr = overlay.ns_window().map_err(err)?;
            // Activation must happen now, not on a subsequent input event.
            #[allow(deprecated)]
            objc2_app_kit::NSRunningApplication::currentApplication().activateWithOptions(
                objc2_app_kit::NSApplicationActivationOptions::ActivateIgnoringOtherApps,
            );
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).makeKeyAndOrderFront(None);
            }
        }
        #[cfg(not(target_os = "macos"))]
        overlay.set_focus().map_err(err)?;
        // A key native window can still route keys to its parent view. Send the
        // first Escape to the same WebView that would receive a pointer click.
        overlay.as_ref().set_focus().map_err(err)?;
    }
    #[cfg(windows)]
    {
        // Show/focus can recalculate invisible native borders. Align the final
        // WebView viewport with the source pixels before accepting selection.
        let screens = app
            .state::<Captures>()
            .0
            .lock()
            .map_err(err)?
            .iter()
            .map(|(label, s)| {
                (
                    label.clone(),
                    s.bounds.x,
                    s.bounds.y,
                    s.image.width(),
                    s.image.height(),
                )
            })
            .collect::<Vec<_>>();
        for (label, x, y, width, height) in screens {
            let overlay = app.get_webview_window(&label).ok_or("캡처 창이 없습니다")?;
            crate::windows_window::set_client_rect(
                &overlay,
                tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32),
                tauri::PhysicalSize::new(width, height),
            )?;
        }
    }
    Ok(())
}
fn failed_selection(window: WebviewWindow) -> Result<(), String> {
    cancel_selection(window.clone())?;
    if let Some(tray) = window.app_handle().tray_by_id("agent-gibbon") {
        let _ = tray.set_tooltip(Some("화면 이미지 로딩 실패. 캡처를 다시 실행하세요."));
    }
    Ok(())
}
#[cfg(windows)]
#[tauri::command]
pub async fn capture_failed(window: WebviewWindow) -> Result<(), String> {
    lifecycle("capture.failed", move || failed_selection(window)).await
}
#[cfg(not(windows))]
#[tauri::command]
pub fn capture_failed(window: WebviewWindow) -> Result<(), String> {
    let _root = crate::telemetry::Timing::new("capture.failed");
    failed_selection(window)
}
#[tauri::command]
pub fn capture_screen(window: WebviewWindow) -> Result<tauri::ipc::Response, String> {
    let _timing = crate::telemetry::Timing::new("capture.preview.copy");
    trace("preview requested");
    let state = window.state::<Captures>();
    let screens = state.0.lock().map_err(err)?;
    let screen = screens.get(window.label()).ok_or("캡처가 종료되었습니다")?;
    let bytes = preview_pixels(&screen.image);
    trace("preview pixels copied");
    Ok(tauri::ipc::Response::new(bytes))
}
// Width/height (little-endian u32) followed by lossless RGBA pixels. The
// selection canvas consumes these directly without PNG encoding or decoding.
fn preview_pixels(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + image.as_raw().len());
    bytes.extend_from_slice(&image.width().to_le_bytes());
    bytes.extend_from_slice(&image.height().to_le_bytes());
    bytes.extend_from_slice(image.as_raw());
    bytes
}
// CSS coordinates are converted using the actual snapshot dimensions (Retina included).
fn crop_rect(
    b: &ImageNote,
    pixels: (u32, u32),
    r: [f64; 4],
) -> Result<(u32, u32, u32, u32), String> {
    let [x, y, w, h] = r;
    if r.iter().any(|n| !n.is_finite())
        || x < 0.0
        || y < 0.0
        || w < 2.0
        || h < 2.0
        || x + w > b.width + 0.5
        || y + h > b.height + 0.5
    {
        return Err("화면 안에서 2px 이상의 영역을 선택하세요".into());
    }
    let sx = f64::from(pixels.0) / b.width;
    let sy = f64::from(pixels.1) / b.height;
    let left = (x * sx).floor() as u32;
    let top = (y * sy).floor() as u32;
    let right = ((x + w) * sx).ceil().min(f64::from(pixels.0)) as u32;
    let bottom = ((y + h) * sy).ceil().min(f64::from(pixels.1)) as u32;
    Ok((left, top, right - left, bottom - top))
}
#[cfg(windows)]
#[tauri::command]
pub async fn finish_capture(
    window: WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    lifecycle("capture.finish", move || {
        finish_selection(window, x, y, width, height)
    })
    .await
}
#[cfg(not(windows))]
#[tauri::command]
pub fn finish_capture(
    window: WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let _root = crate::telemetry::Timing::new("capture.finish");
    finish_selection(window, x, y, width, height)
}
fn finish_selection(
    window: WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let app = window.app_handle();
    // Window getters can wait for the UI thread on Windows. Do not hold the
    // capture state while waiting: capture_ready/cancel use it on that thread.
    let scale = window.scale_factor().map_err(err)?;
    let (bytes, bounds, mode, generation) = {
        let state = app.state::<Captures>();
        let screens = state.0.lock().map_err(err)?;
        let s = screens.get(window.label()).ok_or("캡처가 종료되었습니다")?;
        let (px, py, pw, ph) = {
            let _timing = crate::telemetry::Timing::new("capture.selection.validate");
            crop_rect(&s.bounds, s.image.dimensions(), [x, y, width, height])?
        };
        let offset = if cfg!(target_os = "macos") {
            1.0
        } else {
            scale
        };
        (
            if s.mode == SelectionMode::Image {
                let _timing = crate::telemetry::Timing::new("capture.image.encode");
                Some(png(
                    &image::imageops::crop_imm(&s.image, px, py, pw, ph).to_image()
                )?)
            } else {
                None
            },
            ImageNote {
                x: s.bounds.x + x * offset,
                y: s.bounds.y + y * offset,
                width,
                height,
            },
            s.mode,
            s.generation,
        )
    };
    // Restore the previous app before the new editor deliberately takes focus.
    if mode == SelectionMode::Image {
        let result = end_selection(app, generation).and_then(|()| {
            create_note(
                app,
                bytes.ok_or("캡처 이미지가 없습니다")?,
                bounds,
                Some(generation),
            )
            .map(|_| ())
        });
        if result.is_err() {
            visibility::set(app, generation, false)?;
        }
        return result;
    }
    end_selection(app, generation)?;
    if mode == SelectionMode::Web {
        return crate::web::create_region(app, bounds);
    }
    if mode == SelectionMode::Terminal {
        return crate::terminal::create_terminal(app, bounds).map(|_| ());
    }
    create_text_note(app, bounds, true).map(|_| ())
}
// Capture coordinates describe the image, not the enclosing toolbar window.
fn image_toolbar_below(image_y: i32, work_top: i32, scale: f64) -> bool {
    f64::from(image_y) - f64::from(work_top) < TOOLBAR_ROW_HEIGHT * scale
}
pub fn image_window_bounds(bounds: &ImageNote, position_scale: f64, below: bool) -> ImageNote {
    ImageNote {
        x: bounds.x,
        y: bounds.y
            - if below {
                0.0
            } else {
                TOOLBAR_ROW_HEIGHT * position_scale
            },
        width: bounds.width,
        height: bounds.height + TOOLBAR_ROW_HEIGHT,
    }
}
pub fn place(win: &WebviewWindow, bounds: &ImageNote) -> Result<(), String> {
    #[cfg(windows)]
    let position_scale = win.scale_factor().map_err(err)?;
    #[cfg(not(windows))]
    let position_scale = 1.0;
    // Work-area coordinates are physical; macOS capture origins are logical.
    let below = win
        .available_monitors()
        .map_err(err)?
        .iter()
        .any(|monitor| {
            let point =
                selection_position(bounds.x, bounds.y).to_physical::<i32>(monitor.scale_factor());
            let area = monitor.work_area();
            let display = monitor.position();
            let size = monitor.size();
            point.x >= display.x
                && point.y >= display.y
                && i64::from(point.x) < i64::from(display.x) + i64::from(size.width)
                && i64::from(point.y) < i64::from(display.y) + i64::from(size.height)
                && image_toolbar_below(point.y, area.position.y, monitor.scale_factor())
        });
    crate::sizing::set_image_toolbar_below(win, below)?;
    let bounds = image_window_bounds(bounds, position_scale, below);
    crate::sizing::manual_resize(win.clone(), win.state::<crate::sizing::Sizes>())?;
    win.set_min_size(Some(tauri::LogicalSize::new(2.0, 2.0)))
        .map_err(err)?;
    crate::sizing::constrain_image(win, bounds.width, bounds.height)?;
    set_position(win, bounds.x, bounds.y)?;
    #[cfg(windows)]
    {
        let scale = win.scale_factor().map_err(err)?;
        crate::windows_window::set_client_rect(
            win,
            tauri::PhysicalPosition::new(bounds.x.round() as i32, bounds.y.round() as i32),
            tauri::LogicalSize::new(bounds.width, bounds.height).to_physical::<u32>(scale),
        )?;
    }
    #[cfg(not(windows))]
    win.set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
        .map_err(err)?;
    set_position(win, bounds.x, bounds.y)
}
fn empty_text_note(edit_on_open: bool) -> crate::Note {
    crate::Note {
        title: "새 메모".into(),
        markdown: String::new(),
        html: String::new(),
        change_html: String::new(),
        edit_on_open,
        revision: 1,
        ..crate::Note::with_random_color()
    }
}
pub(crate) fn create_text_note(
    app: &tauri::AppHandle,
    bounds: ImageNote,
    edit_on_open: bool,
) -> Result<String, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(err)?
        .as_nanos();
    let id = format!("text-{stamp}");
    let note = empty_text_note(edit_on_open);
    app.state::<crate::storage::Store>().save(&id, &note)?;
    crate::first_frame::creating(
        app,
        &crate::label(&id),
        crate::first_frame::Presentation::Text { edit_on_open },
        || {
            let win = crate::create_window(app, &id, true)?;
            app.state::<crate::AppNotes>()
                .lock()
                .map_err(err)?
                .insert(id.clone(), note);
            crate::sizing::manual_resize(win.clone(), win.state::<crate::sizing::Sizes>())?;
            win.set_min_size(Some(tauri::LogicalSize::new(2.0, 2.0)))
                .map_err(err)?;
            set_position(&win, bounds.x, bounds.y)?;
            #[cfg(windows)]
            crate::windows_window::set_client_rect(
                &win,
                tauri::PhysicalPosition::new(bounds.x.round() as i32, bounds.y.round() as i32),
                tauri::LogicalSize::new(bounds.width, bounds.height)
                    .to_physical::<u32>(win.scale_factor().map_err(err)?),
            )?;
            #[cfg(not(windows))]
            win.set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
                .map_err(err)?;
            // Resizing a hidden NSWindow can shift its top edge. Reapply the
            // selected top-left after the final size, as image placement does.
            #[cfg(target_os = "macos")]
            set_position(&win, bounds.x, bounds.y)?;
            crate::first_frame::placed(&win)
        },
    )?;
    Ok(id)
}
pub(crate) fn show_text(win: &WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    crate::show_without_focus(win)?;
    #[cfg(not(windows))]
    win.show().map_err(err)?;
    win.set_focus().map_err(err)
}
pub(crate) fn create_note(
    app: &tauri::AppHandle,
    bytes: Vec<u8>,
    bounds: ImageNote,
    generation: Option<u64>,
) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(err)?
        .as_nanos();
    let id = format!("image-{stamp}");
    let note = crate::Note {
        capture: Some(bounds.clone()),
        title: "화면 캡처".into(),
        markdown: String::new(),
        html: String::new(),
        change_html: String::new(),
        revision: 1,
        ..crate::Note::with_random_color()
    };
    let store = app.state::<crate::storage::Store>();
    let path = store.path(&id).with_file_name("capture.png");
    let parent = path.parent().ok_or("이미지 저장 폴더가 없습니다")?;
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent).map_err(err)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let saved = (|| -> Result<(), String> {
        use std::io::Write;
        let mut file = options.open(&path).map_err(err)?;
        file.write_all(&bytes).map_err(err)?;
        file.sync_all().map_err(err)?;
        store.save(&id, &note)
    })();
    if let Err(error) = saved {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    let label = crate::label(&id);
    if let Some(generation) = generation {
        crate::layout::overview::await_capture_frame(app, generation, &label);
    }
    crate::first_frame::creating(app, &label, crate::first_frame::Presentation::Image, || {
        let win = crate::create_window(app, &id, true)?;
        app.state::<crate::AppNotes>()
            .lock()
            .map_err(err)?
            .insert(id.clone(), note);
        place(&win, &bounds)?;
        crate::first_frame::placed(&win)
    })?;
    Ok(id)
}
#[tauri::command]
pub fn note_image(window: WebviewWindow) -> Result<String, String> {
    Ok(data(&note_image_bytes(&window)?))
}

// Resolve only the caller's app-created capture; no frontend path is accepted.
pub(crate) fn note_image_bytes(window: &WebviewWindow) -> Result<Vec<u8>, String> {
    let id = window
        .label()
        .strip_prefix("note-")
        .ok_or("이미지 메모가 아닙니다")?;
    if window
        .state::<crate::AppNotes>()
        .lock()
        .map_err(err)?
        .get(id)
        .is_none_or(|n| n.capture.is_none())
    {
        return Err("이미지 메모가 아닙니다".into());
    }
    std::fs::read(
        window
            .state::<crate::storage::Store>()
            .path(id)
            .with_file_name("capture.png"),
    )
    .map_err(err)
}

#[tauri::command]
pub fn copy_note_image(window: WebviewWindow) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let bytes = note_image_bytes(&window)?;
    let image = tauri::image::Image::from_bytes(&bytes).map_err(err)?;
    window
        .app_handle()
        .clipboard()
        .write_image(&image)
        .map_err(err)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn lifecycle_errors_release_the_gate_without_dropping_next_cleanup() {
        let caller = std::thread::current().id();
        let failed = tauri::async_runtime::block_on(lifecycle("capture.failed", move || {
            assert_ne!(std::thread::current().id(), caller);
            Err("synthetic initialization failure".into())
        }));
        assert!(failed.is_err());
        let cleaned = std::sync::Arc::new(AtomicBool::new(false));
        let result = cleaned.clone();
        tauri::async_runtime::block_on(lifecycle("capture.cancel", move || {
            result.store(true, Ordering::Relaxed);
            Ok(())
        }))
        .unwrap();
        assert!(cleaned.load(Ordering::Relaxed));
    }
    #[test]
    fn transition_waits_for_every_destroyed_window() {
        use std::{sync::mpsc, time::Duration};
        let (first, first_done) = mpsc::sync_channel(0);
        let (second, second_done) = mpsc::sync_channel(0);
        let (created, result) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let completed = wait_for_cleanup(vec![first_done, second_done], Duration::from_secs(2));
            created.send(completed).unwrap();
        });
        first.send(()).unwrap();
        assert!(matches!(result.try_recv(), Err(mpsc::TryRecvError::Empty)));
        second.send(()).unwrap();
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
    }
    #[test]
    fn transition_does_not_continue_after_missing_destroy_acknowledgment() {
        use std::{sync::mpsc, time::Duration};
        let (tx, rx) = mpsc::channel();
        assert!(wait_for_cleanup(vec![rx], Duration::ZERO).is_err());
        drop(tx);
        let (tx, rx) = mpsc::channel();
        drop(tx);
        assert!(wait_for_cleanup(vec![rx], Duration::from_secs(2)).is_err());
    }
    #[test]
    fn selected_origin_keeps_desktop_coordinate_units() {
        let position = selection_position(-1599.6, 240.4);
        #[cfg(target_os = "macos")]
        assert!(matches!(position, tauri::Position::Logical(p) if p.x == -1599.6 && p.y == 240.4));
        #[cfg(not(target_os = "macos"))]
        assert!(matches!(position, tauri::Position::Physical(p) if p.x == -1600 && p.y == 240));
    }
    #[test]
    fn clipboard_png_decode_preserves_rgba_and_dimensions() {
        let pixels = vec![255, 0, 40, 255, 0, 200, 70, 128];
        let original = RgbaImage::from_raw(2, 1, pixels.clone()).unwrap();
        let encoded = png(&original).unwrap();
        let decoded = tauri::image::Image::from_bytes(&encoded).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 1));
        assert_eq!(decoded.rgba(), pixels);
        assert!(tauri::image::Image::from_bytes(b"invalid PNG").is_err());
    }
    #[test]
    fn text_selection_creates_empty_editable_note_without_image() {
        assert!(!empty_text_note(false).edit_on_open);
        let note = empty_text_note(true);
        assert!(note.capture.is_none());
        assert!(note.markdown.is_empty() && note.html.is_empty() && note.change_html.is_empty());
        assert!(note.edit_on_open);
        assert_eq!(note.revision, 1);
    }
    #[test]
    fn preview_preserves_dimensions_and_rgba_bytes() {
        let pixels = vec![255, 0, 1, 255, 2, 3, 254, 255];
        let image = RgbaImage::from_raw(2, 1, pixels.clone()).unwrap();
        let bytes = preview_pixels(&image);
        assert_eq!(&bytes[..8], &[2, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(&bytes[8..], pixels.as_slice());
    }
    #[test]
    fn retina_crop_and_invalid_selection() {
        let b = ImageNote {
            x: -100.0,
            y: 0.0,
            width: 100.0,
            height: 80.0,
        };
        assert_eq!(
            crop_rect(&b, (200, 160), [10.5, 20.0, 30.0, 40.0]).unwrap(),
            (21, 40, 60, 80)
        );
        for r in [
            [f64::NAN, 0.0, 2.0, 2.0],
            [-1.0, 0.0, 2.0, 2.0],
            [99.0, 0.0, 2.0, 2.0],
            [0.0, 0.0, 1.0, 2.0],
        ] {
            assert!(crop_rect(&b, (200, 160), r).is_err());
        }
    }
    #[test]
    fn capture_toolbar_expands_window_without_shrinking_or_moving_image() {
        for (x, y, width, height) in [(50.0, 80.0, 1600.0, 900.0), (-1200.0, -30.0, 320.0, 700.0)] {
            let image = ImageNote {
                x,
                y,
                width,
                height,
            };
            let frame = image_window_bounds(&image, 2.0, false);
            assert_eq!(frame.x, image.x);
            assert_eq!(frame.y + TOOLBAR_ROW_HEIGHT * 2.0, image.y);
            assert_eq!(frame.width, image.width);
            assert_eq!(frame.height - TOOLBAR_ROW_HEIGHT, image.height);
            let below = image_window_bounds(&image, 2.0, true);
            assert_eq!(below.y, image.y);
            assert_eq!(below.width, image.width);
            assert_eq!(below.height - TOOLBAR_ROW_HEIGHT, image.height);
        }
    }
    #[test]
    fn toolbar_uses_bottom_at_work_area_top_including_negative_and_scaled_monitors() {
        for (top, scale) in [(24, 1.0), (-900, 2.0), (0, 1.5)] {
            assert!(image_toolbar_below(top, top, scale));
            assert!(image_toolbar_below(
                top + (22.0 * scale) as i32 - 1,
                top,
                scale
            ));
            assert!(!image_toolbar_below(
                top + (22.0 * scale) as i32,
                top,
                scale
            ));
        }
    }
}
