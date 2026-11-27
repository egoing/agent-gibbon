//! Keep the app's existing surfaces out of selection and frozen screen pixels.
use std::sync::Mutex;
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub(super) struct Visibility(Mutex<Option<Hidden>>);
struct Hidden {
    generation: u64,
    windows: Vec<Surface>,
    #[cfg(target_os = "macos")]
    order: super::window_order::WindowOrder,
    #[cfg(not(target_os = "macos"))]
    frames: Vec<crate::placement::Rect>,
}

struct Surface {
    window: WebviewWindow,
    #[cfg(target_os = "macos")]
    rank: usize,
    #[cfg(target_os = "macos")]
    was_key: bool,
}

#[cfg(not(target_os = "macos"))]
pub(super) fn note_frames(app: &tauri::AppHandle, generation: u64) -> Vec<crate::placement::Rect> {
    app.state::<Visibility>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .filter(|hidden| hidden.generation == generation)
        .map(|hidden| hidden.frames.clone())
        .unwrap_or_default()
}

pub(super) fn set(app: &tauri::AppHandle, generation: u64, hide: bool) -> Result<(), String> {
    let handle = app.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = sender.send(set_on_main(&handle, generation, hide));
    })
    .map_err(|e| e.to_string())?;
    receiver.recv().map_err(|e| e.to_string())?
}

fn set_on_main(app: &tauri::AppHandle, generation: u64, hide: bool) -> Result<(), String> {
    let state = app.state::<Visibility>();
    if hide {
        if let Some(existing) = state.0.lock().unwrap().as_ref() {
            return if existing.generation == generation {
                Ok(())
            } else {
                Err("다른 영역 선택이 진행 중입니다".into())
            };
        }
        let mut windows = Vec::new();
        #[cfg(target_os = "macos")]
        let order = super::window_order::WindowOrder::read()?;
        #[cfg(not(target_os = "macos"))]
        let mut frames = Vec::new();
        for window in crate::web::managed_windows(app).into_values() {
            if window.label().starts_with("capture-")
                || !window.is_visible().map_err(|e| e.to_string())?
                || window.is_minimized().map_err(|e| e.to_string())?
            {
                continue;
            }
            #[cfg(target_os = "macos")]
            let Some(rank) = order.position(&window)?
            else {
                // A visible NSWindow on a different Space is not on this desktop.
                continue;
            };
            #[cfg(not(target_os = "macos"))]
            if crate::layout::arrangement_id(window.label()).is_some()
                && !crate::stash::owns(&window)
            {
                frames.push(crate::layout::window_rect(&window)?);
            }
            windows.push(Surface {
                #[cfg(target_os = "macos")]
                rank,
                #[cfg(target_os = "macos")]
                was_key: super::window_order::native(&window)?.isKeyWindow()
                    && objc2_app_kit::NSRunningApplication::currentApplication().isActive(),
                window,
            });
        }
        #[cfg(target_os = "macos")]
        windows.sort_by_key(|surface| std::cmp::Reverse(surface.rank));
        let mut hidden = Hidden {
            generation,
            windows: Vec::with_capacity(windows.len()),
            #[cfg(target_os = "macos")]
            order,
            #[cfg(not(target_os = "macos"))]
            frames,
        };
        crate::layout::overview::pause_capture(app, generation);
        let result = windows.into_iter().try_for_each(|surface| {
            #[cfg(windows)]
            crate::windows_window::capture_visible(&surface.window, false)?;
            #[cfg(not(windows))]
            surface.window.hide().map_err(|e| e.to_string())?;
            hidden.windows.push(surface);
            Ok(())
        });
        // On partial failure the caller rolls back only completed hides.
        *state.0.lock().unwrap() = Some(hidden);
        result
    } else {
        let Some(hidden) = state
            .0
            .lock()
            .unwrap()
            .take_if(|h| h.generation == generation)
        else {
            return Ok(());
        };
        let resume_overview = crate::layout::overview::resume_capture(app, generation);
        let mut failure = None;
        #[cfg(target_os = "macos")]
        let mut visible = match super::window_order::WindowOrder::read() {
            Ok(order) => order.visible_ids(),
            Err(error) => {
                failure = Some(error);
                Default::default()
            }
        };
        for surface in hidden.windows {
            let window = &surface.window;
            if crate::layout::overview::is_surface(window.label()) && !resume_overview {
                continue;
            }
            #[cfg(target_os = "macos")]
            let restored =
                hidden
                    .order
                    .restore(window, surface.rank, surface.was_key, &mut visible);
            #[cfg(windows)]
            let restored = crate::windows_window::capture_visible(window, true);
            #[cfg(not(any(windows, target_os = "macos")))]
            let restored = crate::stash::show_frame(window);
            if let Err(error) = restored {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
