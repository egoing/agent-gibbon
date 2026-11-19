//! Rectangles use thousandths of the selected display, independent of DPI.
mod mask;
use crate::capture::{self, ImageNote};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use tauri::Manager;
use xcap::Monitor;

#[derive(Clone, Copy)]
struct Frame {
    position: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
}
impl Frame {
    fn of(window: &tauri::WebviewWindow) -> Result<Self, String> {
        Ok(Self {
            position: window.outer_position().map_err(err)?,
            size: window.outer_size().map_err(err)?,
        })
    }
    fn matches(self, other: Self) -> bool {
        self.position.x.abs_diff(other.position.x) <= 2
            && self.position.y.abs_diff(other.position.y) <= 2
            && self.size.width.abs_diff(other.size.width) <= 2
            && self.size.height.abs_diff(other.size.height) <= 2
    }
}
struct Placement {
    expected: Frame,
    changed: bool,
    pending: bool,
}
#[derive(Default)]
pub(crate) struct Placements(Mutex<HashMap<String, Placement>>);

fn record_placement(app: &tauri::AppHandle, result: Value) -> Result<Value, String> {
    if let Some(id) = result["id"].as_str() {
        if let Some(window) = find_window(app, id) {
            let expected = Frame::of(&window)?;
            app.state::<Placements>().0.lock().map_err(err)?.insert(
                window.label().into(),
                Placement {
                    expected,
                    changed: false,
                    pending: !window.is_visible().unwrap_or(false),
                },
            );
        }
    }
    Ok(result)
}
fn find_window(app: &tauri::AppHandle, id: &str) -> Option<tauri::WebviewWindow> {
    crate::web::managed_windows(app)
        .into_iter()
        .find_map(|(label, window)| {
            (crate::layout::arrangement_id(&label) == Some(id)).then_some(window)
        })
}
pub(crate) fn placement_ready(window: &tauri::WebviewWindow) {
    let Ok(frame) = Frame::of(window) else { return };
    if let Ok(mut placements) = window.state::<Placements>().0.lock() {
        if let Some(placement) = placements.get_mut(window.label()) {
            if placement.pending {
                placement.expected = frame;
                placement.pending = false;
            }
        }
    }
}
pub(crate) fn protect(window: &tauri::WebviewWindow) {
    if let Some(state) = window.try_state::<Placements>() {
        if let Ok(mut placements) = state.0.lock() {
            if let Some(placement) = placements.get_mut(window.label()) {
                placement.changed = true;
            }
        }
    }
}
pub(crate) fn remember_frame(
    window: &tauri::WebviewWindow,
    position: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
) {
    if let Ok(mut placements) = window.state::<Placements>().0.lock() {
        if let Some(placement) = placements.get_mut(window.label()) {
            placement.expected = Frame { position, size };
        }
    }
}

pub(crate) fn fixed_for_gesture(window: &tauri::WebviewWindow) -> bool {
    let (source, _, allowed) = placement_state(window);
    if source == "agent" {
        !allowed
    } else {
        crate::layout::excluded(window)
    }
}

fn placement_state(window: &tauri::WebviewWindow) -> (&'static str, Option<bool>, bool) {
    let actual = Frame::of(window).ok();
    let moving = crate::entrance::active(window);
    let unfocused = matches!(window.is_focused(), Ok(false));
    let state = window.state::<Placements>();
    let Ok(mut placements) = state.0.lock() else {
        return ("unknown", None, false);
    };
    let Some(placement) = placements.get_mut(window.label()) else {
        return ("unknown", None, false);
    };
    if !placement.pending
        && !moving
        && actual.is_some_and(|frame| !placement.expected.matches(frame))
    {
        placement.changed = true;
    }
    (
        "agent",
        Some(placement.changed),
        !placement.pending && !moving && !placement.changed && actual.is_some() && unfocused,
    )
}

struct Share {
    displays: Vec<u32>,
    files: tempfile::TempDir,
}
impl Share {
    fn token(&self) -> &str {
        self.files.path().file_name().unwrap().to_str().unwrap()
    }
}
#[derive(Default)]
pub(crate) struct Sharing(Mutex<Option<Share>>);
pub(crate) fn shared_displays(app: &tauri::AppHandle) -> Option<Vec<u32>> {
    app.state::<Sharing>()
        .0
        .lock()
        .ok()?
        .as_ref()
        .map(|s| s.displays.clone())
}
pub(crate) fn stop(app: &tauri::AppHandle, token: Option<&str>) -> Result<(), String> {
    let state = app.state::<Sharing>();
    let mut guard = state.0.lock().map_err(err)?;
    if token.is_some_and(|t| !guard.as_ref().is_some_and(|s| s.token() == t)) {
        return Err("종료된 공유입니다".into());
    }
    guard.take();
    drop(guard);
    crate::tray::refresh(app)
}

pub const USAGE: &str = "workspace run TERMINAL_ID [--request-id ID] -- PROGRAM ARGS | run TERMINAL_ID [--request-id ID] --shell SCRIPT | run-status TOKEN | displays | share DISPLAY... | stop SESSION | snapshot --all | snapshot SESSION | snapshot DISPLAY SESSION | move ID DISPLAY X,Y [--automatic] | fit ID DISPLAY X,Y,W,H [--automatic] | note|terminal DISPLAY X,Y,W,H | web DISPLAY X,Y,W,H URL | capture DISPLAY SESSION SOURCE_RECT DEST_RECT (rect units 0..1000)";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    Run(crate::terminal::cli::Execution),
    RunStatus(String),
    Displays,
    Share(Vec<u32>),
    Batch(String),
    AllSnapshots,
    Stop(String),
    Snapshot(u32, String),
    Move(String, u32, u32, u32, bool),
    Fit(String, u32, Rect, bool),
    Note(u32, Rect),
    Terminal(u32, Rect),
    Web(u32, Rect, String),
    Capture(u32, String, Rect, Rect),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect([u32; 4]);
impl Rect {
    fn components(self) -> Result<[f64; 4], String> {
        let [x, y, w, h] = self.0;
        if w == 0 || h == 0 || x.saturating_add(w) > 1000 || y.saturating_add(h) > 1000 {
            return Err("영역은 화면 안에 있어야 합니다".into());
        }
        Ok(self.0.map(|v| f64::from(v) / 1000.0))
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn parse(args: &[String]) -> Result<Operation, String> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    let rect = |s: &str| -> Result<Rect, String> {
        let values = s
            .split(',')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        let r = Rect(values.try_into().map_err(|_| USAGE)?);
        r.components()?;
        Ok(r)
    };
    Ok(match words.as_slice() {
        ["run", rest @ ..] => Operation::Run(crate::terminal::cli::parse(
            &rest.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )?),
        ["run-status", token] => Operation::RunStatus(token.to_string()),
        ["displays"] => Operation::Displays,
        ["share", ids @ ..] => Operation::Share(
            ids.iter()
                .map(|id| id.parse())
                .collect::<Result<_, _>>()
                .map_err(err)?,
        ),
        ["snapshot", "--all"] => Operation::AllSnapshots,
        ["snapshot", token] => Operation::Batch(token.to_string()),
        ["stop", t] => Operation::Stop(t.to_string()),
        ["snapshot", d, t] => Operation::Snapshot(d.parse().map_err(err)?, t.to_string()),
        ["move", id, d, point, options @ ..]
            if options.is_empty() || *options == ["--automatic"] =>
        {
            let (x, y) = point
                .split_once(',')
                .ok_or("위치는 X,Y 형식이어야 합니다")?;
            Operation::Move(
                id.to_string(),
                d.parse().map_err(err)?,
                x.parse().map_err(err)?,
                y.parse().map_err(err)?,
                !options.is_empty(),
            )
        }
        ["fit", id, d, r, options @ ..] if options.is_empty() || *options == ["--automatic"] => {
            Operation::Fit(
                id.to_string(),
                d.parse().map_err(err)?,
                rect(r)?,
                !options.is_empty(),
            )
        }
        ["note", d, r] => Operation::Note(d.parse().map_err(err)?, rect(r)?),
        ["terminal", d, r] => Operation::Terminal(d.parse().map_err(err)?, rect(r)?),
        ["web", d, r, url] => {
            crate::web::validate_url(url)?;
            Operation::Web(d.parse().map_err(err)?, rect(r)?, url.to_string())
        }
        ["capture", d, t, s, r] => {
            Operation::Capture(d.parse().map_err(err)?, t.to_string(), rect(s)?, rect(r)?)
        }
        _ => return Err(USAGE.into()),
    })
}
pub fn execute(app: &tauri::AppHandle, op: &Operation) -> Result<Value, String> {
    use Operation::*;
    if let Stop(token) = op {
        stop(app, Some(token))?;
        return Ok(json!({"ok":true}));
    }
    if let Run(execution) = op {
        return crate::terminal::cli::run(app, execution);
    }
    if let RunStatus(token) = op {
        return crate::terminal::cli::status(app, token);
    }
    let displays = Monitor::all().map_err(err)?;
    let all = matches!(op, AllSnapshots);
    let all_ids;
    let selected = match op {
        Share(ids) => Some(ids),
        AllSnapshots => {
            all_ids = displays
                .iter()
                .map(|m| m.id().map_err(err))
                .collect::<Result<Vec<_>, _>>()?;
            Some(&all_ids)
        }
        _ => None,
    };
    if let Some(selected) = selected {
        if selected.is_empty()
            || selected.iter().enumerate().any(|(i, id)| {
                selected[..i].contains(id) || !displays.iter().any(|m| m.id().ok() == Some(*id))
            })
        {
            return Err("서로 다른 연결된 모니터 ID를 지정하세요".into());
        }
        let state = app.state::<Sharing>();
        let mut guard = state.0.lock().map_err(err)?;
        if guard.is_some() {
            return Err("다른 화면 공유가 진행 중입니다".into());
        }
        let files = tempfile::Builder::new()
            .prefix("jajan-share-")
            .tempdir()
            .map_err(err)?;
        let share = self::Share {
            displays: selected.clone(),
            files,
        };
        let token = share.token().to_owned();
        *guard = Some(share);
        drop(guard);
        if let Err(error) = crate::tray::refresh(app) {
            stop(app, Some(&token)).ok();
            return Err(error);
        }
        if all {
            return match snapshots(app, &displays, &token, None) {
                Ok(mut result) => {
                    result["session"] = json!(token);
                    Ok(result)
                }
                Err(error) => {
                    let _ = stop(app, Some(&token));
                    Err(error)
                }
            };
        }
        return Ok(json!({"ok":true,"session":token,"display":selected[0],"displays":selected}));
    }
    if let Batch(token) = op {
        return snapshots(app, &displays, token, None);
    }
    if let Snapshot(id, token) = op {
        return snapshots(app, &displays, token, Some(*id));
    }
    let id = match op {
        Displays => {
            let mut items = Vec::new();
            for m in displays {
                items.push(json!({"id":m.id().map_err(err)?,"primary":m.is_primary().map_err(err)?,"width":m.width().map_err(err)?,"height":m.height().map_err(err)?}));
            }
            return Ok(json!({"ok":true,"displays":items}));
        }
        RunStatus(_) | Run(..) | Stop(_) | Share(_) | Batch(_) | AllSnapshots | Snapshot(..) => {
            unreachable!()
        }
        Move(_, d, _, _, _)
        | Fit(_, d, _, _)
        | Note(d, _)
        | Terminal(d, _)
        | Web(d, _, _)
        | Capture(d, _, _, _) => *d,
    };
    let m = displays
        .into_iter()
        .find(|m| m.id().ok() == Some(id))
        .ok_or("모니터가 연결되지 않았습니다")?;
    let bounds = |r: Rect| -> Result<ImageNote, String> {
        let [x, y, w, h] = r.components()?;
        let width = f64::from(m.width().map_err(err)?);
        let height = f64::from(m.height().map_err(err)?);
        let scale = if cfg!(windows) {
            f64::from(m.scale_factor().map_err(err)?)
        } else {
            1.0
        };
        Ok(ImageNote {
            x: f64::from(m.x().map_err(err)?) + x * width,
            y: f64::from(m.y().map_err(err)?) + y * height,
            width: w * width / scale,
            height: h * height / scale,
        })
    };
    match op {
        RunStatus(_) | Run(..) | Stop(_) | Displays | Share(_) | Batch(_) | AllSnapshots
        | Snapshot(..) => {
            unreachable!()
        }
        Move(id, _, x, y, automatic) => move_window(app, &m, id, (*x, *y), None, *automatic),
        Fit(id, _, rect, automatic) => {
            rect.components()?;
            let [x, y, w, h] = rect.0;
            move_window(app, &m, id, (x, y), Some((w, h)), *automatic)
        }
        Web(_, r, url) => record_placement(
            app,
            crate::web::create(app, Some(bounds(*r)?), None, Some(url))?,
        ),
        Note(_, r) => record_placement(
            app,
            json!({"ok":true,"id":capture::create_text_note(app,bounds(*r)?, false)?}),
        ),
        Terminal(_, r) => {
            record_placement(app, crate::terminal::create_terminal(app, bounds(*r)?)?)
        }
        Capture(_, token, source, dest) => {
            let state = app.state::<Sharing>();
            let guard = state.0.lock().map_err(err)?;
            guard
                .as_ref()
                .filter(|s| s.token() == token && s.displays.contains(&id))
                .ok_or("화면 공유가 종료되었거나 대상이 다릅니다")?;
            #[cfg(target_os = "macos")]
            if !capture::permission_status().0 {
                return Err("화면 기록 권한이 필요합니다".into());
            }
            let image = m.capture_image().map_err(err)?;

            let [x, y, w, h] = source.components()?;
            let px = |v: f64| (v * f64::from(image.width())).round() as u32;
            let py = |v: f64| (v * f64::from(image.height())).round() as u32;
            let (width, height) = (px(x + w) - px(x), py(y + h) - py(y));
            if width == 0 || height == 0 {
                return Err("캡처 영역이 너무 작습니다".into());
            }
            let crop =
                xcap::image::imageops::crop_imm(&image, px(x), py(y), width, height).to_image();
            drop(guard);
            let id = capture::create_note(app, capture::png(&crop)?, bounds(*dest)?, None)?;
            record_placement(app, json!({"ok":true,"id":id}))
        }
    }
}

fn move_window(
    app: &tauri::AppHandle,
    monitor: &Monitor,
    id: &str,
    (x, y): (u32, u32),
    extent: Option<(u32, u32)>,
    automatic: bool,
) -> Result<Value, String> {
    if x >= 1000 || y >= 1000 {
        return Err("위치는 0~999 범위여야 합니다".into());
    }
    let window = find_window(app, id).ok_or("메모 창을 찾을 수 없습니다")?;
    if automatic && !placement_state(&window).2 {
        return Err("이 창의 배치는 자동으로 변경할 수 없습니다".into());
    }
    if !crate::manager::window_visible(&window)? || window.is_minimized().map_err(err)? {
        return Err("표시 중인 창만 이동할 수 있습니다".into());
    }
    let scale = f64::from(monitor.scale_factor().map_err(err)?);
    let desktop_unit = if cfg!(windows) { 1.0 } else { scale };
    let width = f64::from(monitor.width().map_err(err)?);
    let height = f64::from(monitor.height().map_err(err)?);
    let size = if let Some((w, h)) = extent {
        if crate::sizing::expanded_size(&window).is_some()
            || crate::sizing::image_size(&window).is_some()
        {
            return Err("접힌 창과 이미지 메모에는 크기를 유지하는 move를 사용하세요".into());
        }
        let logical = tauri::LogicalSize::new(
            f64::from(w) * width * desktop_unit / (1000.0 * scale),
            f64::from(h) * height * desktop_unit / (1000.0 * scale),
        );
        let minimum = if window.label().starts_with("webview-") {
            (240.0, 160.0)
        } else if window.label().starts_with("terminal-") {
            (2.0, 2.0)
        } else {
            (144.0, 32.0)
        };
        if logical.width < minimum.0 || logical.height < minimum.1 {
            return Err("대상 영역이 창의 최소 크기보다 작습니다".into());
        }
        logical.to_physical::<u32>(scale)
    } else {
        window
            .outer_size()
            .map_err(err)?
            .to_logical::<f64>(window.scale_factor().map_err(err)?)
            .to_physical::<u32>(scale)
    };
    let offset_x = f64::from(x) * width / 1000.0;
    let offset_y = f64::from(y) * height / 1000.0;
    if offset_x * desktop_unit + f64::from(size.width) > width * desktop_unit + 1.0
        || offset_y * desktop_unit + f64::from(size.height) > height * desktop_unit + 1.0
    {
        return Err("대상 위치에서 창이 모니터를 벗어납니다".into());
    }
    let position = tauri::PhysicalPosition::new(
        ((f64::from(monitor.x().map_err(err)?) + offset_x) * desktop_unit).round() as i32,
        ((f64::from(monitor.y().map_err(err)?) + offset_y) * desktop_unit).round() as i32,
    );
    crate::sizing::cancel(&window);
    crate::entrance::transition(
        &window,
        crate::entrance::Destination {
            position,
            size,
            scale,
        },
    )?;
    if automatic {
        remember_frame(&window, position, size);
    } else {
        protect(&window);
    }
    Ok(
        json!({"ok":true,"id":id,"display":monitor.id().map_err(err)?,"destination":position,"size":size,"animating":true,"instance":crate::instance::current().id}),
    )
}

fn snapshots(
    app: &tauri::AppHandle,
    monitors: &[Monitor],
    token: &str,
    only: Option<u32>,
) -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    if !capture::permission_status().0 {
        return Err("화면 기록 권한이 필요합니다".into());
    }
    let state = app.state::<Sharing>();
    let guard = state.0.lock().map_err(err)?;
    let share = guard
        .as_ref()
        .filter(|s| s.token() == token && only.is_none_or(|id| s.displays.contains(&id)))
        .ok_or("화면 공유가 종료되었거나 대상이 다릅니다")?;
    let batch = tempfile::Builder::new()
        .prefix("masked-")
        .tempdir_in(share.files.path())
        .map_err(err)?;
    let mut results = Vec::new();
    let notes = crate::web::managed_windows(app)
        .into_iter()
        .filter_map(|(label, window)| {
            let id = crate::memo_id_from_window(&label)
                .or_else(|| label.strip_prefix("webview-"))
                .or_else(|| crate::terminal::valid_id(&label).then_some(label.as_str()))?;
            if !window.is_visible().ok()? || window.is_minimized().ok()? {
                return None;
            }
            let (bounds, physical) = crate::session_restore::logical_geometry(&window).ok()?;
            let scale = if physical {
                window.scale_factor().ok()?
            } else {
                1.0
            };
            Some((
                id.to_owned(),
                placement_state(&window),
                ImageNote {
                    width: bounds.width * scale,
                    height: bounds.height * scale,
                    ..bounds
                },
            ))
        })
        .collect::<Vec<_>>();
    for id in only.map_or_else(|| share.displays.clone(), |id| vec![id]) {
        let monitor = monitors
            .iter()
            .find(|m| m.id().ok() == Some(id))
            .ok_or("공유 모니터 연결이 해제되었습니다")?;
        let bounds = ImageNote {
            x: f64::from(monitor.x().map_err(err)?),
            y: f64::from(monitor.y().map_err(err)?),
            width: f64::from(monitor.width().map_err(err)?),
            height: f64::from(monitor.height().map_err(err)?),
        };
        let scale = f64::from(monitor.scale_factor().map_err(err)?);
        let captured = monitor.capture_image().map_err(err)?;
        let (capture_width, capture_height) = captured.dimensions();
        let reduction = (1200.0 / f64::from(capture_width.max(capture_height))).min(1.0);
        let preview = if reduction < 1.0 {
            let width = (f64::from(capture_width) * reduction).round().max(1.0) as u32;
            let height = (f64::from(capture_height) * reduction).round().max(1.0) as u32;
            xcap::image::RgbaImage::from_fn(width, height, |x, y| {
                *captured.get_pixel(x * capture_width / width, y * capture_height / height)
            })
        } else {
            captured
        };
        let image = mask::redact(preview, scale * reduction);
        let path = batch.path().join(format!("{id}.png"));
        std::fs::File::create(&path)
            .and_then(|mut file| {
                file.write_all(&capture::png(&image).map_err(std::io::Error::other)?)
            })
            .map_err(err)?;
        let windows = notes
            .iter()
            .filter_map(|(id, (source, changed, allowed), note)| clip(note, &bounds).map(|rect| json!({"id":id,"rect":rect,"placement_source":source,"placement_protected":changed,"auto_move_allowed":allowed})))
            .collect::<Vec<_>>();
        results.push(json!({"path":path,"display":id,"width":image.width(),"height":image.height(),"masked":true,"capture_width":capture_width,"capture_height":capture_height,"coordinate_space":{"origin":{"x":bounds.x,"y":bounds.y},"desktop_size":{"width":bounds.width,"height":bounds.height},"desktop_units":if cfg!(windows){"physical"}else{"logical"},"scale_factor":scale,"pixel_scale_x":f64::from(image.width())/bounds.width,"pixel_scale_y":f64::from(image.height())/bounds.height,"placement_units_per_axis":1000},"windows":windows}));
    }
    let _ = batch.keep();
    if only.is_some() {
        let mut result = results.remove(0);
        result["ok"] = json!(true);
        return Ok(result);
    }
    Ok(json!({"ok":true,"displays":results}))
}
fn clip(note: &ImageNote, screen: &ImageNote) -> Option<ImageNote> {
    let left = note.x.max(screen.x);
    let top = note.y.max(screen.y);
    let right = (note.x + note.width).min(screen.x + screen.width);
    let bottom = (note.y + note.height).min(screen.y + screen.height);
    (right > left && bottom > top).then(|| ImageNote {
        x: (left - screen.x) * 1000.0 / screen.width,
        y: (top - screen.y) * 1000.0 / screen.height,
        width: (right - left) * 1000.0 / screen.width,
        height: (bottom - top) * 1000.0 / screen.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fit_validates_bounds_and_automatic_flag() {
        let parse_fit = |args: &str| {
            parse(
                &args
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        };
        assert!(matches!(
            parse_fit("fit memo 2 600,400,400,600 --automatic"),
            Ok(Operation::Fit(_, 2, _, true))
        ));
        assert!(matches!(
            parse_fit("fit memo 2 0,0,500,500"),
            Ok(Operation::Fit(_, 2, _, false))
        ));
        for args in [
            "fit memo 2 600,400,401,600",
            "fit memo 2 0,0,0,200",
            "fit memo 2 0,0",
            "fit memo 2 0,0,500,500 --auto",
        ] {
            assert!(parse_fit(args).is_err(), "{args}");
        }
    }
    #[test]
    fn clips_a_window_to_each_display_in_placement_units() {
        let left = ImageNote {
            x: -1500.0,
            y: 0.0,
            width: 1500.0,
            height: 1000.0,
        };
        let right = ImageNote {
            x: 0.0,
            y: 0.0,
            width: 2000.0,
            height: 1000.0,
        };
        let note = ImageNote {
            x: -150.0,
            y: 100.0,
            width: 350.0,
            height: 200.0,
        };
        let first = clip(&note, &left).unwrap();
        let second = clip(&note, &right).unwrap();
        assert_eq!(
            (first.x, first.width, second.x, second.width),
            (900.0, 100.0, 0.0, 100.0)
        );
        assert_eq!((first.y, first.height), (100.0, 200.0));
    }
}
