//! Read-only manager, independent of floating memo window state.
use crate::AppNotes;
use serde_json::{json, Value};
use tauri::Manager;

pub const LABEL: &str = "manager";

pub fn open(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        crate::stash::release(&window)?;
        window.unminimize().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("manager.html".into()))
        .title(format!(
            "{} — 메모 관리",
            crate::instance::current().title()
        ))
        .transparent(false)
        .inner_size(944.0, 480.0)
        .min_inner_size(600.0, 360.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn manager_notes(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppNotes>,
) -> Result<Value, String> {
    if window.label() != LABEL {
        return Err("관리 창에서만 조회할 수 있습니다".into());
    }
    let notes = state.lock().map_err(|e| e.to_string())?;
    let mut rows: Vec<Value> = notes.iter().map(|(id,note)|json!({"id":id,"title":display_title(note,id),"html":note.html,"capture":note.capture})).collect();
    drop(notes);
    for agent in crate::terminal::entries(window.app_handle()) {
        rows.push(json!({"id":agent.id,"title":agent.display_title(),"kind":agent.kind}));
    }
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
    let text = text.trim();
    if !text.is_empty() {
        text.to_string()
    } else if !note.title.trim().is_empty() {
        note.title.clone()
    } else {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(display_title(&note, "id"), "id");
    }
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
