//! Local, version-bound consent. Diagnostic choices always fail closed;
//! development access can skip the terms gate without granting consent.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Emitter, Manager};

const TERMS: &str = include_str!("../../docs/legal/terms.md");
const NOTICE: &str = include_str!("../../docs/legal/privacy.md");
const DEPLOYMENT: &str = include_str!("privacy/notice.json");

#[derive(Default, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Choice {
    format: u8,
    policy: String,
    terms: bool,
    sharing: bool,
    decided_at: u64,
}
impl Choice {
    fn current(&self) -> bool {
        self.format == 1 && self.policy == policy() && self.terms && self.decided_at > 0
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deployment {
    operator: Option<String>,
    privacy_contact: Option<String>,
    recipient: String,
    storage_region: Option<String>,
    retention_days: Option<u32>,
}
impl Deployment {
    fn ready(&self) -> bool {
        [&self.operator, &self.privacy_contact, &self.storage_region]
            .iter()
            .all(|v| v.as_ref().is_some_and(|s| !s.trim().is_empty()))
            && !self.recipient.trim().is_empty()
            && self.retention_days.is_some_and(|d| d > 0)
    }
}
pub fn policy() -> String {
    format!(
        "{:x}",
        Sha256::digest(format!("{TERMS}\n{NOTICE}\n{DEPLOYMENT}"))
    )
}
pub fn notice_ready() -> bool {
    serde_json::from_str::<Deployment>(DEPLOYMENT).is_ok_and(|d| d.ready())
}
#[derive(Default)]
pub struct Privacy(Mutex<Local>, Mutex<()>);
#[derive(Default)]
struct Local {
    path: Option<PathBuf>,
    choice: Choice,
    load_failed: bool,
}
fn read(path: &Path) -> (Choice, bool) {
    match fs::File::open(path).and_then(|f| {
        let mut bytes = Vec::new();
        f.take(8193).read_to_end(&mut bytes)?;
        Ok(bytes)
    }) {
        Ok(bytes) if bytes.len() > 8192 => (Choice::default(), true),
        Ok(bytes) => match serde_json::from_slice::<Choice>(&bytes) {
            Ok(c) if c.current() => (c, false),
            Ok(_) => (Choice::default(), false),
            Err(_) => (Choice::default(), true),
        },
        Err(e) => (Choice::default(), e.kind() != std::io::ErrorKind::NotFound),
    }
}
pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|_| "Cannot locate local privacy settings")?
        .join("privacy")
        .join(
            crate::instance::current()
                .id
                .as_deref()
                .unwrap_or("installed"),
        )
        .join("consent.json");
    let (choice, load_failed) = read(&path);
    *app.state::<Privacy>()
        .0
        .lock()
        .map_err(|_| "Privacy settings unavailable")? = Local {
        path: Some(path),
        choice,
        load_failed,
    };
    crate::telemetry::refresh(app);
    Ok(())
}
pub fn accepted(app: &tauri::AppHandle) -> bool {
    app.try_state::<Privacy>()
        .is_some_and(|p| p.0.lock().is_ok_and(|s| s.choice.current()))
}
fn skip_terms(
    debug: bool,
    development_instance: bool,
    require_terms: Option<&std::ffi::OsStr>,
) -> bool {
    debug && development_instance && require_terms != Some(std::ffi::OsStr::new("1"))
}
pub fn terms_skipped() -> bool {
    skip_terms(
        cfg!(debug_assertions),
        crate::instance::current().id.is_some(),
        std::env::var_os("AGENT_GIBBON_REQUIRE_TERMS").as_deref(),
    )
}
pub fn onboarding_required(app: &tauri::AppHandle) -> bool {
    !terms_skipped() && !accepted(app)
}
pub fn sharing(app: &tauri::AppHandle) -> bool {
    notice_ready()
        && app.try_state::<Privacy>().is_some_and(|p| {
            p.0.lock()
                .is_ok_and(|s| s.choice.current() && s.choice.sharing)
        })
}
pub fn status(app: &tauri::AppHandle) -> Value {
    let p = app.state::<Privacy>();
    let state = p.0.lock().unwrap_or_else(|e| e.into_inner());
    json!({"terms_accepted":state.choice.current(),"terms_skipped":terms_skipped(),"allowed":state.choice.current() && state.choice.sharing && notice_ready(),
        "sharing":state.choice.current() && state.choice.sharing && notice_ready() && crate::telemetry::enabled(),
        "notice_ready":notice_ready(),"policy":policy(),"load_failed":state.load_failed,
        "terms_html":crate::model::render(TERMS),"notice_html":crate::model::render(NOTICE)})
}
fn save(path: &Path, choice: &Choice) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid privacy settings path")?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(parent)
        .map_err(|_| "Could not create privacy settings")?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "Could not save privacy settings")?;
    serde_json::to_writer(&mut file, choice).map_err(|_| "Could not encode privacy settings")?;
    file.flush()
        .map_err(|_| "Could not save privacy settings")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Could not save privacy settings")?;
    file.persist(path)
        .map_err(|_| "Could not replace privacy settings")?;
    Ok(())
}
fn manager(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != crate::manager::LABEL {
        return Err("Privacy choices are available in the manager only".into());
    }
    Ok(())
}
#[tauri::command]
pub fn privacy_status(window: tauri::WebviewWindow) -> Result<Value, String> {
    manager(&window)?;
    Ok(status(window.app_handle()))
}
#[tauri::command]
pub async fn privacy_save(
    window: tauri::WebviewWindow,
    terms_accepted: bool,
    share_diagnostics: bool,
) -> Result<Value, String> {
    manager(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Disable first, even if the subsequent disk write fails. Reconfiguration
        // and persistence are serialized by this local settings mutex.
        let state = app.state::<Privacy>();
        let _transition = state.1.lock().map_err(|_| "Privacy settings unavailable")?;
        let mut local = state.0.lock().map_err(|_| "Privacy settings unavailable")?;
        crate::telemetry::revoke(&app);
        if !terms_accepted {
            return Err("Accept the Terms of Use to continue, or exit the app".into());
        }
        if share_diagnostics && !notice_ready() {
            return Err(
                "Sharing is unavailable until operator, region and retention details are published"
                    .into(),
            );
        }
        let first_acceptance = !local.choice.current();
        let next = Choice {
            format: 1,
            policy: policy(),
            terms: true,
            sharing: share_diagnostics,
            decided_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        save(
            local
                .path
                .as_deref()
                .ok_or("Privacy settings not initialized")?,
            &next,
        )?;
        local.choice = next;
        local.load_failed = false;
        drop(local);
        crate::telemetry::refresh(&app);
        if first_acceptance {
            crate::skills::synchronize(app.clone());
            let target = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = crate::apply_direct(
                    &target,
                    crate::Request {
                        external: None,
                        action: crate::Action::Open,
                        id: None,
                        title: None,
                        color: None,
                        status: None,
                        markdown: None,
                        change_mode: None,
                        highlight: None,
                        source_path: None,
                    },
                );
            });
        }
        let _ = app.emit("privacy-changed", crate::telemetry::status());
        Ok(status(&app))
    })
    .await
    .map_err(|_| "Could not update privacy settings".to_string())?
}
#[tauri::command]
pub fn privacy_quit(window: tauri::WebviewWindow) -> Result<(), String> {
    manager(&window)?;
    window.app_handle().exit(0);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn choice(sharing: bool) -> Choice {
        Choice {
            format: 1,
            policy: policy(),
            terms: true,
            sharing,
            decided_at: 1,
        }
    }
    #[test]
    fn absent_corrupt_old_and_unaccepted_records_fail_closed() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("consent.json");
        assert!(!read(&p).0.current());
        fs::write(&p, b"broken").unwrap();
        assert!(read(&p).1);
        assert!(!read(&p).0.current());
        for mut c in [choice(true), choice(false)] {
            c.policy = "old".into();
            save(&p, &c).unwrap();
            assert!(!read(&p).0.current());
        }
        let mut c = choice(true);
        c.terms = false;
        save(&p, &c).unwrap();
        assert!(!read(&p).0.current());
    }
    #[test]
    fn local_choice_roundtrips_and_declining_is_a_valid_choice() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("private/consent.json");
        save(&p, &choice(false)).unwrap();
        let (c, error) = read(&p);
        assert!(!error && c.current() && !c.sharing);
        assert_eq!(c.decided_at, 1);
    }
    #[test]
    fn published_notice_enables_choice_without_granting_consent() {
        assert!(notice_ready());
        assert!(!Choice::default().current());
        assert!(!Choice::default().sharing);
    }
    #[test]
    fn terms_skip_is_limited_to_debug_development_instances_and_can_be_disabled() {
        assert!(skip_terms(true, true, None));
        assert!(!skip_terms(true, true, Some(std::ffi::OsStr::new("1"))));
        for (debug, development) in [(false, false), (false, true), (true, false)] {
            assert!(!skip_terms(debug, development, None));
            assert!(!skip_terms(
                debug,
                development,
                Some(std::ffi::OsStr::new("0"))
            ));
        }
        // Skipping a UI gate does not create a current choice or authorize sharing.
        let missing = Choice::default();
        assert!(!missing.current() && !missing.sharing);
    }
    #[test]
    fn unfinished_notice_cannot_enable_sharing() {
        let d: Deployment = serde_json::from_str(r#"{"operator":null,"privacy_contact":null,"recipient":"Sentry","storage_region":null,"retention_days":null}"#).unwrap();
        assert!(!d.ready());
        let complete: Deployment = serde_json::from_str(r#"{"operator":"Example","privacy_contact":"privacy@example.invalid","recipient":"Sentry","storage_region":"Example region","retention_days":30}"#).unwrap();
        assert!(complete.ready());
    }
}
