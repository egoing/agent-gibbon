//! Frozen screen selection; only app-created PNGs are exposed to note windows.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
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
}
struct Screen {
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
fn png(image: &RgbaImage) -> Result<Vec<u8>, String> {
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
        eprintln!("영역 노트 단축키 등록 실패 (메뉴에서 실행 가능): {error}");
    }
    Ok(())
}
static LAUNCH_QUEUED: AtomicBool = AtomicBool::new(false);

pub fn trigger(app: &tauri::AppHandle, mode: SelectionMode) {
    trace("trigger");
    if LAUNCH_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    // Leave the native shortcut/menu callback before building WebViews. Sending
    // from another thread wakes the event loop even when the mouse stays still.
    let app = app.clone();
    std::thread::spawn(move || {
        let handle = app.clone();
        if app
            .run_on_main_thread(move || {
                launch(&handle, mode);
                LAUNCH_QUEUED.store(false, Ordering::Release);
            })
            .is_err()
        {
            LAUNCH_QUEUED.store(false, Ordering::Release);
        }
    });
}
fn launch(app: &tauri::AppHandle, mode: SelectionMode) {
    if let Err(error) = begin(app, mode) {
        cleanup(app);
        if let Some(tray) = app.tray_by_id("agent-gibbon") {
            let _ = tray.set_tooltip(Some(format!(
                "화면 캡처 실패: {error}. 화면 기록 권한을 확인하세요."
            )));
        }
        eprintln!("화면 캡처 실패: {error}");
    }
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
fn begin(app: &tauri::AppHandle, mode: SelectionMode) -> Result<(), String> {
    trace("begin");
    #[cfg(target_os = "macos")]
    // Called only by an explicit capture action. Never capture a wallpaper-only
    // image when macOS has denied access to other applications' windows.
    if unsafe { !CGPreflightScreenCaptureAccess() && !CGRequestScreenCaptureAccess() } {
        return Err("시스템 설정 → 개인정보 보호 및 보안 → 화면 및 시스템 오디오 기록에서 이 앱을 허용한 뒤 다시 실행하세요".into());
    }
    if !app.state::<Captures>().0.lock().map_err(err)?.is_empty() {
        return Ok(());
    }
    // Capture every display before creating any overlays, so dimming never enters the PNG.
    let mut screens = BTreeMap::new();
    for (index, monitor) in xcap::Monitor::all().map_err(err)?.into_iter().enumerate() {
        let scale = if cfg!(target_os = "macos") {
            1.0
        } else {
            f64::from(monitor.scale_factor().map_err(err)?)
        };
        screens.insert(
            format!("capture-{index}"),
            Screen {
                mode,
                ready: false,
                image: monitor.capture_image().map_err(err)?,
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
        crate::focus::remember(app, &label);
        let win = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App("capture.html".into()),
        )
        .title(match mode {
            SelectionMode::Image => "영역 캡처 — Escape로 취소",
            SelectionMode::Text => "빈 노트 영역 선택 — Escape로 취소",
        })
        .decorations(false)
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
fn set_position(win: &WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let position = tauri::Position::Logical(tauri::LogicalPosition::new(x, y));
    #[cfg(not(target_os = "macos"))]
    let position = tauri::Position::Physical(tauri::PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    ));
    win.set_position(position).map_err(err)
}
fn cleanup(app: &tauri::AppHandle) {
    for win in app
        .webview_windows()
        .values()
        .filter(|w| w.label().starts_with("capture-"))
    {
        crate::focus::restore(win);
        let _ = win.destroy();
    }
    if let Ok(mut screens) = app.state::<Captures>().0.lock() {
        screens.clear();
    }
}
#[tauri::command]
pub fn cancel_capture(window: WebviewWindow) -> Result<(), String> {
    if !window.label().starts_with("capture-") {
        return Err("캡처 창이 아닙니다".into());
    }
    cleanup(window.app_handle());
    Ok(())
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
        {
            let ptr = overlay.ns_window().map_err(err)?;
            // This command runs on the main thread. Order the prepared window
            // regardless of app activation, then activate only once below.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
            }
        }
        #[cfg(not(target_os = "macos"))]
        overlay.show().map_err(err)?;
        focus_target = Some(overlay);
    }
    trace("window ordered front");
    if let Some(overlay) = focus_target {
        #[cfg(target_os = "macos")]
        {
            let ptr = overlay.ns_window().map_err(err)?;
            // Activation must happen now, not on a subsequent input event.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).makeKeyAndOrderFront(None);
            }
            #[allow(deprecated)]
            objc2_app_kit::NSRunningApplication::currentApplication().activateWithOptions(
                objc2_app_kit::NSApplicationActivationOptions::ActivateIgnoringOtherApps,
            );
        }
        #[cfg(not(target_os = "macos"))]
        overlay.set_focus().map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub fn capture_failed(window: WebviewWindow) -> Result<(), String> {
    cancel_capture(window.clone())?;
    if let Some(tray) = window.app_handle().tray_by_id("agent-gibbon") {
        let _ = tray.set_tooltip(Some("화면 이미지 로딩 실패. 캡처를 다시 실행하세요."));
    }
    Ok(())
}
#[tauri::command]
pub fn capture_screen(window: WebviewWindow) -> Result<tauri::ipc::Response, String> {
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
#[tauri::command]
pub fn finish_capture(
    window: WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let app = window.app_handle();
    let (bytes, bounds) = {
        let state = app.state::<Captures>();
        let screens = state.0.lock().map_err(err)?;
        let s = screens.get(window.label()).ok_or("캡처가 종료되었습니다")?;
        let (px, py, pw, ph) = crop_rect(&s.bounds, s.image.dimensions(), [x, y, width, height])?;
        let scale = window.scale_factor().map_err(err)?;
        let offset = if cfg!(target_os = "macos") {
            1.0
        } else {
            scale
        };
        (
            if s.mode == SelectionMode::Image {
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
        )
    };
    // Restore the previous app before the new editor deliberately takes focus.
    cleanup(app);
    match bytes {
        Some(bytes) => create_note(app, bytes, bounds),
        None => create_text_note(app, bounds),
    }
}
pub fn place(win: &WebviewWindow, bounds: &ImageNote) -> Result<(), String> {
    crate::sizing::manual_resize(win.clone(), win.state::<crate::sizing::Sizes>())?;
    win.set_min_size(Some(tauri::LogicalSize::new(2.0, 2.0)))
        .map_err(err)?;
    crate::sizing::constrain_image(win, bounds.width, bounds.height)?;
    set_position(win, bounds.x, bounds.y)?;
    win.set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
        .map_err(err)
}
fn empty_text_note() -> crate::Note {
    crate::Note {
        title: "새 노트".into(),
        markdown: String::new(),
        html: String::new(),
        change_html: String::new(),
        edit_on_open: true,
        revision: 1,
        ..crate::Note::default()
    }
}
fn create_text_note(app: &tauri::AppHandle, bounds: ImageNote) -> Result<(), String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(err)?
        .as_nanos();
    let id = format!("text-{stamp}");
    let note = empty_text_note();
    app.state::<crate::storage::Store>().save(&id, &note)?;
    let win = crate::create_window(app, &id)?;
    app.state::<crate::AppNotes>()
        .lock()
        .map_err(err)?
        .insert(id, note);
    crate::sizing::manual_resize(win.clone(), win.state::<crate::sizing::Sizes>())?;
    win.set_min_size(Some(tauri::LogicalSize::new(2.0, 2.0)))
        .map_err(err)?;
    set_position(&win, bounds.x, bounds.y)?;
    win.set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
        .map_err(err)?;
    win.show().map_err(err)?;
    win.set_focus().map_err(err)?;
    crate::tray::refresh(app)
}
fn create_note(app: &tauri::AppHandle, bytes: Vec<u8>, bounds: ImageNote) -> Result<(), String> {
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
        ..crate::Note::default()
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
    let win = crate::create_window(app, &id)?;
    app.state::<crate::AppNotes>()
        .lock()
        .map_err(err)?
        .insert(id.clone(), note);
    place(&win, &bounds)?;
    crate::show_without_focus(&win)?;
    crate::tray::refresh(app)
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
        let note = empty_text_note();
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
}
