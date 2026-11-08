//! Local PTYs, scoped to the calling terminal window. Shell commands stay backend-owned; selected directories are validated.
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{ipc::Channel, Manager, WebviewWindow};

pub mod welcome;
pub use welcome::{create as create_terminal, request_close};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Terminal {
    pub id: String,
    pub bounds: crate::capture::ImageNote,
    // Older terminal metadata stores logical origins. New Windows selections
    // and geometry use physical desktop origins; keep the distinction on disk.
    #[serde(default)]
    pub physical_position: bool,
    #[serde(default)]
    pub kind: Kind,
    #[serde(default)]
    pub last_path: Option<PathBuf>,
    #[serde(default)]
    pub recent_paths: Vec<RecentPath>,
    #[serde(default)]
    pub path_updated: u64,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub title: String,
    /// Identifier from the fixed Welcome choices only; never a shell command.
    #[serde(default)]
    pub last_command: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RecentPath {
    pub path: PathBuf,
    pub updated: u64,
}
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Terminal,
}
impl Terminal {
    fn position(&self) -> tauri::Position {
        #[cfg(windows)]
        if self.physical_position {
            return tauri::PhysicalPosition::new(
                self.bounds.x.round() as i32,
                self.bounds.y.round() as i32,
            )
            .into();
        }
        tauri::LogicalPosition::new(self.bounds.x, self.bounds.y).into()
    }
    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.clone();
        }
        self.last_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "터미널".into())
    }
}
// Reap after Session's PTY handles are dropped; waiting before closing those handles
// can deadlock shell exit on macOS if no reader was successfully attached.
struct OwnedChild(Option<Box<dyn Child + Send + Sync>>);
impl std::ops::Deref for OwnedChild {
    type Target = dyn Child + Send + Sync;
    fn deref(&self) -> &Self::Target {
        self.0.as_deref().unwrap()
    }
}
impl std::ops::DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_deref_mut().unwrap()
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: OwnedChild,
    program: String,
    prompt: welcome::PromptState,
}
impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(pid) = self
            .child
            .process_id()
            .filter(|_| self.child.try_wait().ok().flatten().is_none())
        {
            let _ = std::process::Command::new("taskkill.exe")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output();
        }
        #[cfg(unix)]
        if self.child.try_wait().ok().flatten().is_none() {
            if let Some(pid) = self.child.process_id() {
                // Include background jobs: job-control shells give them separate groups.
                if let Some(children) = welcome::descendants(pid) {
                    for child in children.into_iter().rev() {
                        unsafe {
                            libc::kill(child, libc::SIGKILL);
                        }
                    }
                }
                // portable-pty creates a new session; signal only our child group.
                unsafe {
                    if let Some(fg) = self
                        .master
                        .process_group_leader()
                        .filter(|fg| *fg > 0 && *fg != pid as i32)
                    {
                        libc::kill(-fg, libc::SIGKILL);
                    }
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
        }
        let _ = self.child.kill();
    }
}
enum Slot {
    Starting(u64),
    Running(Arc<Mutex<Session>>),
}
#[derive(Default)]
pub struct Terminals {
    records: Mutex<BTreeMap<String, Terminal>>,
    sessions: Mutex<BTreeMap<String, Slot>>,
    restart_notices: Mutex<BTreeMap<String, Option<String>>>,
}
struct Start<'a> {
    state: &'a Terminals,
    id: String,
    token: u64,
}
impl<'a> Start<'a> {
    fn reserve(state: &'a Terminals, id: &str) -> Result<Self, String> {
        let mut sessions = state.sessions.lock().map_err(err)?;
        if sessions.contains_key(id) {
            return Err("이미 시작 중이거나 시작된 세션입니다. 창을 숨겼다가 복원하세요".into());
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let token = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        sessions.insert(id.into(), Slot::Starting(token));
        Ok(Self {
            state,
            id: id.into(),
            token,
        })
    }
    fn finish(self, session: Session) -> Result<Arc<Mutex<Session>>, String> {
        let mut sessions = self.state.sessions.lock().map_err(err)?;
        if !matches!(sessions.get(&self.id), Some(Slot::Starting(token)) if *token == self.token) {
            return Err("터미널 시작이 취소되었습니다".into());
        }
        let session = Arc::new(Mutex::new(session));
        sessions.insert(self.id.clone(), Slot::Running(session.clone()));
        Ok(session)
    }
}
impl Drop for Start<'_> {
    fn drop(&mut self) {
        let mut sessions = self.state.sessions.lock().unwrap();
        if matches!(sessions.get(&self.id), Some(Slot::Starting(token)) if *token == self.token) {
            sessions.remove(&self.id);
        }
    }
}
fn running(slot: &Slot) -> Option<Arc<Mutex<Session>>> {
    match slot {
        Slot::Starting(_) => None,
        Slot::Running(s) => Some(s.clone()),
    }
}
fn root(app: &tauri::AppHandle) -> PathBuf {
    app.state::<crate::storage::Store>().0.clone()
}
fn metadata(root: &Path) -> PathBuf {
    root.join(".agent-gibbon-terminals")
}
fn valid_id(id: &str) -> bool {
    id.strip_prefix("terminal-")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        && id.len() <= 64
}
fn save(root: &Path, agent: &Terminal) -> Result<(), String> {
    std::fs::create_dir_all(metadata(root)).map_err(err)?;
    let path = metadata(root).join(format!("{}.json", agent.id));
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(agent).map_err(err)?).map_err(err)?;
    std::fs::rename(tmp, path).map_err(err)
}
// Read legacy dedicated-agent metadata only to recognize and ignore it. Never
// rewrite/remove its metadata or project directory, and never load it as a PTY.
fn read_records(root: &Path) -> Result<Vec<Terminal>, String> {
    let path = metadata(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(path).map_err(err)? {
        let path = entry.map_err(err)?.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let value: Value =
            serde_json::from_slice(&std::fs::read(path).map_err(err)?).map_err(err)?;
        if value["kind"] == "agent"
            || value["id"]
                .as_str()
                .is_some_and(crate::model::retired_agent_id)
        {
            continue;
        }
        let record: Terminal = serde_json::from_value(value).map_err(err)?;
        if !valid_id(&record.id) {
            return Err("잘못된 터미널 ID".into());
        }
        records.push(record);
    }
    Ok(records)
}
pub fn restore(app: &tauri::AppHandle) -> Result<(), String> {
    let restored = read_records(&root(app))?;
    let state = app.state::<Terminals>();
    let mut records = state.records.lock().map_err(err)?;
    for record in restored {
        records.insert(record.id.clone(), record);
    }
    Ok(())
}
pub fn entries(app: &tauri::AppHandle) -> Vec<Terminal> {
    app.state::<Terminals>()
        .records
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect()
}
pub fn contains(app: &tauri::AppHandle, id: &str) -> bool {
    app.state::<Terminals>()
        .records
        .lock()
        .unwrap()
        .contains_key(id)
}
pub fn open(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    open_with_visibility(app, id, true)
}

pub fn open_hidden(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    open_with_visibility(app, id, false)
}

pub fn open_hidden_for_restart(
    app: &tauri::AppHandle,
    id: &str,
    choice: Option<String>,
) -> Result<(), String> {
    // Publish the one-shot notice before constructing the WebView, which can
    // begin invoking terminal_welcome as soon as it finishes loading.
    mark_restart_notice(app, id, choice)?;
    if let Err(error) = open_hidden(app, id) {
        if let Ok(mut notices) = app.state::<Terminals>().restart_notices.lock() {
            notices.remove(id);
        }
        return Err(error);
    }
    Ok(())
}

fn open_with_visibility(app: &tauri::AppHandle, id: &str, visible: bool) -> Result<(), String> {
    let _root = crate::telemetry::Timing::new("terminal.window.open");
    let agent = app
        .state::<Terminals>()
        .records
        .lock()
        .map_err(err)?
        .get(id)
        .cloned()
        .ok_or("터미널 창이 없습니다")?;
    crate::focus::remember(app, id);
    let win = if let Some(win) = app.get_webview_window(id) {
        win
    } else {
        let window = {
            let _creation = crate::telemetry::Timing::new("terminal.window.create");
            crate::telemetry::checkpoint("terminal.window.create");
            tauri::WebviewWindowBuilder::new(
                app,
                id,
                tauri::WebviewUrl::App("terminal.html".into()),
            )
            .on_web_resource_request(|_, response| {
                // xterm generates cell/theme CSS at runtime. Scope this exception
                // to terminal WebViews; script policy and memo windows stay strict.
                if let Some(header) = response.headers_mut().get_mut("Content-Security-Policy") {
                    if let Ok(policy) = header.to_str() {
                        let mut directives: std::collections::HashMap<
                            String,
                            tauri::utils::config::CspDirectiveSources,
                        > = tauri::utils::config::Csp::Policy(policy.into()).into();
                        directives.insert(
                            "style-src".into(),
                            tauri::utils::config::CspDirectiveSources::List(vec![
                                "'self'".into(),
                                "'unsafe-inline'".into(),
                            ]),
                        );
                        if let Ok(value) = tauri::utils::config::Csp::from(directives)
                            .to_string()
                            .parse()
                        {
                            *header = value;
                        }
                    }
                }
            })
            .title(format!("{} — {id}", crate::instance::current().title()))
            .decorations(false)
            .resizable(true)
            .min_inner_size(2.0, 2.0)
            .inner_size(agent.bounds.width, agent.bounds.height)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
            .build()
            .map_err(err)?
        };
        {
            let _position = crate::telemetry::Timing::new("terminal.window.position");
            // Position before showing. Legacy logical origins remain compatible;
            // captured Windows origins are already physical desktop coordinates.
            window.set_position(agent.position()).map_err(err)?;
            #[cfg(windows)]
            {
                // Size using the target monitor's DPI after positioning.
                window
                    .set_size(tauri::LogicalSize::new(
                        agent.bounds.width,
                        agent.bounds.height,
                    ))
                    .map_err(err)?;
            }
        }
        window
    };
    let _show = crate::telemetry::Timing::new("terminal.window.show");
    if crate::opacity::configured(app).is_some() {
        crate::sync_opacity(win.clone())?;
    }
    // User explicitly opens a terminal, but background CLI/tray restore preserves focus.
    if visible {
        #[cfg(target_os = "macos")]
        unsafe {
            (&*(win.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow))
                .orderFrontRegardless();
        }
        #[cfg(not(target_os = "macos"))]
        crate::stash::release(&win)?;
        win.show().map_err(err)?;
        #[cfg(any(target_os = "macos", windows))]
        crate::focus::restore(&win);
    } else {
        // WebView construction may activate the app even when the window stays
        // hidden. Close the focus-preservation transaction without showing it.
        #[cfg(any(target_os = "macos", windows))]
        crate::focus::restore(&win);
    }
    Ok(())
}

fn safe_choice_id(choice: &str) -> Option<&'static str> {
    match choice {
        "shell" => Some("shell"),
        "codex" => Some("codex"),
        "claude" => Some("claude"),
        "git" => Some("git"),
        "htop" => Some("htop"),
        _ => None,
    }
}

pub fn restart_notice(app: &tauri::AppHandle, id: &str) -> Result<Option<String>, String> {
    Ok(app
        .state::<Terminals>()
        .restart_notices
        .lock()
        .map_err(err)?
        .remove(id)
        .map(|choice| restart_notice_text(choice.as_deref())))
}

fn restart_notice_text(choice: Option<&str>) -> String {
    let Some(choice) = choice else {
        return "이전 터미널 세션은 종료됐습니다. 기록된 마지막 선택이 없으며 자동 실행되지 않았습니다.".into();
    };
    let label = match choice {
        "codex" => "Codex",
        "claude" => "Claude Code",
        "git" => "git log",
        "htop" => "htop",
        _ => "셸",
    };
    format!("이전 터미널 세션은 종료됐습니다. 마지막 선택은 {label}이며 자동 실행되지 않았습니다.")
}

pub fn mark_restart_notice(
    app: &tauri::AppHandle,
    id: &str,
    choice: Option<String>,
) -> Result<(), String> {
    let choice = choice
        .as_deref()
        .and_then(safe_choice_id)
        .map(str::to_string);
    app.state::<Terminals>()
        .restart_notices
        .lock()
        .map_err(err)?
        .insert(id.into(), choice);
    Ok(())
}

pub fn last_command(app: &tauri::AppHandle, id: &str) -> Option<String> {
    app.state::<Terminals>()
        .records
        .lock()
        .ok()?
        .get(id)?
        .last_command
        .clone()
}
pub fn inspect(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    welcome::refresh_path(app, id)?;
    let agent = app
        .state::<Terminals>()
        .records
        .lock()
        .map_err(err)?
        .get(id)
        .cloned()
        .ok_or("터미널 창이 없습니다")?;
    if !contains(app, id) {
        return Err("터미널 창이 없습니다".into());
    }
    let win = app.get_webview_window(id);
    let session = app
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .and_then(running);
    let (pid, program, size, running) = if let Some(session) = session {
        let mut s = session.lock().map_err(err)?;
        let running = observe_exit(&mut s)?;
        (
            s.child.process_id(),
            Some(s.program.clone()),
            Some(s.master.get_size().map_err(err)?),
            running,
        )
    } else {
        (None, None, None, false)
    };
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":agent.kind,"title":agent.display_title(),"cwd":agent.last_path.clone(),"last_path":agent.last_path,"bounds":agent.bounds,"pid":pid,"program":program,"running":running,"pty_size":size.map(|s|json!({"cols":s.cols,"rows":s.rows})),"visible":win.as_ref().is_some_and(|w|w.is_visible().unwrap_or(false)),"position":win.as_ref().and_then(|w|w.outer_position().ok()),"size":win.as_ref().and_then(|w|w.inner_size().ok())}),
    )
}
pub fn handle(app: &tauri::AppHandle, request: &crate::Request) -> Result<Value, String> {
    let id = request.id();
    match request.action {
        crate::Action::Open => open(app, id)?,
        crate::Action::Close => {
            if let Some(win) = app.get_webview_window(id) {
                crate::stash::release(&win)?;
                win.hide().map_err(err)?;
                crate::tray::sync_visibility(app)?;
            }
        }
        crate::Action::Inspect => (),
        _ => return Err("터미널 창은 내용 갱신을 지원하지 않습니다".into()),
    }
    inspect(app, id)
}
fn caller(window: &WebviewWindow) -> Result<String, String> {
    let id = window.label();
    if !contains(window.app_handle(), id) {
        return Err("터미널 창이 아닙니다".into());
    }
    Ok(id.into())
}
fn size(cols: u16, rows: u16) -> Result<PtySize, String> {
    if !(2..=1000).contains(&cols) || !(1..=1000).contains(&rows) {
        return Err("터미널 크기 범위를 벗어났습니다".into());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}
fn observe_exit(session: &mut Session) -> Result<bool, String> {
    Ok(session.child.try_wait().map_err(err)?.is_none())
}

#[tauri::command]
pub async fn terminal_start(
    window: WebviewWindow,
    cols: u16,
    rows: u16,
    output: Channel<Vec<u8>>,
    choice: Option<String>,
    local_flow: Option<u64>,
) -> Result<Value, String> {
    let _update_activity = crate::updater::activity(window.app_handle())?;
    let mut worker_call = {
        let _scope = crate::local_trace::webview_scope(&window, local_flow);
        crate::local_trace::Call::begin("terminal.worker", crate::local_trace::Stage::UiQueue)
    };
    let id = caller(&window)?;
    let app = window.app_handle().clone();
    let sentry_parent = crate::telemetry::TimingContext::request("terminal.request");
    tauri::async_runtime::spawn_blocking(move || {
        let _sentry_scope = sentry_parent.enter();
        let _scope = worker_call.scope();
        worker_call.stage(crate::local_trace::Stage::UiExecute);
        let result = (|| {
            let _timing = crate::telemetry::Timing::new("terminal.start");
            let dimensions = size(cols, rows)?;
            let state = app.state::<Terminals>();
            let reservation = Start::reserve(&state, &id)?;
            let agent = state
                .records
                .lock()
                .map_err(err)?
                .get(&id)
                .cloned()
                .ok_or("터미널 창이 없습니다")?;
            let (session, mut reader) =
                welcome::start(&agent, choice.as_deref().unwrap_or("shell"), dimensions)?;
            let selected = safe_choice_id(choice.as_deref().unwrap_or("shell"))
                .ok_or("지원하지 않는 실행 항목입니다")?;
            {
                let mut records = state.records.lock().map_err(err)?;
                let record = records.get_mut(&id).ok_or("터미널 창이 없습니다")?;
                record.last_command = Some(selected.to_string());
                save(&root(&app), record)?;
            }
            app.state::<Terminals>()
                .restart_notices
                .lock()
                .map_err(err)?
                .remove(&id);
            let watched_session = Arc::downgrade(&reservation.finish(session)?);
            welcome::touch_path(&app, &id)?;
            let watched_app = app.clone();
            let watched_id = id.clone();
            let output_parent = crate::local_trace::current();
            // Raw bytes preserve UTF-8 across arbitrary PTY read boundaries.
            std::thread::spawn(move || {
                let mut directory_report = welcome::DirectoryReport::default();
                let mut bytes = [0u8; 8192];
                loop {
                    match reader.read(&mut bytes) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let _scope = crate::local_trace::Scope::enter(output_parent);
                            let _timing = crate::telemetry::Timing::new("terminal.output");
                            let paths = {
                                let _filter =
                                    crate::telemetry::Timing::new("terminal.output.filter");
                                directory_report.feed(&bytes[..n])
                            };
                            let state = watched_app.state::<Terminals>();
                            let sessions = state.sessions.lock().unwrap();
                            if sessions
                                .get(&watched_id)
                                .and_then(running)
                                .is_some_and(|s| {
                                    std::ptr::eq(Arc::as_ptr(&s), watched_session.as_ptr())
                                })
                            {
                                let Some(watched_session) = watched_session.upgrade() else {
                                    break;
                                };
                                let mut s = watched_session.lock().unwrap();
                                if directory_report.take_prompt(&s.prompt.token) {
                                    s.prompt.report();
                                }
                                drop(s);
                                for path in paths {
                                    let _ = welcome::set_path(&watched_app, &watched_id, path);
                                }
                            }
                            drop(sessions);
                            // Keep draining after the webview closes: a full PTY can block shell exit.
                            let _dispatch =
                                crate::telemetry::Timing::new("terminal.output.dispatch");
                            let _ = output.send(bytes[..n].to_vec());
                        }
                    }
                }
                if let Some(session) = watched_session.upgrade() {
                    let _ = observe_exit(&mut session.lock().unwrap());
                }
                let _ = output.send(
                    b"\r\n[PTY \xec\x84\xb8\xec\x85\x98 \xec\xa2\x85\xeb\xa3\x8c]\r\n".to_vec(),
                );
            });
            inspect(&app, &id)
        })();
        worker_call.finish(result.is_ok());
        result
    })
    .await
    .map_err(err)?
}
fn session(window: &WebviewWindow) -> Result<Arc<Mutex<Session>>, String> {
    let id = caller(window)?;
    window
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(&id)
        .and_then(running)
        .ok_or("실행 중인 세션이 없습니다".into())
}
#[tauri::command]
pub fn terminal_write(window: WebviewWindow, data: String) -> Result<(), String> {
    let _timing = crate::telemetry::Timing::new("terminal.write");
    if data.len() > 65536 {
        return Err("입력은 64KiB 이하여야 합니다".into());
    }
    let session = session(&window)?;
    let mut session = {
        let _lock = crate::telemetry::Timing::new("terminal.session.lock");
        session.lock().map_err(err)?
    };
    session.prompt.input(data.as_bytes());
    let write = crate::telemetry::Timing::new("terminal.input.write");
    session.writer.write_all(data.as_bytes()).map_err(err)?;
    session.writer.flush().map_err(err)?;
    drop(write);
    drop(session);
    welcome::touch_path(window.app_handle(), &caller(&window)?)
}
#[tauri::command]
pub fn terminal_resize(window: WebviewWindow, cols: u16, rows: u16) -> Result<(), String> {
    let _timing = crate::telemetry::Timing::new("terminal.resize");
    session(&window)?
        .lock()
        .map_err(err)?
        .master
        .resize(size(cols, rows)?)
        .map_err(err)
}
pub fn terminal_hide(window: WebviewWindow) -> Result<(), String> {
    caller(&window)?;
    crate::stash::release(&window)?;
    window.hide().map_err(err)
}
#[tauri::command]
pub fn terminal_info(window: WebviewWindow) -> Result<Value, String> {
    let id = caller(&window)?;
    inspect(window.app_handle(), &id)
}
pub fn geometry(window: &tauri::Window) {
    let app = window.app_handle();
    if app
        .get_webview_window(window.label())
        .is_some_and(|w| crate::stash::owns(&w))
    {
        return;
    }
    let state = app.state::<Terminals>();
    let Ok(mut agents) = state.records.lock() else {
        return;
    };
    if let Some(agent) = agents.get_mut(window.label()) {
        if let (Ok(p), Ok(s), Ok(scale)) = (
            window.outer_position(),
            window.inner_size(),
            window.scale_factor(),
        ) {
            agent.bounds = crate::capture::ImageNote {
                x: f64::from(p.x) / if cfg!(windows) { 1.0 } else { scale },
                y: f64::from(p.y) / if cfg!(windows) { 1.0 } else { scale },
                width: f64::from(s.width) / scale,
                height: f64::from(s.height) / scale,
            };
            agent.physical_position = cfg!(windows);
            let _ = save(&root(app), agent);
        }
    }
}
pub fn update_blocked(app: &tauri::AppHandle) -> Result<bool, String> {
    Ok(!app
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .is_empty())
}

pub fn starting(app: &tauri::AppHandle) -> Result<bool, String> {
    Ok(app
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .values()
        .any(|slot| matches!(slot, Slot::Starting(_))))
}

/// Read the actual live PTY PID without collecting terminal info or history.
#[allow(dead_code)] // Consumed by the explicit update impact review in the dependent #116 change.
pub fn review_pid(app: &tauri::AppHandle, id: &str) -> Result<Option<u32>, String> {
    let session = app
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .and_then(running);
    let Some(session) = session else {
        return Ok(None);
    };
    let mut session = session.lock().map_err(err)?;
    if session.child.try_wait().map_err(err)?.is_some() {
        return Ok(None);
    }
    Ok(session.child.process_id())
}

pub fn shutdown(app: &tauri::AppHandle) {
    for agent in entries(app) {
        let _ = welcome::refresh_path(app, &agent.id);
    }
    app.state::<Terminals>().sessions.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_logical_and_windows_physical_origins_survive_restore() {
        let legacy = json!({"id":"terminal-123","bounds":{"x":-1600.,"y":240.,"width":720.,"height":460.},"kind":"terminal"});
        let mut agent: Terminal = serde_json::from_value(legacy).unwrap();
        assert!(!agent.physical_position);
        let old: tauri::PhysicalPosition<i32> = agent.position().to_physical(1.25);
        assert_eq!((old.x, old.y), (-2000, 300));
        agent.physical_position = true;
        let restored: Terminal =
            serde_json::from_value(serde_json::to_value(agent).unwrap()).unwrap();
        assert!(restored.physical_position);
        assert_eq!((restored.bounds.x, restored.bounds.y), (-1600., 240.));
        #[cfg(windows)]
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let position: tauri::PhysicalPosition<i32> = restored.position().to_physical(scale);
            assert_eq!((position.x, position.y), (-1600, 240));
        }
    }
    #[test]
    fn approved_recent_folders_survive_metadata_roundtrip() {
        let value = json!({"id":"terminal-123","bounds":{"x":0.,"y":0.,"width":720.,"height":460.},"kind":"terminal","recent_paths":[{"path":"/tmp/project","updated":42}]});
        let restored =
            serde_json::to_value(serde_json::from_value::<Terminal>(value.clone()).unwrap())
                .unwrap();
        assert_eq!(restored["recent_paths"], value["recent_paths"]);
    }
    #[test]
    fn welcome_metadata_preserves_only_last_path_title_and_geometry() {
        let original = json!({"id":"terminal-123","bounds":{"x":11.,"y":22.,"width":720.,"height":460.},"kind":"terminal","last_path":"/tmp/project","path_updated":42,"title":"내 작업"});
        let agent: Terminal = serde_json::from_value(original.clone()).unwrap();
        let restored = serde_json::to_value(agent).unwrap();
        for key in ["kind", "last_path", "path_updated", "title", "bounds"] {
            assert_eq!(restored.get(key), original.get(key), "lost {key}");
        }
        assert!(restored.get("history").is_none());
        assert!(restored.get("commands").is_none());
        assert!(valid_id("terminal-123"));
        assert!(!valid_id("terminal-../x"));
    }
    #[test]
    fn retired_agent_records_are_ignored_without_changing_metadata_or_projects() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(metadata(dir.path())).unwrap();
        let records = [
            (
                "agent-123",
                json!({"id":"agent-123","bounds":{"x":1.,"y":2.,"width":640.,"height":400.}}),
            ),
            (
                "agent-456",
                json!({"id":"agent-456","kind":"agent","bounds":{"x":3.,"y":4.,"width":640.,"height":400.}}),
            ),
            (
                "terminal-789",
                json!({"id":"terminal-789","kind":"terminal","title":"保管","last_path":"/tmp","bounds":{"x":5.,"y":6.,"width":640.,"height":400.}}),
            ),
        ];
        for (id, value) in &records {
            std::fs::write(
                metadata(dir.path()).join(format!("{id}.json")),
                serde_json::to_vec(value).unwrap(),
            )
            .unwrap();
            std::fs::create_dir(dir.path().join(id)).unwrap();
            std::fs::write(dir.path().join(id).join("index.md"), "user project").unwrap();
        }
        for _ in 0..2 {
            let loaded = read_records(dir.path()).unwrap();
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].id, "terminal-789");
            assert_eq!(loaded[0].title, "保管");
        }
        for (id, value) in &records {
            assert_eq!(
                std::fs::read(metadata(dir.path()).join(format!("{id}.json"))).unwrap(),
                serde_json::to_vec(value).unwrap()
            );
            assert_eq!(
                std::fs::read_to_string(dir.path().join(id).join("index.md")).unwrap(),
                "user project"
            );
        }
    }
    #[test]
    fn reservation_is_per_window_and_released_on_failure() {
        let state = Terminals::default();
        let first = Start::reserve(&state, "terminal-1").unwrap();
        assert!(Start::reserve(&state, "terminal-1").is_err());
        let other = Start::reserve(&state, "terminal-2").unwrap();
        assert!(state.sessions.try_lock().is_ok());
        drop(first);
        assert!(Start::reserve(&state, "terminal-1").is_ok());
        drop(other);
        assert!(state.sessions.lock().unwrap().is_empty());
        assert!(!valid_id("agent-../x"));
        assert!(!valid_id("agent-123"));
        assert!(size(0, 24).is_err());
    }
    #[test]
    fn metadata_does_not_claim_project_index() {
        let dir = tempfile::tempdir().unwrap();
        let a = Terminal {
            physical_position: false,
            kind: Kind::Terminal,
            last_path: None,
            recent_paths: Vec::new(),
            path_updated: 0,
            updated_at: 0,
            title: String::new(),
            last_command: None,
            id: "terminal-123".into(),
            bounds: crate::capture::ImageNote {
                x: 1.,
                y: 2.,
                width: 640.,
                height: 400.,
            },
        };
        std::fs::create_dir(dir.path().join(&a.id)).unwrap();
        let index = dir.path().join(&a.id).join("index.md");
        std::fs::write(&index, "user project").unwrap();
        save(dir.path(), &a).unwrap();
        assert_eq!(std::fs::read_to_string(index).unwrap(), "user project");
    }
    #[test]
    fn restart_notice_never_claims_an_unrecorded_choice() {
        assert!(restart_notice_text(None).contains("기록된 마지막 선택이 없으며"));
        assert!(restart_notice_text(Some("codex")).contains("Codex"));
        assert_eq!(safe_choice_id("claude"), Some("claude"));
        assert_eq!(safe_choice_id("echo secret"), None);
    }
}
