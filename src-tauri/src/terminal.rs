//! Local PTYs, scoped to the calling terminal window. Shell commands stay backend-owned; selected directories are validated.
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, Emitter, Manager, WebviewWindow};

#[cfg(test)]
use std::io::Read;
mod jobs;
use jobs::OwnedJobs;
mod lifecycle;
#[cfg(all(test, unix))]
mod lifecycle_tests;
use lifecycle::{Control, Event, OwnedChild, PtyReader, Worker, Workers};
mod persistence;
use persistence::EditSource;
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
    #[serde(default)]
    pub restore_plan: Option<String>,
    #[serde(default)]
    pub auto_restore: bool,
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
struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: OwnedChild,
    program: String,
    observed_command: bool,
    prompt: welcome::PromptState,
}
enum Slot {
    Starting {
        token: u64,
        control: Control,
    },
    Running {
        token: u64,
        control: Control,
        session: Arc<Mutex<Session>>,
    },
    Exited {
        token: u64,
        status: portable_pty::ExitStatus,
        jobs: OwnedJobs,
    },
}
impl Slot {
    fn token(&self) -> u64 {
        match self {
            Self::Starting { token, .. }
            | Self::Running { token, .. }
            | Self::Exited { token, .. } => *token,
        }
    }
}
#[derive(Default)]
pub struct Terminals {
    stopping: AtomicBool,
    workers: Arc<Workers>,
    close_saves: Arc<Workers>,
    records: Arc<persistence::Records>,
    metadata_worker: Mutex<Option<std::thread::JoinHandle<Result<(), String>>>>,
    sessions: Mutex<BTreeMap<String, Slot>>,
    // Access only with the sessions mutex acquired first. Closing a native
    // window may call back into application code. Keep its slot until success.
    closing: Mutex<BTreeSet<String>>,
}
struct Start<'a> {
    _worker: Worker,
    sender: mpsc::Sender<Event>,
    cancelled: Arc<AtomicBool>,
    events: Option<mpsc::Receiver<Event>>,
    state: &'a Terminals,
    id: String,
    token: u64,
}
impl<'a> Start<'a> {
    fn reserve(state: &'a Terminals, id: &str) -> Result<Self, String> {
        let mut sessions = state.sessions.lock().map_err(err)?;
        if state.stopping.load(Ordering::Acquire) {
            return Err("앱이 종료 중입니다".into());
        }
        if sessions.contains_key(id) || state.closing.lock().map_err(err)?.contains(id) {
            return Err("이미 시작 중이거나 시작된 세션입니다. 창을 숨겼다가 복원하세요".into());
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let token = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, events) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        sessions.insert(
            id.into(),
            Slot::Starting {
                token,
                control: Control {
                    sender: sender.clone(),
                    cancelled: cancelled.clone(),
                },
            },
        );

        Ok(Self {
            _worker: state.workers.enter(),
            sender,
            events: Some(events),
            cancelled,
            state,
            id: id.into(),
            token,
        })
    }
    fn finish(&self, session: Session) -> Result<Arc<Mutex<Session>>, String> {
        let mut sessions = self.state.sessions.lock().map_err(err)?;
        if self.state.stopping.load(Ordering::Acquire)
            || self.state.closing.lock().map_err(err)?.contains(&self.id)
            || !matches!(sessions.get(&self.id), Some(Slot::Starting {token, ..}) if *token == self.token)
        {
            return Err("터미널 시작이 취소되었습니다".into());
        }
        let Some(Slot::Starting { control, .. }) = sessions.remove(&self.id) else {
            unreachable!()
        };
        let session = Arc::new(Mutex::new(session));
        sessions.insert(
            self.id.clone(),
            Slot::Running {
                token: self.token,
                session: session.clone(),
                control,
            },
        );
        Ok(session)
    }
}
impl Drop for Start<'_> {
    fn drop(&mut self) {
        let mut sessions = self.state.sessions.lock().unwrap();
        if matches!(sessions.get(&self.id), Some(Slot::Starting {token, ..}) if *token == self.token)
        {
            sessions.remove(&self.id);
        }
    }
}
fn running(slot: &Slot) -> Option<Arc<Mutex<Session>>> {
    match slot {
        Slot::Starting { .. } | Slot::Exited { .. } => None,
        Slot::Running { session, .. } => Some(session.clone()),
    }
}
fn running_generation(slot: &Slot) -> Option<(u64, Arc<Mutex<Session>>)> {
    running(slot).map(|session| (slot.token(), session))
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
    // Unique sibling temp + atomic replacement. Failure leaves the previous
    // JSON in place; filenames and user-supplied metadata stay out of errors.
    (|| -> std::io::Result<()> {
        let dir = metadata(root);
        std::fs::create_dir_all(&dir)?;
        let mut temp = tempfile::NamedTempFile::new_in(&dir)?;
        temp.write_all(&serde_json::to_vec(agent)?)?;
        temp.persist(dir.join(format!("{}.json", agent.id)))?;
        Ok(())
    })()
    .map_err(|_| "터미널 메타데이터를 저장할 수 없습니다".into())
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
    let root = root(app);
    let restored = read_records(&root)?;
    let state = app.state::<Terminals>();
    *state.metadata_worker.lock().map_err(err)? = Some(state.records.start(root, restored)?);
    Ok(())
}
pub fn entries(app: &tauri::AppHandle) -> Vec<Terminal> {
    app.state::<Terminals>().records.all()
}
pub fn contains(app: &tauri::AppHandle, id: &str) -> bool {
    app.state::<Terminals>().records.contains(id)
}
pub fn open(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    open_presented(app, id, true)
}
pub(crate) fn open_presented(
    app: &tauri::AppHandle,
    id: &str,
    present: bool,
) -> Result<(), String> {
    let _root = crate::telemetry::Timing::new("terminal.window.open");
    let agent = app.state::<Terminals>().records.get(id)?;
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
            .transparent(true)
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
    if !present {
        return Ok(());
    }
    let _show = crate::telemetry::Timing::new("terminal.window.show");
    if crate::opacity::configured(app).is_some() {
        crate::sync_opacity(win.clone())?;
    }
    // User explicitly opens a terminal, but background CLI/tray restore preserves focus.
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
    Ok(())
}
pub fn inspect(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    welcome::refresh_path(app, id)?;
    let agent = app.state::<Terminals>().records.get(id)?;
    if !contains(app, id) {
        return Err("터미널 창이 없습니다".into());
    }
    let win = app.get_webview_window(id);
    let state = app.state::<Terminals>();
    let sessions = state.sessions.lock().map_err(err)?;
    let slot = sessions.get(id);
    let exit_status = match slot {
        Some(Slot::Exited { status, .. }) => Some(status.clone()),
        _ => None,
    };
    let session = slot.and_then(running);
    drop(sessions);
    let (pid, program, size, running) = if let Some(session) = session {
        let mut s = session.lock().map_err(err)?;
        let running = observe_exit(&mut s)?;
        (
            s.child.process_id(),
            running.then(|| welcome::foreground_name(&s).unwrap_or_else(|| s.program.clone())),
            Some(s.master.get_size().map_err(err)?),
            running,
        )
    } else {
        (None, None, None, false)
    };
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":agent.kind,"color":crate::memo_color::read(&root(app), id)?,"title":agent.display_title(),"cwd":agent.last_path.clone(),"last_path":agent.last_path,"restore_plan":agent.restore_plan,"auto_restore":agent.auto_restore,"restore_supported":cfg!(target_os = "macos"),"bounds":agent.bounds,"metadata_save_failed":app.state::<Terminals>().records.save_failed(id),"pid":pid,"program":program,"running":running,"exit_code":exit_status.as_ref().map(|s|s.exit_code()),"exit_signal":exit_status.as_ref().and_then(|s|s.signal()),"restored":crate::session_restore::terminal_restored(app,id),"last_command":crate::session_restore::last_command(app,id),"pty_size":size.map(|s|json!({"cols":s.cols,"rows":s.rows})),"visible":win.as_ref().is_some_and(|w|w.is_visible().unwrap_or(false)),"position":win.as_ref().and_then(|w|w.outer_position().ok()),"size":win.as_ref().and_then(|w|w.inner_size().ok())}),
    )
}
pub fn handle(app: &tauri::AppHandle, request: &crate::Request) -> Result<Value, String> {
    let id = request.id();
    match request.action {
        crate::Action::Open => open(app, id)?,
        crate::Action::Close => {
            if let Some(win) = app.get_webview_window(id) {
                crate::stash::release(&win)?;
                crate::relation_prototype::changed(win.app_handle(), win.label());
                win.hide().map_err(err)?;
                crate::tray::sync_visibility(app)?;
            }
        }
        crate::Action::Update
            if request.color.is_some()
                && request.title.is_none()
                && request.markdown.is_none()
                && request.status.is_none() =>
        {
            crate::memo_color::write(&root(app), id, request.color.as_deref().unwrap())?;
            app.emit_to(id, "memo-color", crate::memo_color::read(&root(app), id)?)
                .map_err(err)?;
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
    plan: Option<String>,
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
            let mut reservation = Start::reserve(&state, &id)?;
            let agent = state.records.get(&id)?;
            let token = reservation.token;
            let watched_app = app.clone();
            let watched_id = id.clone();
            let end_output = output.clone();
            let events = reservation.events.take().unwrap();
            let supervisor_worker = state.workers.enter();
            let (session, mut reader) = welcome::start(
                &agent,
                choice.as_deref().unwrap_or("shell"),
                plan.as_deref(),
                dimensions,
                move |child| {
                    std::thread::spawn(move || {
                        let _worker = supervisor_worker;
                        lifecycle::supervise(child, events, |status, jobs| {
                            if welcome::retire(&watched_app, &watched_id, token, status, jobs) {
                                let _ =
                                    end_output.send("\r\n[PTY 세션 종료]\r\n".as_bytes().to_vec());
                            }
                        });
                    });
                },
            )?;
            let initial = if choice.as_deref() == Some("auto-monitor") {
                agent.restore_plan.as_deref()
            } else {
                plan.as_deref()
                    .or(choice.as_deref())
                    .filter(|s| *s != "shell")
            };
            welcome::start_metadata(&app, &id, token, initial)?;
            let watched_session = Arc::downgrade(&reservation.finish(session)?);
            crate::session_restore::terminal_started(&app, &id);
            let watched_app = app.clone();
            let watched_id = id.clone();
            let output_parent = crate::local_trace::current();
            // Raw bytes preserve UTF-8 across arbitrary PTY read boundaries.
            let reader_worker = state.workers.enter();
            let cancelled = reservation.cancelled.clone();
            let reader_thread = std::thread::spawn(move || {
                let _worker = reader_worker;
                let mut directory_report = welcome::DirectoryReport::default();
                reader.drain(&cancelled, |bytes| {
                    let _scope = crate::local_trace::Scope::enter(output_parent);
                    let _timing = crate::telemetry::Timing::new("terminal.output");
                    let paths = {
                        let _filter = crate::telemetry::Timing::new("terminal.output.filter");
                        directory_report.feed(bytes)
                    };
                    let Some(session) = watched_session.upgrade() else {
                        return;
                    };
                    let mut s = session.lock().unwrap();
                    if directory_report.take_prompt(&s.prompt.token) {
                        s.prompt.report();
                    }
                    let plan = directory_report.take_plan(&s.prompt.token);
                    s.observed_command |= plan.is_some();
                    drop(s);
                    if plan.is_some() || !paths.is_empty() {
                        welcome::output_metadata(
                            &watched_app,
                            &watched_id,
                            token,
                            plan.as_deref(),
                            paths,
                        );
                    }
                    if cancelled.load(Ordering::Acquire) {
                        return;
                    }
                    // Keep draining after the webview closes: a full PTY can block shell exit.
                    let _dispatch = crate::telemetry::Timing::new("terminal.output.dispatch");
                    let _ = output.send(bytes.to_vec());
                });
            });
            reservation
                .sender
                .send(Event::Reader(reader_thread))
                .map_err(err)?;
            inspect(&app, &id)
        })();
        worker_call.finish(result.is_ok());
        result
    })
    .await
    .map_err(err)?
}
fn session(window: &WebviewWindow) -> Result<(u64, Arc<Mutex<Session>>), String> {
    let id = caller(window)?;
    window
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(&id)
        .and_then(running_generation)
        .ok_or("실행 중인 세션이 없습니다".into())
}
#[tauri::command]
pub fn terminal_write(window: WebviewWindow, data: String) -> Result<(), String> {
    let _timing = crate::telemetry::Timing::new("terminal.write");
    if data.len() > 65536 {
        return Err("입력은 64KiB 이하여야 합니다".into());
    }
    let (token, session) = session(&window)?;
    let mut session = {
        let _lock = crate::telemetry::Timing::new("terminal.session.lock");
        session.lock().map_err(err)?
    };
    if session.child.try_wait().map_err(err)?.is_some() {
        return Err("종료된 세션입니다".into());
    }
    crate::session_restore::command_input(window.app_handle(), &caller(&window)?, &data);
    session.prompt.input(data.as_bytes());
    let write = crate::telemetry::Timing::new("terminal.input.write");
    session.writer.write_all(data.as_bytes()).map_err(err)?;
    session.writer.flush().map_err(err)?;
    drop(write);
    drop(session);
    welcome::touch_path(window.app_handle(), &caller(&window)?, token)
}
#[tauri::command]
pub fn terminal_resize(window: WebviewWindow, cols: u16, rows: u16) -> Result<(), String> {
    let _timing = crate::telemetry::Timing::new("terminal.resize");
    let dimensions = size(cols, rows)?;
    let id = caller(&window)?;
    let state = window.state::<Terminals>();
    let sessions = state.sessions.lock().map_err(err)?;
    if matches!(sessions.get(&id), Some(Slot::Exited { .. })) {
        return Ok(());
    }
    let session = sessions
        .get(&id)
        .and_then(running)
        .ok_or("실행 중인 세션이 없습니다")?;
    drop(sessions);
    let session = session.lock().map_err(err)?;
    session.master.resize(dimensions).map_err(err)
}
pub fn terminal_hide(window: WebviewWindow) -> Result<(), String> {
    caller(&window)?;
    crate::stash::release(&window)?;
    crate::relation_prototype::changed(window.app_handle(), window.label());
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
    if let (Ok(p), Ok(s), Ok(scale)) = (
        window.outer_position(),
        window.inner_size(),
        window.scale_factor(),
    ) {
        let bounds = crate::capture::ImageNote {
            x: f64::from(p.x) / if cfg!(windows) { 1.0 } else { scale },
            y: f64::from(p.y) / if cfg!(windows) { 1.0 } else { scale },
            width: f64::from(s.width) / scale,
            height: f64::from(s.height) / scale,
        };
        app.state::<Terminals>()
            .records
            .geometry(window.label(), bounds, cfg!(windows));
    }
}
pub fn update_blocked(app: &tauri::AppHandle) -> Result<bool, String> {
    app.state::<Terminals>().update_blocked()
}

impl Terminals {
    fn update_blocked(&self) -> Result<bool, String> {
        let sessions = self.sessions.lock().map_err(err)?;
        // A removed slot can still have a draining pump or a child reaper.
        // reserve and window changes take the registry lock first as well.
        Ok(!sessions.is_empty()
            || !self.closing.lock().map_err(err)?.is_empty()
            || self.workers.active())
    }

    fn mark_exited(
        &self,
        id: &str,
        token: u64,
        status: portable_pty::ExitStatus,
        jobs: OwnedJobs,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        if !matches!(sessions.get(id), Some(Slot::Running {token: current, ..}) if *current == token)
        {
            drop(sessions);
            jobs.stop();
            return false;
        }
        let old = sessions.insert(
            id.to_owned(),
            Slot::Exited {
                token,
                status,
                jobs,
            },
        );
        drop(sessions);
        drop(old);
        true
    }
}

const QUIT_BUDGET: Duration = Duration::from_millis(750);

impl Terminals {
    fn final_snapshot(&self, root: &Path, id: &str, session: Option<&mut Session>) {
        let owner = welcome::RecordOwner::Shutdown;
        let result = match session {
            Some(s) => welcome::refresh_session_path(self, root, id, owner, s),
            None => self.edit_owned(
                root,
                id,
                owner,
                EditSource::Observation,
                welcome::reset_auto_restore,
            ),
        };
        if result.is_err() {
            eprintln!("터미널 마지막 상태를 저장할 수 없습니다.");
        }
    }

    fn finish_metadata(&self, drained: mpsc::Receiver<()>) {
        // Final snapshots can outlive the PTY budget when storage is slow.
        // Wait for their producers before sealing and flushing geometry.
        let _ = drained.recv();
        // After stop_all takes the registry, no close can claim another slot.
        // A close that won that race has already registered its pending save.
        self.close_saves.wait();
        self.records.seal();
        let worker = self.metadata_worker.lock().unwrap().take();
        if let Some(worker) = worker {
            if !matches!(worker.join(), Ok(Ok(()))) {
                eprintln!("터미널 기록의 마지막 저장에 실패했습니다.");
            }
        }
    }
    fn stop_all(&self, refresh: impl Fn(&str, Option<&mut Session>) + Send + Sync + 'static) {
        let slots = std::mem::take(&mut *self.sessions.lock().unwrap());
        let refresh = Arc::new(refresh);
        let until = Instant::now() + Duration::from_millis(75);
        for (id, slot) in slots {
            let (session, done) = match slot {
                Slot::Running {
                    session, control, ..
                } => {
                    let (done, saved) = mpsc::channel();
                    let _ = control.sender.send(Event::Quit { saved, until });
                    (Some(session), Some(done))
                }
                Slot::Exited { jobs, .. } => {
                    jobs.stop();
                    (None, None)
                }
                Slot::Starting { .. } => (None, None),
            };
            let refresh = refresh.clone();
            let worker = self.workers.enter();
            std::thread::spawn(move || {
                let _worker = worker;
                let mut guard = session.as_ref().and_then(|s| s.try_lock().ok());
                refresh(&id, guard.as_deref_mut());
                drop(done);
            });
        }
    }
}

fn bounded_shutdown(cleanup: impl FnOnce(Instant) -> bool + Send + 'static) -> bool {
    let deadline = Instant::now() + QUIT_BUDGET;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(cleanup(deadline));
    });
    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .unwrap_or(false)
}

pub fn shutdown(app: &tauri::AppHandle) {
    app.state::<Terminals>()
        .stopping
        .store(true, Ordering::Release);
    let state = app.state::<Terminals>();
    let handle = app.clone();
    let (producers, drained) = mpsc::channel::<()>();
    let complete = bounded_shutdown(move |deadline| {
        let state = handle.state::<Terminals>();
        let app = handle.clone();
        state.stop_all(move |id, s| {
            let _producer = &producers;
            app.state::<Terminals>().final_snapshot(&root(&app), id, s);
        });
        state.workers.wait_until(deadline)
    });
    if !complete {
        eprintln!("터미널 종료 대기 시간 초과");
    }
    state.finish_metadata(drained);
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
            restore_plan: None,
            auto_restore: false,
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
}
