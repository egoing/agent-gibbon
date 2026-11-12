//! Durable window geometry for an application update restart.
//!
//! This deliberately refuses to snapshot while terminal windows, a stash
//! transaction, or an unfinished layout transition exists. Terminal metadata
//! left from an earlier run does not block updates when no terminal window is
//! currently open.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::PathBuf,
    sync::Mutex,
};
use tauri::{AppHandle, Manager, WebviewWindow};

const FILE_NAME: &str = ".update-windows.json";
const MAX_WINDOWS: usize = 1000;
const MAX_DIMENSION: u32 = 100_000;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct WindowState {
    label: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    visible: bool,
    minimized: bool,
    always_on_top: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Snapshot {
    version: u32,
    windows: Vec<WindowState>,
}

fn path(app: &AppHandle) -> PathBuf {
    app.state::<crate::storage::Store>().0.join(FILE_NAME)
}

fn permitted_label(label: &str) -> bool {
    label == "main"
        || label.strip_prefix("note-").is_some_and(|id| {
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
        || label.strip_prefix("terminal-").is_some_and(|id| {
            !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_digit())
        })
}

fn check_live_window_kinds<'a>(labels: impl Iterator<Item = &'a str>) -> Result<(), String> {
    for label in labels {
        if label.starts_with("terminal-") {
            return Err(
                "터미널 시작 화면이 남아 있습니다. 관리 창에서 터미널을 삭제한 뒤 다시 설치하세요."
                    .into(),
            );
        }
        if label.starts_with("webview-") || label.starts_with("webpage-") {
            return Err("웹 창 세션이 있어 업데이트를 보류했습니다. 앱을 다시 시작하면 웹 창이 종료되며, 이후 업데이트를 설치할 수 있습니다.".into());
        }
    }
    Ok(())
}

fn validate(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.version != 1 {
        return Err("업데이트 창 상태의 버전을 지원하지 않습니다".into());
    }
    if snapshot.windows.len() > MAX_WINDOWS {
        return Err("업데이트 창 상태의 개수가 너무 많습니다".into());
    }
    let mut labels = BTreeSet::new();
    for window in &snapshot.windows {
        if !permitted_label(&window.label) || !labels.insert(&window.label) {
            return Err("업데이트 창 상태에 잘못되거나 중복된 창 이름이 있습니다".into());
        }
        if window.width == 0
            || window.height == 0
            || window.width > MAX_DIMENSION
            || window.height > MAX_DIMENSION
        {
            return Err("업데이트 창 상태의 크기가 유효하지 않습니다".into());
        }
    }
    Ok(())
}

fn state(window: &WebviewWindow) -> Result<WindowState, String> {
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    Ok(WindowState {
        label: window.label().to_owned(),
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        visible: window.is_visible().map_err(|e| e.to_string())?,
        minimized: window.is_minimized().map_err(|e| e.to_string())?,
        always_on_top: window.is_always_on_top().map_err(|e| e.to_string())?,
    })
}

/// Non-mutating gate shared by the readiness notice and the final save.
pub fn check(app: &AppHandle) -> Result<(), String> {
    if path(app).exists() {
        return Err("이전 업데이트의 창 복원을 먼저 완료해야 합니다".into());
    }
    let windows = crate::web::managed_windows(app);
    check_live_window_kinds(windows.keys().map(String::as_str))?;
    if crate::stash::active(app) || crate::stash::chord_busy(app) {
        return Err("메모 숨김·복원 동작이 끝난 뒤 다시 설치하세요.".into());
    }
    if crate::capture::update_blocked(app) {
        return Err("화면 캡처나 영역 선택이 끝난 뒤 다시 설치하세요.".into());
    }
    let mut count = 0;
    for window in windows.values() {
        if !permitted_label(window.label()) {
            continue;
        }
        count += 1;
        if crate::sizing::folded(window) || crate::sizing::pending(window) {
            return Err("접힌 메모를 펼치고 진행 중인 크기 조정을 마친 뒤 다시 설치하세요.".into());
        }
    }
    if count > MAX_WINDOWS {
        return Err("열린 창이 너무 많아 업데이트를 보류했습니다.".into());
    }
    Ok(())
}

/// Persist note contents and their current physical window presentation.
/// Terminal windows are conservatively blocked until their welcome state can
/// be restored without opening/focusing them.
pub fn save(app: &AppHandle) -> Result<(), String> {
    check(app)?;
    let windows = crate::web::managed_windows(app);

    let mut snapshot_windows = Vec::new();
    for window in windows.values() {
        let label = window.label();
        if !permitted_label(label) {
            continue;
        }
        snapshot_windows.push(state(window)?);
    }

    // Persist every live note first, including the startup `main` note, so a
    // restored window never points at stale content after the process exits.
    let notes = app
        .state::<Mutex<BTreeMap<String, crate::model::Note>>>()
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let store = app.state::<crate::storage::Store>();
    for (id, note) in &notes {
        store.save(id, note)?;
    }

    let snapshot = Snapshot {
        version: 1,
        windows: snapshot_windows,
    };
    validate(&snapshot)?;
    let path = path(app);
    let parent = path.parent().ok_or("업데이트 상태 폴더가 없습니다")?;
    fs::create_dir_all(parent).map_err(|e| format!("업데이트 창 상태 저장 실패: {e}"))?;
    let mut temp = tempfile::Builder::new()
        .prefix(".update-windows-")
        .tempfile_in(parent)
        .map_err(|e| format!("업데이트 창 상태 임시 파일 생성 실패: {e}"))?;
    serde_json::to_writer(&mut temp, &snapshot).map_err(|e| e.to_string())?;
    temp.write_all(b"\n").map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path)
        .map_err(|e| format!("업데이트 창 상태 저장 실패: {}", e.error))?;
    Ok(())
}

fn apply(window: &WebviewWindow, saved: &WindowState) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Let Windows switch to the saved monitor's DPI before sizing.
        window
            .set_position(tauri::PhysicalPosition::new(saved.x, saved.y))
            .map_err(|e| e.to_string())?;
        crate::windows_window::restore_client_size(
            window,
            tauri::PhysicalSize::new(saved.width, saved.height),
        )?;
    }
    // AppKit resizing can shift the top-left origin. Restore size before position.
    #[cfg(not(windows))]
    window
        .set_size(tauri::PhysicalSize::new(saved.width, saved.height))
        .map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(saved.x, saved.y))
        .map_err(|e| e.to_string())?;
    window
        .set_always_on_top(saved.always_on_top)
        .map_err(|e| e.to_string())?;
    if saved.visible {
        #[cfg(target_os = "macos")]
        {
            let ptr = window.ns_window().map_err(|e| e.to_string())?;
            // SAFETY: this is a live Tauri-owned window and the restore hook is
            // expected to run on the main thread.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
            }
        }
        #[cfg(windows)]
        crate::windows_window::show_without_focus(window)?;
        #[cfg(not(any(target_os = "macos", windows)))]
        window.show().map_err(|e| e.to_string())?;
    } else {
        window.hide().map_err(|e| e.to_string())?;
    }
    if saved.minimized {
        window.minimize().map_err(|e| e.to_string())?;
    } else {
        window.unminimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Apply saved geometry only to windows that already exist in this app run.
/// The update-state file is consumed only after every window was restored.
pub fn restore(app: &AppHandle) -> Result<bool, String> {
    if crate::privacy::onboarding_required(app) {
        return Ok(false);
    }
    let path = path(app);
    if !path.exists() {
        return Ok(false);
    }
    let bytes = fs::read(&path).map_err(|e| format!("업데이트 창 상태 읽기 실패: {e}"))?;
    let snapshot: Snapshot =
        serde_json::from_slice(&bytes).map_err(|e| format!("업데이트 창 상태 해석 실패: {e}"))?;
    validate(&snapshot)?;
    let windows = crate::web::managed_windows(app);
    for saved in &snapshot.windows {
        let Some(window) = windows.get(&saved.label) else {
            return Err(format!(
                "복원할 창이 아직 준비되지 않았습니다: {}",
                saved.label
            ));
        };
        apply(window, saved)?;
    }
    fs::remove_file(path).map_err(|e| format!("업데이트 창 상태 정리 실패: {e}"))?;
    Ok(true)
}

/// Remove a pending snapshot after an update is cancelled or abandoned.
pub fn discard(app: &AppHandle) {
    let _ = fs::remove_file(path(app));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(label: &str) -> WindowState {
        WindowState {
            label: label.into(),
            x: -1200,
            y: 40,
            width: 720,
            height: 460,
            visible: false,
            minimized: false,
            always_on_top: true,
        }
    }

    #[test]
    fn update_rejects_session_windows_including_hidden_web_views() {
        assert!(check_live_window_kinds(["main", "note-a"].into_iter()).is_ok());
        for label in ["terminal-123", "webview-web-a", "webpage-web-b"] {
            let error = check_live_window_kinds(["main", label].into_iter()).unwrap_err();
            assert!(error.contains(if label.starts_with("terminal-") {
                "터미널"
            } else {
                "웹 창"
            }));
        }
    }

    #[test]
    fn snapshot_roundtrips_and_allows_only_application_window_labels() {
        let snapshot = Snapshot {
            version: 1,
            windows: vec![
                sample("main"),
                sample("note-project_2"),
                sample("terminal-123"),
            ],
        };
        validate(&snapshot).unwrap();
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: Snapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.windows, snapshot.windows);
        assert!(!permitted_label("capture-2-0"));
        assert!(!permitted_label("feedback-main"));
        assert!(!permitted_label("note-../outside"));
    }

    #[test]
    fn rejects_duplicate_labels_and_invalid_bounds() {
        let mut snapshot = Snapshot {
            version: 1,
            windows: vec![sample("main"), sample("main")],
        };
        assert!(validate(&snapshot).is_err());
        snapshot.windows = vec![sample("note-a")];
        snapshot.windows[0].width = 0;
        assert!(validate(&snapshot).is_err());
        snapshot.windows[0].width = MAX_DIMENSION + 1;
        assert!(validate(&snapshot).is_err());
        snapshot.windows[0].width = 100;
        snapshot.version = 2;
        assert!(validate(&snapshot).is_err());
    }
}
