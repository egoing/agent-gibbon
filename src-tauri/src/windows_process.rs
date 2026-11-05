//! Do not let a detached GUI retain the invoking agent's redirected pipes.
use std::process::{Child, Command};
#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(kind: u32) -> isize;
    fn GetHandleInformation(handle: isize, flags: *mut u32) -> i32;
    fn SetHandleInformation(handle: isize, mask: u32, flags: u32) -> i32;
}
struct Inheritance(Vec<(isize, u32)>);
impl Drop for Inheritance {
    fn drop(&mut self) {
        for &(handle, flags) in &self.0 {
            unsafe {
                SetHandleInformation(handle, 1, flags & 1);
            }
        }
    }
}
pub fn spawn(command: &mut Command) -> std::io::Result<Child> {
    let mut restore = Inheritance(Vec::new());
    // The CLI is single-threaded here. The child has explicit null stdio.
    for kind in [-10_i32, -11, -12] {
        let handle = unsafe { GetStdHandle(kind as u32) };
        let mut flags = 0;
        if handle != 0 && handle != -1 && unsafe { GetHandleInformation(handle, &mut flags) } != 0 {
            if unsafe { SetHandleInformation(handle, 1, 0) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            restore.0.push((handle, flags));
        }
    }
    command.spawn()
}

// Inspect process identity only, never command lines or shell history.
pub fn has_children(pid: u32) -> Option<bool> {
    #[repr(C)]
    struct Entry {
        size: u32,
        usage: u32,
        pid: u32,
        heap: usize,
        module: u32,
        threads: u32,
        parent: u32,
        priority: i32,
        flags: u32,
        exe: [u16; 260],
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut Entry) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut Entry) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(2, 0);
        if snapshot == -1 {
            return None;
        }
        let mut entry: Entry = std::mem::zeroed();
        entry.size = std::mem::size_of::<Entry>() as u32;
        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return None;
        }
        let mut found = false;
        loop {
            if entry.parent == pid {
                found = true;
                break;
            }
            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }
        CloseHandle(snapshot);
        Some(found)
    }
}
