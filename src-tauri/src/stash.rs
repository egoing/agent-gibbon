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
    scale: f64,
    alpha: f64,
}
struct Session {
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
    shortcut: Mutex<String>,
    chord: Mutex<Chord>,
    lease: Mutex<Option<std::fs::File>>,
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
impl Chord {
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
    let state = app.state::<Stash>();
    let fire = state
        .chord
        .lock()
        .unwrap()
        .sample(mask, shift, crate::stash_input::used());
    if fire && state.lease.lock().unwrap().is_some() {
        trigger(app);
    }
}
#[cfg(target_os = "macos")]
pub fn cancel_chord(app: &tauri::AppHandle) {
    let state = app.state::<Stash>();
    let mut chord = state.chord.lock().unwrap();
    if chord.mask != 0 {
        chord.cancelled = true;
        chord.pending = false;
    }
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
fn show_frame(w: &WebviewWindow) -> Result<(), String> {
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
    last: Option<Instant>,
}
impl EscapeGate {
    fn step(&mut self, distance: f64, now: Instant, tip_visible: bool) -> bool {
        // The initial hint must not swallow the first approach. Only a hint
        // after an escape protects reading time; do not queue reentry in it.
        if self.last.is_some() && tip_visible {
            return false;
        }
        if distance > 96.0 {
            self.armed = true;
        }
        if self.armed
            && distance < 56.0
            && self
                .last
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(2))
        {
            self.armed = false;
            self.last = Some(now);
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
        w.set_size(tauri::PhysicalSize::new(
            r.w.round() as u32,
            r.h.round() as u32,
        ))
        .map_err(err)
    }
}
fn alpha(w: &WebviewWindow, value: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    unsafe {
        (&*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow)).setAlphaValue(value);
    }
    #[cfg(not(target_os = "macos"))]
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
    serde_json::json!({"active":s.is_some(),"animating":s.as_ref().is_some_and(|s|s.animating),"restoring":s.as_ref().is_some_and(|s|s.restoring),"indicator_moving":s.as_ref().is_some_and(|s|s.flight.is_some()),"tip_visible":s.as_ref().is_some_and(|s|s.flight.is_none() && s.displayed.is_some_and(|d|d.1)),"windows":s.as_ref().map(|s|s.windows.keys().collect::<Vec<_>>()),"shortcut":shortcut(app),"indicator":app.get_webview_window(ICON).and_then(|w|rect(&w).ok()).map(|r|serde_json::json!({"x":r.x,"y":r.y,"width":r.w,"height":r.h}))})
}
// Explicit individual open/close takes ownership away from the transaction.
pub fn release(w: &WebviewWindow) -> Result<(), String> {
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
        format!("{} · 다시 누르면 창 복원", shortcut(app))
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
    let previous = app.state::<Stash>().session.lock().unwrap().take();
    if let Some(mut s) = previous {
        let restoring = !s.restoring;
        let mut from = BTreeMap::new();
        for (label, saved) in &mut s.windows {
            if let Some(w) = app.get_webview_window(label) {
                saved.rect = restore_rect(&w, saved)?;
                let frame = if !s.animating {
                    Saved {
                        rect: compact(saved.rect, s.area, s.top),
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
            if let Some(w) = app.get_webview_window(&label) {
                crate::feedback_window::hide(&w);
                geometry(&w, frame.rect, frame.scale)?;
                alpha(&w, frame.alpha)?;
                show_frame(&w)?;
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
    for w in app.webview_windows().into_values() {
        if w.label() == ICON
            || crate::feedback_window::is_surface(w.label())
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
            #[cfg(not(target_os = "macos"))]
            {
                1.0
            }
        };
        windows.insert(
            w.label().into(),
            Saved {
                rect: rect(&w)?,
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
        from: windows.clone(),
        windows,
        restoring: false,
        started: now,
        animating: true,
        area,
        top: false,
        gate: EscapeGate {
            armed: true,
            last: None,
        },
        tip_until: now,
        displayed: None,
        flight: None,
    };
    app.state::<Stash>().session.lock().unwrap().replace(s);
    for w in app.webview_windows().into_values().filter(owns) {
        crate::feedback_window::hide(&w);
    }
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
            if let Some(w) = app.get_webview_window(label) {
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
            if restoring {
                guard.take();
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
                    if let Some(w) = app.get_webview_window(label) {
                        crate::feedback_window::sync(&w);
                    }
                }
                // Feedback can order above notes; keep capture overlays last.
                for label in entries.keys().filter(|label| label.starts_with("capture-")) {
                    if let Some(w) = app.get_webview_window(label) {
                        show_frame(&w)?;
                    }
                }
            }
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
    if s.displayed.is_some() && s.gate.step(distance, now, now < s.tip_until) {
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
    fn escape_requires_exit_and_cooldown() {
        let now = Instant::now();
        let mut g = EscapeGate {
            armed: true,
            last: None,
        };
        assert!(
            g.step(20., now, true),
            "initial hint permits first approach"
        );
        assert!(!g.step(20., now + Duration::from_secs(3), true));
        assert!(!g.step(100., now + Duration::from_secs(3), true));
        assert!(
            !g.step(20., now + Duration::from_secs(7), false),
            "protected exit is not queued"
        );
        assert!(!g.step(96., now + Duration::from_secs(7), false));
        assert!(!g.step(100., now + Duration::from_secs(7), false));
        assert!(!g.step(56., now + Duration::from_secs(7), false));
        assert!(g.step(20., now + Duration::from_secs(7), false));
        g.step(100., now + Duration::from_secs(7), false);
        assert!(!g.step(20., now + Duration::from_millis(7500), false));
        assert!(g.step(20., now + Duration::from_secs(9), false));
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
