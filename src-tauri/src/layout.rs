//! Session-only user intent and open order. No event history or persisted geometry.
use crate::{
    entrance::{self, Destination},
    placement::{self, Rect},
};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Manager, WebviewWindow};
#[derive(Default)]
pub struct Exclusions(Mutex<BTreeMap<String, bool>>);
#[derive(Default)]
pub struct OpenOrder(Mutex<Order>);

#[derive(Default)]
struct Order(Vec<String>);
impl Order {
    fn open(&mut self, label: &str, already_open: bool) {
        if already_open && self.0.iter().any(|entry| entry == label) {
            return;
        }
        self.0.retain(|entry| entry != label);
        self.0.push(label.into());
    }
    fn rank(&self, label: &str) -> usize {
        self.0
            .iter()
            .position(|entry| entry == label)
            .unwrap_or(usize::MAX)
    }
}

// Reserve at request time, before asynchronous content measurement completes.
// Visible/pending reopens keep their place; a hidden restore joins the tail.
pub fn register(w: &WebviewWindow, already_open: bool) {
    w.state::<OpenOrder>()
        .0
        .lock()
        .unwrap()
        .open(w.label(), already_open);
}
pub fn excluded(w: &WebviewWindow) -> bool {
    w.state::<Exclusions>()
        .0
        .lock()
        .unwrap()
        .get(w.label())
        .copied()
        .unwrap_or(false)
}
pub fn mark(w: &WebviewWindow, geometric: bool) {
    w.state::<Exclusions>()
        .0
        .lock()
        .unwrap()
        .insert(w.label().into(), geometric);
}
#[tauri::command]
pub fn content_action(window: WebviewWindow) {
    mark(&window, false);
}

// AppKit rounds window geometry to whole logical pixels. Solve in that same
// coordinate space; other platforms use native physical desktop coordinates.
fn unit(scale: f64) -> f64 {
    if cfg!(target_os = "macos") {
        scale
    } else {
        1.0
    }
}
fn native_rect(p: tauri::PhysicalPosition<i32>, s: tauri::PhysicalSize<u32>, scale: f64) -> Rect {
    let u = unit(scale);
    Rect {
        x: (p.x as f64 / u).round() as i64,
        y: (p.y as f64 / u).round() as i64,
        width: (s.width as f64 / u).round() as i64,
        height: (s.height as f64 / u).round() as i64,
    }
}
fn window_rect(w: &WebviewWindow) -> Result<Rect, String> {
    Ok(native_rect(
        w.outer_position().map_err(|e| e.to_string())?,
        w.outer_size().map_err(|e| e.to_string())?,
        w.scale_factor().map_err(|e| e.to_string())?,
    ))
}
fn item(w: &WebviewWindow, area: Rect, scale: f64) -> placement::Item {
    let scale = scale / unit(scale);
    let width = (300.0 * scale).round() as i64;
    if crate::sizing::folded(w) {
        let h = (12.0 * scale).round() as i64;
        placement::Item {
            width: width.min(area.width),
            min: h,
            max: h,
        }
    } else if let Some(s) = crate::sizing::image_size(w) {
        let factor = 1.0_f64
            .min(300.0 / s.width)
            .min(420.0 / s.height)
            .min(area.width as f64 / scale / s.width)
            .min(area.height as f64 / scale / s.height);
        let h = (s.height * factor * scale).round().max(1.0) as i64;
        placement::Item {
            width: (s.width * factor * scale).round().max(1.0) as i64,
            min: h,
            max: h,
        }
    } else {
        placement::Item {
            width: width.min(area.width),
            min: ((48.0 * scale).round() as i64).min(area.height),
            max: ((crate::sizing::content_height(w) * scale).round() as i64).min(area.height),
        }
    }
}

// Only ordinary hidden/new opens enter here. Finish obsolete reservations by
// replacing their flight from the currently displayed frame, never by snapping.
pub fn reflow(app: &tauri::AppHandle, opened: &WebviewWindow) -> Result<Destination, String> {
    let mut windows: Vec<_> = app
        .webview_windows()
        .into_values()
        .filter(|w| {
            (w.label() == "main" || w.label().starts_with("note-"))
                && (w.label() == opened.label() || w.is_visible().unwrap_or(false))
        })
        .collect();
    {
        let state = app.state::<OpenOrder>();
        let order = state.0.lock().unwrap();
        windows.sort_by_key(|w| order.rank(w.label()));
    }
    let mut obstacles = Vec::new();
    let mut targets = Vec::new();
    for w in windows {
        if excluded(&w) {
            obstacles.push(window_rect(&w)?);
        } else {
            targets.push(w);
        }
    }
    let mut monitors = opened.available_monitors().map_err(|e| e.to_string())?;
    if let Some(current) = opened.current_monitor().map_err(|e| e.to_string())? {
        if let Some(i) = monitors
            .iter()
            .position(|m| m.position() == current.position())
        {
            monitors.swap(0, i);
        }
    }
    let mut planned = Vec::new();
    let mut next = 0;
    for m in &monitors {
        let work = m.work_area();
        let scale = m.scale_factor();
        let area = native_rect(work.position, work.size, scale);
        if area.width < (144.0 * scale / unit(scale)) as i64
            || area.height < (32.0 * scale / unit(scale)) as i64
        {
            continue;
        }
        let width = (300.0 * scale / unit(scale)).round() as i64;
        let specs: Vec<_> = targets[next..]
            .iter()
            .map(|w| item(w, area, scale))
            .collect();
        let rects = placement::stack(area, width.min(area.width), &specs, &obstacles);
        for rect in rects {
            planned.push((targets[next].clone(), rect, scale, work.position.y));
            next += 1;
        }
        if next == targets.len() {
            break;
        }
    }
    // Capacity exhausted: leave all existing windows and their reservations in
    // place, and admit only the new window with the least possible overlap.
    let mut result = if excluded(opened) {
        Some(Destination {
            position: opened.outer_position().map_err(|e| e.to_string())?,
            size: opened.outer_size().map_err(|e| e.to_string())?,
            scale: opened.scale_factor().map_err(|e| e.to_string())?,
        })
    } else {
        None
    };
    if next < targets.len() {
        planned.clear();
        if !excluded(opened) {
            let occupied: Vec<_> = obstacles
                .iter()
                .copied()
                .chain(
                    targets
                        .iter()
                        .filter(|w| w.label() != opened.label())
                        .filter_map(|w| {
                            if let Some(d) = entrance::destination(w) {
                                return Some(native_rect(d.position, d.size, d.scale));
                            }
                            window_rect(w).ok()
                        }),
                )
                .collect();
            let mut best = None;
            for m in &monitors {
                let work = m.work_area();
                let scale = m.scale_factor();
                let area = native_rect(work.position, work.size, scale);
                if area.width < (144.0 * scale / unit(scale)) as i64
                    || area.height < (32.0 * scale / unit(scale)) as i64
                {
                    continue;
                }
                let spec = item(opened, area, scale);
                if let Some((score, r)) =
                    placement::least_overlap(area, (spec.width, spec.min), &occupied)
                {
                    if best.as_ref().is_none_or(|(old, _, _, _)| score < *old) {
                        best = Some((score, r, scale, work.position.y));
                    }
                }
            }
            let (_, r, scale, work_y) = best.ok_or("배치할 화면이 너무 작습니다")?;
            planned.push((opened.clone(), r, scale, work_y));
        }
    }
    for (w, r, scale, work_y) in planned {
        let dest = Destination {
            position: tauri::PhysicalPosition::new(
                (r.x as f64 * unit(scale)).round() as i32,
                (r.y as f64 * unit(scale)).round() as i32,
            ),
            size: tauri::PhysicalSize::new(
                (r.width as f64 * unit(scale)).round() as u32,
                (r.height as f64 * unit(scale)).round() as u32,
            ),
            scale,
        };
        crate::warp::remember(app, &w, dest.position.y.into(), work_y, scale)?;
        if w.label() == opened.label() {
            entrance::cancel(&w)?;
            w.set_size(dest.size).map_err(|e| e.to_string())?;
            w.set_position(dest.position).map_err(|e| e.to_string())?;
            result = Some(dest);
        } else {
            entrance::transition(&w, dest)?;
        }
    }
    result.ok_or_else(|| "열 메모의 배치를 계산하지 못했습니다".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(order: &Order, labels: &[&str]) -> Vec<String> {
        let mut labels: Vec<_> = labels.iter().map(|s| s.to_string()).collect();
        labels.sort_by_key(|s| order.rank(s));
        labels
    }

    #[test]
    fn reverse_ids_follow_requests_even_when_measurements_finish_in_reverse() {
        let mut order = Order::default();
        for label in ["note-z", "note-m", "note-a"] {
            order.open(label, false);
        }
        assert_eq!(
            sorted(&order, &["note-a", "note-m", "note-z"]),
            ["note-z", "note-m", "note-a"]
        );
        // A repeated open while visible or awaiting measurement is idempotent.
        order.open("note-z", true);
        assert_eq!(order.0, ["note-z", "note-m", "note-a"]);
    }

    #[test]
    fn hidden_restore_moves_only_that_note_to_the_tail() {
        let mut order = Order::default();
        // Restored storage/region creation may register before ordinary opens.
        for label in ["note-a", "note-m", "note-z"] {
            order.open(label, true);
        }
        for label in ["note-z", "note-m", "note-a"] {
            order.open(label, false);
        }
        order.open("note-m", false);
        assert_eq!(order.0, ["note-z", "note-a", "note-m"]);
        // Hidden entries/excluded obstacles do not reorder remaining targets.
        assert_eq!(sorted(&order, &["note-m", "note-z"]), ["note-z", "note-m"]);
        order.open("note-a", false);
        assert_eq!(order.0, ["note-z", "note-m", "note-a"]);
    }

    #[test]
    fn request_order_continues_into_left_column() {
        let mut order = Order::default();
        for label in ["note-z", "note-y", "note-b", "note-a"] {
            order.open(label, false);
        }
        let labels = sorted(&order, &["note-a", "note-b", "note-y", "note-z"]);
        let rects = placement::stack(
            Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 120,
            },
            300,
            &vec![
                placement::Item {
                    width: 300,
                    min: 60,
                    max: 60
                };
                labels.len()
            ],
            &[],
        );
        assert_eq!(labels, ["note-z", "note-y", "note-b", "note-a"]);
        assert_eq!(
            rects.iter().map(|r| (r.x, r.y)).collect::<Vec<_>>(),
            [(300, 0), (300, 60), (0, 0), (0, 60)]
        );
    }
}
