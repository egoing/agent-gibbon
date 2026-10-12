use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_REQUEST: u64 = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Open,
    Update,
    Close,
    Inspect,
    List,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    pub id: Option<String>,
    pub color: Option<String>,
    pub title: Option<String>,
    pub markdown: Option<String>,
    pub status: Option<String>,
}

impl Request {
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("main")
    }

    pub fn validate(&self) -> Result<(), String> {
        let id = self.id();
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("창 ID는 영문·숫자·하이픈·밑줄 1~64자여야 합니다".into());
        }
        if self.title.as_ref().is_some_and(|s| s.len() > 1024)
            || self.status.as_ref().is_some_and(|s| s.len() > 1024)
        {
            return Err("제목과 상태는 각각 1024바이트 이하여야 합니다".into());
        }
        if let Some(color) = &self.color {
            if color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("색상은 #RRGGBB 형식이어야 합니다".into());
            }
        }
        if matches!(self.action, Action::Close | Action::Inspect | Action::List)
            && (self.title.is_some()
                || self.markdown.is_some()
                || self.status.is_some()
                || self.color.is_some())
        {
            return Err("close/inspect/list에는 내용·색상 옵션을 사용할 수 없습니다".into());
        }
        if matches!(self.action, Action::List) && self.id.is_some() {
            return Err("list에는 창 ID를 지정할 수 없습니다".into());
        }
        if let Some(markdown) = &self.markdown {
            validate_markdown(markdown)?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize)]
pub struct Note {
    pub title: String,
    pub color: String,
    pub html: String,
    pub status: String,
    pub updated_at: u64,
    pub revision: u64,
}
impl Default for Note {
    fn default() -> Self {
        Self {
            title: "Floatnote".into(),
            color: "#20252e".into(),
            html: "<p>작업 내용을 기다리고 있습니다.</p>".into(),
            status: "대기".into(),
            updated_at: 0,
            revision: 0,
        }
    }
}
impl Note {
    pub fn apply(&mut self, request: &Request) -> Result<(), String> {
        request.validate()?;
        if matches!(
            request.action,
            Action::Close | Action::Inspect | Action::List
        ) {
            return Ok(());
        }
        if let Some(color) = &request.color {
            self.color = color.to_ascii_lowercase();
        }
        if let Some(title) = &request.title {
            self.title = title.clone();
        }
        if let Some(markdown) = &request.markdown {
            self.html = render(markdown);
        }
        if let Some(status) = &request.status {
            self.status = status.clone();
        }
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.revision += 1;
        Ok(())
    }
}

// Markdown accepts plain text, so reject only a likely missing task-list marker.
// Parser offsets exclude fenced/indented code, inline code and actual links.
fn validate_markdown(markdown: &str) -> Result<(), String> {
    let mut excluded = Vec::new();
    let mut code_start = None;
    let mut link_start = None;
    for (event, range) in Parser::new_ext(markdown, Options::ENABLE_TASKLISTS).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_start.take() {
                    excluded.push(start..range.end);
                }
            }
            Event::Start(Tag::Link { .. }) => link_start = Some(range.start),
            Event::End(TagEnd::Link) => {
                if let Some(start) = link_start.take() {
                    excluded.push(start..range.end);
                }
            }
            Event::Code(_) => excluded.push(range),
            _ => (),
        }
    }
    let mut offset = 0;
    for (index, line) in markdown.split_inclusive('\n').enumerate() {
        let text = line.trim_start();
        let start = offset + line.len() - text.len();
        let bytes = text.as_bytes();
        if bytes.len() >= 3
            && bytes[0] == b'['
            && matches!(bytes[1], b' ' | b'x' | b'X')
            && bytes[2] == b']'
            && (bytes.len() == 3 || bytes[3].is_ascii_whitespace())
            && !excluded.iter().any(|range| range.contains(&start))
        {
            return Err(format!("MARKDOWN_TASK_MARKER: {}행: 체크리스트 앞의 목록 기호가 빠졌습니다. '- [ ] 항목' 또는 '- [x] 완료 항목'으로 수정한 뒤 재시도하세요. 문자 그대로 표시하려면 코드(`...`) 또는 이스케이프(\\[)를 사용하세요. 기존 메모는 변경되지 않았습니다.", index + 1));
        }
        offset += line.len();
    }
    Ok(())
}

pub fn render(markdown: &str) -> String {
    let mut events: Vec<_> = Parser::new_ext(
        markdown,
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    )
    .map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        other => other,
    })
    .collect();
    // Annotate only lists whose direct items are all tasks; ordinary and mixed
    // lists retain their indentation. Raw user HTML was escaped above.
    let mut lists: Vec<(usize, Vec<bool>)> = Vec::new();
    let mut items = Vec::new();
    for index in 0..events.len() {
        match &events[index] {
            Event::Start(Tag::List(_)) => lists.push((index, Vec::new())),
            Event::Start(Tag::Item) => {
                if let Some((_, flags)) = lists.last_mut() {
                    flags.push(false);
                }
                items.push((index, false));
            }
            Event::TaskListMarker(checked) => {
                let checked = *checked;
                if let Some((_, task)) = items.last_mut() {
                    *task = true;
                }
                if let Some((_, flags)) = lists.last_mut() {
                    if let Some(flag) = flags.last_mut() {
                        *flag = true;
                    }
                }
                events[index] = Event::Text(if checked { "☑ " } else { "☐ " }.into());
            }
            Event::End(TagEnd::Item) => {
                if let Some((start, true)) = items.pop() {
                    events[start] = Event::Html("<li class=\"task-item\">".into());
                    events[index] = Event::Html("</li>".into());
                }
            }
            Event::End(TagEnd::List(_)) => {
                if let Some((start, flags)) = lists.pop() {
                    if !flags.is_empty() && flags.iter().all(|task| *task) {
                        events[start] = Event::Html("<ul class=\"task-list\">".into());
                        events[index] = Event::Html("</ul>".into());
                    }
                }
            }
            _ => (),
        }
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    // No images, navigation, arbitrary HTML or executable URL schemes.
    ammonia::Builder::default()
        .add_tag_attributes("ul", &["class"])
        .add_tag_attributes("li", &["class"])
        .rm_tags(&["img"])
        .rm_tag_attributes("a", &["href"])
        .clean(&output)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_tasks_report_line_and_preserve_all_fields() {
        let mut note = Note::default();
        let before = serde_json::to_value(&note).unwrap();
        let request: Request = serde_json::from_value(serde_json::json!({
            "action":"update", "title":"changed", "color":"#abcdef", "status":"changed",
            "markdown":"# 제목\n\n[x] 완료\n[ ] 대기"
        }))
        .unwrap();
        let error = note.apply(&request).unwrap_err();
        assert!(error.contains("MARKDOWN_TASK_MARKER") && error.contains("3행"));
        assert!(error.contains("- [x]"));
        assert_eq!(serde_json::to_value(&note).unwrap(), before);
        for text in ["[ ] 대기", "  [X] 완료", "[x]", "제목\r\n[ ] 대기"] {
            assert!(validate_markdown(text).is_err(), "{text}");
        }
    }
    #[test]
    fn markdown_validation_allows_code_links_and_real_tasks() {
        for text in [
            "- [ ] 대기\n- [x] 완료",
            "일반 문장 [ ] 설명",
            "\\[ ] 문자",
            "```md\n[ ] 예제\n```",
            "~~~\n[x] 예제\n~~~",
            "    [ ] 코드",
            "`[ ] 예제`",
            "[x](https://example.com) 링크",
            "[x] 참조\n\n[x]: https://example.com",
            "- 항목\n    ```\n    [ ] 코드\n    ```",
        ] {
            assert!(validate_markdown(text).is_ok(), "{text}");
        }
        assert!(validate_markdown("```\n[ ] 예제\n```\n[ ] 오류")
            .unwrap_err()
            .contains("4행"));
    }
    #[test]
    fn renders_korean_tasks_and_blocks_active_content() {
        let html = render("# 한글\n- [x] 완료\n- [ ] 대기\n<script>alert(1)</script>\n<img src=x onerror=alert(1)>\n[실행](javascript:alert(1))\n![외부](https://example.com/tracker)");
        assert!(html.contains("<h1>한글</h1>"));
        assert!(html.contains("☑ 완료") && html.contains("☐ 대기"));
        assert!(!html.contains("<script") && !html.contains("<img") && !html.contains("href="));
    }
    #[test]
    fn partial_updates_preserve_unspecified_fields() {
        let mut note = Note::default();
        note.apply(&Request {
            id: None,
            color: None,
            action: Action::Open,
            title: Some("작업".into()),
            markdown: Some("본문".into()),
            status: None,
        })
        .unwrap();
        note.apply(&Request {
            id: None,
            color: None,
            action: Action::Update,
            title: None,
            markdown: None,
            status: Some("완료".into()),
        })
        .unwrap();
        assert_eq!(note.title, "작업");
        assert!(note.html.contains("본문"));
        assert_eq!(note.revision, 2);
    }
    #[test]
    fn rejected_update_does_not_change_note() {
        let mut note = Note::default();
        assert!(note
            .apply(&Request {
                id: None,
                color: None,
                action: Action::Update,
                title: Some("x".repeat(1025)),
                markdown: None,
                status: None
            })
            .is_err());
        assert_eq!(note.revision, 0);
    }
    #[test]
    fn old_requests_keep_default_window_and_color() {
        let request: Request =
            serde_json::from_str(r#"{"action":"open","title":"legacy"}"#).unwrap();
        assert_eq!(request.id(), "main");
        let mut note = Note::default();
        note.apply(&request).unwrap();
        assert_eq!(note.color, "#20252e");
    }

    #[test]
    fn color_updates_are_normalized_preserved_and_atomic() {
        let mut note = Note::default();
        let request: Request =
            serde_json::from_str(r##"{"action":"update","color":"#AABBCC"}"##).unwrap();
        note.apply(&request).unwrap();
        assert_eq!(note.color, "#aabbcc");
        let request: Request =
            serde_json::from_str(r#"{"action":"update","status":"done"}"#).unwrap();
        note.apply(&request).unwrap();
        assert_eq!(note.color, "#aabbcc");
        let before = serde_json::to_value(&note).unwrap();
        for color in [
            "",
            "red",
            "#abc",
            "#00000000",
            "#GG0000",
            "#fffff;",
            "#한글",
        ] {
            let request: Request = serde_json::from_value(
                serde_json::json!({"action":"update","title":"bad","color":color}),
            )
            .unwrap();
            assert!(note.apply(&request).is_err(), "{color}");
            assert_eq!(serde_json::to_value(&note).unwrap(), before);
        }
    }

    #[test]
    fn invalid_ids_and_read_only_options_are_rejected() {
        for id in ["", "a/b", "../main", "a b", "한글", &"x".repeat(65)] {
            let request: Request =
                serde_json::from_value(serde_json::json!({"action":"open","id":id})).unwrap();
            assert!(request.validate().is_err(), "{id}");
        }
        for id in ["main", "note-main", "Job_2026-01", &"x".repeat(64)] {
            let request: Request =
                serde_json::from_value(serde_json::json!({"action":"open","id":id})).unwrap();
            request.validate().unwrap();
        }
        for action in ["close", "inspect", "list"] {
            let request: Request =
                serde_json::from_value(serde_json::json!({"action":action,"color":"#112233"}))
                    .unwrap();
            assert!(request.validate().is_err());
        }
        let request: Request = serde_json::from_str(r#"{"action":"list","id":"main"}"#).unwrap();
        assert!(request.validate().is_err());
    }
    #[test]
    fn tasks_use_unindented_rows_but_normal_lists_keep_markers() {
        let tasks = render("- [x] 완료\n- [ ] 대기");
        assert!(tasks.contains("<ul class=\"task-list\">"));
        assert_eq!(tasks.matches("class=\"task-item\"").count(), 2);
        let normal = render("- 첫째\n- 둘째");
        assert!(!normal.contains("task-list") && !normal.contains("task-item"));
        let mixed = render("- 일반\n- [ ] 체크");
        assert!(!mixed.contains("task-list") && mixed.contains("task-item"));
        let nested = render("- 부모\n  - [ ] 자식\n- 형제");
        assert_eq!(nested.matches("class=\"task-list\"").count(), 1);
        let loose = render("- [x] 첫째\n\n- [ ] 둘째");
        assert_eq!(loose.matches("class=\"task-item\"").count(), 2);
    }
}
