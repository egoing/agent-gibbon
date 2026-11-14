//! Opt-in, development-only relation experiment. No saved memo/API schema changes.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, OnceLock,
    },
    time::{Duration, Instant},
};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Relation {
    from: String,
    to: String,
    label: String,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
fn rect(w: &WebviewWindow) -> Option<Rect> {
    if !w.is_visible().ok()? || w.is_minimized().ok()? || crate::stash::owns(w) {
        return None;
    }
    window_rect(w).ok()
}
fn window_rect(w: &WebviewWindow) -> Result<Rect, String> {
    let p = w.outer_position().map_err(|e| e.to_string())?;
    let s = w.outer_size().map_err(|e| e.to_string())?;
    Ok(Rect {
        x: p.x as f64,
        y: p.y as f64,
        w: s.width as f64,
        h: s.height as f64,
    })
}
// Facing side centers and outward tangents produce a cubic without entering either memo.
fn curve(a: Rect, b: Rect) -> Option<[[f64; 2]; 4]> {
    let gap_x = (b.x - a.x - a.w).max(a.x - b.x - b.w);
    let gap_y = (b.y - a.y - a.h).max(a.y - b.y - b.h);
    if gap_x < 0. && gap_y < 0. {
        return None;
    }
    let ac = [a.x + a.w / 2., a.y + a.h / 2.];
    let bc = [b.x + b.w / 2., b.y + b.h / 2.];
    if gap_x >= 0. && gap_x >= gap_y {
        let sign = if bc[0] >= ac[0] { 1. } else { -1. };
        let start = [ac[0] + sign * a.w / 2., ac[1]];
        let end = [bc[0] - sign * b.w / 2., bc[1]];
        let bend = (end[0] - start[0]).abs() / 2.;
        Some([
            start,
            [start[0] + sign * bend, start[1]],
            [end[0] - sign * bend, end[1]],
            end,
        ])
    } else {
        let sign = if bc[1] >= ac[1] { 1. } else { -1. };
        let start = [ac[0], ac[1] + sign * a.h / 2.];
        let end = [bc[0], bc[1] - sign * b.h / 2.];
        let bend = (end[1] - start[1]).abs() / 2.;
        Some([
            start,
            [start[0], start[1] + sign * bend],
            [end[0], end[1] - sign * bend],
            end,
        ])
    }
}
fn frame(points: &[[f64; 2]; 4], previous: Option<Rect>, padding: f64) -> Rect {
    let min_x = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let min_y = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    if let Some(r) = previous {
        if min_x - padding >= r.x
            && max_x + padding <= r.x + r.w
            && min_y - padding >= r.y
            && max_y + padding <= r.y + r.h
        {
            return r;
        }
    }
    // Reserve travel space. Ordinary dragging changes only SVG geometry, not native bounds.
    let reserve = padding + 512.;
    Rect {
        x: (min_x - reserve).floor(),
        y: (min_y - reserve).floor(),
        w: (max_x - min_x + 2. * reserve).ceil(),
        h: (max_y - min_y + 2. * reserve).ceil(),
    }
}
fn local_points(points: [[f64; 2]; 4], actual: Rect) -> [[f64; 2]; 4] {
    points.map(|p| [p[0] - actual.x, p[1] - actual.y])
}
static WAKE: OnceLock<mpsc::SyncSender<()>> = OnceLock::new();
static CONNECTED: OnceLock<std::sync::RwLock<std::collections::BTreeSet<String>>> = OnceLock::new();
static SURFACE_DIRTY: OnceLock<std::sync::Mutex<std::collections::BTreeSet<String>>> =
    OnceLock::new();
const SETTLE: Duration = Duration::from_millis(150);
#[derive(Default)]
struct Motion {
    changed: BTreeMap<String, Instant>,
    all: Option<Instant>,
}
impl Motion {
    fn paused(&self, from: &str, to: &str, now: Instant, stashed: bool) -> bool {
        stashed
            || [
                self.all,
                self.changed.get(from).copied(),
                self.changed.get(to).copied(),
            ]
            .into_iter()
            .flatten()
            .any(|last| now.saturating_duration_since(last) < SETTLE)
    }
    fn dirty_since(&self, from: &str, to: &str, checked: Instant) -> bool {
        [
            self.all,
            self.changed.get(from).copied(),
            self.changed.get(to).copied(),
        ]
        .into_iter()
        .flatten()
        .any(|t| t > checked)
    }
    fn delay(&self, now: Instant) -> Duration {
        self.changed
            .values()
            .copied()
            .chain(self.all)
            .filter_map(|last| SETTLE.checked_sub(now.saturating_duration_since(last)))
            .min()
            .unwrap_or(Duration::from_millis(250))
            .max(Duration::from_millis(1))
    }
}
fn suspend(surface: &mut Surface) {
    surface.generation = surface.generation.wrapping_add(1);
    surface.pending = None;
    surface.payload = None;
    if surface.window.is_visible().unwrap_or(false) {
        let _ = surface.window.hide();
    }
}
pub fn changed(app: &tauri::AppHandle, label: &str) {
    if label.starts_with("relation-prototype-") {
        if let Some(dirty) = SURFACE_DIRTY.get() {
            if let Ok(mut dirty) = dirty.lock() {
                dirty.insert(label.into());
            }
        }
        if let Some(state) = app.try_state::<std::sync::Mutex<Prototype>>() {
            if let Ok(mut state) = state.try_lock() {
                if let Some(surface) = state.surfaces.get_mut(label) {
                    suspend(surface);
                    surface.checked = None;
                }
            }
        }
        if let Some(tx) = WAKE.get() {
            let _ = tx.try_send(());
        }
        return;
    }
    let connected = CONNECTED.get().and_then(|labels| labels.read().ok());
    if !connected
        .as_ref()
        .is_some_and(|labels| labels.contains(label))
    {
        return;
    }
    if let Some(state) = app.try_state::<std::sync::Mutex<Prototype>>() {
        if let Ok(mut state) = state.try_lock() {
            state.motion.changed.insert(label.into(), Instant::now());
            let affected: Vec<_> = state
                .relations
                .iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    (crate::label(&r.from) == label || crate::label(&r.to) == label)
                        .then_some(format!("relation-prototype-{i}"))
                })
                .collect();
            for key in affected {
                if let Some(surface) = state.surfaces.get_mut(&key) {
                    suspend(surface);
                }
            }
        }
    }
    if let Some(tx) = WAKE.get() {
        let _ = tx.try_send(());
    }
}
pub fn transition_started(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<std::sync::Mutex<Prototype>>() {
        if let Ok(mut state) = state.try_lock() {
            state.motion.all = Some(Instant::now());
            for surface in state.surfaces.values_mut() {
                suspend(surface);
            }
        }
    }
    if let Some(tx) = WAKE.get() {
        let _ = tx.try_send(());
    }
}
fn acknowledged(pending: Option<u64>, generation: u64, paused: bool) -> bool {
    !paused && pending == Some(generation)
}
#[tauri::command]
pub fn relation_ready(window: WebviewWindow, generation: u64) -> Result<bool, String> {
    let Some(index) = window
        .label()
        .strip_prefix("relation-prototype-")
        .and_then(|s| s.parse::<usize>().ok())
    else {
        return Ok(false);
    };
    let Some(state) = window
        .app_handle()
        .try_state::<std::sync::Mutex<Prototype>>()
    else {
        return Ok(false);
    };
    if SURFACE_DIRTY
        .get()
        .and_then(|d| d.lock().ok())
        .is_some_and(|d| d.contains(window.label()))
    {
        return Ok(false);
    }
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let Some(r) = state.relations.get(index) else {
        return Ok(false);
    };
    if state.motion.paused(
        &crate::label(&r.from),
        &crate::label(&r.to),
        Instant::now(),
        crate::stash::active(window.app_handle()),
    ) {
        return Ok(false);
    };
    let Some(surface) = state.surfaces.get_mut(window.label()) else {
        return Ok(false);
    };
    if !acknowledged(surface.pending, generation, false) {
        return Ok(false);
    };
    surface.pending = None;
    #[cfg(target_os = "macos")]
    unsafe {
        // SAFETY: trusted own-window acknowledgement, Tauri main thread and live native window.
        (&*(window.ns_window().map_err(|e| e.to_string())? as *const objc2_app_kit::NSWindow))
            .orderFrontRegardless();
    }
    #[cfg(not(target_os = "macos"))]
    window.show().map_err(|e| e.to_string())?;
    Ok(true)
}

struct Surface {
    window: WebviewWindow,
    ready: Arc<AtomicBool>,
    frame: Option<Rect>,
    actual_frame: Option<Rect>,
    payload: Option<serde_json::Value>,
    generation: u64,
    pending: Option<u64>,
    checked: Option<Instant>,
}
#[derive(Default)]
struct Prototype {
    surfaces: BTreeMap<String, Surface>,
    bytes: Vec<u8>,
    relations: Vec<Relation>,
    last_config: Option<Instant>,
    motion: Motion,
    was_stashed: bool,
    layouts: u64,
}
fn valid(r: &Relation) -> bool {
    let id = |s: &str| {
        !s.is_empty()
            && s.len() <= 64
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    };
    id(&r.from)
        && id(&r.to)
        && r.from != r.to
        && r.label.chars().count() <= 80
        && !r.label.chars().any(char::is_control)
}
fn overlay(app: &tauri::AppHandle, key: &str) -> Result<Surface, String> {
    crate::focus::remember(app, key);
    let ready = Arc::new(AtomicBool::new(false));
    let loaded = ready.clone();
    let w = tauri::WebviewWindowBuilder::new(
        app,
        key,
        tauri::WebviewUrl::App("relation-prototype.html".into()),
    )
    .title(format!("{} — {key}", crate::instance::current().title()))
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .visible(false)
    .focused(false)
    .focusable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .on_page_load(move |_, event| {
        if event.event() == tauri::webview::PageLoadEvent::Finished {
            loaded.store(true, Ordering::Release);
            if let Some(tx) = WAKE.get() {
                let _ = tx.try_send(());
            }
        }
    })
    .build()
    .map_err(|e| e.to_string())?;
    if let Err(e) = w.set_ignore_cursor_events(true) {
        let _ = w.destroy();
        return Err(e.to_string());
    }
    Ok(Surface {
        window: w,
        ready,
        frame: None,
        actual_frame: None,
        payload: None,
        generation: 0,
        pending: None,
        checked: None,
    })
}
fn tick(app: &tauri::AppHandle, state: &mut Prototype) -> Result<(), String> {
    if let Some(dirty) = SURFACE_DIRTY.get() {
        if let Ok(mut dirty) = dirty.lock() {
            for label in std::mem::take(&mut *dirty) {
                if let Some(surface) = state.surfaces.get_mut(&label) {
                    suspend(surface);
                    surface.checked = None;
                }
            }
        }
    }
    if state
        .last_config
        .is_none_or(|t| t.elapsed() >= Duration::from_millis(250))
    {
        use std::io::Read;
        let bytes = std::fs::File::open(crate::ipc::directory()?.join("relation-prototype.json"))
            .and_then(|f| {
                let mut bytes = Vec::new();
                f.take(16385).read_to_end(&mut bytes)?;
                Ok(bytes)
            })
            .unwrap_or_default();
        if bytes != state.bytes {
            state.relations = if bytes.len() <= 16384 {
                serde_json::from_slice::<Vec<Relation>>(&bytes)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(valid)
                    .take(8)
                    .collect()
            } else {
                Vec::new()
            };
            if let Some(labels) = CONNECTED.get() {
                if let Ok(mut labels) = labels.write() {
                    *labels = state
                        .relations
                        .iter()
                        .flat_map(|r| [crate::label(&r.from), crate::label(&r.to)])
                        .collect();
                }
            }
            for surface in state.surfaces.values_mut() {
                suspend(surface);
            }
            state.bytes = bytes;
        }
        state.last_config = Some(Instant::now());
    }
    let stashed = crate::stash::active(app);
    if state.was_stashed && !stashed {
        state.motion.all = Some(Instant::now());
    }
    state.was_stashed = stashed;
    for index in 0..8 {
        let key = format!("relation-prototype-{index}");
        let paused = state.relations.get(index).is_some_and(|r| {
            state.motion.paused(
                &crate::label(&r.from),
                &crate::label(&r.to),
                Instant::now(),
                stashed,
            )
        });
        if paused {
            if let Some(surface) = state.surfaces.get_mut(&key) {
                if surface.pending.is_some()
                    || surface.payload.is_some()
                    || surface.window.is_visible().unwrap_or(false)
                {
                    suspend(surface);
                }
            }
            continue;
        }
        if let (Some(r), Some(surface)) = (state.relations.get(index), state.surfaces.get(&key)) {
            let dirty = surface.payload.is_none()
                || surface.checked.is_none_or(|t| {
                    state
                        .motion
                        .dirty_since(&crate::label(&r.from), &crate::label(&r.to), t)
                });
            if !dirty {
                // Visibility-only fallback: stationary relations do not read positions/sizes
                // or rebuild curves when an unrelated memo is moving.
                let visible = [&r.from, &r.to].iter().all(|id| {
                    app.get_webview_window(&crate::label(id)).is_some_and(|w| {
                        w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(true)
                    })
                });
                if !visible {
                    if let Some(surface) = state.surfaces.get_mut(&key) {
                        suspend(surface);
                    }
                }
                continue;
            }
        }
        let geometry = state.relations.get(index).and_then(|r| {
            let notes = app.state::<crate::AppNotes>();
            let notes = notes.lock().ok()?;
            if !notes.contains_key(&r.from) || !notes.contains_key(&r.to) {
                return None;
            }
            drop(notes);
            let a = app.get_webview_window(&crate::label(&r.from))?;
            let b = app.get_webview_window(&crate::label(&r.to))?;
            state.layouts += 1;
            Some((r, curve(rect(&a)?, rect(&b)?)?))
        });
        let Some((relation, points)) = geometry else {
            if let Some(s) = state.surfaces.remove(&key) {
                let _ = s.window.destroy();
            }
            continue;
        };
        if !state.surfaces.contains_key(&key) {
            state.surfaces.insert(key.clone(), overlay(app, &key)?);
        }
        let surface = state.surfaces.get_mut(&key).expect("inserted surface");
        if !surface.ready.load(Ordering::Acquire) {
            continue;
        }
        surface.checked = Some(Instant::now());
        let w = &surface.window;
        crate::focus::restore(w);
        let scale = w.scale_factor().map_err(|e| e.to_string())?;
        let padding =
            ((relation.label.chars().count() as f64 * 12. + 24.) * scale / 2.).max(32. * scale);
        let bounds = frame(&points, surface.frame, padding);
        let reposition = surface.frame != Some(bounds);
        if reposition {
            w.set_position(PhysicalPosition::new(bounds.x as i32, bounds.y as i32))
                .map_err(|e| e.to_string())?;
            w.set_size(PhysicalSize::new(bounds.w as u32, bounds.h as u32))
                .map_err(|e| e.to_string())?;
            surface.frame = Some(bounds);
        }
        // macOS may clamp the native origin below its menu bar and adjust size.
        // Keep the requested reserve for scheduling, but draw in the actual viewport.
        let actual = window_rect(w)?;
        let native_changed = surface.actual_frame != Some(actual);
        surface.actual_frame = Some(actual);
        let relative = local_points(points, actual);
        let payload = serde_json::json!({"width":actual.w,"height":actual.h,"points":relative,"label":relation.label,"scale":scale,"relation":[relation.from,relation.to]});
        if native_changed || surface.payload.as_ref() != Some(&payload) {
            // A fresh generation cannot be revealed by a stale pre-move acknowledgement.
            surface.generation = surface.generation.wrapping_add(1);
            surface.pending = Some(surface.generation);
            let generation = surface.generation;
            w.eval(format!("window.drawRelation({payload},{generation})"))
                .map_err(|e| e.to_string())?;
            surface.payload = Some(payload);
        }
    }
    Ok(())
}
pub fn install(app: &tauri::AppHandle) {
    if !cfg!(debug_assertions)
        || crate::instance::current().id.is_none()
        || std::env::var("AGENT_GIBBON_RELATION_PROTOTYPE").as_deref() != Ok("1")
    {
        return;
    }
    if let Ok(dir) = crate::ipc::directory() {
        let _ = std::fs::remove_file(dir.join("relation-prototype.json"));
    }
    let (wake, changes) = mpsc::sync_channel(1);
    if WAKE.set(wake).is_err() {
        return;
    }
    CONNECTED.get_or_init(Default::default);
    SURFACE_DIRTY.get_or_init(Default::default);
    app.manage(std::sync::Mutex::new(Prototype::default()));
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            let start = Instant::now();
            let (tx, rx) = mpsc::sync_channel(1);
            let target = app.clone();
            if app
                .run_on_main_thread(move || {
                    let state = target.state::<std::sync::Mutex<Prototype>>();
                    if let Ok(mut state) = state.lock() {
                        if let Err(e) = tick(&target, &mut state) {
                            eprintln!("관계 조사: {e}");
                        }
                    }
                    let wait = if let Ok(state) = state.lock() {
                        if state.was_stashed {
                            Duration::from_millis(50)
                        } else {
                            state.motion.delay(Instant::now())
                        }
                    } else {
                        Duration::from_millis(250)
                    };
                    let _ = tx.send(wait);
                })
                .is_err()
            {
                break;
            }
            let Ok(wait) = rx.recv_timeout(Duration::from_secs(5)) else {
                break;
            };
            // Idle fallback detects visibility and atomic config replacement, without repainting.
            let _ = changes.recv_timeout(wait);
            let frame = Duration::from_micros(16667);
            if let Some(remaining) = frame.checked_sub(start.elapsed()) {
                std::thread::sleep(remaining);
            }
            while changes.try_recv().is_ok() {}
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curve_faces_boundaries_in_both_directions() {
        let a = Rect {
            x: -300.,
            y: 0.,
            w: 100.,
            h: 100.,
        };
        let b = Rect {
            x: 100.,
            y: 200.,
            w: 200.,
            h: 100.,
        };
        let p = curve(a, b).unwrap();
        assert_eq!(p[0], [-200., 50.]);
        assert_eq!(p[3], [100., 250.]);
        assert_eq!(p[0][1], p[1][1]);
        assert_eq!(p[2][1], p[3][1]);
        let reverse = curve(b, a).unwrap();
        assert_eq!(reverse, [p[3], p[2], p[1], p[0]]);
        assert!(curve(a, a).is_none());
        let below = Rect {
            x: -300.,
            y: 300.,
            w: 100.,
            h: 100.,
        };
        let vertical = curve(a, below).unwrap();
        assert_eq!(vertical[0], [-250., 100.]);
        assert_eq!(vertical[3], [-250., 300.]);
    }
    #[test]
    fn native_frame_is_stable_during_small_movements() {
        let p = [[0., 0.], [80., 0.], [120., 100.], [200., 100.]];
        let initial = frame(&p, None, 40.);
        assert_eq!(
            frame(&p.map(|p| [p[0] + 100., p[1] + 50.]), Some(initial), 40.),
            initial
        );
        assert_ne!(
            frame(&p.map(|p| [p[0] + 2000., p[1]]), Some(initial), 40.),
            initial
        );
        let touching = Rect {
            x: 0.,
            y: 0.,
            w: 100.,
            h: 100.,
        };
        assert!(curve(
            touching,
            Rect {
                x: 100.,
                ..touching
            }
        )
        .unwrap()
        .iter()
        .flatten()
        .all(|n| n.is_finite()));
    }
    #[test]
    fn native_clamping_uses_actual_origin_without_accumulating_offset() {
        let points = [[1006., 474.], [1006., 887.], [464., 887.], [464., 1300.]];
        // Requested Y=-102px is clamped by macOS to 38pt=76px at scale 2.
        let actual = Rect {
            x: -112.,
            y: 76.,
            w: 1682.,
            h: 1982.,
        };
        let relative = local_points(points, actual);
        assert_eq!(relative[0], [1118., 398.]);
        for (global, local) in points.into_iter().zip(relative) {
            assert_eq!([actual.x + local[0], actual.y + local[1]], global);
        }
        assert_eq!(relative, local_points(points, actual));
        let resized = Rect {
            x: -1500.,
            y: -900.,
            w: 3000.,
            h: 2000.,
        };
        let relative = local_points(points, resized);
        for (global, local) in points.into_iter().zip(relative) {
            assert_eq!([resized.x + local[0], resized.y + local[1]], global);
        }
    }
    #[test]
    fn motion_quiet_period_is_per_endpoint_and_restarts_on_each_event() {
        let start = Instant::now();
        let mut motion = Motion::default();
        motion.changed.insert("note-a".into(), start);
        assert!(motion.paused(
            "note-a",
            "note-b",
            start + Duration::from_millis(149),
            false
        ));
        assert!(!motion.paused("note-a", "note-b", start + SETTLE, false));
        assert!(!motion.paused("note-c", "note-d", start, false));
        motion
            .changed
            .insert("note-a".into(), start + Duration::from_millis(100));
        assert!(motion.paused(
            "note-a",
            "note-b",
            start + Duration::from_millis(249),
            false
        ));
        assert!(!motion.paused(
            "note-a",
            "note-b",
            start + Duration::from_millis(250),
            false
        ));
        assert!(motion.dirty_since("note-a", "note-b", start));
        assert!(!motion.dirty_since("note-a", "note-b", start + Duration::from_millis(250)));
    }
    #[test]
    fn stash_and_stale_reveal_never_restart_a_hidden_connection() {
        let start = Instant::now();
        let motion = Motion::default();
        assert!(motion.paused("note-a", "note-b", start, true));
        assert!(!acknowledged(Some(7), 7, true));
        assert!(!acknowledged(None, 7, false));
        assert!(!acknowledged(Some(8), 7, false));
        assert!(acknowledged(Some(8), 8, false));
        let motion = Motion {
            all: Some(start),
            ..Default::default()
        };
        assert!(motion.paused("note-a", "note-b", start, false));
        assert!(!motion.paused("note-a", "note-b", start + SETTLE, false));
    }
    #[test]
    fn relation_validation() {
        let mut r = Relation {
            from: "a".into(),
            to: "b".into(),
            label: "선행 작업".into(),
        };
        assert!(valid(&r));
        r.to = "a".into();
        assert!(!valid(&r));
        r.to = "b".into();
        r.label = "x\ny".into();
        assert!(!valid(&r));
    }
}
