//! Feedback state is local to the memo viewport; no companion native window.
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
impl View {
    fn active(&self, folded: bool) -> bool {
        !folded && (!self.message.is_empty() || !self.buttons.is_empty())
    }
}
#[derive(Default)]
pub struct Surfaces(Mutex<BTreeMap<String, View>>);
fn memo(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" || window.label().starts_with("note-") {
        Ok(())
    } else {
        Err("메모 창에서만 사용할 수 있습니다".into())
    }
}

#[tauri::command]
pub fn memo_frame_focus(window: WebviewWindow) -> Result<bool, String> {
    window.is_focused().map_err(|e| e.to_string())
}
pub fn sync_frame_focus(window: &WebviewWindow) {
    if let Ok(focused) = memo_frame_focus(window.clone()) {
        let _ = window.emit_to(window.label(), "memo-frame-focus", focused);
    }
}

// Store the rendered view for inspect/action validation. Never resize or focus.
#[tauri::command]
pub fn present_feedback(window: WebviewWindow, view: View) -> Result<(), String> {
    memo(&window)?;
    window
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), view);
    Ok(())
}
fn accepts_action(view: &View, action: &str, revision: Option<u64>) -> bool {
    (matches!(action, "export" | "save-shortcut")
        && (!view.message.is_empty() || !view.buttons.is_empty()))
        || (action != "acknowledge" || view.revision == revision)
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
    memo(&window)?;
    if !window.is_visible().map_err(|e| e.to_string())? || crate::sizing::folded(&window) {
        return Ok(());
    }
    let allowed = window
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(window.label())
        .is_some_and(|v| accepts_action(v, &action, revision));
    if allowed {
        window
            .emit_to(
                window.label(),
                "feedback-action",
                serde_json::json!({"action":action,"revision":revision,"owner":window.label()}),
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn destroy(window: &WebviewWindow) -> Result<(), String> {
    window
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    Ok(())
}
pub fn inspect(window: &WebviewWindow) -> Result<serde_json::Value, String> {
    let view = window
        .state::<Surfaces>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(window.label())
        .cloned();
    let Some(view) = view else {
        return Ok(serde_json::Value::Null);
    };
    let visible = !crate::stash::owns(window)
        && view.active(crate::sizing::folded(window))
        && window.is_visible().map_err(|e| e.to_string())?
        && !window.is_minimized().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"mode":"inline", "visible":visible,
        "focused":visible && window.is_focused().map_err(|e| e.to_string())?,
        "view":view}))
}

#[cfg(test)]
mod tests {
    #[test]
    fn folded_feedback_is_suppressed_even_for_legacy_tips_and_errors() {
        for error in [false, true] {
            let view: super::View = serde_json::from_value(serde_json::json!({
                "revision": null, "message": "안내", "error": error,
                "buttons": [{"id":"save","label":"저장","disabled":false}],
                "color":"#20252e", "folded_tip":true
            }))
            .unwrap();
            assert!(!view.active(true));
            assert!(view.active(false));
        }
        assert!(!super::View::default().active(false));
    }

    use super::{accepts_action, Button, View};
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
    fn shortcut_reaches_owner_without_trusting_stale_edit_buttons() {
        let mut view = View {
            message: "저장 완료".into(),
            ..View::default()
        };
        assert!(accepts_action(&view, "save-shortcut", None));
        view.message.clear();
        assert!(!accepts_action(&view, "save-shortcut", None));
        view.buttons.push(Button {
            id: "save".into(),
            disabled: true,
            ..Button::default()
        });
        // The owner checks live editing/saving/exporting before acting.
        assert!(accepts_action(&view, "save-shortcut", None));
        assert!(!accepts_action(&view, "save", None));
    }
}
