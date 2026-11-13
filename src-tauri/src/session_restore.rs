//! Local-only presentation journal. Never persists terminal input/output or starts a PTY.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{LogicalSize, Manager};
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
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    windows: Vec<Entry>,
    #[serde(default)]
    closed_notes: Vec<String>,
}
#[derive(Default)]
struct Journal {
    ready: AtomicBool,
    queued: AtomicBool,
    last: Mutex<Vec<u8>>,
    terminals: Mutex<BTreeSet<String>>,
    commands: Mutex<std::collections::BTreeMap<String, CommandPreview>>,
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
pub fn restore(app: &tauri::AppHandle) -> Result<bool, String> {
    if crate::privacy::onboarding_required(app) {
        return Ok(false);
    }
    let path = app.state::<crate::storage::Store>().0.join(FILE);
    if !path.exists() {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("시작 상태 파일이 너무 큽니다".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes)
        .map_err(|_| "시작 상태 파일을 해석할 수 없습니다".to_string())?;
    validate(&snapshot)?;
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
        crate::update_restore::apply(&window, &entry.window)?;
        keep_reachable(&window)?;
        crate::layout::mark(&window, true);
    }
    Ok(true)
}
fn keep_reachable(window: &tauri::WebviewWindow) -> Result<(), String> {
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let areas: Vec<_> = monitors
        .iter()
        .map(|m| {
            let a = m.work_area();
            (
                i64::from(a.position.x),
                i64::from(a.position.y),
                i64::from(a.size.width),
                i64::from(a.size.height),
            )
        })
        .collect();
    let (x, y) = reachable_position(
        (i64::from(p.x), i64::from(p.y)),
        (i64::from(size.width), i64::from(size.height)),
        &areas,
    );
    if (x, y) != (i64::from(p.x), i64::from(p.y)) {
        window
            .set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn reachable_position(
    p: (i64, i64),
    size: (i64, i64),
    areas: &[(i64, i64, i64, i64)],
) -> (i64, i64) {
    let visible_width = size.0.min(48);
    let visible_height = size.1.min(24);
    let candidate = |&(x, y, w, h): &(i64, i64, i64, i64)| {
        (
            p.0.clamp(
                x - size.0 + visible_width,
                (x + w - visible_width).max(x - size.0 + visible_width),
            ),
            p.1.clamp(y, (y + h - visible_height).max(y)),
        )
    };
    areas
        .iter()
        .map(candidate)
        .min_by_key(|&(x, y)| (i128::from(x - p.0)).pow(2) + (i128::from(y - p.1)).pow(2))
        .unwrap_or(p)
}
// Keep invalid snapshots intact for diagnosis rather than silently replacing them.
fn save(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(journal) = app.try_state::<Journal>() else {
        return Ok(());
    };
    if !journal.ready.load(Ordering::Acquire)
        || crate::privacy::onboarding_required(app)
        || crate::stash::active(app)
        || crate::stash::chord_busy(app)
    {
        return Ok(());
    }
    let mut windows = Vec::new();
    let managed = crate::web::managed_windows(app);
    for (label, window) in managed {
        if !supported(&label) {
            continue;
        }
        if crate::sizing::pending(&window) || crate::entrance::active(&window) {
            return Ok(());
        }
        let url = if let Some(id) = label.strip_prefix("webview-") {
            let value = crate::web::inspect(app, id)?;
            if value["welcome"] == false {
                Some(
                    value["current_url"]
                        .as_str()
                        .ok_or("웹 주소 없음")?
                        .to_owned(),
                )
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
        });
    }
    windows.sort_by(|a, b| a.window.label.cmp(&b.window.label));
    let snapshot = Snapshot {
        version: 1,
        windows,
        closed_notes: app
            .state::<crate::ClosedNotes>()
            .lock()
            .map_err(|e| e.to_string())?
            .keys()
            .cloned()
            .collect(),
    };
    validate(&snapshot)?;
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let mut last = journal.last.lock().map_err(|e| e.to_string())?;
    if *last == bytes {
        return Ok(());
    }
    let store = app.state::<crate::storage::Store>();
    // Open/update/save_edit commit contents synchronously through Store. Geometry
    // checkpoints must not rewrite every note on every drag or URL navigation.
    fs::create_dir_all(&store.0).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&store.0).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(store.0.join(FILE))
        .map_err(|e| e.to_string())?;
    *last = bytes;
    Ok(())
}
pub fn checkpoint(app: &tauri::AppHandle) {
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
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        if app.state::<Journal>().queued.swap(true, Ordering::AcqRel) {
            continue;
        }
        let handle = app.clone();
        if app
            .run_on_main_thread(move || {
                checkpoint(&handle);
                handle
                    .state::<Journal>()
                    .queued
                    .store(false, Ordering::Release);
            })
            .is_err()
        {
            break;
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
    fn disconnected_monitor_uses_nearest_area_and_keeps_spanning_windows() {
        let areas = [(0, 0, 1000, 800), (1000, 0, 1000, 800)];
        assert_eq!(reachable_position((900, 20), (600, 400), &areas), (900, 20));
        assert_eq!(
            reachable_position((2500, 40), (600, 400), &areas),
            (1952, 40)
        );
        assert_eq!(
            reachable_position((-800, -500), (600, 400), &areas),
            (-552, 0)
        );
    }
    #[test]
    fn roundtrip_hidden_folded_web() {
        let s = sample();
        validate(&s).unwrap();
        validate(&serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap()).unwrap();
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
