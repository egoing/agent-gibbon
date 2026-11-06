//! Opt-in local evidence, independent of Sentry/consent. No content or network API.
use serde::Serialize;
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

const ACTIVE_LIMIT: usize = 64;
const HISTORY_LIMIT: usize = 256;
static RECORDER: OnceLock<Recorder> = OnceLock::new();

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    UiQueue,
    UiExecute,
    Dispatch,
    NotesWait,
    NotesCopy,
    Opacity,
    Feedback,
    Position,
    Size,
    Visibility,
    Focus,
    Pin,
    Response,
    Complete,
    Aborted,
    Expired,
    Request,
    Dom,
    Measure,
    Fit,
    FrameReady,
    Failed,
}
#[derive(Clone, Serialize)]
struct Entry {
    call: u64,
    operation: &'static str,
    parent: Option<u64>,
    cause: Option<u64>,
    stage: Stage,
    elapsed_us: u64,
    duration_us: Option<u64>,
}
#[derive(Default, Clone, Serialize)]
struct Evidence {
    active: BTreeMap<u64, Entry>,
    history: VecDeque<Entry>,
    #[serde(skip)]
    flows: BTreeMap<u64, Flow>,
}
#[derive(Clone)]
struct Flow {
    owner: String,
    started: Instant,
    events: u8,
}

thread_local! { static PARENT: Cell<Option<u64>> = const { Cell::new(None) }; }
/// Scope is deliberately !Send: never retain it across await or move it to a worker.
pub struct Scope {
    previous: Option<u64>,
    _local: PhantomData<Rc<()>>,
}
impl Scope {
    pub fn enter(parent: Option<u64>) -> Self {
        Self {
            previous: PARENT.with(|p| p.replace(parent)),
            _local: PhantomData,
        }
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        PARENT.with(|p| p.set(self.previous));
    }
}
pub fn current() -> Option<u64> {
    PARENT.with(Cell::get)
}
pub fn enabled() -> bool {
    RECORDER.get().is_some()
}

struct Recorder {
    start: Instant,
    next: AtomicU64,
    lost: AtomicU64,
    evidence: Mutex<Evidence>,
}
impl Recorder {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            next: AtomicU64::new(1),
            lost: AtomicU64::new(0),
            evidence: Mutex::new(Evidence::default()),
        }
    }
    #[cfg(test)]
    fn record(&self, call: u64, operation: &'static str, stage: Stage) {
        self.record_linked(call, operation, stage, None, None, None);
    }
    fn record_linked(
        &self,
        call: u64,
        operation: &'static str,
        stage: Stage,
        parent: Option<u64>,
        cause: Option<u64>,
        duration_us: Option<u64>,
    ) {
        // Never make a UI callback wait for the evidence collector.
        let Ok(mut state) = self.evidence.try_lock() else {
            self.lost.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let entry = Entry {
            call,
            operation,
            parent,
            cause,
            stage,
            duration_us,
            elapsed_us: self.start.elapsed().as_micros().min(u64::MAX as u128) as u64,
        };
        self.store(&mut state, entry);
    }
    fn store(&self, state: &mut Evidence, entry: Entry) {
        let call = entry.call;
        if matches!(
            entry.stage,
            Stage::Complete | Stage::Aborted | Stage::Failed | Stage::Expired
        ) {
            state.active.remove(&call);
        } else if state.active.contains_key(&call) || state.active.len() < ACTIVE_LIMIT {
            state.active.insert(call, entry.clone());
        } else {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
        if state.history.len() == HISTORY_LIMIT {
            state.history.pop_front();
        }
        state.history.push_back(entry);
    }
}

pub struct Call {
    recorder: Option<&'static Recorder>,
    id: u64,
    operation: &'static str,
    complete: bool,
    parent: Option<u64>,
    cause: Option<u64>,
}
impl Call {
    pub fn ipc() -> Self {
        Self::begin("ipc", Stage::UiQueue)
    }
    pub fn new(snapshot: bool) -> Self {
        Self::begin(
            if snapshot { "snapshot" } else { "inspect" },
            Stage::Dispatch,
        )
    }
    pub fn begin(operation: &'static str, stage: Stage) -> Self {
        let recorder = RECORDER.get();
        let call = Self {
            recorder,
            id: recorder.map_or(0, |r| r.next.fetch_add(1, Ordering::Relaxed)),
            operation,
            complete: false,
            parent: current(),
            cause: None,
        };
        call.stage(stage);
        call
    }
    pub fn stage(&self, stage: Stage) {
        if let Some(r) = self.recorder {
            r.record_linked(
                self.id,
                self.operation,
                stage,
                self.parent,
                self.cause,
                None,
            );
        }
    }
    pub fn id(&self) -> Option<u64> {
        self.recorder.map(|_| self.id)
    }
    pub fn scope(&self) -> Scope {
        Scope::enter(self.id())
    }
    pub fn cause(&mut self, cause: Option<u64>) {
        self.cause = cause;
    }
    pub fn finish(&mut self, success: bool) {
        self.stage(if success {
            Stage::Complete
        } else {
            Stage::Failed
        });
        self.complete = true;
    }
    pub fn complete(&mut self) {
        self.stage(Stage::Complete);
        self.complete = true;
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        if !self.complete {
            self.stage(Stage::Aborted);
        }
    }
}

#[tauri::command]
pub fn local_flow_status() -> bool {
    enabled()
}
#[tauri::command]
pub fn local_flow_begin(window: tauri::WebviewWindow, operation: String) -> Option<u64> {
    let operation = match operation.as_str() {
        "note.refresh" => "note.refresh",
        "note.save" => "note.save",
        "terminal.start" => "webview.terminal.start",
        _ => return None,
    };
    begin_flow(RECORDER.get()?, window.label(), operation)
}
fn begin_flow(r: &Recorder, owner: &str, operation: &'static str) -> Option<u64> {
    let id = r.next.fetch_add(1, Ordering::Relaxed);
    {
        let Ok(mut state) = r.evidence.try_lock() else {
            r.lost.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        if state.active.len() >= ACTIVE_LIMIT || state.flows.len() >= ACTIVE_LIMIT {
            r.lost.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        state.flows.insert(
            id,
            Flow {
                owner: owner.into(),
                started: Instant::now(),
                events: 0,
            },
        );
        r.store(
            &mut state,
            Entry {
                call: id,
                operation,
                parent: None,
                cause: None,
                duration_us: None,
                stage: Stage::Request,
                elapsed_us: r.start.elapsed().as_micros() as u64,
            },
        );
    }
    Some(id)
}
pub fn webview_scope(window: &tauri::WebviewWindow, flow: Option<u64>) -> Scope {
    Scope::enter(flow.filter(|id| valid_flow(window.label(), *id)))
}
fn valid_flow(owner: &str, id: u64) -> bool {
    let Some(r) = RECORDER.get() else {
        return false;
    };
    let Ok(state) = r.evidence.try_lock() else {
        r.lost.fetch_add(1, Ordering::Relaxed);
        return false;
    };
    state
        .flows
        .get(&id)
        .is_some_and(|f| f.owner == owner && f.started.elapsed() < Duration::from_secs(60))
}
#[tauri::command]
pub fn local_flow_stage(
    window: tauri::WebviewWindow,
    flow: u64,
    stage: String,
    duration_ms: Option<f64>,
) {
    let stage = match stage.as_str() {
        "dom" => Stage::Dom,
        "measure" => Stage::Measure,
        "fit" => Stage::Fit,
        "frame_ready" => Stage::FrameReady,
        "response" => Stage::Response,
        "complete" => Stage::Complete,
        "failed" => Stage::Failed,
        _ => return,
    };
    let Some(r) = RECORDER.get() else {
        return;
    };
    let duration = match duration_ms {
        Some(ms) if ms.is_finite() && (0.0..=600_000.).contains(&ms) => Some((ms * 1000.) as u64),
        Some(_) => return,
        None => None,
    };
    advance_flow(r, window.label(), flow, stage, duration);
}
fn advance_flow(r: &Recorder, owner: &str, flow: u64, stage: Stage, duration_us: Option<u64>) {
    let Ok(mut state) = r.evidence.try_lock() else {
        r.lost.fetch_add(1, Ordering::Relaxed);
        return;
    };
    let Some(f) = state.flows.get_mut(&flow) else {
        return;
    };
    if f.owner != owner || f.started.elapsed() >= Duration::from_secs(60) || f.events >= 32 {
        return;
    }
    f.events += 1;
    let Some(entry) = state.active.get(&flow).cloned() else {
        return;
    };
    if matches!(stage, Stage::Complete | Stage::Failed) {
        state.flows.remove(&flow);
    }
    r.store(
        &mut state,
        Entry {
            stage,
            duration_us,
            elapsed_us: r.start.elapsed().as_micros() as u64,
            ..entry
        },
    );
}
fn expire(r: &Recorder, state: &mut Evidence) {
    let ids: Vec<_> = state
        .flows
        .iter()
        .filter(|(_, f)| f.started.elapsed() >= Duration::from_secs(60))
        .map(|(id, _)| *id)
        .collect();
    for id in ids {
        state.flows.remove(&id);
        if let Some(mut entry) = state.active.remove(&id) {
            entry.stage = Stage::Expired;
            entry.elapsed_us = r.start.elapsed().as_micros() as u64;
            if state.history.len() == HISTORY_LIMIT {
                state.history.pop_front();
            }
            state.history.push_back(entry);
        }
    }
}

pub fn start(app: &tauri::AppHandle) {
    if std::env::var("AGENT_GIBBON_LOCAL_TRACE").as_deref() != Ok("1") {
        return;
    }
    let Ok(root) = app.path().app_data_dir() else {
        return;
    };
    let path = root
        .join("diagnostics")
        .join(
            crate::instance::current()
                .id
                .as_deref()
                .unwrap_or("installed"),
        )
        .join("local-stages.json");
    if RECORDER.set(Recorder::new()).is_err() {
        return;
    }
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::thread::spawn(move || {
        let recorder = RECORDER.get().unwrap();
        loop {
            // This worker never calls a native window API or application-state lock.
            let evidence = recorder.evidence.lock().ok().map(|mut s| {
                expire(recorder, &mut s);
                s.clone()
            });
            if let Some(evidence) = evidence {
                let value = serde_json::json!({"format":2,"pid":std::process::id(),"started_at_ms":started_at,"build":env!("AGENT_GIBBON_BUILD_REVISION"),"lost":recorder.lost.load(Ordering::Relaxed),"evidence":evidence});
                let _ = persist(&path, &value);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}
fn persist(path: &std::path::Path, value: &serde_json::Value) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    // The profiler showed per-token writes dominating this worker in debug mode.
    // Buffer serialization so recording does not issue a syscall for every JSON token.
    {
        let mut writer = std::io::BufWriter::new(&mut file);
        serde_json::to_writer(&mut writer, value)?;
        writer.flush()?;
    }
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_scopes_are_explicit_and_restore_even_after_unwind() {
        let _root = Scope::enter(Some(10));
        let worker = std::thread::spawn(|| {
            assert_eq!(current(), None);
            {
                let _parent = Scope::enter(Some(20));
                assert_eq!(current(), Some(20));
            }
            assert_eq!(current(), None);
        });
        {
            let _nested = Scope::enter(Some(11));
            assert_eq!(current(), Some(11));
        }
        worker.join().unwrap();
        assert_eq!(current(), Some(10));
        let _ = std::panic::catch_unwind(|| {
            let _nested = Scope::enter(Some(12));
            panic!("test");
        });
        assert_eq!(current(), Some(10));
    }
    #[test]
    fn flows_are_owner_scoped_bounded_expire_and_reject_late_events() {
        let r = Recorder::new();
        let a = begin_flow(&r, "window-a", "note.refresh").unwrap();
        let b = begin_flow(&r, "window-b", "note.refresh").unwrap();
        advance_flow(&r, "window-b", a, Stage::Complete, None);
        assert!(r.evidence.lock().unwrap().active.contains_key(&a));
        advance_flow(&r, "window-a", a, Stage::Failed, None);
        advance_flow(&r, "window-a", a, Stage::Dom, None);
        let mut state = r.evidence.lock().unwrap();
        assert!(!state.active.contains_key(&a));
        assert!(matches!(state.history.back().unwrap().stage, Stage::Failed));
        state.flows.get_mut(&b).unwrap().started = Instant::now() - Duration::from_secs(61);
        expire(&r, &mut state);
        assert!(state.active.is_empty());
        assert!(state.flows.is_empty());
        assert!(matches!(
            state.history.back().unwrap().stage,
            Stage::Expired
        ));
        drop(state);
        for _ in 0..ACTIVE_LIMIT {
            assert!(begin_flow(&r, "a", "note.refresh").is_some());
        }
        assert!(begin_flow(&r, "a", "note.refresh").is_none());
        let state = r.evidence.lock().unwrap();
        assert_eq!(state.active.len(), ACTIVE_LIMIT);
        assert_eq!(state.flows.len(), ACTIVE_LIMIT);
        assert!(!serde_json::to_string(&*state).unwrap().contains("owner"));
    }
    #[test]
    fn stage_flood_is_bounded_and_expiration_removes_unfinished_flow() {
        let r = Recorder::new();
        let id = begin_flow(&r, "a", "note.refresh").unwrap();
        for _ in 0..1000 {
            advance_flow(&r, "a", id, Stage::Dom, Some(1));
        }
        let mut state = r.evidence.lock().unwrap();
        assert_eq!(state.flows[&id].events, 32);
        assert_eq!(state.history.len(), 33);
        state.flows.get_mut(&id).unwrap().started = Instant::now() - Duration::from_secs(61);
        expire(&r, &mut state);
        assert!(!state.active.contains_key(&id));
        assert!(state.flows.is_empty());
    }
    #[test]
    fn linked_child_keeps_parent_and_mutation_cause_after_parent_completed() {
        let r = Recorder::new();
        r.record(10, "ipc", Stage::Complete);
        r.record_linked(20, "snapshot", Stage::NotesWait, Some(15), Some(10), None);
        let s = r.evidence.lock().unwrap();
        assert_eq!(s.active[&20].parent, Some(15));
        assert_eq!(s.active[&20].cause, Some(10));
    }
    #[test]
    fn blocked_operation_is_persistable_before_it_returns() {
        let r = std::sync::Arc::new(Recorder::new());
        let worker = r.clone();
        let (started, ready) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let task = std::thread::spawn(move || {
            worker.record(1, "snapshot", Stage::NotesWait);
            started.send(()).unwrap();
            wait.recv().unwrap();
            worker.record(1, "snapshot", Stage::Complete);
        });
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let value = serde_json::to_value(r.evidence.lock().unwrap().clone()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("local-stages.json");
        // The operation cannot finish until after the file is read.
        let result = persist(&path, &value);
        release.send(()).unwrap();
        task.join().unwrap();
        result.unwrap();
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(stored["active"]["1"]["stage"], "notes_wait");
        assert!(r.evidence.lock().unwrap().active.is_empty());
    }
    #[test]
    fn unfinished_call_survives_history_turnover_and_records_completion() {
        let r = Recorder::new();
        r.record(1, "snapshot", Stage::NotesWait);
        for id in 2..1000 {
            r.record(id, "inspect", Stage::Dispatch);
            r.record(id, "inspect", Stage::Complete);
        }
        let s = r.evidence.lock().unwrap();
        assert_eq!(s.active.len(), 1);
        assert!(matches!(s.active[&1].stage, Stage::NotesWait));
        assert_eq!(s.history.len(), HISTORY_LIMIT);
        drop(s);
        r.record(1, "snapshot", Stage::Complete);
        assert!(r.evidence.lock().unwrap().active.is_empty());
    }
    #[test]
    fn recorder_is_bounded_and_never_waits_for_its_mutex() {
        let r = Recorder::new();
        for id in 0..1000 {
            r.record(id, "snapshot", Stage::NotesCopy);
        }
        let s = r.evidence.lock().unwrap();
        assert_eq!(s.active.len(), ACTIVE_LIMIT);
        assert_eq!(s.history.len(), HISTORY_LIMIT);
        r.record(1, "snapshot", Stage::Position);
        assert!(r.lost.load(Ordering::Relaxed) > 0);
    }
    #[test]
    fn atomic_local_file_contains_only_fixed_fields_and_numeric_call_ids() {
        let r = Recorder::new();
        r.record(1, "snapshot", Stage::Position);
        let value = serde_json::to_value(r.evidence.lock().unwrap().clone()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("local-stages.json");
        persist(&path, &value).unwrap();
        persist(&path, &value).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()).unwrap(),
            value
        );
        assert_eq!(value["active"]["1"].as_object().unwrap().len(), 7);
    }
}
