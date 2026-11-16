//! Directly exec this test binary inside an owned PTY with an empty environment.
//! The fixture never sources a shell profile or reads a user's terminal.
use super::*;
use portable_pty::{Child, ChildKiller, ExitStatus};
use std::{io, sync::atomic::AtomicUsize, thread};

const FIXTURE: &str = "terminal::lifecycle_tests::owned_fixture_child";
const MODE: &str = "GIBBON_OWNED_PTY_TEST";
const TRAILER: &[u8] = b"end-of-owned-output";

#[test]
fn owned_fixture_child() {
    let Ok(mode) = std::env::var(MODE) else {
        return;
    };
    match mode.as_str() {
        "tree-root" | "tree-orphan" | "tree-retired" => {
            let orphan = mode != "tree-root";
            let close_stdio = mode == "tree-retired";
            unsafe {
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                libc::signal(libc::SIGTTOU, libc::SIG_IGN);
            }
            let mut jobs = Vec::new();
            for _ in 0..2 {
                let pid = unsafe { libc::fork() };
                assert!(pid >= 0);
                if pid == 0 {
                    unsafe {
                        libc::setpgid(0, 0);
                        if close_stdio {
                            for fd in 0..3 {
                                libc::close(fd);
                            }
                        }
                        libc::usleep(4_000_000);
                        libc::_exit(0);
                    }
                }
                assert_eq!(unsafe { libc::setpgid(pid, pid) }, 0);
                jobs.push(pid);
            }
            if !orphan {
                assert_eq!(unsafe { libc::tcsetpgrp(0, jobs[0]) }, 0);
            }
            std::fs::write("owned-jobs", format!("{} {}", jobs[0], jobs[1])).unwrap();
            if !orphan {
                thread::sleep(Duration::from_secs(5));
            }
        }
        "no-input" => {
            let mut tty = unsafe { std::mem::zeroed() };
            assert_eq!(unsafe { libc::tcgetattr(0, &mut tty) }, 0);
            unsafe { libc::cfmakeraw(&mut tty) };
            assert_eq!(unsafe { libc::tcsetattr(0, libc::TCSANOW, &tty) }, 0);
            std::fs::write("ready", b"").unwrap();
            thread::sleep(Duration::from_secs(5));
        }
        "wait" => thread::sleep(Duration::from_secs(12)),
        "close-stdio" => unsafe {
            for fd in 0..3 {
                libc::close(fd);
            }
            thread::sleep(Duration::from_millis(400));
        },
        "background" => unsafe {
            // The grandchild uses only async-signal-safe calls after fork.
            libc::signal(libc::SIGHUP, libc::SIG_IGN);
            let pid = libc::fork();
            assert!(pid >= 0);
            if pid == 0 {
                libc::usleep(600_000);
                libc::write(1, TRAILER.as_ptr().cast(), TRAILER.len());
                libc::_exit(0);
            }
        },
        "flood" => {
            for _ in 0..32 {
                io::stdout().write_all(&[b'x'; 8192]).unwrap();
            }
            io::stdout().write_all(TRAILER).unwrap();
            io::stdout().flush().unwrap();
        }
        "error" => unsafe { libc::_exit(23) },
        "signal" => unsafe {
            libc::raise(libc::SIGTERM);
        },
        _ => panic!("unknown fixture mode"),
    }
    unsafe { libc::_exit(0) }
}

#[derive(Debug)]
struct Spy {
    kills: Arc<AtomicUsize>,
    result: io::Result<Option<ExitStatus>>,
}

impl ChildKiller for Spy {
    fn kill(&mut self) -> io::Result<()> {
        self.kills.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        panic!("unowned killer")
    }
}

impl Child for Spy {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.result
            .as_ref()
            .cloned()
            .map_err(|e| io::Error::from(e.kind()))
    }
    fn wait(&mut self) -> io::Result<ExitStatus> {
        Ok(ExitStatus::with_exit_code(0))
    }
    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[test]
fn only_a_verified_live_child_receives_a_signal() {
    for (status, expected) in [
        (Ok(None), 1),
        (Ok(Some(ExitStatus::with_exit_code(1))), 0),
        (Err(io::Error::from_raw_os_error(libc::ECHILD)), 0),
    ] {
        let kills = Arc::new(AtomicUsize::new(0));
        let child = OwnedChild::new(Box::new(Spy {
            kills: kills.clone(),
            result: status,
        }));
        let _ = child.try_wait();
        let (tx, rx) = mpsc::channel();
        tx.send(Event::Stop).unwrap();
        lifecycle::supervise(child, rx, |_, _| panic!("cancel is not natural exit"));
        assert_eq!(kills.load(Ordering::SeqCst), expected);
    }
}

struct Fixture {
    child: OwnedChild,
    pid: i32,
    token: u64,
    done: mpsc::Receiver<bool>,
    output_done: mpsc::Receiver<()>,
    tail: Arc<Mutex<Vec<u8>>>,
    bytes: Arc<AtomicUsize>,
}

fn fixture(state: &Arc<Terminals>, cwd: &Path, id: &str, mode: &str, disconnect: bool) -> Fixture {
    let mut start = Start::reserve(state, id).unwrap();
    let token = start.token;
    let events = start.events.take().unwrap();
    let pair = native_pty_system().openpty(size(80, 24).unwrap()).unwrap();
    let mut cmd = CommandBuilder::new(std::env::current_exe().unwrap());
    cmd.env_clear();
    if matches!(mode, "background" | "tree-orphan") {
        cmd.set_controlling_tty(false);
    }
    cmd.env(MODE, mode);
    cmd.args(["--exact", FIXTURE, "--nocapture", "--test-threads=1"]);
    cmd.cwd(cwd);
    let child = OwnedChild::new(pair.slave.spawn_command(cmd).unwrap());
    drop(pair.slave);
    let pid = child.process_id().unwrap() as i32;
    let mut reader = PtyReader::new(&*pair.master).unwrap();
    let writer = pair.master.take_writer().unwrap();
    let session = Session {
        master: pair.master,
        writer,
        child: child.clone(),
        program: "fixture".into(),
        prompt: welcome::PromptState::new(),
        observed_command: false,
    };
    let lifecycle_state = state.clone();
    let watched_id = id.to_string();
    let watched_child = child.clone();
    let (done, finished) = mpsc::channel();
    let worker = state.workers.enter();
    thread::spawn(move || {
        let _worker = worker;
        lifecycle::supervise(watched_child, events, |status, jobs| {
            let applied = lifecycle_state.mark_exited(&watched_id, token, status, jobs);
            let _ = done.send(applied);
        });
    });
    start.finish(session).unwrap();
    let tail = Arc::new(Mutex::new(Vec::new()));
    let captured = tail.clone();
    let bytes = Arc::new(AtomicUsize::new(0));
    let total = bytes.clone();
    let cancelled = start.cancelled.clone();
    let worker = state.workers.enter();
    let (end, output_done) = mpsc::channel();
    let pump = thread::spawn(move || {
        let _worker = worker;
        let (dead_channel, receiver) = mpsc::channel::<Vec<u8>>();
        drop(receiver);
        reader.drain(&cancelled, |buf| {
            total.fetch_add(buf.len(), Ordering::Relaxed);
            if disconnect {
                assert!(dead_channel.send(buf.to_vec()).is_err());
            } else {
                let mut tail = captured.lock().unwrap();
                tail.extend_from_slice(buf);
                let trim = tail.len().saturating_sub(TRAILER.len());
                tail.drain(..trim);
            }
            if buf.len() >= 2048 {
                thread::sleep(Duration::from_millis(1));
            }
        });
        let _ = end.send(());
    });
    start.sender.send(Event::Reader(pump)).unwrap();
    Fixture {
        child,
        pid,
        token,
        done: finished,
        output_done,
        tail,
        bytes,
    }
}

fn reaped(pid: i32) {
    let rc = unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) };
    assert_eq!(rc, -1);
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn owned_pty_drains_output_before_retiring_its_generation() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Terminals::default());
    let id = "terminal-1";
    for (mode, code, disconnect) in [
        ("error", 23, false),
        ("signal", 1, false),
        ("close-stdio", 0, false),
        ("background", 0, false),
        ("flood", 0, false),
        ("flood", 0, true),
    ] {
        let fixture = fixture(&state, dir.path(), id, mode, disconnect);
        if mode == "close-stdio" {
            fixture
                .output_done
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            assert!(
                fixture.child.try_wait().unwrap().is_none(),
                "EOF killed a live root or failed to arrive first"
            );
        }
        if mode == "background" {
            let deadline = Instant::now() + Duration::from_secs(3);
            while fixture.child.process_id().is_some() {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(10));
            }
            assert!(matches!(
                state.sessions.lock().unwrap().get(id),
                Some(Slot::Running { .. })
            ));
        }
        assert!(fixture.done.recv_timeout(Duration::from_secs(5)).unwrap());
        reaped(fixture.pid);
        if mode == "background" || mode == "flood" && !disconnect {
            assert_eq!(*fixture.tail.lock().unwrap(), TRAILER);
        }
        if mode == "flood" {
            assert!(fixture.bytes.load(Ordering::Relaxed) >= 32 * 8192);
        }
        assert!(
            matches!(state.sessions.lock().unwrap().get(id), Some(Slot::Exited {status,..}) if status.exit_code()==code && status.signal().is_some()==(mode=="signal"))
        );
        assert!(Start::reserve(&state, id).is_err());
        state.sessions.lock().unwrap().remove(id);
        let _next = Start::reserve(&state, id).unwrap();
        assert!(!state.mark_exited(
            id,
            fixture.token,
            ExitStatus::with_exit_code(code),
            OwnedJobs::default()
        ));
    }
}

#[cfg(target_os = "macos")]
fn alive(pid: i32) -> bool {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = size_of_val(&info) as i32;
    unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        ) == size
            && info.pbi_status != libc::SZOMB
    }
}

fn wait_for(condition: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(2);
    while !condition() {
        assert!(Instant::now() < until, "fixture condition timed out");
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn quit_reclaims_confirmed_jobs_in_separate_groups_and_preserves_sentinel() {
    for mode in ["tree-root", "tree-orphan", "tree-retired"] {
        let dir = tempfile::tempdir().unwrap();
        let sentinel_state = Arc::new(Terminals::default());
        let sentinel = fixture(&sentinel_state, dir.path(), "terminal-2", "wait", true);
        let state = Arc::new(Terminals::default());
        let root = fixture(&state, dir.path(), "terminal-1", mode, true);
        let file = dir.path().join("owned-jobs");
        wait_for(|| {
            std::fs::read_to_string(&file).is_ok_and(|s| s.split_whitespace().count() == 2)
        });
        let jobs = std::fs::read_to_string(&file)
            .unwrap()
            .split_whitespace()
            .map(|p| p.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        assert!(jobs.iter().all(|p| alive(*p)));
        if mode == "tree-orphan" {
            wait_for(|| root.child.process_id().is_none());
        }
        if mode == "tree-retired" {
            assert!(root.done.recv_timeout(Duration::from_secs(2)).unwrap());
            assert!(
                jobs.iter().all(|p| alive(*p)),
                "natural retirement signalled a live job"
            );
            assert!(
                matches!(state.sessions.lock().unwrap().get("terminal-1"), Some(Slot::Exited {jobs, ..}) if jobs.may_have_jobs())
            );
        }
        let state2 = state.clone();
        let elapsed = Instant::now();
        assert!(bounded_shutdown(move |deadline| {
            state2.stop_all(|_, _| {});
            state2.workers.wait_until(deadline)
        }));
        assert!(elapsed.elapsed() < Duration::from_secs(1));
        reaped(root.pid);
        wait_for(|| jobs.iter().all(|p| !alive(*p)));
        assert!(alive(sentinel.pid));
        eprintln!("case={mode} runner={} root={} jobs={jobs:?} sentinel={} root_reaped=true jobs_stopped=true sentinel_alive=true", std::process::id(), root.pid, sentinel.pid);
        sentinel_state.stop_all(|_, _| {});
        assert!(sentinel_state
            .workers
            .wait_until(Instant::now() + Duration::from_secs(2)));
        reaped(sentinel.pid);
    }
}

#[cfg(target_os = "macos")]
fn resources() -> (usize, i32, u64) {
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = size_of_val(&info) as i32;
    assert_eq!(
        unsafe {
            libc::proc_pidinfo(
                libc::getpid(),
                libc::PROC_PIDTASKINFO,
                0,
                (&mut info as *mut libc::proc_taskinfo).cast(),
                size,
            )
        },
        size
    );
    (
        std::fs::read_dir("/dev/fd").unwrap().count(),
        info.pti_threadnum,
        info.pti_resident_size,
    )
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "single test process for FD, thread and RSS measurements"]
fn repeated_owned_sessions_release_handles_and_workers() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Terminals::default());
    let mut samples = Vec::new();
    for batch in 0..4 {
        for i in 0..12 {
            let id = format!("terminal-{}", batch * 12 + i);
            let f = fixture(&state, dir.path(), &id, "error", true);
            assert!(f.done.recv_timeout(Duration::from_secs(3)).unwrap());
            reaped(f.pid);
        }
        assert!(state
            .workers
            .wait_until(Instant::now() + Duration::from_secs(2)));
        thread::sleep(Duration::from_millis(180));
        samples.push(resources());
    }
    eprintln!("owned PTY batches (12,24,36,48): {samples:?}");
    assert!(samples
        .iter()
        .all(|s| s.0 == samples[0].0 && s.1 == samples[0].1));
    assert!(samples[3].2 <= samples[0].2 + 4 * 1024 * 1024);
    assert_eq!(state.sessions.lock().unwrap().len(), 48);
    assert!(state
        .sessions
        .lock()
        .unwrap()
        .values()
        .all(|s| matches!(s, Slot::Exited {status, ..} if status.exit_code()==23)));
}

#[test]
fn shutdown_budget_is_shared_and_writer_cannot_hide_owned_children() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Terminals::default());
    let mut pids = Vec::new();
    for n in 0..8 {
        let f = fixture(&state, dir.path(), &format!("terminal-{n}"), "wait", true);
        pids.push(f.pid);
    }
    let locked = state
        .sessions
        .lock()
        .unwrap()
        .get("terminal-0")
        .and_then(running)
        .unwrap();
    let _writer = locked.lock().unwrap();
    let mut starting = Start::reserve(&state, "terminal-9").unwrap();
    let events = starting.events.take().unwrap();
    state.stopping.store(true, Ordering::Release);
    let state2 = state.clone();
    let start = Instant::now();
    // Deliberately retain one Starting ticket: timeout must exclude it from
    // completion without delaying any other child's termination request.
    assert!(!bounded_shutdown(move |deadline| {
        state2.stop_all(|_, _| {});
        state2.workers.wait_until(deadline)
    }));
    let elapsed = start.elapsed();
    eprintln!(
        "8 live roots + held session lock + held Starting ticket: {} ms",
        elapsed.as_millis()
    );
    assert!(elapsed >= QUIT_BUDGET && elapsed < QUIT_BUDGET + Duration::from_millis(250));
    assert!(matches!(events.try_recv(), Ok(Event::Stop)));
    for pid in pids {
        reaped(pid);
    }
    assert!(Start::reserve(&state, "terminal-10").is_err());
    drop(starting);
    assert!(state
        .workers
        .wait_until(Instant::now() + Duration::from_secs(1)));
}

#[test]
fn slow_final_metadata_cannot_delay_signalling_the_direct_child() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Terminals::default());
    let f = fixture(&state, dir.path(), "terminal-1", "wait", true);
    let (release, released) = mpsc::channel::<()>();
    let released = Mutex::new(released);
    let state2 = state.clone();
    assert!(!bounded_shutdown(move |deadline| {
        state2.stop_all(move |_, _| {
            let _ = released
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3));
        });
        state2.workers.wait_until(deadline)
    }));
    // Saving is still blocked, but the direct child has already been reaped.
    reaped(f.pid);
    release.send(()).unwrap();
    assert!(state
        .workers
        .wait_until(Instant::now() + Duration::from_secs(2)));
}

#[test]
fn a_full_pty_input_queue_does_not_block_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Terminals::default());
    let f = fixture(&state, dir.path(), "terminal-1", "no-input", true);
    wait_for(|| dir.path().join("ready").exists());
    let session = state
        .sessions
        .lock()
        .unwrap()
        .get("terminal-1")
        .and_then(running)
        .unwrap();
    let (entered, writing) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    let writer = thread::spawn(move || {
        let mut session = session.lock().unwrap();
        entered.send(()).unwrap();
        let result = session.writer.write_all(&vec![0; 1024 * 1024]);
        let _ = finished.send(result.is_err());
    });
    writing.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        done.recv_timeout(Duration::from_millis(80)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    let state2 = state.clone();
    assert!(bounded_shutdown(move |deadline| {
        state2.stop_all(|_, s| assert!(s.is_none()));
        state2.workers.wait_until(deadline)
    }));
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    writer.join().unwrap();
    reaped(f.pid);
}

#[test]
fn update_stays_blocked_after_close_until_the_reaper_finishes() {
    #[derive(Debug)]
    struct SlowChild {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }
    impl ChildKiller for SlowChild {
        fn kill(&mut self) -> io::Result<()> {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            Ok(())
        }
        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            panic!("unexpected clone")
        }
    }
    impl Child for SlowChild {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            Ok(None)
        }
        fn wait(&mut self) -> io::Result<ExitStatus> {
            Ok(ExitStatus::with_exit_code(0))
        }
        fn process_id(&self) -> Option<u32> {
            None
        }
    }
    let state = Arc::new(Terminals::default());
    let mut start = Start::reserve(&state, "terminal-1").unwrap();
    let (entered, killing) = mpsc::channel();
    let (release, continue_kill) = mpsc::channel();
    let child = OwnedChild::new(Box::new(SlowChild {
        entered,
        release: Mutex::new(continue_kill),
    }));
    let receiver = start.events.take().unwrap();
    let watch = child.clone();
    let ticket = state.workers.enter();
    let reaper = thread::spawn(move || {
        let _ticket = ticket;
        lifecycle::supervise(watch, receiver, |_, _| panic!("close must terminate"));
    });
    let pair = native_pty_system().openpty(size(80, 24).unwrap()).unwrap();
    let session = Session {
        master: pair.master,
        writer: Box::new(io::sink()),
        child,
        program: "fixture".into(),
        prompt: welcome::PromptState::new(),
        observed_command: false,
    };
    start.finish(session).unwrap();
    drop(start);
    let slot = state.sessions.lock().unwrap().remove("terminal-1");
    drop(slot);
    killing.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(state.sessions.lock().unwrap().is_empty());
    assert!(state.update_blocked().unwrap());
    release.send(()).unwrap();
    reaper.join().unwrap();
    assert!(!state.update_blocked().unwrap());
}

fn metadata_record(id: &str) -> Terminal {
    serde_json::from_value(json!({
        "id": id, "auto_restore": true, "restore_plan": "htop",
        "bounds": {"x":0., "y":0., "width":720., "height":460.}
    }))
    .unwrap()
}

fn background_title_edit(
    state: &Arc<Terminals>,
    root: &Path,
    id: &'static str,
) -> thread::JoinHandle<()> {
    let state = state.clone();
    let root = root.to_owned();
    thread::spawn(move || {
        assert!(state
            .records
            .edit(&root, id, EditSource::Request, |r| {
                r.title = "slow edit".into();
                true
            })
            .unwrap());
    })
}

#[test]
fn final_snapshots_outlive_the_pty_budget_and_preserve_a_concurrent_metadata_edit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let state = Arc::new(Terminals::default());
    let live = "terminal-1";
    let starting = "terminal-2";
    let exited = "terminal-3";
    let locked = "terminal-4";
    for id in [live, starting, exited, locked] {
        state.records.insert(&root, metadata_record(id)).unwrap();
    }
    let session = fixture(&state, &root, live, "wait", true);
    let blocked = fixture(&state, &root, locked, "wait", true);
    let blocked_session = state
        .sessions
        .lock()
        .unwrap()
        .get(locked)
        .and_then(running)
        .unwrap();
    let guard = blocked_session.lock().unwrap();
    let finished = fixture(&state, &root, exited, "error", true);
    assert!(finished.done.recv_timeout(Duration::from_secs(3)).unwrap());
    let ticket = Start::reserve(&state, starting).unwrap();

    let (entered, writing) = mpsc::channel();
    let (release, released) = mpsc::channel();
    *state.records.before_write.lock().unwrap() = Some(Box::new(move || {
        entered.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(6)).unwrap();
    }));
    let edit = background_title_edit(&state, &root, live);
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    state.stopping.store(true, Ordering::Release);
    let (producers, drained) = mpsc::channel::<()>();
    let stopping = state.clone();
    let snapshot_root = root.clone();
    let started = Instant::now();
    assert!(!bounded_shutdown(move |deadline| {
        let observer = stopping.clone();
        stopping.stop_all(move |id, s| {
            let _producer = &producers;
            observer.final_snapshot(&snapshot_root, id, s);
        });
        stopping.workers.wait_until(deadline)
    }));
    let pty_time = started.elapsed();
    reaped(session.pid);
    reaped(blocked.pid);
    assert!(Start::reserve(&state, "terminal-10").is_err());
    let flush = state.clone();
    let (flushed, done) = mpsc::channel();
    let flush_root = root.clone();
    let join = thread::spawn(move || {
        flush.finish_metadata(&flush_root, drained);
        flushed.send(()).unwrap();
    });
    assert!(matches!(
        done.recv_timeout(Duration::from_millis(120)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    release.send(()).unwrap();
    edit.join().unwrap();
    done.recv_timeout(Duration::from_secs(3)).unwrap();
    join.join().unwrap();
    drop(guard);
    drop(ticket);
    assert!(state
        .workers
        .wait_until(Instant::now() + Duration::from_secs(2)));
    let records = read_records(&root).unwrap();
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|r| !r.auto_restore));
    let stored = records.iter().find(|r| r.id == live).unwrap();
    assert_eq!(stored.title, "slow edit");
    assert_eq!(stored.last_path.as_ref(), Some(&root));
    assert!(state
        .records
        .edit(&root, live, EditSource::Request, |_| true)
        .is_err());
    eprintln!("direct child PIDs {}, {} reaped at {} ms; final file flush at {} ms; cwd, title and four eligibility flags verified",session.pid,blocked.pid,pty_time.as_millis(),started.elapsed().as_millis());
}

#[test]
fn old_reader_and_retirement_cannot_modify_reused_id_after_a_slow_write() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_owned();
    let id = "terminal-1";
    let state = Arc::new(Terminals::default());
    state.records.insert(&root, metadata_record(id)).unwrap();
    let old = fixture(&state, &root, id, "error", true);
    assert!(old.done.recv_timeout(Duration::from_secs(3)).unwrap());
    let (entered, writing) = mpsc::channel();
    let (release, released) = mpsc::channel();
    *state.records.before_write.lock().unwrap() = Some(Box::new(move || {
        entered.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(4)).unwrap();
    }));
    let edit = background_title_edit(&state, &root, id);
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    let stale = state.clone();
    let old_root = root.clone();
    let observer = thread::spawn(move || {
        for owner in [
            welcome::RecordOwner::Active(old.token),
            welcome::RecordOwner::Exited(old.token),
        ] {
            stale
                .edit_owned(&old_root, id, owner, EditSource::Observation, |_| {
                    panic!("stale observation reached the replacement")
                })
                .unwrap();
            assert!(stale
                .edit_owned(&old_root, id, owner, EditSource::Request, |_| {
                    panic!("stale request reached the replacement")
                })
                .is_err());
        }
    });
    state.sessions.lock().unwrap().remove(id);
    let next = Start::reserve(&state, id).unwrap();
    release.send(()).unwrap();
    state
        .edit_owned(
            &root,
            id,
            welcome::RecordOwner::Active(next.token),
            EditSource::Request,
            |r| {
                r.restore_plan = Some("new".into());
                true
            },
        )
        .unwrap();
    observer.join().unwrap();
    edit.join().unwrap();
    let (tx, rx) = mpsc::channel();
    drop(tx);
    state.finish_metadata(&root, rx);
    let record = read_records(&root).unwrap().pop().unwrap();
    assert_eq!(record.restore_plan.as_deref(), Some("new"));
    assert!(record.auto_restore);
    assert_eq!(record.title, "slow edit");
}

fn already_reaped(pid: i32) -> bool {
    // Observe ownership without consuming a waitable child's status.
    let mut info = unsafe { std::mem::zeroed() };
    (unsafe {
        libc::waitid(
            libc::P_PID,
            pid as _,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    }) == -1
        && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD)
}

fn explicit_close_stalled_storage(before_lock: bool, locked_writer: bool) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_owned();
    let state = Arc::new(Terminals::default());
    let id = "terminal-1";
    state.records.insert(&root, metadata_record(id)).unwrap();
    let f = fixture(&state, &root, id, "wait", true);
    let running_session = state
        .sessions
        .lock()
        .unwrap()
        .get(id)
        .and_then(running)
        .unwrap();
    let input = locked_writer.then(|| running_session.lock().unwrap());
    let (write, writing) = mpsc::channel();
    let (release, released) = mpsc::channel();
    *state.records.before_write.lock().unwrap() = Some(Box::new(move || {
        write.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(8)).unwrap();
    }));
    let prior_edit = before_lock.then(|| background_title_edit(&state, &root, id));
    if before_lock {
        writing.recv_timeout(Duration::from_secs(2)).unwrap();
    }
    let closing = state.clone();
    let close_root = root.clone();
    let (destroy, destroyed) = mpsc::channel();
    let close = thread::spawn(move || {
        welcome::close_fixture(
            &closing,
            &close_root,
            id,
            f.token,
            || {
                destroy.send(()).unwrap();
                Ok(())
            },
            |()| Ok(()),
        )
    });
    destroyed.recv_timeout(Duration::from_secs(2)).unwrap();
    if !before_lock {
        writing.recv_timeout(Duration::from_secs(2)).unwrap();
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    while !already_reaped(f.pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let before_quit = already_reaped(f.pid);
    let registry_present = state.sessions.lock().unwrap().contains_key(id);
    assert!(Start::reserve(&state, id).is_err());
    let late_state = state.clone();
    let late_root = root.clone();
    let late = thread::spawn(move || {
        late_state.edit_owned(
            &late_root,
            id,
            welcome::RecordOwner::Active(f.token),
            EditSource::Observation,
            |r| {
                r.auto_restore = true;
                true
            },
        )
    });
    state.stopping.store(true, Ordering::Release);
    let quit = state.clone();
    let snapshots = state.clone();
    let snapshot_root = root.clone();
    let (producers, drained) = mpsc::channel::<()>();
    let pty_complete = bounded_shutdown(move |deadline| {
        quit.stop_all(move |key, s| {
            let _producer = &producers;
            snapshots.final_snapshot(&snapshot_root, key, s);
        });
        quit.workers.wait_until(deadline)
    });
    let at_quit = already_reaped(f.pid);
    assert!(state.close_saves.active());
    let final_state = state.clone();
    let (flushed, done) = mpsc::channel();
    let flush_root = root.clone();
    let flush = thread::spawn(move || {
        final_state.finish_metadata(&flush_root, drained);
        flushed.send(()).unwrap();
    });
    assert!(matches!(
        done.recv_timeout(Duration::from_millis(80)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    release.send(()).unwrap();
    if let Some(edit) = prior_edit {
        edit.join().unwrap();
    }
    drop(input);
    assert!(close.join().unwrap().unwrap());
    if let Err(error) = late.join().unwrap() {
        assert_eq!(error, "터미널 기록 저장을 종료했습니다");
    }
    flush.join().unwrap();
    assert!(state
        .workers
        .wait_until(Instant::now() + Duration::from_secs(2)));
    let record = read_records(&root).unwrap().pop().unwrap();
    eprintln!("explicit_close before_io_lock={before_lock} locked_writer={locked_writer} pid={} slot_present={registry_present} reaped_before_quit={before_quit} reaped_at_pty_return={at_quit} pty_complete={pty_complete} final_auto_restore={}",f.pid,record.auto_restore);
    if !locked_writer {
        assert_eq!(
            record.last_path.unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }
    assert!(before_quit && at_quit && !record.auto_restore && !registry_present && pty_complete);
}

#[test]
fn explicit_close_reaps_before_the_common_journal_write_completes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_owned();
    let state = Arc::new(Terminals::default());
    let id = "terminal-1";
    state.records.insert(&root, metadata_record(id)).unwrap();
    let f = fixture(&state, &root, id, "wait", true);
    let (entered, writing) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let closing = state.clone();
    let root_for_close = root.clone();
    let close = thread::spawn(move || {
        welcome::close_fixture(
            &closing,
            &root_for_close,
            id,
            f.token,
            || Ok(()),
            |()| {
                entered.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(())
            },
        )
    });
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(state.close_saves.active());
    assert!(!state.sessions.lock().unwrap().contains_key(id));
    assert!(Start::reserve(&state, id).is_err());
    let deadline = Instant::now() + Duration::from_millis(750);
    while !already_reaped(f.pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let reaped_during_write = already_reaped(f.pid);
    release.send(()).unwrap();
    assert!(close.join().unwrap().unwrap());
    assert!(reaped_during_write);
    assert!(!state.close_saves.active());
    assert!(!read_records(&root).unwrap()[0].auto_restore);
}

#[test]
fn explicit_close_reaps_while_waiting_for_the_file_lock() {
    explicit_close_stalled_storage(true, true);
}

#[test]
fn explicit_close_reaps_while_its_own_file_write_is_stalled() {
    explicit_close_stalled_storage(false, true);
}

#[test]
fn explicit_close_reaps_and_saves_observed_cwd_despite_prior_metadata_io() {
    explicit_close_stalled_storage(true, false);
}
