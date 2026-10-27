//! Companion surface: never changes the memo's geometry or activates it on show.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, Manager, WebviewWindow};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Button {
    id: String,
    label: String,
    disabled: bool,
    #[serde(default)]
    primary: bool,
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct View {
    revision: Option<u64>,
    message: String,
    error: bool,
    buttons: Vec<Button>,
    color: String,
}
#[derive(Default)]
pub struct Surfaces(Mutex<BTreeMap<String, View>>);
pub fn is_surface(label: &str) -> bool {
    label.starts_with("feedback-")
}
fn label(owner: &str) -> String {
    format!("feedback-{owner}")
}
fn owner(window: &WebviewWindow) -> Result<WebviewWindow, String> {
    let name = window
        .label()
        .strip_prefix("feedback-")
        .ok_or("Feedback 창이 아닙니다")?;
    window
        .app_handle()
        .get_webview_window(name)
        .ok_or("메모 창이 없습니다".into())
}
fn memo(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" || window.label().starts_with("note-") {
        Ok(())
    } else {
        Err("메모 창에서만 사용할 수 있습니다".into())
    }
}

#[tauri::command]
pub async fn present_feedback(window: WebviewWindow, view: View) -> Result<(), String> {
    memo(&window)?;
    let show = !view.message.is_empty() || !view.buttons.is_empty();
    window
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), view);
    if show
        && window
            .app_handle()
            .get_webview_window(&label(window.label()))
            .is_none()
    {
        crate::focus::remember(window.app_handle(), &label(window.label()));
        // Async command avoids WebView2's synchronous creation deadlock.
        tauri::WebviewWindowBuilder::new(
            window.app_handle(),
            label(window.label()),
            tauri::WebviewUrl::App("feedback.html".into()),
        )
        .title(format!(
            "{} · Feedback",
            window.title().map_err(|e| e.to_string())?
        ))
        .inner_size(120., 32.)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .accept_first_mouse(true)
        .minimizable(false)
        .maximizable(false)
        .shadow(false)
        .parent(&window)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    }
    if !show {
        hide(&window);
    }
    Ok(())
}
#[tauri::command]
pub fn feedback_snapshot(window: WebviewWindow) -> Result<serde_json::Value, String> {
    let parent = owner(&window)?;
    let view = parent
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(parent.label())
        .cloned()
        .unwrap_or_default();
    let scale = parent.scale_factor().map_err(|e| e.to_string())?;
    let width = f64::from(parent.inner_size().map_err(|e| e.to_string())?.width) / scale;
    Ok(serde_json::json!({"view": view, "max_width": width}))
}

// Prefer below, then above. If neither fits, use the larger free strip with
// scrolling; at a full-height memo use an on-screen edge fallback.
fn geometry(
    p: (i32, i32),
    memo: (u32, u32),
    desired: (u32, u32),
    work: (i32, i32, u32, u32),
    gap: u32,
) -> (i32, i32, u32, u32) {
    let (left, top, sw, sh) = work;
    let right = i64::from(left) + i64::from(sw);
    let bottom = i64::from(top) + i64::from(sh);
    let w = desired.0.min(memo.0).min(sw).max(1);
    let below = (bottom - i64::from(p.1) - i64::from(memo.1) - i64::from(gap)).max(0) as u32;
    let above = (i64::from(p.1) - i64::from(top) - i64::from(gap)).max(0) as u32;
    let (y, h) = if desired.1 <= below {
        (
            i64::from(p.1) + i64::from(memo.1) + i64::from(gap),
            desired.1,
        )
    } else if desired.1 <= above {
        (
            i64::from(p.1) - i64::from(gap) - i64::from(desired.1),
            desired.1,
        )
    } else if above.max(below) >= 32 {
        if above > below {
            (i64::from(top), above)
        } else {
            (bottom - i64::from(below), below)
        }
    } else {
        (bottom - i64::from(desired.1.min(sh)), desired.1.min(sh))
    };
    let x = (i64::from(p.0) + i64::from(memo.0) - i64::from(w))
        .clamp(i64::from(left), right - i64::from(w));
    (
        x as i32,
        y.clamp(i64::from(top), bottom - i64::from(h)) as i32,
        w,
        h.max(1),
    )
}
#[tauri::command]
pub fn layout_feedback(
    window: WebviewWindow,
    width: f64,
    height: f64,
) -> Result<serde_json::Value, String> {
    let parent = owner(&window)?;
    if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
        return Err("잘못된 Feedback 크기".into());
    }
    let active = parent
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(parent.label())
        .is_some_and(|v| !v.message.is_empty() || !v.buttons.is_empty());
    if !active
        || !parent.is_visible().map_err(|e| e.to_string())?
        || parent.is_minimized().map_err(|e| e.to_string())?
        || crate::sizing::folded(&parent)
    {
        hide(&parent);
        return Ok(serde_json::json!({"edge":"none"}));
    }
    let scale = parent.scale_factor().map_err(|e| e.to_string())?;
    let p = parent.outer_position().map_err(|e| e.to_string())?;
    let size = parent.outer_size().map_err(|e| e.to_string())?;
    let monitor = parent
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("화면을 찾지 못했습니다")?;
    let work = monitor.work_area();
    let (x, y, w, h) = geometry(
        (p.x, p.y),
        (size.width, size.height),
        (
            (width * scale).ceil() as u32,
            (height * scale).ceil() as u32,
        ),
        (
            work.position.x,
            work.position.y,
            work.size.width,
            work.size.height,
        ),
        0,
    );
    let edge = if y == p.y + size.height as i32 {
        "below"
    } else if y + h as i32 == p.y {
        "above"
    } else {
        "none"
    };
    let connection = serde_json::json!({"owner":parent.label(), "edge":edge,
        "width":f64::from(w)/scale, "memoWidth":f64::from(size.width)/scale,
        "memoHeight":f64::from(size.height)/scale});
    parent
        .set_shadow(edge == "none")
        .map_err(|e| e.to_string())?;
    parent
        .emit_to(parent.label(), "feedback-connection", &connection)
        .map_err(|e| e.to_string())?;
    let target_size = tauri::PhysicalSize::new(w, h);
    if window.inner_size().map_err(|e| e.to_string())? != target_size {
        window.set_size(target_size).map_err(|e| e.to_string())?;
    }
    let target_position = tauri::PhysicalPosition::new(x, y);
    if window.outer_position().map_err(|e| e.to_string())? != target_position {
        window
            .set_position(target_position)
            .map_err(|e| e.to_string())?;
    }
    crate::sync_opacity(parent.clone())?;
    let pinned = parent.is_always_on_top().map_err(|e| e.to_string())?;
    if window.is_always_on_top().map_err(|e| e.to_string())? != pinned {
        window
            .set_always_on_top(pinned)
            .map_err(|e| e.to_string())?;
    }
    if !window.is_visible().map_err(|e| e.to_string())? {
        #[cfg(target_os = "macos")]
        {
            let ptr = window.ns_window().map_err(|e| e.to_string())?;
            // SAFETY: synchronous command runs on the main thread; Tauri owns NSWindow.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
            }
            crate::focus::restore(&window);
        }
        #[cfg(windows)]
        crate::focus::show_without_focus(&window)?;
        #[cfg(not(any(target_os = "macos", windows)))]
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(connection)
}
fn accepts_action(view: &View, action: &str, revision: Option<u64>) -> bool {
    (action != "acknowledge" || view.revision == revision)
        && view
            .buttons
            .iter()
            .any(|button| button.id == action && !button.disabled)
}

#[tauri::command]
pub fn feedback_action(
    window: WebviewWindow,
    action: String,
    revision: Option<u64>,
) -> Result<(), String> {
    let parent = owner(&window)?;
    if !parent.is_visible().map_err(|e| e.to_string())? || crate::sizing::folded(&parent) {
        return Ok(());
    }
    let allowed = parent
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(parent.label())
        .is_some_and(|v| accepts_action(v, &action, revision));
    if allowed {
        parent
            .emit_to(
                parent.label(),
                "feedback-action",
                serde_json::json!({"action": action, "revision": revision, "owner": parent.label()}),
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn hide(parent: &WebviewWindow) {
    let _ = parent.set_shadow(true);
    let _ = parent.emit_to(
        parent.label(),
        "feedback-connection",
        serde_json::json!({"owner":parent.label(),"edge":"none"}),
    );
    if let Some(window) = parent
        .app_handle()
        .get_webview_window(&label(parent.label()))
    {
        let _ = window.hide();
    }
}

// Treat the visible companion as part of the memo's hover surface.
#[cfg(target_os = "macos")]
pub fn pointer_over(parent: &WebviewWindow) -> bool {
    parent
        .app_handle()
        .get_webview_window(&label(parent.label()))
        .is_some_and(|child| {
            child.is_visible().unwrap_or(false) && crate::pointer_over(&child).unwrap_or(false)
        })
}

#[cfg(target_os = "macos")]
pub fn set_opacity(parent: &WebviewWindow, alpha: f64) -> Result<(), String> {
    if let Some(child) = parent
        .app_handle()
        .get_webview_window(&label(parent.label()))
    {
        let ptr = child.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: opacity synchronization runs on the main thread; Tauri owns NSWindow.
        unsafe {
            let native = &*(ptr as *const objc2_app_kit::NSWindow);
            if native.alphaValue() != alpha {
                native.setAlphaValue(alpha);
            }
        }
    }
    Ok(())
}

// Native move/resize events avoid visible lag while dragging or warping.
pub fn sync(parent: &WebviewWindow) {
    if let Some(child) = parent
        .app_handle()
        .get_webview_window(&label(parent.label()))
    {
        if let (Ok(size), Ok(scale)) = (child.inner_size(), child.scale_factor()) {
            let _ = layout_feedback(
                child,
                f64::from(size.width) / scale,
                f64::from(size.height) / scale,
            );
        }
    }
}

pub fn inspect(parent: &WebviewWindow) -> Result<serde_json::Value, String> {
    let Some(child) = parent
        .app_handle()
        .get_webview_window(&label(parent.label()))
    else {
        return Ok(serde_json::Value::Null);
    };
    let p = child.outer_position().map_err(|e| e.to_string())?;
    let s = child.inner_size().map_err(|e| e.to_string())?;
    let view = parent
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(parent.label())
        .cloned();
    let result = serde_json::json!({"visible": child.is_visible().map_err(|e| e.to_string())?, "focused": child.is_focused().map_err(|e| e.to_string())?, "always_on_top": child.is_always_on_top().map_err(|e| e.to_string())?, "position":{"x":p.x,"y":p.y},"size":{"width":s.width,"height":s.height},"view":view});
    #[cfg(target_os = "macos")]
    let result = {
        let mut result = result;
        let ptr = child.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: inspect runs synchronously on the main thread and borrows Tauri's live window.
        result["opacity"] =
            unsafe { serde_json::json!((&*(ptr as *const objc2_app_kit::NSWindow)).alphaValue()) };
        result["window_id"] = unsafe {
            serde_json::json!((&*(ptr as *const objc2_app_kit::NSWindow)).windowNumber())
        };
        result
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{accepts_action, geometry, Button, View};
    #[test]
    fn stale_confirmation_is_blocked_but_edit_actions_reach_conflict_check() {
        let mut view = View {
            revision: Some(3),
            buttons: vec![Button {
                id: "acknowledge".into(),
                ..Button::default()
            }],
            ..View::default()
        };
        assert!(!accepts_action(&view, "acknowledge", Some(2)));
        assert!(accepts_action(&view, "acknowledge", Some(3)));
        view.buttons[0].id = "save".into();
        assert!(accepts_action(&view, "save", Some(2)));
        view.buttons[0].disabled = true;
        assert!(!accepts_action(&view, "save", Some(2)));
    }
    #[test]
    fn placement_below_above_narrow_and_negative_monitor() {
        assert_eq!(
            geometry((100, 100), (300, 200), (90, 32), (0, 0, 800, 600), 4),
            (310, 304, 90, 32)
        );
        assert_eq!(
            geometry((100, 400), (300, 200), (90, 32), (0, 0, 800, 600), 4),
            (310, 364, 90, 32)
        );
        assert_eq!(
            geometry((-500, 100), (144, 200), (300, 60), (-800, 0, 800, 600), 4),
            (-500, 304, 144, 60)
        );
        let (_, y, _, h) = geometry((0, 0), (300, 600), (300, 80), (0, 0, 800, 600), 4);
        assert!(y >= 0 && y + h as i32 <= 600);
    }
}
