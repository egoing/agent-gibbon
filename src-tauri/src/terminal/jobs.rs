//! Remember kernel-validated job identities while our own session leader PID
//! is still pinned by its waitable child. Never interpret a reaped SID again.
#[derive(Default)]
pub(super) struct OwnedJobs {
    unverified: bool,
    #[cfg(target_os = "macos")]
    tokens: Vec<[u32; 8]>,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn gibbon_job_token(owner: i32, candidate: i32, token: *mut u32) -> i32;
    fn gibbon_stop_job(token: *const u32) -> i32;
}

#[cfg(target_os = "macos")]
impl OwnedJobs {
    pub(super) fn snapshot(owner: u32) -> Self {
        let Ok(owner) = i32::try_from(owner) else {
            return Self {
                unverified: true,
                ..Self::default()
            };
        };
        if owner <= 1 || unsafe { libc::getsid(0) } == owner {
            return Self {
                unverified: true,
                ..Self::default()
            };
        }
        // libproc returns only numeric PIDs for this effective UID, never
        // argv, environment or terminal contents. Check private SID before
        // requesting a name port. This also finds reparented jobs.
        const PROC_UID_ONLY: u32 = 4;
        let uid = unsafe { libc::geteuid() };
        let bytes = unsafe { libc::proc_listpids(PROC_UID_ONLY, uid, std::ptr::null_mut(), 0) };
        if bytes <= 0 {
            return Self {
                unverified: true,
                ..Self::default()
            };
        }
        let mut pids = vec![0i32; bytes as usize / size_of::<i32>() + 32];
        let used = unsafe {
            libc::proc_listpids(
                PROC_UID_ONLY,
                uid,
                pids.as_mut_ptr().cast(),
                std::mem::size_of_val(&pids[..]) as i32,
            )
        };
        if used <= 0 {
            return Self {
                unverified: true,
                ..Self::default()
            };
        }
        pids.truncate(used as usize / size_of::<i32>());
        let mut tokens = Vec::new();
        let mut unverified = false;
        for pid in pids {
            let mut token = [0; 8];
            match unsafe { gibbon_job_token(owner, pid, token.as_mut_ptr()) } {
                1 => tokens.push(token),
                0 => (),
                _ => unverified = true,
            }
        }
        Self { tokens, unverified }
    }
}

impl OwnedJobs {
    pub(super) fn may_have_jobs(&self) -> bool {
        #[cfg(target_os = "macos")]
        let tracked = !self.tokens.is_empty();
        #[cfg(not(target_os = "macos"))]
        let tracked = false;
        tracked || self.unverified
    }

    pub(super) fn stop(self) {
        #[cfg(target_os = "macos")]
        let signals_ok = {
            let mut signals_ok = true;
            for token in &self.tokens {
                // Attempt each independently; ESRCH can also mean exec changed
                // the identity. Do not substitute its numeric PID.
                signals_ok &= unsafe { gibbon_stop_job(token.as_ptr()) } == 0;
            }
            signals_ok
        };
        #[cfg(not(target_os = "macos"))]
        let signals_ok = true;
        if self.unverified || !signals_ok {
            eprintln!("터미널 하위 작업 정리 일부 미확인");
        }
    }
}
