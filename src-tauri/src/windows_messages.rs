//! Shared Win32 MSG ABI for both non-invasive event observers.
use std::ffi::c_void;
#[repr(C)]
#[derive(Default)]
pub(crate) struct Message {
    pub hwnd: *mut c_void,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub x: i32,
    pub y: i32,
    pub private: u32,
}
#[link(name = "user32")]
extern "system" {
    pub(crate) fn GetMessageW(
        message: *mut Message,
        window: *mut c_void,
        min: u32,
        max: u32,
    ) -> i32;
    pub(crate) fn PeekMessageW(
        message: *mut Message,
        window: *mut c_void,
        min: u32,
        max: u32,
        remove: u32,
    ) -> i32;
}
