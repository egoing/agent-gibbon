//! App-level activation opens the singleton manager; passive windows opt out.
use std::{ffi::c_void, sync::OnceLock};
use tauri::Manager;

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

extern "C" {
    fn jajan_activation_begin_creation();
    fn jajan_activation_end_creation();
    fn jajan_activation_install(callback: extern "C" fn());
    fn jajan_activation_preserve_focus();
    fn jajan_activation_focus_content(window: *mut std::ffi::c_void);
    fn jajan_activation_focus_manager(window: *mut std::ffi::c_void);
}

// Interception is installed once; suppression is scoped to passive construction.
// Drop also releases the guard on every error path; nesting supports startup.
pub struct PassiveCreation;
pub fn passive_creation() -> PassiveCreation {
    // SAFETY: all callers build native windows on the AppKit main thread.
    unsafe { jajan_activation_begin_creation() };
    PassiveCreation
}
impl Drop for PassiveCreation {
    fn drop(&mut self) {
        // SAFETY: guards never leave the main thread or outlive their scope.
        unsafe { jajan_activation_end_creation() };
    }
}

pub fn install(app: &tauri::AppHandle) {
    if APP.set(app.clone()).is_ok() {
        // SAFETY: RunEvent::Ready runs on the AppKit main thread. The callback
        // and cloned handle live until process exit; native code calls on main.
        unsafe { jajan_activation_install(open_manager) };
    }
}

pub fn preserve_focus() {
    // SAFETY: focus transactions and native window operations run on main.
    unsafe { jajan_activation_preserve_focus() };
}

pub fn focus_content(window: &tauri::WebviewWindow) -> Result<(), String> {
    let pointer = window.ns_window().map_err(|e| e.to_string())?;
    // SAFETY: Tauri owns the live window; callers focus content on main only.
    unsafe { jajan_activation_focus_content(pointer) };
    Ok(())
}

pub fn focus_manager(window: &tauri::WebviewWindow) -> Result<(), String> {
    let pointer = window.ns_window().map_err(|e| e.to_string())?;
    // SAFETY: Tauri owns the NSWindow; the native completion uses a weak
    // reference and is canceled by input/deactivation. Called on main only.
    unsafe { jajan_activation_focus_manager(pointer) };
    Ok(())
}

// AppKit requests this snapshot on the main thread each time the Dock menu opens.
#[no_mangle]
extern "C" fn jajan_populate_dock_menu(menu: *mut c_void) {
    extern "C" {
        fn jajan_add_dock_window(
            menu: *mut c_void,
            window: *mut c_void,
            bytes: *const u8,
            len: usize,
        );
    }
    let Some(app) = APP.get() else { return };
    let add = |window: tauri::WebviewWindow, title: &str| {
        if crate::stash::owns(&window) {
            return;
        }
        if let Ok(pointer) = window.ns_window() {
            // The live menu, window and UTF-8 slice are borrowed synchronously.
            unsafe { jajan_add_dock_window(menu, pointer, title.as_ptr(), title.len()) }
        }
    };
    let notes = app.state::<crate::AppNotes>();
    if let Ok(notes) = notes.lock() {
        let titles: Vec<_> = notes
            .iter()
            .map(|(id, note)| {
                let title = if note.title.trim().is_empty() {
                    crate::language::tr(if note.capture.is_some() {
                        "화면 캡처"
                    } else {
                        "메모"
                    })
                } else {
                    &note.title
                };
                (crate::label(id), title.to_owned())
            })
            .collect();
        drop(notes);
        for (label, title) in titles {
            if let Some(window) = app.get_webview_window(&label) {
                add(window, &title);
            }
        }
    }
    for terminal in crate::terminal::entries(app) {
        if let Some(window) = app.get_webview_window(&terminal.id) {
            add(window, &terminal.display_title());
        }
    }
    for (window, title) in crate::web::dock_windows(app) {
        add(window, &title);
    }
}

extern "C" fn open_manager() {
    let Some(app) = APP.get() else {
        return;
    };
    // An overlay becoming key is part of the active selection. Direct manager
    // actions (Dock reopen, tray, onboarding) use their own entry points.
    if crate::capture::active(app) {
        return;
    }
    let Ok(_activity) = crate::updater::activity(app) else {
        return;
    };
    if let Err(error) = crate::manager::open(app) {
        eprintln!("앱 전환으로 관리 창을 열지 못했습니다: {error}");
    }
}
