//! Main-thread window movement; sleeping happens only on the worker thread.
use std::{
    collections::BTreeMap,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, PhysicalPosition, WebviewWindow};

// Opt-in local diagnostics only: no note contents, default logging or history.
fn trace(window: &WebviewWindow, event: &str, data: impl FnOnce() -> serde_json::Value) {
    use std::{io::Write, sync::OnceLock};
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    static WRITER: Mutex<()> = Mutex::new(());
    let Some(path) =
        PATH.get_or_init(|| std::env::var_os("AGENT_GIBBON_ENTRANCE_TRACE").map(Into::into))
    else {
        return;
    };
    let row = serde_json::json!({
        "event":event,"window":window.label(),"pid":std::process::id(),
        "instance":crate::instance::current().id,
        "time_us":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_micros(),
        "data":data()
    });
    let line = format!("{row}\n");
    let _writer = WRITER.lock().unwrap();
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}
fn flight_trace(flight: &Flight) -> serde_json::Value {
    serde_json::json!({"generation":format!("{:?}",flight.started),"expected":flight.expected,
        "destination":{"position":flight.destination.position,"size":flight.destination.size,"scale":flight.destination.scale}})
}

type Point = PhysicalPosition<f64>;

#[derive(Clone, Copy, Default)]
pub enum Presentation {
    #[default]
    Cursor,
    Direct,
}

#[derive(Clone, Copy)]
pub struct Destination {
    pub position: PhysicalPosition<i32>,
    pub scale: f64,
    pub size: tauri::PhysicalSize<u32>,
}

struct Flight {
    destination: Destination,
    expected: Point,
    started: Instant,
    motion: Motion,
    from_size: Option<tauri::LogicalSize<f64>>,
}
#[derive(Default)]
pub struct Entrances(Mutex<BTreeMap<String, Flight>>);

// macOS positions on different displays must share desktop logical coordinates.
// Tao reports the cursor at primary scale and each window/monitor at its own scale.
fn coordinate_scale(scale: f64) -> f64 {
    if cfg!(target_os = "macos") {
        scale
    } else {
        1.0
    }
}
fn point(position: PhysicalPosition<i32>, scale: f64) -> Point {
    let scale = coordinate_scale(scale);
    Point::new(f64::from(position.x) / scale, f64::from(position.y) / scale)
}
fn cursor(window: &WebviewWindow) -> Result<Point, String> {
    let p = window.cursor_position().map_err(|e| e.to_string())?;
    let scale = coordinate_scale(
        window
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .map(|m| m.scale_factor())
            .unwrap_or(1.0),
    );
    Ok(Point::new(p.x / scale, p.y / scale))
}
fn position(window: &WebviewWindow) -> Result<Point, String> {
    Ok(point(
        window.outer_position().map_err(|e| e.to_string())?,
        window.scale_factor().map_err(|e| e.to_string())?,
    ))
}
fn move_to(window: &WebviewWindow, p: Point) -> Result<(), String> {
    geometry(window, p, None)
}

// Recover the native bottom-left origin from the current frame/top-left pair.
// This avoids assuming a primary display's origin, height or backing scale.
#[cfg(any(target_os = "macos", test))]
fn native_y(origin: f64, height: f64, top: f64, next_top: f64, next_height: f64) -> f64 {
    origin + height + top - next_top - next_height
}

// All entrance writes run on the main thread. Tao's macOS position/size setters
// enqueue GCD work even there: a later tick can read the previous native frame
// and mistake the unacknowledged write for manual movement. Apply the whole
// native frame synchronously, before advancing our expected position.
fn geometry(
    window: &WebviewWindow,
    p: Point,
    size: Option<tauri::LogicalSize<f64>>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _main =
            objc2::MainThreadMarker::new().ok_or("창 전환은 메인 스레드에서 실행해야 합니다")?;
        let top = position(window)?;
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: checked main thread, live window borrowed only for this call.
        let native = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        let mut frame = native.frame();
        let (width, height) = size
            .map(|s| (s.width.round(), s.height.round()))
            .unwrap_or((frame.size.width, frame.size.height));
        frame.origin.y = native_y(
            frame.origin.y,
            frame.size.height,
            top.y,
            p.y.round(),
            height,
        );
        frame.origin.x = p.x.round();
        frame.size.width = width;
        frame.size.height = height;
        // Mark for normal AppKit display rather than synchronously forcing
        // every WebView to draw while applying a batch of window frames.
        native.setFrame_display(frame, false);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(size) = size {
            window.set_size(size).map_err(|e| e.to_string())?;
        }
        window
            .set_position(PhysicalPosition::new(
                p.x.round() as i32,
                p.y.round() as i32,
            ))
            .map_err(|e| e.to_string())
    }
}

pub fn place(window: &WebviewWindow, destination: Destination) -> Result<(), String> {
    geometry(
        window,
        point(destination.position, destination.scale),
        Some(destination.size.to_logical::<f64>(destination.scale)),
    )
}

// Latch the first deliberate movement, even during the minimum one-second hold.
// A timer alone cannot start movement; later reversals do not cancel it.
struct Motion {
    shown_at: Instant,
    moved: bool,
    threshold: f64,
    anchor: Point,
    moving_since: Option<Instant>,
}
impl Motion {
    fn progress(&mut self, pointer: Point, now: Instant) -> Option<f64> {
        self.moved |= (pointer.x - self.anchor.x)
            .abs()
            .max((pointer.y - self.anchor.y).abs())
            >= self.threshold;
        if self.moving_since.is_none()
            && self.moved
            && now.duration_since(self.shown_at) >= Duration::from_secs(1)
        {
            self.moving_since = Some(now);
        }
        self.moving_since
            .map(|start| (now.duration_since(start).as_secs_f64() / 0.420).min(1.0))
    }
}

// Try all four adjacent quadrants, preferring the side toward the screen centre.
// Clamping is followed by a cursor-clearance check: never clamp onto the pointer.
fn nearby(area: (f64, f64, f64, f64), size: (f64, f64), cursor: Point, gap: f64) -> Option<Point> {
    let (left, top, width, height) = area;
    let (w, h) = size;
    if w <= 0.0 || h <= 0.0 || w > width || h > height {
        return None;
    }
    let right = left + width - w;
    let bottom = top + height - h;
    let mut xs = [cursor.x + gap, cursor.x - gap - w];
    let mut ys = [cursor.y + gap, cursor.y - gap - h];
    if cursor.x > left + width / 2.0 {
        xs.swap(0, 1);
    }
    if cursor.y > top + height / 2.0 {
        ys.swap(0, 1);
    }
    for y in ys {
        for x in xs {
            let x = x.clamp(left, right);
            let y = y.clamp(top, bottom);
            if x >= cursor.x + gap
                || x + w <= cursor.x - gap
                || y >= cursor.y + gap
                || y + h <= cursor.y - gap
            {
                return Some(Point::new(x, y));
            }
        }
    }
    None
}

// Stop pending frames before hiding/warping, retaining the original destination.
pub fn finish(window: &WebviewWindow) -> Result<(), String> {
    let flight = window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    if let Some(flight) = flight {
        trace(window, "finish", || flight_trace(&flight));
        if near(position(window)?, flight.expected) {
            place(window, flight.destination)?;
        }
    }
    Ok(())
}
pub fn active(window: &WebviewWindow) -> bool {
    window
        .state::<Entrances>()
        .0
        .lock()
        .map(|flights| flights.contains_key(window.label()))
        .unwrap_or(false)
}
// Keep the destination rectangle in its own monitor's physical coordinate space.
// The live window may temporarily reside on a monitor with another scale.
pub fn destination(window: &WebviewWindow) -> Option<Destination> {
    window
        .state::<Entrances>()
        .0
        .lock()
        .ok()?
        .get(window.label())
        .map(|f| f.destination)
}
#[cfg(test)]
impl Destination {
    pub fn rect(self) -> crate::placement::Rect {
        crate::placement::Rect {
            x: self.position.x.into(),
            y: self.position.y.into(),
            width: self.size.width.into(),
            height: self.size.height.into(),
        }
    }
}
// Direct manipulation owns the current position, unlike hiding/warping which
// completes the entrance at its destination.
#[track_caller]
pub fn cancel(window: &WebviewWindow) -> Result<(), String> {
    let flight = window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    if let Some(flight) = flight {
        let caller = std::panic::Location::caller();
        trace(
            window,
            "cancel",
            || serde_json::json!({"flight":flight_trace(&flight),"caller":caller.to_string()}),
        );
    }
    Ok(())
}
#[tauri::command]
pub fn drag_note(window: WebviewWindow) -> Result<(), String> {
    if crate::stash::owns(&window) {
        return Ok(());
    }
    cancel(&window)?;
    crate::layout::mark(&window, true);
    window.start_dragging().map_err(|e| e.to_string())
}
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= 2.0 && (a.y - b.y).abs() <= 2.0
}

pub fn show(
    window: &WebviewWindow,
    destination: Option<Destination>,
    presentation: Presentation,
) -> Result<(), String> {
    if crate::stash::active(window.app_handle()) {
        return crate::show_without_focus(window);
    }
    if window.is_visible().map_err(|e| e.to_string())? {
        return crate::show_without_focus(window);
    }
    finish(window)?;
    let destination = match destination {
        Some(p) => p,
        None => crate::layout::reflow(window.app_handle(), window)?,
    };
    if crate::layout::excluded(window) {
        return crate::show_without_focus(window);
    }
    // Placement is complete while hidden. Direct user opens need no cursor wait.
    if matches!(presentation, Presentation::Direct) {
        return crate::show_without_focus(window);
    }
    let anchor = cursor(window)?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = coordinate_scale(window.scale_factor().map_err(|e| e.to_string())?);
    let mut start = None;
    let mut monitor_scale = 1.0;
    for monitor in window.available_monitors().map_err(|e| e.to_string())? {
        let s = coordinate_scale(monitor.scale_factor());
        let p = point(*monitor.position(), monitor.scale_factor());
        let m = monitor.size();
        if anchor.x < p.x
            || anchor.y < p.y
            || anchor.x >= p.x + f64::from(m.width) / s
            || anchor.y >= p.y + f64::from(m.height) / s
        {
            continue;
        }
        let area = monitor.work_area();
        let p = point(area.position, monitor.scale_factor());
        let entrance_area = (
            p.x,
            p.y,
            f64::from(area.size.width) / s,
            f64::from(area.size.height) / s,
        );
        monitor_scale = monitor.scale_factor();
        start = nearby(
            entrance_area,
            (
                f64::from(size.width) / scale,
                f64::from(size.height) / scale,
            ),
            anchor,
            12.0 * monitor.scale_factor() / s,
        );
        break;
    }
    // Oversized/manual windows may leave no cursor-safe rectangle. Preserve size
    // and use the existing destination without a misleading/occluding wait.
    let Some(start) = start else {
        return crate::show_without_focus(window);
    };
    move_to(window, start)?;
    let started = Instant::now();
    trace(
        window,
        "show",
        || serde_json::json!({"generation":format!("{started:?}"),"start":start,"destination":destination.position,"size":destination.size,"anchor":anchor}),
    );
    window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            window.label().into(),
            Flight {
                destination,
                expected: start,
                started,
                from_size: None,
                motion: Motion {
                    shown_at: started,
                    moved: false,
                    threshold: 8.0 * monitor_scale / coordinate_scale(monitor_scale),
                    anchor,
                    moving_since: None,
                },
            },
        );
    if let Err(error) = crate::show_without_focus(window) {
        let _ = finish(window);
        return Err(error);
    }
    // Start the minimum hold after native show has completed.
    if let Some(flight) = window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get_mut(window.label())
    {
        flight.motion.shown_at = Instant::now();
    }
    run_frames(window, start, started);
    Ok(())
}

/// Re-target an active flight from its current rendered frame. Transient
/// interpolation state is discarded when complete; it is not user history.
pub fn transition(window: &WebviewWindow, destination: Destination) -> Result<(), String> {
    cancel(window)?;
    let start = position(window)?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let size = window
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let started = Instant::now();
    trace(
        window,
        "transition",
        || serde_json::json!({"generation":format!("{started:?}"),"start":start,"destination":destination.position,"size":destination.size}),
    );
    window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            window.label().into(),
            Flight {
                destination,
                expected: start,
                started,
                from_size: Some(size),
                motion: Motion {
                    shown_at: started,
                    moved: true,
                    threshold: 0.0,
                    anchor: start,
                    moving_since: Some(started),
                },
            },
        );
    run_frames(window, start, started);
    Ok(())
}
fn run_frames(window: &WebviewWindow, start: Point, started: Instant) {
    let window = window.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(16));
        let frame = window.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let queued = window.run_on_main_thread(move || {
            let result = tick(&frame, start, started);
            if let Err(error) = &result {
                eprintln!("패널 등장 이동 실패: {error}");
                let _ = finish(&frame);
            }
            let _ = tx.send(result.unwrap_or(false));
        });
        if queued.is_err() {
            trace(
                &window,
                "queue-error",
                || serde_json::json!({"generation":format!("{started:?}")}),
            );
            break;
        }
        if !await_frame(&rx, Duration::from_secs(1), || {
            trace(
                &window,
                "frame-wait-late",
                || serde_json::json!({"generation":format!("{started:?}")}),
            );
        }) {
            break;
        }
    });
}

// Only one frame is queued per worker. A busy main thread is not cancellation:
// keep its receiver alive so a late callback can continue this generation.
// Channel disconnection/app teardown still ends the worker.
fn await_frame(rx: &mpsc::Receiver<bool>, timeout: Duration, late: impl FnOnce()) -> bool {
    match rx.recv_timeout(timeout) {
        Ok(next) => next,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            late();
            rx.recv().unwrap_or(false)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => false,
    }
}
fn tick(window: &WebviewWindow, start: Point, started: Instant) -> Result<bool, String> {
    let state = window.state::<Entrances>();
    let mut flights = state.0.lock().map_err(|e| e.to_string())?;
    if flights
        .get(window.label())
        .is_none_or(|f| f.started != started)
    {
        return Ok(false);
    }
    // Own this generation outside the lock while updating the native window.
    let mut flight = flights.remove(window.label()).unwrap();
    drop(flights);
    let actual = position(window)?;
    if !near(actual, flight.expected) {
        trace(
            window,
            "position-mismatch",
            || serde_json::json!({"flight":flight_trace(&flight),"actual":actual}),
        );
        return Ok(false);
    }
    let hidden = !window.is_visible().map_err(|e| e.to_string())?;
    let t = if hidden {
        1.0
    } else {
        let Some(t) = flight.motion.progress(cursor(window)?, Instant::now()) else {
            state
                .0
                .lock()
                .map_err(|e| e.to_string())?
                .insert(window.label().into(), flight);
            return Ok(true);
        };
        t
    };
    let ease = 1.0 - (1.0 - t).powi(3);
    let destination = point(flight.destination.position, flight.destination.scale);
    let next = Point::new(
        start.x + (destination.x - start.x) * ease,
        start.y + (destination.y - start.y) * ease,
    );
    let size = flight.from_size.map(|size| {
        let target = flight
            .destination
            .size
            .to_logical::<f64>(flight.destination.scale);
        tauri::LogicalSize::new(
            size.width + (target.width - size.width) * ease,
            size.height + (target.height - size.height) * ease,
        )
    });
    geometry(window, next, size)?;
    trace(
        window,
        "frame",
        || serde_json::json!({"flight":flight_trace(&flight),"t":t,"next":next,"actual_after":position(window).ok(),"size_after":window.outer_size().ok()}),
    );
    flight.expected = next;
    if t >= 1.0 {
        return Ok(false);
    }
    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), flight);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_frame_response_keeps_worker_alive_without_queuing_another_frame() {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut observed_late = false;
        assert!(await_frame(&rx, Duration::ZERO, || {
            observed_late = true;
            tx.send(true).unwrap();
        }));
        assert!(observed_late);
    }
    #[test]
    fn completed_or_disconnected_frame_stops_the_worker() {
        let (tx, rx) = mpsc::sync_channel(1);
        tx.send(false).unwrap();
        assert!(!await_frame(&rx, Duration::ZERO, || panic!(
            "unexpected timeout"
        )));
        drop(tx);
        assert!(!await_frame(&rx, Duration::ZERO, || panic!(
            "unexpected timeout"
        )));
    }
    #[test]
    fn native_frame_conversion_preserves_top_left_while_resizing() {
        // Native bottom-left (40), height (500), top-left (900): desktop 1440.
        assert_eq!(native_y(40., 500., 900., 25., 420.), 995.);
        assert_eq!(native_y(40., 500., 900., 900., 100.), 440.);
    }
    #[test]
    fn native_frame_conversion_handles_negative_desktop_positions() {
        assert_eq!(native_y(-100., 200., 1340., -300., 100.), 1640.);
        assert_eq!(native_y(1640., 100., -300., 1800., 420.), -780.);
    }
    #[test]
    fn every_direction_and_diagonal_starts_the_same_flight_after_hold() {
        for (dx, dy) in [
            (-8.0, 0.0),
            (8.0, 0.0),
            (0.0, -8.0),
            (0.0, 8.0),
            (-8.0, -8.0),
            (8.0, 8.0),
        ] {
            let now = Instant::now();
            let mut motion = Motion {
                shown_at: now,
                moved: false,
                threshold: 8.0,
                anchor: Point::new(-100.0, 200.0),
                moving_since: None,
            };
            assert_eq!(motion.progress(Point::new(-92.1, 192.1), now), None);
            assert!(!motion.moved);
            let pointer = Point::new(motion.anchor.x + dx, motion.anchor.y + dy);
            assert_eq!(
                motion.progress(pointer, now + Duration::from_millis(100)),
                None
            );
            assert!(motion.moved);
            let reverse = Point::new(motion.anchor.x - dx, motion.anchor.y - dy);
            assert_eq!(
                motion.progress(reverse, now + Duration::from_secs(1)),
                Some(0.0)
            );
            assert_eq!(
                motion.progress(motion.anchor, now + Duration::from_millis(1420)),
                Some(1.0)
            );
        }
    }
    #[test]
    fn corners_and_negative_monitor_origin_keep_clearance() {
        let area = (-1600.0, -900.0, 1600.0, 900.0);
        for (cursor, expected) in [
            (Point::new(-20.0, -20.0), Point::new(-332.0, -232.0)),
            (Point::new(-1580.0, -880.0), Point::new(-1568.0, -868.0)),
            (Point::new(-20.0, -880.0), Point::new(-332.0, -868.0)),
            (Point::new(-1580.0, -20.0), Point::new(-1568.0, -232.0)),
        ] {
            assert_eq!(nearby(area, (300.0, 200.0), cursor, 12.0), Some(expected));
        }
    }
    #[test]
    fn clamping_never_covers_pointer_and_large_windows_fall_back() {
        for x in (0..1000).step_by(13) {
            for y in (0..700).step_by(17) {
                let p = Point::new(x as f64, y as f64);
                let q = nearby((0.0, 0.0, 1000.0, 700.0), (600.0, 400.0), p, 12.0);
                if let Some(q) = q {
                    assert!(
                        q.x >= 0.0 && q.y >= 0.0 && q.x + 600.0 <= 1000.0 && q.y + 400.0 <= 700.0
                    );
                    assert!(
                        q.x >= p.x + 12.0
                            || q.x + 600.0 <= p.x - 12.0
                            || q.y >= p.y + 12.0
                            || q.y + 400.0 <= p.y - 12.0
                    );
                }
            }
        }
        assert!(nearby(
            (0.0, 0.0, 300.0, 200.0),
            (300.0, 200.0),
            Point::new(150.0, 100.0),
            12.0
        )
        .is_none());
        assert!(nearby(
            (0.0, 0.0, 300.0, 200.0),
            (301.0, 200.0),
            Point::new(0.0, 0.0),
            12.0
        )
        .is_none());
    }
    #[test]
    fn only_pointer_movement_starts_one_flight() {
        let now = Instant::now();
        let mut motion = Motion {
            shown_at: now,
            moved: false,
            threshold: 8.0,
            anchor: Point::new(100.0, 100.0),
            moving_since: None,
        };
        assert_eq!(
            motion.progress(motion.anchor, now + Duration::from_secs(3600)),
            None
        );
        let moved = now + Duration::from_secs(3601);
        assert_eq!(motion.progress(Point::new(108.0, 100.0), moved), Some(0.0));
        assert_eq!(
            motion.progress(motion.anchor, moved + Duration::from_millis(210)),
            Some(0.5)
        );
        assert_eq!(
            motion.progress(motion.anchor, moved + Duration::from_millis(420)),
            Some(1.0)
        );
    }
    #[test]
    fn early_pointer_movement_waits_until_one_second_even_if_pointer_returns() {
        let now = Instant::now();
        let mut motion = Motion {
            shown_at: now,
            moved: false,
            threshold: 8.0,
            anchor: Point::new(100.0, 100.0),
            moving_since: None,
        };
        assert_eq!(motion.progress(motion.anchor, now), None);
        assert_eq!(
            motion.progress(Point::new(110.0, 100.0), now + Duration::from_millis(100)),
            None
        );
        assert_eq!(
            motion.progress(motion.anchor, now + Duration::from_millis(999)),
            None
        );
        assert_eq!(
            motion.progress(motion.anchor, now + Duration::from_secs(1)),
            Some(0.0)
        );
        assert_eq!(
            motion.progress(motion.anchor, now + Duration::from_millis(1210)),
            Some(0.5)
        );
        assert_eq!(
            motion.progress(motion.anchor, now + Duration::from_millis(1420)),
            Some(1.0)
        );
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn mixed_dpi_positions_share_desktop_coordinates() {
        assert_eq!(
            point(PhysicalPosition::new(-3200, 200), 2.0),
            Point::new(-1600.0, 100.0)
        );
        assert_eq!(
            point(PhysicalPosition::new(-1600, 100), 1.0),
            Point::new(-1600.0, 100.0)
        );
    }
    #[test]
    fn destination_reservation_retains_size_in_both_mixed_dpi_directions() {
        for (destination_scale, cursor_scale) in [(2.0, 1.0), (1.0, 2.0)] {
            let destination = Destination {
                position: PhysicalPosition::new(100, 200),
                scale: destination_scale,
                size: tauri::PhysicalSize::new(
                    (300.0 * destination_scale) as u32,
                    (200.0 * destination_scale) as u32,
                ),
            };
            let temporary_size = (300.0 * cursor_scale) as i64;
            let reserved = destination.rect();
            assert_eq!(reserved.width, (300.0 * destination_scale) as i64);
            assert_eq!(reserved.height, (200.0 * destination_scale) as i64);
            assert_ne!(reserved.width, temporary_size);
            assert_eq!((reserved.x, reserved.y), (100, 200));
        }
    }
}
