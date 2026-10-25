use std::{collections::BTreeMap, sync::Mutex};
use tauri::{LogicalSize, Manager, State, WebviewWindow};

#[derive(Default)]
struct SizeState {
    manual: bool,
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
    sizes
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().into())
        .or_default()
        .manual = true;
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
    if state.manual {
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
