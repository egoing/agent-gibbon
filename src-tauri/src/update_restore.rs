//! Durable window geometry for an application update restart.
//!
//! Automatic installs refuse live terminal/web sessions, stash transactions,
//! and unfinished layout transitions. A separately confirmed manual restart
//! may snapshot supported web/terminal windows without closing them first.
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
    #[serde(default)]
    web_url: Option<String>,
    #[serde(default)]
    terminal_last_command: Option<String>,
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
        || ["webview-", "webpage-"].iter().any(|prefix| {
            label.strip_prefix(prefix).is_some_and(|id| {
                id.starts_with("web-")
                    && id.len() <= 132
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
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
    let mut web_ids = BTreeSet::new();
    for window in &snapshot.windows {
        if !permitted_label(&window.label) || !labels.insert(&window.label) {
            return Err("업데이트 창 상태에 잘못되거나 중복된 창 이름이 있습니다".into());
        }
        if let Some(id) = window
            .label
            .strip_prefix("webview-")
            .or_else(|| window.label.strip_prefix("webpage-"))
        {
            if !web_ids.insert(id) {
                return Err("복원 데이터에 중복된 웹 세션 ID가 있습니다".into());
            }
        }
        if window.width == 0
            || window.height == 0
            || window.width > MAX_DIMENSION
            || window.height > MAX_DIMENSION
        {
            return Err("업데이트 창 상태의 크기가 유효하지 않습니다".into());
        }
        if window.label.starts_with("webpage-")
            && window
                .web_url
                .as_deref()
                .is_none_or(|url| crate::web::validate_url(url).is_err())
        {
            return Err("복원할 웹 주소가 유효하지 않습니다".into());
        }
        if window.label.starts_with("webview-") && window.web_url.is_some() {
            return Err("웹 Welcome 창에 외부 주소가 설정되어 있습니다".into());
        }
        if !window.label.starts_with("webpage-") && window.web_url.is_some() {
            return Err("웹 주소가 외부 페이지 창에 연결되지 않았습니다".into());
        }
        if window
            .terminal_last_command
            .as_deref()
            .is_some_and(|choice| !matches!(choice, "shell" | "codex" | "claude" | "git" | "htop"))
        {
            return Err("복원할 터미널 선택 항목이 유효하지 않습니다".into());
        }
        if !window.label.starts_with("terminal-") && window.terminal_last_command.is_some() {
            return Err("터미널 선택 항목이 터미널 창에 연결되지 않았습니다".into());
        }
    }
    Ok(())
}

fn state(app: &AppHandle, window: &WebviewWindow) -> Result<WindowState, String> {
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
        web_url: if window.label().starts_with("webpage-") {
            crate::web::url_for_restore(app, window.label())
        } else {
            None
        },
        terminal_last_command: if window.label().starts_with("terminal-") {
            crate::terminal::last_command(app, window.label())
        } else {
            None
        },
    })
}

/// Non-mutating gate shared by the readiness notice and the final save.
pub fn check(app: &AppHandle) -> Result<(), String> {
    check_impl(app, false)
}

/// Gate used only after the user confirms the manual install action.
#[allow(dead_code)] // Consumed by the explicit update impact review in the dependent #116 change.
pub fn check_for_restart(app: &AppHandle) -> Result<(), String> {
    if crate::terminal::starting(app)? {
        return Err("터미널을 시작하는 중입니다. 시작이 끝난 뒤 다시 확인하세요.".into());
    }
    check_impl(app, true)
}

fn check_impl(app: &AppHandle, allow_sessions: bool) -> Result<(), String> {
    if path(app).exists() {
        return Err("이전 업데이트의 창 복원을 먼저 완료해야 합니다".into());
    }
    let windows = app.webview_windows();
    if !allow_sessions {
        check_live_window_kinds(windows.keys().map(String::as_str))?;
    } else {
        for label in windows.keys().map(String::as_str) {
            if label.starts_with("terminal-") && !crate::terminal::contains(app, label) {
                return Err("확인할 수 없는 터미널 창이 있어 업데이트를 보류했습니다.".into());
            }
            if (label.starts_with("webview-") || label.starts_with("webpage-"))
                && !crate::web::contains_label(app, label)
            {
                return Err("확인할 수 없는 웹 창이 있어 업데이트를 보류했습니다.".into());
            }
        }
    }
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
/// Automatic installs retain the conservative session block.
pub fn save(app: &AppHandle) -> Result<(), String> {
    save_impl(app, false)
}

#[allow(dead_code)] // Consumed by the explicit update impact review in the dependent #116 change.
pub fn save_for_restart(app: &AppHandle) -> Result<(), String> {
    // The user's final install action rechecks readiness here before writing.
    save_impl(app, true)?;
    Ok(())
}

fn save_impl(app: &AppHandle, allow_sessions: bool) -> Result<(), String> {
    if allow_sessions {
        check_for_restart(app)?;
    } else {
        check(app)?;
    }
    let windows = app.webview_windows();

    let mut snapshot_windows = Vec::new();
    for window in windows.values() {
        let label = window.label();
        if !permitted_label(label) {
            continue;
        }
        snapshot_windows.push(state(app, window)?);
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
    for saved in &snapshot.windows {
        if app.get_webview_window(&saved.label).is_some() {
            continue;
        }
        if let Some(id) = saved.label.strip_prefix("terminal-") {
            let id = format!("terminal-{id}");
            crate::terminal::open_hidden_for_restart(
                app,
                &id,
                saved.terminal_last_command.clone(),
            )?;
        } else if let Some(id) = saved.label.strip_prefix("webview-") {
            crate::web::restore_session(app, id, None)?;
        } else if let Some(id) = saved.label.strip_prefix("webpage-") {
            crate::web::restore_session(app, id, saved.web_url.as_deref())?;
        }
    }
    let windows = app.webview_windows();
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

    #[test]
    fn restart_roundtrip_preserves_each_session_without_storing_raw_commands() {
        use serde_json::{json, to_value};
        let mut windows = Vec::new();
        for (label, field, value) in [
            ("webpage-web-first", "web_url", "https://example.com/first"),
            (
                "webpage-web-second",
                "web_url",
                "https://example.com/second",
            ),
            ("terminal-111", "terminal_last_command", "shell"),
            ("terminal-222", "terminal_last_command", "git"),
        ] {
            let mut window = to_value(sample(label)).unwrap();
            window[field] = json!(value);
            windows.push(window);
        }
        let snapshot: Snapshot =
            serde_json::from_value(json!({"version":1,"windows":windows})).unwrap();
        assert!(validate(&snapshot).is_ok());
        let saved = to_value(&snapshot).unwrap();
        for (idx, expected) in windows.iter().enumerate() {
            for field in ["label", "web_url", "terminal_last_command"] {
                assert_eq!(saved["windows"][idx][field], expected[field]);
            }
        }
        windows[2]["terminal_last_command"] = json!("printf sensitive-input");
        let unsafe_snapshot: Snapshot =
            serde_json::from_value(json!({"version":1,"windows":windows})).unwrap();
        assert!(validate(&unsafe_snapshot).is_err());
    }

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
            web_url: None,
            terminal_last_command: None,
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
    fn manual_snapshot_accepts_valid_web_and_terminal_resume_metadata_only() {
        let mut page = sample("webpage-web-102");
        page.web_url = Some("https://example.com/path".into());
        let mut terminal = sample("terminal-123");
        terminal.terminal_last_command = Some("codex".into());
        let snapshot = Snapshot {
            version: 1,
            windows: vec![page.clone(), terminal.clone()],
        };
        validate(&snapshot).unwrap();
        let mut unsafe_command = terminal;
        unsafe_command.terminal_last_command = Some("rm -rf /".into());
        assert!(validate(&Snapshot {
            version: 1,
            windows: vec![unsafe_command]
        })
        .is_err());
        page.web_url = Some("javascript:alert(1)".into());
        assert!(validate(&Snapshot {
            version: 1,
            windows: vec![page]
        })
        .is_err());
    }

    #[test]
    fn rejects_welcome_and_remote_window_with_same_session_id_before_creating_windows() {
        let mut page = sample("webpage-web-shared");
        page.web_url = Some("https://example.com/".into());
        let snapshot = Snapshot {
            version: 1,
            windows: vec![sample("webview-web-shared"), page],
        };
        assert!(validate(&snapshot).is_err());
    }

    #[test]
    fn restore_metadata_is_bound_to_its_window_kind() {
        let mut note = sample("note-abc");
        note.terminal_last_command = Some("codex".into());
        assert!(validate(&Snapshot {
            version: 1,
            windows: vec![note]
        })
        .is_err());

        let mut terminal = sample("terminal-123");
        terminal.web_url = Some("https://example.com/".into());
        assert!(validate(&Snapshot {
            version: 1,
            windows: vec![terminal]
        })
        .is_err());
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
