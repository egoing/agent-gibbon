//! Pure, fail-closed installation reservation shared by native entry points.
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Safety {
    pub frozen: bool,
    pub activities: usize,
    pub epoch: u64,
    pub acknowledgements: BTreeMap<String, Option<bool>>,
}
impl Safety {
    pub fn enter(&mut self) -> Result<(), String> {
        if self.frozen {
            return Err("업데이트 설치를 준비하고 있습니다. 잠시 후 다시 시도하세요".into());
        }
        self.activities += 1;
        Ok(())
    }
    pub fn prepare(&mut self, labels: Vec<String>) -> Result<u64, String> {
        if self.frozen || self.activities != 0 {
            return Err("진행 중인 작업이 끝나면 설치합니다".into());
        }
        self.frozen = true;
        self.epoch += 1;
        self.acknowledgements = labels.into_iter().map(|s| (s, None)).collect();
        Ok(self.epoch)
    }
    pub fn acknowledge(&mut self, label: &str, epoch: u64, safe: bool) {
        if self.frozen && epoch == self.epoch {
            if let Some(entry) = self.acknowledgements.get_mut(label) {
                *entry = Some(safe);
            }
        }
    }
    pub fn refused(&self) -> bool {
        self.acknowledgements.values().any(|v| *v == Some(false))
    }
    pub fn ready(&self) -> bool {
        self.frozen
            && self.activities == 0
            && self.acknowledgements.values().all(|v| *v == Some(true))
    }
    pub fn release(&mut self) {
        self.frozen = false;
        self.acknowledgements.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ongoing_operation_and_duplicate_prepare_block_install() {
        let mut s = Safety::default();
        s.enter().unwrap();
        assert!(s.prepare(vec![]).is_err());
        s.activities -= 1;
        s.prepare(vec![]).unwrap();
        assert!(s.prepare(vec![]).is_err());
        assert!(s.enter().is_err());
        s.release();
        assert!(s.enter().is_ok());
    }
    #[test]
    fn every_hidden_or_visible_window_must_acknowledge_current_epoch() {
        let mut s = Safety::default();
        let epoch = s
            .prepare(vec!["note-visible".into(), "note-hidden".into()])
            .unwrap();
        s.acknowledge("note-visible", epoch, true);
        assert!(!s.ready());
        s.acknowledge("note-hidden", epoch - 1, true);
        assert!(!s.ready());
        s.acknowledge("unknown", epoch, true);
        assert!(!s.ready());
        s.acknowledge("note-hidden", epoch, false);
        assert!(!s.ready());
        s.acknowledge("note-hidden", epoch, true);
        assert!(s.ready());
        s.release();
        assert!(!s.ready());
    }
}
