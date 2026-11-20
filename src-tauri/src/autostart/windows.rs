//! NSIS current-user desktop application. Never writes StartupApproved.
use super::OsStatus;
use std::{io, path::Path};
use winreg::{enums::*, RegKey};
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "Jajan";
#[derive(Debug, PartialEq, Eq)]
enum Change {
    Keep,
    Register,
    Unregister,
}
fn plan(existing: Option<&str>, expected: &str, enabled: bool) -> Result<Change, String> {
    match existing {
        Some(value) if value != expected => Err(
            "다른 경로의 자동 실행 항목을 변경하지 않았습니다. 기존 설치 앱에서 설정하세요.".into(),
        ),
        Some(_) if enabled => Ok(Change::Keep),
        Some(_) => Ok(Change::Unregister),
        None if enabled => Ok(Change::Register),
        None => Ok(Change::Keep),
    }
}
fn command(exe: &Path) -> Result<String, String> {
    let exe = exe.to_str().ok_or("자동 실행 경로를 읽지 못했습니다.")?;
    if exe.contains(['"', '\0']) {
        return Err("자동 실행 경로가 올바르지 않습니다.".into());
    }
    let command = format!("\"{exe}\" --login-startup");
    // Documented Run value limit (260 characters, including arguments).
    if command.encode_utf16().count() > 260 {
        return Err("자동 실행 경로가 너무 깁니다.".into());
    }
    Ok(command)
}
fn registered() -> Result<Option<String>, String> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let key = match root.open_subkey_with_flags(RUN, KEY_READ) {
        Ok(key) => key,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    match key.get_value(NAME) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
pub fn status() -> Result<(OsStatus, Option<String>), String> {
    let Some(value) = registered()? else {
        return Ok((OsStatus::Disabled, None));
    };
    if value != command(&std::env::current_exe().map_err(|e| e.to_string())?)? {
        return Ok((
            OsStatus::Unavailable,
            Some(
                "다른 경로의 Jajan 자동 실행 항목이 있습니다. 기존 설치 앱에서 설정하세요.".into(),
            ),
        ));
    }
    // Run existence is registration, not Windows Startup Apps approval. Its
    // undocumented StartupApproved binary layout is deliberately not inferred.
    Ok((OsStatus::RegisteredUnknown, Some("등록됨 · Windows 허용 여부 미확인. Windows 시작 앱에서 Jajan의 켜짐/꺼짐을 확인하세요.".into())))
}
pub fn set(enabled: bool) -> Result<(), String> {
    let expected = command(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    let change = plan(registered()?.as_deref(), &expected, enabled)?;
    if change == Change::Keep {
        return Ok(());
    } // Preserve OS disablement; never rewrite.
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = root.create_subkey(RUN).map_err(|e| e.to_string())?;
    if enabled {
        key.set_value(NAME, &expected).map_err(|e| e.to_string())
    } else {
        key.delete_value(NAME)
            .or_else(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|e| e.to_string())
    }
}
pub fn open_settings() -> Result<(), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            window: isize,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    let uri: Vec<u16> = OsStr::new("ms-settings:startupapps")
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: static Settings URI, NUL-terminated buffer live for the call.
    let result = unsafe {
        ShellExecuteW(
            0,
            std::ptr::null(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result > 32 {
        Ok(())
    } else {
        Err("Windows 시작 앱 설정을 열지 못했습니다.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn run_command_quotes_paths_and_only_uses_passive_launch() {
        assert_eq!(
            command(Path::new(r"C:\Users\한 글\Jajan\jajan.exe")).unwrap(),
            "\"C:\\Users\\한 글\\Jajan\\jajan.exe\" --login-startup"
        );
        assert!(command(Path::new("bad\"path")).is_err());
        assert!(command(Path::new(&"a".repeat(261))).is_err());
    }
    #[test]
    fn registration_removal_and_os_disabled_entry_are_independent() {
        assert_eq!(plan(None, "this-app", true).unwrap(), Change::Register);
        assert_eq!(
            plan(Some("this-app"), "this-app", false).unwrap(),
            Change::Unregister
        );
        assert_eq!(plan(None, "this-app", false).unwrap(), Change::Keep);
        // Run remains present when Windows disables a Startup App. An explicit
        // On (as well as a future update/restart) must not rewrite that entry.
        assert_eq!(
            plan(Some("this-app"), "this-app", true).unwrap(),
            Change::Keep
        );
        assert!(plan(Some("other-install"), "this-app", true).is_err());
        assert!(plan(Some("other-install"), "this-app", false).is_err());
    }
}
