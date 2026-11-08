//! Local-only information for the two trusted update controls.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Default)]
pub struct Editors(Mutex<BTreeSet<String>>);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Target {
    pub label: String,
    pub kind: String,
    pub title: String,
    pub pid: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Review {
    pub version: String,
    pub targets: Vec<Target>,
    pub restores_sessions: bool,
}

impl Review {
    pub fn can_confirm(&self) -> bool {
        self.restores_sessions
            && !self.targets.is_empty()
            && self
                .targets
                .iter()
                .all(|t| matches!(t.kind.as_str(), "web" | "terminal"))
    }
}

fn controls(window: &WebviewWindow) -> Result<(), String> {
    if matches!(window.label(), crate::manager::LABEL | "updater-notice") {
        Ok(())
    } else {
        Err("업데이트 안내와 설정에서만 실행할 수 있습니다".into())
    }
}

#[tauri::command]
pub fn updater_editor(window: WebviewWindow, editing: bool) -> Result<(), String> {
    let label = window.label();
    if label != "main" && !label.starts_with("note-") {
        return Err("메모 창에서만 편집 상태를 알릴 수 있습니다".into());
    }
    let editors = window.state::<Editors>();
    let mut labels = editors.0.lock().map_err(|e| e.to_string())?;
    if editing {
        labels.insert(label.into());
    } else {
        labels.remove(label);
    }
    Ok(())
}

pub fn current(app: &AppHandle, version: String) -> Result<Review, String> {
    let editors = app
        .state::<Editors>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let mut targets = Vec::new();
    for (label, window) in app.webview_windows() {
        let kind = if editors.contains(&label) {
            "editor"
        } else if label.starts_with("terminal-") {
            "terminal"
        } else if label.starts_with("webpage-") || label.starts_with("webview-") {
            "web"
        } else {
            continue;
        };
        let pid = if kind == "terminal" {
            crate::terminal::review_pid(app, &label)?
        } else {
            None
        };
        let title = window.title().map_err(|e| e.to_string())?;
        targets.push(Target {
            label,
            kind: kind.into(),
            title,
            pid,
        });
    }
    targets.sort_by(|a, b| a.label.cmp(&b.label));
    let restores_sessions = crate::updater::restores_sessions(app, &version)?;
    Ok(Review {
        version,
        targets,
        restores_sessions,
    })
}

#[tauri::command]
pub fn updater_show_target(window: WebviewWindow, label: String) -> Result<(), String> {
    controls(&window)?;
    let app = window.app_handle();
    let _activity = crate::updater::activity(app)?;
    if !current(app, String::new())?
        .targets
        .iter()
        .any(|t| t.label == label)
    {
        return Err("이 창은 더 이상 업데이트를 보류하지 않습니다".into());
    }
    let target = app.get_webview_window(&label).ok_or("창이 닫혔습니다")?;
    crate::stash::release(&target)?;
    target.unminimize().map_err(|e| e.to_string())?;
    target.show().map_err(|e| e.to_string())?;
    if label.starts_with("webpage-") || label.starts_with("webview-") {
        crate::web::geometry(&target);
    }
    target.set_focus().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_restart_never_overrides_an_editor_or_an_unknown_restore_contract() {
        let mut review = Review {
            version: "1.0.0".into(),
            restores_sessions: true,
            targets: vec![],
        };
        assert!(!review.can_confirm());
        review.targets.push(Target {
            label: "terminal-1".into(),
            title: "셸".into(),
            kind: "terminal".into(),
            pid: Some(42),
        });
        assert!(review.can_confirm());
        review.restores_sessions = false;
        assert!(!review.can_confirm());
        review.restores_sessions = true;
        review.targets.push(Target {
            label: "main".into(),
            title: "메모".into(),
            kind: "editor".into(),
            pid: None,
        });
        assert!(!review.can_confirm());
    }
}
