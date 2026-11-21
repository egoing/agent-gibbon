//! Explicit local opt-in; only future PTY creations observe a change.
use super::*;
#[cfg(windows)]
const PREFIX: &str = ".pty-owner-opt-in";

#[cfg(any(windows, test))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preference {
    version: u8,
    enabled: bool,
}

#[cfg(any(windows, test))]
fn read(path: &Path) -> Result<bool, String> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("터미널 분리 설정을 읽을 수 없습니다".into()),
    };
    let mut bytes = Vec::new();
    file.take(1025).read_to_end(&mut bytes).map_err(err)?;
    if bytes.len() > 1024 {
        return Err("터미널 분리 설정 크기가 유효하지 않습니다".into());
    }
    let preference: Preference =
        serde_json::from_slice(&bytes).map_err(|_| "터미널 분리 설정 형식이 유효하지 않습니다")?;
    if preference.version != 1 {
        return Err("터미널 분리 설정 버전을 지원하지 않습니다".into());
    }
    Ok(preference.enabled)
}
#[cfg(windows)]
fn path(root: &Path) -> Result<PathBuf, String> {
    let suffix = super::windows_owner::installation_namespace().map_err(err)?;
    Ok(root.join(match suffix {
        Some(id) => format!("{PREFIX}-{id}.json"),
        None => format!("{PREFIX}.json"),
    }))
}
#[cfg(windows)]
pub(super) fn initialize(root: &Path) {
    // Legacy permanent creation opt-in is retired. Never apply it on startup.
    if let Ok(target) = path(root) {
        if read(&target) == Ok(true) {
            let result = (|| -> Result<(), String> {
                let mut file = tempfile::NamedTempFile::new_in(root).map_err(err)?;
                serde_json::to_writer(
                    &mut file,
                    &Preference {
                        version: 1,
                        enabled: false,
                    },
                )
                .map_err(err)?;
                file.as_file().sync_all().map_err(err)?;
                file.persist(target).map_err(err)?;
                Ok(())
            })();
            if result.is_err() {
                eprintln!("이전 터미널 분리 설정을 정리하지 못했지만 신규 생성은 기본 OFF입니다");
            }
        }
    }
}
fn response(supported: bool, enabled: bool, reconnect_sessions: usize) -> Value {
    json!({"supported":supported,"enabled":supported && enabled,"persisted":false,
        "reconnect_persisted":supported && reconnect_sessions > 0,"reconnect_sessions":reconnect_sessions,
        "backend":if supported {Some("windows-conpty-owner")} else {None},"applies_to":"new_sessions",
        "reason":if supported {"신규 터미널 분리는 이번 앱 실행에서만 적용됩니다. 재시작 후 기본 OFF이며, 기존 셸은 별도의 인증된 세션 체크포인트로 재접속합니다"} else {"현재 운영체제 빌드에는 메뉴에서 활성화할 분리 소유자 연동이 없습니다"}})
}
pub(crate) fn status() -> Value {
    #[cfg(windows)]
    {
        response(
            true,
            super::windows_owner::enabled(),
            super::windows_owner::reconnect_count(),
        )
    }
    #[cfg(not(windows))]
    {
        response(false, false, 0)
    }
}
pub(crate) fn enable(app: &tauri::AppHandle) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = app;
        super::windows_owner::prepare_owner_executable().map_err(err)?;
        super::windows_owner::set_runtime_enabled(true);
        Ok(status())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(status())
    }
}

fn manager_label(label: &str) -> Result<(), String> {
    if label != crate::manager::LABEL {
        return Err("관리 창에서만 터미널 분리 설정을 변경할 수 있습니다".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn terminal_persistence_status(window: WebviewWindow) -> Result<Value, String> {
    manager_label(window.label())?;
    tauri::async_runtime::spawn_blocking(status)
        .await
        .map_err(err)
}
#[tauri::command]
pub async fn terminal_persistence_enable(window: WebviewWindow) -> Result<Value, String> {
    manager_label(window.label())?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || enable(&app))
        .await
        .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_invalid_preferences_never_opt_in() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mode.json");
        assert!(!read(&p).unwrap());
        for bytes in [
            b"bad".to_vec(),
            br#"{"version":2,"enabled":true}"#.to_vec(),
            vec![b'x'; 1025],
        ] {
            std::fs::write(&p, bytes).unwrap();
            assert!(read(&p).is_err());
        }
        std::fs::write(&p, br#"{"version":1,"enabled":true}"#).unwrap();
        assert!(read(&p).unwrap());
        std::fs::write(&p, br#"{"version":1,"enabled":false}"#).unwrap();
        assert!(!read(&p).unwrap());
    }
    #[test]
    fn creation_activation_is_never_persisted_and_reconnect_is_separate() {
        for enabled in [true, false] {
            let state = response(true, enabled, 1);
            assert_eq!(state["enabled"], enabled);
            assert_eq!(state["persisted"], false);
            assert_eq!(state["reconnect_persisted"], true);
            assert_eq!(state["reconnect_sessions"], 1);
        }
    }
    #[test]
    fn unsupported_backends_do_not_claim_activation() {
        let state = response(false, true, 0);
        assert_eq!(state["enabled"], false);
        assert_eq!(state["persisted"], false);
        assert_eq!(state["backend"], Value::Null);
    }
    #[test]
    fn only_manager_can_enable_the_future_session_policy() {
        assert!(manager_label("manager").is_ok());
        for label in ["main", "note-a", "terminal-1", "webview-a"] {
            assert!(manager_label(label).is_err());
        }
    }
}
