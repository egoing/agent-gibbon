//! One disk writer at a time, shared by create, edit, delete and the geometry worker.
//! The IO lock comes before the data lock. Geometry only takes the latter. No
//! native window calls, PTY calls or file IO are made while holding data.
use super::{err, metadata, save, Terminal};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditSource {
    Request,
    Observation,
}

#[derive(Default)]
struct Data {
    records: BTreeMap<String, Terminal>,
    // Fixed deadlines guarantee progress even during a continuous drag.
    due: BTreeMap<String, Instant>,
    // Retry on the next change or once on orderly exit, never in a busy loop.
    failed: BTreeSet<String>,
    closing: bool,
}
#[cfg(test)]
type Pause = Mutex<Option<Box<dyn FnOnce() + Send>>>;
#[derive(Default)]
pub(super) struct Records {
    data: Mutex<Data>,
    changed: Condvar,
    io: Mutex<()>,
    #[cfg(test)]
    writes: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    pub(super) before_write: Pause,
    #[cfg(test)]
    before_claim: Pause,
}

impl Records {
    pub(super) fn start(
        self: &Arc<Self>,
        root: PathBuf,
        restored: Vec<Terminal>,
    ) -> Result<JoinHandle<Result<(), String>>, String> {
        {
            let mut data = self.data.lock().map_err(err)?;
            for record in restored {
                data.records.insert(record.id.clone(), record);
            }
        }
        let this = Arc::clone(self);
        std::thread::Builder::new()
            .name("terminal-geometry".into())
            .spawn(move || this.run(&root))
            .map_err(err)
    }

    pub(super) fn all(&self) -> Vec<Terminal> {
        self.data
            .lock()
            .unwrap()
            .records
            .values()
            .cloned()
            .collect()
    }

    pub(super) fn contains(&self, id: &str) -> bool {
        self.data.lock().unwrap().records.contains_key(id)
    }

    pub(super) fn get(&self, id: &str) -> Result<Terminal, String> {
        self.data
            .lock()
            .map_err(err)?
            .records
            .get(id)
            .cloned()
            .ok_or_else(|| "터미널 창이 없습니다".into())
    }

    pub(super) fn save_failed(&self, id: &str) -> bool {
        self.data.lock().unwrap().failed.contains(id)
    }

    fn write(&self, root: &Path, record: &Terminal) -> Result<(), String> {
        #[cfg(test)]
        if let Some(hook) = self.before_write.lock().unwrap().take() {
            hook();
        }
        save(root, record)?;
        self.data.lock().map_err(err)?.failed.remove(&record.id);
        #[cfg(test)]
        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub(super) fn insert(&self, root: &Path, record: Terminal) -> Result<(), String> {
        let _writer = self.io.lock().map_err(err)?;
        {
            let data = self.data.lock().map_err(err)?;
            if data.closing || data.records.contains_key(&record.id) {
                return Err("터미널을 생성할 수 없습니다".into());
            }
        }
        self.write(root, &record)?;
        self.data
            .lock()
            .map_err(err)?
            .records
            .insert(record.id.clone(), record);
        Ok(())
    }

    pub(super) fn delete(&self, root: &Path, id: &str) -> Result<(), String> {
        let _writer = self.io.lock().map_err(err)?;
        {
            let data = self.data.lock().map_err(err)?;
            if data.closing || !data.records.contains_key(id) {
                return Err("터미널을 삭제할 수 없습니다".into());
            }
        }
        std::fs::remove_file(metadata(root).join(format!("{id}.json")))
            .map_err(|_| "터미널 기록을 삭제할 수 없습니다".to_string())?;
        let mut data = self.data.lock().map_err(err)?;
        data.records.remove(id);
        data.due.remove(id);
        data.failed.remove(id);
        Ok(())
    }

    // Explicit metadata edits retain synchronous success/error semantics. The
    // closure returns whether a disk write is needed; the result says whether
    // the record existed. New geometry can queue while the snapshot is written.
    pub(super) fn edit(
        &self,
        root: &Path,
        id: &str,
        source: EditSource,
        edit: impl FnOnce(&mut Terminal) -> bool,
    ) -> Result<bool, String> {
        self.edit_checked(root, id, source, |record| Ok(edit(record)))
    }

    pub(super) fn edit_checked(
        &self,
        root: &Path,
        id: &str,
        source: EditSource,
        edit: impl FnOnce(&mut Terminal) -> Result<bool, String>,
    ) -> Result<bool, String> {
        let _writer = self.io.lock().map_err(err)?;
        let snapshot = {
            let mut data = self.data.lock().map_err(err)?;
            if data.closing {
                return Err("터미널 기록 저장을 종료했습니다".into());
            }
            // An unchanged explicit retry must still persist a failed edit.
            // Periodic cwd/plan observations must not restart a failing write loop.
            let retry = source == EditSource::Request && data.failed.contains(id);
            let Some(record) = data.records.get_mut(id) else {
                return Ok(false);
            };
            if !edit(record)? && !retry {
                return Ok(true);
            }
            let snapshot = record.clone();
            data.due.remove(id);
            snapshot
        };
        if let Err(error) = self.write(root, &snapshot) {
            self.failed(id);
            return Err(error);
        }
        Ok(true)
    }

    pub(super) fn geometry(&self, id: &str, bounds: crate::capture::ImageNote, physical: bool) {
        let mut data = self.data.lock().unwrap();
        if data.closing {
            return;
        }
        let Some(record) = data.records.get_mut(id) else {
            return;
        };
        record.bounds = bounds;
        record.physical_position = physical;
        if !data.due.contains_key(id) {
            data.due.insert(id.to_owned(), Instant::now() + WINDOW);
            self.changed.notify_one();
        }
    }

    fn failed(&self, id: &str) {
        // Keep a geometry event that arrived during IO. Nothing is logged from
        // the record (including its filesystem paths, title or restore plan).
        self.data.lock().unwrap().failed.insert(id.to_owned());
        eprintln!("터미널 메타데이터를 저장할 수 없습니다.");
    }

    // Only ID + deadline cross the wait, never a record snapshot. An edit or
    // delete/recreate can win the IO lock first; revalidate after acquiring it.
    fn run(&self, root: &Path) -> Result<(), String> {
        let mut flush_failed = false;
        loop {
            let id = {
                let mut data = self.data.lock().map_err(err)?;
                loop {
                    if data.closing {
                        if let Some(id) = data.due.keys().next().or_else(|| data.failed.first()) {
                            break id.clone();
                        }
                        return if flush_failed {
                            Err("터미널 기록의 마지막 저장에 실패했습니다".into())
                        } else {
                            Ok(())
                        };
                    }
                    let earliest = data.due.iter().min_by_key(|(_, at)| *at);
                    data = if let Some((id, &at)) = earliest {
                        let delay = at.saturating_duration_since(Instant::now());
                        if delay.is_zero() {
                            break id.clone();
                        }
                        self.changed.wait_timeout(data, delay).map_err(err)?.0
                    } else {
                        self.changed.wait(data).map_err(err)?
                    };
                }
            };
            #[cfg(test)]
            if let Some(hook) = self.before_claim.lock().unwrap().take() {
                hook();
            }
            let _writer = self.io.lock().map_err(err)?;
            let (snapshot, final_attempt) = {
                let mut data = self.data.lock().map_err(err)?;
                if data.closing {
                    if !data.due.contains_key(&id) && !data.failed.contains(&id) {
                        continue;
                    }
                    data.failed.remove(&id);
                } else if data.due.get(&id).is_none_or(|at| *at > Instant::now()) {
                    continue;
                }
                data.due.remove(&id);
                (data.records.get(&id).cloned(), data.closing)
            };
            if let Some(snapshot) = snapshot {
                if self.write(root, &snapshot).is_err() {
                    if final_attempt {
                        flush_failed = true;
                    } else {
                        self.failed(&id);
                    }
                }
            }
        }
    }

    pub(super) fn seal(&self) {
        // Finish any sync transaction before declaring the queue closed. The
        // caller joins with *no* locks held. The worker never calls Tauri.
        let _writer = self.io.lock().unwrap();
        self.data.lock().unwrap().closing = true;
        self.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::ImageNote, terminal::read_records};
    use std::sync::{atomic::Ordering, mpsc};

    fn bounds(i: u32) -> ImageNote {
        let i = f64::from(i);
        ImageNote {
            x: i,
            y: -i,
            width: 640. + i,
            height: 400. + i,
        }
    }
    fn record(id: &str) -> Terminal {
        serde_json::from_value(serde_json::json!({"id": id, "bounds": bounds(0)})).unwrap()
    }
    struct Fixture {
        dir: tempfile::TempDir,
        store: Arc<Records>,
        worker: Option<JoinHandle<Result<(), String>>>,
    }
    impl Fixture {
        fn new(ids: &[&str]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = Arc::new(Records::default());
            let worker = store.start(dir.path().into(), vec![]).unwrap();
            for id in ids {
                store.insert(dir.path(), record(id)).unwrap();
            }
            Self {
                dir,
                store,
                worker: Some(worker),
            }
        }
        fn finish(&mut self) -> Result<(), String> {
            self.store.seal();
            self.worker.take().unwrap().join().unwrap()
        }
        fn on_disk(&self, id: &str) -> Terminal {
            read_records(self.dir.path())
                .unwrap()
                .into_iter()
                .find(|r| r.id == id)
                .unwrap()
        }
        // Pause the actual worker after it has selected a snapshot and while
        // it owns IO, but before the real temporary file is written.
        fn pause(hook: &Pause) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
            let (start, entered) = mpsc::channel();
            let (release, wait) = mpsc::channel();
            *hook.lock().unwrap() = Some(Box::new(move || {
                start.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
            }));
            (entered, release)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if self.worker.is_some() {
                let _ = self.finish();
            }
        }
    }

    #[test]
    fn io_in_flight_does_not_block_geometry_and_edits_keep_the_latest_record() {
        let (a, b) = ("terminal-1", "terminal-2");
        let mut f = Fixture::new(&[a, b]);
        let (entered, release) = Fixture::pause(&f.store.before_write);
        f.store.geometry(a, bounds(1), false);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        // These updates (and reads) complete while the first snapshot cannot
        // possibly have reached disk. Both IDs retain independent final bounds.
        f.store.geometry(a, bounds(7), true);
        f.store.geometry(b, bounds(8), false);
        assert_eq!(f.store.get(a).unwrap().bounds.x, 7.);
        assert_eq!(f.on_disk(a).bounds.x, 0.);
        let store = Arc::clone(&f.store);
        let root = f.dir.path().to_owned();
        let edit = std::thread::spawn(move || {
            store.edit(&root, a, EditSource::Request, |r| {
                r.title = "fixture".into();
                r.last_path = Some("synthetic".into());
                r.path_updated = 13;
                r.restore_plan = Some("fixture".into());
                r.auto_restore = true;
                true
            })
        });
        release.send(()).unwrap();
        assert!(edit.join().unwrap().unwrap());
        f.store.geometry(a, bounds(9), true);
        f.finish().unwrap();
        let r = f.on_disk(a);
        assert_eq!(
            (r.bounds.x, r.bounds.y, r.bounds.width, r.bounds.height),
            (9., -9., 649., 409.)
        );
        assert!(r.physical_position && r.auto_restore);
        assert_eq!(r.title, "fixture");
        assert_eq!(r.last_path.as_deref(), Some(Path::new("synthetic")));
        assert_eq!(r.restore_plan.as_deref(), Some("fixture"));
        assert_eq!(r.path_updated, 13);
        assert_eq!(f.on_disk(b).bounds.x, 8.);
    }

    #[test]
    fn deletion_and_reuse_cannot_be_overwritten_by_an_old_in_flight_save() {
        let id = "terminal-3";
        let mut f = Fixture::new(&[id]);
        let (entered, release) = Fixture::pause(&f.store.before_write);
        f.store.geometry(id, bounds(10), true);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        f.store.geometry(id, bounds(11), true);
        let store = Arc::clone(&f.store);
        let root = f.dir.path().to_owned();
        let recreate = std::thread::spawn(move || {
            store.delete(&root, id).unwrap();
            store.geometry(id, bounds(12), true); // deleted record cannot queue a save
            let mut replacement = record(id);
            replacement.title = "new".into();
            store.insert(&root, replacement).unwrap();
        });
        release.send(()).unwrap();
        recreate.join().unwrap();
        f.finish().unwrap();
        let r = f.on_disk(id);
        assert_eq!(r.title, "new");
        assert_eq!(r.bounds.x, 0.);
        assert!(!r.physical_position);
    }

    #[test]
    fn selected_id_is_revalidated_after_a_new_edit_and_after_delete_recreate() {
        for recreate in [false, true] {
            let id = "terminal-5";
            let mut f = Fixture::new(&[id]);
            let (selected, release) = Fixture::pause(&f.store.before_claim);
            f.store.geometry(id, bounds(1), true);
            // The worker has selected this ID but does not own the IO lock.
            selected.recv_timeout(Duration::from_secs(5)).unwrap();
            if recreate {
                f.store.delete(f.dir.path(), id).unwrap();
                let mut new = record(id);
                new.title = "new generation".into();
                f.store.insert(f.dir.path(), new).unwrap();
            } else {
                f.store
                    .edit(f.dir.path(), id, EditSource::Request, |r| {
                        r.title = "new edit".into();
                        true
                    })
                    .unwrap();
            }
            let writes = f.store.writes.load(Ordering::Relaxed);
            release.send(()).unwrap();
            f.finish().unwrap();
            assert_eq!(f.store.writes.load(Ordering::Relaxed), writes);
            let r = f.on_disk(id);
            if recreate {
                assert_eq!((r.title.as_str(), r.bounds.x), ("new generation", 0.));
            } else {
                assert_eq!((r.title.as_str(), r.bounds.x), ("new edit", 1.));
            }
        }
    }

    #[test]
    fn failed_save_preserves_previous_bytes_and_flush_retries_once() {
        let id = "terminal-4";
        for recover in [true, false] {
            let mut f = Fixture::new(&[id]);
            let directory = metadata(f.dir.path());
            let held = f.dir.path().join("held");
            let filename = format!("{id}.json");
            let original = std::fs::read(directory.join(&filename)).unwrap();
            // Real failure, independent of permissions/root privilege.
            std::fs::rename(&directory, &held).unwrap();
            std::fs::write(&directory, b"blocked").unwrap();
            f.store.geometry(id, bounds(42), true);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !f.store.save_failed(id) {
                assert!(Instant::now() < deadline, "background error not observable");
                std::thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(std::fs::read(held.join(&filename)).unwrap(), original);
            assert!(f.store.data.lock().unwrap().due.is_empty());
            if recover {
                std::fs::remove_file(&directory).unwrap();
                std::fs::rename(&held, &directory).unwrap();
            }
            assert_eq!(f.finish().is_ok(), recover);
            if recover {
                assert_eq!(f.on_disk(id).bounds.x, 42.);
                assert!(!f.store.save_failed(id));
            } else {
                assert_eq!(std::fs::read(held.join(&filename)).unwrap(), original);
            }
        }
    }

    #[test]
    fn unchanged_explicit_retry_persists_a_failed_edit_but_polling_does_not_retry() {
        let id = "terminal-7";
        let mut f = Fixture::new(&[id]);
        let directory = metadata(f.dir.path());
        let held = f.dir.path().join("held");
        std::fs::rename(&directory, &held).unwrap();
        std::fs::write(&directory, b"blocked").unwrap();
        let choose = |r: &mut Terminal| {
            let path = PathBuf::from("fixture");
            if r.last_path.as_ref() == Some(&path) {
                return false;
            }
            r.last_path = Some(path);
            true
        };
        assert!(f
            .store
            .edit(f.dir.path(), id, EditSource::Request, choose)
            .is_err());
        // A second explicit request must not report success while still blocked.
        assert!(f
            .store
            .edit(f.dir.path(), id, EditSource::Request, choose)
            .is_err());
        std::fs::remove_file(&directory).unwrap();
        std::fs::rename(held, directory).unwrap();
        f.store
            .edit(f.dir.path(), id, EditSource::Observation, choose)
            .unwrap();
        assert!(f.store.save_failed(id));
        assert_eq!(f.on_disk(id).last_path, None);
        assert!(f
            .store
            .edit(f.dir.path(), id, EditSource::Request, choose)
            .unwrap());
        assert!(!f.store.save_failed(id));
        assert_eq!(
            f.on_disk(id).last_path.as_deref(),
            Some(Path::new("fixture"))
        );
        f.finish().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn serialization_failure_keeps_original_json_in_place_and_removes_tempfile() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let id = "terminal-6";
        let mut f = Fixture::new(&[id]);
        let dir = metadata(f.dir.path());
        let file = dir.join(format!("{id}.json"));
        let original = std::fs::read(&file).unwrap();
        let changed = f.store.edit(f.dir.path(), id, EditSource::Request, |r| {
            r.last_path = Some(OsString::from_vec(vec![0xff]).into());
            true
        });
        assert!(changed.is_err());
        assert!(f.store.save_failed(id));
        assert_eq!(std::fs::read(&file).unwrap(), original);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        assert!(f.finish().is_err());
        assert_eq!(std::fs::read(&file).unwrap(), original);
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), 1);
    }

    #[test]
    fn continuous_updates_keep_the_first_deadline_and_serve_the_oldest_window() {
        let root = tempfile::tempdir().unwrap();
        let records = Arc::new(Records::default());
        let cold = "terminal-2";
        let hot = "terminal-1";
        for id in [cold, hot] {
            records.insert(root.path(), record(id)).unwrap();
        }
        records.geometry(cold, bounds(1), true);
        records.geometry(hot, bounds(1), true);
        let hot_deadline = records.data.lock().unwrap().due[hot];
        for i in 2..240 {
            records.geometry(hot, bounds(i), true);
        }
        assert_eq!(records.data.lock().unwrap().due[hot], hot_deadline);

        // Both windows are eligible. The lexically later window is older.
        let past = Instant::now() - Duration::from_secs(1);
        {
            let mut data = records.data.lock().unwrap();
            data.due.insert(cold.into(), past - WINDOW);
            data.due.insert(hot.into(), past);
        }
        let (entered, release) = Fixture::pause(&records.before_write);
        let worker = records.start(root.path().into(), vec![]).unwrap();
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        {
            let data = records.data.lock().unwrap();
            assert!(!data.due.contains_key(cold));
            assert!(data.due.contains_key(hot));
        }
        release.send(()).unwrap();
        records.seal();
        worker.join().unwrap().unwrap();
        let disk = read_records(root.path()).unwrap();
        assert_eq!(disk.iter().find(|r| r.id == cold).unwrap().bounds.x, 1.);
        assert_eq!(disk.iter().find(|r| r.id == hot).unwrap().bounds.x, 239.);
    }

    #[test]
    #[ignore = "manual headless geometry benchmark; no native window or real user data"]
    fn measure_geometry_burst() {
        for ids in [
            &["terminal-1"][..],
            &["terminal-1", "terminal-2", "terminal-3", "terminal-4"][..],
        ] {
            let mut f = Fixture::new(ids);
            let before = f.store.writes.load(Ordering::Relaxed);
            let start = Instant::now();
            for i in 0..240 {
                for &id in ids {
                    f.store.geometry(id, bounds(i), cfg!(windows));
                }
            }
            let event_us = start.elapsed().as_micros();
            f.finish().unwrap();
            let writes = f.store.writes.load(Ordering::Relaxed) - before;
            for id in ids {
                let r = f.on_disk(id);
                assert_eq!(
                    (r.bounds.x, r.bounds.y, r.bounds.width, r.bounds.height),
                    (239., -239., 879., 639.)
                );
                assert_eq!(r.physical_position, cfg!(windows));
            }
            println!("geometry_after ids={} events={} event_us={} successful_writes={} final_verified=true", ids.len(), ids.len()*240, event_us, writes);
        }
    }
}
