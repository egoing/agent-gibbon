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
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(handle, &mut pid);
        }
        (handle != 0 && pid != std::process::id()).then_some(handle as i64)
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
        if let Some(pid) = frontmost().filter(|pid| *pid != std::process::id() as i64) {
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
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(foreground, &mut pid);
        }
        if pid == std::process::id() {
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
    fn GetWindowThreadProcessId(window: isize, process: *mut u32) -> u32;
    fn SetForegroundWindow(window: isize) -> i32;
    fn ShowWindow(window: isize, command: i32) -> i32;
}

#[cfg(windows)]
pub fn show_without_focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    // SW_SHOWNOACTIVATE displays the note without making it the foreground window.
    unsafe {
        ShowWindow(handle.0 as isize, 4);
    }
    restore(window);
    Ok(())
}
