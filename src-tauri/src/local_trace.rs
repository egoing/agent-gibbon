//! Opt-in local evidence, independent of Sentry/consent. No content or network API.
use serde::Serialize;
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
}
#[derive(Clone, Serialize)]
struct Entry {
    call: u64,
    operation: &'static str,
    stage: Stage,
    elapsed_us: u64,
}
#[derive(Default, Clone, Serialize)]
struct Evidence {
    active: BTreeMap<u64, Entry>,
    history: VecDeque<Entry>,
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
    fn record(&self, call: u64, operation: &'static str, stage: Stage) {
        // Never make a UI callback wait for the evidence collector.
        let Ok(mut state) = self.evidence.try_lock() else {
            self.lost.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let entry = Entry {
            call,
            operation,
            stage,
            elapsed_us: self.start.elapsed().as_micros().min(u64::MAX as u128) as u64,
        };
        if matches!(stage, Stage::Complete | Stage::Aborted) {
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
    fn begin(operation: &'static str, stage: Stage) -> Self {
        let recorder = RECORDER.get();
        let call = Self {
            recorder,
            id: recorder.map_or(0, |r| r.next.fetch_add(1, Ordering::Relaxed)),
            operation,
            complete: false,
        };
        call.stage(stage);
        call
    }
    pub fn stage(&self, stage: Stage) {
        if let Some(r) = self.recorder {
            r.record(self.id, self.operation, stage);
        }
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
            let evidence = recorder.evidence.lock().ok().map(|s| s.clone());
            if let Some(evidence) = evidence {
                let value = serde_json::json!({"format":1,"pid":std::process::id(),"started_at_ms":started_at,"build":env!("AGENT_GIBBON_BUILD_REVISION"),"lost":recorder.lost.load(Ordering::Relaxed),"evidence":evidence});
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
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(value["active"]["1"].as_object().unwrap().len(), 4);
    }
}
