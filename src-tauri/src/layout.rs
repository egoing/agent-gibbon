//! Session-only user intent and open order. No event history or persisted geometry.
mod arrangement;
mod free_space;
pub(crate) use free_space::LastLayout;
pub(crate) mod overview;
use crate::{
    entrance::{self, Destination},
    placement::{self, Rect},
};
pub(crate) use arrangement::Arrangement;
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
pub(crate) fn forget(w: &WebviewWindow) {
    overview::forget(w);
    w.state::<Exclusions>().0.lock().unwrap().remove(w.label());
    w.state::<OpenOrder>()
        .0
        .lock()
        .unwrap()
        .0
        .retain(|label| label != w.label());
    if let Some((_, frames)) = w.state::<Undo>().0.lock().unwrap().as_mut() {
        frames.retain(|(window, _, _)| window.label() != w.label());
    }
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
    if geometric {
        crate::workspace::protect(w);
    }
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

// Explicit arrangement has a wider scope than ordinary note-open reflow.
// Child WebViews and utility windows must never become separate targets.
pub(crate) fn arrangement_id(label: &str) -> Option<&str> {
    crate::memo_id_from_window(label).or_else(|| {
        ["terminal-", "webview-"]
            .iter()
            .find_map(|prefix| label.strip_prefix(prefix).filter(|id| !id.is_empty()))
            .map(|id| {
                if label.starts_with("terminal-") {
                    label
                } else {
                    id
                }
            })
    })
}

fn arrangement_spec(
    mut spec: placement::Item,
    label: &str,
    folded: bool,
    area: Rect,
    u: f64,
) -> placement::Item {
    if folded {
        spec.min = (24.0 * u).round() as i64;
        spec.max = spec.min;
    } else if label.starts_with("terminal-") || label.starts_with("webview-") {
        // These surfaces do not report Markdown content height. A 48px default
        // would leave only a toolbar and an unusable terminal/page viewport.
        spec.min = ((144.0 * u).round() as i64).min(area.height);
        spec.max = ((420.0 * u).round() as i64).min(area.height);
    }
    spec
}
fn arrangement_item(w: &WebviewWindow, area: Rect, scale: f64) -> placement::Item {
    arrangement_spec(
        item(w, area, scale),
        w.label(),
        crate::sizing::folded(w),
        area,
        scale / unit(scale),
    )
}

fn created_at(app: &tauri::AppHandle, label: &str, id: &str) -> Option<std::time::SystemTime> {
    let store = app.state::<crate::storage::Store>();
    if label.starts_with("webview-") {
        return crate::web::created_at(app, id);
    }
    if label.starts_with("terminal-") {
        // Terminal metadata is atomically replaced on geometry changes, so its
        // file birth time is not the terminal's creation time.
        let nanos = id.strip_prefix("terminal-")?.parse::<u64>().ok()?;
        return std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_nanos(nanos));
    }
    let path = store.path(id).parent()?.to_path_buf();
    path.metadata().ok()?.created().ok()
}

// Only ordinary hidden/new opens enter here. Finish obsolete reservations by
// replacing their flight from the currently displayed frame, never by snapping.
pub fn reflow(app: &tauri::AppHandle, opened: &WebviewWindow) -> Result<Destination, String> {
    if overview::protects(opened) {
        return overview::current_frame(opened);
    }
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
        if excluded(&w) || overview::protects(&w) {
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

type Frames = Vec<(
    WebviewWindow,
    Destination,
    crate::monitor_layout::RecoveryPoint,
)>;
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
    for (w, d, recovery) in frames {
        if !w.is_visible().unwrap_or(false) {
            continue;
        }
        entrance::cancel(&w)?;
        entrance::place(&w, d)?;
        crate::workspace::remember_frame(&w, d.position, d.size);
        crate::monitor_layout::restore_override(&w, recovery)?;
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

fn arrangement_rects(
    area: Rect,
    scale: f64,
    arrangement: Arrangement,
    specs: &[placement::Item],
    images: &[Option<tauri::LogicalSize<f64>>],
    viewports: &[bool],
) -> Result<Vec<Rect>, String> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    if area.width <= 0 || area.height <= 0 {
        return Err("메모를 배치할 공간이 부족합니다".into());
    }
    let width = specs.iter().map(|s| s.width).max().unwrap();
    let rects = if arrangement == Arrangement::Grid {
        let gap = (12.0 * scale / unit(scale)).round() as i64;
        let n = specs.len();
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
                    .enumerate()
                    .map(|(i, s)| {
                        if viewports[i] {
                            s.min
                        } else {
                            s.min.min((48.0 * scale / unit(scale)) as i64)
                        }
                    })
                    .max()
                    .unwrap()
        {
            return Err("메모를 배치할 공간이 부족합니다".into());
        }
        let mut frames = Vec::new();
        for i in 0..specs.len() {
            let max_width = cw.min(specs[i].width);
            let max_height = ch.min(specs[i].max);
            let (width, height) = if let Some(img) = images[i] {
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
        arrangement.columns(area, width, specs)
    };
    if rects.len() != specs.len() {
        return Err("메모를 배치할 공간이 부족합니다".into());
    }
    Ok(rects)
}

fn visible_hosts(app: &tauri::AppHandle) -> Result<Vec<WebviewWindow>, String> {
    let mut windows = Vec::new();
    for w in crate::web::managed_windows(app).into_values() {
        let Some(id) = arrangement_id(w.label()) else {
            continue;
        };
        if crate::stash::owns(&w)
            || !w.is_visible().map_err(|e| e.to_string())?
            || w.is_minimized().map_err(|e| e.to_string())?
        {
            continue;
        }
        let created = created_at(app, w.label(), id);
        windows.push((created, w));
    }
    windows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.label().cmp(b.1.label())));
    Ok(windows.into_iter().map(|(_, window)| window).collect())
}

fn planned_frames(
    windows: &[WebviewWindow],
    area: Rect,
    scale: f64,
    arrangement: Arrangement,
) -> Result<Vec<Rect>, String> {
    let specs: Vec<_> = windows
        .iter()
        .map(|w| arrangement_item(w, area, scale))
        .collect();
    let images: Vec<_> = windows
        .iter()
        .map(|w| crate::sizing::image_size(w).filter(|_| !crate::sizing::folded(w)))
        .collect();
    let viewports: Vec<_> = windows
        .iter()
        .map(|w| w.label().starts_with("terminal-") || w.label().starts_with("webview-"))
        .collect();
    arrangement_rects(area, scale, arrangement, &specs, &images, &viewports)
}

fn move_hosts(
    app: &tauri::AppHandle,
    windows: &[WebviewWindow],
    rects: Vec<Rect>,
    scale: f64,
    work_y: i32,
) -> Result<(), String> {
    for (w, r) in windows.iter().zip(rects) {
        move_host(app, w, r, scale, work_y)?;
    }
    Ok(())
}

struct HostMove {
    window: WebviewWindow,
    rect: Rect,
    scale: f64,
    work_y: i32,
}

fn move_host(
    app: &tauri::AppHandle,
    w: &WebviewWindow,
    r: Rect,
    scale: f64,
    work_y: i32,
) -> Result<(), String> {
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
    crate::sizing::cancel(w);
    crate::warp::remember(app, w, dest.position.y.into(), work_y, scale)?;
    entrance::transition(w, dest)?;
    crate::workspace::remember_frame(w, dest.position, dest.size);
    Ok(())
}

pub(crate) fn gesture_monitor(
    app: &tauri::AppHandle,
    x: f64,
    y: f64,
) -> Result<tauri::Monitor, String> {
    pointer_monitor(app, x, y)
        .map_err(|e| e.to_string())?
        .or_else(|| {
            app.get_webview_window("direction-guide")
                .and_then(|w| w.current_monitor().ok().flatten())
        })
        .ok_or_else(|| "포인터 화면 없음".into())
}

pub(crate) fn arrange(
    app: &tauri::AppHandle,
    monitor: tauri::Monitor,
    arrangement: Arrangement,
) -> Result<(), String> {
    let _activity = crate::updater::activity(app)?;
    overview::leave(app)?;
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let area = native_rect(work.position, work.size, scale);
    let windows = visible_hosts(app)?;
    let moves = if arrangement == Arrangement::FreeSpace {
        free_space::frames(app, windows, &monitor)?
    } else {
        let rects = planned_frames(&windows, area, scale, arrangement)?;
        windows
            .into_iter()
            .zip(rects)
            .map(|(window, rect)| HostMove {
                window,
                rect,
                scale,
                work_y: work.position.y,
            })
            .collect()
    };
    if moves.is_empty() {
        return Ok(());
    }
    let panel = app
        .get_webview_window("layout-undo")
        .ok_or("정렬 취소 창 없음")?;
    let mut frames = Vec::new();
    for movement in &moves {
        let w = &movement.window;
        frames.push((
            w.clone(),
            Destination {
                position: w.outer_position().map_err(|e| e.to_string())?,
                size: w.outer_size().map_err(|e| e.to_string())?,
                scale: w.scale_factor().map_err(|e| e.to_string())?,
            },
            crate::monitor_layout::begin_override(w)?,
        ));
    }
    let apply = (|| -> Result<(), String> {
        for m in moves {
            move_host(app, &m.window, m.rect, m.scale, m.work_y)?;
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

    #[test]
    fn explicit_arrangement_includes_all_memo_hosts_but_no_children_or_tools() {
        for (label, id) in [
            ("main", "main"),
            ("note-text", "text"),
            ("note-image-123", "image-123"),
            ("terminal-123", "terminal-123"),
            ("webview-web-123", "web-123"),
        ] {
            assert_eq!(arrangement_id(label), Some(id));
        }
        for label in [
            "note-",
            "terminal-",
            "webview-",
            "webpage-web-123",
            "capture-123-0",
            "direction-guide",
            "layout-undo",
            "manager",
            "stash-indicator",
        ] {
            assert_eq!(arrangement_id(label), None, "{label}");
        }
    }

    #[test]
    fn mixed_surface_fixtures_fit_both_arrangements_at_each_scale_and_negative_origin() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let u = scale / unit(scale);
            let area = Rect {
                x: (-1600.0 * u) as i64,
                y: (-200.0 * u) as i64,
                width: (1440.0 * u) as i64,
                height: (900.0 * u) as i64,
            };
            let labels = ["note-text", "note-image-1", "terminal-1", "webview-web-1"];
            let specs: Vec<_> = labels
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    arrangement_spec(
                        placement::Item {
                            width: (300.0 * u).round() as i64,
                            min: (if i == 1 { 200.0 } else { 48.0 } * u).round() as i64,
                            max: (if i == 1 { 200.0 } else { 48.0 } * u).round() as i64,
                        },
                        label,
                        false,
                        area,
                        u,
                    )
                })
                .collect();
            assert_eq!(specs[2].min, (144.0 * u).round() as i64);
            assert_eq!(specs[3].max, (420.0 * u).round() as i64);
            let images = [
                None,
                Some(tauri::LogicalSize::new(600.0, 400.0)),
                None,
                None,
            ];
            for arrangement in [Arrangement::Left, Arrangement::Right, Arrangement::Grid] {
                let rects = arrangement_rects(
                    area,
                    scale,
                    arrangement,
                    &specs,
                    &images,
                    &[false, false, true, true],
                )
                .unwrap();
                assert_eq!(rects.len(), 4);
                for (i, r) in rects.iter().enumerate() {
                    assert!(r.x >= area.x && r.y >= area.y);
                    assert!(
                        r.x + r.width <= area.x + area.width
                            && r.y + r.height <= area.y + area.height
                    );
                    assert!(r.width > 0 && r.height > 0);
                    if i >= 2 {
                        assert!(r.height >= specs[i].min);
                    }
                    for other in &rects[..i] {
                        assert!(
                            r.x + r.width <= other.x
                                || other.x + other.width <= r.x
                                || r.y + r.height <= other.y
                                || other.y + other.height <= r.y
                        );
                    }
                }
                let image = rects[1];
                assert!((image.width as f64 / image.height as f64 - 1.5).abs() < 0.02);
            }
        }
    }

    #[test]
    fn insufficient_viewport_space_fails_before_any_frames_are_applied() {
        let specs = [placement::Item {
            width: 300,
            min: 144,
            max: 420,
        }; 4];
        for arrangement in [Arrangement::Left, Arrangement::Right, Arrangement::Grid] {
            assert!(arrangement_rects(
                Rect {
                    x: -100,
                    y: 0,
                    width: 300,
                    height: 100
                },
                1.0,
                arrangement,
                &specs,
                &[None; 4],
                &[true; 4]
            )
            .is_err());
        }
        let folded = arrangement_spec(
            specs[0],
            "webview-web-1",
            true,
            Rect {
                x: 0,
                y: 0,
                width: 500,
                height: 500,
            },
            1.0,
        );
        assert_eq!((folded.min, folded.max), (24, 24));
    }

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
