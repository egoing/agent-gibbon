//! An active scroll-only session tap. AppKit global monitors cannot consume
//! another app's events. Keep persistence and window work off this tap thread.
use super::{capture_wheel, held_input, mac_wheel_units, MacScrollCapture, Ordering, USED};
use objc2::{msg_send, ClassType};
use objc2_app_kit::{NSEvent, NSEventModifierFlags as F, NSEventPhase};
use std::{ffi::c_void, ptr};

type Ref = *mut c_void;
type Callback = unsafe extern "C" fn(Ref, u32, Ref, Ref) -> Ref;
const SCROLL: u32 = 22;
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        location: u32,
        placement: u32,
        options: u32,
        mask: u64,
        callback: Callback,
        context: Ref,
    ) -> Ref;
    fn CGEventTapEnable(tap: Ref, enable: bool);
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: Ref;
    fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
    fn CFMachPortInvalidate(port: Ref);
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(run_loop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRemoveSource(run_loop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRun();
    fn CFRelease(object: Ref);
}
struct Context {
    app: tauri::AppHandle,
    tap: Ref,
    capture: MacScrollCapture,
}

unsafe extern "C" fn scroll(_proxy: Ref, kind: u32, event: Ref, context: Ref) -> Ref {
    let context = &mut *(context as *mut Context);
    if kind == 0xffff_fffe {
        // The OS can disable a slow tap. Recover after timeout; a user-requested
        // disable is deliberately left disabled. Never suppress control events.
        CGEventTapEnable(context.tap, true);
        return event;
    }
    if kind != SCROLL || event.is_null() {
        return event;
    }
    objc2::rc::autoreleasepool(|_| {
        // NSEvent's CGEvent bridge preserves precise deltas, physical direction
        // and momentum flags. The borrowed object lives through this pool.
        let native: *mut NSEvent = msg_send![NSEvent::class(), eventWithCGEvent: event];
        let Some(native) = native.as_ref() else {
            return event;
        };
        USED.store(true, Ordering::Release);
        let flags = native.modifierFlags();
        let chord = flags.contains(F::Control | F::Option | F::Command)
            && !flags.intersects(F::Shift | F::Function)
            && !held_input()
            && native.momentumPhase() == NSEventPhase::None;
        let units = mac_wheel_units(
            native.scrollingDeltaY(),
            native.hasPreciseScrollingDeltas(),
            native.isDirectionInvertedFromDevice(),
        );
        let owned = crate::stash::input_owned(&context.app);
        let eligible = capture_wheel(owned, chord, units);
        let capture = context.capture.consume(
            eligible,
            owned,
            native.phase().0 as u64,
            native.momentumPhase().0 as u64,
        );
        let app = context.app.clone();
        let queued = context.app.run_on_main_thread(move || {
            crate::stash::cancel_chord(&app);
            crate::opacity::wheel(&app, units, chord);
        });
        if capture && queued.is_ok() {
            ptr::null_mut()
        } else {
            if queued.is_err() {
                context.capture.gesture = false;
            }
            event
        }
    })
}

pub(super) fn install(app: tauri::AppHandle) -> bool {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || unsafe {
        let mut context = Box::new(Context {
            app,
            tap: ptr::null_mut(),
            capture: MacScrollCapture::default(),
        });
        // Session (1), head (0), active filter (0); no root-only HID tap.
        let tap = CGEventTapCreate(1, 0, 0, 1 << SCROLL, scroll, &mut *context as *mut _ as Ref);
        if tap.is_null() {
            let _ = tx.send(false);
            return;
        }
        context.tap = tap;
        let source = CFMachPortCreateRunLoopSource(ptr::null_mut(), tap, 0);
        if source.is_null() {
            CFMachPortInvalidate(tap);
            CFRelease(tap);
            let _ = tx.send(false);
            return;
        }
        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        let _ = tx.send(true);
        CFRunLoopRun();
        CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
        CFMachPortInvalidate(tap);
        CFRelease(source);
        CFRelease(tap);
        // Context remains alive until the callback source is invalidated.
    });
    rx.recv().unwrap_or(false)
}
