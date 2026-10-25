use crate::placement::Rect;
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Manager, State};

#[derive(Default)]
pub struct Warp {
    initial_y: f64,
    contact: Contact,
}
pub type Warps = Mutex<BTreeMap<String, Warp>>;

// A move must not create another contact by itself, even if a large note
// still covers the pointer. Re-arm after release or a real exit of the pointer.
#[derive(Default)]
struct Contact {
    blocked_at: Option<(f64, f64)>,
}
impl Contact {
    fn trigger(&mut self, alt: bool, inside: bool, cursor: (f64, f64)) -> bool {
        if !alt || (!inside && self.blocked_at != Some(cursor)) {
            self.blocked_at = None;
        }
        if alt && inside && self.blocked_at.is_none() {
            self.blocked_at = Some(cursor);
            true
        } else {
            false
        }
    }
}

pub fn remember(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    y: i64,
    work_y: i32,
    scale: f64,
) -> Result<(), String> {
    app.state::<Warps>()
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            window.label().into(),
            Warp {
                initial_y: (y as f64 - f64::from(work_y)) / scale,
                ..Warp::default()
            },
        );
    Ok(())
}

fn destination(area: Rect, current: Rect, initial_y: i64, gap: i64) -> Result<(i32, i32), String> {
    if current.width > area.width - 2 * gap || current.height > area.height - 2 * gap {
        return Err("노트가 화면보다 큽니다. 크기를 줄인 후 워프하세요".into());
    }
    let on_left = current.x * 2 + current.width < area.x * 2 + area.width;
    let x = if on_left {
        area.x + area.width - current.width - gap
    } else {
        area.x + gap
    };
    let y = (area.y + initial_y).clamp(area.y + gap, area.y + area.height - current.height - gap);
    Ok((x as i32, y as i32))
}

fn move_note(window: &tauri::WebviewWindow, warp: &mut Warp) -> Result<(), String> {
    crate::entrance::finish(window)?;
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("노트가 있는 모니터를 찾을 수 없습니다")?;
    let area = monitor.work_area();
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let s = window.outer_size().map_err(|e| e.to_string())?;
    let (x, y) = destination(
        Rect {
            x: area.position.x.into(),
            y: area.position.y.into(),
            width: area.size.width.into(),
            height: area.size.height.into(),
        },
        Rect {
            x: p.x.into(),
            y: p.y.into(),
            width: s.width.into(),
            height: s.height.into(),
        },
        (warp.initial_y * monitor.scale_factor()).round() as i64,
        (8.0 * monitor.scale_factor()).round() as i64,
    )?;
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn warp_note(window: tauri::WebviewWindow, state: State<Warps>) -> Result<(), String> {
    let mut warps = state.lock().map_err(|e| e.to_string())?;
    let warp = warps
        .get_mut(window.label())
        .ok_or("최초 위치가 아직 준비되지 않았습니다")?;
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    warp.contact.blocked_at = Some((cursor.x, cursor.y));
    move_note(&window, warp)
}

#[tauri::command]
pub fn poll_warp(window: tauri::WebviewWindow, state: State<Warps>) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    let alt = objc2_app_kit::NSEvent::modifierFlags_class()
        .contains(objc2_app_kit::NSEventModifierFlags::Option);
    #[cfg(not(target_os = "macos"))]
    let alt = false;
    let mut warps = state.lock().map_err(|e| e.to_string())?;
    let Some(warp) = warps.get_mut(window.label()) else {
        return Ok(false);
    };
    if !window.is_visible().map_err(|e| e.to_string())? {
        warp.contact = Contact::default();
        return Ok(false);
    }
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let s = window.outer_size().map_err(|e| e.to_string())?;
    let inside = cursor.x >= p.x as f64
        && cursor.y >= p.y as f64
        && cursor.x < p.x as f64 + s.width as f64
        && cursor.y < p.y as f64 + s.height as f64;
    // Native hit testing also works while the WebView is inactive.
    if inside {
        if let Some(revision) = crate::feedback::highlight_revision(&window) {
            crate::feedback::acknowledge_feedback(window.clone(), revision)?;
        }
    }
    if warp.contact.trigger(alt, inside, (cursor.x, cursor.y)) {
        move_note(&window, warp)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sides_initial_height_and_monitor_bounds() {
        let area = Rect {
            x: -1600,
            y: -100,
            width: 1600,
            height: 900,
        };
        let mut note = Rect {
            x: -400,
            y: 450,
            width: 300,
            height: 240,
        };
        assert_eq!(destination(area, note, 70, 8).unwrap(), (-1592, -30));
        note.x = -1592;
        assert_eq!(destination(area, note, 70, 8).unwrap(), (-308, -30));
        assert_eq!(destination(area, note, 9999, 8).unwrap().1, 552);
        assert_eq!(destination(area, note, -999, 8).unwrap().1, -92);
        note.width = 1600;
        assert!(destination(area, note, 70, 8).is_err());
    }
    #[test]
    fn contact_does_not_bounce_and_rearms_after_real_exit_or_release() {
        let mut c = Contact::default();
        assert!(!c.trigger(false, true, (10., 20.)));
        assert!(c.trigger(true, true, (10., 20.)));
        assert!(!c.trigger(true, false, (10., 20.)));
        assert!(!c.trigger(true, true, (11., 20.)));
        assert!(!c.trigger(true, false, (30., 20.)));
        assert!(c.trigger(true, true, (40., 20.)));
        assert!(!c.trigger(false, true, (40., 20.)));
        assert!(c.trigger(true, true, (40., 20.)));
    }
}
