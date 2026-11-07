//! Downloading never grants permission to exit. Native activity reservations and
//! an acknowledgement from every WebView precede the final installation gate.
use crate::update_policy::Safety;
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
const ENDPOINT: &str =
    "https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json";
const IDLE_SECONDS: f64 = 300.;
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
        && crate::instance::current().id.is_none()
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
    serde_json::json!({"enabled": enabled(), "status": status(app).ok()})
}
fn set(app: &tauri::AppHandle, phase: &str, message: &str) {
    if let Ok(mut s) = app.state::<Updates>().status.lock() {
        s.phase = phase.into();
        s.message = message.into();
    }
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
pub fn updater_ack(window: tauri::WebviewWindow, epoch: u64, safe: bool) -> Result<(), String> {
    window
        .state::<Updates>()
        .safety
        .lock()
        .map_err(err)?
        .acknowledge(window.label(), epoch, safe);
    Ok(())
}
fn builder(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    configured_builder(app)?.build().map_err(err)
}
fn configured_builder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri_plugin_updater::UpdaterBuilder, String> {
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
        format!("Agent Gibbon_{version}_x64-setup.exe")
    } else {
        format!("Agent Gibbon_{version}_{target}.app.tar.gz")
    }
}
fn validate_download(update: &Update) -> Result<(), String> {
    let u = &update.download_url;
    let filename = package_name(&update.version, platform()).replace(' ', "%20");
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
    Ok(())
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
    let result = async {
        let Some(mut update) = builder(&app)?.check().await.map_err(err)? else {
            set(&app, "idle", "최신 버전입니다");
            return Ok::<_, String>(());
        };
        validate_download(&update)?;
        update.timeout = Some(Duration::from_secs(300));
        {
            let state = app.state::<Updates>();
            let mut s = state.status.lock().map_err(err)?;
            s.version = Some(update.version.clone());
            s.phase = "downloading".into();
            s.message = "업데이트를 다운로드하고 있습니다".into();
        }
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
        verify_package(&update, &bytes)?;
        write_cache(&app, &update, &bytes)?;
        ready(&app, update, bytes)
    }
    .await;
    if result.is_err() {
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
    if crate::capture::update_blocked(app)
        || crate::stash::active(app)
        || crate::stash::chord_busy(app)
    {
        return Err("캡처 또는 창 전환이 끝나면 설치합니다".into());
    }
    Ok(())
}
fn release(app: &tauri::AppHandle) {
    if let Ok(mut s) = app.state::<Updates>().safety.lock() {
        s.release();
    }
    let _ = app.emit("updater-release", ());
}
fn install(app: &tauri::AppHandle, automatic: bool) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if automatic && idle_seconds().is_none_or(|s| s < IDLE_SECONDS) {
        return Ok(());
    }
    {
        let state = app.state::<Updates>();
        let mut s = state.status.lock().map_err(err)?;
        if s.phase != "ready" {
            return Ok(());
        }
        s.phase = "preparing".into();
        s.message = "작업 상태를 확인하고 있습니다".into();
    }
    let mut snapshot_written = false;
    let result: Result<(), String> = (|| {
        if crate::privacy::onboarding_required(app) {
            return Err("먼저 약관을 확인하세요".into());
        }
        let epoch = on_main(app, |app| {
            safe_native(&app)?;
            let labels: Vec<String> = app.webview_windows().keys().cloned().collect();
            let epoch = app
                .state::<Updates>()
                .safety
                .lock()
                .map_err(err)?
                .prepare(labels)?;
            app.emit("updater-prepare", epoch).map_err(err)?;
            Ok(epoch)
        })?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if app.state::<Updates>().safety.lock().map_err(err)?.refused() {
                return Err("편집·저장 중인 창이 있어 설치를 보류했습니다".into());
            }
            if app.state::<Updates>().safety.lock().map_err(err)?.ready() {
                break;
            }
            if Instant::now() >= deadline {
                return Err(
                    "편집·저장 중이거나 응답하지 않는 창이 있어 설치를 보류했습니다".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        on_main(app, move |app| {
            safe_native(&app)?;
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
        set(app, "installing", "업데이트를 설치하고 재시작합니다");
        let pending = app.state::<Updates>();
        let pending = pending.pending.lock().map_err(err)?;
        let (update, bytes) = pending.as_ref().ok_or("다운로드된 업데이트가 없습니다")?;
        verify_package(update, bytes)?;
        update.install(bytes).map_err(err)?;
        app.restart();
        #[allow(unreachable_code)]
        Ok(())
    })();
    if let Err(ref reason) = result {
        if snapshot_written {
            crate::update_restore::discard(app);
        }
        set(app, "ready", reason);
        release(app);
    }
    // A blocked installation is a retryable ready state, exposed to the UI.
    Ok(())
}
pub fn initialize(app: &tauri::AppHandle) {
    if !enabled() {
        return;
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
        let mut last = Instant::now() - Duration::from_secs(6 * 60 * 60);
        loop {
            std::thread::sleep(Duration::from_secs(15));
            if !crate::privacy::onboarding_required(&app)
                && last.elapsed() >= Duration::from_secs(6 * 60 * 60)
            {
                let _ = tauri::async_runtime::block_on(check(app.clone(), false));
                last = Instant::now();
            }
            let _ = install(&app, !resumed);
            resumed = false;
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
            json!({"pubkey":"", "requireSignedVersion":true}),
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
}
