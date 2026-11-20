//! Explicit local preference; startup/status checks never reconcile OS registration.
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path, sync::Mutex};
use tauri::Manager;

#[cfg(target_os = "macos")]
#[path = "autostart/macos.rs"]
mod platform;
#[cfg(windows)]
#[path = "autostart/windows.rs"]
mod platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // Shared wire protocol; each OS produces a subset.
pub enum OsStatus {
    Disabled,
    Enabled,
    RequiresApproval,
    RegisteredUnknown,
    Unavailable,
}
#[derive(Default, Serialize, Deserialize)]
struct Choice {
    // None is an existing user's unmade choice, never implied consent.
    selected: Option<bool>,
}
#[derive(Default)]
pub struct Autostart(Mutex<()>);

fn excluded(debug: bool, instance: bool, verification: bool) -> bool {
    debug || instance || verification
}
fn eligible() -> bool {
    !excluded(
        cfg!(debug_assertions),
        crate::instance::current().id.is_some(),
        cfg!(feature = "verification-updater"),
    )
}
fn path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autostart")
        .join(
            crate::instance::current()
                .id
                .as_deref()
                .unwrap_or("installed"),
        )
        .join("choice.json"))
}
fn read(path: &Path) -> Result<Choice, String> {
    match fs::read(path) {
        Ok(bytes) if bytes.len() <= 4096 => serde_json::from_slice(&bytes)
            .map_err(|_| "자동 실행 선택을 읽지 못했습니다.".to_string()),
        Ok(_) => Err("자동 실행 선택 파일이 너무 큽니다.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Choice::default()),
        Err(e) => Err(e.to_string()),
    }
}
fn persist(path: &Path, choice: &Choice) -> Result<(), String> {
    let parent = path.parent().ok_or("자동 실행 저장 경로 오류")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut file, choice).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn change_choice(
    path: &Path,
    enabled: bool,
    change_os: impl FnOnce(bool) -> Result<(), String>,
) -> Result<(), String> {
    persist(
        path,
        &Choice {
            selected: Some(enabled),
        },
    )?;
    change_os(enabled)
}
pub fn diagnostics(app: &tauri::AppHandle) -> serde_json::Value {
    match status(app) {
        Ok(status) => serde_json::json!(status),
        Err(error) => serde_json::json!({"error":error}),
    }
}
#[derive(Serialize)]
pub struct Status {
    selected: Option<bool>,
    os_status: OsStatus,
    available: bool,
    reason: Option<String>,
}
pub fn status(app: &tauri::AppHandle) -> Result<Status, String> {
    let selected = read(&path(app)?)?.selected;
    if !eligible() {
        return Ok(Status {
            selected,
            os_status: OsStatus::Unavailable,
            available: false,
            reason: Some("개발·검증 앱은 로그인 자동 실행에 등록하지 않습니다.".into()),
        });
    }
    #[cfg(any(target_os = "macos", windows))]
    let (os_status, reason) = platform::status()?;
    #[cfg(not(any(target_os = "macos", windows)))]
    let (os_status, reason) = (
        OsStatus::Unavailable,
        Some("지원하지 않는 운영체제입니다.".into()),
    );
    Ok(Status {
        selected,
        os_status,
        available: os_status != OsStatus::Unavailable,
        reason,
    })
}
fn manager(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != crate::manager::LABEL {
        return Err("관리 창에서만 자동 실행을 설정할 수 있습니다.".into());
    }
    Ok(())
}
#[tauri::command]
pub fn autostart_status(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    let state = window.state::<Autostart>();
    let _guard = state.0.lock().map_err(|e| e.to_string())?;
    status(window.app_handle())
}
// Synchronous main-thread command: SMAppService and its NSBundle use AppKit.
#[tauri::command]
pub fn autostart_save(window: tauri::WebviewWindow, enabled: bool) -> Result<Status, String> {
    manager(&window)?;
    if !eligible() {
        return Err("개발·검증 앱은 로그인 자동 실행에 등록하지 않습니다.".into());
    }
    let state = window.state::<Autostart>();
    let _guard = state.0.lock().map_err(|e| e.to_string())?;
    // Persist the explicit intent first. OS failure may leave a mismatch, which
    // status reports independently; never retry or enable on the next launch.
    #[cfg(any(target_os = "macos", windows))]
    change_choice(&path(window.app_handle())?, enabled, platform::set)?;
    status(window.app_handle())
}
#[tauri::command]
pub fn autostart_open_settings(window: tauri::WebviewWindow) -> Result<(), String> {
    manager(&window)?;
    #[cfg(any(target_os = "macos", windows))]
    return platform::open_settings();
    #[cfg(not(any(target_os = "macos", windows)))]
    Err("지원하지 않는 운영체제입니다.".into())
}
pub fn login_launch() -> bool {
    if std::env::args().any(|arg| arg == "--login-startup") {
        return true;
    }
    #[cfg(target_os = "macos")]
    return platform::login_launch();
    #[cfg(not(target_os = "macos"))]
    false
}
#[cfg(target_os = "macos")]
pub fn current_login_event() -> bool {
    platform::current_login_event()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn development_and_verification_never_register() {
        for debug in [false, true] {
            for instance in [false, true] {
                for verification in [false, true] {
                    assert_eq!(
                        excluded(debug, instance, verification),
                        debug || instance || verification
                    );
                }
            }
        }
    }
    #[test]
    fn missing_is_unselected_and_explicit_off_survives_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("choice.json");
        assert_eq!(read(&path).unwrap().selected, None);
        persist(
            &path,
            &Choice {
                selected: Some(false),
            },
        )
        .unwrap();
        assert_eq!(read(&path).unwrap().selected, Some(false));
        fs::write(&path, b"broken").unwrap();
        assert!(read(&path).is_err());
    }
    #[test]
    fn explicit_intent_survives_os_denial_without_retry_on_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("choice.json");
        let calls = std::cell::Cell::new(0);
        assert!(change_choice(&path, true, |enabled| {
            assert!(enabled);
            calls.set(calls.get() + 1);
            Err("denied".into())
        })
        .is_err());
        for _ in 0..3 {
            assert_eq!(read(&path).unwrap().selected, Some(true));
        }
        assert_eq!(calls.get(), 1);
        change_choice(&path, false, |enabled| {
            assert!(!enabled);
            Ok(())
        })
        .unwrap();
        assert_eq!(read(&path).unwrap().selected, Some(false));
    }
    #[test]
    fn failed_preference_write_does_not_touch_os() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blocked");
        fs::create_dir(&path).unwrap();
        let called = std::cell::Cell::new(false);
        assert!(change_choice(&path, true, |_| {
            called.set(true);
            Ok(())
        })
        .is_err());
        assert!(!called.get());
    }
}
