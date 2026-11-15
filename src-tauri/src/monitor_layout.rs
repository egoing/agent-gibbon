//! Local display recovery. Never opens, focuses, closes or recreates a window.
//! Coordinates are desktop points on AppKit, physical desktop pixels elsewhere.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}
impl Frame {
    fn right(self) -> i64 {
        self.x + self.width
    }
    fn bottom(self) -> i64 {
        self.y + self.height
    }
    fn valid(self) -> bool {
        self.x.unsigned_abs() <= 10_000_000
            && self.y.unsigned_abs() <= 10_000_000
            && (1..=200_000).contains(&self.width)
            && (1..=200_000).contains(&self.height)
    }
    fn inside(self, area: Self) -> bool {
        self.x >= area.x
            && self.y >= area.y
            && self.right() <= area.right()
            && self.bottom() <= area.bottom()
    }
    fn overlaps(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
    fn distance(self, other: Self) -> i128 {
        i128::from(self.x - other.x).pow(2) + i128::from(self.y - other.y).pow(2)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    name: Option<String>,
    bounds: Frame,
    work: Frame,
    scale: f64,
}
impl Screen {
    fn valid(&self) -> bool {
        self.bounds.valid()
            && self.work.valid()
            && self.scale.is_finite()
            && (0.25..=8.).contains(&self.scale)
            && self.name.as_ref().is_none_or(|name| name.len() <= 1024)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Home {
    frame: Frame,
    screen: Screen,
    #[serde(default)]
    unfolded_height: Option<i64>,
}
impl Home {
    fn valid(&self) -> bool {
        self.frame.valid()
            && self.screen.valid()
            && self
                .unfolded_height
                .is_none_or(|h| (1..=200_000).contains(&h))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
    last: Frame,
    screen: Screen,
    home: Option<Home>,
    folded: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    records: BTreeMap<String, Record>,
}
// Explicit arrangement/zoom owns geometry, but undo must recover its previous
// temporary-home relationship as well as the native frame.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RecoveryPoint(Option<Record>);
impl RecoveryPoint {
    pub fn validate(&self) -> bool {
        self.0.as_ref().is_none_or(|r| {
            r.last.valid() && r.screen.valid() && r.home.as_ref().is_none_or(Home::valid)
        })
    }
}
pub fn begin_override(w: &WebviewWindow) -> Result<RecoveryPoint, String> {
    let actual = read(w)?;
    let state = w.app_handle().state::<State>();
    let mut s = state.0.lock().map_err(|e| e.to_string())?;
    let previous = RecoveryPoint(s.saved.records.get(w.label()).cloned());
    if let Some(r) = s.saved.records.get_mut(w.label()) {
        r.home = None;
        r.last = actual;
    }
    s.manual.insert(w.label().into());
    Ok(previous)
}
pub fn restore_override(w: &WebviewWindow, previous: RecoveryPoint) -> Result<(), String> {
    let actual = read(w)?;
    let screens = screens(w.app_handle())?;
    let state = w.app_handle().state::<State>();
    let mut s = state.0.lock().map_err(|e| e.to_string())?;
    s.manual.remove(w.label());
    if let Some(mut record) = previous.0 {
        record.last = actual;
        record.screen = screens[nearest(actual, &screens)].clone();
        s.saved.records.insert(w.label().into(), record);
    } else {
        s.saved.records.remove(w.label());
    }
    Ok(())
}
impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.records.len() > 1000
            || self.records.iter().any(|(label, r)| {
                !supported(label)
                    || !r.last.valid()
                    || !r.screen.valid()
                    || r.home.as_ref().is_some_and(|h| !h.valid())
            })
        {
            return Err("모니터 복원 상태가 유효하지 않습니다".into());
        }
        Ok(())
    }
}
#[derive(Default)]
struct Runtime {
    saved: Saved,
    screens: Vec<Screen>,
    pending: Option<(Vec<Screen>, Instant)>,
    // Set by trusted explicit drag/resize commands or native pointer movement.
    manual: std::collections::BTreeSet<String>,
    error: Option<String>,
    crowded: bool,
}
impl Runtime {
    fn stable(&mut self, current: &[Screen], now: Instant) -> bool {
        if self.screens == current {
            self.pending = None;
            return true;
        }
        match &self.pending {
            Some((pending, at)) if pending == current => {
                now.duration_since(*at) >= Duration::from_millis(750)
            }
            _ => {
                self.pending = Some((current.to_vec(), now));
                false
            }
        }
    }
}
#[derive(Default)]
pub struct State(Mutex<Runtime>);

fn supported(label: &str) -> bool {
    (label == "main"
        || ["note-", "terminal-", "webview-"]
            .iter()
            .any(|p| label.starts_with(p)))
        && label.len() <= 140
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn unit(scale: f64) -> f64 {
    if cfg!(target_os = "macos") {
        scale
    } else {
        1.
    }
}
fn frame(p: tauri::PhysicalPosition<i32>, s: tauri::PhysicalSize<u32>, scale: f64) -> Frame {
    let u = unit(scale);
    Frame {
        x: (f64::from(p.x) / u).round() as i64,
        y: (f64::from(p.y) / u).round() as i64,
        width: (f64::from(s.width) / u).round() as i64,
        height: (f64::from(s.height) / u).round() as i64,
    }
}
fn screens(app: &tauri::AppHandle) -> Result<Vec<Screen>, String> {
    let mut result: Vec<_> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| {
            let work = m.work_area();
            Screen {
                name: m.name().cloned(),
                bounds: frame(*m.position(), *m.size(), m.scale_factor()),
                work: frame(work.position, work.size, m.scale_factor()),
                scale: m.scale_factor(),
            }
        })
        .collect();
    if result.is_empty() || result.iter().any(|s| !s.valid()) {
        return Err("사용 가능한 화면 작업 영역을 확인하지 못했습니다".into());
    }
    result
        .sort_by(|a, b| (a.bounds.x, a.bounds.y, &a.name).cmp(&(b.bounds.x, b.bounds.y, &b.name)));
    Ok(result)
}
fn read(w: &WebviewWindow) -> Result<Frame, String> {
    Ok(frame(
        w.outer_position().map_err(|e| e.to_string())?,
        w.outer_size().map_err(|e| e.to_string())?,
        w.scale_factor().map_err(|e| e.to_string())?,
    ))
}
fn nearest(rect: Frame, screens: &[Screen]) -> usize {
    screens
        .iter()
        .enumerate()
        .min_by_key(|(_, s)| {
            let x = rect.x.clamp(s.work.x, s.work.right());
            let y = rect.y.clamp(s.work.y, s.work.bottom());
            i128::from(x - rect.x).pow(2) + i128::from(y - rect.y).pow(2)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}
// Never identify a monitor by a potentially duplicated human name alone.
fn matching(screen: &Screen, screens: &[Screen]) -> Option<usize> {
    let mut matches = screens.iter().enumerate().filter(|(_, s)| *s == screen);
    let found = matches.next()?.0;
    matches.next().is_none().then_some(found)
}
// A spanning window stays untouched when the union of adjacent work areas
// covers it. Merely intersecting one screen is not sufficient.
fn covered(rect: Frame, screens: &[Screen]) -> bool {
    let mut xs = vec![rect.x, rect.right()];
    for s in screens {
        xs.extend([
            s.work.x.clamp(rect.x, rect.right()),
            s.work.right().clamp(rect.x, rect.right()),
        ]);
    }
    xs.sort_unstable();
    xs.dedup();
    xs.windows(2).all(|xs| {
        let mut spans: Vec<_> = screens
            .iter()
            .filter(|s| s.work.x <= xs[0] && s.work.right() >= xs[1])
            .map(|s| (s.work.y, s.work.bottom()))
            .collect();
        spans.sort_unstable();
        let mut end = rect.y;
        for (a, b) in spans {
            if a > end {
                break;
            }
            end = end.max(b);
        }
        end >= rect.bottom()
    })
}

struct Plan {
    dest: Frame,
    target: usize,
    returning: bool,
    rescued: bool,
}
fn plan(
    r: &mut Record,
    actual: Frame,
    current: &[Screen],
    changed: bool,
    manual: bool,
    image: bool,
) -> Plan {
    let manual = manual && actual != r.last;
    if manual {
        r.home = None;
        r.last = actual;
        r.screen = current[nearest(actual, current)].clone();
    }
    let origin = r.home.clone().unwrap_or(Home {
        frame: r.last,
        screen: r.screen.clone(),
        unfolded_height: None,
    });
    let returning = changed && r.home.is_some() && matching(&origin.screen, current).is_some();
    let source = if changed && !manual {
        origin.frame
    } else {
        actual
    };
    let target = matching(&origin.screen, current)
        .filter(|_| changed && !manual)
        .unwrap_or_else(|| nearest(source, current));
    let desired = if changed && !manual {
        resized(source, &origin.screen, &current[target])
    } else {
        source
    };
    let dest = if covered(desired, current) {
        desired
    } else {
        fit(desired, current[target].work, image)
    };
    let rescued = dest != desired || (changed && !manual && dest != origin.frame);
    if r.home.is_none() && (rescued || dest != actual) && !manual {
        r.home = Some(origin);
    }
    Plan {
        dest,
        target,
        returning,
        rescued,
    }
}
fn presentation(r: &mut Record, actual: Frame, folded: bool) {
    if r.folded == folded {
        return;
    }
    if let Some(h) = &mut r.home {
        if folded {
            h.unfolded_height = Some(h.frame.height);
            h.frame.height = resized(actual, &r.screen, &h.screen).height;
        } else {
            h.frame.height = h
                .unfolded_height
                .take()
                .unwrap_or_else(|| resized(actual, &r.screen, &h.screen).height);
        }
    }
    r.last.height = actual.height;
    r.folded = folded;
}
fn return_expanded(w: &WebviewWindow, r: &Record) {
    if let Some(h) = &r.home {
        if let Some(height) = h.unfolded_height {
            let factor = unit(h.screen.scale) / h.screen.scale;
            crate::sizing::recover_expanded(
                w,
                tauri::LogicalSize::new(h.frame.width as f64 * factor, height as f64 * factor),
            );
        }
    }
}
fn resized(rect: Frame, from: &Screen, to: &Screen) -> Frame {
    // Preserve logical size while Windows changes the native DPI of a window.
    let ratio = (to.scale / unit(to.scale)) / (from.scale / unit(from.scale));
    Frame {
        width: (rect.width as f64 * ratio).round().max(1.) as i64,
        height: (rect.height as f64 * ratio).round().max(1.) as i64,
        ..rect
    }
}
fn fit(rect: Frame, area: Frame, image: bool) -> Frame {
    let mut r = rect;
    if image {
        let factor = 1_f64
            .min(area.width as f64 / r.width as f64)
            .min(area.height as f64 / r.height as f64);
        r.width = (r.width as f64 * factor).floor().max(1.) as i64;
        r.height = (r.height as f64 * factor).floor().max(1.) as i64;
    } else {
        r.width = r.width.min(area.width);
        r.height = r.height.min(area.height);
    }
    r.x = r.x.clamp(area.x, area.right() - r.width);
    r.y = r.y.clamp(area.y, area.bottom() - r.height);
    r
}
// Move rescued windows only. Preserve every unchanged window as an obstacle.
// Candidate edges include chrome-sized steps so overlapping bodies can still
// expose toolbars when a full-body packing would unnecessarily fail.
fn distribute(rect: Frame, area: Frame, obstacles: &[Frame], chrome: i64) -> (Frame, bool) {
    if !obstacles.iter().any(|o| rect.overlaps(*o)) {
        return (rect, false);
    }
    let mut xs = vec![rect.x, area.x, area.right() - rect.width];
    let mut ys = vec![rect.y, area.y, area.bottom() - rect.height];
    for o in obstacles {
        xs.extend([o.x - rect.width, o.right()]);
        ys.extend([o.y - rect.height, o.bottom(), o.y + chrome]);
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    // Bound collision search on the UI thread even for a large journal.
    xs.sort_by_key(|x| (*x - rect.x).unsigned_abs());
    xs.truncate(48);
    ys.sort_by_key(|y| (*y - rect.y).unsigned_abs());
    ys.truncate(48);
    let mut candidates = Vec::new();
    for x in xs {
        for &y in &ys {
            let r = Frame { x, y, ..rect };
            if r.inside(area) {
                candidates.push(r);
            }
        }
    }
    candidates.sort_by_key(|r| r.distance(rect));
    if let Some(r) = candidates
        .iter()
        .find(|r| !obstacles.iter().any(|o| r.overlaps(*o)))
    {
        return (*r, false);
    }
    // Keep complete top and bottom control strips clear in either stacking
    // order. The manager's window list is the fallback if capacity is exhausted.
    let strip = |r: Frame| Frame {
        height: chrome.min(r.height),
        ..r
    };
    let bottom = |r: Frame| Frame {
        y: r.bottom() - chrome.min(r.height),
        height: chrome.min(r.height),
        ..r
    };
    if let Some(r) = candidates.into_iter().find(|r| {
        !obstacles.iter().any(|o| {
            strip(*r).overlaps(*o)
                || bottom(*r).overlaps(*o)
                || strip(*o).overlaps(*r)
                || bottom(*o).overlaps(*r)
        })
    }) {
        return (r, false);
    }
    (rect, true)
}

pub fn initialize(app: &tauri::AppHandle) {
    app.manage(State::default());
}
pub fn restore(app: &tauri::AppHandle, saved: Saved) -> Result<(), String> {
    saved.validate()?;
    app.state::<State>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .saved = saved;
    Ok(())
}
pub fn snapshot(app: &tauri::AppHandle) -> Saved {
    app.state::<State>()
        .0
        .lock()
        .map(|s| s.saved.clone())
        .unwrap_or_default()
}
pub fn ready(app: &tauri::AppHandle) -> bool {
    app.state::<State>()
        .0
        .lock()
        .is_ok_and(|s| s.pending.is_none() && !s.screens.is_empty() && s.error.is_none())
}
pub fn inspect(app: &tauri::AppHandle) -> serde_json::Value {
    let state = app.state::<State>();
    let s = state.0.lock().unwrap();
    serde_json::json!({"pending":s.pending.is_some(),"error":s.error,"crowded":s.crowded,"screens":s.screens,
        "temporary_windows":s.saved.records.iter().filter(|(_,r)|r.home.is_some()).map(|(label,_)|label).collect::<Vec<_>>()})
}
pub fn manual(w: &WebviewWindow) {
    let Some(state) = w.app_handle().try_state::<State>() else {
        return;
    };
    if let Ok(mut s) = state.0.try_lock() {
        s.manual.insert(w.label().into());
    };
}
// Native drags on terminal chrome and OS resize borders bypass app commands.
pub fn native_event(w: &tauri::Window, event: &tauri::WindowEvent) {
    if !matches!(
        event,
        tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
    ) {
        return;
    }
    if mouse_held() {
        let app = w.app_handle();
        if crate::stash::active(app) || crate::capture::active(app) {
            return;
        }
        let Some(state) = app.try_state::<State>() else {
            return;
        };
        let Ok(s) = state.0.try_lock() else {
            return;
        };
        // Ignore display-driven events, including when the user happens to
        // have a mouse button held during a monitor transition.
        if s.pending.is_some() || screens(app).ok().as_ref() != Some(&s.screens) {
            return;
        }
        drop(s);
        if let Some(window) = crate::web::managed_window(app, w.label()) {
            manual(&window);
        }
    }
}
fn mouse_held() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSEvent::pressedMouseButtons() & 1 != 0
    }
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetAsyncKeyState(key: i32) -> i16;
        }
        unsafe { GetAsyncKeyState(1) < 0 }
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        false
    }
}

fn apply(w: &WebviewWindow, r: Frame, screen: &Screen) -> Result<Frame, String> {
    let u = unit(screen.scale);
    let p = tauri::PhysicalPosition::new(
        (r.x as f64 * u).round() as i32,
        (r.y as f64 * u).round() as i32,
    );
    let size = tauri::PhysicalSize::new(
        (r.width as f64 * u).round() as u32,
        (r.height as f64 * u).round() as u32,
    );
    // Temporarily permit a small work area without changing the normal minimum
    // contract. Fold's expanded size and native image constraint remain intact.
    let scale = screen.scale;
    let (mw, mh) = if crate::sizing::image_size(w).is_some() {
        (2., 2.)
    } else if crate::sizing::folded(w) {
        (144., 24.)
    } else if w.label().starts_with("webview-") {
        (240., 160.)
    } else {
        (144., 32.)
    };
    let min = tauri::LogicalSize::new(mw, mh).to_physical::<u32>(scale);
    w.set_min_size(None::<tauri::PhysicalSize<u32>>)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        #[cfg(windows)]
        {
            w.set_position(p).map_err(|e| e.to_string())?;
            crate::windows_window::set_outer_size(w, size)?;
        }
        #[cfg(target_os = "macos")]
        crate::entrance::place(
            w,
            crate::entrance::Destination {
                position: p,
                size,
                scale: screen.scale,
            },
        )?;
        #[cfg(not(any(target_os = "macos", windows)))]
        w.set_size(size).map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        w.set_position(p).map_err(|e| e.to_string())?;
        read(w)
    })();
    // Do not reinstate a minimum greater than the temporary actual client size:
    // some window managers would immediately enlarge it again.
    let client = w.inner_size().map_err(|e| e.to_string())?;
    w.set_min_size(Some(tauri::PhysicalSize::new(
        min.width.min(client.width),
        min.height.min(client.height),
    )))
    .map_err(|e| e.to_string())?;
    result
}

/// Run only on the UI thread. The session journal calls this every 250ms.
pub fn tick(app: &tauri::AppHandle) -> Result<(), String> {
    let current = screens(app)?;
    let state = app.state::<State>();
    let mut s = state.0.lock().map_err(|e| e.to_string())?;
    if !s.stable(&current, Instant::now()) {
        return Ok(());
    }
    // Avoid replacing a stash/entrance transaction or a live user drag.
    let windows = crate::web::managed_windows(app);
    if crate::stash::active(app)
        || crate::stash::chord_busy(app)
        || crate::capture::active(app)
        || crate::privacy::onboarding_required(app)
        || crate::updater::frozen(app)
        || mouse_held()
        || windows.values().any(|w| {
            supported(w.label()) && (crate::entrance::active(w) || crate::sizing::pending(w))
        })
    {
        return Ok(());
    }
    let changed = s.screens != current;
    let mut obstacles = Vec::new();
    let mut plans = Vec::new();
    let mut records = s.saved.records.clone();
    records.retain(|label, _| windows.contains_key(label));
    for (label, w) in windows.iter().filter(|(label, _)| supported(label)) {
        if w.is_minimized().map_err(|e| e.to_string())? {
            continue;
        }
        let actual = read(w)?;
        let folded = crate::sizing::folded(w);
        let screen = &current[nearest(actual, &current)];
        let r = records.entry(label.clone()).or_insert_with(|| Record {
            last: actual,
            screen: screen.clone(),
            home: None,
            folded,
        });
        let manual = s.manual.remove(label);
        // Fold changes only the presentation height, not the remembered home
        // position or expanded size. Unfold is subsequently fitted if needed.
        presentation(r, actual, folded);
        let Plan {
            mut dest,
            target,
            returning,
            rescued,
        } = plan(
            r,
            actual,
            &current,
            changed,
            manual,
            crate::sizing::image_size(w).is_some()
                && !folded
                && !crate::sizing::work_area_active(w),
        );
        let target_screen = &current[target];
        if changed && crate::sizing::work_area_active(w) && !folded {
            if r.home.is_none() && target_screen.work != r.last {
                r.home = Some(Home {
                    frame: r.last,
                    screen: r.screen.clone(),
                    unfolded_height: None,
                });
            }
            dest = target_screen.work;
        }
        let visible = w.is_visible().map_err(|e| e.to_string())?;
        // Hidden windows are geometrically safe when later shown, but never
        // participate as visible obstacles or acquire visibility/focus here.
        let needs = dest != actual;
        if needs || rescued {
            plans.push((label.clone(), dest, target, visible, returning));
        } else {
            if visible {
                obstacles.push(actual);
            }
            r.last = actual;
            r.screen = target_screen.clone();
            if returning {
                return_expanded(w, r);
                r.home = None;
            }
        }
        // An old journal with no monitor data is treated as current geometry;
        // its original offscreen position is still remembered by the fit above.
    }
    if changed || !plans.is_empty() {
        s.crowded = false;
    }
    for (label, mut dest, target, visible, returning) in plans {
        let w = &windows[&label];
        let screen = &current[target];
        let requested = dest;
        if visible && !covered(dest, &current) {
            // fitted destinations are contained below
            s.crowded = true;
        }
        if visible && dest.inside(screen.work) {
            let chrome = (30. * screen.scale / unit(screen.scale)).ceil() as i64;
            let (r, crowded) = distribute(dest, screen.work, &obstacles, chrome);
            dest = r;
            s.crowded |= crowded;
            obstacles.push(dest);
        }
        let actual = apply(w, dest, screen)?;
        if !covered(actual, &current) {
            return Err("운영체제의 크기 제약으로 창을 작업 영역 안에 맞추지 못했습니다. 메모 관리에서 해당 창을 선택해 복원하세요".into());
        }
        let r = records.get_mut(&label).unwrap();
        r.last = actual;
        r.screen = screen.clone();
        if returning && dest == requested && actual == requested {
            return_expanded(w, r);
            r.home = None;
        }
    }
    s.saved.records = records;
    s.screens = current;
    s.pending = None;
    s.error = None;
    Ok(())
}
pub fn poll(app: &tauri::AppHandle) {
    if let Err(error) = tick(app) {
        let state = app.state::<State>();
        let mut s = state.0.lock().unwrap();
        if s.error.as_ref() != Some(&error) {
            eprintln!("화면 배치 보정 보류: {error}");
        }
        s.error = Some(error);
        s.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i64, y: i64, w: i64, h: i64) -> Frame {
        Frame {
            x,
            y,
            width: w,
            height: h,
        }
    }
    fn screen(name: &str, x: i64, y: i64, w: i64, h: i64, scale: f64) -> Screen {
        Screen {
            name: Some(name.into()),
            bounds: rect(x, y, w, h),
            work: rect(x, y + 24, w, h - 64),
            scale,
        }
    }
    fn record(f: Frame, s: &Screen) -> Record {
        Record {
            last: f,
            screen: s.clone(),
            home: None,
            folded: false,
        }
    }
    fn acknowledge(r: &mut Record, p: &Plan, screens: &[Screen]) {
        r.last = p.dest;
        r.screen = screens[p.target].clone();
        if p.returning {
            r.home = None;
        }
    }
    #[test]
    fn valid_and_adjacent_spanning_windows_do_not_move() {
        let screens = [
            screen("left", -1000, 0, 1000, 800, 2.),
            screen("right", 0, 0, 1000, 800, 2.),
        ];
        for f in [rect(-900, 50, 300, 200), rect(-100, 100, 300, 200)] {
            let mut r = record(f, &screens[0]);
            let p = plan(&mut r, f, &screens, true, false, false);
            assert_eq!(p.dest, f);
            assert!(r.home.is_none());
        }
        let hole = [screen("left", -1000, 0, 900, 800, 2.), screens[1].clone()];
        assert!(!covered(rect(-150, 100, 300, 200), &hole));
    }
    #[test]
    fn detach_os_move_and_reconnect_preserve_home_across_restart() {
        let primary = screen("primary", 0, 0, 1000, 800, 2.);
        let external = screen("external", 1000, 0, 1000, 800, 2.);
        let home = rect(1400, 100, 300, 200);
        let mut r = record(home, &external);
        let single = [primary.clone()];
        // The OS moved first, before our stable-monitor timer fired.
        let os = rect(700, 100, 300, 200);
        let p = plan(&mut r, os, &single, true, false, false);
        assert_eq!(p.dest, os);
        assert_eq!(r.home.as_ref().unwrap().frame, home);
        acknowledge(&mut r, &p, &single);
        let saved = Saved {
            records: BTreeMap::from([("note-a".into(), r)]),
        };
        let bytes = serde_json::to_vec(&saved).unwrap();
        let mut saved: Saved = serde_json::from_slice(&bytes).unwrap();
        saved.validate().unwrap();
        let r = saved.records.get_mut("note-a").unwrap();
        let both = [primary, external];
        let p = plan(r, os, &both, true, false, false);
        assert!(p.returning);
        assert_eq!(p.dest, home);
    }
    #[test]
    fn user_move_and_resize_override_only_the_touched_window() {
        let primary = screen("p", 0, 0, 1000, 800, 2.);
        let external = screen("e", 1000, 0, 1000, 800, 2.);
        let home = rect(1300, 80, 300, 200);
        let mut untouched = record(home, &external);
        let single = [primary.clone()];
        let p = plan(&mut untouched, home, &single, true, false, false);
        acknowledge(&mut untouched, &p, &single);
        let mut touched = untouched.clone();
        let chosen = rect(200, 180, 450, 250);
        let p = plan(&mut touched, chosen, &single, false, true, false);
        acknowledge(&mut touched, &p, &single);
        assert!(touched.home.is_none());
        let both = [primary, external];
        assert_eq!(
            plan(&mut touched, chosen, &both, true, false, false).dest,
            chosen
        );
        let last = untouched.last;
        assert_eq!(
            plan(&mut untouched, last, &both, true, false, false).dest,
            home
        );
    }
    #[test]
    fn unchanged_drag_click_does_not_discard_temporary_home() {
        let p = screen("p", 0, 0, 1000, 800, 1.);
        let e = screen("e", 1000, 0, 1000, 800, 1.);
        let mut r = record(rect(1300, 80, 300, 200), &e);
        let actual = r.last;
        let screens = [p];
        let p = plan(&mut r, actual, &screens, true, false, false);
        acknowledge(&mut r, &p, &screens);
        let actual = r.last;
        plan(&mut r, actual, &screens, false, true, false);
        assert!(r.home.is_some());
    }
    #[test]
    fn resolution_and_work_area_changes_fit_and_return_original_size() {
        let full = screen("p", 0, 0, 1600, 1000, 1.);
        let small = screen("p", 0, 0, 800, 600, 1.);
        let home = rect(500, 200, 1000, 700);
        let mut r = record(home, &full);
        let screens = [small];
        let p = plan(&mut r, home, &screens, true, false, false);
        assert!(p.dest.inside(screens[0].work));
        assert_eq!(p.dest.width, 800);
        acknowledge(&mut r, &p, &screens);
        let actual = r.last;
        assert_eq!(plan(&mut r, actual, &[full], true, false, false).dest, home);
    }
    #[test]
    fn image_fit_keeps_ratio_and_source_geometry() {
        let area = rect(-800, -500, 800, 400);
        let image = rect(-1000, -600, 1600, 900);
        let r = fit(image, area, true);
        assert!(r.inside(area));
        assert!((r.width as f64 / r.height as f64 - 1600. / 900.).abs() < 0.01);
        assert_eq!(image.width, 1600);
    }
    #[test]
    fn dpi_conversion_preserves_logical_size_without_scaling_desktop_origin() {
        let from = screen("p", -1000, 0, 1000, 800, 2.);
        let to = screen("e", 0, 0, 1000, 800, 1.);
        let r = rect(-100, 40, 600, 400);
        let next = resized(r, &from, &to);
        assert_eq!((next.x, next.y), (r.x, r.y));
        let expected = if cfg!(target_os = "macos") { 600 } else { 300 };
        assert_eq!(next.width, expected);
    }
    #[test]
    fn only_rescue_candidates_move_and_capacity_failure_is_reported() {
        let area = rect(0, 0, 900, 600);
        let obstacle = rect(600, 100, 300, 200);
        let (r, crowded) = distribute(obstacle, area, &[obstacle], 30);
        assert!(!crowded);
        assert!(!r.overlaps(obstacle));
        assert!(r.inside(area));
        assert_eq!(obstacle, rect(600, 100, 300, 200));
        let all = rect(0, 0, 300, 200);
        assert!(distribute(all, all, &[all], 30).1);
    }
    #[test]
    fn duplicated_monitor_identity_and_invalid_journal_are_rejected() {
        let screen = screen("same", 0, 0, 1000, 800, 1.);
        assert_eq!(matching(&screen, &[screen.clone(), screen.clone()]), None);
        let mut r = record(rect(0, 50, 300, 200), &screen);
        r.last.x = i64::MIN;
        assert!(Saved {
            records: BTreeMap::from([("note-a".into(), r)])
        }
        .validate()
        .is_err());
    }
    #[test]
    fn undo_and_zoom_recovery_payload_retains_the_temporary_home() {
        let p = screen("p", 0, 0, 1000, 800, 2.);
        let e = screen("e", 1000, 0, 1000, 800, 2.);
        let home = rect(1400, 80, 300, 200);
        let mut r = record(home, &e);
        let dest = plan(&mut r, home, &[p], true, false, false).dest;
        r.last = dest;
        let point = RecoveryPoint(Some(r));
        assert!(point.validate());
        let bytes = serde_json::to_vec(&point).unwrap();
        let point: RecoveryPoint = serde_json::from_slice(&bytes).unwrap();
        let mut r = point.0.unwrap();
        let p = plan(&mut r, dest, &[e], true, false, false);
        assert_eq!(p.dest, home);
        assert!(p.returning);
    }
    #[test]
    fn rapid_monitor_notifications_wait_for_the_final_stable_configuration() {
        let a = screen("a", 0, 0, 1000, 800, 1.);
        let b = screen("b", 0, 0, 800, 600, 1.);
        let mut state = Runtime::default();
        let now = Instant::now();
        assert!(!state.stable(std::slice::from_ref(&a), now));
        assert!(!state.stable(&[a], now + Duration::from_millis(749)));
        assert!(!state.stable(std::slice::from_ref(&b), now + Duration::from_millis(750)));
        assert!(!state.stable(std::slice::from_ref(&b), now + Duration::from_millis(1499)));
        assert!(state.stable(std::slice::from_ref(&b), now + Duration::from_millis(1500)));
        state.screens = vec![b.clone()];
        assert!(state.stable(&[b], now + Duration::from_millis(1600)));
        assert!(state.pending.is_none());
    }
    #[test]
    fn folding_after_rescue_does_not_replace_the_original_expanded_height() {
        let full = screen("p", 0, 0, 1600, 1000, 2.);
        let small = screen("p", 0, 0, 800, 600, 2.);
        let home = rect(0, 24, 1400, 900);
        let mut r = record(home, &full);
        let smalls = [small.clone()];
        let p = plan(&mut r, home, &smalls, true, false, true);
        acknowledge(&mut r, &p, &smalls);
        let folded = Frame {
            height: 24,
            ..r.last
        };
        presentation(&mut r, folded, true);
        assert_eq!(r.home.as_ref().unwrap().unfolded_height, Some(900));
        assert_eq!(fit(rect(0, 24, 1400, 24), small.work, false).height, 24);
        let unfolded = Frame {
            height: 400,
            ..r.last
        };
        presentation(&mut r, unfolded, false);
        assert_eq!(r.home.as_ref().unwrap().frame.height, 900);
        assert_eq!(
            plan(&mut r, unfolded, &[full], true, false, true).dest,
            home
        );
    }
}
