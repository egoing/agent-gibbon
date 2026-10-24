//! Main-thread window movement; sleeping happens only on the worker thread.
use std::{
    collections::BTreeMap,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, PhysicalPosition, WebviewWindow};

struct Flight {
    destination: PhysicalPosition<i32>,
    expected: PhysicalPosition<i32>,
    started: Instant,
}
#[derive(Default)]
pub struct Entrances(Mutex<BTreeMap<String, Flight>>);

// Stop pending frames before hiding/warping, retaining the original destination.
pub fn finish(window: &WebviewWindow) -> Result<(), String> {
    let state = window.state::<Entrances>();
    let flight = state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    if let Some(flight) = flight {
        if near(
            window.outer_position().map_err(|e| e.to_string())?,
            flight.expected,
        ) {
            window
                .set_position(flight.destination)
                .map_err(|e| e.to_string())?;
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

pub fn destination(window: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    window
        .state::<Entrances>()
        .0
        .lock()
        .ok()?
        .get(window.label())
        .map(|flight| flight.destination)
}

fn near(a: PhysicalPosition<i32>, b: PhysicalPosition<i32>) -> bool {
    (i64::from(a.x) - i64::from(b.x)).abs() <= 2 && (i64::from(a.y) - i64::from(b.y)).abs() <= 2
}

pub fn show(
    window: &WebviewWindow,
    destination: Option<PhysicalPosition<i32>>,
) -> Result<(), String> {
    if window.is_visible().map_err(|e| e.to_string())? {
        return crate::show_without_focus(window);
    }
    finish(window)?;
    let destination = match destination {
        Some(p) => p,
        None => crate::place_new_window(window.app_handle(), window, None)?,
    };
    let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? else {
        return crate::show_without_focus(window);
    };
    let area = monitor.work_area();
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let start = PhysicalPosition::new(
        (i64::from(area.position.x) + (i64::from(area.size.width) - i64::from(size.width)) / 2)
            as i32,
        (i64::from(area.position.y) + (i64::from(area.size.height) - i64::from(size.height)) / 2)
            as i32,
    );
    window.set_position(start).map_err(|e| e.to_string())?;
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
            },
        );
    if let Err(error) = crate::show_without_focus(window) {
        let _ = finish(window);
        return Err(error);
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

fn tick(
    window: &WebviewWindow,
    start: PhysicalPosition<i32>,
    started: Instant,
) -> Result<bool, String> {
    let state = window.state::<Entrances>();
    let mut flights = state.0.lock().map_err(|e| e.to_string())?;
    let Some(flight) = flights.get_mut(window.label()) else {
        return Ok(false);
    };
    // Ignore frames from a previous opening of the same window.
    if flight.started != started {
        return Ok(false);
    }
    let current = window.outer_position().map_err(|e| e.to_string())?;
    if !near(current, flight.expected) {
        // A user drag or other explicit move takes precedence.
        flights.remove(window.label());
        return Ok(false);
    }
    let hidden = !window.is_visible().map_err(|e| e.to_string())?;
    let elapsed = started.elapsed().as_secs_f64();
    if elapsed < 3.0 && !hidden {
        return Ok(true);
    }
    let t = ((elapsed - 3.0).max(0.0) / 0.420).min(1.0);
    let ease = if hidden { 1.0 } else { 1.0 - (1.0 - t).powi(3) };
    let interpolate =
        |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * ease).round() as i32;
    let next = PhysicalPosition::new(
        interpolate(start.x, flight.destination.x),
        interpolate(start.y, flight.destination.y),
    );
    window.set_position(next).map_err(|e| e.to_string())?;
    flight.expected = next;
    if t >= 1.0 || hidden {
        flights.remove(window.label());
        return Ok(false);
    }
    Ok(true)
}
