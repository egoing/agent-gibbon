//! Visual hover for inactive WKWebViews, using the existing contact monitor.
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, Manager, WebviewWindow};

type Point = Option<(f64, f64)>;
#[derive(Default)]
pub struct Pointers(Mutex<BTreeMap<String, Point>>);

// AppKit screen coordinates and content rect are both logical points, even
// across monitors with different backing scales. DOM y runs down from the top.
fn local_point((cx, cy): (f64, f64), (x, y, w, h): (f64, f64, f64, f64)) -> Point {
    let point = (cx - x, y + h - cy);
    (point.0 >= 0.0 && point.1 >= 0.0 && point.0 < w && point.1 < h).then_some(point)
}

pub fn sync(window: &WebviewWindow) -> Result<(), String> {
    let point = if window.is_visible().map_err(|e| e.to_string())? {
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: the native contact/opacity callbacks run on the main thread;
        // Tauri owns this window throughout the synchronous borrow.
        let native = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
        let rect = native.contentRectForFrameRect(native.frame());
        let cursor = objc2_app_kit::NSEvent::mouseLocation();
        // A rectangle hit alone also matches covered memos. AppKit resolves
        // the actual mouse target across windows/apps without activating it.
        let mtm = objc2::MainThreadMarker::new().ok_or("toolbar pointer off main thread")?;
        let front = objc2_app_kit::NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
            cursor, 0, mtm,
        );
        (front == native.windowNumber())
            .then(|| {
                local_point(
                    (cursor.x, cursor.y),
                    (
                        rect.origin.x,
                        rect.origin.y,
                        rect.size.width,
                        rect.size.height,
                    ),
                )
            })
            .flatten()
    } else {
        None
    };
    let changed = window
        .state::<Pointers>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), point)
        != Some(point);
    if changed {
        // Address only this trusted toolbar; never broadcast pointer locations.
        window
            .emit_to(
                window.label(),
                "toolbar-pointer",
                point.map(|(x, y)| serde_json::json!({ "x": x, "y": y })),
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appkit_points_keep_fractional_positions_and_negative_screen_origins() {
        assert_eq!(
            local_point((-401.5, -212.25), (-800.0, -224.0, 400.0, 24.0)),
            Some((398.5, 12.25))
        );
        assert_eq!(
            local_point((0.0, 24.0), (0.0, 0.0, 400.0, 24.0)),
            Some((0.0, 0.0))
        );
        for cursor in [(-1.0, 12.0), (400.0, 12.0), (10.0, 0.0), (10.0, 25.0)] {
            assert_eq!(local_point(cursor, (0.0, 0.0, 400.0, 24.0)), None);
        }
    }
}
