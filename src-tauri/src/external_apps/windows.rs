use super::*;
use fs2::FileExt;
use std::os::windows::process::CommandExt;
use std::{
    fs::{File, OpenOptions},
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Placement {
    length: u32,
    flags: u32,
    show: u32,
    min: Point,
    max: Point,
    normal: Rect,
}
#[link(name = "user32")]
extern "system" {
    fn EnumWindows(callback: unsafe extern "system" fn(isize, isize) -> i32, param: isize) -> i32;
    fn IsWindow(hwnd: isize) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn IsIconic(hwnd: isize) -> i32;
    fn IsZoomed(hwnd: isize) -> i32;
    fn GetWindow(hwnd: isize, cmd: u32) -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn GetWindowTextW(hwnd: isize, text: *mut u16, max: i32) -> i32;
    fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
    fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn GetWindowPlacement(hwnd: isize, p: *mut Placement) -> i32;
    fn SetWindowPlacement(hwnd: isize, p: *const Placement) -> i32;
    fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn CloseHandle(handle: isize) -> i32;
    fn QueryFullProcessImageNameW(handle: isize, flags: u32, name: *mut u16, len: *mut u32) -> i32;
    fn GetProcessTimes(
        handle: isize,
        created: *mut u64,
        exit: *mut u64,
        kernel: *mut u64,
        user: *mut u64,
    ) -> i32;
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Identity {
    hwnd: isize,
    pid: u32,
    created: u64,
    exe: String,
}
#[derive(Clone, Debug, Serialize)]
struct Window {
    window: String,
    title: String,
    identity: Identity,
    rect: [i32; 4],
    topmost: bool,
}
#[derive(Clone, Deserialize, Serialize)]
struct App {
    id: String,
    label: String,
    exe: String,
    arguments: String,
}
struct Session {
    original: Window,
    placement: Placement,
    _lease: File,
    journal: std::path::PathBuf,
}
#[derive(Default)]
pub(super) struct State {
    root: std::path::PathBuf,
    initialized: bool,
    apps: Vec<App>,
    inventory: BTreeMap<isize, Identity>,
    sessions: BTreeMap<String, Session>,
    launches: BTreeMap<String, Value>,
    next: u64,
    shutting_down: bool,
}
fn native_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
fn identity(hwnd: isize) -> Result<Identity, String> {
    let mut pid = 0;
    if unsafe { IsWindow(hwnd) } == 0
        || unsafe { GetWindowThreadProcessId(hwnd, &mut pid) } == 0
        || pid == std::process::id()
    {
        return Err("선택한 외부 창이 종료되었거나 Gibbon 창입니다".into());
    }
    let handle = unsafe { OpenProcess(0x1000, 0, pid) };
    if handle == 0 {
        return Err(native_error("프로세스 확인 실패; 권한 경계를 확인하세요"));
    }
    let mut name = [0u16; 32768];
    let mut n = name.len() as u32;
    let (mut created, mut exit, mut kernel, mut user) = (0u64, 0u64, 0u64, 0u64);
    let valid = unsafe { QueryFullProcessImageNameW(handle, 0, name.as_mut_ptr(), &mut n) } != 0
        && unsafe { GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user) } != 0;
    unsafe { CloseHandle(handle) };
    if !valid {
        return Err(native_error("외부 창 출처 확인 실패"));
    }
    Ok(Identity {
        hwnd,
        pid,
        created,
        exe: String::from_utf16_lossy(&name[..n as usize]),
    })
}
fn snap(hwnd: isize) -> Result<Window, String> {
    let identity = identity(hwnd)?;
    if unsafe { IsWindowVisible(hwnd) } == 0 || unsafe { GetWindow(hwnd, 4) } != 0 {
        return Err("숨긴 창과 소유된 대화상자는 선택할 수 없습니다".into());
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, -16) };
    let ex = unsafe { GetWindowLongPtrW(hwnd, -20) };
    if style & 0x08000000 != 0 || ex & (0x80 | 0x08000000 | 0x20) != 0 {
        return Err("일반 입력 가능한 최상위 창만 선택할 수 있습니다".into());
    }
    let mut title = [0u16; 1024];
    let n = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    if n <= 0 {
        return Err("제목 없는 창은 선택할 수 없습니다".into());
    }
    let mut rect = Rect::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err(native_error("창 크기 조회 실패"));
    }
    Ok(Window {
        window: hwnd.to_string(),
        title: String::from_utf16_lossy(&title[..n as usize]),
        identity,
        rect: [
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ],
        topmost: ex & 8 != 0,
    })
}
unsafe extern "system" fn collect(hwnd: isize, data: isize) -> i32 {
    // EnumWindows calls synchronously while this Vec remains exclusively borrowed.
    if let Ok(w) = snap(hwnd) {
        unsafe { (&mut *(data as *mut Vec<Window>)).push(w) };
    }
    1
}
fn windows() -> Result<Vec<Window>, String> {
    let mut rows = Vec::new();
    if unsafe { EnumWindows(collect, &mut rows as *mut Vec<Window> as isize) } == 0 {
        return Err(native_error("창 목록 조회 실패"));
    }
    Ok(rows)
}
fn same(id: &Identity) -> bool {
    identity(id.hwnd).is_ok_and(|v| v == *id)
}
fn placement(hwnd: isize) -> Result<Placement, String> {
    let mut p = Placement {
        length: std::mem::size_of::<Placement>() as u32,
        ..Placement::default()
    };
    if unsafe { GetWindowPlacement(hwnd, &mut p) } == 0 {
        return Err(native_error("원래 창 상태 저장 실패"));
    }
    Ok(p)
}
fn forget_journal(s: &Session) -> Result<(), String> {
    match std::fs::remove_file(&s.journal) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
fn restore(s: &Session) -> Result<(), String> {
    if !same(&s.original.identity) {
        return forget_journal(s);
    } // Never touch a recycled HWND/PID.
    let h = s.original.identity.hwnd;
    if unsafe {
        SetWindowPos(
            h,
            if s.original.topmost { -1 } else { -2 },
            0,
            0,
            0,
            0,
            0x4000 | 0x10 | 0x200 | 1 | 2,
        )
    } == 0
    {
        return Err(native_error("고정 해제 실패"));
    }
    // Supported sessions start normal and visible; SW_SHOWNOACTIVATE preserves focus.
    let mut p = s.placement;
    p.show = 4;
    if unsafe { SetWindowPlacement(h, &p) } == 0 {
        return Err(native_error("창 위치 복원 실패"));
    }
    let deadline = Instant::now() + Duration::from_millis(900);
    loop {
        if let Ok(w) = snap(h) {
            if w.rect == s.original.rect && w.topmost == s.original.topmost {
                return forget_journal(s);
            }
        }
        if Instant::now() >= deadline {
            return Err("원래 창 상태 복원을 확인하지 못했습니다. 해제를 다시 시도하세요".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
fn attach(state: &mut State, id: Identity, region: [i32; 4]) -> Result<Value, String> {
    if state.shutting_down {
        return Err("앱 종료 중입니다".into());
    }
    if !same(&id) {
        return Err("선택 이후 창 출처가 변경되었습니다. 목록을 새로 고치세요".into());
    }
    let original = snap(id.hwnd)?;
    if unsafe { IsIconic(id.hwnd) } != 0 || unsafe { IsZoomed(id.hwnd) } != 0 {
        return Err("최소화·최대화 창은 보통 크기로 복원한 뒤 선택하세요".into());
    }
    if state.sessions.values().any(|s| s.original.identity == id) {
        return Err("이미 플로팅 중인 창입니다".into());
    }
    if state.sessions.len() >= 32 {
        return Err("플로팅 창은 인스턴스당 최대 32개입니다".into());
    }
    let lease = lease(&id)?;
    let saved = Saved {
        identity: id.clone(),
        rect: original.rect,
        topmost: original.topmost,
        placement: placement(id.hwnd)?,
    };
    std::fs::create_dir_all(&state.root).map_err(err)?;
    let journal = state
        .root
        .join(format!("{}-{}-{}.json", id.pid, id.created, id.hwnd));
    let mut temp = tempfile::NamedTempFile::new_in(&state.root).map_err(err)?;
    use std::io::Write;
    temp.write_all(&serde_json::to_vec(&saved).map_err(err)?)
        .map_err(err)?;
    temp.as_file().sync_all().map_err(err)?;
    temp.persist_noclobber(&journal).map_err(err)?;
    let session = Session {
        original,
        placement: saved.placement,
        _lease: lease,
        journal,
    };
    let [x, y, w, h] = region;
    let result = if unsafe { SetWindowPos(id.hwnd, -1, x, y, w, h, 0x4000 | 0x10 | 0x200) } == 0 {
        Err(native_error("외부 창 배치 실패"))
    } else {
        let deadline = Instant::now() + Duration::from_millis(900);
        loop {
            if !same(&id) {
                break Err("배치 중 창이 종료되었습니다".into());
            }
            if let Ok(observed) = snap(id.hwnd) {
                if observed.rect == region && observed.topmost {
                    break Ok(observed);
                }
            }
            if Instant::now() >= deadline {
                break Err(
                    "앱의 최소 크기·권한·응답 제한으로 요청 영역을 적용하지 못했습니다".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    match result {
        Ok(observed) => {
            state.next += 1;
            let key = format!("floating-{}-{}", std::process::id(), state.next);
            let value = json!({"session":key,"window":observed,"original":session.original,"state":"attached"});
            state.sessions.insert(key, session);
            Ok(value)
        }
        Err(e) => match restore(&session) {
            Ok(()) => Err(e),
            Err(rollback) => {
                state.next += 1;
                let key = format!("floating-{}-{}", std::process::id(), state.next);
                state.sessions.insert(key.clone(), session);
                Err(format!("{e}; {rollback}; 복원 재시도 ID: {key}"))
            }
        },
    }
}
fn catalog() -> Result<Vec<App>, String> {
    // Read classic desktop Start Menu shortcuts only. Resolve, never execute script targets.
    const SCRIPT: &str = r#"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $ws=New-Object -ComObject WScript.Shell; $roots=@([Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('CommonPrograms')); $rows=@(Get-ChildItem -LiteralPath $roots -Filter *.lnk -Recurse -ErrorAction SilentlyContinue | Select-Object -First 500 | ForEach-Object { $l=$ws.CreateShortcut($_.FullName); if($l.TargetPath -match '(?i)\.exe$' -and (Test-Path -LiteralPath $l.TargetPath -PathType Leaf)) { [pscustomobject]@{id=$_.FullName;label=$_.BaseName;exe=$l.TargetPath;arguments=$l.Arguments} } }); ConvertTo-Json -InputObject $rows -Compress"#;
    let output = tempfile::tempfile().map_err(err)?;
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(output.try_clone().map_err(err)?)
        .stderr(Stdio::null())
        .spawn()
        .map_err(err)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().map_err(err)? {
            if !status.success() {
                return Err("설치 앱 탐색 실패. 열린 창 목록에서 선택하세요".into());
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("설치 앱 탐색 시간 초과. 열린 창 목록에서 선택하세요".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    use std::io::{Seek, SeekFrom};
    let mut output = output;
    output.seek(SeekFrom::Start(0)).map_err(err)?;
    let mut bytes = String::new();
    output
        .take(2 * 1024 * 1024)
        .read_to_string(&mut bytes)
        .map_err(err)?;
    let mut apps: Vec<App> =
        serde_json::from_str(bytes.trim_start_matches('\u{feff}').trim()).map_err(err)?;
    apps.sort_by_key(|a| a.label.to_lowercase());
    apps.dedup_by(|a, b| a.exe.eq_ignore_ascii_case(&b.exe) && a.arguments == b.arguments);
    Ok(apps)
}
pub(super) fn execute(app: &tauri::AppHandle, r: &ExternalRequest) -> Result<Value, String> {
    initialize(app)?;
    let store = app.state::<ExternalApps>();
    let result = match r.operation.as_str() {
        "list" => {
            let apps = catalog()?;
            let public: Vec<Value> = apps
                .iter()
                .map(|a| json!({"id":a.id,"label":a.label}))
                .collect();
            store.inner.lock().map_err(err)?.apps = apps;
            json!({"apps":public,"limit":"Start Menu의 classic .exe 바로가기; Store 앱은 열린 창 목록 사용"})
        }
        "windows" => {
            let rows = windows()?;
            store.inner.lock().map_err(err)?.inventory = rows
                .iter()
                .map(|w| (w.identity.hwnd, w.identity.clone()))
                .collect();
            json!({"windows":rows})
        }
        "sessions" => {
            let s = store.inner.lock().map_err(err)?;
            json!({"sessions":s.sessions.iter().map(|(id,s)|json!({"session":id,"window":s.original,"alive":same(&s.original.identity)})).collect::<Vec<_>>(),"launches":s.launches})
        }
        "attach" => {
            let hwnd = r.window.as_ref().unwrap().parse::<isize>().map_err(err)?;
            let mut s = store.inner.lock().map_err(err)?;
            let identity = s
                .inventory
                .get(&hwnd)
                .cloned()
                .ok_or("먼저 열린 창 목록을 조회하고 대상을 명시적으로 선택하세요")?;
            attach(&mut s, identity, r.region.unwrap())?
        }
        "release" => {
            let mut s = store.inner.lock().map_err(err)?;
            let key = r.session.as_ref().unwrap();
            let session = s.sessions.get(key).ok_or("플로팅 세션이 없습니다")?;
            restore(session)?;
            s.sessions.remove(key);
            json!({"released":key})
        }
        "cancel" => {
            let mut s = store.inner.lock().map_err(err)?;
            let key = r.session.as_ref().unwrap();
            let op = s
                .launches
                .get(key)
                .cloned()
                .ok_or("실행을 찾을 수 없습니다")?;
            // Cancellation linearizes under the same lock as attach, including a
            // just-completed attach whose response has not reached the chooser.
            if let Some(session) = op["result"]["session"].as_str() {
                if let Some(owned) = s.sessions.get(session) {
                    restore(owned)?;
                }
                s.sessions.remove(session);
            }
            s.launches.insert(key.clone(), json!({"state":"cancelled"}));
            json!({"cancelled":key})
        }
        "open" => {
            let rows = windows()?;
            let mut s = store.inner.lock().map_err(err)?;
            let selected = s
                .apps
                .iter()
                .find(|a| Some(&a.id) == r.app_id.as_ref())
                .cloned()
                .ok_or("앱 목록을 새로 고친 뒤 선택하세요")?;
            let existing: Vec<_> = rows
                .into_iter()
                .filter(|w| w.identity.exe.eq_ignore_ascii_case(&selected.exe))
                .collect();
            if !existing.is_empty() {
                s.inventory.extend(
                    existing
                        .iter()
                        .map(|w| (w.identity.hwnd, w.identity.clone())),
                );
                return Ok(
                    json!({"ok":true,"instance":crate::instance::current().id,"existing_windows":existing}),
                );
            }
            if s.launches.values().any(|v| v["state"] == "pending") {
                return Err("다른 앱 시작이 진행 중입니다. 완료하거나 취소하세요".into());
            }
            if s.launches.len() >= 64 {
                s.launches.retain(|_, v| v["state"] == "pending");
            }
            s.next += 1;
            let key = format!("launch-{}-{}", std::process::id(), s.next);
            s.launches.insert(key.clone(), json!({"state":"pending"}));
            let inner = store.inner.clone();
            let op = key.clone();
            let region = r.region.unwrap();
            drop(s);
            std::thread::spawn(move || {
                if inner.lock().map_or(true, |s| {
                    s.shutting_down || s.launches.get(&op).is_none_or(|v| v["state"] != "pending")
                }) {
                    return;
                }
                let launch = Command::new(&selected.exe)
                    .raw_arg(&selected.arguments)
                    .spawn();
                let mut child = match launch {
                    Ok(c) => c,
                    Err(e) => {
                        if let Ok(mut s) = inner.lock() {
                            s.launches
                                .insert(op, json!({"state":"failed","error":e.to_string()}));
                        }
                        return;
                    }
                };
                let pid = child.id();
                let deadline = Instant::now() + Duration::from_secs(12);
                loop {
                    if let Ok(mut s) = inner.lock() {
                        if s.shutting_down
                            || s.launches.get(&op).is_none_or(|v| v["state"] != "pending")
                        {
                            break;
                        }
                        if let Ok(rows) = windows() {
                            let candidates: Vec<_> = rows
                                .into_iter()
                                .filter(|w| {
                                    w.identity.pid == pid
                                        && w.identity.exe.eq_ignore_ascii_case(&selected.exe)
                                })
                                .collect();
                            if candidates.len() == 1 {
                                let result = attach(&mut s, candidates[0].identity.clone(), region);
                                s.launches.insert(
                                    op.clone(),
                                    match result {
                                        Ok(v) => json!({"state":"attached","result":v}),
                                        Err(e) => json!({"state":"failed","error":e}),
                                    },
                                );
                                break;
                            }
                            if candidates.len() > 1 {
                                s.launches.insert(op.clone(),json!({"state":"failed","error":"여러 창이 열렸습니다. 열린 창 목록에서 선택하세요"}));
                                break;
                            }
                        }
                        if Instant::now() >= deadline {
                            s.launches.insert(op.clone(),json!({"state":"failed","error":"12초 안에 단일 대상 창을 확인하지 못했습니다. 열린 창 목록에서 선택하세요"}));
                            break;
                        }
                    } else {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                // The external app remains user-owned on cancellation/failure. Reap only.
                let _ = child.wait();
            });
            json!({"launch":key,"state":"pending"})
        }
        _ => return Err("지원하지 않는 외부 앱 작업입니다".into()),
    };
    let mut result = result;
    result["ok"] = json!(true);
    result["instance"] = json!(crate::instance::current().id);
    Ok(result)
}
pub(super) fn shutdown(app: &tauri::AppHandle) {
    if let Ok(mut s) = app.state::<ExternalApps>().inner.lock() {
        s.shutting_down = true;
        for session in s.sessions.values() {
            if let Err(e) = restore(session) {
                eprintln!("외부 창 복원 실패: {e}");
            }
        }
    }
}

pub(super) fn release_all(app: &tauri::AppHandle) -> Result<(), String> {
    let store = app.state::<ExternalApps>();
    let mut s = store.inner.lock().map_err(err)?;
    let keys: Vec<_> = s.sessions.keys().cloned().collect();
    let mut errors = Vec::new();
    for key in keys {
        match restore(&s.sessions[&key]) {
            Ok(()) => {
                s.sessions.remove(&key);
            }
            Err(e) => errors.push(format!("{key}: {e}")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    identity: Identity,
    rect: [i32; 4],
    topmost: bool,
    placement: Placement,
}
fn lease(id: &Identity) -> Result<File, String> {
    let dir = std::env::temp_dir().join("agent-gibbon-external-leases");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(format!("{}-{}-{}.lock", id.pid, id.created, id.hwnd)))
        .map_err(err)?;
    f.try_lock_exclusive()
        .map_err(|_| "다른 Gibbon 인스턴스가 이 창을 관리 중입니다")?;
    Ok(f)
}
pub(super) fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let store = app.state::<ExternalApps>();
    let mut s = store.inner.lock().map_err(err)?;
    if s.initialized {
        return Ok(());
    }
    s.root = app
        .state::<crate::storage::Store>()
        .0
        .join(".external-recovery");
    std::fs::create_dir_all(&s.root).map_err(err)?;
    for entry in std::fs::read_dir(&s.root).map_err(err)?.take(64) {
        let path = entry.map_err(err)?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(err)?;
        if bytes.len() > 65536 {
            return Err("외부 창 복원 기록이 너무 큽니다".into());
        }
        let saved: Saved = serde_json::from_slice(&bytes).map_err(err)?;
        let f = match lease(&saved.identity) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let session = Session {
            original: Window {
                window: saved.identity.hwnd.to_string(),
                title: "복원 대기 창".into(),
                identity: saved.identity,
                rect: saved.rect,
                topmost: saved.topmost,
            },
            placement: saved.placement,
            _lease: f,
            journal: path,
        };
        if let Err(e) = restore(&session) {
            s.next += 1;
            let key = format!("recovery-{}", s.next);
            s.sessions.insert(key, session);
            eprintln!("외부 창 복원 보류: {e}");
        }
    }
    s.initialized = true;
    Ok(())
}
