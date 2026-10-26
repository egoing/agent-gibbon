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
