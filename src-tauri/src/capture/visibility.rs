//! Keep the app's existing surfaces out of selection and frozen screen pixels.
use std::sync::Mutex;
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub(super) struct Visibility(Mutex<Option<Hidden>>);
struct Hidden {
    generation: u64,
    windows: Vec<WebviewWindow>,
    #[cfg(not(target_os = "macos"))]
    frames: Vec<crate::placement::Rect>,
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
        let mut windows = Vec::new();
        #[cfg(not(target_os = "macos"))]
        let mut frames = Vec::new();
        for window in crate::web::managed_windows(app).into_values() {
            if !window.label().starts_with("capture-")
                && window.is_visible().map_err(|e| e.to_string())?
                && !window.is_minimized().map_err(|e| e.to_string())?
            {
                #[cfg(not(target_os = "macos"))]
                if crate::layout::arrangement_id(window.label()).is_some()
                    && !crate::stash::owns(&window)
                {
                    frames.push(crate::layout::window_rect(&window)?);
                }
                windows.push(window);
            }
        }
        // Save before the first mutation so a partial hide also restores safely.
        *state.0.lock().unwrap() = Some(Hidden {
            generation,
            windows: windows.clone(),
            #[cfg(not(target_os = "macos"))]
            frames,
        });
        crate::layout::overview::capture_visibility(app, generation, true)?;
        for window in windows {
            window.hide().map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        let hidden = state
            .0
            .lock()
            .unwrap()
            .take_if(|h| h.generation == generation);
        let mut failure = None;
        if let Some(hidden) = hidden {
            for window in hidden.windows {
                if matches!(window.label(), "memo-backdrop" | "memo-overview") {
                    continue;
                }
                if let Err(error) = crate::stash::show_frame(&window) {
                    failure.get_or_insert(error);
                }
            }
        }
        if let Err(error) = crate::layout::overview::capture_visibility(app, generation, false) {
            failure.get_or_insert(error);
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
