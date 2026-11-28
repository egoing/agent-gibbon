//! Frozen screen selection; only app-created PNGs are exposed to note windows.
mod visibility;
#[cfg(target_os = "macos")]
mod window_order;
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

pub(crate) const PERMISSION_GUIDE_LABEL: &str = "permission-guide";

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
    pixels: (u32, u32),
    #[cfg(not(target_os = "macos"))]
    monitor_id: u32,
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
            #[cfg(not(target_os = "macos"))]
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
        if let Some(tray) = app.tray_by_id("jajan") {
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
    if let Some(window) = app.get_webview_window(PERMISSION_GUIDE_LABEL) {
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
        PERMISSION_GUIDE_LABEL,
        tauri::WebviewUrl::App(if shortcut {
            "permission.html?shortcut=1".into()
        } else {
            "permission.html".into()
        }),
    )
    .title(crate::language::tr("Jajan 권한 설정"))
    .title_bar_style(tauri::TitleBarStyle::Transparent)
    .hidden_title(true)
    .background_color(tauri::window::Color(223, 226, 221, 255))
    .inner_size(600.0, 268.0)
    .min_inner_size(380.0, 200.0)
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

type PermissionGuideStatus = (bool, bool, String, bool, Option<String>);

#[tauri::command]
pub fn permission_guide(
    window: tauri::WebviewWindow,
    action: String,
) -> Result<PermissionGuideStatus, String> {
    if window.label() != PERMISSION_GUIDE_LABEL {
        return Err("권한 안내 창에서 실행하세요".into());
    }
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn jajan_show_permission_panel(screen: bool, overview: *mut std::ffi::c_void);
            fn jajan_hide_permission_panel();
            fn jajan_request_accessibility() -> bool;
        }
        let pane = match action.as_str() {
            "screen" => Some("Privacy_ScreenCapture"),
            "access" => Some("Privacy_Accessibility"),
            "status" | "observe-screen" | "observe-access" => None,
            _ => return Err("알 수 없는 동작입니다".into()),
        };
        if action == "access" {
            // The OS owns the prompt and trust decision for this process.
            unsafe {
                jajan_request_accessibility();
            }
        }
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
                            jajan_show_permission_panel(screen, pointer);
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
                .run_on_main_thread(|| unsafe { jajan_hide_permission_panel() })
                .map_err(err)?;
        }
        let executable = std::env::current_exe().map_err(err)?;
        // Settings registers the enclosing application bundle. The executable
        // path is diagnostic information, not the file-picker target.
        let application = executable
            .ancestors()
            .find(|path| {
                path.extension().is_some_and(|extension| extension == "app")
                    && path.join("Contents/Info.plist").is_file()
            })
            .map(|path| path.display().to_string());
        Ok((
            screen,
            access,
            executable.display().to_string(),
            crate::stash::input_owned(window.app_handle()),
            application,
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = action;
        Err("macOS에서 지원합니다".into())
    }
}

#[cfg(target_os = "macos")]
fn background_windows(
) -> Result<objc2_core_foundation::CFRetained<objc2_core_foundation::CFArray>, String> {
    use objc2_core_foundation::{CFArray, CFRetained};
    unsafe extern "C" {
        fn jajan_copy_background_windows() -> *mut CFArray;
    }
    std::ptr::NonNull::new(unsafe { jajan_copy_background_windows() })
        .map(|ptr| unsafe { CFRetained::from_raw(ptr) })
        .ok_or_else(|| "화면 창 목록을 가져오지 못했습니다".into())
}

#[cfg(target_os = "macos")]
// Match xcap's Quartz backend, including on pre-ScreenshotManager macOS.
#[allow(deprecated)]
fn background_image(
    windows: &objc2_core_foundation::CFArray,
    bounds: &ImageNote,
) -> Result<RgbaImage, String> {
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::{
        CGDataProvider, CGImage, CGWindowImageOption, CGWindowListCreateImageFromArray,
    };
    let area = CGRect::new(
        CGPoint::new(bounds.x, bounds.y),
        CGSize::new(bounds.width, bounds.height),
    );
    let image =
        unsafe { CGWindowListCreateImageFromArray(area, windows, CGWindowImageOption::Default) }
            .ok_or("화면 배경을 캡처하지 못했습니다")?;
    let width = CGImage::width(Some(&image));
    let height = CGImage::height(Some(&image));
    let stride = CGImage::bytes_per_row(Some(&image));
    let provider = CGImage::data_provider(Some(&image));
    let data = CGDataProvider::data(provider.as_deref()).ok_or("화면 픽셀이 없습니다")?;
    if width == 0
        || height == 0
        || CGImage::bits_per_pixel(Some(&image)) != 32
        || stride < width * 4
        || data.len() / stride < height
    {
        return Err("지원하지 않는 화면 픽셀 형식입니다".into());
    }
    unsafe extern "C" {
        fn jajan_copy_rgba(
            source: *const u8,
            destination: *mut u8,
            width: usize,
            height: usize,
            stride: usize,
        ) -> bool;
    }
    let mut pixels = vec![0; width * height * 4];
    // The provider copy is immutable and both buffers live through this synchronous conversion.
    if !unsafe { jajan_copy_rgba(data.byte_ptr(), pixels.as_mut_ptr(), width, height, stride) } {
        return Err("화면 픽셀을 변환하지 못했습니다".into());
    }
    RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| "화면 크기가 올바르지 않습니다".into())
}

fn begin(app: &tauri::AppHandle, mode: SelectionMode, generation: u64) -> Result<(), String> {
    let _update_activity = crate::updater::activity(app)?;
    #[cfg(not(windows))]
    let _root = crate::telemetry::Timing::new("capture.prepare");
    trace("begin");
    #[cfg(target_os = "macos")]
    // Called only by an explicit capture action. Never capture a wallpaper-only
    // image when macOS has denied access to other applications' windows.
    if mode == SelectionMode::Image && unsafe { !CGPreflightScreenCaptureAccess() } {
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
    if let Some(guide) = app.get_webview_window(PERMISSION_GUIDE_LABEL) {
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
    #[cfg(target_os = "macos")]
    let frames = crate::layout::visible_hosts(app)?
        .iter()
        .map(crate::layout::window_rect)
        .collect::<Result<Vec<_>, _>>()?;
    #[cfg(not(target_os = "macos"))]
    let frames = visibility::note_frames(app, generation);
    let mut displays = Vec::new();
    for (index, monitor) in monitors.into_iter().enumerate() {
        let scale = if cfg!(target_os = "macos") {
            1.0
        } else {
            f64::from(monitor.scale_factor().map_err(err)?)
        };
        let label = format!("capture-{generation}-{index}");
        let bounds = ImageNote {
            x: f64::from(monitor.x().map_err(err)?),
            y: f64::from(monitor.y().map_err(err)?),
            width: f64::from(monitor.width().map_err(err)?) / scale,
            height: f64::from(monitor.height().map_err(err)?) / scale,
        };
        // Keep full rectangles so a window spanning displays has no artificial seam.
        let outlines: Vec<_> = frames
            .iter()
            .map(|frame| ImageNote {
                x: (frame.x as f64 - bounds.x) / scale,
                y: (frame.y as f64 - bounds.y) / scale,
                width: frame.width as f64 / scale,
                height: frame.height as f64 / scale,
            })
            .filter(|r| {
                r.x < bounds.width
                    && r.y < bounds.height
                    && r.x + r.width > 0.0
                    && r.y + r.height > 0.0
            })
            .collect();
        displays.push((label.clone(), bounds.clone(), outlines));
        screens.insert(
            label,
            Screen {
                generation,
                mode,
                ready: false,
                pixels: (
                    monitor.width().map_err(err)?,
                    monitor.height().map_err(err)?,
                ),
                #[cfg(not(target_os = "macos"))]
                monitor_id: monitor.id().map_err(err)?,
                bounds,
            },
        );
    }
    trace("display bounds ready");
    *app.state::<Captures>().0.lock().map_err(err)? = screens;
    for (label, b, outlines) in displays {
        #[cfg(target_os = "macos")]
        let _passive = crate::app_activation::passive_creation();
        let _timing = crate::telemetry::Timing::new("capture.overlay.create");
        crate::focus::remember(app, &label);
        let win = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App("capture.html".into()),
        )
        .initialization_script(format!(
            "window.captureNoteFrames = {};",
            serde_json::to_string(&outlines).map_err(err)?
        ))
        .title(crate::language::tr(match mode {
            SelectionMode::Image => "영역 캡처",
            SelectionMode::Text => "빈 메모 영역 선택",
            SelectionMode::Terminal => "터미널 영역 선택",
            SelectionMode::Web => "웹 창 영역 선택",
        }))
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
    #[cfg(target_os = "macos")]
    {
        // Restore the desktop while the selection overlay still owns input.
        let restored = visibility::set(app, generation, false);
        cleanup_before_create(app)?;
        restored
    }
    #[cfg(not(target_os = "macos"))]
    {
        let cleanup = cleanup_before_create(app);
        visibility::set(app, generation, false)?;
        cleanup
    }
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
    trace("input ready");
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
    // Snapshot the desktop before any selection overlay changes its window order.
    #[cfg(target_os = "macos")]
    {
        let state = app.state::<Captures>();
        let generation = state
            .0
            .lock()
            .map_err(err)?
            .get(window.label())
            .ok_or("캡처가 종료되었습니다")?
            .generation;
        visibility::set(app, generation, true)?;
    }
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
        if focus_target.is_none() || under_pointer {
            focus_target = Some(overlay);
        }
    }
    trace("window ordered front");
    if let Some(overlay) = focus_target {
        #[cfg(target_os = "macos")]
        crate::app_activation::focus_content(&overlay)?;
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
                    s.pixels.0,
                    s.pixels.1,
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
    if let Some(tray) = window.app_handle().tray_by_id("jajan") {
        let _ = tray.set_tooltip(Some(crate::language::tr(
            "화면 이미지 로딩 실패. 캡처를 다시 실행하세요.",
        )));
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
    let result = create_from_selection(&window, x, y, width, height);
    if result.is_err() {
        // Failures before end_selection (for example a disconnected display)
        // still own a live selection. A stale overlay cannot cancel a newer one.
        if let Err(error) = cancel_selection(window) {
            eprintln!("영역 선택 실패 후 복원 오류: {error}");
        }
    }
    result
}

fn create_from_selection(
    window: &WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let app = window.app_handle();
    // Window getters can wait for the UI thread on Windows. Do not hold the
    // capture state while waiting: capture_ready/cancel use it on that thread.
    let scale = window.scale_factor().map_err(err)?;
    let (bounds, mode, generation, region) = {
        let state = app.state::<Captures>();
        let screens = state.0.lock().map_err(err)?;
        let s = screens.get(window.label()).ok_or("캡처가 종료되었습니다")?;
        let region = {
            let _timing = crate::telemetry::Timing::new("capture.selection.validate");
            crop_rect(&s.bounds, s.pixels, [x, y, width, height])?
        };
        let offset = if cfg!(target_os = "macos") {
            1.0
        } else {
            scale
        };
        (
            ImageNote {
                x: s.bounds.x + x * offset,
                y: s.bounds.y + y * offset,
                width,
                height,
            },
            s.mode,
            s.generation,
            region,
        )
    };
    // Restore the previous app before the new editor deliberately takes focus.
    if mode == SelectionMode::Image {
        // Keep original memo windows suspended until the selected pixels are acquired.
        let image = (|| {
            #[cfg(target_os = "macos")]
            {
                let _ = region;
                let windows = background_windows()?;
                background_image(&windows, &bounds)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let monitor_id = app
                    .state::<Captures>()
                    .0
                    .lock()
                    .map_err(err)?
                    .get(window.label())
                    .ok_or("캡처가 종료되었습니다")?
                    .monitor_id;
                let monitor = xcap::Monitor::all()
                    .map_err(err)?
                    .into_iter()
                    .find(|monitor| monitor.id().ok() == Some(monitor_id))
                    .ok_or("선택한 디스플레이가 분리되었습니다")?;
                cleanup_before_create(app)?;
                #[cfg(windows)]
                {
                    #[link(name = "dwmapi")]
                    unsafe extern "system" {
                        fn DwmFlush() -> i32;
                    }
                    if unsafe { DwmFlush() } < 0 {
                        return Err("선택창 숨김을 확인하지 못했습니다".into());
                    }
                }
                monitor
                    .capture_region(region.0, region.1, region.2, region.3)
                    .map_err(err)
            }
        })();
        end_selection(app, generation)?;
        return create_note(app, png(&image?)?, bounds).map(|_| ());
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
        title: crate::language::tr("새 메모").into(),
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
    #[cfg(target_os = "macos")]
    {
        crate::focus::discard_restore(win.app_handle(), win.label());
        crate::app_activation::focus_content(win)
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(windows)]
        crate::show_without_focus(win)?;
        #[cfg(not(windows))]
        win.show().map_err(err)?;
        win.set_focus().map_err(err)
    }
}

pub(crate) fn create_note(
    app: &tauri::AppHandle,
    bytes: Vec<u8>,
    bounds: ImageNote,
) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(err)?
        .as_nanos();
    let id = format!("image-{stamp}");
    let note = crate::Note {
        capture: Some(bounds.clone()),
        title: crate::language::tr("화면 캡처").into(),
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
