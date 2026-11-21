//! Child ownership is independent of a potentially blocked PTY input write.
use super::jobs::OwnedJobs;
use portable_pty::{Child, ExitStatus};
use std::{
    io,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

type ChildBox = Box<dyn Child + Send + Sync>;

struct ChildState {
    child: Option<ChildBox>,
    status: Option<ExitStatus>,
    jobs: OwnedJobs,
    #[cfg(windows)]
    remote: Option<super::windows_owner::Link>,
}

#[derive(Clone)]
pub(super) struct OwnedChild(Arc<Mutex<ChildState>>);

impl OwnedChild {
    #[cfg(windows)]
    pub(super) fn survives_update(&self) -> bool {
        let state = self.0.lock().unwrap();
        let remote = state.remote.clone();
        let running = state.status.is_none();
        drop(state);
        running && remote.is_some_and(|r| r.survives_update())
    }
    pub(super) fn new(child: ChildBox) -> Self {
        #[cfg(windows)]
        let remote = (&*child as &dyn Child)
            .downcast_ref::<super::windows_owner::RemoteChild>()
            .map(super::windows_owner::RemoteChild::control);
        Self(Arc::new(Mutex::new(ChildState {
            child: Some(child),
            status: None,
            jobs: OwnedJobs::default(),
            #[cfg(windows)]
            remote,
        })))
    }

    pub(super) fn process_id(&self) -> Option<u32> {
        self.0
            .lock()
            .unwrap()
            .child
            .as_ref()
            .and_then(|c| c.process_id())
    }

    pub(super) fn try_wait(&self) -> io::Result<Option<ExitStatus>> {
        let mut state = self.0.lock().unwrap();
        #[cfg(target_os = "macos")]
        let jobs = if let Some(pid) = state.child.as_ref().and_then(|c| c.process_id()) {
            if !waitable_exit(pid)? {
                return Ok(None);
            }
            // No one else can reap this PID until this mutex is released.
            Some(OwnedJobs::snapshot(pid))
        } else {
            None
        };
        if let Some(child) = &mut state.child {
            if let Some(status) = child.try_wait()? {
                #[cfg(target_os = "macos")]
                if let Some(jobs) = jobs {
                    state.jobs = jobs;
                }
                state.status = Some(status);
                state.child.take();
            }
        } else if state.status.is_none() {
            return Err(io::Error::other("터미널이 종료 중입니다"));
        }
        Ok(state.status.clone())
    }

    fn take_jobs(&self) -> OwnedJobs {
        std::mem::take(&mut self.0.lock().unwrap().jobs)
    }

    fn terminate(&self) {
        let mut state = self.0.lock().unwrap();
        let child = state.child.take();
        let jobs = std::mem::take(&mut state.jobs);
        #[cfg(windows)]
        let remote = state.remote.take();
        drop(state);
        #[cfg(windows)]
        if let Some(remote) = remote {
            remote.stop();
        }
        jobs.stop();
        if let Some(child) = child {
            terminate_and_reap(child);
        }
    }

    #[cfg(windows)]
    fn detach_remote(&self) -> bool {
        let mut state = self.0.lock().unwrap();
        let Some(remote) = state.remote.take() else {
            return false;
        };
        remote.detach();
        state.child.take();
        state.status = Some(ExitStatus::with_exit_code(0));
        true
    }
}

#[cfg(target_os = "macos")]
fn waitable_exit(pid: u32) -> io::Result<bool> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        libc::waitid(
            libc::P_PID,
            pid,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info.si_pid != 0)
}

fn terminate_and_reap(mut child: ChildBox) {
    // The exclusive handle pins an unreaped child's PID. A wait error or
    // cached exit never authorizes portable-pty's preliminary Unix SIGHUP.
    #[cfg(target_os = "macos")]
    if let Some(pid) = child.process_id() {
        // waitid authorizes enumeration while the real owned child, alive
        // or a zombie, still prevents PID reuse. Do not use a cached PID.
        if waitable_exit(pid).is_ok() {
            OwnedJobs::snapshot(pid).stop();
        }
    }
    match child.try_wait() {
        Ok(None) => {
            #[cfg(windows)]
            if child.as_raw_handle().is_some() {
                if let Some(pid) = child.process_id().filter(|pid| *pid > 0) {
                    // Keep the process HANDLE alive across taskkill /T. Its
                    // potentially blocking wait stays on this cleanup worker.
                    if !super::super::windows_process::stop_owned_tree(pid) {
                        eprintln!("터미널 하위 작업 정리 일부 미확인");
                    }
                }
            }
            let _ = child.kill();
        }
        Ok(Some(_)) => return,
        Err(_) => (),
    }
    let _ = child.wait();
}

impl Drop for ChildState {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(remote) = self.remote.take() {
            thread::spawn(move || remote.stop());
            return;
        }
        // Spawn/attach failures before publication have no supervisor yet.
        if let Some(child) = self.child.take() {
            let jobs = std::mem::take(&mut self.jobs);
            thread::spawn(move || {
                jobs.stop();
                terminate_and_reap(child)
            });
        }
    }
}

pub(super) const PROBE: Duration = Duration::from_millis(100);

pub(super) enum Event {
    Reader(thread::JoinHandle<()>),
    Stop,
    Quit {
        saved: mpsc::Receiver<()>,
        until: std::time::Instant,
    },
}

pub(super) struct Control {
    pub sender: mpsc::Sender<Event>,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for Control {
    fn drop(&mut self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        let _ = self.sender.send(Event::Stop);
    }
}

/// Starts as soon as the direct child exists, even if PTY setup has not
/// completed. Natural completion needs both an exit status and a drained pump.
pub(super) fn supervise(
    child: OwnedChild,
    events: mpsc::Receiver<Event>,
    on_exit: impl FnOnce(ExitStatus, OwnedJobs),
) {
    let mut reader: Option<thread::JoinHandle<()>> = None;
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            if reader.as_ref().is_some_and(thread::JoinHandle::is_finished) {
                let _ = reader.take().unwrap().join();
                on_exit(status, child.take_jobs());
                return;
            }
        }
        match events.recv_timeout(PROBE) {
            Ok(Event::Reader(handle)) => reader = Some(handle),
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            event => {
                if let Ok(Event::Quit { saved, until }) = event {
                    let _ = saved
                        .recv_timeout(until.saturating_duration_since(std::time::Instant::now()));
                    #[cfg(windows)]
                    if child.detach_remote() {
                        return;
                    }
                }
                child.terminate();
                return;
            }
        }
    }
}

pub(super) struct PtyReader {
    #[cfg(unix)]
    file: std::fs::File,
    #[cfg(windows)]
    pipe: Box<dyn io::Read + Send>,
}

impl PtyReader {
    pub(super) fn new(master: &dyn portable_pty::MasterPty) -> Result<Self, String> {
        #[cfg(unix)]
        {
            use std::os::fd::BorrowedFd;
            let raw = master.as_raw_fd().ok_or("PTY 파일 디스크립터가 없습니다")?;
            // master stays borrowed until its descriptor has been duplicated.
            let fd = unsafe { BorrowedFd::borrow_raw(raw) }
                .try_clone_to_owned()
                .map_err(super::err)?;
            Ok(Self {
                file: std::fs::File::from(fd),
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                pipe: master.try_clone_reader().map_err(super::err)?,
            })
        }
    }

    /// A lost delivery channel discards its bytes but cannot stop PTY
    /// draining. Only explicit cancellation or a terminal read result ends it.
    pub(super) fn drain(
        &mut self,
        cancelled: &std::sync::atomic::AtomicBool,
        mut consume: impl FnMut(&[u8]),
    ) {
        let mut bytes = [0; 8192];
        while !cancelled.load(std::sync::atomic::Ordering::Acquire) {
            match self.read_until_cancelled(&mut bytes, cancelled) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Ok(0) | Err(_) => break,
                Ok(n) => consume(&bytes[..n]),
            }
        }
    }

    pub(super) fn read_until_cancelled(
        &mut self,
        buf: &mut [u8],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> io::Result<usize> {
        use std::io::Read;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let mut fd = libc::pollfd {
                fd: self.file.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            while !cancelled.load(std::sync::atomic::Ordering::Acquire) {
                match unsafe { libc::poll(&mut fd, 1, PROBE.as_millis() as i32) } {
                    -1 => return Err(io::Error::last_os_error()),
                    0 => continue,
                    _ => return self.read(buf),
                }
            }
            Ok(0)
        }
        #[cfg(windows)]
        {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                Ok(0)
            } else {
                self.read(buf)
            }
        }
    }
}

impl io::Read for PtyReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            match self.file.read(buf) {
                Err(e) if e.raw_os_error() == Some(libc::EIO) => Ok(0),
                result => result,
            }
        }
        #[cfg(windows)]
        {
            self.pipe.read(buf)
        }
    }
}

#[derive(Default)]
pub(super) struct Workers {
    count: Mutex<usize>,
    changed: std::sync::Condvar,
}

pub(super) struct Worker(Arc<Workers>);

impl Workers {
    #[cfg(windows)]
    pub(super) fn count(&self) -> usize {
        *self.count.lock().unwrap()
    }
    pub(super) fn enter(self: &Arc<Self>) -> Worker {
        *self.count.lock().unwrap() += 1;
        Worker(self.clone())
    }

    pub(super) fn active(&self) -> bool {
        *self.count.lock().unwrap() != 0
    }

    pub(super) fn wait(&self) {
        let count = self.count.lock().unwrap();
        drop(self.changed.wait_while(count, |n| *n != 0).unwrap());
    }

    pub(super) fn wait_until(&self, deadline: std::time::Instant) -> bool {
        let count = self.count.lock().unwrap();
        let (count, _) = self
            .changed
            .wait_timeout_while(
                count,
                deadline.saturating_duration_since(std::time::Instant::now()),
                |n| *n > 0,
            )
            .unwrap();
        *count == 0
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let mut count = self.0.count.lock().unwrap();
        *count -= 1;
        if *count == 0 {
            self.0.changed.notify_all();
        }
    }
}
