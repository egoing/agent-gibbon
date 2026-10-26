//! Sample native input independently of WebView timers, including unfocused notes.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::Manager;

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(key: i32) -> i16;
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub windows: bool,
}

impl Modifiers {
    pub(crate) fn close(self) -> bool {
        self.control && self.alt && self.windows
    }
}

pub(crate) fn modifiers() -> Modifiers {
    // Only the sign bit describes the current state. The low bit is shared
    // with other processes and cannot reliably identify a new key press.
    // SAFETY: GetAsyncKeyState accepts virtual key codes without pointers.
    unsafe {
        Modifiers {
            control: GetAsyncKeyState(0x11) < 0, // VK_CONTROL, either side
            alt: GetAsyncKeyState(0x12) < 0,     // VK_MENU, either side
            windows: GetAsyncKeyState(0x5b) < 0 || GetAsyncKeyState(0x5c) < 0,
        }
    }
}

pub fn install(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let pending = Arc::new(AtomicBool::new(false));
        let mut last_opacity = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(16));
            // Never accumulate main-thread work while native dialogs or window
            // operations block the event loop. Sample current input when run.
            if pending.swap(true, Ordering::AcqRel) {
                continue;
            }
            let finished = pending.clone();
            let handle = app.clone();
            let refresh_opacity = last_opacity.elapsed() >= Duration::from_millis(100);
            if refresh_opacity {
                last_opacity = Instant::now();
            }
            if app
                .run_on_main_thread(move || {
                    for window in handle.webview_windows().into_values() {
                        // Close must run before folding so the three-key chord
                        // preserves the note's existing size and position.
                        if let Err(error) = crate::warp::poll_native(&window) {
                            eprintln!("포인터 워프 감지 실패: {error}");
                        }
                        if let Err(error) = crate::sizing::poll_fold(
                            window.clone(),
                            window.state::<crate::sizing::Sizes>(),
                        ) {
                            eprintln!("포인터 접기 감지 실패: {error}");
                        }
                        // Background WebViews can throttle JavaScript timers.
                        // Keep hover alpha and native style recovery responsive
                        // without adding another thread or showing hidden notes.
                        if refresh_opacity && window.is_visible().unwrap_or(false) {
                            if let Err(error) = crate::sync_opacity(window) {
                                eprintln!("창 반투명도 동기화 실패: {error}");
                            }
                        }
                    }
                    finished.store(false, Ordering::Release);
                })
                .is_err()
            {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_requires_all_three_modifiers() {
        for bits in 0..8 {
            let modifiers = Modifiers {
                control: bits & 1 != 0,
                alt: bits & 2 != 0,
                windows: bits & 4 != 0,
            };
            assert_eq!(modifiers.close(), bits == 7);
        }
    }
}
