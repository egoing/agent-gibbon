//! Wry activates NSApplication when attaching a new WKWebView. Preserve the
//! previous app until first show (after WebView startup), not just build return.
use std::{collections::BTreeMap, sync::Mutex};
use tauri::Manager;

#[derive(Default)]
pub struct Targets(Mutex<BTreeMap<String, i64>>);
fn frontmost() -> Option<i64> {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier() as i64)
    }
    #[cfg(windows)]
    {
        let handle = unsafe { GetForegroundWindow() };
        // Another note in this process is also a valid foreground target.
        (handle != 0).then_some(handle as i64)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    None
}
impl Targets {
    pub fn initial() -> Self {
        let targets = Self::default();
        targets.remember("main");
        targets
    }
    fn remember(&self, label: &str) {
        let previous = frontmost();
        #[cfg(target_os = "macos")]
        let previous = previous.filter(|pid| *pid != std::process::id() as i64);
        if let Some(pid) = previous {
            self.0.lock().unwrap().entry(label.into()).or_insert(pid);
        }
    }
}
pub fn remember(app: &tauri::AppHandle, label: &str) {
    app.state::<Targets>().remember(label);
}
#[cfg(any(target_os = "macos", windows))]
pub fn restore(window: &tauri::WebviewWindow) {
    let previous = window
        .state::<Targets>()
        .0
        .lock()
        .unwrap()
        .remove(window.label());
    #[cfg(target_os = "macos")]
    if let Some(pid) = previous {
        // Never override a user switch to a third app during construction.
        if frontmost() == Some(std::process::id() as i64) {
            if let Some(app) =
                objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(
                    pid as i32,
                )
            {
                app.activateWithOptions(objc2_app_kit::NSApplicationActivationOptions::empty());
            }
        }
    }
    #[cfg(windows)]
    if let Some(handle) = previous {
        let foreground = unsafe { GetForegroundWindow() };
        // Restore only if this particular construction stole focus. A user
        // switch to another note or another app must never be overridden.
        if window.hwnd().is_ok_and(|own| own.0 as isize == foreground) {
            unsafe {
                SetForegroundWindow(handle as isize);
            }
        }
    }
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(window: isize) -> i32;
}
