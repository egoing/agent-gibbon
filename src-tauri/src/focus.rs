//! Wry activates NSApplication when attaching a new WKWebView. Preserve the
//! previous app until first show (after WebView startup), not just build return.
use std::{collections::BTreeMap, sync::Mutex};
use tauri::Manager;

#[derive(Default)]
pub struct Targets(Mutex<BTreeMap<String, i32>>);
fn frontmost() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier())
    }
    #[cfg(not(target_os = "macos"))]
    None
}
impl Targets {
    pub fn initial() -> Self {
        let targets = Self::default();
        targets.remember("main");
        targets
    }
    fn remember(&self, label: &str) {
        if let Some(pid) = frontmost().filter(|pid| *pid != std::process::id() as i32) {
            self.0.lock().unwrap().entry(label.into()).or_insert(pid);
        }
    }
}
pub fn remember(app: &tauri::AppHandle, label: &str) {
    app.state::<Targets>().remember(label);
}
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
        if frontmost() == Some(std::process::id() as i32) {
            if let Some(app) =
                objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            {
                app.activateWithOptions(objc2_app_kit::NSApplicationActivationOptions::empty());
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = previous;
}
