//! WindowServer includes other apps in front-to-back order. Restore relative to
//! a surviving window at the same level without activating the application.
use objc2_app_kit::{NSWindow, NSWindowOrderingMode};
use std::collections::HashSet;
use tauri::WebviewWindow;

type Entry = (u32, i32);

pub(super) struct WindowOrder(Vec<Entry>);

impl WindowOrder {
    pub fn read() -> Result<Self, String> {
        unsafe extern "C" {
            fn jajan_visit_window_order(
                context: *mut std::ffi::c_void,
                visit: extern "C" fn(*mut std::ffi::c_void, u32, i32),
            ) -> bool;
        }
        extern "C" fn visit(context: *mut std::ffi::c_void, id: u32, level: i32) {
            // The native visitor calls synchronously with our live Vec.
            unsafe { &mut *context.cast::<Vec<Entry>>() }.push((id, level));
        }
        let mut entries = Vec::<Entry>::new();
        let ok =
            unsafe { jajan_visit_window_order((&mut entries as *mut Vec<Entry>).cast(), visit) };
        ok.then_some(Self(entries))
            .ok_or_else(|| "화면 창 순서를 가져오지 못했습니다".into())
    }

    pub fn position(&self, window: &WebviewWindow) -> Result<Option<usize>, String> {
        let native = native(window)?;
        Ok(self
            .0
            .iter()
            .position(|&(id, _)| id == native.windowNumber() as u32))
    }

    pub fn visible_ids(&self) -> HashSet<u32> {
        self.0.iter().map(|&(id, _)| id).collect()
    }

    pub fn restore(
        &self,
        window: &WebviewWindow,
        index: usize,
        was_key: bool,
        visible: &mut HashSet<u32>,
    ) -> Result<(), String> {
        let native = native(window)?;
        let (id, level) = self.0[index];
        if native.windowNumber() as u32 != id || native.isMiniaturized() {
            return Ok(());
        }
        let available = |&&(other, layer): &&Entry| layer == level && visible.contains(&other);
        if let Some(&(above, _)) = self.0[..index].iter().rev().find(available) {
            native.orderWindow_relativeTo(NSWindowOrderingMode::Below, above as isize);
        } else if let Some(&(below, _)) = self.0[index + 1..].iter().find(available) {
            native.orderWindow_relativeTo(NSWindowOrderingMode::Above, below as isize);
        } else {
            native.orderBack(None);
        }
        if was_key && objc2_app_kit::NSRunningApplication::currentApplication().isActive() {
            native.makeKeyWindow();
        }
        visible.insert(id);
        Ok(())
    }
}

pub(super) fn native(window: &WebviewWindow) -> Result<&NSWindow, String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    // Tauri owns this window. Capture visibility is serialized on the UI thread.
    Ok(unsafe { &*ptr.cast::<NSWindow>() })
}
