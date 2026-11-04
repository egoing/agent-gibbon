//! Observe only whether a chord was mixed with other input. Never retain key
//! contents or suppress/modify an event destined for another application.
use std::sync::atomic::{AtomicBool, Ordering};
static USED: AtomicBool = AtomicBool::new(false);
pub fn used() -> bool {
    let event = USED.swap(false, Ordering::AcqRel);
    event || held_input()
}
fn held_input() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
        }
        objc2_app_kit::NSEvent::pressedMouseButtons() != 0
            || (0..128)
                .any(|key| !matches!(key, 54..=63) && unsafe { CGEventSourceKeyState(1, key) })
    }
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetAsyncKeyState(key: i32) -> i16;
        }
        (1..=254).any(|key| {
            !matches!(key, 0x10..=0x12 | 0x5b..=0x5c | 0xa0..=0xa5)
                && unsafe { GetAsyncKeyState(key) < 0 }
        })
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        false
    }
}
#[cfg(target_os = "macos")]
pub fn install(app: &tauri::AppHandle) -> bool {
    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::AnyObject};
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::{cell::RefCell, ptr::NonNull};
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    // Without global key observation we cannot safely distinguish a bare chord
    // from another app's shortcut. Leave the tray available instead.
    if !unsafe { AXIsProcessTrusted() } {
        return false;
    }
    struct Monitor(Retained<AnyObject>);
    impl Drop for Monitor {
        fn drop(&mut self) {
            unsafe { NSEvent::removeMonitor(&self.0) };
        }
    }
    thread_local! { static MONITORS: RefCell<Vec<Monitor>> = const { RefCell::new(Vec::new()) }; }
    let mask = NSEventMask::KeyDown
        | NSEventMask::LeftMouseDown
        | NSEventMask::RightMouseDown
        | NSEventMask::OtherMouseDown
        | NSEventMask::ScrollWheel
        | NSEventMask::FlagsChanged;
    fn observe(app: &tauri::AppHandle, event: NonNull<NSEvent>) {
        // Event flags preserve fast press/release order even if the run loop is
        // drawing an animation. Sampling current flags here would lose edges.
        let event = unsafe { event.as_ref() };
        if event.r#type() == objc2_app_kit::NSEventType::FlagsChanged {
            use objc2_app_kit::NSEventModifierFlags as F;
            let f = event.modifierFlags();
            let mask = u8::from(f.contains(F::Control))
                | (u8::from(f.contains(F::Option)) << 1)
                | (u8::from(f.contains(F::Command)) << 2);
            if !matches!(event.keyCode(), 54..=56 | 58..=62) {
                crate::stash::cancel_chord(app);
            }
            crate::stash::modifier_event(app, mask, f.intersects(F::Shift | F::Function));
        } else {
            crate::stash::cancel_chord(app);
        }
    }
    let handle = app.clone();
    let global = RcBlock::new(move |event: NonNull<NSEvent>| observe(&handle, event));
    let handle = app.clone();
    let local = RcBlock::new(move |event: NonNull<NSEvent>| {
        observe(&handle, event);
        event.as_ptr()
    });
    let global = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global);
    let local = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) };
    let ready = global.is_some() && local.is_some();
    MONITORS.with(|m| {
        m.borrow_mut()
            .extend(global.into_iter().chain(local).map(Monitor))
    });
    ready
}
#[cfg(windows)]
pub fn install(_app: &tauri::AppHandle) -> bool {
    windows::install()
}
#[cfg(not(any(target_os = "macos", windows)))]
pub fn install(_app: &tauri::AppHandle) -> bool {
    false
}
#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    type Handle = *mut c_void;
    type Hook = unsafe extern "system" fn(i32, usize, isize) -> isize;
    #[repr(C)]
    struct Message {
        hwnd: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        x: i32,
        y: i32,
        private: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowsHookExW(
            kind: i32,
            callback: Option<Hook>,
            module: Handle,
            thread: u32,
        ) -> Handle;
        fn CallNextHookEx(hook: Handle, code: i32, wparam: usize, lparam: isize) -> isize;
        fn UnhookWindowsHookEx(hook: Handle) -> i32;
        fn GetMessageW(message: *mut Message, window: Handle, min: u32, max: u32) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Handle;
    }
    unsafe extern "system" fn keyboard(code: i32, w: usize, l: isize) -> isize {
        if code >= 0 && matches!(w, 0x100 | 0x104) {
            // vkCode is the first DWORD of KBDLLHOOKSTRUCT; no text is decoded.
            let key = *(l as *const u32);
            if !matches!(key,0x11..=0x12 | 0x5b..=0x5c | 0xa2..=0xa5) {
                USED.store(true, Ordering::Release);
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }
    unsafe extern "system" fn mouse(code: i32, w: usize, l: isize) -> isize {
        if code >= 0 && matches!(w, 0x201 | 0x204 | 0x207 | 0x20b | 0x20a | 0x20e) {
            USED.store(true, Ordering::Release);
        }
        CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }
    pub fn install() -> bool {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let keyboard = SetWindowsHookExW(13, Some(keyboard), module, 0);
            let mouse = SetWindowsHookExW(14, Some(mouse), module, 0);
            let ready = !keyboard.is_null() && !mouse.is_null();
            let _ = tx.send(ready);
            if ready {
                let mut message: Message = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if !keyboard.is_null() {
                UnhookWindowsHookEx(keyboard);
            }
            if !mouse.is_null() {
                UnhookWindowsHookEx(mouse);
            }
        });
        rx.recv().unwrap_or(false)
    }
}
