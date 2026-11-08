//! Downloading never grants permission to exit. Native activity reservations and
//! an acknowledgement from every WebView precede the final installation gate.
use crate::update_policy::Safety;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_updater::{Update, UpdaterExt};
#[cfg(feature = "verification-updater")]
const ENDPOINT: &str = env!("GIBBON_VERIFICATION_UPDATE_URL");
#[cfg(not(feature = "verification-updater"))]
const ENDPOINT: &str =
    "https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json";
const VERIFICATION: bool = cfg!(feature = "verification-updater");
const FAST: bool = VERIFICATION && option_env!("GIBBON_VERIFICATION_STANDARD_TIMING").is_none();
const IDLE_SECONDS: f64 = if FAST { 30. } else { 300. };
const CHECK_SECONDS: u64 = if FAST { 60 } else { 21600 };
const RETRY_SECONDS: u64 = if FAST { 5 } else { 15 };
const NOTICE_LABEL: &str = "updater-notice";
const PUBLIC_KEY: &str = match option_env!("TAURI_UPDATER_PUBLIC_KEY") {
    Some(key) => key,
    None => "",
};
#[derive(Clone, Serialize)]
pub struct Status {
    phase: String,
    version: Option<String>,
    current_version: String,
    downloaded: u64,
    total: Option<u64>,
    message: String,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            phase: "disabled".into(),
            version: None,
            current_version: env!("CARGO_PKG_VERSION").into(),
            downloaded: 0,
            total: None,
            message: "개발 실행 또는 업데이트 키가 없는 빌드에서는 업데이트를 실행하지 않습니다"
                .into(),
        }
    }
}
#[derive(Default)]
pub struct Updates {
    status: Mutex<Status>,
    pending: Mutex<Option<(Update, Vec<u8>)>>,
    safety: Arc<Mutex<Safety>>,
    last_handshake: Mutex<serde_json::Value>,
    last_check_failure: Mutex<Option<&'static str>>,
    incident: Mutex<Option<(String, &'static str, Instant, bool)>>,
    notice_lock: Mutex<()>,
    notice_pending_version: Mutex<Option<String>>,
    notice_loaded: Mutex<bool>,
    notice_displayed_version: Mutex<Option<String>>,
}
pub struct Activity(Arc<Mutex<Safety>>);
impl Drop for Activity {
    fn drop(&mut self) {
        if let Ok(mut s) = self.0.lock() {
            s.activities = s.activities.saturating_sub(1);
        }
    }
}
pub fn activity(app: &tauri::AppHandle) -> Result<Activity, String> {
    let state = app.state::<Updates>();
    state.safety.lock().map_err(err)?.enter()?;
    Ok(Activity(state.safety.clone()))
}
pub fn frozen(app: &tauri::AppHandle) -> bool {
    app.try_state::<Updates>()
        .is_some_and(|s| s.safety.lock().map(|s| s.frozen).unwrap_or(true))
}
fn enabled() -> bool {
    !cfg!(debug_assertions)
        && (VERIFICATION || crate::instance::current().id.is_none())
        && !PUBLIC_KEY.trim().is_empty()
        && cfg!(any(target_os = "macos", windows))
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn status(app: &tauri::AppHandle) -> Result<Status, String> {
    Ok(app.state::<Updates>().status.lock().map_err(err)?.clone())
}
pub fn diagnostics(app: &tauri::AppHandle) -> serde_json::Value {
    let updates = app.state::<Updates>();
    let last = updates.last_handshake.lock().ok().map(|v| v.clone());
    serde_json::json!({
        "enabled": enabled(),
        "status": status(app).ok(),
        "idle_threshold_seconds": IDLE_SECONDS,
        "check_seconds": CHECK_SECONDS,
        "retry_seconds": RETRY_SECONDS,
        "last_window_reports": last,
        "last_check_failure": updates.last_check_failure.lock().ok().and_then(|v| *v),
    })
}
fn set(app: &tauri::AppHandle, phase: &str, message: &str) {
    if let Ok(mut s) = app.state::<Updates>().status.lock() {
        s.phase = phase.into();
        s.message = message.into();
    }
}
fn set_ready_message(app: &tauri::AppHandle, version: &str, message: &str) {
    if let Ok(mut current) = app.state::<Updates>().status.lock() {
        if current.phase == "ready" && current.version.as_deref() == Some(version) {
            current.message = message.into();
        }
    }
}
fn set_install_message(
    app: &tauri::AppHandle,
    version: &str,
    phase: &str,
    message: &str,
) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let mut current = updates.status.lock().map_err(err)?;
    if current.phase != "preparing" || current.version.as_deref() != Some(version) {
        return Err("업데이트 상태가 바뀌어 설치를 보류했습니다".into());
    }
    current.phase = phase.into();
    current.message = message.into();
    Ok(())
}
fn manager(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != crate::manager::LABEL {
        Err("관리 창에서만 업데이트를 실행할 수 있습니다".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn updater_status(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    status(window.app_handle())
}
#[tauri::command]
pub async fn updater_check(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    check(window.app_handle().clone(), true).await?;
    status(window.app_handle())
}
#[tauri::command]
pub async fn updater_install(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || install(&app, false))
        .await
        .map_err(err)??;
    status(window.app_handle())
}
fn notice_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == NOTICE_LABEL {
        Ok(())
    } else {
        Err("업데이트 안내 창에서만 실행할 수 있습니다".into())
    }
}
#[tauri::command]
pub fn updater_notice_status(window: WebviewWindow) -> Result<Status, String> {
    notice_window(&window)?;
    status(window.app_handle())
}
#[tauri::command]
pub fn updater_notice_ready(window: WebviewWindow) -> Result<(), String> {
    notice_window(&window)?;
    *window
        .state::<Updates>()
        .notice_loaded
        .lock()
        .map_err(err)? = true;
    present_pending_notice(&window)
}
#[tauri::command]
pub async fn updater_notice_install(window: WebviewWindow) -> Result<Status, String> {
    notice_window(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || install(&app, false))
        .await
        .map_err(err)??;
    status(window.app_handle())
}
#[tauri::command]
pub fn updater_notice_dismiss(window: WebviewWindow) -> Result<(), String> {
    close_notice(&window)
}
#[tauri::command]
pub fn updater_ack(
    window: tauri::WebviewWindow,
    epoch: u64,
    safe: bool,
    report: Option<crate::update_policy::WindowReport>,
) -> Result<(), String> {
    let updates = window.state::<Updates>();
    let visible = window.is_visible().ok();
    let mut gate = updates.safety.lock().map_err(err)?;
    if gate.frozen && gate.epoch == epoch && gate.acknowledgements.contains_key(window.label()) {
        if let Some(mut report) = report {
            report.visible = visible;
            gate.reports.insert(window.label().to_owned(), report);
        }
    }
    gate.acknowledge(window.label(), epoch, safe);
    Ok(())
}

fn builder(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    configured_builder(app)?.build().map_err(err)
}
fn configured_builder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri_plugin_updater::UpdaterBuilder, String> {
    if VERIFICATION {
        let configured_key = app
            .config()
            .plugins
            .0
            .get("updater")
            .and_then(|config| config.get("pubkey"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if configured_key.trim() != PUBLIC_KEY.trim() {
            return Err("검증 빌드의 업데이트 공개키가 Tauri 설정과 일치하지 않습니다".into());
        }
    }
    Ok(app
        .updater_builder()
        .pubkey(PUBLIC_KEY)
        .endpoints(vec![ENDPOINT.parse().map_err(err)?])
        .map_err(err)?
        .timeout(Duration::from_secs(30))
        .version_comparator(|current, release| {
            release.version.pre.is_empty() && release.version > current
        }))
}

fn cache(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.state::<crate::storage::Store>().0.join(".update-cache")
}
fn write_cache(app: &tauri::AppHandle, update: &Update, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let dir = cache(app);
    std::fs::create_dir_all(&dir).map_err(err)?;
    for (name, data) in [
        ("package", bytes.to_vec()),
        (
            "manifest.json",
            serde_json::to_vec(&update.raw_json).map_err(err)?,
        ),
    ] {
        let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(err)?;
        file.write_all(&data).map_err(err)?;
        file.as_file().sync_all().map_err(err)?;
        file.persist(dir.join(name)).map_err(err)?;
    }
    Ok(())
}
fn restore_cache(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = cache(app);
    if !dir.join("manifest.json").exists() {
        return Ok(());
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).map_err(err)?)
            .map_err(err)?;
    let Some(update) = builder(app)?.restore_update(manifest).map_err(err)? else {
        return Ok(());
    };
    validate_download(&update)?;
    let bytes = std::fs::read(dir.join("package")).map_err(err)?;
    verify_package(&update, &bytes)?;
    ready(app, update, bytes)
}
fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => "unsupported",
    }
}
fn package_name(version: &str, target: &str) -> String {
    if target == "windows-x86_64" {
        format!("Agent.Gibbon_{version}_x64-setup.exe")
    } else {
        format!("Agent.Gibbon_{version}_{target}.app.tar.gz")
    }
}
fn validate_download(update: &Update) -> Result<(), String> {
    let u = &update.download_url;
    let filename = package_name(&update.version, platform()).replace(' ', "%20");
    #[cfg(feature = "verification-updater")]
    if platform() != "unsupported" {
        let feed: tauri::Url = ENDPOINT.parse().map_err(err)?;
        if (feed.scheme() == "https"
            || (feed.scheme() == "http" && feed.host_str() == Some("127.0.0.1")))
            && feed
                .join(&format!("v{}/{filename}", update.version))
                .map_err(err)?
                == *u
        {
            return Ok(());
        }
        return Err("검증 피드의 업데이트 파일 경로가 일치하지 않습니다".into());
    }
    if platform() == "unsupported"
        || u.scheme() != "https"
        || u.host_str() != Some("github.com")
        || u.path()
            != format!(
                "/egoing/agent-gibbon/releases/download/v{}/{filename}",
                update.version
            )
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err("이 플랫폼의 공식 릴리스 업데이트 파일이 아닙니다".into());
    }
    Ok(())
}
fn verify_package(update: &Update, bytes: &[u8]) -> Result<(), String> {
    use base64::Engine;
    update.verify(bytes).map_err(err)?;
    // Only inspect this comment AFTER the plugin authenticates it cryptographically.
    // Renamed platform archives are re-signed in CI to bind architecture as well as version.
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(update.signature.trim())
        .map_err(err)?;
    let decoded = String::from_utf8(decoded).map_err(err)?;
    let comment = decoded
        .lines()
        .nth(2)
        .and_then(|s| s.strip_prefix("trusted comment: "))
        .ok_or("서명에 플랫폼 파일 이름이 없습니다")?;
    let files: Vec<_> = comment
        .split('\t')
        .filter_map(|s| s.strip_prefix("file:"))
        .collect();
    if files != vec![package_name(&update.version, platform())] {
        return Err("서명된 업데이트 플랫폼이 일치하지 않습니다".into());
    }
    Ok(())
}

fn ready(app: &tauri::AppHandle, update: Update, bytes: Vec<u8>) -> Result<(), String> {
    let mut s = app.state::<Updates>().status.lock().map_err(err)?.clone();
    s.phase = "ready".into();
    s.version = Some(update.version.clone());
    s.downloaded = bytes.len() as u64;
    s.total = Some(bytes.len() as u64);
    s.message = "다운로드와 서명 검증을 마쳤습니다. 작업이 안전하게 끝나면 설치합니다".into();
    *app.state::<Updates>().pending.lock().map_err(err)? = Some((update, bytes));
    *app.state::<Updates>().status.lock().map_err(err)? = s;
    evaluate_ready_later(app);
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct NoticeRecord {
    version: String,
}

fn notice_record_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.state::<crate::storage::Store>()
        .0
        .join(".update-notice.json")
}

fn read_notice_version(path: &std::path::Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<NoticeRecord>(&bytes).ok())
        .map(|record| record.version)
}

fn last_notice_version(app: &tauri::AppHandle) -> Option<String> {
    read_notice_version(&notice_record_path(app))
}

fn write_notice_version(path: &std::path::Path, version: &str) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("업데이트 안내 상태 폴더가 없습니다")?;
    std::fs::create_dir_all(parent).map_err(err)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".update-notice-")
        .tempfile_in(parent)
        .map_err(err)?;
    serde_json::to_writer(
        &mut temp,
        &NoticeRecord {
            version: version.into(),
        },
    )
    .map_err(err)?;
    temp.write_all(b"\n").map_err(err)?;
    temp.as_file().sync_all().map_err(err)?;
    temp.persist(path).map_err(|e| err(e.error))?;
    Ok(())
}

fn close_notice_version(displayed: Option<&str>, _current_status: Option<&str>) -> Option<String> {
    displayed.map(str::to_owned)
}

fn mark_notice_seen(app: &tauri::AppHandle, version: &str) -> Result<(), String> {
    write_notice_version(&notice_record_path(app), version)
}

fn evaluate_ready_later(app: &tauri::AppHandle) {
    if !enabled() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        evaluate_ready(&app, true);
    });
}

fn preflight_reason(app: &tauri::AppHandle) -> Option<String> {
    if crate::terminal::update_blocked(app).unwrap_or(true) {
        return Some("터미널 작업이 실행 중이거나 시작되고 있습니다. 종료를 선택해 작업을 마친 뒤, 숨긴 터미널도 관리 창에서 삭제하고 다시 시도하세요.".into());
    }
    if crate::capture::update_blocked(app) {
        return Some("화면 캡처나 영역 선택이 끝난 뒤 다시 시도하세요.".into());
    }
    if crate::stash::active(app) || crate::stash::chord_busy(app) {
        return Some("메모 숨김·복원 동작이 끝난 뒤 다시 시도하세요.".into());
    }
    crate::update_restore::check(app).err()
}

fn evaluate_ready(app: &tauri::AppHandle, try_automatic_install: bool) {
    if !enabled() || crate::privacy::onboarding_required(app) {
        return;
    }
    let Ok(current) = status(app) else { return };
    if current.phase != "ready" {
        return;
    }
    let Some(version) = current.version else {
        return;
    };
    let native_reason = preflight_reason(app);
    let idle = idle_seconds();
    let idle_ready = crate::update_policy::idle_hold_reason(idle, IDLE_SECONDS).is_none();
    if try_automatic_install && native_reason.is_none() && idle_ready {
        // Attempt immediately after verification once idle is already met.
        // install() repeats every safety check before writing state or restart.
        let _ = install(app, true);
    }
    let Ok(current) = status(app) else { return };
    if current.phase != "ready" {
        return;
    }
    let reason =
        crate::update_policy::ready_hold_reason(idle, IDLE_SECONDS, native_reason.as_deref());
    if let Some(reason) = reason {
        set_ready_message(app, &version, &reason);
    } else if current
        .message
        .starts_with("다운로드와 서명 검증을 마쳤습니다")
    {
        set_ready_message(
            app,
            &version,
            "작업 중인 창이 있어 자동 설치를 보류했습니다. 상태를 확인한 뒤 다시 시도하세요.",
        );
    }
    if let Err(error) = show_ready_notice(app) {
        set_ready_message(
            app,
            &version,
            "새 버전은 준비됐지만 안내 창을 표시하지 못했습니다. 설정에서 업데이트 상태를 확인하세요.",
        );
        eprintln!("업데이트 안내 창 표시 실패: {error}");
    }
}

fn show_without_focus(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let ptr = window.ns_window().map_err(err)?;
        // SAFETY: Tauri owns this live NSWindow and this closure runs on the
        // main thread. Ordering front does not make it key.
        unsafe {
            (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
        }
        crate::focus::restore(window);
    }
    #[cfg(windows)]
    {
        crate::windows_window::show_without_focus(window)?;
        crate::focus::restore(window);
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    window.show().map_err(err)?;
    Ok(())
}

fn show_ready_notice(app: &tauri::AppHandle) -> Result<(), String> {
    let current = status(app)?;
    if current.phase != "ready" || frozen(app) {
        return Ok(());
    }
    let Some(version) = current.version else {
        return Ok(());
    };
    let updates = app.state::<Updates>();
    let loaded = {
        let _serial = updates.notice_lock.lock().map_err(err)?;
        if !crate::update_policy::should_show_notice(&version, last_notice_version(app).as_deref())
        {
            return Ok(());
        }
        *updates.notice_pending_version.lock().map_err(err)? = Some(version.clone());
        *updates.notice_loaded.lock().map_err(err)?
    };
    on_main(app, move |app| {
        let updates = app.state::<Updates>();
        let _serial = updates.notice_lock.lock().map_err(err)?;
        if updates
            .notice_pending_version
            .lock()
            .map_err(err)?
            .as_deref()
            != Some(&version)
        {
            return Ok(());
        }
        let current = updates.status.lock().map_err(err)?;
        if current.phase != "ready"
            || current.version.as_deref() != Some(&version)
            || frozen(&app)
            || !crate::update_policy::should_show_notice(
                &version,
                last_notice_version(&app).as_deref(),
            )
        {
            return Ok(());
        }
        let window = if let Some(window) = app.get_webview_window(NOTICE_LABEL) {
            window
        } else {
            crate::focus::remember(&app, NOTICE_LABEL);
            WebviewWindowBuilder::new(
                &app,
                NOTICE_LABEL,
                tauri::WebviewUrl::App("updater-notice.html".into()),
            )
            .title("Agent Gibbon · 업데이트")
            .inner_size(360., 140.)
            .min_inner_size(320., 140.)
            .decorations(true)
            .resizable(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
            .build()
            .map_err(err)?
        };
        place_notice(&app, &window)?;
        if loaded {
            if !window.is_visible().map_err(err)? {
                mark_notice_seen(&app, &version)?;
                show_without_focus(&window)?;
            } else {
                mark_notice_seen(&app, &version)?;
            }
            *updates.notice_displayed_version.lock().map_err(err)? = Some(version.clone());
            *updates.notice_pending_version.lock().map_err(err)? = None;
        }
        drop(current);
        Ok(())
    })
}

// Work area and outer size are physical pixels; only the inset is logical.
fn notice_position(
    work: &tauri::PhysicalRect<i32, u32>,
    size: tauri::PhysicalSize<u32>,
    scale: f64,
) -> tauri::PhysicalPosition<i32> {
    let margin = (16. * scale).round() as i64;
    let axis = |start: i32, available: u32, occupied: u32| {
        let space = i64::from(available)
            .saturating_sub(i64::from(occupied))
            .max(0);
        (i64::from(start) + (space - margin).max(0)).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
            as i32
    };
    tauri::PhysicalPosition::new(
        axis(work.position.x, work.size.width, size.width),
        axis(work.position.y, work.size.height, size.height),
    )
}

fn place_notice(app: &tauri::AppHandle, window: &WebviewWindow) -> Result<(), String> {
    if let Some(monitor) = window
        .current_monitor()
        .map_err(err)?
        .or(app.primary_monitor().map_err(err)?)
    {
        window
            .set_position(notice_position(
                monitor.work_area(),
                window.outer_size().map_err(err)?,
                monitor.scale_factor(),
            ))
            .map_err(err)?;
    }
    Ok(())
}

fn present_pending_notice(window: &WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let updates = app.state::<Updates>();
    let _serial = updates.notice_lock.lock().map_err(err)?;
    let Some(version) = updates.notice_pending_version.lock().map_err(err)?.clone() else {
        return Ok(());
    };
    drop(_serial);
    on_main(app, {
        let window = window.clone();
        move |app| {
            let updates = app.state::<Updates>();
            let _serial = updates.notice_lock.lock().map_err(err)?;
            if updates
                .notice_pending_version
                .lock()
                .map_err(err)?
                .as_deref()
                != Some(&version)
            {
                return Ok(());
            }
            let current = updates.status.lock().map_err(err)?;
            if current.phase != "ready"
                || current.version.as_deref() != Some(&version)
                || frozen(&app)
                || !crate::update_policy::should_show_notice(
                    &version,
                    last_notice_version(&app).as_deref(),
                )
            {
                return Ok(());
            }
            if window.is_visible().map_err(err)? {
                mark_notice_seen(&app, &version)?;
            } else {
                mark_notice_seen(&app, &version)?;
                show_without_focus(&window)?;
            }
            *updates.notice_displayed_version.lock().map_err(err)? = Some(version.clone());
            *updates.notice_pending_version.lock().map_err(err)? = None;
            drop(current);
            Ok(())
        }
    })
}

pub fn close_notice(window: &WebviewWindow) -> Result<(), String> {
    notice_window(window)?;
    let app = window.app_handle();
    let updates = app.state::<Updates>();
    let _serial = updates.notice_lock.lock().map_err(err)?;
    let current_version = status(app)?.version;
    let displayed_version = updates
        .notice_displayed_version
        .lock()
        .map_err(err)?
        .clone();
    if let Some(version) =
        close_notice_version(displayed_version.as_deref(), current_version.as_deref())
    {
        mark_notice_seen(app, &version)?;
    }
    *updates.notice_pending_version.lock().map_err(err)? = None;
    window.hide().map_err(err)
}
fn check_failed(current: &mut Status, previous: Option<Status>) {
    if let Some(mut ready) = previous {
        ready.message =
            "새 업데이트 확인에 실패했습니다. 이미 검증한 업데이트는 설치할 수 있습니다".into();
        *current = ready;
        return;
    }
    current.phase = "error".into();
    current.message =
        "업데이트를 받지 못했습니다. 네트워크 또는 서명을 확인한 뒤 다시 시도하세요".into();
}
async fn check(app: tauri::AppHandle, manual: bool) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if crate::privacy::onboarding_required(&app) {
        return Err("먼저 약관을 확인하세요".into());
    }
    let previous = {
        let state = app.state::<Updates>();
        let mut s = state.status.lock().map_err(err)?;
        if matches!(
            s.phase.as_str(),
            "checking" | "downloading" | "preparing" | "installing"
        ) || (s.phase == "ready" && !manual)
        {
            return Ok(());
        }
        let previous = (s.phase == "ready").then(|| s.clone());
        s.phase = "checking".into();
        s.message = "업데이트를 확인하고 있습니다".into();
        s.downloaded = 0;
        s.total = None;
        previous
    };
    let mut stage = "manifest";
    let result = async {
        crate::telemetry::checkpoint("updater.check");
        let Some(mut update) = builder(&app)?.check().await.map_err(err)? else {
            set(&app, "idle", "최신 버전입니다");
            return Ok::<_, String>(());
        };
        stage = "asset_url";
        validate_download(&update)?;
        update.timeout = Some(Duration::from_secs(300));
        {
            let state = app.state::<Updates>();
            let mut s = state.status.lock().map_err(err)?;
            s.version = Some(update.version.clone());
            s.phase = "downloading".into();
            s.message = "업데이트를 다운로드하고 있습니다".into();
        }
        stage = "download";
        crate::telemetry::checkpoint("updater.download");
        let bytes = update
            .download(
                |n, total| {
                    if let Ok(mut s) = app.state::<Updates>().status.lock() {
                        s.downloaded += n as u64;
                        s.total = total;
                    }
                },
                || {},
            )
            .await
            .map_err(err)?;
        stage = "signature_or_platform_binding";
        verify_package(&update, &bytes)?;
        crate::telemetry::checkpoint("updater.verified");
        stage = "cache_write";
        write_cache(&app, &update, &bytes)?;
        stage = "ready";
        ready(&app, update, bytes)
    }
    .await;
    *app.state::<Updates>()
        .last_check_failure
        .lock()
        .map_err(err)? = result.is_err().then_some(stage);
    if result.is_err() {
        report_incident(&app, env!("CARGO_PKG_VERSION"), "check_failed");
        check_failed(
            &mut *app.state::<Updates>().status.lock().map_err(err)?,
            previous,
        );
    }
    result
}
fn on_main<T: Send + 'static>(
    app: &tauri::AppHandle,
    work: impl FnOnce(tauri::AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let a = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(work(a));
    })
    .map_err(err)?;
    rx.recv().map_err(err)?
}
fn safe_native(app: &tauri::AppHandle) -> Result<(), String> {
    if crate::terminal::update_blocked(app)? {
        return Err("터미널 세션을 종료한 뒤 설치할 수 있습니다. 숨긴 터미널도 포함됩니다".into());
    }
    crate::update_restore::check(app)?;
    if crate::capture::update_blocked(app)
        || crate::stash::active(app)
        || crate::stash::chord_busy(app)
    {
        return Err("캡처 또는 창 전환이 끝나면 설치합니다".into());
    }
    Ok(())
}
fn release(app: &tauri::AppHandle, epoch: u64) {
    let released = app
        .state::<Updates>()
        .safety
        .lock()
        .is_ok_and(|mut safety| safety.release_epoch(epoch));
    if released {
        let _ = app.emit("updater-release", ());
    }
}
fn report_incident(app: &tauri::AppHandle, version: &str, kind: &'static str) {
    let updates = app.state::<Updates>();
    let Ok(mut current) = updates.incident.lock() else {
        return;
    };
    if kind == "protected" {
        *current = None;
        return;
    }
    if !current
        .as_ref()
        .is_some_and(|(v, k, _, _)| v == version && *k == kind)
    {
        *current = Some((version.to_owned(), kind, Instant::now(), false));
    }
    if let Some((_, _, start, sent)) = current.as_mut() {
        if !*sent && (kind != "pending_request" || start.elapsed() >= Duration::from_secs(120)) {
            *sent = true;
            crate::telemetry::diagnostic(
                match kind {
                    "check_failed" => "Updater check failed",
                    "ack_timeout" => "Updater acknowledgement timeout",
                    "pending_request" => "Updater repeated pending request",
                    _ => "Updater installation failed",
                },
                "app",
                start.elapsed().as_secs_f64() * 1000.,
            );
        }
    }
}
fn record_handshake(app: &tauri::AppHandle, epoch: u64, version: &str, outcome: &str) {
    let state = app.state::<Updates>();
    let Ok(gate) = state.safety.lock() else {
        return;
    };
    if gate.epoch != epoch {
        return;
    }
    let windows: Vec<_> = gate.acknowledgements.iter().enumerate().map(|(index, (label, ack))| {
        serde_json::json!({"number": index + 1, "kind": crate::telemetry::window_kind(label), "ack": ack, "report": gate.reports.get(label)})
    }).collect();
    drop(gate);
    if let Ok(mut latest) = state.last_handshake.lock() {
        *latest = serde_json::json!({"epoch": epoch, "windows": windows, "source_version": env!("CARGO_PKG_VERSION"), "target_version": version, "outcome": outcome, "timestamp_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()});
        let root = &app.state::<crate::storage::Store>().0;
        if let Ok(mut tmp) = tempfile::NamedTempFile::new_in(root) {
            if serde_json::to_writer(&mut tmp, &*latest).is_ok() {
                let _ = tmp.persist(root.join(".last-update-attempt.json"));
            }
        }
    };
}
fn install(app: &tauri::AppHandle, automatic: bool) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if !crate::update_policy::idle_allows_install(automatic, idle_seconds(), IDLE_SECONDS) {
        return Ok(());
    }
    let install_version = {
        let state = app.state::<Updates>();
        let mut s = state.status.lock().map_err(err)?;
        if s.phase != "ready" {
            return Ok(());
        }
        let Some(version) = s.version.clone() else {
            return Ok(());
        };
        s.phase = "preparing".into();
        s.message = "작업 상태를 확인하고 있습니다".into();
        version
    };
    let mut snapshot_written = false;
    let mut prepared_epoch = None;
    let result: Result<(), String> = (|| {
        if crate::privacy::onboarding_required(app) {
            return Err("먼저 약관을 확인하세요".into());
        }
        crate::telemetry::checkpoint("updater.prepare");
        let epoch = on_main(app, |app| {
            safe_native(&app)?;
            // Remote page WebViews cannot invoke the local updater guard. Keep
            // them outside the acknowledgement barrier; local web Welcome and
            // companion toolbars still participate through the local guard.
            let labels: Vec<String> = app
                .webview_windows()
                .keys()
                .filter(|label| !label.starts_with("webpage-"))
                .cloned()
                .collect();
            let epoch = {
                let state = app.state::<Updates>();
                let mut safety = state.safety.lock().map_err(err)?;
                safety.prepare(labels)?
            };
            if let Err(error) = app.emit("updater-prepare", epoch) {
                release(&app, epoch);
                return Err(err(error));
            }
            Ok(epoch)
        })?;
        prepared_epoch = Some(epoch);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if app.state::<Updates>().safety.lock().map_err(err)?.refused() {
                let updates = app.state::<Updates>();
                let gate = updates.safety.lock().map_err(err)?;
                return Err(gate.refusal_reason().into());
            }
            if app.state::<Updates>().safety.lock().map_err(err)?.ready() {
                break;
            }
            if Instant::now() >= deadline {
                return Err("창의 설치 준비 응답을 기다리다 시간이 초과됐습니다".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        record_handshake(app, epoch, &install_version, "prepared");
        let expected_version = install_version.clone();
        on_main(app, move |app| {
            safe_native(&app)?;
            {
                let updates = app.state::<Updates>();
                let current = updates.status.lock().map_err(err)?;
                if current.phase != "preparing"
                    || current.version.as_deref() != Some(&expected_version)
                {
                    return Err("업데이트 상태가 바뀌어 설치를 보류했습니다".into());
                }
            }
            let safety = app.state::<Updates>();
            let gate = safety.safety.lock().map_err(err)?;
            if gate.epoch != epoch || !gate.ready() {
                return Err("작업 상태가 바뀌어 설치를 보류했습니다".into());
            }
            if automatic && idle_seconds().is_none_or(|s| s < IDLE_SECONDS) {
                return Err("다시 사용 중이어서 설치를 보류했습니다".into());
            }
            crate::update_restore::save(&app)?;
            Ok(())
        })?;
        snapshot_written = true;
        set_install_message(
            app,
            &install_version,
            "installing",
            "업데이트를 설치하고 재시작합니다",
        )?;
        let pending = app.state::<Updates>();
        let pending = pending.pending.lock().map_err(err)?;
        let (update, bytes) = pending.as_ref().ok_or("다운로드된 업데이트가 없습니다")?;
        verify_package(update, bytes)?;
        crate::telemetry::checkpoint("updater.install");
        update.install(bytes).map_err(err)?;
        app.restart();
        #[allow(unreachable_code)]
        Ok(())
    })();
    if let Err(ref reason) = result {
        if snapshot_written {
            crate::update_restore::discard(app);
        }
        if let Ok(mut current) = app.state::<Updates>().status.lock() {
            if matches!(current.phase.as_str(), "preparing" | "installing")
                && current.version.as_deref() == Some(&install_version)
            {
                current.phase = "ready".into();
                current.message = reason.clone();
            }
        }
        if let Some(epoch) = prepared_epoch {
            let kind = if snapshot_written {
                "installation_failed"
            } else if reason.contains("초과") {
                "ack_timeout"
            } else if reason.contains("내부 요청") {
                "pending_request"
            } else {
                "protected"
            };
            record_handshake(app, epoch, &install_version, kind);
            report_incident(app, &install_version, kind);
            release(app, epoch);
        }
    }
    // A blocked installation is a retryable ready state, exposed to the UI.
    Ok(())
}
pub fn initialize(app: &tauri::AppHandle) {
    if !enabled() {
        return;
    }
    let path = app
        .state::<crate::storage::Store>()
        .0
        .join(".last-update-attempt.json");
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(previous) = serde_json::from_reader(file) {
            if let Ok(mut report) = app.state::<Updates>().last_handshake.lock() {
                *report = previous;
            }
        }
    }
    set(app, "idle", "업데이트를 자동으로 확인합니다");
    let app = app.clone();
    std::thread::spawn(move || {
        if restore_cache(&app).is_err() {
            set(
                &app,
                "error",
                "보관된 업데이트를 검증하지 못했습니다. 다시 다운로드합니다",
            );
        }
        let mut resumed = status(&app).is_ok_and(|s| s.phase == "ready");
        let mut last = Instant::now() - Duration::from_secs(CHECK_SECONDS);
        std::thread::sleep(Duration::from_secs(15));
        loop {
            if !crate::privacy::onboarding_required(&app)
                && last.elapsed() >= Duration::from_secs(CHECK_SECONDS)
            {
                let _ = tauri::async_runtime::block_on(check(app.clone(), false));
                last = Instant::now();
            }
            let _ = install(&app, !resumed);
            resumed = false;
            evaluate_ready(&app, false);
            std::thread::sleep(Duration::from_secs(RETRY_SECONDS));
        }
    });
}
#[cfg(target_os = "macos")]
fn idle_seconds() -> Option<f64> {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state: i32, event: u32) -> f64;
    }
    // Combined session state, any input event; no accessibility permission or input recording.
    let value = unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) };
    (value.is_finite() && value >= 0.).then_some(value)
}
#[cfg(windows)]
fn idle_seconds() -> Option<f64> {
    #[repr(C)]
    struct LastInput {
        size: u32,
        time: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(info: *mut LastInput) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount() -> u32;
    }
    let mut info = LastInput {
        size: std::mem::size_of::<LastInput>() as u32,
        time: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return None;
    }
    Some(unsafe { GetTickCount() }.wrapping_sub(info.time) as f64 / 1000.)
}
#[cfg(not(any(target_os = "macos", windows)))]
fn idle_seconds() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use serde_json::json;
    use tauri::test::{mock_builder, mock_context, noop_assets};
    fn app() -> tauri::App<tauri::test::MockRuntime> {
        let mut context = mock_context(noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            json!({"pubkey":PUBLIC_KEY, "requireSignedVersion":true}),
        );
        mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap()
    }
    fn manifest(version: &str) -> serde_json::Value {
        json!({"version":version,"platforms":{"darwin-aarch64":{"url":format!("https://github.com/egoing/agent-gibbon/releases/download/v{version}/app.tar.gz"),"signature":"invalid"}}})
    }
    #[test]
    fn notice_sits_inside_bottom_right_work_area_with_real_outer_size() {
        let work = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(0, 48),
            size: tauri::PhysicalSize::new(1920, 1032),
        };
        let size = tauri::PhysicalSize::new(720, 324);
        assert_eq!(
            notice_position(&work, size, 2.),
            tauri::PhysicalPosition::new(1168, 724)
        );
        let work = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(-1920, -1080),
            size: tauri::PhysicalSize::new(1920, 1040),
        };
        assert_eq!(
            notice_position(&work, tauri::PhysicalSize::new(360, 162), 1.),
            tauri::PhysicalPosition::new(-376, -218)
        );
        // A resized or unusually small work area cannot move the title bar off its top/left.
        assert_eq!(
            notice_position(&work, tauri::PhysicalSize::new(2400, 1400), 1.5),
            work.position
        );
        assert_eq!(
            notice_position(&work, tauri::PhysicalSize::new(400, 300), 1.5),
            tauri::PhysicalPosition::new(-424, -364)
        );
    }

    #[test]
    fn production_comparator_rejects_equal_older_and_prerelease_versions() {
        let app = app();
        let updater = configured_builder(app.handle())
            .unwrap()
            .target("darwin-aarch64")
            .build()
            .unwrap();
        for version in [
            app.package_info().version.to_string(),
            "0.0.0".into(),
            "99.0.0-beta.1".into(),
        ] {
            assert!(
                updater
                    .restore_update(manifest(&version))
                    .unwrap()
                    .is_none(),
                "{version}"
            );
        }
        assert!(updater
            .restore_update(manifest("99.0.0"))
            .unwrap()
            .is_some());
        assert!(updater
            .restore_update(json!({"version":"99.0.0","platforms":{}}))
            .is_err());
    }
    #[test]
    fn cached_bytes_and_signature_must_verify_before_any_install() {
        // Public fixture: Tauri updater 2.14.1 offline_tests (MIT/Apache-2.0).
        let public = "untrusted comment: minisign public key: A49EF4746FA6E6D7\nRWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe";
        let signature = "untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==";
        let app = app();
        let updater = configured_builder(app.handle())
            .unwrap()
            .target("darwin-aarch64")
            .pubkey(base64::engine::general_purpose::STANDARD.encode(public))
            .build()
            .unwrap();
        let mut manifest = manifest("2.0.0");
        manifest["platforms"]["darwin-aarch64"]["signature"] =
            json!(base64::engine::general_purpose::STANDARD.encode(signature));
        let update = updater.restore_update(manifest).unwrap().unwrap();
        update.verify(b"test").unwrap();
        let mut inflated = update.raw_json.clone();
        inflated["version"] = json!("99.0.0");
        assert!(updater
            .restore_update(inflated)
            .unwrap()
            .unwrap()
            .verify(b"test")
            .is_err());
        assert!(update.verify(b"Test").is_err());
        let unsigned = updater
            .restore_update(self::manifest("99.0.0"))
            .unwrap()
            .unwrap();
        assert!(unsigned.verify(b"test").is_err());
    }
    #[test]
    fn download_location_is_limited_to_official_https_version_path() {
        let app = app();
        let updater = configured_builder(app.handle())
            .unwrap()
            .target("darwin-aarch64")
            .build()
            .unwrap();
        let mut update = updater.restore_update(manifest("99.0.0")).unwrap().unwrap();
        update.download_url = format!(
            "https://github.com/egoing/agent-gibbon/releases/download/v99.0.0/{}",
            package_name("99.0.0", platform()).replace(' ', "%20")
        )
        .parse()
        .unwrap();
        if VERIFICATION {
            assert!(validate_download(&update).is_err());
            update.download_url = ENDPOINT
                .parse::<tauri::Url>()
                .unwrap()
                .join(&format!("v99.0.0/{}", package_name("99.0.0", platform())))
                .unwrap();
        }
        assert!(validate_download(&update).is_ok());
        for url in [
            "http://github.com/egoing/agent-gibbon/releases/download/v99.0.0/a",
            "https://example.com/a",
            "https://github.com/other/repo/releases/download/v99.0.0/a",
            "https://github.com/egoing/agent-gibbon/releases/download/v1.0.0/a",
        ] {
            update.download_url = url.parse().unwrap();
            assert!(validate_download(&update).is_err());
        }
    }
    #[test]
    fn package_names_match_github_release_asset_normalization() {
        assert_eq!(
            package_name("0.2.9", "darwin-aarch64"),
            "Agent.Gibbon_0.2.9_darwin-aarch64.app.tar.gz"
        );
        assert_eq!(
            package_name("0.2.9", "windows-x86_64"),
            "Agent.Gibbon_0.2.9_x64-setup.exe"
        );
    }
    #[test]
    fn debug_instances_cannot_enable_updater() {
        assert!(!enabled());
    }
    #[test]
    fn signed_platform_binding_rejects_cross_architecture_cache_swap() {
        let app = app();
        let updater = configured_builder(app.handle())
            .unwrap()
            .target("fixture")
            .pubkey(include_str!("../../tests/fixtures/updater/public.key"))
            .build()
            .unwrap();
        for (target, signature) in [
            (
                "darwin-aarch64",
                include_str!("../../tests/fixtures/updater/darwin-aarch64.sig"),
            ),
            (
                "darwin-x86_64",
                include_str!("../../tests/fixtures/updater/darwin-x86_64.sig"),
            ),
            (
                "windows-x86_64",
                include_str!("../../tests/fixtures/updater/windows-x86_64.sig"),
            ),
        ] {
            let update = updater.restore_update(json!({"version":"9.0.0", "platforms":{"fixture":{"url":"https://example.invalid/payload", "signature":signature}}})).unwrap().unwrap();
            let bytes = include_bytes!("../../tests/fixtures/updater/payload");
            update.verify(bytes).unwrap(); // Every source signature is valid for these same bytes.
            assert_eq!(verify_package(&update, bytes).is_ok(), target == platform());
            assert!(verify_package(&update, b"tampered payload").is_err());
        }
    }
    #[test]
    fn offline_manual_recheck_preserves_the_verified_download() {
        let previous = Status {
            phase: "ready".into(),
            version: Some("9.0.0".into()),
            downloaded: 123,
            total: Some(123),
            ..Status::default()
        };
        let mut current = Status {
            phase: "checking".into(),
            ..Status::default()
        };
        check_failed(&mut current, Some(previous));
        assert_eq!(current.phase, "ready");
        assert_eq!(current.version.as_deref(), Some("9.0.0"));
        assert_eq!(current.downloaded, 123);
        assert_eq!(current.total, Some(123));
        check_failed(&mut current, None);
        assert_eq!(current.phase, "error");
    }

    #[test]
    fn notice_version_is_atomically_persisted_and_deduplicated_across_reads() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested").join(".update-notice.json");
        assert_eq!(read_notice_version(&path), None);
        write_notice_version(&path, "1.3.0").unwrap();
        assert_eq!(read_notice_version(&path).as_deref(), Some("1.3.0"));
        assert!(!crate::update_policy::should_show_notice(
            "1.3.0",
            read_notice_version(&path).as_deref()
        ));
        assert!(crate::update_policy::should_show_notice(
            "1.4.0",
            read_notice_version(&path).as_deref()
        ));
        write_notice_version(&path, "1.4.0").unwrap();
        assert_eq!(read_notice_version(&path).as_deref(), Some("1.4.0"));
    }

    #[test]
    fn closing_an_old_notice_does_not_mark_a_newer_downloading_version_as_seen() {
        assert_eq!(
            close_notice_version(Some("1.3.0"), Some("1.4.0")),
            Some("1.3.0".into())
        );
        assert_eq!(close_notice_version(None, Some("1.4.0")), None);
    }
}
