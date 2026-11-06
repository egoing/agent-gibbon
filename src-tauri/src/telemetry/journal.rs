//! Bounded, content-free freeze evidence survives restart; all disk I/O is off the UI thread.
use super::*;
use std::{
    collections::VecDeque,
    fs,
    io::Write,
    sync::{atomic::AtomicU64, mpsc, OnceLock},
};
static SENDER: OnceLock<mpsc::SyncSender<(u64, Value)>> = OnceLock::new();
static WRITES: AtomicU64 = AtomicU64::new(0);
static FAILURES: AtomicU64 = AtomicU64::new(0);
const LIMIT: usize = 64;
pub fn start(app: &tauri::AppHandle) {
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
        .join("recent.json");
    let (sender, receiver) = mpsc::sync_channel::<(u64, Value)>(32);
    if SENDER.set(sender).is_err() {
        return;
    }
    std::thread::spawn(move || {
        let mut history: VecDeque<Value> = fs::read(&path)
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default();
        // Never preserve unknown or manually injected history fields.
        history.retain(valid);
        while let Ok((epoch, value)) = receiver.recv() {
            if !detailed() || super::transport::epoch() != Some(epoch) {
                continue;
            }
            history.push_back(value);
            while history.len() > LIMIT {
                history.pop_front();
            }
            if write(&path, &history).is_ok() {
                WRITES.fetch_add(1, Ordering::Relaxed);
            } else {
                FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
}
fn valid(v: &Value) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    o.len() == 5
        && v["message"].as_str().is_some_and(|s| MESSAGES.contains(&s))
        && v["kind"]
            .as_str()
            .is_some_and(|s| matches!(s, "native" | "memo" | "terminal" | "agent" | "manager"))
        && v["duration_ms"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && (0.0..=600_000.).contains(&n))
        && v["timestamp"].as_u64().is_some()
        && v["build"].as_str().is_some_and(|s| {
            s == "unknown" || (s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        })
}
fn write(path: &std::path::Path, history: &VecDeque<Value>) -> std::io::Result<()> {
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut temp, history)?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub fn record(message: &str, kind: &str, elapsed_ms: f64) {
    let v = json!({"message":message,"kind":kind,"duration_ms":elapsed_ms.min(600_000.),"timestamp":SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),"build":env!("AGENT_GIBBON_BUILD_REVISION")});
    if valid(&v) && detailed() {
        if let (Some(s), Some(epoch)) = (SENDER.get(), super::transport::epoch()) {
            if s.try_send((epoch, v)).is_err() {
                FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
pub fn status() -> Value {
    json!({"enabled":detailed() && SENDER.get().is_some(),"capacity":LIMIT,"writes":WRITES.load(Ordering::Relaxed),"failures":FAILURES.load(Ordering::Relaxed)})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fixed_bounded_history_is_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("diagnostics/recent.json");
        let good = json!({"message":"WebView stalled","kind":"memo","duration_ms":13000.,"timestamp":123,"build":"abcdef123456"});
        assert!(valid(&good));
        let mut bad = good.clone();
        bad["note"] = json!("PRIVATE");
        assert!(!valid(&bad));
        let h = VecDeque::from([good]);
        write(&path, &h).unwrap();
        let read: VecDeque<Value> = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(read, h);
    }
}
