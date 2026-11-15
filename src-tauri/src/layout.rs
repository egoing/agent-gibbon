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
    // Content edits and hidden reopens must not reflow a deliberately zoomed frame.
    if !geometric && crate::sizing::work_area_active(w) {
        return;
    }
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
    let mut windows: Vec<_> = crate::web::managed_windows(app)
        .into_values()
        .filter(|w| {
            !crate::stash::owns(w)
                && (w.label() == "main" || w.label().starts_with("note-"))
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
            #[cfg(target_os = "macos")]
            entrance::place(&w, dest)?;
            #[cfg(windows)]
            {
                w.set_position(dest.position).map_err(|e| e.to_string())?;
                crate::windows_window::set_outer_size(&w, dest.size)?;
            }
            #[cfg(not(any(target_os = "macos", windows)))]
            {
                // Keep the existing physical-coordinate path on other OSes.
                w.set_size(dest.size).map_err(|e| e.to_string())?;
                w.set_position(dest.position).map_err(|e| e.to_string())?;
            }
            result = Some(dest);
        } else {
            entrance::transition(&w, dest)?;
        }
    }
    result.ok_or_else(|| "열 메모의 배치를 계산하지 못했습니다".into())
}

type Frames = Vec<(WebviewWindow, Destination)>;
#[derive(Default)]
pub struct Undo(Mutex<Option<(std::time::Instant, Frames)>>);

pub fn pointer_monitor(
    app: &tauri::AppHandle,
    x: f64,
    y: f64,
) -> tauri::Result<Option<tauri::Monitor>> {
    Ok(app.available_monitors()?.into_iter().find(|m| {
        let u = unit(m.scale_factor());
        let left = m.position().x as f64 / u;
        let top = m.position().y as f64 / u;
        x >= left
            && y >= top
            && x < left + m.size().width as f64 / u
            && y < top + m.size().height as f64 / u
    }))
}
pub fn expire_undo(app: &tauri::AppHandle) {
    let state = app.state::<Undo>();
    let mut slot = state.0.lock().unwrap();
    if slot
        .as_ref()
        .is_some_and(|(until, _)| std::time::Instant::now() >= *until)
    {
        slot.take();
        if let Some(w) = app.get_webview_window("layout-undo") {
            let _ = w.hide();
        }
    }
}
fn restore_frames(frames: Frames) -> Result<(), String> {
    for (w, d) in frames {
        if !w.is_visible().unwrap_or(false) {
            continue;
        }
        entrance::cancel(&w)?;
        entrance::place(&w, d)?;
        if let Some(m) = w.current_monitor().map_err(|e| e.to_string())? {
            crate::warp::remember(
                w.app_handle(),
                &w,
                d.position.y.into(),
                m.work_area().position.y,
                d.scale,
            )?;
        }
    }
    Ok(())
}
#[cfg(target_os = "macos")]
pub fn click_undo_if_hovered(app: &tauri::AppHandle) {
    if app.state::<Undo>().0.lock().unwrap().is_none() {
        return;
    }
    let Some(w) = app.get_webview_window("layout-undo") else {
        return;
    };
    if let Ok(ptr) = w.ns_window() {
        let native = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        let p = objc2_app_kit::NSEvent::mouseLocation();
        let r = native.frame();
        if p.x >= r.origin.x
            && p.x < r.origin.x + r.size.width
            && p.y >= r.origin.y
            && p.y < r.origin.y + r.size.height
        {
            let _ = undo_arrangement(w);
        }
    }
}
#[tauri::command]
pub fn undo_arrangement(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "layout-undo" {
        return Err("정렬 취소 창 전용입니다".into());
    }
    let snapshot = window.state::<Undo>().0.lock().unwrap().take();
    window.hide().map_err(|e| e.to_string())?;
    if let Some((until, frames)) = snapshot {
        if std::time::Instant::now() < until {
            return restore_frames(frames);
        }
    }
    Ok(())
}

pub fn arrange(app: &tauri::AppHandle, x: f64, y: f64, center: bool) -> Result<(), String> {
    let _activity = crate::updater::activity(app)?;
    let monitor = pointer_monitor(app, x, y)
        .map_err(|e| e.to_string())?
        .or_else(|| {
            app.get_webview_window("direction-guide")
                .and_then(|w| w.current_monitor().ok().flatten())
        })
        .ok_or("포인터 화면 없음")?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let area = native_rect(work.position, work.size, scale);
    let mut windows = Vec::new();
    for w in crate::web::managed_windows(app).into_values() {
        let Some(id) = crate::memo_id_from_window(w.label()) else {
            continue;
        };
        if crate::stash::owns(&w)
            || !w.is_visible().map_err(|e| e.to_string())?
            || w.is_minimized().map_err(|e| e.to_string())?
        {
            continue;
        }
        let created = app
            .state::<crate::storage::Store>()
            .path(id)
            .parent()
            .and_then(|p| p.metadata().ok())
            .and_then(|m| m.created().ok());
        windows.push((created, w));
    }
    windows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.label().cmp(b.1.label())));
    if windows.is_empty() {
        return Ok(());
    }
    let specs: Vec<_> = windows
        .iter()
        .map(|(_, w)| {
            let mut s = item(w, area, scale);
            if crate::sizing::folded(w) {
                s.min = (24.0 * scale / unit(scale)).round() as i64;
                s.max = s.min;
            }
            s
        })
        .collect();
    let width = specs.iter().map(|s| s.width).max().unwrap();
    let rects = if center {
        let gap = (12.0 * scale / unit(scale)).round() as i64;
        let n = windows.len();
        let cols = ((n as f64 * area.width as f64 / area.height as f64)
            .sqrt()
            .round() as usize)
            .clamp(1, n);
        let rows = n.div_ceil(cols);
        let cw = (area.width - gap * (cols as i64 + 1)) / cols as i64;
        let ch = (area.height - gap * (rows as i64 + 1)) / rows as i64;
        if cw < (144.0 * scale / unit(scale)) as i64
            || ch
                < specs
                    .iter()
                    .map(|s| s.min.min((48.0 * scale / unit(scale)) as i64))
                    .max()
                    .unwrap()
        {
            return Err("메모를 배치할 공간이 부족합니다".into());
        }
        let mut frames = Vec::new();
        for (i, (_, w)) in windows.iter().enumerate() {
            let max_width = cw.min(specs[i].width);
            let max_height = ch.min(specs[i].max);
            let (width, height) = if let Some(img) =
                crate::sizing::image_size(w).filter(|_| !crate::sizing::folded(w))
            {
                let fit = (max_width as f64 / img.width).min(max_height as f64 / img.height);
                (
                    (img.width * fit).round().max(1.) as i64,
                    (img.height * fit).round().max(1.) as i64,
                )
            } else {
                (max_width, max_height)
            };
            frames.push(Rect {
                x: area.x + gap + (i % cols) as i64 * (cw + gap) + (cw - width) / 2,
                y: area.y + gap + (i / cols) as i64 * (ch + gap) + (ch - height) / 2,
                width,
                height,
            });
        }
        frames
    } else {
        placement::stack(area, width, &specs, &[])
    };
    if rects.len() != windows.len() {
        return Err("메모를 배치할 공간이 부족합니다".into());
    }
    let panel = app
        .get_webview_window("layout-undo")
        .ok_or("정렬 취소 창 없음")?;
    let mut frames = Vec::new();
    for (_, w) in &windows {
        frames.push((
            w.clone(),
            Destination {
                position: w.outer_position().map_err(|e| e.to_string())?,
                size: w.outer_size().map_err(|e| e.to_string())?,
                scale: w.scale_factor().map_err(|e| e.to_string())?,
            },
        ));
    }
    let apply = (|| -> Result<(), String> {
        for ((_, w), r) in windows.into_iter().zip(rects) {
            let u = unit(scale);
            let dest = Destination {
                position: tauri::PhysicalPosition::new(
                    (r.x as f64 * u).round() as i32,
                    (r.y as f64 * u).round() as i32,
                ),
                size: tauri::PhysicalSize::new(
                    (r.width as f64 * u).round() as u32,
                    (r.height as f64 * u).round() as u32,
                ),
                scale,
            };
            crate::warp::remember(app, &w, dest.position.y.into(), work.position.y, scale)?;
            entrance::transition(&w, dest)?;
        }
        let x = work.position.x as f64 + work.size.width as f64 / 2. - 72. * scale;
        let y = work.position.y as f64 + work.size.height as f64 - 56. * scale;
        panel
            .set_position(tauri::PhysicalPosition::new(
                x.round() as i32,
                y.round() as i32,
            ))
            .map_err(|e| e.to_string())?;
        panel
            .eval("window.startCountdown()")
            .map_err(|e| e.to_string())?;
        crate::stash::show_frame(&panel)
    })();
    if let Err(e) = apply {
        restore_frames(frames)?;
        return Err(e);
    }
    *app.state::<Undo>().0.lock().unwrap() = Some((
        std::time::Instant::now() + std::time::Duration::from_secs(5),
        frames,
    ));
    Ok(())
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
            [(300, 60), (300, 0), (0, 60), (0, 0)]
        );
    }
}
