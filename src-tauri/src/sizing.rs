use std::{collections::BTreeMap, sync::Mutex};
use tauri::{LogicalSize, Manager, State, WebviewWindow};

#[derive(Default)]
struct SizeState {
    manual: bool,
    expanded: Option<LogicalSize<f64>>,
    contact: crate::warp::Contact,
    placed: bool,
    pending: bool,
}
#[derive(Default)]
pub struct Sizes(Mutex<BTreeMap<String, SizeState>>);

pub fn prepare(window: &WebviewWindow) -> Result<(), String> {
    window
        .state::<Sizes>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().into())
        .or_default()
        .pending = true;
    Ok(())
}
pub fn cancel(window: &WebviewWindow) {
    if let Ok(mut states) = window.state::<Sizes>().0.lock() {
        if let Some(state) = states.get_mut(window.label()) {
            state.pending = false;
        }
    }
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
                // Let native move/resize notifications settle before showing beside the cursor.
                std::thread::sleep(std::time::Duration::from_millis(20));
                let app = next.app_handle().clone();
                let _ = app.run_on_main_thread(move || {
                    let should_show = if let Ok(mut states) = next.state::<Sizes>().0.lock() {
                        let state = states.entry(next.label().into()).or_default();
                        std::mem::take(&mut state.pending)
                    } else {
                        false
                    };
                    if should_show {
                        if let Err(error) = crate::entrance::show(&next, Some(destination)) {
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
            .set_min_size(Some(LogicalSize::new(144.0, 32.0)))
            .map_err(|e| e.to_string())?;
        state.expanded = None;
    } else {
        let size = window
            .inner_size()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
        window
            .set_min_size(Some(LogicalSize::new(144.0, 12.0)))
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
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    state.contact.trigger(true, true, (cursor.x, cursor.y));
    toggle(&window, state)?;
    Ok(state.expanded.is_some())
}

#[tauri::command]
pub fn poll_fold(window: WebviewWindow, sizes: State<Sizes>) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    let control = objc2_app_kit::NSEvent::modifierFlags_class()
        .contains(objc2_app_kit::NSEventModifierFlags::Control);
    #[cfg(not(target_os = "macos"))]
    let control = false;
    let mut states = sizes.0.lock().map_err(|e| e.to_string())?;
    let state = states.entry(window.label().into()).or_default();
    if !window.is_visible().map_err(|e| e.to_string())? {
        state.contact = crate::warp::Contact::default();
        return Ok(state.expanded.is_some());
    }
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let s = window.outer_size().map_err(|e| e.to_string())?;
    let inside = cursor.x >= p.x as f64
        && cursor.y >= p.y as f64
        && cursor.x < p.x as f64 + s.width as f64
        && cursor.y < p.y as f64 + s.height as f64;
    if state.contact.trigger(control, inside, (cursor.x, cursor.y)) {
        toggle(&window, state)?;
    }
    Ok(state.expanded.is_some())
}
