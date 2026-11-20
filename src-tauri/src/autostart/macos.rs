use super::OsStatus;
extern "C" {
    fn jajan_autostart_status() -> i32;
    fn jajan_autostart_set(enabled: bool) -> bool;
    fn jajan_autostart_settings();
    fn jajan_login_launch() -> bool;
    fn jajan_login_event() -> bool;
}
pub fn status() -> Result<(OsStatus, Option<String>), String> {
    // SAFETY: main-thread manager command; native bridge checks OS availability.
    let state = match unsafe { jajan_autostart_status() } {
        0 => OsStatus::Disabled,
        1 => OsStatus::Enabled,
        2 => OsStatus::RequiresApproval,
        _ => OsStatus::Unavailable,
    };
    let reason = (state == OsStatus::Unavailable)
        .then(|| "macOS 13 이상에서 설치한 앱 번들로 실행해야 합니다.".into());
    Ok((state, reason))
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
