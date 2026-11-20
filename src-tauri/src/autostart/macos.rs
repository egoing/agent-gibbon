use super::OsStatus;
extern "C" {
    fn jajan_autostart_status() -> i32;
    fn jajan_autostart_environment() -> i32;
    fn jajan_autostart_set(enabled: bool) -> bool;
    fn jajan_autostart_settings();
    fn jajan_login_launch() -> bool;
    fn jajan_login_event() -> bool;
}
fn interpret(raw: i32, environment: i32) -> (OsStatus, Option<String>) {
    if environment != 0 {
        return (OsStatus::Unavailable, Some(match environment {
            -1 => "로그인 자동 실행은 macOS 13 이상에서 지원합니다.",
            -2 => "설치 앱 번들을 확인하지 못했습니다. 공식 설치 파일에서 Jajan을 응용 프로그램 폴더에 다시 설치한 뒤 Finder에서 여세요.",
            -3 => "macOS가 앱을 임시 격리 경로에서 실행했습니다. 앱을 종료하고 Finder의 응용 프로그램 폴더에서 Jajan을 여세요. 반복되면 공식 설치 파일을 다시 받아 Finder로 다시 설치하세요. 보안 속성을 제거할 필요는 없습니다.",
            _ => "자동 실행 환경을 확인하지 못했습니다.",
        }.into()));
    }
    match raw {
        0 => (OsStatus::Disabled, None),
        1 => (OsStatus::Enabled, None),
        2 => (OsStatus::RequiresApproval, None),
        3 => (OsStatus::Unavailable, Some("macOS가 이 앱의 로그인 서비스를 찾지 못했습니다. 앱을 종료하고 응용 프로그램 폴더에서 다시 여세요. 계속되면 공식 설치 파일로 다시 설치하거나 시스템 설정의 일반 → 로그인 항목 및 확장 프로그램 → 로그인 시 열기에서 Jajan을 직접 추가하세요. 앱 선택과 OS 등록 상태는 다를 수 있습니다.".into())),
        _ => (OsStatus::Unavailable, Some("macOS 자동 실행 상태를 확인하지 못했습니다.".into())),
    }
}
pub fn status() -> Result<(OsStatus, Option<String>), String> {
    // Read only: never attempt registration to repair status.
    Ok(interpret(unsafe { jajan_autostart_status() }, unsafe {
        jajan_autostart_environment()
    }))
}
pub fn diagnostics() -> serde_json::Value {
    let raw = unsafe { jajan_autostart_status() };
    let environment = unsafe { jajan_autostart_environment() };
    serde_json::json!({
        "service_status": raw,
        "service_status_name": match raw { 0 => "not_registered", 1 => "enabled", 2 => "requires_approval", 3 => "not_found", _ => "unavailable" },
        "environment": match environment { 0 => "supported", -1 => "unsupported_os", -2 => "invalid_bundle", -3 => "app_translocation", _ => "unknown" },
    })
}
pub fn set(enabled: bool) -> Result<(), String> {
    // SAFETY: same main-thread boundary as status(). No borrowed pointers.
    if unsafe { jajan_autostart_set(enabled) } {
        Ok(())
    } else {
        Err("로그인 항목을 변경하지 못했습니다. 시스템 설정에서 현재 상태를 확인하세요.".into())
    }
}
pub fn open_settings() -> Result<(), String> {
    unsafe { jajan_autostart_settings() };
    Ok(())
}
pub fn login_launch() -> bool {
    // SAFETY: called on the application's main thread during launch/events.
    unsafe { jajan_login_launch() }
}
pub fn current_login_event() -> bool {
    // SAFETY: RunEvent callbacks are on AppKit's main thread. Unlike the cached
    // launch mode, this does not suppress a later explicit Dock/app reopen.
    unsafe { jajan_login_event() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_failure_and_environment_are_distinct() {
        assert_eq!(interpret(0, 0), (OsStatus::Disabled, None));
        assert_eq!(interpret(1, 0), (OsStatus::Enabled, None));
        assert_eq!(interpret(2, 0), (OsStatus::RequiresApproval, None));
        for (raw, environment, message) in [
            (3, 0, "찾지 못했습니다"),
            (3, -3, "임시 격리"),
            (-1, -1, "macOS 13"),
            (-2, -2, "앱 번들"),
        ] {
            let (status, reason) = interpret(raw, environment);
            assert_eq!(status, OsStatus::Unavailable);
            assert!(reason.unwrap().contains(message));
        }
    }
}
