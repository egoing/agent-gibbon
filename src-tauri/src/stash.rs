//! One reversible, in-memory visibility transaction. All native mutations run on
//! the main thread; no WebView is destroyed, reloaded, or focused.
use fs2::FileExt;
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, LogicalSize};
use tauri::{Manager, WebviewWindow};
const GUIDE: &str = "direction-guide";
const ICON: &str = "stash-indicator";
const DURATION: f64 = 0.420;
#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
impl Rect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn clamp(self, area: Self) -> Self {
        let w = self.w.min(area.w);
        let h = self.h.min(area.h);
        Self {
            x: self.x.clamp(area.x, area.x + area.w - w),
            y: self.y.clamp(area.y, area.y + area.h - h),
            w,
            h,
        }
    }
}
#[derive(Clone)]
struct Saved {
    rect: Rect,
    inner_size: tauri::PhysicalSize<u32>,
    scale: f64,
    alpha: f64,
}
struct Session {
    after_restore: Option<(Direction, f64, f64)>,
    windows: BTreeMap<String, Saved>,
    started: Instant,
    animating: bool,
    restoring: bool,
    from: BTreeMap<String, Saved>,
    area: Rect,
    top: bool,
    gate: EscapeGate,
    tip_until: Instant,
    displayed: Option<(bool, bool)>,
    flight: Option<(Rect, Instant)>,
}
#[derive(Default)]
pub struct Stash {
    session: Mutex<Option<Session>>,
    gesture: Mutex<Gesture>,
    last_arrangement: Mutex<Option<String>>,
    restored_arrangement: Mutex<Option<(Direction, f64, f64)>>,
    shortcut: Mutex<String>,
    chord: Mutex<Chord>,
    lease: Mutex<Option<std::fs::File>>,
}
#[derive(Default)]
struct Gesture {
    hide_at: Option<Instant>,
    origin: Option<(f64, f64)>,
    scale: f64,
    moved: bool,
    direction: Option<Direction>,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Direction {
    Left,
    Down,
    Right,
    Up,
}
impl Gesture {
    fn track(&mut self, x: f64, y: f64) {
        let Some((a, b)) = self.origin else { return };
        let (dx, dy) = (x - a, y - b);
        if self.direction.is_some() || dx.hypot(dy) < 40.0 * self.scale {
            return;
        }
        self.moved = true;
        self.direction = if dx.abs() >= 1.5 * dy.abs() {
            Some(if dx > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            })
        } else if dy.abs() >= 1.5 * dx.abs() {
            Some(if dy > 0.0 {
                Direction::Down
            } else {
                Direction::Up
            })
        } else {
            None
        };
    }
}
// Quartz display bounds and event locations share the same global point space.
fn gesture_pointer(app: &tauri::AppHandle) -> Result<tauri::PhysicalPosition<f64>, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        #[repr(C)]
        struct QuartzPoint {
            x: f64,
            y: f64,
        }
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventCreate(source: *const std::ffi::c_void) -> *const std::ffi::c_void;
            fn CGEventGetLocation(event: *const std::ffi::c_void) -> QuartzPoint;
        }
        #[link(name = "CoreFoundation", kind = "framework")]
        extern "C" {
            fn CFRelease(object: *mut std::ffi::c_void);
        }
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            if event.is_null() {
                return Err("포인터 위치 없음".into());
            }
            let p = CGEventGetLocation(event);
            CFRelease(event.cast_mut());
            Ok(tauri::PhysicalPosition::new(p.x, p.y))
        }
    }
    #[cfg(not(target_os = "macos"))]
    app.cursor_position().map_err(err)
}
fn sample_pointer(app: &tauri::AppHandle) {
    let state = app.state::<Stash>();
    let mut g = state.gesture.lock().unwrap();
    if g.hide_at.is_some_and(|deadline| Instant::now() >= deadline) {
        g.hide_at = None;
        drop(g);
        hide_guide(app);
        return;
    }
    let Some((x, y)) = g.origin else { return };
    #[cfg(target_os = "macos")]
    if crate::stash_input::escape_down() {
        drop(g);
        dismiss_guide(app);
        return;
    }
    if state.chord.lock().unwrap().mask != 0 {
        return;
    }
    if let Ok(p) = gesture_pointer(app) {
        g.track(p.x, p.y);
    }
    let Some(direction) = g.direction else { return };
    g.origin = None;
    g.hide_at = Some(Instant::now() + Duration::from_millis(180));
    drop(g);
    if let Some(w) = app.get_webview_window(GUIDE) {
        let _ = w.eval("document.body.classList.add('closing')");
    }
    if crate::updater::frozen(app) {
        return;
    }
    match direction {
        Direction::Down => trigger(app),
        Direction::Left | Direction::Right | Direction::Up => {
            let mut session = state.session.lock().unwrap();
            if let Some(s) = session.as_mut() {
                s.after_restore = Some((direction, x, y));
                let restoring = s.restoring;
                drop(session);
                if !restoring {
                    if let Err(error) = toggle(app) {
                        *state.last_arrangement.lock().unwrap() = Some(error);
                    }
                }
            } else {
                drop(session);
                arrange_direction(app, direction, x, y);
            }
        }
    }
}
fn arrange_direction(app: &tauri::AppHandle, direction: Direction, x: f64, y: f64) {
    let result = crate::layout::gesture_monitor(app, x, y).and_then(|monitor| match direction {
        Direction::Up => crate::layout::overview::enter(app, monitor),
        Direction::Left => crate::layout::arrange(app, monitor, crate::layout::Arrangement::Left),
        Direction::Right => crate::layout::arrange(app, monitor, crate::layout::Arrangement::Right),
        Direction::Down => unreachable!("숨김은 trigger에서 처리"),
    });
    *app.state::<Stash>().last_arrangement.lock().unwrap() = Some(
        result
            .map(|_| format!("{direction:?}"))
            .unwrap_or_else(|e| e),
    );
}

// Commit only a bare chord on complete release. Other input cancels the chord.
// Partial release/repress and key repeat cannot trigger a second toggle.
#[derive(Default)]
struct Chord {
    latched: bool,
    mask: u8,
    pending: bool,
    cancelled: bool,
}
#[derive(Debug, PartialEq)]
enum GuideInput {
    Hold,
    Cancel,
    Open,
}
impl Chord {
    fn guide_input(&mut self, mask: u8, shift: bool, used: bool) -> GuideInput {
        let released = self.mask != 0 && mask == 0;
        if self.sample(mask, shift, used) {
            GuideInput::Open
        } else if released || shift || used || self.cancelled {
            GuideInput::Cancel
        } else {
            GuideInput::Hold
        }
    }

    fn sample(&mut self, mask: u8, shift: bool, used: bool) -> bool {
        if self.mask == 0 && mask != 0 {
            self.pending = false;
            self.cancelled = false;
        }
        self.mask = mask;
        if shift || used {
            self.cancelled = true;
            self.pending = false;
        }
        if mask == 7 {
            self.latched = true;
            if !self.cancelled {
                self.pending = true;
            }
        }
        if mask == 0 {
            let fire = self.pending && !self.cancelled;
            self.pending = false;
            self.latched = false;
            self.cancelled = false;
            return fire;
        }
        false
    }
}
fn modifiers() -> (u8, bool) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSEvent, NSEventModifierFlags as F};
        let f = NSEvent::modifierFlags_class();
        (
            (u8::from(f.contains(F::Control)))
                | (u8::from(f.contains(F::Option)) << 1)
                | (u8::from(f.contains(F::Command)) << 2),
            f.intersects(F::Shift | F::Function),
        )
    }
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetAsyncKeyState(key: i32) -> i16;
        }
        let down = |k| unsafe { GetAsyncKeyState(k) < 0 };
        (
            u8::from(down(0x11))
                | (u8::from(down(0x12)) << 1)
                | (u8::from(down(0x5b) || down(0x5c)) << 2),
            down(0x10),
        )
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        (0, false)
    }
}
pub fn input_owned(app: &tauri::AppHandle) -> bool {
    app.state::<Stash>().lease.lock().unwrap().is_some()
}
pub fn chord_busy(app: &tauri::AppHandle) -> bool {
    let state = app.state::<Stash>();
    let c = state.chord.lock().unwrap();
    c.latched && c.mask != 0
}
#[cfg(not(target_os = "macos"))]
fn poll_shortcut(app: &tauri::AppHandle) {
    let (mask, shift) = modifiers();
    modifier_event(app, mask, shift);
}
pub fn modifier_event(app: &tauri::AppHandle, mask: u8, shift: bool) {
    if mask != 7 || shift {
        crate::opacity::reset_wheel(app);
    }
    let state = app.state::<Stash>();
    let mut chord = state.chord.lock().unwrap();
    let input = chord.guide_input(mask, shift, crate::stash_input::used());
    drop(chord);
    match input {
        GuideInput::Hold => return,
        GuideInput::Cancel => return dismiss_guide(app),
        GuideInput::Open => {}
    }
    if !input_owned(app) || crate::updater::frozen(app) {
        return;
    }
    if let (Ok(p), Some(w)) = (gesture_pointer(app), app.get_webview_window(GUIDE)) {
        let scale = crate::layout::pointer_monitor(app, p.x, p.y)
            .ok()
            .flatten()
            .map_or(1., |m| m.scale_factor());
        let restores = app
            .state::<Stash>()
            .session
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| !s.restoring);
        let already_open = state.gesture.lock().unwrap().origin.is_some();
        if let Err(e) = position_guide(&w, p.x, p.y, scale)
            .and_then(|_| {
                w.eval(format!("window.showDirectionGuide({restores})"))
                    .map_err(err)
            })
            .and_then(|_| if already_open { Ok(()) } else { show_frame(&w) })
        {
            eprintln!("방향 안내 표시 실패: {e}");
            return;
        }
        *state.gesture.lock().unwrap() = Gesture {
            origin: Some((p.x, p.y)),
            scale: if cfg!(target_os = "macos") { 1. } else { scale },
            ..Default::default()
        };
    }
}
fn position_guide(w: &WebviewWindow, x: f64, y: f64, scale: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = (x, y, scale);
        let mut point = objc2_app_kit::NSEvent::mouseLocation();
        unsafe {
            let native = &*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow);
            let size = native.frame().size;
            point.x -= size.width / 2.;
            point.y -= size.height / 2.;
            native.setFrameOrigin(point);
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    w.set_position(tauri::PhysicalPosition::new(
        (x - 116. * scale).round() as i32,
        (y - 116. * scale).round() as i32,
    ))
    .map_err(err)
}
fn hide_guide(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(GUIDE) {
        let _ = w.hide();
        let _ = w.eval("document.body.classList.remove('open', 'closing')");
    }
}
fn dismiss_guide(app: &tauri::AppHandle) {
    if app
        .state::<Stash>()
        .gesture
        .lock()
        .unwrap()
        .origin
        .take()
        .is_some()
    {
        hide_guide(app);
    }
}
pub fn cancel_chord(app: &tauri::AppHandle) {
    let state = app.state::<Stash>();
    dismiss_guide(app);
    let mut chord = state.chord.lock().unwrap();
    if chord.mask != 0 {
        chord.cancelled = true;
        chord.pending = false;
    }
}
fn snapshot_candidate(label: &str) -> bool {
    !matches!(
        label,
        ICON | GUIDE | "layout-undo" | "memo-overview" | "memo-backdrop"
    ) && !label.starts_with("relation-prototype-")
        && !label.starts_with("relation-pick-")
        && label != "relation-settings"
}
fn compact(r: Rect, area: Rect, top: bool) -> Rect {
    let w = (r.w * 0.18).max(144.).min(r.w);
    let h = (r.h * 0.18).max(32.).min(r.h);
    Rect {
        x: area.x + area.w - w - 12.,
        y: if top {
            area.y + 12.
        } else {
            area.y + area.h - h - 12.
        },
        w,
        h,
    }
}
fn interpolate(a: Rect, b: Rect, t: f64) -> Rect {
    Rect {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        w: a.w + (b.w - a.w) * t,
        h: a.h + (b.h - a.h) * t,
    }
}
pub(crate) fn show_frame(w: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    unsafe {
        (&*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
    }
    #[cfg(windows)]
    crate::focus::show_without_focus(w)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    w.show().map_err(err)?;
    Ok(())
}
struct EscapeGate {
    armed: bool,
}
impl EscapeGate {
    fn step(&mut self, distance: f64) -> bool {
        // Only sampled while stationary. A flight disarms the gate and input
        // during flight is not queued. Hint lifetime never blocks a fresh exit
        // and reentry; hysteresis prevents a held pointer from chasing the icon.
        if distance > 96.0 {
            self.armed = true;
        }
        if self.armed && distance < 56.0 {
            self.armed = false;
            true
        } else {
            false
        }
    }
}
fn unit(scale: f64) -> f64 {
    if cfg!(target_os = "macos") {
        scale
    } else {
        1.0
    }
}
fn rect(w: &WebviewWindow) -> Result<Rect, String> {
    let u = unit(w.scale_factor().map_err(err)?);
    let p = w.outer_position().map_err(err)?;
    let s = w.outer_size().map_err(err)?;
    Ok(Rect {
        x: p.x as f64 / u,
        y: p.y as f64 / u,
        w: s.width as f64 / u,
        h: s.height as f64 / u,
    })
}
fn areas(w: &WebviewWindow) -> Result<Vec<Rect>, String> {
    Ok(w.available_monitors()
        .map_err(err)?
        .iter()
        .map(|m| {
            let a = m.work_area();
            let u = unit(m.scale_factor());
            Rect {
                x: a.position.x as f64 / u,
                y: a.position.y as f64 / u,
                w: a.size.width as f64 / u,
                h: a.size.height as f64 / u,
            }
        })
        .collect())
}
fn pointer(w: &WebviewWindow) -> Result<(f64, f64), String> {
    let p = w.cursor_position().map_err(err)?;
    let u = unit(
        w.primary_monitor()
            .map_err(err)?
            .map(|m| m.scale_factor())
            .unwrap_or(1.),
    );
    Ok((p.x / u, p.y / u))
}
fn geometry(w: &WebviewWindow, r: Rect, scale: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = scale;
        w.set_size(LogicalSize::new(r.w, r.h)).map_err(err)?;
        w.set_position(LogicalPosition::new(r.x, r.y)).map_err(err)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = scale;
        w.set_position(tauri::PhysicalPosition::new(r.x as i32, r.y as i32))
            .map_err(err)?;
        let size = tauri::PhysicalSize::new(r.w.round() as u32, r.h.round() as u32);
        #[cfg(windows)]
        {
            crate::windows_window::set_outer_size(w, size)
        }
        #[cfg(not(windows))]
        w.set_size(size).map_err(err)
    }
}
fn alpha(w: &WebviewWindow, value: f64) -> Result<(), String> {
    let value = if crate::layout::overview::protects(w) {
        1.0
    } else {
        value
    };
    #[cfg(target_os = "macos")]
    unsafe {
        (&*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow)).setAlphaValue(value);
    }
    #[cfg(windows)]
    crate::windows_window::set_opacity(w, value)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    w.eval(format!("document.documentElement.style.opacity='{value}'"))
        .map_err(err)?;
    Ok(())
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn active(app: &tauri::AppHandle) -> bool {
    app.state::<Stash>().session.lock().unwrap().is_some()
}
// Inspect uses the pre-animation client size while the stash owns the frame.
pub(crate) fn saved_geometry(w: &WebviewWindow) -> Option<crate::capture::ImageNote> {
    let stash = w.state::<Stash>();
    let session = stash.session.lock().ok()?;
    let saved = session.as_ref()?.windows.get(w.label())?;
    let pos_scale = if cfg!(windows) { 1.0 } else { saved.scale };
    Some(crate::capture::ImageNote {
        x: saved.rect.x * unit(saved.scale) / pos_scale,
        y: saved.rect.y * unit(saved.scale) / pos_scale,
        width: f64::from(saved.inner_size.width) / saved.scale,
        height: f64::from(saved.inner_size.height) / saved.scale,
    })
}
pub fn owns(w: &WebviewWindow) -> bool {
    w.state::<Stash>()
        .session
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|s| s.windows.contains_key(w.label()))
}
pub fn shortcut(app: &tauri::AppHandle) -> String {
    app.state::<Stash>().shortcut.lock().unwrap().clone()
}
pub fn inspect(app: &tauri::AppHandle) -> serde_json::Value {
    let state = app.state::<Stash>();
    let s = state.session.lock().unwrap();
    serde_json::json!({"last_arrangement":state.last_arrangement.lock().unwrap().clone(),"active":s.is_some(),"animating":s.as_ref().is_some_and(|s|s.animating),"restoring":s.as_ref().is_some_and(|s|s.restoring),"indicator_moving":s.as_ref().is_some_and(|s|s.flight.is_some()),"tip_visible":s.as_ref().is_some_and(|s|s.flight.is_none() && s.displayed.is_some_and(|d|d.1)),"windows":s.as_ref().map(|s|s.windows.keys().collect::<Vec<_>>()),"shortcut":shortcut(app),"indicator":app.get_webview_window(ICON).and_then(|w|rect(&w).ok()).map(|r|serde_json::json!({"x":r.x,"y":r.y,"width":r.w,"height":r.h}))})
}
pub(crate) fn forget(w: &WebviewWindow) {
    if let Some(s) = w.state::<Stash>().session.lock().unwrap().as_mut() {
        s.windows.remove(w.label());
        s.from.remove(w.label());
    }
}
// Explicit individual open/close takes ownership away from the transaction.
pub fn release(w: &WebviewWindow) -> Result<(), String> {
    crate::first_frame::cancel(w.app_handle(), w.label());
    let saved = w
        .state::<Stash>()
        .session
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|s| s.windows.remove(w.label()));
    if let Some(saved) = saved {
        w.hide().map_err(err)?;
        geometry(w, saved.rect, saved.scale)?;
        alpha(w, saved.alpha)?;
    }
    Ok(())
}
pub fn trigger(app: &tauri::AppHandle) {
    if crate::updater::frozen(app) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Err(e) = toggle(&handle) {
                eprintln!("전체 접어두기 실패: {e}");
            }
        });
    });
}
fn indicator(app: &tauri::AppHandle) -> Result<WebviewWindow, String> {
    if let Some(w) = app.get_webview_window(ICON) {
        return Ok(w);
    }
    crate::focus::remember(app, ICON);
    let w =
        tauri::WebviewWindowBuilder::new(app, ICON, tauri::WebviewUrl::App("stash.html".into()))
            .title(format!(
                "{} — stash-indicator",
                crate::instance::current().title()
            ))
            .inner_size(320., 48.)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .visible(false)
            .focused(false)
            .focusable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .build()
            .map_err(err)?;
    w.set_ignore_cursor_events(true).map_err(err)?;
    Ok(w)
}
fn indicator_rect(area: Rect, top: bool, factor: f64) -> Rect {
    let width = 320. * factor;
    let height = 48. * factor;
    let margin = 12. * factor;
    Rect {
        x: (area.x + area.w - width - margin).max(area.x),
        y: if top {
            area.y + margin
        } else {
            area.y + area.h - height - margin
        },
        w: width.min(area.w),
        h: height,
    }
}
fn place_indicator(app: &tauri::AppHandle, area: Rect, top: bool, tip: bool) -> Result<(), String> {
    let w = indicator(app)?;
    let scale = w.scale_factor().map_err(err)?;
    let factor = scale / unit(scale);
    geometry(&w, indicator_rect(area, top, factor), scale)?;
    set_tip(app, &w, tip)?;
    crate::show_without_focus(&w)
}
fn set_tip(app: &tauri::AppHandle, w: &WebviewWindow, tip: bool) -> Result<(), String> {
    let text = if tip {
        format!(
            "복원: {} 놓고 ↓",
            crate::presentation::shortcut(shortcut(app).split_whitespace().next().unwrap_or(""))
        )
    } else {
        String::new()
    };
    w.eval(format!(
        "window.setStashTip({})",
        serde_json::to_string(&text).map_err(err)?
    ))
    .map_err(err)
}
fn restore_rect(w: &WebviewWindow, s: &Saved) -> Result<Rect, String> {
    let screens = areas(w)?;
    let area = screens
        .iter()
        .find(|a| a.contains(s.rect.x + s.rect.w / 2., s.rect.y + s.rect.h / 2.));
    // Full-screen capture overlays may legitimately extend beyond the work
    // area. Preserve their exact frame while their original display remains.
    let r = if area.is_some() {
        s.rect
    } else {
        screens.first().map(|a| s.rect.clamp(*a)).unwrap_or(s.rect)
    };
    Ok(r)
}
pub fn toggle(app: &tauri::AppHandle) -> Result<(), String> {
    let _update_activity = crate::updater::activity(app)?;
    crate::layout::overview::leave(app)?;
    crate::relation_prototype::transition_started(app);
    let previous = app.state::<Stash>().session.lock().unwrap().take();
    if let Some(mut s) = previous {
        let restoring = !s.restoring;
        if !restoring {
            s.after_restore = None;
        }
        let mut from = BTreeMap::new();
        for (label, saved) in &mut s.windows {
            if let Some(w) = crate::web::managed_window(app, label) {
                saved.rect = restore_rect(&w, saved)?;
                let frame = if !s.animating {
                    Saved {
                        rect: compact(saved.rect, s.area, s.top),
                        inner_size: saved.inner_size,
                        scale: saved.scale,
                        alpha: saved.alpha * 0.1,
                    }
                } else {
                    let start = s.from.get(label).unwrap_or(saved);
                    let e =
                        1. - (1. - (s.started.elapsed().as_secs_f64() / DURATION).min(1.)).powi(3);
                    let target = if s.restoring {
                        saved.alpha
                    } else {
                        saved.alpha * 0.1
                    };
                    Saved {
                        rect: rect(&w)?,
                        inner_size: saved.inner_size,
                        scale: saved.scale,
                        alpha: start.alpha + (target - start.alpha) * e,
                    }
                };
                from.insert(label.clone(), frame);
            }
        }
        s.from = from.clone();
        s.restoring = restoring;
        s.flight = None;
        s.animating = true;
        s.started = Instant::now();
        s.displayed = None;
        app.state::<Stash>().session.lock().unwrap().replace(s);
        if let Some(w) = app.get_webview_window(ICON) {
            w.hide().map_err(err)?;
        }
        let mut entries: Vec<_> = from.into_iter().collect();
        entries.sort_by_key(|(label, _)| label.starts_with("capture-"));
        for (label, frame) in entries {
            if let Some(w) = crate::web::managed_window(app, &label) {
                geometry(&w, frame.rect, frame.scale)?;
                alpha(&w, frame.alpha)?;
                show_frame(&w)?;
                // Windows focusability changes during show may replace the
                // layered style. Reapply the saved alpha before the first frame.
                #[cfg(windows)]
                alpha(&w, frame.alpha)?;
            }
        }
        return Ok(());
    }
    let icon = indicator(app)?;
    let (x, y) = pointer(&icon)?;
    let screens = areas(&icon)?;
    // Screen selection uses full bounds: menu bars/Docks are outside the work
    // area but still belong to the screen under the cursor.
    let monitor = icon
        .available_monitors()
        .map_err(err)?
        .into_iter()
        .find(|m| {
            let u = unit(m.scale_factor());
            let p = m.position();
            let size = m.size();
            Rect {
                x: p.x as f64 / u,
                y: p.y as f64 / u,
                w: size.width as f64 / u,
                h: size.height as f64 / u,
            }
            .contains(x, y)
        });
    let area = if let Some(m) = monitor {
        let a = m.work_area();
        let u = unit(m.scale_factor());
        Rect {
            x: a.position.x as f64 / u,
            y: a.position.y as f64 / u,
            w: a.size.width as f64 / u,
            h: a.size.height as f64 / u,
        }
    } else {
        *screens.first().ok_or("사용 가능한 화면 없음")?
    };
    let mut windows = BTreeMap::new();
    for w in crate::web::managed_windows(app).into_values() {
        if !snapshot_candidate(w.label())
            || !w.is_visible().map_err(err)?
            || w.is_minimized().map_err(err)?
        {
            continue;
        }
        crate::entrance::cancel(&w)?;
        crate::sizing::cancel(&w);
        let alpha_value = {
            #[cfg(target_os = "macos")]
            {
                unsafe {
                    (&*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow)).alphaValue()
                }
            }
            #[cfg(windows)]
            {
                crate::windows_window::opacity(&w)?
            }
            #[cfg(not(any(target_os = "macos", windows)))]
            {
                1.0
            }
        };
        windows.insert(
            w.label().into(),
            Saved {
                rect: rect(&w)?,
                inner_size: w.inner_size().map_err(err)?,
                scale: w.scale_factor().map_err(err)?,
                alpha: alpha_value,
            },
        );
    }
    if windows.is_empty() {
        return Ok(());
    }
    let now = Instant::now();
    let s = Session {
        after_restore: None,
        from: windows.clone(),
        windows,
        restoring: false,
        started: now,
        animating: true,
        area,
        top: false,
        gate: EscapeGate { armed: true },
        tip_until: now,
        displayed: None,
        flight: None,
    };
    app.state::<Stash>().session.lock().unwrap().replace(s);
    Ok(())
}
fn tick(app: &tauri::AppHandle) -> Result<(), String> {
    // Native work and toggle requests are serialized on the main thread.
    let state = app.state::<Stash>();
    let mut guard = state.session.lock().unwrap();
    let Some(s) = guard.as_mut() else {
        return Ok(());
    };
    let t = (s.started.elapsed().as_secs_f64() / DURATION).min(1.);
    let e = 1. - (1. - t).powi(3);
    if s.animating {
        let entries = s.windows.clone();
        let from = s.from.clone();
        let area = s.area;
        let top = s.top;
        let restoring = s.restoring;
        drop(guard);
        for (label, saved) in &entries {
            if let Some(w) = crate::web::managed_window(app, label) {
                let start = from.get(label).unwrap_or(saved);
                let target = if restoring {
                    saved.rect
                } else {
                    compact(saved.rect, area, top)
                };
                let target_alpha = if restoring {
                    saved.alpha
                } else {
                    saved.alpha * 0.1
                };
                geometry(
                    &w,
                    if t >= 1. {
                        target
                    } else {
                        interpolate(start.rect, target, e)
                    },
                    saved.scale,
                )?;
                alpha(&w, start.alpha + (target_alpha - start.alpha) * e)?;
                if t >= 1. && !restoring {
                    w.hide().map_err(err)?;
                    geometry(&w, saved.rect, saved.scale)?;
                    alpha(&w, saved.alpha)?;
                }
            }
        }
        if t >= 1. {
            let mut guard = state.session.lock().unwrap();
            let mut arrangement = None;
            if restoring {
                arrangement = guard.take().and_then(|s| s.after_restore);
            } else if let Some(s) = guard.as_mut() {
                s.animating = false;
                s.tip_until = Instant::now() + Duration::from_secs(6);
            }
            drop(guard);
            if restoring {
                for label in entries
                    .keys()
                    .filter(|label| !label.starts_with("capture-"))
                {
                    if let Some(w) = crate::web::managed_window(app, label) {
                        if crate::opacity::configured(app).is_some() {
                            crate::sync_opacity(w.clone())?;
                        }
                    }
                }
                // Keep capture overlays above restored notes.
                for label in entries.keys().filter(|label| label.starts_with("capture-")) {
                    if let Some(w) = crate::web::managed_window(app, label) {
                        show_frame(&w)?;
                    }
                }
            }
            // Let the native event loop apply the final restore frame before placing windows.
            *state.restored_arrangement.lock().unwrap() = arrangement;
            crate::tray::sync_visibility(app)?;
        }
        return Ok(());
    }
    let w = indicator(app)?;
    let (x, y) = pointer(&w)?;
    let r = rect(&w)?;
    let factor = w.scale_factor().map_err(err)? / unit(w.scale_factor().map_err(err)?);
    let distance = ((x - (r.x + r.w - 24. * factor)).powi(2) + (y - (r.y + 24. * factor)).powi(2))
        .sqrt()
        / factor;
    let now = Instant::now();
    // Monitor replacement cancels the flight and selects a valid work area.
    let screens = areas(&w)?;
    if !screens
        .iter()
        .any(|a| a.contains(s.area.x + s.area.w / 2., s.area.y + s.area.h / 2.))
    {
        if let Some(area) = screens.first() {
            s.area = *area;
            s.flight = None;
            s.displayed = None;
            s.tip_until = now + Duration::from_secs(6);
        }
    }
    if let Some((start, began)) = s.flight {
        let t = (now.duration_since(began).as_secs_f64() / DURATION).min(1.);
        let target = indicator_rect(s.area, s.top, factor);
        let next = if t >= 1. {
            target
        } else {
            interpolate(start, target, t * t * (3. - 2. * t))
        };
        if t >= 1. {
            s.flight = None;
            // Show on the following tick, after the final native move settles.
            s.tip_until = now + Duration::from_secs(6);
            s.displayed = None;
        }
        drop(guard);
        geometry(&w, next, w.scale_factor().map_err(err)?)?;
        return Ok(());
    }
    if s.displayed.is_some() && s.gate.step(distance) {
        s.top = !s.top;
        s.flight = Some((r, now));
        s.displayed = Some((s.top, false));
        drop(guard);
        return set_tip(app, &w, false);
    }
    let display = (s.top, now < s.tip_until);
    if s.displayed == Some(display) {
        return Ok(());
    }
    s.displayed = Some(display);
    let area = s.area;
    drop(guard);
    place_indicator(app, area, display.0, display.1)
}

pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    // Prepare the WebView before any shortcut, avoiding first-toggle activation.
    let icon = indicator(app)?;
    #[cfg(any(target_os = "macos", windows))]
    crate::focus::restore(&icon);
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = icon;
    let key = if cfg!(target_os = "macos") {
        "Ctrl+Alt+Cmd"
    } else {
        "Ctrl+Alt+Win"
    };
    // Modifier-only chords cannot be registered with RegisterHotKey/global-hotkey.
    // Coordinate participating Gibbon instances using one per-user lease.
    let directory = app.path().app_data_dir().map_err(err)?;
    std::fs::create_dir_all(&directory).map_err(err)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let lease = options
        .open(directory.join("modifier-stash.lock"))
        .map_err(err)?;
    let available = lease.try_lock_exclusive().is_ok();
    let observed = available && crate::stash_input::install(app);
    let owned = available && observed;
    if owned {
        crate::layout::overview::install(app)?;
        tauri::WebviewWindowBuilder::new(
            app,
            "layout-undo",
            tauri::WebviewUrl::App("layout-undo.html".into()),
        )
        .title("정렬 취소하기")
        .accept_first_mouse(true)
        .inner_size(144., 40.)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .visible(false)
        .focused(false)
        .focusable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
        .map_err(err)?;
        let guide = tauri::WebviewWindowBuilder::new(
            app,
            GUIDE,
            tauri::WebviewUrl::App("direction-guide.html".into()),
        )
        .title(format!(
            "{} — 방향 안내",
            crate::instance::current().title()
        ))
        .inner_size(232., 232.)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .visible(false)
        .focused(false)
        .focusable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
        .map_err(err)?;
        guide.set_ignore_cursor_events(true).map_err(err)?;
        *app.state::<Stash>().lease.lock().unwrap() = Some(lease);
    }
    *app.state::<Stash>().shortcut.lock().unwrap() = if owned {
        key.into()
    } else if !available {
        format!("{key} 사용 중 · 트레이로 실행")
    } else {
        format!("{key} 입력 감지 권한 필요 · 트레이로 실행")
    };
    let (mask, _) = modifiers();
    *app.state::<Stash>().chord.lock().unwrap() = Chord {
        latched: mask != 0,
        cancelled: mask != 0,
        pending: false,
        mask,
    };
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(16));
        let handle = app.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if app
            .run_on_main_thread(move || {
                #[cfg(not(target_os = "macos"))]
                poll_shortcut(&handle);
                crate::layout::expire_undo(&handle);
                let pending = handle
                    .state::<Stash>()
                    .restored_arrangement
                    .lock()
                    .unwrap()
                    .take();
                if let Some((direction, x, y)) = pending {
                    if !active(&handle) {
                        arrange_direction(&handle, direction, x, y);
                    }
                }
                #[cfg(target_os = "macos")]
                if let Some(panel) = handle.get_webview_window("memo-overview") {
                    let _ = crate::toolbar_pointer::sync(&panel);
                }
                sample_pointer(&handle);
                if let Err(e) = tick(&handle) {
                    eprintln!("접어두기 전환 실패: {e}");
                    let _ = toggle(&handle);
                }
                let _ = tx.send(());
            })
            .is_err()
        {
            break;
        }
        // Native menus/dialogs can delay dispatch; keep the worker alive.
        if rx.recv().is_err() {
            break;
        }
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn left_alignment_is_latched_even_if_pointer_later_crosses_right() {
        let mut gesture = Gesture {
            origin: Some((100., 100.)),
            scale: 1.,
            ..Default::default()
        };
        gesture.track(40., 105.);
        gesture.track(180., 100.);
        assert_eq!(gesture.direction, Some(Direction::Left));
    }
    #[test]
    fn direction_uses_screen_scale_and_locks_the_first_valid_axis() {
        let mut g = Gesture {
            origin: Some((-60., 100.)),
            scale: 2.,
            ..Default::default()
        };
        g.track(0., 100.);
        assert!(!g.moved);
        g.track(20., 180.);
        assert!(g.moved && g.direction.is_none());
        g.track(20., 100.);
        assert_eq!(g.direction, Some(Direction::Right));
        g.track(-60., 0.);
        assert_eq!(g.direction, Some(Direction::Right));
    }
    #[test]
    fn repeat_chord_keeps_the_guide_until_the_new_anchor_is_known() {
        let mut chord = Chord::default();
        for _ in 0..2 {
            for mask in [1, 3, 7, 7, 3, 7, 1] {
                assert_eq!(chord.guide_input(mask, false, false), GuideInput::Hold);
            }
            assert_eq!(chord.guide_input(0, false, false), GuideInput::Open);
            assert_eq!(chord.guide_input(0, false, false), GuideInput::Hold);
        }
        assert_eq!(chord.guide_input(1, false, false), GuideInput::Hold);
        assert_eq!(chord.guide_input(0, false, false), GuideInput::Cancel);
        assert_eq!(chord.guide_input(7, true, false), GuideInput::Cancel);
        assert_eq!(chord.guide_input(0, false, false), GuideInput::Cancel);
        assert_eq!(chord.guide_input(7, false, true), GuideInput::Cancel);
        assert_eq!(chord.guide_input(0, false, false), GuideInput::Cancel);
    }
    #[test]
    fn bare_modifier_chord_commits_once_on_complete_release() {
        for masks in [[1, 3, 7], [2, 6, 7], [4, 5, 7]] {
            let mut c = Chord::default();
            for m in masks {
                assert!(!c.sample(m, false, false));
            }
            for m in [7, 7, 3, 7, 1] {
                assert!(!c.sample(m, false, false));
            }
            assert!(c.sample(0, false, false));
            assert!(!c.sample(0, false, false));
        }
    }
    #[test]
    fn other_shortcuts_clicks_shift_and_preheld_keys_cancel_until_release() {
        for cancel_at in 0..4 {
            let mut c = Chord::default();
            for (i, m) in [1, 3, 7, 7].into_iter().enumerate() {
                assert!(!c.sample(m, false, i == cancel_at));
            }
            assert!(!c.sample(0, false, false));
            assert!(!c.sample(7, false, false));
            assert!(c.sample(0, false, false));
        }
        let mut c = Chord::default();
        assert!(!c.sample(7, true, false));
        assert!(!c.sample(7, false, false));
        assert!(!c.sample(0, false, false));
        assert!(!c.sample(3, false, false));
        assert!(!c.sample(0, false, false));
    }
    #[test]
    fn opacity_wheel_cancels_stash_until_all_modifiers_are_released() {
        let mut c = Chord::default();
        for mask in [1, 3, 7] {
            assert!(!c.sample(mask, false, false));
        }
        // The shared observer marks any wheel as mixed input, even at an
        // opacity limit or when a fractional notch has not changed the value.
        assert!(!c.sample(7, false, true));
        for mask in [3, 7, 1, 0] {
            assert!(!c.sample(mask, false, false));
        }
        assert!(!c.sample(7, false, false));
        assert!(c.sample(0, false, false));
    }
    #[test]
    fn derived_connections_never_enter_the_memo_stash_snapshot() {
        for label in [
            "stash-indicator",
            GUIDE,
            "layout-undo",
            "relation-prototype-0",
            "relation-prototype-7",
        ] {
            assert!(!snapshot_candidate(label));
        }
        for label in [
            "main",
            "note-one",
            "capture-0",
            "terminal-1",
            "webview-web-one",
            "manager",
        ] {
            assert!(snapshot_candidate(label));
        }
    }
    #[test]
    fn reverse_animation_has_exact_endpoints_and_continuous_retarget() {
        let r = Rect {
            x: -900.,
            y: 200.,
            w: 300.,
            h: 240.,
        };
        let area = Rect {
            x: 0.,
            y: 25.,
            w: 1440.,
            h: 875.,
        };
        let small = compact(r, area, true);
        let mid = interpolate(small, r, 0.5);
        assert!(mid.w > small.w && mid.w < r.w);
        let retarget = interpolate(mid, small, 0.0);
        assert_eq!((retarget.x, retarget.y, retarget.w), (mid.x, mid.y, mid.w));
        let end = interpolate(small, r, 1.0);
        assert_eq!((end.x, end.y, end.w, end.h), (r.x, r.y, r.w, r.h));
    }
    #[test]
    fn reentry_during_hint_does_not_wait_for_hint_or_cooldown() {
        let mut gate = EscapeGate { armed: true };
        for _ in 0..3 {
            assert!(gate.step(20.));
            assert!(!gate.step(100.));
        }
    }
    #[test]
    fn escape_requires_fresh_exit_and_reentry_without_pointer_chase() {
        let mut gate = EscapeGate { armed: true };
        assert!(!gate.step(56.));
        assert!(gate.step(55.));
        // Arrival under a stationary pointer, and jitter in the hysteresis
        // band, must not schedule another flight even after the hint expires.
        for distance in [0., 55., 56., 70., 96., 20., 0.] {
            assert!(!gate.step(distance));
        }
        assert!(!gate.step(97.));
        assert!(!gate.step(56.));
        assert!(gate.step(55.));
        assert!(!gate.step(55.));
    }
    #[test]
    fn unplugged_and_oversized_rectangles_stay_in_work_area() {
        let a = Rect {
            x: -1440.,
            y: 25.,
            w: 1440.,
            h: 875.,
        };
        let r = Rect {
            x: 2000.,
            y: 1000.,
            w: 2000.,
            h: 1000.,
        }
        .clamp(a);
        assert_eq!((r.x, r.y, r.w, r.h), (-1440., 25., 1440., 875.));
        let r = Rect {
            x: -900.,
            y: 30.,
            w: 300.,
            h: 12.,
        }
        .clamp(a);
        assert_eq!((r.x, r.y, r.h), (-900., 30., 12.));
    }
}
