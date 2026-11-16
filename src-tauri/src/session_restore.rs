//! Local-only presentation journal. Never persists terminal input/output or starts a PTY.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    closed_geometry: BTreeMap<String, ReopenGeometry>,
}
#[derive(Default)]
struct Journal {
    ready: AtomicBool,
    queued: AtomicBool,
    file: Mutex<JournalFile>,
    closes: Mutex<CloseState>,
    terminals: Mutex<BTreeSet<String>>,
    commands: Mutex<std::collections::BTreeMap<String, CommandPreview>>,
}
#[derive(Default)]
struct CloseState {
    generation: u64,
    active: usize,
}
impl Journal {
    fn collection_generation(&self) -> Result<Option<u64>, String> {
        let closes = self.closes.lock().map_err(|e| e.to_string())?;
        Ok((closes.active == 0).then_some(closes.generation))
    }

    fn during_close<T>(&self, close: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        {
            // Never block PTY teardown on a checkpoint's disk I/O.
            let mut closes = self.closes.lock().map_err(|e| e.to_string())?;
            closes.generation += 1;
            closes.active += 1;
        }
        // Native destroy can dispatch another callback; no journal lock spans it.
        let result = close();
        self.closes.lock().map_err(|e| e.to_string())?.active -= 1;
        result
    }
}

pub(crate) fn during_native_close<T>(
    app: &tauri::AppHandle,
    close: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if app.try_state::<Journal>().is_none() {
        app.manage(Journal::default());
    }
    app.state::<Journal>().during_close(close)
}
pub(crate) fn during_web_close(
    app: &tauri::AppHandle,
    close: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    during_native_close(app, close)
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
#[derive(Clone, Serialize, Deserialize)]
struct ReopenGeometry {
    bounds: crate::capture::ImageNote,
    physical_position: bool,
}

impl ReopenGeometry {
    fn valid(&self, id: &str) -> bool {
        crate::terminal::valid_id(id)
            && self.bounds.x.is_finite()
            && self.bounds.y.is_finite()
            && (1.0..=100_000.).contains(&self.bounds.width)
            && (1.0..=100_000.).contains(&self.bounds.height)
    }
}

#[derive(Default)]
struct JournalFile {
    last: Vec<u8>,
    closed: BTreeMap<String, ReopenGeometry>,
    revision: u64,
}

impl JournalFile {
    fn load(path: &Path) -> Result<Option<(Snapshot, Self)>, String> {
        let mut bytes = Vec::new();
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("시작 상태 파일이 너무 큽니다".into());
        }
        let snapshot: Snapshot =
            serde_json::from_slice(&bytes).map_err(|_| "시작 상태 파일을 해석할 수 없습니다")?;
        validate(&snapshot)?;
        let closed = snapshot.closed_geometry.clone();
        Ok(Some((
            snapshot,
            Self {
                last: bytes,
                closed,
                ..Default::default()
            },
        )))
    }

    // Web close and rollback operate on the current durable open-window list.
    // Keep its bytes in sync without resetting pending closed terminal frames.
    fn read_current(&mut self, root: &Path) -> Result<Option<Snapshot>, String> {
        let Some((snapshot, durable)) = Self::load(&root.join(FILE))? else {
            self.last.clear();
            return Ok(None);
        };
        self.last = durable.last;
        Ok(Some(snapshot))
    }

    fn persist(&mut self, root: &Path, snapshot: &Snapshot) -> Result<(), String> {
        validate(snapshot)?;
        let bytes = serde_json::to_vec(snapshot).map_err(|e| e.to_string())?;
        if bytes == self.last {
            return Ok(());
        }
        write_bytes(root, &bytes)?;
        self.last = bytes;
        Ok(())
    }

    fn checkpoint(
        &mut self,
        root: &Path,
        revision: u64,
        mut snapshot: Snapshot,
    ) -> Result<(), String> {
        if revision != self.revision {
            return Ok(());
        }
        snapshot
            .windows
            .retain(|e| !self.closed.contains_key(&e.window.label));
        snapshot.closed_geometry.clone_from(&self.closed);
        self.persist(root, &snapshot)
    }

    fn reopened(&mut self, id: &str) {
        if self.closed.remove(id).is_some() {
            self.revision += 1;
        }
    }

    // The host has already been destroyed. Retain a valid frame for the
    // next full checkpoint even when this immediate disk write fails.
    fn closed_after_destroy(
        &mut self,
        root: &Path,
        id: &str,
        frame: ReopenGeometry,
    ) -> Result<bool, String> {
        if !frame.valid(id) {
            return Err("닫힌 창의 위치 또는 크기가 유효하지 않습니다".into());
        }
        if let Err(error) = self.set_closed(root, id, Some(frame.clone())) {
            self.closed.insert(id.into(), frame);
            self.revision += 1;
            return Err(error);
        }
        Ok(!self.last.is_empty())
    }

    // Persist just the transition. Other windows may be in a transient layout.
    fn set_closed(
        &mut self,
        root: &Path,
        id: &str,
        frame: Option<ReopenGeometry>,
    ) -> Result<(), String> {
        // Before the first checkpoint no complete open-window list exists.
        // Leave the legacy close frame durable and include this entry in the
        // first complete snapshot instead of creating an empty session.
        if !self.last.is_empty() {
            let mut snapshot: Snapshot =
                serde_json::from_slice(&self.last).map_err(|e| e.to_string())?;
            snapshot.windows.retain(|entry| entry.window.label != id);
            if let Some(frame) = &frame {
                snapshot.closed_geometry.insert(id.into(), frame.clone());
            } else {
                snapshot.closed_geometry.remove(id);
            }
            self.persist(root, &snapshot)?;
        }
        if let Some(frame) = frame {
            self.closed.insert(id.into(), frame);
        } else {
            self.closed.remove(id);
        }
        self.revision += 1;
        Ok(())
    }
}

pub(crate) fn logical_geometry(
    w: &tauri::WebviewWindow,
) -> Result<(crate::capture::ImageNote, bool), String> {
    if let Some(g) = crate::stash::saved_geometry(w) {
        return Ok((g, cfg!(windows)));
    }
    let p = w.outer_position().map_err(|e| e.to_string())?;
    let s = w.inner_size().map_err(|e| e.to_string())?;
    let scale = w.scale_factor().map_err(|e| e.to_string())?;
    let unit = if cfg!(windows) { 1. } else { scale };
    Ok((
        crate::capture::ImageNote {
            x: p.x as f64 / unit,
            y: p.y as f64 / unit,
            width: s.width as f64 / scale,
            height: s.height as f64 / scale,
        },
        cfg!(windows),
    ))
}

pub(crate) fn remember_closed(
    app: &tauri::AppHandle,
    id: &str,
    bounds: crate::capture::ImageNote,
    physical_position: bool,
) -> Result<bool, String> {
    let Some(j) = app.try_state::<Journal>() else {
        return Ok(false);
    };
    if !j.ready.load(Ordering::Acquire) || crate::privacy::onboarding_required(app) {
        return Ok(false);
    }
    let mut file = j.file.lock().map_err(|e| e.to_string())?;
    file.closed_after_destroy(
        &app.state::<crate::storage::Store>().0,
        id,
        ReopenGeometry {
            bounds,
            physical_position,
        },
    )
}

pub(crate) fn reopen_geometry(
    app: &tauri::AppHandle,
    id: &str,
) -> Option<(crate::capture::ImageNote, bool)> {
    let j = app.try_state::<Journal>()?;
    if crate::privacy::onboarding_required(app) {
        return None;
    }
    let f = j.file.lock().ok()?;
    f.closed
        .get(id)
        .map(|g| (g.bounds.clone(), g.physical_position))
}

pub(crate) fn mark_reopened(app: &tauri::AppHandle, id: &str) {
    if let Some(j) = app.try_state::<Journal>() {
        j.file.lock().unwrap().reopened(id);
    }
}

pub(crate) fn forget_closed(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let Some(j) = app.try_state::<Journal>() else {
        return Ok(());
    };
    if j.ready.load(Ordering::Acquire) && !crate::privacy::onboarding_required(app) {
        let mut file = j.file.lock().map_err(|e| e.to_string())?;
        let result = file.set_closed(&app.state::<crate::storage::Store>().0, id, None);
        if result.is_err() {
            // The terminal metadata has already been removed. Let the next
            // checkpoint discard its stale index entry even if this write fails.
            file.reopened(id);
        }
        result?;
    }
    Ok(())
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
    for (id, g) in &snapshot.closed_geometry {
        if !g.valid(id) {
            return Err("닫힌 창의 위치 또는 크기가 유효하지 않습니다".into());
        }
    }
    let mut labels = BTreeSet::new();
    for entry in &snapshot.windows {
        if entry.work_area_restore.as_ref().is_some_and(|r| !r.valid()) {
            return Err("잘못된 작업 영역 확대 복원 상태".into());
        }
        if !supported(&entry.window.label)
            || snapshot.closed_geometry.contains_key(&entry.window.label)
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
    let Some((snapshot, file)) = JournalFile::load(&path)? else {
        return Ok(false);
    };
    crate::monitor_layout::restore(app, snapshot.monitor_layout)?;
    app.manage(Journal {
        file: Mutex::new(file),
        ..Default::default()
    });
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
        closed_geometry: Default::default(),
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
    let Some(generation) = journal.collection_generation()? else {
        return Ok(());
    };
    let revision = journal.file.lock().map_err(|e| e.to_string())?.revision;
    let Some(snapshot) = collect()? else {
        return Ok(());
    };
    let mut file = journal.file.lock().map_err(|e| e.to_string())?;
    // A close may have started during collection or while waiting for file
    // access. Any later close persists its transition after this writer.
    if journal.collection_generation()? != Some(generation) {
        return Ok(());
    }
    file.checkpoint(root, revision, snapshot)
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
        let mut undo = None;
        if let Some(mut snapshot) = file.read_current(root)? {
            let entry = snapshot
                .windows
                .iter()
                .position(|e| e.window.label == label)
                .map(|index| (index, snapshot.windows.remove(index)));
            let layout = snapshot.monitor_layout.take_record(label);
            if entry.is_some() || layout.is_some() {
                file.persist(root, &snapshot)?;
                undo = Some((entry, layout));
            }
        }
        undo
    };
    if let Err(error) = destroy() {
        if let Some((entry, layout)) = undo {
            let rollback = || -> Result<(), String> {
                let mut file = journal.file.lock().map_err(|e| e.to_string())?;
                let mut snapshot = file
                    .read_current(root)?
                    .ok_or("시작 기록을 찾을 수 없습니다")?;
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
                file.persist(root, &snapshot)
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
    let mut enabled = enabled;
    if app.try_state::<Journal>().is_none() {
        // Update restoration may skip normal startup restoration. Keep the
        // closed-window index without applying the old open-window list.
        let file = if enabled {
            match JournalFile::load(&app.state::<crate::storage::Store>().0.join(FILE)) {
                Ok(saved) => saved.map(|(_, file)| file).unwrap_or_default(),
                Err(_) => {
                    enabled = false;
                    eprintln!("시작 창 상태 파일을 보존하기 위해 저장을 중지했습니다.");
                    JournalFile::default()
                }
            }
        } else {
            JournalFile::default()
        };
        app.manage(Journal {
            file: Mutex::new(file),
            ..Default::default()
        });
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
    fn read_snapshot(root: &Path) -> Result<Option<Snapshot>, String> {
        Ok(JournalFile::load(&root.join(FILE))?.map(|(snapshot, _)| snapshot))
    }
    fn sample() -> Snapshot {
        serde_json::from_value(serde_json::json!({"version":1,"windows":[{"window":{"label":"webview-web-a","x":-100,"y":20,"width":600,"height":24,"visible":false,"minimized":false,"always_on_top":true},"expanded":[600,400],"url":"https://example.com/last"}]})).unwrap()
    }
    fn moved() -> ReopenGeometry {
        ReopenGeometry {
            bounds: crate::capture::ImageNote {
                x: -1740.,
                y: 42.,
                width: 803.5,
                height: 411.,
            },
            physical_position: true,
        }
    }

    #[test]
    fn checkpoint_close_reload_and_reopen_preserve_distinct_window_lifecycles() {
        let root = tempfile::tempdir().unwrap();
        let id = "terminal-42";
        let open = || {
            let mut s = sample();
            let mut terminal = sample().windows.pop().unwrap();
            terminal.url = None;
            terminal.expanded = None;
            terminal.window.label = id.into();
            terminal.terminal_command = Some("pwd".into());
            s.windows.push(terminal);
            s
        };
        let mut file = JournalFile::default();
        file.checkpoint(root.path(), 0, open()).unwrap();
        // The v1 journal has no new field until a terminal actually closes.
        assert!(!String::from_utf8_lossy(&file.last).contains("closed_geometry"));
        file.set_closed(root.path(), id, Some(moved())).unwrap();
        let committed = file.last.clone();
        file.checkpoint(root.path(), 0, open()).unwrap();
        assert_eq!(file.last, committed); // stale collection cannot reopen it
        let (disk, mut loaded) = JournalFile::load(&root.path().join(FILE)).unwrap().unwrap();
        assert_eq!(disk.windows.len(), 1);
        assert_eq!(
            disk.windows[0].url.as_deref(),
            Some("https://example.com/last")
        );
        assert!(loaded.closed[id].physical_position);
        assert_eq!(loaded.closed[id].bounds.width, 803.5);
        loaded.reopened(id);
        loaded
            .checkpoint(root.path(), loaded.revision, open())
            .unwrap();
        let (disk, mut loaded) = JournalFile::load(&root.path().join(FILE)).unwrap().unwrap();
        assert!(loaded.closed.is_empty());
        assert_eq!(disk.windows[1].terminal_command.as_deref(), Some("pwd"));
        loaded.set_closed(root.path(), id, Some(moved())).unwrap();
        loaded.set_closed(root.path(), id, None).unwrap();
        assert!(JournalFile::load(&root.path().join(FILE))
            .unwrap()
            .unwrap()
            .1
            .closed
            .is_empty());
    }

    #[test]
    fn moving_open_windows_uses_the_shared_checkpoint_and_skips_equal_frames() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = |step: u32| {
            let mut s = sample();
            for label in ["note-text", "note-image", "note-capture", "terminal-12"] {
                let mut e = sample().windows.pop().unwrap();
                e.window.label = label.to_owned();
                e.url = None;
                e.window.width += step;
                s.windows.push(e);
            }
            s
        };
        let mut file = JournalFile::default();
        file.checkpoint(root.path(), 0, snapshot(0)).unwrap();
        file.checkpoint(root.path(), 0, snapshot(240)).unwrap();
        // An equal checkpoint succeeds even with an unusable destination: it
        // must return before directory creation, temp-file I/O or rename.
        file.checkpoint(&root.path().join(FILE), 0, snapshot(240))
            .unwrap();
        let (s, _) = JournalFile::load(&root.path().join(FILE)).unwrap().unwrap();
        assert_eq!(s.windows.len(), 5);
        assert!(s.windows.iter().skip(1).all(|e| e.window.width == 840));
        assert!(s.closed_geometry.is_empty());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_close_keeps_last_good_journal_and_first_close_does_not_invent_an_empty_session() {
        let root = tempfile::tempdir().unwrap();
        let id = "terminal-10";
        let mut file = JournalFile::default();
        file.set_closed(root.path(), id, Some(moved())).unwrap();
        assert!(!root.path().join(FILE).exists());
        file.checkpoint(root.path(), file.revision, sample())
            .unwrap();
        let original = fs::read(root.path().join(FILE)).unwrap();
        assert!(file.set_closed(&root.path().join(FILE), id, None).is_err());
        assert!(file.closed.contains_key(id));
        assert_eq!(file.last, original);
        assert_eq!(fs::read(root.path().join(FILE)).unwrap(), original);
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
    fn native_close_guard_never_waits_for_the_journal_writer() {
        let journal = Journal::default();
        std::thread::scope(|threads| {
            let writer = journal.file.lock().unwrap();
            let (done, finished) = std::sync::mpsc::channel();
            let shared = &journal;
            threads.spawn(move || {
                let result = shared.during_close(|| {
                    assert_eq!(shared.collection_generation()?, None);
                    Ok(true)
                });
                let _ = done.send(result);
            });
            let result = finished.recv_timeout(Duration::from_secs(2));
            drop(writer);
            assert_eq!(result.unwrap(), Ok(true));
        });
        assert_eq!(journal.collection_generation().unwrap(), Some(1));
    }

    #[test]
    fn web_rollback_preserves_nested_terminal_close_and_its_final_geometry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let journal = Journal::default();
        let id = "terminal-42";
        let mixed = || {
            let mut s = sample();
            let mut t = sample().windows.remove(0);
            t.window.label = id.into();
            t.url = None;
            s.windows.push(t);
            s
        };
        checkpoint_file(&journal, root, || Ok(Some(mixed()))).unwrap();
        checkpoint_file(&journal, root, || {
            let failed = journal.during_close(|| {
                forget_saved_web(&journal, root, "webview-web-a", || {
                    journal.during_close(|| {
                        journal
                            .file
                            .lock()
                            .unwrap()
                            .closed_after_destroy(root, id, moved())
                    })?;
                    Err("web dispatch failed".into())
                })
            });
            assert_eq!(failed, Err("web dispatch failed".into()));
            Ok(Some(mixed()))
        })
        .unwrap();
        let restored = read_snapshot(root).unwrap().unwrap();
        assert_eq!(restored.windows.len(), 1);
        assert_eq!(restored.windows[0].window.label, "webview-web-a");
        assert_eq!(restored.closed_geometry[id].bounds.x, -1740.);
        assert!(restored.closed_geometry[id].physical_position);
        assert_eq!(
            journal.file.lock().unwrap().last,
            fs::read(root.join(FILE)).unwrap()
        );
    }

    #[test]
    fn failed_terminal_close_write_recovers_via_checkpoint_after_web_close() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let journal = Journal::default();
        let id = "terminal-10";
        checkpoint_file(&journal, root, || Ok(Some(sample()))).unwrap();
        let original = fs::read(root.join(FILE)).unwrap();
        assert!(journal
            .during_close(|| {
                journal
                    .file
                    .lock()
                    .unwrap()
                    .closed_after_destroy(&root.join(FILE), id, moved())
            })
            .is_err());
        assert_eq!(fs::read(root.join(FILE)).unwrap(), original);
        journal
            .during_close(|| forget_saved_web(&journal, root, "webview-web-a", || Ok(())))
            .unwrap();
        // The web transition read a durable file without the failed frame.
        // The in-memory frame must still be present for the next full snapshot.
        checkpoint_file(&journal, root, || read_snapshot(root)).unwrap();
        let restored = read_snapshot(root).unwrap().unwrap();
        assert!(restored.windows.is_empty());
        assert_eq!(restored.closed_geometry[id].bounds.width, 803.5);
        assert!(restored.closed_geometry[id].physical_position);
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
