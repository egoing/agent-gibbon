//! Manager selection is read-only; explicit actions use existing window behavior.
use crate::AppNotes;
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::Manager;

pub const LABEL: &str = "manager";

pub fn open(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        crate::stash::release(&window)?;
        window.unminimize().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        return focus(&window);
    }
    let builder =
        tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("manager.html".into()))
            .title(format!(
                "{} — 메모 관리",
                crate::instance::current().title()
            ))
            .transparent(false)
            .inner_size(944.0, 480.0)
            .min_inner_size(600.0, 360.0);
    // Preserve native traffic lights, dragging and sizing; let the window's
    // background show through instead of a separate system-colored strip.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Transparent)
        .hidden_title(true)
        .background_color(tauri::window::Color(223, 226, 221, 255));
    let window = builder.build().map_err(|e| e.to_string())?;
    focus(&window)
}

fn focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    crate::app_activation::focus_manager(window)?;
    // macOS deminiaturization is animated. Tao's set_focus is a no-op while
    // isMiniaturized is still true, even after unminimize/show were requested.
    // The native completion above is canceled by input or app deactivation;
    // there is no timer retry that could steal focus after switching away.
    #[cfg(target_os = "macos")]
    objc2_app_kit::NSRunningApplication::currentApplication()
        .activateWithOptions(objc2_app_kit::NSApplicationActivationOptions::empty());
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagerTheme {
    White,
    Gray,
    Black,
}

// Cosmetic only, including before onboarding. No arbitrary window, color,
// app-wide appearance, memo state or persisted consent crosses this command.
#[tauri::command]
pub fn manager_set_theme(window: tauri::WebviewWindow, theme: ManagerTheme) -> Result<(), String> {
    if window.label() != LABEL {
        return Err("관리 창에서만 테마를 변경할 수 있습니다".into());
    }
    #[cfg(target_os = "macos")]
    {
        let color = match theme {
            ManagerTheme::White => tauri::window::Color(248, 248, 248, 255),
            ManagerTheme::Gray => tauri::window::Color(223, 226, 221, 255),
            ManagerTheme::Black => tauri::window::Color(38, 38, 39, 255),
        };
        // Use the native Window layer; macOS WebView background setters are
        // unsupported. Do not use set_theme, which is app-wide on macOS.
        window
            .as_ref()
            .window()
            .set_background_color(Some(color))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = theme;
    Ok(())
}

// More always targets the full list even when Settings was previously selected.
pub fn open_list(app: &tauri::AppHandle) -> Result<(), String> {
    open(app)?;
    if let Some(window) = app.get_webview_window(LABEL) {
        window
            .eval("window.jajanShowNotes?.()")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// A digest of rendered HTML also distinguishes reused IDs and revisions after reload.
fn note_row(id: &str, note: &crate::model::Note, known: Option<&HashMap<String, String>>) -> Value {
    let mut row = json!({"id":id, "title":display_title(note,id), "sourceTitle":note.title, "capture":note.capture});
    if note.capture.is_none() {
        use sha2::{Digest, Sha256};
        let token = note
            .manager_body_token
            .get_or_init(|| format!("{:x}", Sha256::digest(note.html.as_bytes())));
        row["bodyToken"] = json!(token);
        if known.and_then(|map| map.get(id)) != Some(token) {
            row["html"] = json!(note.html);
        }
    } else {
        row["html"] = json!(note.html);
        row["revision"] = json!(note.revision.to_string());
    }
    row
}

// Captures are created once per ID. Also notice normal on-disk replacements
// without reading or base64-encoding image bytes on every poll.
fn image_token(path: &std::path::Path) -> Option<String> {
    let metadata = path.metadata().ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let stamp = format!("{}-{}", metadata.len(), modified.as_nanos());
    #[cfg(unix)]
    let stamp = {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{stamp}-{}-{}-{}",
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec()
        )
    };
    Some(stamp)
}

#[tauri::command]
pub fn manager_notes(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppNotes>,
    known_bodies: Option<HashMap<String, String>>,
) -> Result<Value, String> {
    if window.label() != LABEL {
        return Err("관리 창에서만 조회할 수 있습니다".into());
    }
    let notes = state.lock().map_err(|e| e.to_string())?;
    let mut rows: Vec<_> = notes
        .iter()
        .map(|(id, note)| note_row(id, note, known_bodies.as_ref()))
        .collect();
    drop(notes);
    let app = window.app_handle();
    // Native window calls must run after releasing the Notes lock.
    for row in &mut rows {
        let id = row["id"].as_str().unwrap();
        if !row["capture"].is_null() {
            let path = app
                .state::<crate::storage::Store>()
                .path(id)
                .with_file_name("capture.png");
            row["imageToken"] = json!(image_token(&path));
        }
        row["visible"] = json!(visible(app, &crate::label(row["id"].as_str().unwrap()))?);
    }
    for agent in crate::terminal::entries(window.app_handle()) {
        rows.push(json!({"id":agent.id,"title":crate::presentation::list_title(&agent.title, &agent.id, crate::presentation::Kind::Terminal),"sourceTitle":agent.title,"kind":agent.kind,"visible":visible(app, &agent.id)?}));
    }
    rows.extend(crate::web::manager_entries(app)?);
    Ok(Value::Array(rows))
}

#[tauri::command]
pub fn manager_image(window: tauri::WebviewWindow, id: String) -> Result<String, String> {
    use base64::Engine;
    if window.label() != LABEL {
        return Err("관리 창에서만 조회할 수 있습니다".into());
    }
    let state = window.state::<AppNotes>();
    let notes = state.lock().map_err(|e| e.to_string())?;
    if notes.get(&id).is_none_or(|n| n.capture.is_none()) {
        return Err("이미지 메모가 아닙니다".into());
    }
    let path = window
        .state::<crate::storage::Store>()
        .path(&id)
        .with_file_name("capture.png");
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

// First visible Markdown text block, without heading/list/emphasis syntax.
fn display_title(note: &crate::model::Note, id: &str) -> String {
    use pulldown_cmark::{Event, Parser, TagEnd};
    let mut text = String::new();
    if note.capture.is_none() {
        for event in Parser::new(&note.markdown) {
            match event {
                Event::Text(value) | Event::Code(value) => text.push_str(&value),
                Event::SoftBreak
                | Event::HardBreak
                | Event::End(
                    TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock,
                ) if !text.trim().is_empty() => break,
                _ => {}
            }
        }
    }
    let raw = if text.trim().is_empty() {
        &note.title
    } else {
        &text
    };
    crate::presentation::list_title(
        raw,
        id,
        if note.capture.is_some() {
            crate::presentation::Kind::Capture
        } else {
            crate::presentation::Kind::Note
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_rows_only_omit_the_exact_rendered_body() {
        let mut note = crate::model::Note::default();
        let full = note_row("same-id", &note, None);
        let known = HashMap::from([("same-id".into(), full["bodyToken"].as_str().unwrap().into())]);
        assert_eq!(full["html"], note.html);
        assert!(note_row("same-id", &note, Some(&known))
            .get("html")
            .is_none());
        assert_eq!(note_row("new-id", &note, Some(&known))["html"], note.html);

        let update =
            serde_json::from_value(json!({"action":"update","markdown":"새 본문"})).unwrap();
        note.apply(&update).unwrap();
        // Disk restoration or recreation can reuse a revision and ID.
        note.revision = 0;
        let changed = note_row("same-id", &note, Some(&known));
        assert_ne!(changed["bodyToken"], full["bodyToken"]);
        assert_eq!(changed["html"], note.html);
        assert_eq!(changed["title"], "새 본문");
        assert_eq!(changed["sourceTitle"], note.title);
    }

    #[test]
    fn capture_file_replacement_changes_image_stamp() {
        let path = std::env::temp_dir().join(format!("jajan-manager-image-{}", std::process::id()));
        std::fs::write(&path, b"one").unwrap();
        let old = image_token(&path).unwrap();
        std::fs::write(&path, b"another").unwrap();
        assert_ne!(image_token(&path).unwrap(), old);
        std::fs::remove_file(&path).unwrap();
        assert!(image_token(&path).is_none());
    }

    #[test]
    fn first_text_ignores_markdown_and_blank_lines() {
        let mut note = crate::model::Note {
            title: "메타 제목".into(),
            ..Default::default()
        };
        note.markdown = "\n\n# 첫 제목\n\n두 번째".into();
        assert_eq!(display_title(&note, "id"), "첫 제목");
        note.markdown = "- 첫 항목\n- 다음 항목".into();
        assert_eq!(display_title(&note, "id"), "첫 항목");
        note.markdown = "**첫** 텍스트와 `코드`".into();
        assert_eq!(display_title(&note, "id"), "첫 텍스트와 코드");
        note.markdown = "".into();
        assert_eq!(display_title(&note, "id"), "메타 제목");
        note.title.clear();
        assert_eq!(display_title(&note, "id"), "메모");
    }
}

#[derive(Clone, Copy, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RowKind {
    Note,
    Terminal,
    Web,
}

// A closed memo/terminal needs a new WebView; never build it from WebView2's
// synchronous IPC callback on Windows.
#[cfg_attr(windows, tauri::command(async))]
#[cfg_attr(not(windows), tauri::command)]
pub fn manager_set_visibility(
    window: tauri::WebviewWindow,
    id: String,
    visible: bool,
    kind: Option<RowKind>,
    session: Option<String>,
) -> Result<bool, String> {
    if window.label() != LABEL {
        return Err("관리 창에서만 사용할 수 있습니다".into());
    }
    let app = window.app_handle();
    // Serialize the live lookup and action with native close/destroy and CLI
    // opens. On Windows, post after the synchronous WebView IPC has returned.
    #[cfg(windows)]
    {
        let handle = app.clone();
        let (send, receive) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = send.send(set_visibility(
                &handle,
                &id,
                visible,
                kind,
                session.as_deref(),
            ));
        })
        .map_err(|e| e.to_string())?;
        receive.recv().map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    set_visibility(app, &id, visible, kind, session.as_deref())
}

fn set_visibility(
    app: &tauri::AppHandle,
    id: &str,
    visible: bool,
    kind: Option<RowKind>,
    session: Option<&str>,
) -> Result<bool, String> {
    if kind == Some(RowKind::Web) {
        return crate::web::set_manager_visibility(
            app,
            id,
            visible,
            session.ok_or("웹 세션을 확인할 수 없습니다")?,
        );
    }
    let exists = app
        .state::<AppNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(id);
    // Never create a blank memo when a stale selection has disappeared.
    let is_terminal = crate::terminal::contains(app, id);
    if !exists && !is_terminal {
        return Err("메모가 없습니다".into());
    }
    let actual = if is_terminal {
        RowKind::Terminal
    } else {
        RowKind::Note
    };
    if kind.is_some_and(|expected| expected != actual) || crate::web::contains(app, id) {
        return Err("메모 종류가 변경되었습니다".into());
    }
    let label = if is_terminal {
        id.to_owned()
    } else {
        crate::label(id)
    };
    if visible {
        crate::focus::discard_restore(app, &label);
    }
    let response = crate::apply_with_presentation(
        app,
        crate::Request {
            url: None,
            action: if visible {
                crate::Action::Open
            } else {
                crate::Action::Close
            },
            id: Some(id.to_owned()),
            color: None,
            title: None,
            markdown: None,
            change_mode: None,
            highlight: None,
            source_path: None,
            status: None,
        },
        crate::entrance::Presentation::Direct,
        Some(actual),
    )?;
    if response["ok"] != true {
        return Err("메모 표시 상태를 변경하지 못했습니다".into());
    }
    self::visible(app, &label)
}

fn visible(app: &tauri::AppHandle, label: &str) -> Result<bool, String> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(false);
    };
    window_visible(&window)
}

pub(crate) fn window_visible(window: &tauri::WebviewWindow) -> Result<bool, String> {
    Ok(window.is_visible().map_err(|e| e.to_string())?
        && !window.is_minimized().map_err(|e| e.to_string())?)
}

// A closed terminal needs a new WebView; never build it from WebView2's
// synchronous IPC callback on Windows.
#[cfg_attr(windows, tauri::command(async))]
#[cfg_attr(not(windows), tauri::command)]
pub fn manager_terminal(
    window: tauri::WebviewWindow,
    id: String,
    action: String,
) -> Result<(), String> {
    if window.label() != LABEL {
        return Err("관리 창에서만 사용할 수 있습니다".into());
    }
    let app = window.app_handle();
    if !crate::terminal::contains(app, &id) {
        return Err("터미널이 없습니다".into());
    }
    match action.as_str() {
        "open" => {
            let _timing = crate::telemetry::Timing::new("manager.terminal.open");
            crate::terminal::open(app, &id)
        }
        "delete" => {
            let _timing = crate::telemetry::Timing::new("manager.terminal.delete");
            crate::terminal::welcome::delete(app, &id)
        }
        _ => Err("지원하지 않는 동작입니다".into()),
    }
}
