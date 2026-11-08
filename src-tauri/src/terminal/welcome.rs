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
        last_command: None,
    };
    save(&root(app), &agent)?;
    state.records.lock().map_err(err)?.insert(id.clone(), agent);
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
    let restart_notice = super::restart_notice(window.app_handle(), &id)?;
    Ok(
        json!({"info":info,"choices":options,"folders":folders,"location":folder_value(&selected,&home),"home":home,"restart_notice":restart_notice}),
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
            set_path(&app, &id, PathBuf::from(path))?;
        } else if let Some(path) = app
            .dialog()
            .file()
            .set_title("작업 폴더 선택")
            .blocking_pick_folder()
        {
            selected = true;
            let path = path.into_path().map_err(err)?;
            set_path(&app, &id, path)?;
        }
        let mut info = inspect(&app, &id)?;
        info["folder_selected"] = json!(selected);
        Ok(info)
    })
    .await
    .map_err(err)?
}
pub(super) fn set_path(app: &tauri::AppHandle, id: &str, path: PathBuf) -> Result<(), String> {
    if !path.is_absolute() || !path.is_dir() {
        return Err("사용 가능한 폴더가 아닙니다".into());
    }
    let state = app.state::<Terminals>();
    let mut agents = state.records.lock().map_err(err)?;
    let agent = agents.get_mut(id).ok_or("터미널 창이 없습니다")?;
    if agent.kind != Kind::Terminal {
        return Ok(());
    }
    if agent.last_path.as_ref() != Some(&path) {
        remember_path(agent, path, now());
        save(&root(app), agent)?;
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
        let mut agents = state.records.lock().map_err(err)?;
        let a = agents.get_mut(&id).ok_or("터미널 창이 없습니다")?;
        a.title = title.trim().into();
        a.updated_at = now();
        save(&root(window.app_handle()), a)?;
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
pub(super) fn refresh_path(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<Terminals>();
    if state
        .records
        .lock()
        .map_err(err)?
        .get(id)
        .is_none_or(|a| a.kind != Kind::Terminal)
    {
        return Ok(());
    }
    let session = state
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .and_then(running);
    if let Some(session) = session {
        let mut s = session.lock().map_err(err)?;
        if s.child.try_wait().map_err(err)?.is_none() {
            if let Some(path) = s.child.process_id().and_then(process_cwd) {
                drop(s);
                set_path(app, id, path)?;
            }
        }
    }
    Ok(())
}
pub(super) fn start(
    agent: &Terminal,
    choice: &str,
    dimensions: PtySize,
) -> Result<(Session, Box<dyn Read + Send>), String> {
    let prepare = crate::telemetry::Timing::new("terminal.command.prepare");
    let cwd = agent.last_path.clone().map(Ok).unwrap_or_else(home)?;
    if !cwd.is_dir() {
        return Err("마지막 폴더가 없습니다. 다른 폴더를 선택하세요".into());
    }
    let line = command(&search_path(), choice)?;
    let shell = default_shell()?;
    drop(prepare);
    start_shell(&cwd, line, dimensions, &shell)
}
fn start_shell(
    cwd: &Path,
    line: Option<String>,
    dimensions: PtySize,
    shell: &Path,
) -> Result<(Session, Box<dyn Read + Send>), String> {
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
        let prompt_script = "function global:prompt { [Console]::Write(([char]27 + ']777;gibbon-cwd;' + (Get-Location).Path + [char]7)); [Console]::Write(([char]27 + ']777;gibbon-ready;TOKEN' + [char]7)); 'PS> ' }; ";
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
    let mut session = Session {
        master: pair.master,
        writer: Box::new(std::io::sink()),
        child: OwnedChild(Some(child)),
        program: "shell".into(),
        prompt,
    };
    let _attach = crate::telemetry::Timing::new("terminal.io.attach");
    let reader = session.master.try_clone_reader().map_err(err)?;
    session.writer = session.master.take_writer().map_err(err)?;
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
        return false;
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
    refresh_path(app, &id)?;
    let state = app.state::<Terminals>();
    let sessions = state.sessions.lock().map_err(err)?;
    let expected = close_identity(sessions.get(&id));
    let session = sessions.get(&id).and_then(running);
    drop(sessions);
    let needs_confirm = session
        .as_ref()
        .is_some_and(|s| busy(&mut s.lock().unwrap()));
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
fn close_identity(slot: Option<&Slot>) -> Option<(bool, usize)> {
    slot.map(|slot| match slot {
        Slot::Starting(token) => (false, *token as usize),
        Slot::Running(session) => (true, Arc::as_ptr(session) as usize),
    })
}
fn finish_close(
    app: &tauri::AppHandle,
    id: &str,
    expected: Option<(bool, usize)>,
) -> Result<(), String> {
    let state = app.state::<Terminals>();
    refresh_path(app, id)?;
    let mut sessions = state.sessions.lock().map_err(err)?;
    if close_identity(sessions.get(id)) != expected {
        return Ok(());
    }
    if let Some(window) = app.get_webview_window(id) {
        crate::stash::release(&window)?;
        window.destroy().map_err(err)?;
    }
    sessions.remove(id);
    drop(sessions);
    crate::tray::refresh(app)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            last_command: None,
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
}
impl DirectoryReport {
    pub(super) fn take_prompt(&mut self, token: &str) -> bool {
        self.prompt
            .take()
            .is_some_and(|value| !token.is_empty() && value == token)
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
                if let Some(raw) = self.frame.strip_prefix(b"]777;gibbon-ready;") {
                    self.prompt = std::str::from_utf8(raw).ok().map(str::to_owned);
                }
                self.frame.clear();
                self.escape = false;
            } else if self.frame.len() < 32768 {
                self.frame.push(byte);
                let prefixes: [&[u8]; 2] = [b"]777;gibbon-cwd;", b"]777;gibbon-ready;"];
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
    if state.sessions.lock().map_err(err)?.contains_key(id) {
        return Err("먼저 터미널 창을 종료하세요".into());
    }
    if !state.records.lock().map_err(err)?.contains_key(id) {
        return Err("터미널이 없습니다".into());
    }
    if let Some(w) = app.get_webview_window(id) {
        crate::stash::release(&w)?;
        w.destroy().map_err(err)?;
    }
    std::fs::remove_file(metadata(&root(app)).join(format!("{id}.json"))).map_err(err)?;
    state.records.lock().map_err(err)?.remove(id);
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
    fn new() -> Self {
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
pub(super) fn touch_path(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<Terminals>();
    let mut agents = state.records.lock().map_err(err)?;
    if let Some(agent) = agents
        .get_mut(id)
        .filter(|a| a.kind == Kind::Terminal && a.last_path.is_some())
    {
        agent.path_updated = now();
        save(&root(app), agent)?;
    }
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
        assert_ne!(
            close_identity(Some(&Slot::Starting(1))),
            close_identity(Some(&Slot::Starting(2)))
        );
        assert_ne!(
            close_identity(None),
            close_identity(Some(&Slot::Starting(1)))
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
        for child in children {
            // A reparented zombie may remain until launchd/init reaps it; it must not run.
            let status = std::process::Command::new("/bin/ps")
                .args(["-o", "stat=", "-p", &child.to_string()])
                .output()
                .unwrap();
            let status = String::from_utf8_lossy(&status.stdout);
            assert!(
                status.trim().is_empty() || status.trim().starts_with('Z'),
                "child remains alive: {status}"
            );
        }
    }
}
