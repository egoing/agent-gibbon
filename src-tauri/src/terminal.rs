//! Local PTYs, scoped to the calling agent window. No shell command or cwd from JS.
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, Manager, WebviewWindow};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub bounds: crate::capture::ImageNote,
}
struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    program: String,
}
impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(unix)]
        if self.child.try_wait().ok().flatten().is_none() {
            if let Some(pid) = self.child.process_id() {
                // portable-pty creates a new session; signal only our child group.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[derive(Default)]
pub struct Terminals {
    agents: Mutex<BTreeMap<String, Agent>>,
    sessions: Mutex<BTreeMap<String, Arc<Mutex<Session>>>>,
}
fn root(app: &tauri::AppHandle) -> PathBuf {
    app.state::<crate::storage::Store>().0.clone()
}
fn metadata(root: &Path) -> PathBuf {
    root.join(".agent-gibbon-terminals")
}
fn valid_id(id: &str) -> bool {
    id.strip_prefix("agent-")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        && id.len() <= 64
}
fn save(root: &Path, agent: &Agent) -> Result<(), String> {
    std::fs::create_dir_all(metadata(root)).map_err(err)?;
    let path = metadata(root).join(format!("{}.json", agent.id));
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(agent).map_err(err)?).map_err(err)?;
    std::fs::rename(tmp, path).map_err(err)
}
pub fn restore(app: &tauri::AppHandle) -> Result<(), String> {
    let path = metadata(&root(app));
    if !path.exists() {
        return Ok(());
    }
    let state = app.state::<Terminals>();
    let mut agents = state.agents.lock().map_err(err)?;
    for entry in std::fs::read_dir(path).map_err(err)? {
        let path = entry.map_err(err)?.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let agent: Agent =
            serde_json::from_slice(&std::fs::read(path).map_err(err)?).map_err(err)?;
        if !valid_id(&agent.id) {
            return Err("잘못된 에이전트 ID".into());
        }
        agents.insert(agent.id.clone(), agent);
    }
    Ok(())
}
pub fn entries(app: &tauri::AppHandle) -> Vec<Agent> {
    app.state::<Terminals>()
        .agents
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect()
}
pub fn contains(app: &tauri::AppHandle, id: &str) -> bool {
    app.state::<Terminals>()
        .agents
        .lock()
        .unwrap()
        .contains_key(id)
}
pub fn create(app: &tauri::AppHandle, bounds: crate::capture::ImageNote) -> Result<Value, String> {
    if !cfg!(target_os = "macos") {
        return Err("에이전트 터미널 MVP는 macOS에서만 지원합니다".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(err)?
        .as_nanos();
    let agent = Agent {
        id: format!("agent-{stamp}"),
        bounds,
    };
    let cwd = root(app).join(&agent.id);
    // Refuse existing paths; never adopt someone else's project directory.
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    std::fs::create_dir_all(root(app)).map_err(err)?;
    builder.create(&cwd).map_err(err)?;
    save(&root(app), &agent)?;
    app.state::<Terminals>()
        .agents
        .lock()
        .map_err(err)?
        .insert(agent.id.clone(), agent.clone());
    open(app, &agent.id)?;
    crate::tray::refresh(app)?;
    inspect(app, &agent.id)
}
pub fn open(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let agent = app
        .state::<Terminals>()
        .agents
        .lock()
        .map_err(err)?
        .get(id)
        .cloned()
        .ok_or("에이전트 창이 없습니다")?;
    let win = if let Some(win) = app.get_webview_window(id) {
        win
    } else {
        tauri::WebviewWindowBuilder::new(app, id, tauri::WebviewUrl::App("terminal.html".into()))
            .on_web_resource_request(|_, response| {
                // xterm generates cell/theme CSS at runtime. Scope this exception
                // to terminal WebViews; script policy and memo windows stay strict.
                if let Some(header) = response.headers_mut().get_mut("Content-Security-Policy") {
                    if let Ok(policy) = header.to_str() {
                        let mut directives: std::collections::HashMap<
                            String,
                            tauri::utils::config::CspDirectiveSources,
                        > = tauri::utils::config::Csp::Policy(policy.into()).into();
                        directives.insert(
                            "style-src".into(),
                            tauri::utils::config::CspDirectiveSources::List(vec![
                                "'self'".into(),
                                "'unsafe-inline'".into(),
                            ]),
                        );
                        if let Ok(value) = tauri::utils::config::Csp::from(directives)
                            .to_string()
                            .parse()
                        {
                            *header = value;
                        }
                    }
                }
            })
            .title(format!("{} — {id}", crate::instance::current().title()))
            .decorations(false)
            .resizable(true)
            .min_inner_size(2.0, 2.0)
            .inner_size(agent.bounds.width, agent.bounds.height)
            .position(agent.bounds.x, agent.bounds.y)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
            .build()
            .map_err(err)?
    };
    // User explicitly opens a terminal, but background CLI/tray restore preserves focus.
    #[cfg(target_os = "macos")]
    unsafe {
        (&*(win.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow))
            .orderFrontRegardless();
    }
    #[cfg(not(target_os = "macos"))]
    win.show().map_err(err)?;
    Ok(())
}
pub fn inspect(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    if !contains(app, id) {
        return Err("에이전트 창이 없습니다".into());
    }
    let win = app.get_webview_window(id);
    let session = app
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(id)
        .cloned();
    let (pid, program, size, running) = if let Some(session) = session {
        let mut s = session.lock().map_err(err)?;
        (
            s.child.process_id(),
            Some(s.program.clone()),
            Some(s.master.get_size().map_err(err)?),
            s.child.try_wait().map_err(err)?.is_none(),
        )
    } else {
        (None, None, None, false)
    };
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":"agent","title":"에이전트 터미널","cwd":root(app).join(id),"pid":pid,"program":program,"running":running,"pty_size":size.map(|s|json!({"cols":s.cols,"rows":s.rows})),"visible":win.as_ref().is_some_and(|w|w.is_visible().unwrap_or(false)),"position":win.as_ref().and_then(|w|w.outer_position().ok()),"size":win.as_ref().and_then(|w|w.inner_size().ok())}),
    )
}
pub fn handle(app: &tauri::AppHandle, request: &crate::Request) -> Result<Value, String> {
    let id = request.id();
    match request.action {
        crate::Action::Open => open(app, id)?,
        crate::Action::Close => {
            if let Some(win) = app.get_webview_window(id) {
                win.hide().map_err(err)?;
            }
        }
        crate::Action::Inspect => (),
        _ => return Err("에이전트 창은 내용 갱신을 지원하지 않습니다".into()),
    }
    inspect(app, id)
}
fn caller(window: &WebviewWindow) -> Result<String, String> {
    let id = window.label();
    if !contains(window.app_handle(), id) {
        return Err("에이전트 창이 아닙니다".into());
    }
    Ok(id.into())
}
// Bound login-shell PATH discovery. Redirect output to a file so large startup
// output cannot deadlock a pipe; only accept a marked, bounded PATH line.
fn shell_path() -> Result<String, String> {
    let fallback = std::env::var("PATH").unwrap_or_default();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let file = tempfile::tempfile().map_err(err)?;
    let output = file.try_clone().map_err(err)?;
    let mut cmd = std::process::Command::new(shell);
    cmd.args(["-ilc", "printf '\\nAGENT_GIBBON_PATH=%s\\n' \"$PATH\""])
        .stdin(std::process::Stdio::null())
        .stdout(output)
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn().map_err(err)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if child.try_wait().map_err(err)?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Ok(fallback);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut text = String::new();
    use std::io::{Seek, SeekFrom};
    let mut file = file;
    file.seek(SeekFrom::Start(0)).map_err(err)?;
    file.take(65536).read_to_string(&mut text).map_err(err)?;
    Ok(text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("AGENT_GIBBON_PATH="))
        .filter(|p| !p.is_empty())
        .unwrap_or(&fallback)
        .into())
}
fn resolve(path: &str) -> Option<(String, PathBuf)> {
    for name in ["codex", "claude"] {
        for dir in std::env::split_paths(path).filter(|p| p.is_absolute()) {
            let candidate = dir.join(name);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if candidate
                    .metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                {
                    return Some((name.into(), candidate));
                }
            }
        }
    }
    None
}
fn size(cols: u16, rows: u16) -> Result<PtySize, String> {
    if !(2..=1000).contains(&cols) || !(1..=1000).contains(&rows) {
        return Err("터미널 크기 범위를 벗어났습니다".into());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}
#[tauri::command]
pub async fn terminal_start(
    window: WebviewWindow,
    cols: u16,
    rows: u16,
    output: Channel<Vec<u8>>,
) -> Result<Value, String> {
    let id = caller(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dimensions = size(cols, rows)?;
        let state = app.state::<Terminals>();
        // Serialize creation, including duplicate frontend requests.
        let mut sessions = state.sessions.lock().map_err(err)?;
        if sessions.contains_key(&id) { return Err("이미 시작된 세션입니다. 창을 숨겼다가 복원하세요".into()); }
        let path = shell_path()?;
        let (program, executable) = resolve(&path).ok_or("Codex CLI 또는 Claude Code를 설치한 뒤 새 에이전트 창을 여세요. 로그인과 권한 확인은 CLI 안내를 따르세요.")?;
        let pair = native_pty_system().openpty(dimensions).map_err(err)?;
        let mut command = CommandBuilder::new(executable);
        command.cwd(root(&app).join(&id));
        command.env("PATH",path); command.env("TERM","xterm-256color");
        command.env_remove("NO_COLOR");
        let child = pair.slave.spawn_command(command).map_err(err)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(err)?;
        let writer = pair.master.take_writer().map_err(err)?;
        sessions.insert(id.clone(),Arc::new(Mutex::new(Session { master:pair.master, writer, child, program })));
        // Raw bytes preserve UTF-8 across arbitrary PTY read boundaries.
        std::thread::spawn(move || {
            let mut bytes = [0u8;8192];
            loop { match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(n) => { if output.send(bytes[..n].to_vec()).is_err() { break; } }
            }}
            let _ = output.send(b"\r\n[PTY \xec\x84\xb8\xec\x85\x98 \xec\xa2\x85\xeb\xa3\x8c]\r\n".to_vec());
        });
        drop(sessions);
        inspect(&app,&id)
    }).await.map_err(err)?
}
fn session(window: &WebviewWindow) -> Result<Arc<Mutex<Session>>, String> {
    let id = caller(window)?;
    window
        .state::<Terminals>()
        .sessions
        .lock()
        .map_err(err)?
        .get(&id)
        .cloned()
        .ok_or("실행 중인 세션이 없습니다".into())
}
#[tauri::command]
pub fn terminal_write(window: WebviewWindow, data: String) -> Result<(), String> {
    if data.len() > 65536 {
        return Err("입력은 64KiB 이하여야 합니다".into());
    }
    let session = session(&window)?;
    let mut session = session.lock().map_err(err)?;
    session.writer.write_all(data.as_bytes()).map_err(err)?;
    session.writer.flush().map_err(err)
}
#[tauri::command]
pub fn terminal_resize(window: WebviewWindow, cols: u16, rows: u16) -> Result<(), String> {
    session(&window)?
        .lock()
        .map_err(err)?
        .master
        .resize(size(cols, rows)?)
        .map_err(err)
}
#[tauri::command]
pub fn terminal_hide(window: WebviewWindow) -> Result<(), String> {
    caller(&window)?;
    window.hide().map_err(err)
}
#[tauri::command]
pub fn terminal_info(window: WebviewWindow) -> Result<Value, String> {
    let id = caller(&window)?;
    inspect(window.app_handle(), &id)
}
pub fn geometry(window: &tauri::Window) {
    let app = window.app_handle();
    let state = app.state::<Terminals>();
    let Ok(mut agents) = state.agents.lock() else {
        return;
    };
    if let Some(agent) = agents.get_mut(window.label()) {
        if let (Ok(p), Ok(s), Ok(scale)) = (
            window.outer_position(),
            window.inner_size(),
            window.scale_factor(),
        ) {
            agent.bounds = crate::capture::ImageNote {
                x: f64::from(p.x) / scale,
                y: f64::from(p.y) / scale,
                width: f64::from(s.width) / scale,
                height: f64::from(s.height) / scale,
            };
            let _ = save(&root(app), agent);
        }
    }
}
pub fn shutdown(app: &tauri::AppHandle) {
    app.state::<Terminals>().sessions.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_priority_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(resolve(dir.path().to_str().unwrap()).is_none());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for name in ["claude", "codex"] {
                let p = dir.path().join(name);
                std::fs::write(&p, "#!/bin/sh\n").unwrap();
                std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            assert_eq!(resolve(dir.path().to_str().unwrap()).unwrap().0, "codex");
            std::fs::remove_file(dir.path().join("codex")).unwrap();
            assert_eq!(resolve(dir.path().to_str().unwrap()).unwrap().0, "claude");
        }
        assert!(resolve(".").is_none());
        assert!(!valid_id("agent-../x"));
        assert!(valid_id("agent-123"));
        assert!(size(0, 24).is_err());
    }
    #[test]
    fn metadata_does_not_claim_project_index() {
        let dir = tempfile::tempdir().unwrap();
        let a = Agent {
            id: "agent-123".into(),
            bounds: crate::capture::ImageNote {
                x: 1.,
                y: 2.,
                width: 640.,
                height: 400.,
            },
        };
        std::fs::create_dir(dir.path().join(&a.id)).unwrap();
        let index = dir.path().join(&a.id).join("index.md");
        std::fs::write(&index, "user project").unwrap();
        save(dir.path(), &a).unwrap();
        assert_eq!(std::fs::read_to_string(index).unwrap(), "user project");
    }
}
