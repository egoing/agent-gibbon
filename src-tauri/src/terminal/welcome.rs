//! Generic shell windows. Bounded recent directories and presentation metadata persist.
use super::*;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn home() -> Result<PathBuf, String> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .ok_or("홈 폴더를 찾을 수 없습니다".into())
}
fn latest(agents: impl Iterator<Item = Terminal>) -> Option<PathBuf> {
    agents
        .filter(|a| a.kind == Kind::Terminal && a.last_path.as_ref().is_some_and(|p| p.is_dir()))
        .max_by_key(|a| a.path_updated)
        .and_then(|a| a.last_path)
}
pub fn create(app: &tauri::AppHandle, bounds: crate::capture::ImageNote) -> Result<Value, String> {
    let state = app.state::<Terminals>();
    let last_path = latest(entries(app).into_iter());
    let id = format!(
        "terminal-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_nanos()
    );
    let agent = Terminal {
        id: id.clone(),
        bounds,
        physical_position: cfg!(windows),
        kind: Kind::Terminal,
        last_path,
        recent_paths: Vec::new(),
        path_updated: 0,
        updated_at: 0,
        title: String::new(),
        restore_plan: None,
        auto_restore: false,
    };
    crate::memo_color::create(&root(app), &id)?;
    state.records.insert(&root(app), agent)?;
    crate::session_restore::mark_reopened(app, &id);
    open(app, &id)?;
    crate::tray::refresh(app)?;
    inspect(app, &id)
}

fn default_shell() -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        Ok(std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && executable(p))
            .unwrap_or_else(|| PathBuf::from("/bin/sh")))
    }
    #[cfg(windows)]
    {
        Ok(
            PathBuf::from(std::env::var_os("SystemRoot").ok_or("SystemRoot가 없습니다")?)
                .join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        )
    }
}
fn remember_path(agent: &mut Terminal, path: PathBuf, updated: u64) {
    // Migrate the former single-path record on the first directory change.
    if let Some(old) = agent.last_path.as_ref() {
        if old != &path {
            if let Some(record) = agent.recent_paths.iter_mut().find(|r| &r.path == old) {
                record.updated = record.updated.max(agent.path_updated);
            } else {
                agent.recent_paths.push(RecentPath {
                    path: old.clone(),
                    updated: agent.path_updated,
                });
            }
        }
    }
    agent.recent_paths.retain(|r| r.path != path);
    agent.recent_paths.push(RecentPath {
        path: path.clone(),
        updated,
    });
    agent
        .recent_paths
        .sort_by_key(|r| std::cmp::Reverse(r.updated));
    agent.recent_paths.truncate(20);
    agent.last_path = Some(path);
    agent.path_updated = updated;
}
fn recent_directories(agents: impl Iterator<Item = Terminal>) -> Vec<RecentPath> {
    let mut paths = BTreeMap::<PathBuf, u64>::new();
    for a in agents.filter(|a| a.kind == Kind::Terminal) {
        let mut recent = a.recent_paths;
        if let Some(path) = a.last_path {
            recent.push(RecentPath {
                path,
                updated: a.path_updated,
            });
        }
        for r in recent
            .into_iter()
            .filter(|r| r.path.is_absolute() && r.path.is_dir())
        {
            paths
                .entry(r.path)
                .and_modify(|t| *t = (*t).max(r.updated))
                .or_insert(r.updated);
        }
    }
    let mut recent: Vec<_> = paths
        .into_iter()
        .map(|(path, updated)| RecentPath { path, updated })
        .collect();
    recent.sort_by_key(|r| std::cmp::Reverse(r.updated));
    recent.truncate(20);
    recent
}
fn folder_value(path: &Path, home: &Path) -> Value {
    let parent = path.parent().unwrap_or(path);
    let display_parent = parent
        .strip_prefix(home)
        .map(|suffix| {
            if suffix.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", suffix.to_string_lossy())
            }
        })
        .unwrap_or_else(|_| parent.to_string_lossy().into_owned());
    json!({"path":path,"name":path.file_name().unwrap_or(path.as_os_str()).to_string_lossy(),"parent":display_parent,"command":format!("{}{}",if cfg!(windows){"Set-Location -LiteralPath "}else{"cd -- "},quote(&path.to_string_lossy()))})
}
fn executable(path: &Path) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
fn search_path() -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|p| p.is_absolute())
        .collect();
    if let Ok(home) = home() {
        for suffix in [
            ".local/bin",
            ".cargo/bin",
            ".npm-global/bin",
            "AppData/Roaming/npm",
        ] {
            paths.push(home.join(suffix));
        }
    }
    for path in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        paths.push(PathBuf::from(path));
    }
    paths
}
fn resolve(paths: &[PathBuf], name: &str) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
        ]
    } else {
        vec![name.into()]
    };
    paths
        .iter()
        .flat_map(|p| names.iter().map(move |n| p.join(n)))
        .find(|p| executable(p))
}
fn quote(value: &str) -> String {
    format!(
        "'{}'",
        value.replace('\'', if cfg!(windows) { "''" } else { "'\\''" })
    )
}
fn command(paths: &[PathBuf], choice: &str) -> Result<Option<String>, String> {
    let (name, args) = match choice {
        "shell" => return Ok(None),
        "codex" => ("codex", ""),
        "claude" => ("claude", ""),
        "git" => ("git", " log --oneline --decorate -20"),
        "htop" => ("htop", ""),
        _ => return Err("지원하지 않는 실행 항목입니다".into()),
    };
    let path =
        resolve(paths, name).ok_or("실행 파일을 찾을 수 없습니다. 설치 상태를 확인하세요")?;
    Ok(Some(format!(
        "{}{}{args}",
        if cfg!(windows) { "& " } else { "" },
        quote(&path.to_string_lossy())
    )))
}
fn choices(paths: &[PathBuf]) -> Vec<Value> {
    [
        ("claude", "Claude Code"),
        ("codex", "Codex"),
        ("git", "git log"),
        ("htop", "htop"),
    ]
    .into_iter()
    .filter_map(|(id, label)| {
        command(paths, id)
            .ok()
            .flatten()
            .map(|cmd| json!({"id":id,"label":label,"command":cmd}))
    })
    .collect()
}
#[tauri::command]
pub fn terminal_welcome(window: WebviewWindow) -> Result<Value, String> {
    let id = caller(&window)?;
    let info = inspect(window.app_handle(), &id)?;
    let home = home()?;
    let recent = recent_directories(entries(window.app_handle()).into_iter());
    let folders: Vec<_> = recent
        .iter()
        .map(|r| folder_value(&r.path, &home))
        .collect();
    let mut options = choices(&search_path());
    let shell = default_shell()?;
    let shell_command = format!(
        "{}{}{}",
        if cfg!(windows) { "& " } else { "" },
        quote(&shell.to_string_lossy()),
        if cfg!(windows) {
            " -NoLogo -NoExit"
        } else {
            " -i"
        }
    );
    options.push(json!({"id":"shell","label":"셸","command":shell_command}));
    let selected = info["last_path"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.clone());
    Ok(
        json!({"info":info,"choices":options,"folders":folders,"location":folder_value(&selected,&home),"home":home}),
    )
}
#[tauri::command]
pub async fn terminal_folder(window: WebviewWindow, path: Option<String>) -> Result<Value, String> {
    let id = caller(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if app
            .state::<Terminals>()
            .sessions
            .lock()
            .map_err(err)?
            .contains_key(&id)
        {
            return Err("실행 중에는 셸에서 폴더를 변경하세요".into());
        }
        let mut selected = false;
        if let Some(path) = path {
            selected = true;
            set_path(&app, &id, PathBuf::from(path), EditSource::Request)?;
        } else if let Some(path) = app
            .dialog()
            .file()
            .set_title("작업 폴더 선택")
            .blocking_pick_folder()
        {
            selected = true;
            let path = path.into_path().map_err(err)?;
            set_path(&app, &id, path, EditSource::Request)?;
        }
        let mut info = inspect(&app, &id)?;
        info["folder_selected"] = json!(selected);
        Ok(info)
    })
    .await
    .map_err(err)?
}
pub(super) fn set_path(
    app: &tauri::AppHandle,
    id: &str,
    path: PathBuf,
    source: EditSource,
) -> Result<(), String> {
    if !path.is_absolute() || !path.is_dir() {
        return Err("사용 가능한 폴더가 아닙니다".into());
    }
    let state = app.state::<Terminals>();
    if !state.records.edit(&root(app), id, source, |agent| {
        if agent.kind != Kind::Terminal || agent.last_path.as_ref() == Some(&path) {
            return false;
        }
        remember_path(agent, path, now());
        true
    })? {
        return Err("터미널 창이 없습니다".into());
    }
    Ok(())
}
#[tauri::command]
pub fn terminal_rename(window: WebviewWindow, title: String) -> Result<Value, String> {
    let id = caller(&window)?;
    if title.chars().count() > 120 || title.contains(['\n', '\r']) {
        return Err("제목은 한 줄, 120자 이하여야 합니다".into());
    }
    {
        let state = window.state::<Terminals>();
        if !state.records.edit(
            &root(window.app_handle()),
            &id,
            EditSource::Request,
            |record| {
                record.title = title.trim().into();
                record.updated_at = now();
                true
            },
        )? {
            return Err("터미널 창이 없습니다".into());
        }
    }
    crate::tray::refresh(window.app_handle())?;
    inspect(window.app_handle(), &id)
}
#[cfg(target_os = "macos")]
fn process_cwd(pid: u32) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let count = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            &mut info as *mut _ as *mut _,
            size,
        )
    };
    if count != size {
        return None;
    }
    let bytes =
        unsafe { std::ffi::CStr::from_ptr(info.pvi_cdir.vip_path.as_ptr().cast()) }.to_bytes();
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}
#[cfg(target_os = "linux")]
fn process_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn process_cwd(_pid: u32) -> Option<PathBuf> {
    None
}
#[cfg(target_os = "macos")]
pub(super) fn foreground_name(session: &Session) -> Option<String> {
    let pid = session.master.process_group_leader()?;
    if Some(pid as u32) == session.child.process_id() {
        return None;
    }
    let mut buf = [0u8; 256];
    let len = unsafe { libc::proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if len <= 0 {
        return None;
    }
    let name = std::str::from_utf8(&buf[..len as usize])
        .ok()?
        .trim_end_matches(char::from(0));
    plan_preview(name)
}
#[cfg(not(target_os = "macos"))]
pub(super) fn foreground_name(_: &Session) -> Option<String> {
    None
}
// Resolve names only inside the installation roots associated with their source.
fn monitor_path(name: &str) -> Option<PathBuf> {
    let allowlist: BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("auto-restore-macos.json")).ok()?;
    for (source, names) in allowlist {
        if !names.iter().any(|allowed| allowed == name) {
            continue;
        }
        let roots: &[&str] = match source.as_str() {
            "system" => &["/usr/bin", "/usr/sbin", "/bin", "/sbin"],
            "homebrew" => &["/opt/homebrew/bin", "/usr/local/bin"],
            _ => continue,
        };
        if let Some(path) = roots
            .iter()
            .map(|root| Path::new(root).join(name))
            .find(|path| executable(path))
        {
            return Some(path);
        }
    }
    None
}

fn require_restore_support(os: &str, restoring: bool) -> Result<(), String> {
    if restoring && os != "macos" {
        return Err("터미널 복원 실행은 이 운영체제에서 아직 지원하지 않습니다. 현재 macOS에서만 사용할 수 있습니다.".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn bare_monitor(session: &Session) -> Option<String> {
    use std::ffi::CStr;
    let pid = session.master.process_group_leader()?;
    let mut path = [0u8; 4096];
    if unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) } <= 0 {
        return None;
    }
    let path = Path::new(CStr::from_bytes_until_nul(&path).ok()?.to_str().ok()?);
    let name = path.file_name()?.to_str()?;
    if monitor_path(name)?.canonicalize().ok()? != path.canonicalize().ok()? {
        return None;
    }
    // Darwin returns argc first. Discard the buffer without decoding argv or env.
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut length = 0;
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        ) != 0
            || !(4..=2_097_152).contains(&length)
        {
            return None;
        }
        let mut bytes = vec![0u8; length];
        if libc::sysctl(
            mib.as_mut_ptr(),
            3,
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        ) != 0
            || length < 4
            || i32::from_ne_bytes(bytes[..4].try_into().ok()?) != 1
        {
            return None;
        }
    }
    Some(name.to_owned())
}
#[cfg(not(target_os = "macos"))]
fn bare_monitor(_: &Session) -> Option<String> {
    None
}

pub(super) fn refresh_path(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<Terminals>();
    if !state.records.contains(id) {
        return Ok(());
    }
    let session = state
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .and_then(running_generation);
    if let Some((token, session)) = session {
        let mut s = session.lock().map_err(err)?;
        refresh_session_path(&state, &root(app), id, RecordOwner::Active(token), &mut s)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum RecordOwner {
    Active(u64),
    Exited(u64),
    Shutdown,
}

impl Terminals {
    pub(super) fn edit_owned(
        &self,
        root: &Path,
        id: &str,
        owner: RecordOwner,
        source: EditSource,
        edit: impl FnOnce(&mut Terminal) -> bool,
    ) -> Result<(), String> {
        // Serialize with all metadata writers, then recheck the slot at the
        // point of mutation. Release the registry before doing any disk I/O.
        let exists = self.records.edit_checked(root, id, source, |record| {
            let slots = self.sessions.lock().map_err(err)?;
            let slot = slots.get(id);
            let allowed = match owner {
                RecordOwner::Shutdown => self.stopping.load(Ordering::Acquire) && slot.is_none(),
                RecordOwner::Active(token) => {
                    !self.stopping.load(Ordering::Acquire)
                        && slot.is_some_and(|s| {
                            s.token() == token && !matches!(s, Slot::Exited { .. })
                        })
                }
                RecordOwner::Exited(token) => {
                    matches!(slot, Some(Slot::Exited { token: current, .. }) if *current == token)
                }
            };
            if !allowed {
                return match source {
                    EditSource::Request => Err("터미널 세션이 변경되었습니다".into()),
                    EditSource::Observation => Ok(false),
                };
            }
            Ok(edit(record))
        })?;
        if !exists && source == EditSource::Request {
            return Err("터미널 창이 없습니다".into());
        }
        Ok(())
    }
}

pub(super) fn start_metadata(
    app: &tauri::AppHandle,
    id: &str,
    token: u64,
    initial: Option<&str>,
) -> Result<(), String> {
    app.state::<Terminals>().edit_owned(
        &root(app),
        id,
        RecordOwner::Active(token),
        EditSource::Request,
        |record| {
            let plan = initial.and_then(plan_preview);
            let changed =
                record.restore_plan != plan || record.last_path.is_some() || record.auto_restore;
            record.restore_plan = plan;
            record.auto_restore = false;
            if record.last_path.is_some() {
                record.path_updated = now();
            }
            changed
        },
    )
}

pub(super) fn reset_auto_restore(record: &mut Terminal) -> bool {
    std::mem::take(&mut record.auto_restore)
}

pub(super) fn retire(
    app: &tauri::AppHandle,
    id: &str,
    token: u64,
    status: portable_pty::ExitStatus,
    jobs: OwnedJobs,
) -> bool {
    let state = app.state::<Terminals>();
    if !state.mark_exited(id, token, status, jobs) {
        return false;
    }
    let _ = state.edit_owned(
        &root(app),
        id,
        RecordOwner::Exited(token),
        EditSource::Observation,
        reset_auto_restore,
    );
    true
}

pub(super) fn output_metadata(
    app: &tauri::AppHandle,
    id: &str,
    token: u64,
    plan: Option<&str>,
    paths: Vec<PathBuf>,
) {
    let paths: Vec<_> = paths
        .into_iter()
        .filter(|p| p.is_absolute() && p.is_dir())
        .collect();
    let _ = app.state::<Terminals>().edit_owned(
        &root(app),
        id,
        RecordOwner::Active(token),
        EditSource::Observation,
        |record| {
            let mut changed = false;
            if let Some(plan) = plan {
                let plan = plan_preview(plan);
                if record.restore_plan != plan {
                    record.restore_plan = plan;
                    changed = true;
                }
            }
            for path in paths {
                if record.last_path.as_ref() != Some(&path) {
                    remember_path(record, path, now());
                    changed = true;
                }
            }
            changed
        },
    );
}

#[derive(Default)]
struct SessionSnapshot {
    plan: Option<String>,
    monitor: Option<String>,
    path: Option<PathBuf>,
}

impl SessionSnapshot {
    fn observe(s: &mut Session) -> Result<Self, String> {
        if s.child.try_wait().map_err(err)?.is_some() {
            return Ok(Self::default());
        }
        Ok(Self {
            plan: (!s.observed_command).then(|| foreground_name(s)).flatten(),
            monitor: bare_monitor(s),
            path: s
                .child
                .process_id()
                .and_then(process_cwd)
                .filter(|p| p.is_absolute() && p.is_dir()),
        })
    }

    fn apply(self, record: &mut Terminal) -> bool {
        let mut changed = false;
        if let Some(name) = self.plan {
            let plan = plan_preview(&name);
            changed |= record.restore_plan != plan;
            record.restore_plan = plan;
        }
        let eligible = self.monitor.is_some() && self.monitor == record.restore_plan;
        changed |= record.auto_restore != eligible;
        record.auto_restore = eligible;
        if let Some(path) = self.path.filter(|p| record.last_path.as_ref() != Some(p)) {
            remember_path(record, path, now());
            changed = true;
        }
        changed
    }
}

pub(super) fn refresh_session_path(
    state: &Terminals,
    root: &Path,
    id: &str,
    owner: RecordOwner,
    s: &mut Session,
) -> Result<(), String> {
    let snapshot = match SessionSnapshot::observe(s) {
        Ok(value) => value,
        Err(_) if matches!(owner, RecordOwner::Shutdown) => SessionSnapshot::default(),
        Err(error) => return Err(error),
    };
    state.edit_owned(root, id, owner, EditSource::Observation, |record| {
        snapshot.apply(record)
    })
}
fn plan_preview(line: &str) -> Option<String> {
    let mut words = line.split_ascii_whitespace();
    let name = words.next()?.rsplit('/').next()?;
    if name.len() > 80
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return None;
    }
    let allowed: &[&str] = match name {
        "df" | "du" => &["-h", "-k"],
        "ps" => &["-A", "-e"],
        _ => &[],
    };
    let args: Vec<_> = words.collect();
    if args.len() <= 8 && args.iter().all(|a| allowed.contains(a)) {
        Some(
            std::iter::once(name)
                .chain(args)
                .collect::<Vec<_>>()
                .join(" "),
        )
    } else {
        Some(name.into())
    }
}
pub(super) fn start(
    agent: &Terminal,
    choice: &str,
    plan: Option<&str>,
    dimensions: PtySize,
    spawned: impl FnOnce(OwnedChild),
) -> Result<(Session, PtyReader), String> {
    require_restore_support(
        std::env::consts::OS,
        plan.is_some() || choice == "auto-monitor",
    )?;
    let prepare = crate::telemetry::Timing::new("terminal.command.prepare");
    let cwd = agent.last_path.clone().map(Ok).unwrap_or_else(home)?;
    if !cwd.is_dir() {
        return Err("마지막 폴더가 없습니다. 다른 폴더를 선택하세요".into());
    }
    let line = if choice == "auto-monitor" {
        let path = agent
            .restore_plan
            .as_deref()
            .filter(|_| agent.auto_restore && plan.is_none())
            .and_then(monitor_path)
            .ok_or("자동 복원할 프로그램을 확인할 수 없습니다")?;
        Some(quote(&path.to_string_lossy()))
    } else {
        match plan {
            Some(value)
                if agent.restore_plan.is_some()
                    && !value.trim().is_empty()
                    && value.len() <= 4096
                    && !value.chars().any(char::is_control) =>
            {
                Some(value.to_owned())
            }
            Some(_) => return Err("실행 계획을 확인하세요".into()),
            None => command(&search_path(), choice)?,
        }
    };
    let shell = default_shell()?;
    drop(prepare);
    start_shell_supervised(&cwd, line, dimensions, &shell, spawned)
}
#[cfg(test)]
fn start_shell(
    cwd: &Path,
    line: Option<String>,
    dimensions: PtySize,
    shell: &Path,
) -> Result<(Session, PtyReader), String> {
    start_shell_supervised(cwd, line, dimensions, shell, drop)
}
fn start_shell_supervised(
    cwd: &Path,
    line: Option<String>,
    dimensions: PtySize,
    shell: &Path,
    spawned: impl FnOnce(OwnedChild),
) -> Result<(Session, PtyReader), String> {
    let prompt = PromptState::new();
    let pair = {
        let _timing = crate::telemetry::Timing::new("terminal.pty.create");
        native_pty_system().openpty(dimensions).map_err(err)?
    };
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env_remove("NO_COLOR");
    #[cfg(unix)]
    let setup = match shell.file_name().and_then(|name| name.to_str()) {
        Some("zsh") => Some("autoload -Uz add-zsh-hook; add-zsh-hook preexec _gibbon_record"),
        Some("bash") => Some(include_str!("restore-bash-hook.sh")),
        _ => None,
    };
    #[cfg(unix)]
    if let Some(hook) = setup {
        let definitions = include_str!("restore-hook.sh").replace("GIBBON_TOKEN", &prompt.token);
        cmd.env("AGENT_GIBBON_SHELL_SETUP", format!("{definitions}\n{hook}"));
    }
    #[cfg(unix)]
    {
        cmd.arg("-i");
        // A shell which replaces PS1 remains unknown and asks before closing.
        cmd.env(
            "PS1",
            format!("\x1b]777;gibbon-ready;{}\x07$ ", prompt.token),
        );
    }
    #[cfg(windows)]
    {
        cmd.args(["-NoLogo", "-NoExit", "-Command"]);
        let prompt_script = r#"function global:prompt { $entry = Get-History -Count 1; if ($entry -and $global:gibbonLastId -ne $entry.Id) { $global:gibbonLastId = $entry.Id; $name = ''; if ($entry.CommandLine -match '^\s*([A-Za-z][A-Za-z0-9._-]{0,79})(?:\s|$)') { $name = $Matches[1] }; [Console]::Write(([char]27 + ']777;gibbon-plan;TOKEN;' + $name + [char]7)) }; [Console]::Write(([char]27 + ']777;gibbon-cwd;' + (Get-Location).Path + [char]7)); [Console]::Write(([char]27 + ']777;gibbon-ready;TOKEN' + [char]7)); 'PS> ' }; "#;
        cmd.arg(format!(
            "{}{}",
            prompt_script.replace("TOKEN", &prompt.token),
            line.as_deref().unwrap_or("")
        ));
    }
    let child = {
        let _spawn = crate::telemetry::Timing::new("terminal.process.spawn");
        crate::telemetry::checkpoint("terminal.process.spawn");
        pair.slave.spawn_command(cmd).map_err(err)?
    };
    drop(pair.slave);
    let child = OwnedChild::new(child);
    spawned(child.clone());
    let mut session = Session {
        master: pair.master,
        writer: Box::new(std::io::sink()),
        child,
        program: "shell".into(),
        observed_command: false,
        prompt,
    };
    let _attach = crate::telemetry::Timing::new("terminal.io.attach");
    let reader = PtyReader::new(&*session.master)?;
    session.writer = session.master.take_writer().map_err(err)?;
    #[cfg(unix)]
    if setup.is_some() {
        // Keep the initial PTY input short, so startup can finish before output draining begins.
        session
            .writer
            .write_all(b" eval \"$AGENT_GIBBON_SHELL_SETUP\"; unset AGENT_GIBBON_SHELL_SETUP\r")
            .map_err(err)?;
    }
    #[cfg(unix)]
    if let Some(line) = line {
        session.prompt.input(format!("{line}\r").as_bytes());
        session
            .writer
            .write_all(format!("{line}\r").as_bytes())
            .map_err(err)?;
        session.writer.flush().map_err(err)?;
    }
    Ok((session, reader))
}
fn busy(session: &mut Session) -> bool {
    if session.child.try_wait().ok().flatten().is_some() {
        // The root has exited but the pump may still be draining a background job.
        return true;
    }
    if !session.prompt.ready {
        return true;
    }
    #[cfg(unix)]
    {
        session
            .child
            .process_id()
            .zip(session.master.process_group_leader())
            .is_none_or(|(pid, fg)| {
                pid as i32 != fg || descendants(pid).is_none_or(|children| !children.is_empty())
            })
    }
    #[cfg(windows)]
    {
        session
            .child
            .process_id()
            .and_then(crate::windows_process::has_children)
            .unwrap_or(true)
    }
}
#[tauri::command]
pub fn terminal_close(window: WebviewWindow) -> Result<(), String> {
    caller(&window)?;
    request_close(&window)
}
pub fn request_close(window: &WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let id = window.label().to_string();
    let state = app.state::<Terminals>();
    let sessions = state.sessions.lock().map_err(err)?;
    let expected = close_identity(sessions.get(&id));
    let remaining_jobs =
        matches!(sessions.get(&id), Some(Slot::Exited {jobs,..}) if jobs.may_have_jobs());
    let session = sessions.get(&id).and_then(running);
    drop(sessions);
    let needs_confirm = remaining_jobs
        || session
            .as_ref()
            .is_some_and(|s| s.try_lock().map(|mut s| busy(&mut s)).unwrap_or(true));
    if needs_confirm {
        let app = app.clone();
        app.clone()
            .dialog()
            .message("실행 중인 작업을 종료하고 창을 닫을까요?")
            .title("터미널 종료")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "종료".into(),
                "취소".into(),
            ))
            .show(move |yes| {
                if yes {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        let _ = finish_close(&app2, &id, expected);
                    });
                }
            });
        Ok(())
    } else {
        finish_close(app, &id, expected)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseIdentity {
    Starting(u64),
    Session(u64),
}
fn close_identity(slot: Option<&Slot>) -> Option<CloseIdentity> {
    slot.map(|s| match s {
        Slot::Starting { token, .. } => CloseIdentity::Starting(*token),
        Slot::Running { token, .. } | Slot::Exited { token, .. } => CloseIdentity::Session(*token),
    })
}
struct WindowChange<'a> {
    state: &'a Terminals,
    id: &'a str,
}

impl<'a> WindowChange<'a> {
    fn begin(
        state: &'a Terminals,
        id: &'a str,
        expected: Option<CloseIdentity>,
    ) -> Result<Option<Self>, String> {
        let sessions = state.sessions.lock().map_err(err)?;
        if state.stopping.load(Ordering::Acquire)
            || close_identity(sessions.get(id)) != expected
            || !state.closing.lock().map_err(err)?.insert(id.to_owned())
        {
            return Ok(None);
        }
        Ok(Some(Self { state, id }))
    }
}
impl Drop for WindowChange<'_> {
    fn drop(&mut self) {
        let _sessions = self.state.sessions.lock().unwrap();
        self.state.closing.lock().unwrap().remove(self.id);
    }
}

type CloseFrame = (crate::capture::ImageNote, bool);

// Finish all native calls before releasing the slot. File IO happens afterwards.
fn destroy_capturing_frame(app: &tauri::AppHandle, id: &str) -> Result<Option<CloseFrame>, String> {
    let Some(window) = app.get_webview_window(id) else {
        return Ok(None);
    };
    crate::stash::release(&window)?;
    let frame = crate::session_restore::logical_geometry(&window)?;
    window.destroy().map_err(err)?;
    Ok(Some(frame))
}

fn save_close_frame(
    app: &tauri::AppHandle,
    id: &str,
    frame: Option<CloseFrame>,
) -> Result<(), String> {
    let Some((bounds, physical)) = frame else {
        return Ok(());
    };
    let journal = crate::session_restore::remember_closed(app, id, bounds.clone(), physical);
    if !matches!(journal, Ok(true)) {
        // Preserve the frame even if the first journal commit is unavailable or fails.
        if !app
            .state::<Terminals>()
            .records
            .edit(&root(app), id, EditSource::Request, |r| {
                r.bounds = bounds;
                r.physical_position = physical;
                true
            })?
        {
            return Err("터미널 창이 없습니다".into());
        }
    }
    journal.map(|_| ())
}

fn finish_close(
    app: &tauri::AppHandle,
    id: &str,
    expected: Option<CloseIdentity>,
) -> Result<(), String> {
    let state = app.state::<Terminals>();
    let closed = crate::session_restore::during_native_close(app, || {
        close_window(
            &state,
            &root(app),
            id,
            expected,
            || destroy_capturing_frame(app, id),
            |frame| save_close_frame(app, id, frame),
        )
    })?;
    if closed {
        crate::tray::refresh(app)?;
    }
    Ok(())
}

fn close_window<F>(
    state: &Terminals,
    root: &Path,
    id: &str,
    expected: Option<CloseIdentity>,
    destroy: impl FnOnce() -> Result<F, String>,
    save_frame: impl FnOnce(F) -> Result<(), String>,
) -> Result<bool, String> {
    let Some(guard) = WindowChange::begin(state, id, expected)? else {
        return Ok(false);
    };
    let session = state
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .and_then(running);
    let snapshot = session.and_then(|s| SessionSnapshot::observe(&mut *s.try_lock().ok()?).ok());
    let frame = destroy()?;

    // No native calls remain. Register the final save under the slot registry
    // lock used by quit, then drop Control before either kind of disk write.
    let (slot, _save) = {
        let mut slots = state.sessions.lock().map_err(err)?;
        (slots.remove(id), state.close_saves.enter())
    };
    let snapshot = slot.map(|slot| {
        let mut snapshot = snapshot.unwrap_or_default();
        if !matches!(slot, Slot::Running { .. }) {
            snapshot.monitor = None;
        }
        match slot {
            Slot::Exited { jobs, .. } => jobs.stop(),
            other => drop(other),
        }
        snapshot
    });
    let geometry_result = save_frame(frame);
    if let Some(snapshot) = snapshot {
        state
            .records
            .edit(root, id, EditSource::Observation, |r| snapshot.apply(r))?;
    }
    geometry_result?;
    drop(guard);
    Ok(true)
}

#[cfg(all(test, unix))]
pub(super) fn close_fixture(
    state: &Terminals,
    root: &Path,
    id: &str,
    token: u64,
    destroyed: impl FnOnce() -> Result<(), String>,
    save_frame: impl FnOnce(()) -> Result<(), String>,
) -> Result<bool, String> {
    close_window(
        state,
        root,
        id,
        Some(CloseIdentity::Session(token)),
        destroyed,
        save_frame,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoration_is_gated_before_starting_on_unsupported_platforms() {
        for os in ["windows", "linux"] {
            assert!(require_restore_support(os, true)
                .unwrap_err()
                .contains("아직 지원하지 않습니다"));
            assert!(require_restore_support(os, false).is_ok());
        }
        assert!(require_restore_support("macos", true).is_ok());
    }

    #[cfg(test)]
    fn agent(path: Option<PathBuf>, updated: u64) -> Terminal {
        Terminal {
            physical_position: false,
            id: "terminal-1".into(),
            bounds: crate::capture::ImageNote {
                x: 1.,
                y: 2.,
                width: 720.,
                height: 460.,
            },
            kind: Kind::Terminal,
            last_path: path,
            recent_paths: Vec::new(),
            path_updated: updated,
            updated_at: 0,
            title: String::new(),
            restore_plan: None,
            auto_restore: false,
        }
    }
    #[test]
    fn recent_folders_migrate_deduplicate_bound_and_filter_missing() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("old");
        std::fs::create_dir(&old).unwrap();
        let mut a = agent(Some(old.clone()), 1);
        for i in 0..25 {
            let path = root.path().join(format!("p{i}"));
            std::fs::create_dir(&path).unwrap();
            remember_path(&mut a, path, i + 2);
        }
        assert_eq!(a.recent_paths.len(), 20);
        let chosen = root.path().join("p10");
        remember_path(&mut a, chosen.clone(), 100);
        assert_eq!(a.recent_paths.len(), 20);
        assert_eq!(a.recent_paths[0].path, chosen);
        let restored: Terminal = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
        let duplicate = agent(Some(chosen.clone()), 99);
        std::fs::remove_dir(root.path().join("p24")).unwrap();
        let recent = recent_directories([restored, duplicate].into_iter());
        assert_eq!(recent[0].path, chosen);
        assert_eq!(recent.iter().filter(|r| r.path == chosen).count(), 1);
        assert!(recent.iter().all(|r| r.path.is_dir()));
        let mut legacy = agent(Some(old.clone()), 42);
        remember_path(&mut legacy, chosen, 43);
        assert!(legacy
            .recent_paths
            .iter()
            .any(|r| r.path == old && r.updated == 42));
    }
    #[test]
    fn preview_directory_quotes_spaces_and_metacharacters() {
        let folder = folder_value(Path::new("/tmp/a ' $(cmd)"), Path::new("/home/user"));
        assert_eq!(folder["path"], "/tmp/a ' $(cmd)");
        assert_eq!(
            folder["command"],
            format!(
                "{}{}",
                if cfg!(windows) {
                    "Set-Location -LiteralPath "
                } else {
                    "cd -- "
                },
                quote("/tmp/a ' $(cmd)")
            )
        );
    }
    #[test]
    fn no_history_has_no_display_path_and_home_is_execution_fallback() {
        let a = agent(None, 0);
        assert!(a.last_path.is_none());
        assert_eq!(a.display_title(), "터미널");
        assert!(home().unwrap().is_dir());
        assert!(latest([a].into_iter()).is_none());
    }
    #[test]
    fn latest_valid_path_and_custom_title_are_independent() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        assert_eq!(
            latest(
                [
                    agent(Some(a.path().into()), 1),
                    agent(Some(b.path().into()), 2),
                    agent(Some(a.path().join("missing")), 3)
                ]
                .into_iter()
            ),
            Some(b.path().into())
        );
        let mut a = agent(Some(a.path().into()), 1);
        a.title = "내 제목".into();
        assert_eq!(a.display_title(), "내 제목");
    }
    #[cfg(unix)]
    #[test]
    fn recommendations_do_not_execute_candidates_and_show_exact_quoted_command() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("path with ' quote");
        std::fs::create_dir(&bin).unwrap();
        for name in ["codex", "claude", "git", "htop"] {
            let p = bin.join(name);
            std::fs::write(
                &p,
                format!(
                    "#!/bin/sh\ntouch {}\n",
                    quote(&dir.path().join("SHOULD_NOT_EXIST").to_string_lossy())
                ),
            )
            .unwrap();
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let list = choices(std::slice::from_ref(&bin));
        assert_eq!(list.len(), 4);
        assert!(list[0]["command"].as_str().unwrap().contains("'\\''"));
        assert!(list[2]["command"]
            .as_str()
            .unwrap()
            .ends_with(" log --oneline --decorate -20"));
        assert!(!dir.path().join("SHOULD_NOT_EXIST").exists());
        std::fs::set_permissions(bin.join("codex"), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        assert_eq!(choices(&[bin]).len(), 3);
        assert_eq!(command(&[], "shell").unwrap(), None);
        assert!(command(&[], "resume").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn shell_command_exit_returns_to_same_pty_and_cwd_is_observed() {
        let dir = tempfile::tempdir().unwrap();
        let (mut session, mut reader) = start_shell(
            dir.path(),
            None,
            size(80, 24).unwrap(),
            Path::new("/bin/sh"),
        )
        .unwrap();
        let pid = session.child.process_id().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = [0; 4096];
            while let Ok(n) = reader.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                if tx
                    .send(String::from_utf8_lossy(&bytes[..n]).into_owned())
                    .is_err()
                {
                    break;
                }
            }
        });
        session.writer.write_all(b"/bin/sh -c 'exit 7'; printf '\\123\\110\\105\\114\\114\\137\\101\\114\\111\\126\\105\\n'\r").unwrap();
        session.writer.flush().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut output = String::new();
        while !output.contains("SHELL_ALIVE") {
            output.push_str(
                &rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .unwrap(),
            );
        }
        assert!(session.child.try_wait().unwrap().is_none());
        assert_eq!(session.child.process_id(), Some(pid));
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        assert_eq!(
            process_cwd(pid).unwrap().canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
        session.master.resize(size(100, 30).unwrap()).unwrap();
        assert_eq!(session.master.get_size().unwrap().cols, 100);
        drop(session);
    }
}

// Only the bounded directory control sequence is retained, never PTY transcript.
#[derive(Default)]
pub(super) struct DirectoryReport {
    frame: Vec<u8>,
    escape: bool,
    prompt: Option<String>,
    plan: Option<String>,
}
impl DirectoryReport {
    pub(super) fn take_prompt(&mut self, token: &str) -> bool {
        self.prompt
            .take()
            .is_some_and(|value| !token.is_empty() && value == token)
    }
    pub fn take_plan(&mut self, token: &str) -> Option<String> {
        let frame = self.plan.take()?;
        let (key, plan) = frame.split_once(';')?;
        (key == token).then(|| plan.to_owned())
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for &byte in bytes {
            if byte == 27 {
                self.frame.clear();
                self.escape = true;
                continue;
            }
            if !self.escape {
                continue;
            }
            if byte == 7 {
                if let Some(raw) = self.frame.strip_prefix(b"]777;gibbon-cwd;") {
                    if let Ok(path) = std::str::from_utf8(raw) {
                        paths.push(PathBuf::from(path));
                    }
                }
                if let Some(raw) = self.frame.strip_prefix(b"]777;gibbon-plan;") {
                    self.plan = std::str::from_utf8(raw).ok().map(str::to_owned);
                }
                if let Some(raw) = self.frame.strip_prefix(b"]777;gibbon-ready;") {
                    self.prompt = std::str::from_utf8(raw).ok().map(str::to_owned);
                }
                self.frame.clear();
                self.escape = false;
            } else if self.frame.len() < 32768 {
                self.frame.push(byte);
                let prefixes: [&[u8]; 3] = [
                    b"]777;gibbon-plan;",
                    b"]777;gibbon-cwd;",
                    b"]777;gibbon-ready;",
                ];
                if !prefixes
                    .iter()
                    .any(|prefix| prefix.starts_with(&self.frame) || self.frame.starts_with(prefix))
                {
                    self.frame.clear();
                    self.escape = false;
                }
            } else {
                self.frame.clear();
                self.escape = false;
            }
        }
        paths
    }
}
#[cfg(test)]
mod report_tests {
    use super::*;
    #[cfg(test)]
    #[test]
    fn directory_control_sequence_survives_chunks_without_retaining_transcript() {
        let mut parser = DirectoryReport::default();
        assert!(parser.feed(b"private output and commands").is_empty());
        assert!(parser.frame.is_empty());
        assert!(parser.feed(b"\x1b]777;gibbon-cwd;C:\\work").is_empty());
        assert_eq!(
            parser.feed(b"\\project\x07other text"),
            vec![PathBuf::from("C:\\work\\project")]
        );
        assert!(parser.frame.is_empty());
        parser.feed(b"\x1b[31mprivate output");
        assert!(parser.frame.is_empty());
        parser.feed(b"\x1b]");
        parser.feed(&vec![b'x'; 40000]);
        assert!(parser.frame.is_empty());
    }
}

// List deletion is deliberately distinct from closing a window or hiding a session.
pub fn delete(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<Terminals>();
    let _deleting = WindowChange::begin(&state, id, None)?.ok_or("먼저 터미널 창을 종료하세요")?;
    if !state.records.contains(id) {
        return Err("터미널이 없습니다".into());
    }
    crate::session_restore::during_native_close(app, || {
        let frame = destroy_capturing_frame(app, id)?;
        if let Err(error) = state.records.delete(&root(app), id) {
            save_close_frame(app, id, frame)?;
            return Err(error);
        }
        crate::session_restore::forget_closed(app, id)
    })?;
    crate::tray::refresh(app)
}

#[cfg(unix)]
pub(super) fn descendants(pid: u32) -> Option<Vec<i32>> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let pairs: Vec<(i32, i32)> = String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
        })
        .collect();
    let mut found = vec![pid as i32];
    let mut index = 0;
    while index < found.len() {
        let parent = found[index];
        for &(child, ppid) in &pairs {
            if ppid == parent && !found.contains(&child) {
                found.push(child)
            }
        }
        index += 1;
    }
    found.remove(0);
    Some(found)
}

#[derive(Default)]
pub(super) struct PromptState {
    pub token: String,
    pub ready: bool,
    due: usize,
    editing: bool,
}
impl PromptState {
    pub fn input(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.ready = false;
        self.editing = true;
        for byte in data {
            if matches!(byte, b'\r' | b'\n') {
                self.due = self.due.saturating_add(1);
                self.editing = false;
            }
        }
    }
    pub fn report(&mut self) {
        self.due = self.due.saturating_sub(1);
        self.ready = self.due == 0 && !self.editing;
    }
    pub(super) fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            token: format!(
                "{}-{}",
                now(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ),
            ready: false,
            due: 1,
            editing: false,
        }
    }
}
pub(super) fn touch_path(app: &tauri::AppHandle, id: &str, token: u64) -> Result<(), String> {
    let state = app.state::<Terminals>();
    state.edit_owned(
        &root(app),
        id,
        RecordOwner::Active(token),
        EditSource::Request,
        |agent| {
            if agent.kind != Kind::Terminal || agent.last_path.is_none() {
                return false;
            }
            agent.path_updated = now();
            true
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[cfg(test)]
    #[test]
    fn queued_startup_prompt_cannot_mark_submitted_builtin_idle() {
        let mut prompt = PromptState::new();
        prompt.input(b"while :; do :; done\r");
        prompt.report();
        assert!(!prompt.ready);
        prompt.report();
        assert!(prompt.ready);
        prompt.input(b"partial input");
        assert!(!prompt.ready);
    }
    #[test]
    fn old_reservation_does_not_match_reopened_window() {
        let state = Terminals::default();
        let first = Start::reserve(&state, "terminal-1").unwrap();
        let old = close_identity(state.sessions.lock().unwrap().get("terminal-1"));
        drop(first);
        let _next = Start::reserve(&state, "terminal-1").unwrap();
        assert_ne!(
            old,
            close_identity(state.sessions.lock().unwrap().get("terminal-1"))
        );
        assert_ne!(old, None);
    }
    #[test]
    fn deleting_or_closing_a_window_excludes_start_and_duplicate_changes() {
        let state = Terminals::default();
        let guard = WindowChange::begin(&state, "terminal-1", None)
            .unwrap()
            .unwrap();
        assert!(Start::reserve(&state, "terminal-1").is_err());
        assert!(WindowChange::begin(&state, "terminal-1", None)
            .unwrap()
            .is_none());
        drop(guard);
        let start = Start::reserve(&state, "terminal-1").unwrap();
        assert!(WindowChange::begin(&state, "terminal-1", None)
            .unwrap()
            .is_none());
        let close = WindowChange::begin(
            &state,
            "terminal-1",
            Some(CloseIdentity::Starting(start.token)),
        )
        .unwrap()
        .unwrap();
        drop(start);
        assert!(Start::reserve(&state, "terminal-1").is_err());
        drop(close);
        assert!(Start::reserve(&state, "terminal-1").is_ok());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn restore_runs_edited_command_in_real_shell_and_reports_program() {
        for shell in ["/bin/bash", "/bin/zsh"] {
            let dir = tempfile::tempdir().unwrap();
            let (session, mut reader) = start_shell(
                dir.path(),
                Some("touch ./restored".into()),
                size(80, 24).unwrap(),
                Path::new(shell),
            )
            .unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            let token = session.prompt.token.clone();
            std::thread::spawn(move || {
                let mut parser = DirectoryReport::default();
                let mut buf = [0; 4096];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    parser.feed(&buf[..n]);
                    if parser.take_plan(&token).as_deref() == Some("touch") {
                        let _ = tx.send(());
                        break;
                    }
                }
            });
            rx.recv_timeout(std::time::Duration::from_secs(8))
                .expect(shell);
            for _ in 0..100 {
                if dir.path().join("restored").exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(dir.path().join("restored").exists(), "{shell}");
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn bash_prompt_commands_and_blank_lines_do_not_replace_the_user_command() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join("rc");
        std::fs::write(&rc, "PROMPT_COMMAND='printf prompt'\n").unwrap();
        let bash = dir.path().join("bash");
        std::fs::write(
            &bash,
            format!(
                "#!/bin/sh\nexec /bin/bash --noprofile --rcfile {} -i\n",
                quote(&rc.to_string_lossy())
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bash, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (mut session, mut reader) = start_shell(
            dir.path(),
            Some("date".into()),
            size(80, 24).unwrap(),
            &bash,
        )
        .unwrap();
        let token = session.prompt.token.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut parser = DirectoryReport::default();
            let mut bytes = [0u8; 4096];
            while let Ok(n) = reader.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                parser.feed(&bytes[..n]);
                if let Some(plan) = parser.take_plan(&token) {
                    if tx.send(plan).is_err() {
                        break;
                    }
                }
            }
        });
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
            "date"
        );
        session.writer.write_all(b"\r").unwrap();
        session.writer.flush().unwrap();
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(300))
            .is_err());
        session.writer.write_all(b"pwd\r").unwrap();
        session.writer.flush().unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
            "pwd"
        );
    }

    #[cfg(unix)]
    #[test]
    fn real_shell_prompt_builtin_busy_and_child_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let (mut session, mut reader) = start_shell(
            dir.path(),
            None,
            size(80, 24).unwrap(),
            Path::new("/bin/sh"),
        )
        .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut parser = DirectoryReport::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.prompt.ready {
            parser.feed(
                &rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .unwrap(),
            );
            if parser.take_prompt(&session.prompt.token) {
                session.prompt.report();
            }
        }
        assert!(!busy(&mut session));
        let pid = session.child.process_id().unwrap();
        let input = b"while :; do :; done\r";
        session.prompt.input(input);
        session.writer.write_all(input).unwrap();
        session.writer.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(descendants(pid).unwrap().is_empty());
        assert!(busy(&mut session));
        drop(session);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while unsafe { libc::kill(pid as i32, 0) } == 0 {
            assert!(std::time::Instant::now() < deadline, "shell was not reaped");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let (mut session, mut reader) = start_shell(
            dir.path(),
            None,
            size(80, 24).unwrap(),
            Path::new("/bin/sh"),
        )
        .unwrap();
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut reader, &mut std::io::sink());
        });
        let pid = session.child.process_id().unwrap();
        session.writer.write_all(b"sleep 60 & wait\r").unwrap();
        session.writer.flush().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let children = loop {
            let children = descendants(pid).unwrap();
            if !children.is_empty() {
                break children;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "background child did not start"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(busy(&mut session));
        drop(session);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        for child in children {
            // The reaper runs outside the window-close callback. A reparented
            // zombie may remain until launchd/init reaps it; it must not run.
            loop {
                let status = std::process::Command::new("/bin/ps")
                    .args(["-o", "stat=", "-p", &child.to_string()])
                    .output()
                    .unwrap();
                let status = String::from_utf8_lossy(&status.stdout);
                if status.trim().is_empty() || status.trim().starts_with('Z') {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "child remains alive: {status}"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
