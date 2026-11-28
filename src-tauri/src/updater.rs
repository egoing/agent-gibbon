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
const OFFICIAL_ENDPOINT: &str =
    "https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json";
#[cfg(feature = "verification-updater")]
const ENDPOINT: &str = env!("JAJAN_VERIFICATION_UPDATE_URL");
#[cfg(not(feature = "verification-updater"))]
const ENDPOINT: &str = OFFICIAL_ENDPOINT;
const VERIFICATION: bool = cfg!(feature = "verification-updater");
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct TrialPair {
    id: String,
    previous: String,
    latest: String,
}
fn trial_pair() -> Option<TrialPair> {
    // No compatible pair containing this menu has been publicly released yet.
    // This pair is built from the same schema-compatible source for isolated NSIS tests only.
    cfg!(feature = "verification-roundtrip").then(|| TrialPair {
        id: "isolated-schema-v1-20261121".into(),
        previous: "0.3.12".into(),
        latest: "0.3.13".into(),
    })
}
fn pair_allows(pair: &TrialPair, current: &str, target: &str) -> bool {
    [pair.previous.as_str(), pair.latest.as_str()].contains(&current)
        && [pair.previous.as_str(), pair.latest.as_str()].contains(&target)
}
#[derive(Clone, Copy)]
struct TimingProfile {
    idle_seconds: f64,
    check_seconds: u64,
    retry_seconds: u64,
}
const fn timing_profile(verification: bool, standard_timing: bool) -> TimingProfile {
    if verification && !standard_timing {
        TimingProfile {
            idle_seconds: 30.,
            check_seconds: 60,
            retry_seconds: 5,
        }
    } else {
        TimingProfile {
            idle_seconds: 300.,
            check_seconds: 900,
            retry_seconds: 15,
        }
    }
}
const TIMING: TimingProfile = timing_profile(
    VERIFICATION,
    option_env!("JAJAN_VERIFICATION_STANDARD_TIMING").is_some(),
);
const IDLE_SECONDS: f64 = TIMING.idle_seconds;
const CHECK_SECONDS: u64 = TIMING.check_seconds;
const RETRY_SECONDS: u64 = TIMING.retry_seconds;
pub(crate) const NOTICE_LABEL: &str = "updater-notice";
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
    manual_test: bool,
    signature_verified: bool,
    cache_ready: bool,
    failure_stage: Option<String>,
    roundtrip_pair: Option<TrialPair>,
    roundtrip_reason: String,
    roundtrip_active: bool,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            phase: "disabled".into(),
            version: None,
            current_version: env!("CARGO_PKG_VERSION").into(),
            downloaded: 0,
            total: None,
            manual_test: false,
            signature_verified: false,
            cache_ready: false,
            failure_stage: None,
            roundtrip_pair: trial_pair(),
            roundtrip_active: false,
            roundtrip_reason: if trial_pair().is_some() {
                "격리 검증 채널의 호환 버전 쌍입니다. 공식 배포판이 아닙니다.".into()
            } else {
                "개발자 메뉴와 복귀 기능이 포함된 공식 호환 버전 쌍이 아직 없습니다. 이전 버전 교체를 사용할 수 없습니다.".into()
            },
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
// Measure from completion so an offline or slow request also gets a full interval.
fn check_when_due(
    last: &mut Option<Instant>,
    now: impl Fn() -> Instant,
    check: impl FnOnce() -> Result<(), String>,
) {
    if last.is_none_or(|time| now().duration_since(time) >= Duration::from_secs(CHECK_SECONDS)) {
        let _ = check();
        *last = Some(now());
    }
}

fn enabled() -> bool {
    (VERIFICATION || (!cfg!(debug_assertions) && crate::instance::current().id.is_none()))
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
#[tauri::command]
pub async fn updater_test_prepare(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    verification_event(window.app_handle(), "test-prepare-request", None, None);
    check_with_policy(window.app_handle().clone(), true, true).await?;
    status(window.app_handle())
}
#[tauri::command]
pub async fn updater_test_install(window: tauri::WebviewWindow) -> Result<Status, String> {
    manager(&window)?;
    verification_event(window.app_handle(), "test-install-request", None, None);
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || install_with_policy(&app, false, true))
        .await
        .map_err(err)??;
    status(window.app_handle())
}
#[tauri::command]
pub async fn updater_test_roundtrip(
    window: tauri::WebviewWindow,
    destination: String,
) -> Result<Status, String> {
    manager(&window)?;
    let pair = trial_pair().ok_or("공식 호환 시험 버전 쌍이 아직 없습니다")?;
    let target = match destination.as_str() {
        "previous" => pair.previous.clone(),
        "latest" => pair.latest.clone(),
        _ => return Err("시험 방향을 확인할 수 없습니다".into()),
    };
    if !enabled() || !pair_allows(&pair, env!("CARGO_PKG_VERSION"), &target) {
        return Err("이 빌드는 지정된 격리 호환 버전 쌍에 포함되지 않습니다".into());
    }
    let app = window.app_handle().clone();
    if crate::privacy::onboarding_required(&app) {
        return Err("먼저 약관을 확인하세요".into());
    }
    verification_event(&app, "roundtrip-request", Some(&target), None);
    write_trial_hold(&app, &pair)?;
    {
        let updates = app.state::<Updates>();
        let mut current = updates.status.lock().map_err(err)?;
        current.manual_test = true;
        current.roundtrip_active = true;
    }
    check_for_trial(app.clone(), true, true, Some(target.clone())).await?;
    let prepared = status(&app)?;
    if prepared.phase == "ready" && prepared.version.as_deref() == Some(&target) {
        tauri::async_runtime::spawn_blocking(move || install_with_policy(&app, false, true))
            .await
            .map_err(err)??;
    }
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
fn test_builder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri_plugin_updater::UpdaterBuilder, String> {
    Ok(
        configured_builder(app)?.version_comparator(|current, release| {
            release.version.pre.is_empty() && release.version >= current
        }),
    )
}
fn pair_builder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    target: String,
) -> Result<tauri_plugin_updater::UpdaterBuilder, String> {
    let pair = trial_pair().ok_or("호환 시험 버전 쌍이 없습니다")?;
    if !pair_allows(&pair, &app.package_info().version.to_string(), &target) {
        return Err("허용된 호환 버전 쌍이 아닙니다".into());
    }
    let endpoint: tauri::Url = ENDPOINT.parse().map_err(err)?;
    let endpoint = endpoint
        .join(if target == pair.previous {
            "previous.json"
        } else {
            "latest.json"
        })
        .map_err(err)?;
    Ok(configured_builder(app)?
        .endpoints(vec![endpoint])
        .map_err(err)?
        .version_comparator(move |current, release| {
            release.version.pre.is_empty()
                && release.version.to_string() == target
                && pair_allows(&pair, &current.to_string(), &target)
        }))
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

// Local, bounded trial evidence only. No user content or input is recorded and
// ordinary release builds compile out all filesystem access here.
fn verification_event(
    app: &tauri::AppHandle,
    event: &'static str,
    target: Option<&str>,
    automatic: Option<bool>,
) {
    #[cfg(feature = "verification-updater")]
    {
        use std::io::Write;
        let path = app
            .state::<crate::storage::Store>()
            .0
            .join(".verification-update-events.jsonl");
        if std::fs::metadata(&path).is_ok_and(|m| m.len() >= 1024 * 1024) {
            return;
        }
        let record = serde_json::json!({"event":event,"unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(),"pid":std::process::id(),"current_version":env!("CARGO_PKG_VERSION"),"target_version":target,"automatic":automatic});
        if let Ok(mut bytes) = serde_json::to_vec(&record) {
            bytes.push(b'\n');
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = file.write_all(&bytes);
            }
        }
    }
    #[cfg(not(feature = "verification-updater"))]
    let _ = (app, event, target, automatic);
}
fn cache(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.state::<crate::storage::Store>().0.join(".update-cache")
}
fn trial_hold_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.state::<crate::storage::Store>()
        .0
        .join(".developer-update-trial.json")
}
fn write_trial_hold(app: &tauri::AppHandle, pair: &TrialPair) -> Result<(), String> {
    use std::io::Write;
    let path = trial_hold_path(app);
    let parent = path.parent().ok_or("시험 상태 폴더가 없습니다")?;
    std::fs::create_dir_all(parent).map_err(err)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
    serde_json::to_writer(&mut file, pair).map_err(err)?;
    file.flush().map_err(err)?;
    file.as_file().sync_all().map_err(err)?;
    file.persist(path).map_err(err)?;
    Ok(())
}
fn write_cache(
    app: &tauri::AppHandle,
    update: &Update,
    bytes: &[u8],
    manual_test: bool,
    roundtrip: bool,
) -> Result<(), String> {
    use std::io::Write;
    let dir = cache(app);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let mut manifest = update.raw_json.clone();
    if let Some(object) = manifest.as_object_mut() {
        object.remove("_jajan_roundtrip_pair");
    }
    if roundtrip {
        manifest["_jajan_roundtrip_pair"] = serde_json::to_value(trial_pair()).map_err(err)?;
    }
    manifest["_jajan_install_policy"] = serde_json::json!(if manual_test {
        "manual-test-v1"
    } else {
        "automatic-v1"
    });
    for (name, data) in [
        ("package", bytes.to_vec()),
        ("manifest.json", serde_json::to_vec(&manifest).map_err(err)?),
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
    // Preserve an explicit manual hold even when its cached bytes later fail
    // verification. Recovery must not silently convert a test into auto-install.
    if manifest
        .get("_jajan_install_policy")
        .is_some_and(|v| v != "automatic-v1")
    {
        app.state::<Updates>()
            .status
            .lock()
            .map_err(err)?
            .manual_test = true;
    }
    let manual_test = cache_manual_test(&manifest)? || trial_hold_path(app).exists();
    let updater = if manual_test && manifest.get("_jajan_roundtrip_pair").is_some() {
        let recorded: TrialPair =
            serde_json::from_value(manifest["_jajan_roundtrip_pair"].clone()).map_err(err)?;
        if trial_pair().as_ref() != Some(&recorded) {
            return Err("보관된 호환 시험 버전 쌍이 일치하지 않습니다".into());
        }
        let target = manifest["version"]
            .as_str()
            .ok_or("보관된 버전이 없습니다")?
            .to_owned();
        pair_builder(app, target)?.build().map_err(err)?
    } else if manual_test {
        test_builder(app)?.build().map_err(err)?
    } else {
        builder(app)?
    };
    let Some(update) = updater.restore_update(manifest).map_err(err)? else {
        return Ok(());
    };
    validate_download(&update)?;
    let bytes = std::fs::read(dir.join("package")).map_err(err)?;
    verify_package(&update, &bytes)?;
    ready(app, update, bytes, manual_test)?;
    finish_returned_trial(app)
}
fn finish_returned_trial(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(pair) = trial_pair() else {
        return Ok(());
    };
    if env!("CARGO_PKG_VERSION") != pair.latest {
        return Ok(());
    }
    let updates = app.state::<Updates>();
    let mut pending = updates.pending.lock().map_err(err)?;
    let Some((update, bytes)) = pending.as_ref() else {
        return Ok(());
    };
    if update.version != pair.latest
        || update.raw_json.get("_jajan_roundtrip_pair")
            != Some(&serde_json::to_value(&pair).map_err(err)?)
    {
        return Ok(());
    }
    // Only after the signed latest package is observed as the running version.
    write_cache(app, update, bytes, false, false)?;
    if trial_hold_path(app).exists() {
        std::fs::remove_file(trial_hold_path(app)).map_err(err)?;
    }
    *pending = None;
    let mut current = updates.status.lock().map_err(err)?;
    current.manual_test = false;
    current.roundtrip_active = false;
    current.phase = "idle".into();
    current.version = None;
    current.message =
        "최신 버전 복귀를 확인했습니다. 일반 자동 업데이트 정책을 다시 적용합니다.".into();
    Ok(())
}
fn cache_manual_test(manifest: &serde_json::Value) -> Result<bool, String> {
    match manifest.get("_jajan_install_policy") {
        None => Ok(false), // Existing verified automatic caches remain compatible.
        Some(value) if value == "automatic-v1" => Ok(false),
        Some(value) if value == "manual-test-v1" => Ok(true),
        _ => Err("보관된 업데이트의 설치 정책을 확인할 수 없습니다".into()),
    }
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
        format!("Jajan_{version}_x64-setup.exe")
    } else {
        format!("Jajan_{version}_{target}.app.tar.gz")
    }
}
fn validate_download(update: &Update) -> Result<(), String> {
    let u = &update.download_url;
    let filename = package_name(&update.version, platform()).replace(' ', "%20");
    #[cfg(feature = "verification-updater")]
    if ENDPOINT != OFFICIAL_ENDPOINT && platform() != "unsupported" {
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

fn ready(
    app: &tauri::AppHandle,
    update: Update,
    bytes: Vec<u8>,
    manual_test: bool,
) -> Result<(), String> {
    // The durable trial hold also wins over an older automatic cache or an
    // automatic download already in flight when the developer entered the trial.
    let manual_test = manual_test || trial_hold_path(app).exists();
    let mut s = app.state::<Updates>().status.lock().map_err(err)?.clone();
    s.phase = "ready".into();
    s.version = Some(update.version.clone());
    s.downloaded = bytes.len() as u64;
    s.total = Some(bytes.len() as u64);
    s.manual_test = manual_test;
    s.signature_verified = true;
    s.cache_ready = true;
    s.message = "다운로드와 서명 검증을 마쳤습니다. 작업이 안전하게 끝나면 설치합니다".into();
    if manual_test {
        s.message = "수동 시험 대기 중입니다. 준비한 버전 설치·재시작을 눌러야 설치합니다.".into();
    }
    *app.state::<Updates>().pending.lock().map_err(err)? = Some((update, bytes));
    *app.state::<Updates>().status.lock().map_err(err)? = s;
    if !manual_test {
        evaluate_ready_later(app);
    }
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
    if current.manual_test {
        return; // No notice or automatic retry may install a manually prepared package.
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
    if current.phase != "ready" || current.manual_test || frozen(app) {
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
            .title(crate::language::tr("Jajan · 업데이트"))
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
        "업데이트를 받거나 검증하지 못했습니다. 연결 상태를 확인하고 ‘다시 확인’을 누르세요. 반복되면 공식 릴리스의 설치 파일로 수동 설치하세요.".into();
}
async fn check(app: tauri::AppHandle, manual: bool) -> Result<(), String> {
    check_with_policy(app, manual, false).await
}
async fn check_with_policy(
    app: tauri::AppHandle,
    manual: bool,
    manual_test: bool,
) -> Result<(), String> {
    check_for_trial(app, manual, manual_test, None).await
}
async fn check_for_trial(
    app: tauri::AppHandle,
    manual: bool,
    manual_test: bool,
    target: Option<String>,
) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if crate::privacy::onboarding_required(&app) {
        return Err("먼저 약관을 확인하세요".into());
    }
    let previous = {
        let state = app.state::<Updates>();
        let mut s = state.status.lock().map_err(err)?;
        if s.manual_test && !manual_test {
            return Ok(());
        }
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
        s.signature_verified = false;
        s.cache_ready = false;
        s.failure_stage = None;
        previous
    };
    let mut stage = "manifest";
    let result = async {
        crate::telemetry::checkpoint("updater.check");
        let updater = if let Some(target) = target.clone() {
            pair_builder(&app, target)?.build().map_err(err)?
        } else if manual_test {
            test_builder(&app)?.build().map_err(err)?
        } else {
            builder(&app)?
        };
        let Some(mut update) = updater.check().await.map_err(err)? else {
            if manual_test {
                return Err("공식 최신 피드에 현재 버전 이상의 안정 배포판이 없습니다".into());
            }
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
        write_cache(&app, &update, &bytes, manual_test, target.is_some())?;
        stage = "ready";
        ready(&app, update, bytes, manual_test)
    }
    .await;
    *app.state::<Updates>()
        .last_check_failure
        .lock()
        .map_err(err)? = result.is_err().then_some(stage);
    if result.is_err() {
        report_incident(&app, env!("CARGO_PKG_VERSION"), "check_failed");
        let updates = app.state::<Updates>();
        let mut current = updates.status.lock().map_err(err)?;
        check_failed(&mut current, previous);
        current.failure_stage = Some(stage.into());
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
        let _ = app.emit("updater-release", epoch);
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
    install_with_policy(app, automatic, false)
}
fn install_policy_allows(manual_test: bool, automatic: bool, explicit_test: bool) -> bool {
    if manual_test {
        explicit_test && !automatic
    } else {
        !explicit_test
    }
}
fn install_with_policy(
    app: &tauri::AppHandle,
    automatic: bool,
    explicit_test: bool,
) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if !crate::update_policy::idle_allows_install(automatic, idle_seconds(), IDLE_SECONDS) {
        return Ok(());
    }
    let install_version = {
        let state = app.state::<Updates>();
        let mut s = state.status.lock().map_err(err)?;
        if !install_policy_allows(s.manual_test, automatic, explicit_test) {
            return Ok(());
        }
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
    verification_event(
        app,
        if explicit_test {
            "test-install-start"
        } else {
            "normal-install-start"
        },
        Some(&install_version),
        Some(automatic),
    );
    let mut snapshot_written = false;
    let mut prepared_epoch = None;
    let result: Result<(), String> = (|| {
        if crate::privacy::onboarding_required(app) {
            return Err("먼저 약관을 확인하세요".into());
        }
        crate::telemetry::checkpoint("updater.prepare");
        let epoch = on_main(app, |app| {
            safe_native(&app)?;
            let labels: Vec<String> = crate::web::managed_windows(&app).keys().cloned().collect();
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
                return Err("창의 설치 준비 응답 시간이 초과됐습니다. 작업을 저장한 뒤 다시 시도하세요. 반복되면 짜잔을 수동으로 종료하고 다시 실행하세요.".into());
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
    verification_event(app, "initialize", None, None);
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
    set(app, "checking", "보관된 업데이트를 확인하고 있습니다");
    if trial_hold_path(app).exists() {
        if let Ok(mut current) = app.state::<Updates>().status.lock() {
            current.manual_test = true;
            current.roundtrip_active = true;
        }
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if restore_cache(&app).is_err() {
            let manual_test = status(&app).is_ok_and(|s| s.manual_test);
            set(
                &app,
                "error",
                if manual_test {
                    "보관된 시험 패키지를 검증하지 못했습니다. 수동 대기를 유지합니다. 개발자 메뉴에서 최신 배포판을 다시 준비하세요."
                } else {
                    "보관된 업데이트를 검증하지 못했습니다. 다시 다운로드합니다"
                },
            );
        }
        if status(&app).is_ok_and(|s| s.phase == "checking") {
            set(&app, "idle", "업데이트를 자동으로 확인합니다");
        }
        let mut last = None;
        std::thread::sleep(Duration::from_secs(15));
        loop {
            if !crate::privacy::onboarding_required(&app) {
                check_when_due(&mut last, Instant::now, || {
                    tauri::async_runtime::block_on(check(app.clone(), false))
                });
            }
            // Restoring verified bytes does not grant permission to interrupt input.
            let _ = install(&app, true);
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
    #[test]
    fn trial_pair_never_admits_arbitrary_past_versions_or_outside_source_versions() {
        let pair = TrialPair {
            id: "schema-v1".into(),
            previous: "0.3.12".into(),
            latest: "0.3.13".into(),
        };
        assert!(pair_allows(&pair, "0.3.13", "0.3.12"));
        assert!(pair_allows(&pair, "0.3.12", "0.3.13"));
        for (current, target) in [
            ("0.3.13", "0.3.11"),
            ("0.3.11", "0.3.12"),
            ("0.3.13", "0.3.12-beta.1"),
            ("0.3.14", "0.3.13"),
        ] {
            assert!(!pair_allows(&pair, current, target));
        }
        assert_eq!(
            trial_pair().is_some(),
            cfg!(feature = "verification-roundtrip")
        );
    }
    #[test]
    fn manual_cache_policy_survives_serialization_and_never_grants_automatic_install() {
        for (policy, manual) in [
            (None, false),
            (Some("automatic-v1"), false),
            (Some("manual-test-v1"), true),
        ] {
            let mut manifest = json!({"version":"9.0.0"});
            if let Some(policy) = policy {
                manifest["_jajan_install_policy"] = json!(policy);
            }
            let restored = serde_json::from_slice(&serde_json::to_vec(&manifest).unwrap()).unwrap();
            let actual = cache_manual_test(&restored).unwrap();
            assert_eq!(actual, manual);
            assert_eq!(install_policy_allows(actual, true, false), !manual);
            assert_eq!(install_policy_allows(actual, false, false), !manual);
            assert_eq!(install_policy_allows(actual, false, true), manual);
            assert!(!install_policy_allows(actual, true, true));
        }
        assert!(cache_manual_test(&json!({"_jajan_install_policy":"unknown"})).is_err());
        assert!(cache_manual_test(&json!({"_jajan_install_policy":null})).is_err());
    }
    #[test]
    fn test_comparator_accepts_real_equal_version_but_rejects_downgrades_and_prereleases() {
        let app = app();
        let updater = test_builder(app.handle())
            .unwrap()
            .target("darwin-aarch64")
            .build()
            .unwrap();
        let current = app.package_info().version.to_string();
        let update = updater.restore_update(manifest(&current)).unwrap().unwrap();
        assert_eq!(update.version, current);
        assert_eq!(update.current_version, current);
        for version in ["0.0.0", "99.0.0-beta.1"] {
            assert!(updater.restore_update(manifest(version)).unwrap().is_none());
        }
        assert!(updater
            .restore_update(manifest("99.0.0"))
            .unwrap()
            .is_some());
        assert!(updater
            .restore_update(json!({"version":current,"platforms":{}}))
            .is_err());
    }
    #[test]
    fn active_profile_checks_every_fifteen_minutes_in_production() {
        let fast = VERIFICATION && option_env!("JAJAN_VERIFICATION_STANDARD_TIMING").is_none();
        assert_eq!(CHECK_SECONDS, if fast { 60 } else { 15 * 60 });
        assert_eq!(IDLE_SECONDS, if fast { 30. } else { 300. });
        assert_eq!(RETRY_SECONDS, if fast { 5 } else { 15 });
    }

    #[test]
    fn first_check_and_retries_after_offline_and_failed_requests_use_completion_time() {
        use std::cell::Cell;
        let clock = Cell::new(Instant::now());
        let mut last = None;
        let interval = Duration::from_secs(CHECK_SECONDS);
        for result in [
            Err("offline".into()),
            Err("invalid response".into()),
            Ok(()),
        ] {
            let start = clock.get();
            let completed = start + Duration::from_secs(2);
            check_when_due(
                &mut last,
                || clock.get(),
                || {
                    clock.set(completed);
                    result
                },
            );
            assert_eq!(last, Some(completed));
            clock.set(completed + interval - Duration::from_millis(1));
            check_when_due(&mut last, || clock.get(), || panic!("premature check"));
            clock.set(completed + interval);
        }
    }

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
        if VERIFICATION && ENDPOINT != OFFICIAL_ENDPOINT {
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
            "Jajan_0.2.9_darwin-aarch64.app.tar.gz"
        );
        assert_eq!(
            package_name("0.2.9", "windows-x86_64"),
            "Jajan_0.2.9_x64-setup.exe"
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
