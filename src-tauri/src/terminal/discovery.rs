//! Successful discovery is process-local and shared; session locks never enter here.
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Condvar, Mutex},
    time::{Duration, Instant, SystemTime},
};

pub(super) const TOTAL_TIMEOUT: Duration = Duration::from_secs(4);
const STAGE_TIMEOUT: Duration = Duration::from_secs(3);
const MISSING: &str = "Codex CLI 또는 Claude Code를 설치한 뒤 새 에이전트 창을 여세요. 로그인과 권한 확인은 CLI 안내를 따르세요.";
const TIMEOUT: &str = "에이전트 탐색 시간이 초과되었습니다. 셸 시작 설정과 CLI 설치를 확인한 뒤 새 에이전트 창을 여세요.";

#[derive(Clone, Debug, PartialEq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}
fn stamp(path: &Path) -> Option<Stamp> {
    let m = path.metadata().ok()?;
    if !m.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if m.permissions().mode() & 0o111 == 0 {
            return None;
        }
        Some(Stamp {
            len: m.len(),
            modified: m.modified().ok(),
            identity: (m.dev(), m.ino(), m.ctime(), m.ctime_nsec()),
        })
    }
    #[cfg(not(unix))]
    {
        Some(Stamp {
            len: m.len(),
            modified: m.modified().ok(),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Found {
    pub program: String,
    pub executable: PathBuf,
    pub path: String,
    stamp: Stamp,
}
impl Found {
    fn valid(&self) -> bool {
        stamp(&self.executable).as_ref() == Some(&self.stamp)
    }
}
fn resolve(path: &str, name: &str) -> Option<Found> {
    std::env::split_paths(path)
        .filter(|p| p.is_absolute())
        .find_map(|dir| {
            let executable = dir.join(name);
            Some(Found {
                stamp: stamp(&executable)?,
                program: name.into(),
                executable,
                path: path.into(),
            })
        })
}

#[derive(Default)]
struct State {
    cached: Option<Found>,
    busy: bool,
    generation: u64,
    // Waiting callers share failures too; a later independent request may retry.
    last: Option<Result<Found, String>>,
}
#[derive(Default)]
pub(super) struct Discovery {
    state: Mutex<State>,
    ready: Condvar,
}
impl Discovery {
    pub fn get(&self, deadline: Instant) -> Result<Found, String> {
        self.get_with(deadline, || {
            discover(
                &std::env::var("PATH").unwrap_or_default(),
                &std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()),
                deadline,
                STAGE_TIMEOUT,
            )
        })
    }
    fn get_with(
        &self,
        deadline: Instant,
        run: impl FnOnce() -> Result<Found, String>,
    ) -> Result<Found, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        let generation = state.generation;
        loop {
            if let Some(found) = state.cached.as_ref().filter(|f| f.valid()) {
                return Ok(found.clone());
            }
            state.cached = None;
            if state.generation != generation {
                if let Some(Err(e)) = &state.last {
                    return Err(e.clone());
                }
            }
            if !state.busy {
                break;
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(TIMEOUT)?;
            state = self
                .ready
                .wait_timeout(state, remaining)
                .map_err(|e| e.to_string())?
                .0;
        }
        if Instant::now() >= deadline {
            return Err(TIMEOUT.into());
        }
        state.busy = true;
        drop(state);
        let result = run();
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.cached = result.as_ref().ok().cloned();
        state.last = Some(result.clone());
        state.generation = state.generation.wrapping_add(1);
        state.busy = false;
        self.ready.notify_all();
        result
    }
    pub fn spawn<T>(
        &self,
        deadline: Instant,
        spawn: impl FnMut(&Found) -> Result<T, String>,
    ) -> Result<(Found, T), String> {
        self.spawn_with(deadline, || self.get(deadline), spawn)
    }
    fn spawn_with<T>(
        &self,
        deadline: Instant,
        mut find: impl FnMut() -> Result<Found, String>,
        mut spawn: impl FnMut(&Found) -> Result<T, String>,
    ) -> Result<(Found, T), String> {
        for attempt in 0..2 {
            if Instant::now() >= deadline {
                return Err(TIMEOUT.into());
            }
            let found = find()?;
            if Instant::now() >= deadline {
                return Err(TIMEOUT.into());
            }
            match spawn(&found) {
                Ok(child) => return Ok((found, child)),
                Err(e) => {
                    self.invalidate(&found);
                    if attempt == 1 || Instant::now() >= deadline {
                        return Err(format!("에이전트를 시작하지 못했습니다: {e}"));
                    }
                }
            }
        }
        unreachable!()
    }
    pub fn invalidate(&self, failed: &Found) {
        let mut state = self.state.lock().unwrap();
        if state.cached.as_ref() == Some(failed) {
            state.cached = None;
        }
    }
}

// Reap every shell; kill its private group on timeout/error. A successful
// startup may intentionally launch a background service, as before this change.
struct Shell {
    child: Child,
    completed: bool,
}
impl Drop for Shell {
    fn drop(&mut self) {
        if !self.completed {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
fn shell_path(shell: &str, mode: &str, deadline: Instant) -> Result<String, String> {
    let mut file = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(shell);
    cmd.args([mode, "printf '\\nAGENT_GIBBON_PATH=%s\\n' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(file.try_clone().map_err(|e| e.to_string())?)
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = Shell {
        child: cmd.spawn().map_err(|e| e.to_string())?,
        completed: false,
    };
    loop {
        if Instant::now() >= deadline {
            return Err(TIMEOUT.into());
        }
        if let Some(status) = child.child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err("셸 PATH 탐색에 실패했습니다".into());
            }
            child.completed = true;
            break;
        }
        std::thread::sleep(
            Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut text = String::new();
    file.take(65536)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    text.lines()
        .rev()
        .find_map(|l| l.strip_prefix("AGENT_GIBBON_PATH="))
        .filter(|p| !p.is_empty())
        .map(String::from)
        .ok_or_else(|| "셸에서 PATH를 얻지 못했습니다".into())
}
fn discover(
    path: &str,
    shell: &str,
    deadline: Instant,
    stage_timeout: Duration,
) -> Result<Found, String> {
    if let Some(found) = resolve(path, "codex") {
        return Ok(found);
    }
    let mut fallback = resolve(path, "claude");
    let mut timed_out = false;
    for mode in ["-lc", "-ilc"] {
        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }
        match shell_path(shell, mode, deadline.min(Instant::now() + stage_timeout)) {
            Ok(path) => {
                if let Some(found) = resolve(&path, "codex") {
                    return Ok(found);
                }
                if fallback.is_none() {
                    fallback = resolve(&path, "claude");
                }
            }
            Err(e) => timed_out |= e == TIMEOUT,
        }
    }
    fallback.ok_or_else(|| {
        if timed_out {
            TIMEOUT.into()
        } else {
            MISSING.into()
        }
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Barrier,
        },
    };
    fn executable(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn run(path: &str, shell: &Path) -> Result<Found, String> {
        discover(
            path,
            shell.to_str().unwrap(),
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
    }
    #[test]
    fn current_codex_never_starts_a_shell() {
        let dir = tempfile::tempdir().unwrap();
        executable(dir.path(), "codex", "exit 0");
        let f = run(dir.path().to_str().unwrap(), Path::new("/not/a/shell")).unwrap();
        assert_eq!(f.program, "codex");
        assert!(resolve(".", "codex").is_none());
    }
    #[test]
    fn login_then_interactive_preserve_codex_priority_over_current_claude() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("current");
        let login = dir.path().join("login");
        let interactive = dir.path().join("interactive");
        for p in [&current, &login, &interactive] {
            std::fs::create_dir(p).unwrap();
        }
        executable(&current, "claude", "exit 0");
        executable(&interactive, "codex", "exit 0");
        let log = dir.path().join("calls");
        let shell=executable(dir.path(),"shell",&format!("echo \\\"$1\\\" >> '{}'\ncase \\\"$1\\\" in -lc) p='{}';; *) p='{}';; esac\nprintf '\\nAGENT_GIBBON_PATH=%s\\n' \\\"$p\\\"",log.display(),login.display(),interactive.display()).replace("\\\"","\""));
        let f = run(current.to_str().unwrap(), &shell).unwrap();
        assert_eq!(f.program, "codex");
        assert_eq!(f.path, interactive.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "-lc\n-ilc\n");
        executable(&login, "codex", "exit 0");
        std::fs::write(&log, "").unwrap();
        assert_eq!(run("", &shell).unwrap().path, login.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "-lc\n");
        std::fs::remove_file(login.join("codex")).unwrap();
        std::fs::remove_file(interactive.join("codex")).unwrap();
        assert_eq!(
            run(current.to_str().unwrap(), &shell).unwrap().program,
            "claude"
        );
        assert!(run("", &shell).unwrap_err().contains("설치"));
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn zshrc_only_path_with_missing_inherited_path() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        executable(&bin, "codex", "exit 0");
        std::fs::write(dir.path().join(".zshenv"), "PATH=/usr/bin:/bin\n").unwrap();
        std::fs::write(
            dir.path().join(".zshrc"),
            format!("export PATH='{}:/usr/bin:/bin'\n", bin.display()),
        )
        .unwrap();
        let shell = executable(
            dir.path(),
            "shell",
            &format!(
                "export ZDOTDIR='{}'\nexec /bin/zsh \\\"$@\\\"",
                dir.path().display()
            )
            .replace("\\\"", "\""),
        );
        assert_eq!(run("", &shell).unwrap().executable, bin.join("codex"));
    }
    #[test]
    fn successful_cache_is_shared_and_deleted_changed_or_failed_entries_are_invalidated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let exe = executable(dir.path(), "codex", "exit 0");
        let d = Discovery::default();
        let n = AtomicUsize::new(0);
        let get = || {
            d.get_with(Instant::now() + Duration::from_secs(1), || {
                n.fetch_add(1, Ordering::SeqCst);
                resolve(path, "codex").ok_or("missing".into())
            })
        };
        let first = get().unwrap();
        assert_eq!(get().unwrap(), first);
        assert_eq!(n.load(Ordering::SeqCst), 1);
        executable(dir.path(), "codex", "exit 22 # changed");
        let changed = get().unwrap();
        assert_ne!(changed, first);
        d.invalidate(&first);
        assert_eq!(get().unwrap(), changed); // stale failure cannot evict the new entry
        d.invalidate(&changed);
        assert!(get().is_ok());
        assert_eq!(n.load(Ordering::SeqCst), 3);
        std::fs::remove_file(exe).unwrap();
        assert!(get().is_err());
        assert_eq!(n.load(Ordering::SeqCst), 4);
        executable(dir.path(), "codex", "exit 0");
        assert!(get().is_ok());
    }
    #[test]
    fn simultaneous_windows_share_one_slow_discovery() {
        let dir = tempfile::tempdir().unwrap();
        executable(dir.path(), "codex", "exit 0");
        let found = resolve(dir.path().to_str().unwrap(), "codex").unwrap();
        let d = Arc::new(Discovery::default());
        let barrier = Arc::new(Barrier::new(5));
        let n = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let (d, b, n, f) = (d.clone(), barrier.clone(), n.clone(), found.clone());
                std::thread::spawn(move || {
                    b.wait();
                    d.get_with(Instant::now() + Duration::from_secs(2), || {
                        n.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(120));
                        Ok(f)
                    })
                    .unwrap()
                })
            })
            .collect();
        barrier.wait();
        for t in threads {
            assert_eq!(t.join().unwrap(), found);
        }
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn stage_and_total_timeouts_kill_shell_descendants_and_allow_claude() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let shell = executable(
            dir.path(),
            "shell",
            &format!("/bin/sleep 60 &\necho $! >> '{}'\nwait", pidfile.display()),
        );
        let start = Instant::now();
        let result = discover(
            "",
            shell.to_str().unwrap(),
            start + Duration::from_millis(1400),
            Duration::from_millis(900),
        );
        assert!(result.unwrap_err().contains("초과"));
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(50));
        for pid in std::fs::read_to_string(&pidfile).unwrap().lines() {
            let output = Command::new("/bin/ps")
                .args(["-o", "stat=", "-p", pid])
                .output()
                .unwrap();
            let status = String::from_utf8_lossy(&output.stdout);
            assert!(
                status.trim().is_empty() || status.trim().starts_with('Z'),
                "live descendant {pid}: {status}"
            );
        }
        executable(dir.path(), "claude", "exit 0");
        assert_eq!(
            discover(
                dir.path().to_str().unwrap(),
                shell.to_str().unwrap(),
                Instant::now() + Duration::from_millis(80),
                Duration::from_millis(40)
            )
            .unwrap()
            .program,
            "claude"
        );
    }
    #[test]
    fn shell_failure_and_bounded_output_do_not_break_fallback() {
        let dir = tempfile::tempdir().unwrap();
        executable(dir.path(), "claude", "exit 0");
        let shell = executable(dir.path(), "shell", "exit 9");
        assert_eq!(
            run(dir.path().to_str().unwrap(), &shell).unwrap().program,
            "claude"
        );
        executable(
            dir.path(),
            "shell",
            "/usr/bin/head -c 70000 /dev/zero; printf '\\nAGENT_GIBBON_PATH=/invalid\\n'",
        );
        assert_eq!(
            run(dir.path().to_str().unwrap(), &shell).unwrap().program,
            "claude"
        );
    }
    #[test]
    fn spawn_errors_rediscover_once_and_share_the_original_deadline() {
        let dir = tempfile::tempdir().unwrap();
        executable(dir.path(), "codex", "exit 0");
        let f = resolve(dir.path().to_str().unwrap(), "codex").unwrap();
        let d = Discovery::default();
        let mut finds = 0;
        let mut attempts = 0;
        let result = d
            .spawn_with(
                Instant::now() + Duration::from_secs(1),
                || {
                    finds += 1;
                    Ok(f.clone())
                },
                |_| {
                    attempts += 1;
                    if attempts == 1 {
                        Err("deleted between lookup and spawn".into())
                    } else {
                        Ok(42)
                    }
                },
            )
            .unwrap();
        assert_eq!(result.1, 42);
        assert_eq!((finds, attempts), (2, 2));
        let mut attempts = 0;
        let result: Result<(Found, ()), _> = d.spawn_with(
            Instant::now() + Duration::from_secs(1),
            || Ok(f.clone()),
            |_| {
                attempts += 1;
                Err("permission denied".into())
            },
        );
        assert!(result.unwrap_err().contains("permission denied"));
        assert_eq!(attempts, 2);
        let mut attempts = 0;
        let result: Result<(Found, ()), _> = d.spawn_with(
            Instant::now() + Duration::from_millis(20),
            || Ok(f.clone()),
            |_| {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(30));
                Err("failure".into())
            },
        );
        assert!(result.is_err());
        assert_eq!(attempts, 1);
    }
    #[test]
    fn discovery_and_cache_times_are_separate_from_cli_startup() {
        let dir = tempfile::tempdir().unwrap();
        executable(dir.path(), "codex", "/bin/sleep 1");
        let shell = executable(
            dir.path(),
            "shell",
            &format!(
                "/bin/sleep .15\nprintf '\\nAGENT_GIBBON_PATH={}\\n'",
                dir.path().display()
            ),
        );
        let d = Discovery::default();
        let first = Instant::now();
        d.get_with(first + Duration::from_secs(2), || run("", &shell))
            .unwrap();
        let cold = first.elapsed();
        let second = Instant::now();
        d.get_with(second + Duration::from_secs(2), || {
            panic!("cache must not run shell")
        })
        .unwrap();
        let warm = second.elapsed();
        eprintln!("discovery-only cold={cold:?} cache={warm:?}; CLI never spawned");
        assert!(cold >= Duration::from_millis(150));
        assert!(warm < Duration::from_millis(50));
    }
}
