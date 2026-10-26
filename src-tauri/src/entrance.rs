//! Main-thread window movement; sleeping happens only on the worker thread.
use std::{
    collections::BTreeMap,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, PhysicalPosition, WebviewWindow};

type Point = PhysicalPosition<f64>;

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
    area: (f64, f64, f64, f64),
    monitor_scale: f64,
    action_started: bool,
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
    #[cfg(target_os = "macos")]
    let position = tauri::Position::Logical(tauri::LogicalPosition::new(p.x, p.y));
    #[cfg(not(target_os = "macos"))]
    let position = tauri::Position::Physical(PhysicalPosition::new(
        p.x.round() as i32,
        p.y.round() as i32,
    ));
    window.set_position(position).map_err(|e| e.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}
fn direction(anchor: Point, pointer: Point, threshold: f64) -> Option<Direction> {
    let dx = pointer.x - anchor.x;
    let dy = pointer.y - anchor.y;
    if dx.abs().max(dy.abs()) < threshold {
        return None;
    }
    // Equal diagonals prefer horizontal movement over folding/hiding.
    Some(if dx.abs() >= dy.abs() {
        if dx < 0.0 {
            Direction::Left
        } else {
            Direction::Right
        }
    } else if dy < 0.0 {
        Direction::Up
    } else {
        Direction::Down
    })
}

// Latch the first deliberate movement, even during the minimum one-second hold.
// A timer or later reversal never chooses a new direction.
struct Motion {
    shown_at: Instant,
    direction: Option<Direction>,
    threshold: f64,
    anchor: Point,
    moving_since: Option<Instant>,
}
impl Motion {
    fn progress(&mut self, pointer: Point, now: Instant) -> Option<f64> {
        if self.direction.is_none() {
            self.direction = direction(self.anchor, pointer, self.threshold);
        }
        if self.moving_since.is_none()
            && self.direction.is_some()
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
        if near(position(window)?, flight.expected) {
            move_to(
                window,
                point(flight.destination.position, flight.destination.scale),
            )?;
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
pub fn cancel(window: &WebviewWindow) -> Result<(), String> {
    window
        .state::<Entrances>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    Ok(())
}
#[tauri::command]
pub fn drag_note(window: WebviewWindow) -> Result<(), String> {
    cancel(&window)?;
    window.start_dragging().map_err(|e| e.to_string())
}
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= 2.0 && (a.y - b.y).abs() <= 2.0
}

pub fn show(window: &WebviewWindow, destination: Option<Destination>) -> Result<(), String> {
    if window.is_visible().map_err(|e| e.to_string())? {
        return crate::show_without_focus(window);
    }
    finish(window)?;
    let destination = match destination {
        Some(p) => p,
        None => crate::place_new_window(window.app_handle(), window, None)?,
    };
    let anchor = cursor(window)?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = coordinate_scale(window.scale_factor().map_err(|e| e.to_string())?);
    let mut start = None;
    let mut entrance_area = (0.0, 0.0, 0.0, 0.0);
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
        entrance_area = (
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
                area: entrance_area,
                monitor_scale,
                action_started: false,
                motion: Motion {
                    shown_at: started,
                    direction: None,
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
    let window = window.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(16));
        let frame = window.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        if window
            .run_on_main_thread(move || {
                let result = tick(&frame, start, started);
                if let Err(error) = &result {
                    eprintln!("패널 등장 이동 실패: {error}");
                    let _ = finish(&frame);
                }
                let _ = tx.send(result.unwrap_or(false));
            })
            .is_err()
            || rx.recv_timeout(Duration::from_secs(1)) != Ok(true)
        {
            break;
        }
    });
    Ok(())
}
fn tick(window: &WebviewWindow, start: Point, started: Instant) -> Result<bool, String> {
    // A native border drag must take ownership before the next animation frame.
    #[cfg(windows)]
    crate::sizing::native_resize(window)?;
    let state = window.state::<Entrances>();
    let mut flights = state.0.lock().map_err(|e| e.to_string())?;
    if flights
        .get(window.label())
        .is_none_or(|f| f.started != started)
    {
        return Ok(false);
    }
    // Own this generation outside the lock: fold/hide share cancellation paths.
    let mut flight = flights.remove(window.label()).unwrap();
    drop(flights);
    if !near(position(window)?, flight.expected) {
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
    if !hidden && !flight.action_started {
        flight.action_started = true;
        let direction = flight.motion.direction.expect("motion requires direction");
        if direction == Direction::Down {
            // The flight is already cancelled, so the shared X-button path hides
            // at the current position instead of finishing at the old target.
            crate::hide_note(window.clone())?;
            return Ok(false);
        }
        if direction == Direction::Up {
            crate::sizing::fold_for_entrance(window)?;
        }
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let s = coordinate_scale(scale);
        let gap = 8.0 * flight.monitor_scale / coordinate_scale(flight.monitor_scale);
        let target = edge_target(
            flight.area,
            (f64::from(size.width) / s, f64::from(size.height) / s),
            start,
            direction,
            gap,
        );
        let ds = coordinate_scale(flight.monitor_scale);
        flight.destination = Destination {
            position: PhysicalPosition::new(
                (target.x * ds).round() as i32,
                (target.y * ds).round() as i32,
            ),
            scale: flight.monitor_scale,
            size: tauri::PhysicalSize::new(
                (f64::from(size.width) / s * ds).round() as u32,
                (f64::from(size.height) / s * ds).round() as u32,
            ),
        };
        crate::warp::remember(
            window.app_handle(),
            window,
            flight.destination.position.y.into(),
            (flight.area.1 * ds).round() as i32,
            flight.monitor_scale,
        )?;
    }
    let ease = 1.0 - (1.0 - t).powi(3);
    let destination = point(flight.destination.position, flight.destination.scale);
    let next = Point::new(
        start.x + (destination.x - start.x) * ease,
        start.y + (destination.y - start.y) * ease,
    );
    move_to(window, next)?;
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

fn edge_target(
    (x, y, w, h): (f64, f64, f64, f64),
    (nw, nh): (f64, f64),
    start: Point,
    direction: Direction,
    gap: f64,
) -> Point {
    let right = (x + w - nw - gap).max(x + gap);
    let bottom = (y + h - nh - gap).max(y + gap);
    Point::new(
        if direction == Direction::Left {
            x + gap
        } else {
            right
        },
        start.y.clamp(y + gap, bottom),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jitter_dominant_axis_and_equal_diagonals() {
        let anchor = Point::new(-100.0, 200.0);
        for (dx, dy, expected) in [
            (7.9, -7.9, None),
            (8.0, 8.0, Some(Direction::Right)),
            (-8.0, -8.0, Some(Direction::Left)),
            (9.0, -10.0, Some(Direction::Up)),
            (-9.0, 10.0, Some(Direction::Down)),
        ] {
            assert_eq!(
                direction(anchor, Point::new(anchor.x + dx, anchor.y + dy), 8.0),
                expected
            );
        }
    }
    #[test]
    fn first_direction_survives_reversal_before_and_after_hold() {
        for (pointer, expected) in [
            (Point::new(-10.0, 0.0), Direction::Left),
            (Point::new(10.0, 0.0), Direction::Right),
            (Point::new(0.0, -10.0), Direction::Up),
            (Point::new(0.0, 10.0), Direction::Down),
        ] {
            let now = Instant::now();
            let mut motion = Motion {
                shown_at: now,
                direction: None,
                threshold: 8.0,
                anchor: Point::new(0.0, 0.0),
                moving_since: None,
            };
            assert_eq!(motion.progress(Point::new(1.0, 1.0), now), None);
            assert_eq!(motion.direction, None);
            assert_eq!(
                motion.progress(pointer, now + Duration::from_millis(100)),
                None
            );
            assert_eq!(
                motion.progress(
                    Point::new(-pointer.x, -pointer.y),
                    now + Duration::from_secs(1)
                ),
                Some(0.0)
            );
            assert_eq!(motion.direction, Some(expected));
            assert_eq!(
                motion.progress(motion.anchor, now + Duration::from_millis(1420)),
                Some(1.0)
            );
            assert_eq!(motion.direction, Some(expected));
        }
    }
    #[test]
    fn edge_targets_preserve_height_and_clamp_negative_work_areas() {
        let area = (-1600.0, -900.0, 1600.0, 900.0);
        let start = Point::new(-800.0, -400.0);
        assert_eq!(
            edge_target(area, (300.0, 200.0), start, Direction::Left, 8.0),
            Point::new(-1592.0, -400.0)
        );
        assert_eq!(
            edge_target(area, (300.0, 200.0), start, Direction::Right, 8.0),
            Point::new(-308.0, -400.0)
        );
        assert_eq!(
            edge_target(
                area,
                (300.0, 12.0),
                Point::new(-800.0, -10.0),
                Direction::Up,
                8.0
            ),
            Point::new(-308.0, -20.0)
        );
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
            direction: None,
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
            direction: None,
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
