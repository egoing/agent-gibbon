//! The saved preference and the resolved display language are separate.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, RwLock},
};
use tauri::Manager;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    #[default]
    System,
    Ko,
    En,
    Ja,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Ko,
    En,
    Ja,
}

fn resolve(choice: Choice, preferred: impl IntoIterator<Item = impl AsRef<str>>) -> Locale {
    match choice {
        Choice::Ko => Locale::Ko,
        Choice::En => Locale::En,
        Choice::Ja => Locale::Ja,
        Choice::System => preferred
            .into_iter()
            .find_map(|tag| {
                match tag
                    .as_ref()
                    .split(['-', '_'])
                    .next()?
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "en" => Some(Locale::En),
                    "ko" => Some(Locale::Ko),
                    "ja" => Some(Locale::Ja),
                    _ => None,
                }
            })
            .unwrap_or(Locale::En),
    }
}

#[derive(Clone, Copy, Serialize)]
pub struct Snapshot {
    choice: Choice,
    locale: Locale,
    revision: u64,
}
struct Preferences {
    path: PathBuf,
    current: Mutex<Snapshot>,
}
// Source-language defaults are also used by standalone, non-UI callers.
// The app plugin resolves the real preference before creating any windows.
static DISPLAY: RwLock<Locale> = RwLock::new(Locale::Ko);

pub fn locale() -> Locale {
    *DISPLAY.read().unwrap_or_else(|e| e.into_inner())
}
fn load(path: &Path) -> Result<Choice, String> {
    Ok(match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Choice::System,
        Err(e) => return Err(e.to_string()),
    })
}

fn save(path: &Path, choice: Choice) -> Result<(), String> {
    let dir = path.parent().ok_or("Missing settings directory")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut temp, &choice).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("appearance")
        .join(
            crate::instance::current()
                .id
                .as_deref()
                .unwrap_or("installed"),
        )
        .join("language.json");
    let choice = load(&path)?;
    let locale = resolve(choice, sys_locale::get_locales());
    *DISPLAY.write().unwrap_or_else(|e| e.into_inner()) = locale;
    app.manage(Preferences {
        path,
        current: Mutex::new(Snapshot {
            choice,
            locale,
            revision: 0,
        }),
    });
    Ok(())
}

#[derive(Deserialize)]
struct Message {
    ko: String,
    en: String,
    ja: String,
}
static MESSAGES: LazyLock<HashMap<String, Message>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../ui/messages.json")).expect("valid UI message catalog")
});

pub fn tr(key: &str) -> &str {
    match MESSAGES.get(key) {
        Some(message) => match locale() {
            Locale::Ko => &message.ko,
            Locale::En => &message.en,
            Locale::Ja => &message.ja,
        },
        None => key,
    }
}

// The native permission panel copies the UTF-8 bytes during this call. Known
// translations live in MESSAGES for the process lifetime.
#[cfg(target_os = "macos")]
#[no_mangle]
unsafe extern "C" fn jajan_ui_label(key: *const std::ffi::c_char, size: *mut usize) -> *const u8 {
    let key = std::ffi::CStr::from_ptr(key).to_str().unwrap_or("");
    let text = tr(key);
    *size = text.len();
    text.as_ptr()
}

#[tauri::command]
pub fn language_status(app: tauri::AppHandle) -> Result<Snapshot, String> {
    app.state::<Preferences>()
        .current
        .lock()
        .map(|s| *s)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct Applied {
    #[serde(flatten)]
    snapshot: Snapshot,
    refresh_failed: bool,
}

#[tauri::command]
pub fn language_save(window: tauri::WebviewWindow, choice: Choice) -> Result<Applied, String> {
    if window.label() != crate::manager::LABEL {
        return Err("Manager required".into());
    }
    let app = window.app_handle();
    let prefs = app.state::<Preferences>();
    let snapshot = {
        let mut current = prefs.current.lock().map_err(|e| e.to_string())?;
        save(&prefs.path, choice)?;
        let locale = resolve(choice, sys_locale::get_locales());
        *current = Snapshot {
            choice,
            locale,
            revision: current.revision + 1,
        };
        *DISPLAY.write().unwrap_or_else(|e| e.into_inner()) = locale;
        *current
    };
    let script = format!(
        "window.jajanLanguage?.apply({})",
        serde_json::to_string(&snapshot).map_err(|e| e.to_string())?
    );
    // A closing view may disappear between enumeration and delivery. Other
    // windows must still receive the committed preference.
    let mut refresh_failed = false;
    for (label, view) in app.webviews() {
        if !label.starts_with("webpage-") {
            refresh_failed |= view.eval(&script).is_err() && app.get_webview(&label).is_some();
        }
    }
    for (label, window) in app.webview_windows() {
        let instance = crate::instance::current().title();
        let title = match label.as_str() {
            crate::manager::LABEL => crate::manager::title(),
            "permission-guide" => tr("Jajan 권한 설정").into(),
            "relation-settings" => tr("메모 연결 설정").into(),
            "layout-undo" => tr("정렬 취소하기").into(),
            "memo-backdrop" => format!("{instance} — {}", tr("메모 배경")),
            "memo-overview" => format!("{instance} — {}", tr("메모 모아보기")),
            "direction-guide" => format!("{instance} — {}", tr("방향 안내")),
            "updater-notice" => tr("Jajan · 업데이트").into(),
            _ if label.starts_with("webview-") => {
                crate::web::host_title(&label["webview-".len()..])
            }
            _ if label.starts_with("capture-") => {
                let old = window.title().unwrap_or_default();
                let Some(key) = [
                    "영역 캡처",
                    "빈 메모 영역 선택",
                    "터미널 영역 선택",
                    "웹 창 영역 선택",
                ]
                .into_iter()
                .find(|key| {
                    MESSAGES
                        .get(*key)
                        .is_some_and(|m| m.ko == old || m.en == old || m.ja == old)
                }) else {
                    continue;
                };
                tr(key).into()
            }
            _ => continue,
        };
        refresh_failed |=
            window.set_title(&title).is_err() && app.get_webview_window(&label).is_some();
    }
    #[cfg(target_os = "macos")]
    {
        refresh_failed |= app
            .run_on_main_thread(|| unsafe {
                extern "C" {
                    fn jajan_refresh_permission_language();
                }
                jajan_refresh_permission_language();
            })
            .is_err();
        refresh_failed |= crate::language_menu::install(app).is_err();
    }
    refresh_failed |= crate::tray::refresh(app).is_err();
    Ok(Applied {
        snapshot,
        refresh_failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_preference_is_independent_of_resolved_locale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dev-a/language.json");
        assert_eq!(load(&path).unwrap(), Choice::System);
        for choice in [Choice::Ja, Choice::Ko, Choice::En, Choice::System] {
            save(&path, choice).unwrap();
            assert_eq!(load(&path).unwrap(), choice);
            assert_eq!(
                load(&dir.path().join("dev-b/language.json")).unwrap(),
                Choice::System
            );
        }
        fs::write(&path, b"invalid").unwrap();
        assert_eq!(load(&path).unwrap(), Choice::System);
        assert!(save(&path.join("blocked"), Choice::Ja).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
    }

    #[test]
    fn matches_in_user_order() {
        assert_eq!(
            resolve(Choice::System, ["fr", "JA-jpan-JP", "ko"]),
            Locale::Ja
        );
        assert_eq!(resolve(Choice::System, ["ko_KR", "en-US"]), Locale::Ko);
        assert_eq!(resolve(Choice::System, ["fr"]), Locale::En);
        assert_eq!(resolve(Choice::En, ["ja"]), Locale::En);
    }
}
