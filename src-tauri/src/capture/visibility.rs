//! Keep the app's existing surfaces out of selection and frozen screen pixels.
use std::sync::Mutex;
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub(super) struct Visibility(Mutex<Option<Hidden>>);
struct Hidden {
    generation: u64,
    windows: Vec<WebviewWindow>,
}

pub(super) fn set(app: &tauri::AppHandle, generation: u64, hide: bool) -> Result<(), String> {
    let handle = app.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = sender.send(set_on_main(&handle, generation, hide));
    })
    .map_err(|e| e.to_string())?;
    let ids = receiver.recv().map_err(|e| e.to_string())??;
    #[cfg(target_os = "macos")]
    if hide && !ids.is_empty() {
        // This function is called from the launch worker. Let AppKit process
        // ordering changes while WindowServer confirms their actual removal.
        unsafe extern "C" {
            fn gibbon_capture_windows_hidden(ids: *const i64, count: usize) -> bool;
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if unsafe { gibbon_capture_windows_hidden(ids.as_ptr(), ids.len()) } {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err("메모창이 화면에서 사라지는 것을 확인하지 못했습니다".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = ids;
    Ok(())
}

fn set_on_main(app: &tauri::AppHandle, generation: u64, hide: bool) -> Result<Vec<i64>, String> {
    let state = app.state::<Visibility>();
    if hide {
        let mut windows = Vec::new();
        #[cfg(target_os = "macos")]
        let mut ids = Vec::new();
        #[cfg(not(target_os = "macos"))]
        let ids = Vec::new();
        for window in crate::web::managed_windows(app).into_values() {
            if !window.label().starts_with("capture-")
                && window.is_visible().map_err(|e| e.to_string())?
                && !window.is_minimized().map_err(|e| e.to_string())?
            {
                #[cfg(target_os = "macos")]
                {
                    let ptr = window.ns_window().map_err(|e| e.to_string())?;
                    ids.push(unsafe {
                        (&*(ptr as *const objc2_app_kit::NSWindow)).windowNumber() as i64
                    });
                }
                windows.push(window);
            }
        }
        // Save before the first mutation so a partial hide also restores safely.
        *state.0.lock().unwrap() = Some(Hidden {
            generation,
            windows: windows.clone(),
        });
        crate::layout::overview::capture_visibility(app, generation, true)?;
        for window in windows {
            window.hide().map_err(|e| e.to_string())?;
        }
        Ok(ids)
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
            None => Ok(Vec::new()),
        }
    }
}
