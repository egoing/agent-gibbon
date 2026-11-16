//! Local-only presentation journal. Never persists terminal input/output or starts a PTY.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{LogicalSize, Manager};
// During initial load WebKit has no current URL. Preserve the requested HTTP(S)
// address for restoration without presenting it as an observed current URL.
fn web_restore_url(value: &serde_json::Value) -> Result<String, String> {
    let url = value["current_url"]
        .as_str()
        .filter(|url| crate::web::validate_url(url).is_ok())
        .or_else(|| value["url"].as_str())
        .ok_or("웹 주소 없음")?;
    crate::web::validate_url(url)?;
    Ok(url.to_owned())
}
const FILE: &str = ".session-windows.json";
const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Entry {
    window: crate::update_restore::WindowState,
    expanded: Option<(f64, f64)>,
    // None means Welcome for a web window; never a terminal command.
    url: Option<String>,
    #[serde(default)]
    terminal_command: Option<String>,
    #[serde(default)]
    work_area_restore: Option<crate::sizing::WorkAreaRestore>,
    #[serde(default)]
    image_toolbar_below: bool,
    #[serde(default)]
    auto_layout_excluded: Option<bool>,
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    windows: Vec<Entry>,
    #[serde(default)]
    closed_notes: Vec<String>,
    #[serde(default)]
    monitor_layout: crate::monitor_layout::Saved,
}
#[derive(Default)]
struct Journal {
    ready: AtomicBool,
    queued: AtomicBool,
    file: Mutex<FileState>,
    terminals: Mutex<BTreeSet<String>>,
    commands: Mutex<std::collections::BTreeMap<String, CommandPreview>>,
}
#[derive(Default)]
struct FileState {
    generation: u64,
    closing: usize,
    last: Vec<u8>,
}

impl Journal {
    fn during_close(&self, close: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
        {
            let mut file = self.file.lock().map_err(|e| e.to_string())?;
            file.generation += 1;
            file.closing += 1;
        }
        // Native destroy can dispatch another callback; no journal lock spans it.
        let result = close();
        self.file.lock().map_err(|e| e.to_string())?.closing -= 1;
        result
    }
}

pub(crate) fn during_web_close(
    app: &tauri::AppHandle,
    close: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if app.try_state::<Journal>().is_none() {
        app.manage(Journal::default());
    }
    app.state::<Journal>().during_close(close)
}
// A fixed allowlist is deliberately stricter than secret-pattern guessing. No raw
// command line reaches disk; edits, escape sequences, arguments and multiline input
// invalidate the preview. Even the bounded in-memory buffer holds only safe prefixes.
const SAFE_COMMANDS: &[&str] = &[
    "pwd",
    "ls",
    "git status",
    "git diff",
    "git log",
    "whoami",
    "date",
    "clear",
];
#[derive(Default)]
struct CommandPreview {
    prefix: String,
    rejected: bool,
    last: Option<String>,
}
impl CommandPreview {
    fn input(&mut self, data: &str) {
        for c in data.chars() {
            self.last = None;
            if c == '\r' || c == '\n' {
                if !self.rejected && SAFE_COMMANDS.contains(&self.prefix.as_str()) {
                    self.last = Some(self.prefix.clone());
                }
                self.prefix.clear();
                self.rejected = false;
            } else if !self.rejected {
                self.prefix.push(c);
                if !SAFE_COMMANDS
                    .iter()
                    .any(|safe| safe.starts_with(&self.prefix))
                {
                    self.prefix.clear();
                    self.rejected = true;
                }
            }
        }
    }
}
pub fn terminal_started(app: &tauri::AppHandle, id: &str) {
    if let Some(j) = app.try_state::<Journal>() {
        if let Ok(mut commands) = j.commands.lock() {
            commands.insert(id.into(), CommandPreview::default());
        }
    }
}
pub fn command_input(app: &tauri::AppHandle, id: &str, data: &str) {
    if let Some(j) = app.try_state::<Journal>() {
        if let Ok(mut commands) = j.commands.lock() {
            commands.entry(id.into()).or_default().input(data);
        }
    }
}
pub fn last_command(app: &tauri::AppHandle, id: &str) -> Option<String> {
    app.try_state::<Journal>()?
        .commands
        .lock()
        .ok()?
        .get(id)?
        .last
        .clone()
}
fn supported(label: &str) -> bool {
    label == "main"
        || ["note-", "terminal-", "webview-"].iter().any(|prefix| {
            label.strip_prefix(prefix).is_some_and(|id| {
                !id.is_empty()
                    && id.len() <= 128
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
        })
}
fn validate(snapshot: &Snapshot) -> Result<(), String> {
    snapshot.monitor_layout.validate()?;
    if snapshot.version != 1 || snapshot.windows.len() > 1000 {
        return Err("시작 상태의 형식이 유효하지 않습니다".into());
    }
    if snapshot.closed_notes.len() > 1000
        || snapshot
            .closed_notes
            .iter()
            .any(|id| !supported(&crate::label(id)) || id.is_empty() || id.len() > 64)
    {
        return Err("닫힌 메모 목록이 유효하지 않습니다".into());
    }
    let closed: BTreeSet<_> = snapshot
        .closed_notes
        .iter()
        .map(|id| crate::label(id))
        .collect();
    if closed.len() != snapshot.closed_notes.len() {
        return Err("닫힌 메모가 중복되었습니다".into());
    }
    let mut labels = BTreeSet::new();
    for entry in &snapshot.windows {
        if entry.work_area_restore.as_ref().is_some_and(|r| !r.valid()) {
            return Err("잘못된 작업 영역 확대 복원 상태".into());
        }
        if !supported(&entry.window.label)
            || closed.contains(&entry.window.label)
            || !labels.insert(&entry.window.label)
        {
            return Err("잘못되거나 중복된 창 이름".into());
        }
        if !(1..=100_000).contains(&entry.window.width)
            || !(1..=100_000).contains(&entry.window.height)
        {
            return Err("잘못된 창 크기".into());
        }
        if entry.expanded.is_some_and(|(w, h)| {
            !w.is_finite()
                || !h.is_finite()
                || w <= 0.0
                || h <= 0.0
                || w > 100_000.0
                || h > 100_000.0
        }) {
            return Err("잘못된 펼침 크기".into());
        }
        if entry.terminal_command.as_ref().is_some_and(|cmd| {
            !entry.window.label.starts_with("terminal-") || !SAFE_COMMANDS.contains(&cmd.as_str())
        }) {
            return Err("저장할 수 없는 터미널 명령".into());
        }
        if let Some(url) = &entry.url {
            if !entry.window.label.starts_with("webview-") {
                return Err("웹 이외의 창에 URL이 있습니다".into());
            }
            crate::web::validate_url(url)?;
        }
    }
    Ok(())
}
pub fn terminal_restored(app: &tauri::AppHandle, id: &str) -> bool {
    app.try_state::<Journal>()
        .is_some_and(|j| j.terminals.lock().is_ok_and(|ids| ids.contains(id)))
}
fn read_snapshot(root: &Path) -> Result<Option<Snapshot>, String> {
    let file = match fs::File::open(root.join(FILE)) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("시작 상태 파일이 너무 큽니다".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes)
        .map_err(|_| "시작 상태 파일을 해석할 수 없습니다".to_string())?;
    validate(&snapshot)?;
    Ok(Some(snapshot))
}
pub fn restore(app: &tauri::AppHandle) -> Result<bool, String> {
    if crate::privacy::onboarding_required(app) {
        return Ok(false);
    }
    let store = app.state::<crate::storage::Store>();
    let Some(snapshot) = read_snapshot(&store.0)? else {
        return Ok(false);
    };
    crate::monitor_layout::restore(app, snapshot.monitor_layout)?;
    app.manage(Journal::default());
    for id in &snapshot.closed_notes {
        if let Some(window) = app.get_webview_window(&crate::label(id)) {
            crate::close_note(window)?;
        }
    }
    for entry in snapshot.windows {
        let label = &entry.window.label;
        if let Some(id) = label.strip_prefix("webview-") {
            crate::web::create_presented(app, None, Some(id), entry.url.as_deref(), false)?;
        } else if label.starts_with("terminal-") {
            // Deleted metadata is authoritative; never resurrect a removed terminal.
            if !crate::terminal::contains(app, label) {
                continue;
            }
            app.state::<Journal>()
                .terminals
                .lock()
                .map_err(|e| e.to_string())?
                .insert(label.clone());
            app.state::<Journal>()
                .commands
                .lock()
                .map_err(|e| e.to_string())?
                .insert(
                    label.clone(),
                    CommandPreview {
                        last: entry.terminal_command.clone(),
                        ..Default::default()
                    },
                );
            crate::terminal::open_presented(app, label, false)?;
        }
        let Some(window) = crate::web::managed_window(app, label) else {
            continue;
        };
        crate::sizing::restore_fold(&window, entry.expanded.map(|(w, h)| LogicalSize::new(w, h)))?;
        crate::sizing::set_image_toolbar_below(&window, entry.image_toolbar_below)?;
        crate::sizing::restore_work_area(&window, entry.work_area_restore)?;
        crate::update_restore::apply(&window, &entry.window)?;
        crate::layout::mark(&window, entry.auto_layout_excluded.unwrap_or(true));
    }
    Ok(true)
}
// Keep invalid snapshots intact for diagnosis rather than silently replacing them.
fn save(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(journal) = app.try_state::<Journal>() else {
        return Ok(());
    };
    if !journal.ready.load(Ordering::Acquire)
        || !crate::monitor_layout::ready(app)
        || crate::privacy::onboarding_required(app)
        || crate::stash::active(app)
        || crate::stash::chord_busy(app)
    {
        return Ok(());
    }
    let store = app.state::<crate::storage::Store>();
    checkpoint_file(&journal, &store.0, || collect_windows(app))
}

fn collect_windows(app: &tauri::AppHandle) -> Result<Option<Snapshot>, String> {
    let mut windows = Vec::new();
    let managed = crate::web::managed_windows(app);
    for (label, window) in managed {
        if !supported(&label)
            || label
                .strip_prefix("webview-")
                .is_some_and(|id| crate::web::closing(app, id))
        {
            continue;
        }
        if crate::sizing::pending(&window) || crate::entrance::active(&window) {
            return Ok(None);
        }
        let url = if let Some(id) = label.strip_prefix("webview-") {
            let value = crate::web::inspect(app, id)?;
            if value["welcome"] == false {
                Some(web_restore_url(&value)?)
            } else {
                None
            }
        } else {
            None
        };
        windows.push(Entry {
            window: crate::update_restore::state(&window)?,
            expanded: crate::sizing::expanded_size(&window).map(|s| (s.width, s.height)),
            url,
            terminal_command: last_command(app, &label),
            work_area_restore: crate::sizing::work_area_snapshot(&window),
            image_toolbar_below: crate::sizing::image_toolbar_below(&window),
            auto_layout_excluded: Some(crate::layout::excluded(&window)),
        });
    }
    windows.sort_by(|a, b| a.window.label.cmp(&b.window.label));
    let snapshot = Snapshot {
        version: 1,
        monitor_layout: crate::monitor_layout::snapshot(app),
        windows,
        closed_notes: app
            .state::<crate::ClosedNotes>()
            .lock()
            .map_err(|e| e.to_string())?
            .keys()
            .cloned()
            .collect(),
    };
    Ok(Some(snapshot))
}

fn checkpoint_file(
    journal: &Journal,
    root: &Path,
    collect: impl FnOnce() -> Result<Option<Snapshot>, String>,
) -> Result<(), String> {
    let generation = {
        let file = journal.file.lock().map_err(|e| e.to_string())?;
        if file.closing != 0 {
            return Ok(());
        }
        file.generation
    };
    let Some(snapshot) = collect()? else {
        return Ok(());
    };
    validate(&snapshot)?;
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let mut file = journal.file.lock().map_err(|e| e.to_string())?;
    if file.generation != generation || file.last == bytes {
        return Ok(());
    }
    write_bytes(root, &bytes)?;
    file.last = bytes;
    Ok(())
}
fn write_bytes(root: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err("시작 상태 파일이 너무 큽니다".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(root.join(FILE)).map_err(|e| e.to_string())?;
    Ok(())
}
// A full checkpoint can be deferred while other windows are stashed or moving.
// Edit the last durable snapshot so a restart immediately after close is safe.
fn forget_saved_web(
    journal: &Journal,
    root: &Path,
    label: &str,
    destroy: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if !label.starts_with("webview-") || !supported(label) {
        return Err("웹 창 이름이 유효하지 않습니다".into());
    }
    let undo = {
        let mut file = journal.file.lock().map_err(|e| e.to_string())?;
        file.last.clear();
        let mut undo = None;
        if let Some(mut snapshot) = read_snapshot(root)? {
            let entry = snapshot
                .windows
                .iter()
                .position(|e| e.window.label == label)
                .map(|index| (index, snapshot.windows.remove(index)));
            let layout = snapshot.monitor_layout.take_record(label);
            if entry.is_some() || layout.is_some() {
                write_bytes(
                    root,
                    &serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
                )?;
                undo = Some((entry, layout));
            }
        }
        undo
    };
    if let Err(error) = destroy() {
        if let Some((entry, layout)) = undo {
            let rollback = || -> Result<(), String> {
                let _file = journal.file.lock().map_err(|e| e.to_string())?;
                let mut snapshot = read_snapshot(root)?.ok_or("시작 기록을 찾을 수 없습니다")?;
                // Another web close may have completed during native dispatch.
                // Restore only this session, leaving that close intact.
                if let Some((index, entry)) = entry {
                    snapshot
                        .windows
                        .insert(index.min(snapshot.windows.len()), entry);
                }
                if let Some(layout) = layout {
                    snapshot.monitor_layout.put_record(label, layout);
                }
                write_bytes(
                    root,
                    &serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
                )
            };
            rollback().map_err(|rollback| format!("{error}; 시작 기록 복구 실패: {rollback}"))?;
        }
        return Err(error);
    }
    Ok(())
}
pub(crate) fn close_web(window: &tauri::WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let root = app.state::<crate::storage::Store>();
    forget_saved_web(&app.state::<Journal>(), &root.0, window.label(), || {
        window.destroy().map_err(|e| e.to_string())
    })
}
pub fn checkpoint(app: &tauri::AppHandle) {
    // Flush explicit geometry ownership before serializing the matching frame.
    crate::monitor_layout::poll(app);
    if let Err(error) = save(app) {
        eprintln!("시작 창 상태 저장 실패: {error}");
    }
}
pub fn start(app: &tauri::AppHandle, enabled: bool) {
    if app.try_state::<Journal>().is_none() {
        app.manage(Journal::default());
    }
    app.state::<Journal>()
        .ready
        .store(enabled, Ordering::Release);
    let app = app.clone();
    std::thread::spawn(move || {
        let mut checkpoint_at = std::time::Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            if app.state::<Journal>().queued.swap(true, Ordering::AcqRel) {
                continue;
            }
            let handle = app.clone();
            let save = checkpoint_at.elapsed() >= Duration::from_secs(1);
            if save {
                checkpoint_at = std::time::Instant::now();
            }
            if app
                .run_on_main_thread(move || {
                    crate::monitor_layout::poll(&handle);
                    if save {
                        checkpoint(&handle);
                    }
                    handle
                        .state::<Journal>()
                        .queued
                        .store(false, Ordering::Release);
                })
                .is_err()
            {
                break;
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Snapshot {
        serde_json::from_value(serde_json::json!({"version":1,"windows":[{"window":{"label":"webview-web-a","x":-100,"y":20,"width":600,"height":24,"visible":false,"minimized":false,"always_on_top":true},"expanded":[600,400],"url":"https://example.com/last"}]})).unwrap()
    }
    #[test]
    fn web_url_before_load_restores_request_without_confusing_navigation() {
        use serde_json::json;
        assert_eq!(web_restore_url(&json!({"current_url":null,"url":"https://example.com/start","navigation_url":"https://example.com/pending"})).unwrap(), "https://example.com/start");
        assert_eq!(web_restore_url(&json!({"current_url":"https://example.com/redirect","url":"https://example.com/start"})).unwrap(), "https://example.com/redirect");
        assert!(web_restore_url(&json!({"current_url":null,"url":"file:///tmp/x"})).is_err());
        assert!(web_restore_url(&json!({"current_url":null,"url":null})).is_err());
        assert_eq!(
            web_restore_url(
                &json!({"current_url":"about:blank","url":"https://example.com/start"})
            )
            .unwrap(),
            "https://example.com/start"
        );
    }
    #[test]
    fn safe_command_preview_discards_secrets_and_edits() {
        let mut preview = CommandPreview::default();
        for input in ["g", "it ", "status", "\r"] {
            preview.input(input);
        }
        assert_eq!(preview.last.as_deref(), Some("git status"));
        preview.input("export API_KEY=secret\r");
        assert!(preview.last.is_none());
        preview.input("pwd\u{1b}[A\r");
        assert!(preview.last.is_none());
        preview.input("ls\r");
        assert_eq!(preview.last.as_deref(), Some("ls"));
        preview.input("x");
        assert!(preview.last.is_none());
        assert!(preview.prefix.is_empty());
    }
    #[test]
    fn roundtrip_hidden_folded_web() {
        let s = sample();
        validate(&s).unwrap();
        validate(&serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap()).unwrap();
    }
    #[test]
    fn old_journal_defaults_new_recovery_fields_without_losing_fold_or_url() {
        let s = sample();
        assert!(s.windows[0].work_area_restore.is_none());
        assert!(!s.windows[0].image_toolbar_below);
        let value = serde_json::to_value(s).unwrap();
        assert_eq!(
            value["windows"][0]["expanded"],
            serde_json::json!([600., 400.])
        );
        assert_eq!(value["windows"][0]["url"], "https://example.com/last");
    }
    #[test]
    fn close_web_removes_only_its_restart_entry_even_when_retried() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut snapshot = sample();
        for label in ["webview-web-b", "note-text", "note-image", "terminal-demo"] {
            let mut entry = sample().windows.remove(0);
            entry.window.label = label.into();
            if !label.starts_with("webview-") {
                entry.url = None;
            }
            if label.starts_with("terminal-") {
                entry.terminal_command = Some("pwd".into());
            }
            snapshot.windows.push(entry);
        }
        snapshot.closed_notes.push("closed-text".into());
        let frame = serde_json::json!({"x":0,"y":0,"width":600,"height":400});
        let layout = serde_json::json!({
            "last":frame, "screen":{"bounds":frame,"work":frame,"scale":1.0},
            "folded":true
        });
        snapshot.monitor_layout = serde_json::from_value(serde_json::json!({
            "records": {"webview-web-a":layout, "note-text":layout}
        }))
        .unwrap();
        let expected = serde_json::to_value(&snapshot.windows[1..]).unwrap();
        write_bytes(root, &serde_json::to_vec(&snapshot).unwrap()).unwrap();

        for _ in 0..2 {
            forget_saved_web(&Journal::default(), root, "webview-web-a", || Ok(())).unwrap();
            let restored = read_snapshot(root).unwrap().unwrap();
            assert_eq!(serde_json::to_value(restored.windows).unwrap(), expected);
            assert_eq!(restored.closed_notes, ["closed-text"]);
            let layout = serde_json::to_value(restored.monitor_layout).unwrap();
            let records = layout["records"].as_object().unwrap();
            assert_eq!(records.len(), 1);
            assert!(records.contains_key("note-text"));
        }
        forget_saved_web(&Journal::default(), root, "webview-web-b", || Ok(())).unwrap();
        let restored = read_snapshot(root).unwrap().unwrap();
        assert!(restored.windows.iter().all(|e| e.url.is_none()));
        assert_eq!(restored.windows.len(), 3);
    }

    #[test]
    fn checkpoint_collected_before_close_cannot_restore_the_closed_web() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let journal = Journal::default();
        write_bytes(root, &serde_json::to_vec(&sample()).unwrap()).unwrap();
        checkpoint_file(&journal, root, || {
            let prepared = read_snapshot(root)?;
            journal
                .during_close(|| forget_saved_web(&journal, root, "webview-web-a", || Ok(())))?;
            Ok(prepared)
        })
        .unwrap();
        assert!(read_snapshot(root).unwrap().unwrap().windows.is_empty());
    }

    #[test]
    fn reentrant_checkpoint_and_second_close_preserve_only_the_failed_session() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let journal = Journal::default();
        let mut snapshot = sample();
        let mut other = sample().windows.remove(0);
        other.window.label = "webview-web-b".into();
        snapshot.windows.push(other);
        write_bytes(root, &serde_json::to_vec(&snapshot).unwrap()).unwrap();
        let failed = journal.during_close(|| {
            forget_saved_web(&journal, root, "webview-web-a", || {
                checkpoint_file(&journal, root, || {
                    panic!("native callback must defer collection")
                })?;
                journal.during_close(|| {
                    forget_saved_web(&journal, root, "webview-web-b", || Ok(()))
                })?;
                Err("failed".into())
            })
        });
        assert_eq!(failed, Err("failed".into()));
        let restored = read_snapshot(root).unwrap().unwrap();
        assert_eq!(restored.windows.len(), 1);
        assert_eq!(restored.windows[0].window.label, "webview-web-a");
        checkpoint_file(&journal, root, || {
            let mut next = restored;
            next.windows[0].window.width = 900;
            Ok(Some(next))
        })
        .unwrap();
        assert_eq!(
            read_snapshot(root).unwrap().unwrap().windows[0]
                .window
                .width,
            900
        );
    }

    #[test]
    fn failed_destroy_dispatch_restores_restart_entry() {
        let dir = tempfile::tempdir().unwrap();
        let original = serde_json::to_vec(&sample()).unwrap();
        write_bytes(dir.path(), &original).unwrap();
        let result = forget_saved_web(&Journal::default(), dir.path(), "webview-web-a", || {
            assert!(read_snapshot(dir.path())
                .unwrap()
                .unwrap()
                .windows
                .is_empty());
            Err("dispatch failed".into())
        });
        assert_eq!(result, Err("dispatch failed".into()));
        assert_eq!(fs::read(dir.path().join(FILE)).unwrap(), original);
    }

    #[test]
    fn close_preserves_invalid_journal_and_never_creates_an_absent_one() {
        let dir = tempfile::tempdir().unwrap();
        forget_saved_web(&Journal::default(), dir.path(), "webview-web-a", || Ok(())).unwrap();
        let path = dir.path().join(FILE);
        assert!(!path.exists());
        fs::write(&path, b"invalid").unwrap();
        assert!(
            forget_saved_web(&Journal::default(), dir.path(), "webview-web-a", || Ok(())).is_err()
        );
        assert_eq!(fs::read(path).unwrap(), b"invalid");
    }
    #[test]
    fn invalid_records_are_rejected() {
        let mut s = sample();
        s.windows[0].url = Some("file:///etc/passwd".into());
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.windows[0].window.label = "note-../outside".into();
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.windows[0].expanded = Some((f64::NAN, 100.0));
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.version = 2;
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.windows.push(sample().windows.remove(0));
        assert!(validate(&s).is_err());
    }
}
