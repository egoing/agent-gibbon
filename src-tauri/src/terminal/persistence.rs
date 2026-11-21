//! Terminal metadata edits commit synchronously. Serialize create, update and
//! delete without holding the records lock during IO. Only failed edits need
//! an explicit retry or a final attempt at shutdown.
use super::{err, metadata, save, Terminal};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Mutex,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditSource {
    Request,
    Observation,
}

#[derive(Default)]
struct Data {
    records: BTreeMap<String, Terminal>,
    failed: BTreeSet<String>,
    closing: bool,
}

#[derive(Default)]
pub(super) struct Records {
    #[cfg(test)]
    pub(super) before_write: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    io: Mutex<()>,
    data: Mutex<Data>,
}

impl Records {
    #[cfg(windows)]
    pub(super) fn checkpoint_ready(&self, ids: &[String]) -> bool {
        let Ok(_writer) = self.io.try_lock() else {
            return false;
        };
        let Ok(data) = self.data.try_lock() else {
            return false;
        };
        !data.closing
            && data.failed.is_empty()
            && ids.iter().all(|id| data.records.contains_key(id))
    }
    #[cfg(windows)]
    pub(super) fn checkpoint(&self, root: &Path, ids: &[String]) -> Result<(), String> {
        let _writer = self
            .io
            .try_lock()
            .map_err(|_| "터미널 기록 저장이 끝난 뒤 다시 설치하세요".to_string())?;
        let snapshots = {
            let data = self.data.lock().map_err(err)?;
            if data.closing || !data.failed.is_empty() {
                return Err("터미널 기록 저장 오류를 해결한 뒤 다시 설치하세요".into());
            }
            ids.iter()
                .map(|id| {
                    data.records
                        .get(id)
                        .cloned()
                        .ok_or_else(|| "복원할 터미널 기록이 없습니다".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        for record in snapshots {
            if let Err(error) = save(root, &record) {
                self.data.lock().map_err(err)?.failed.insert(record.id);
                return Err(error);
            }
        }
        Ok(())
    }
    pub(super) fn restore(&self, records: Vec<Terminal>) -> Result<(), String> {
        let mut data = self.data.lock().map_err(err)?;
        for record in records {
            data.records.insert(record.id.clone(), record);
        }
        Ok(())
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

    pub(super) fn insert(&self, root: &Path, record: Terminal) -> Result<(), String> {
        let _writer = self.io.lock().map_err(err)?;
        {
            let data = self.data.lock().map_err(err)?;
            if data.closing || data.records.contains_key(&record.id) {
                return Err("터미널을 생성할 수 없습니다".into());
            }
        }
        save(root, &record)?;
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
        data.failed.remove(id);
        Ok(())
    }

    // Explicit metadata edits retain synchronous success/error semantics. The
    // closure returns whether a disk write is needed; the result says whether
    // the record existed.
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
            record.clone()
        };
        #[cfg(test)]
        if let Some(hook) = self.before_write.lock().unwrap().take() {
            hook();
        }
        if let Err(error) = save(root, &snapshot) {
            self.data.lock().map_err(err)?.failed.insert(id.to_owned());
            eprintln!("터미널 메타데이터를 저장할 수 없습니다.");
            return Err(error);
        }
        self.data.lock().map_err(err)?.failed.remove(id);
        Ok(true)
    }

    pub(super) fn finish(&self, root: &Path) -> Result<(), String> {
        let _writer = self.io.lock().map_err(err)?;
        let retry = {
            let mut data = self.data.lock().map_err(err)?;
            data.closing = true;
            std::mem::take(&mut data.failed)
                .iter()
                .filter_map(|id| data.records.get(id).cloned())
                .collect::<Vec<_>>()
        };
        let mut result = Ok(());
        for record in retry {
            if let Err(error) = save(root, &record) {
                result = Err(error);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::read_records;
    use std::path::PathBuf;

    #[cfg(windows)]
    #[test]
    fn update_checkpoint_blocks_pending_or_failed_saves_and_preserves_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let store = Records::default();
        let mut r = record("terminal-1");
        r.restore_plan = Some("preserve-plan".into());
        store.restore(vec![r]).unwrap();
        let ids = vec!["terminal-1".to_string()];
        assert!(store.checkpoint_ready(&ids));
        {
            let _writer = store.io.lock().unwrap();
            assert!(!store.checkpoint_ready(&ids));
            assert!(store.checkpoint(dir.path(), &ids).is_err());
        }
        store.checkpoint(dir.path(), &ids).unwrap();
        assert_eq!(
            read_records(dir.path()).unwrap()[0].restore_plan.as_deref(),
            Some("preserve-plan")
        );
        store
            .data
            .lock()
            .unwrap()
            .failed
            .insert("terminal-1".into());
        assert!(!store.checkpoint_ready(&ids));
        assert!(store.checkpoint(dir.path(), &ids).is_err());
        assert_eq!(
            read_records(dir.path()).unwrap()[0].restore_plan.as_deref(),
            Some("preserve-plan")
        );
    }
    fn record(id: &str) -> Terminal {
        serde_json::from_value(
            serde_json::json!({"id":id,"bounds":{"x":0.,"y":0.,"width":640.,"height":400.}}),
        )
        .unwrap()
    }
    struct Fixture {
        dir: tempfile::TempDir,
        store: Records,
    }
    impl Fixture {
        fn new(ids: &[&str]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = Records::default();
            for id in ids {
                store.insert(dir.path(), record(id)).unwrap();
            }
            Self { dir, store }
        }
        fn finish(&self) -> Result<(), String> {
            self.store.finish(self.dir.path())
        }
        fn on_disk(&self, id: &str) -> Terminal {
            read_records(self.dir.path())
                .unwrap()
                .into_iter()
                .find(|r| r.id == id)
                .unwrap()
        }
    }

    #[test]
    fn unchanged_explicit_retry_persists_a_failed_edit_but_polling_does_not_retry() {
        let id = "terminal-7";
        let f = Fixture::new(&[id]);
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
        let f = Fixture::new(&[id]);
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
    fn shutdown_retries_live_failures_once_and_skips_deleted_generations() {
        let (live, deleted) = ("terminal-1", "terminal-2");
        let f = Fixture::new(&[live, deleted]);
        let root = f.dir.path();
        let directory = metadata(root);
        let held = root.join("held");
        std::fs::rename(&directory, &held).unwrap();
        std::fs::write(&directory, b"blocked").unwrap();
        for id in [live, deleted] {
            assert!(f
                .store
                .edit(root, id, EditSource::Request, |r| {
                    r.title = "old edit".into();
                    true
                })
                .is_err());
        }
        std::fs::remove_file(&directory).unwrap();
        std::fs::rename(held, directory).unwrap();
        f.store.delete(root, deleted).unwrap();
        let mut new = record(deleted);
        new.title = "replacement".into();
        f.store.insert(root, new).unwrap();
        f.finish().unwrap();
        assert_eq!(f.on_disk(live).title, "old edit");
        assert_eq!(f.on_disk(deleted).title, "replacement");
        assert!(f
            .store
            .edit(root, live, EditSource::Request, |_| true)
            .is_err());
    }
}
