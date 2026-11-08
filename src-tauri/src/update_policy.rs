//! Pure, fail-closed installation reservation shared by native entry points.
use std::collections::BTreeMap;
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestKind {
    Snapshot,
    ManagerNotes,
    FitNote,
    SaveNote,
    ExportNote,
    TerminalInfo,
    PollFold,
    PollWarp,
    PresentFeedback,
    FeedbackSnapshot,
    Other,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestStats {
    pub count: u32,
    pub oldest_ms: u64,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowReport {
    pub editing: bool,
    pub saving: bool,
    pub exporting: bool,
    pub privacy: bool,
    pub pending: u32,
    pub requests: BTreeMap<RequestKind, RequestStats>,
    pub oldest_ms: u64,
    pub visible: Option<bool>,
}
#[derive(Default)]
pub struct Safety {
    pub reports: BTreeMap<String, WindowReport>,
    pub frozen: bool,
    pub activities: usize,
    pub epoch: u64,
    pub acknowledgements: BTreeMap<String, Option<bool>>,
}

/// A release gets one user-visible ready notice per version. The record is
/// persisted by the updater after the non-focusing window is presented.
pub fn should_show_notice(version: &str, last_shown: Option<&str>) -> bool {
    !version.is_empty() && last_shown != Some(version)
}

pub fn idle_hold_reason(idle_seconds: Option<f64>, threshold: f64) -> Option<&'static str> {
    match idle_seconds {
        Some(seconds) if seconds.is_finite() && seconds >= threshold => None,
        Some(_) if threshold == 30.0 => Some(
            "최근 입력이 있어 자동 설치를 보류했습니다. 30초 동안 조작하지 않으면 자동 설치합니다.",
        ),
        Some(_) => Some(
            "최근 입력이 있어 자동 설치를 보류했습니다. 5분 동안 조작하지 않으면 자동 설치합니다.",
        ),
        None => Some("사용자 입력 상태를 확인할 수 없어 자동 설치를 보류했습니다."),
    }
}

pub fn idle_allows_install(automatic: bool, idle_seconds: Option<f64>, threshold: f64) -> bool {
    !automatic || idle_hold_reason(idle_seconds, threshold).is_none()
}

pub fn ready_hold_reason(
    idle_seconds: Option<f64>,
    threshold: f64,
    native_reason: Option<&str>,
) -> Option<String> {
    native_reason
        .map(str::to_owned)
        .or_else(|| idle_hold_reason(idle_seconds, threshold).map(str::to_owned))
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
        self.reports.clear();
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
    pub fn refusal_reason(&self) -> &'static str {
        let reports = self.reports.values();
        if reports.clone().any(|r| r.editing || r.saving) {
            "편집·저장 중인 창이 있어 설치를 보류했습니다"
        } else if reports.clone().any(|r| r.exporting) {
            "내보내기가 끝나면 설치할 수 있습니다"
        } else if reports.clone().any(|r| r.privacy) {
            "개인정보 설정 작업이 끝나면 설치할 수 있습니다"
        } else if reports.clone().any(|r| r.pending > 0) {
            "내부 요청이 처리 중이어서 설치를 보류했습니다"
        } else {
            "창이 설치 준비를 보류했습니다"
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

    pub fn release_epoch(&mut self, epoch: u64) -> bool {
        if !self.frozen || self.epoch != epoch {
            return false;
        }
        self.release();
        true
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

    #[test]
    fn ready_notice_is_deduplicated_by_exact_release_version() {
        assert!(should_show_notice("1.3.0", None));
        assert!(!should_show_notice("1.3.0", Some("1.3.0")));
        assert!(should_show_notice("1.4.0", Some("1.3.0")));
        assert!(!should_show_notice("", None));
    }

    #[test]
    fn idle_gate_produces_a_specific_reason_or_allows_automatic_install() {
        assert!(idle_hold_reason(Some(299.9), 300.0).is_some());
        assert!(idle_hold_reason(None, 300.0).is_some());
        assert_eq!(idle_hold_reason(Some(300.0), 300.0), None);
        assert!(idle_hold_reason(Some(f64::NAN), 300.0).is_some());
    }

    #[test]
    fn manual_install_skips_only_the_idle_gate() {
        assert!(!idle_allows_install(true, Some(40.0), 300.0));
        assert!(idle_allows_install(false, Some(40.0), 300.0));
        assert!(!idle_allows_install(true, None, 300.0));
        assert!(idle_allows_install(true, Some(300.0), 300.0));
    }

    #[test]
    fn stale_installer_cannot_release_a_newer_frozen_epoch() {
        let mut s = Safety::default();
        let first = s.prepare(vec!["note-a".into()]).unwrap();
        assert!(s.release_epoch(first));
        let second = s.prepare(vec!["note-b".into()]).unwrap();
        assert_ne!(first, second);
        assert!(!s.release_epoch(first));
        assert!(s.frozen);
        assert!(!s.ready());
        assert!(s.release_epoch(second));
    }

    #[test]
    fn concrete_native_block_reason_precedes_the_idle_reminder() {
        assert_eq!(
            ready_hold_reason(
                Some(40.0),
                300.0,
                Some("터미널 작업을 마친 뒤 다시 시도하세요.")
            ),
            Some("터미널 작업을 마친 뒤 다시 시도하세요.".into())
        );
    }
}
