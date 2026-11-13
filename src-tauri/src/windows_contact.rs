//! Sample pointer shortcuts without depending on background WebView timers.
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
    pub shift: bool,
    pub windows: bool,
}

impl Modifiers {
    pub(crate) fn control_only(self) -> bool {
        self.control && !self.alt && !self.shift && !self.windows
    }

    pub(crate) fn alt_only(self) -> bool {
        self.alt && !self.control && !self.shift && !self.windows
    }

    pub(crate) fn stash_chord(self) -> bool {
        self.control && self.alt && self.windows
    }
}

pub(crate) fn modifiers() -> Modifiers {
    // The sign bit describes the current state. The low "recently pressed"
    // bit is shared with other processes and must not be used for rearming.
    // SAFETY: the function accepts virtual-key codes and no pointers.
    unsafe {
        Modifiers {
            control: GetAsyncKeyState(0x11) < 0, // Either Control key
            shift: GetAsyncKeyState(0x10) < 0,   // Either Shift key
            alt: GetAsyncKeyState(0x12) < 0,     // Either Alt key
            windows: GetAsyncKeyState(0x5b) < 0 || GetAsyncKeyState(0x5c) < 0,
        }
    }
}

pub fn install(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let pending = Arc::new(AtomicBool::new(false));
        let mut last_pointer = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(16));
            // Keep at most one callback pending while dialogs or native window
            // operations block the main loop. Read live input when it executes.
            if pending.swap(true, Ordering::AcqRel) {
                continue;
            }
            let finished = pending.clone();
            let handle = app.clone();
            let refresh_pointer = last_pointer.elapsed() >= Duration::from_millis(100);
            if refresh_pointer {
                last_pointer = Instant::now();
            }
            if app
                .run_on_main_thread(move || {
                    for window in crate::web::managed_windows(&handle).into_values() {
                        // Manager, terminals, feedback, capture overlays and the
                        // stash indicator retain their own input and opacity.
                        if !crate::warp::is_note_label(window.label()) {
                            continue;
                        }
                        if let Err(error) = crate::warp::poll_native(&window) {
                            eprintln!("포인터 워프 감지 실패: {error}");
                        }
                        if let Err(error) = crate::sizing::poll_fold(
                            window.clone(),
                            window.state::<crate::sizing::Sizes>(),
                        ) {
                            eprintln!("포인터 접기 감지 실패: {error}");
                        }
                        if refresh_pointer
                            && !crate::capture::active(&handle)
                            && !crate::stash::owns(&window)
                            && window.is_visible().unwrap_or(false)
                        {
                            if let Err(error) = crate::sync_opacity(window) {
                                eprintln!("메모 포인터 상태 동기화 실패: {error}");
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
    fn pointer_gestures_require_their_single_modifier() {
        for bits in 0..16 {
            let modifiers = Modifiers {
                control: bits & 1 != 0,
                alt: bits & 2 != 0,
                windows: bits & 4 != 0,
                shift: bits & 8 != 0,
            };
            assert_eq!(modifiers.control_only(), bits == 1, "mask {bits}");
            assert_eq!(modifiers.alt_only(), bits == 2, "mask {bits}");
        }
    }

    #[test]
    fn only_the_complete_three_modifier_chord_is_reserved_for_stash() {
        for bits in 0..8 {
            let modifiers = Modifiers {
                control: bits & 1 != 0,
                alt: bits & 2 != 0,
                windows: bits & 4 != 0,
                shift: false,
            };
            assert_eq!(modifiers.stash_chord(), bits == 7);
        }
    }
}
