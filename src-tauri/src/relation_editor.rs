//! Trusted local toolbar connection gesture. Remote webpages cannot invoke commands.
use crate::relation_prototype::{self as relations, Arrow, Rect};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager, WebviewWindow};

#[derive(Default)]
pub(crate) struct Editor(Mutex<Option<Draft>>);
struct Draft {
    generation: u64,
    from: String,
    to: Option<String>,
    pickers: Vec<String>,
}
static GENERATION: AtomicU64 = AtomicU64::new(1);
fn id(label: &str) -> Option<&str> {
    if label == "main" || label.starts_with("terminal-") {
        Some(label)
    } else {
        label
            .strip_prefix("note-")
            .or_else(|| label.strip_prefix("webview-"))
    }
}
fn caller(view: &tauri::Webview) -> Result<String, String> {
    let id = id(view.label()).ok_or("메모 툴바에서만 연결할 수 있습니다")?;
    let host =
        relations::endpoint_window(view.app_handle(), id).ok_or("메모를 찾을 수 없습니다")?;
    if host.label() != view.window().label() {
        return Err("메모 툴바가 아닙니다".into());
    }
    Ok(id.into())
}
fn contains(r: Rect, p: [f64; 2]) -> bool {
    p[0] >= r.x && p[0] < r.x + r.w && p[1] >= r.y && p[1] < r.y + r.h
}
// Ambiguous overlap is deliberately not guessed: never attach to a hidden memo.
fn unique_target<'a>(from: &str, p: [f64; 2], candidates: &'a [(String, Rect)]) -> Option<&'a str> {
    let mut hits = candidates.iter().filter(|(_, r)| contains(*r, p));
    let (target, _) = hits.next()?;
    (hits.next().is_none() && target != from).then_some(target.as_str())
}
pub(crate) fn cancel(app: &tauri::AppHandle, restore: bool) {
    let Some(state) = app.try_state::<Editor>() else {
        return;
    };
    let draft = state.0.lock().ok().and_then(|mut s| s.take());
    if let Some(draft) = draft {
        for label in draft
            .pickers
            .into_iter()
            .chain(std::iter::once("relation-settings".into()))
        {
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.destroy();
            }
        }
        if restore {
            if let Some(w) = relations::endpoint_window(app, &draft.from) {
                if relations::rect(&w).is_some() {
                    let _ = w.set_focus();
                }
            }
        }
    }
}
#[tauri::command]
pub fn relation_status(view: tauri::Webview) -> bool {
    relations::enabled(view.app_handle()) && caller(&view).is_ok()
}
#[tauri::command]
pub fn relation_begin(view: tauri::Webview) -> Result<u64, String> {
    let from = caller(&view)?;
    let app = view.app_handle();
    if !relations::enabled(app) {
        return Err("이 앱에서 연결 기능이 활성화되지 않았습니다".into());
    }
    if crate::stash::active(app) {
        return Err("전체 숨김을 복원한 뒤 연결해 주세요".into());
    }
    let source = relations::endpoint_window(app, &from).ok_or("메모를 찾을 수 없습니다")?;
    if relations::rect(&source).is_none() {
        return Err("표시된 메모에서 연결해 주세요".into());
    }
    cancel(app, false);
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
    *app.state::<Editor>().0.lock().map_err(|e| e.to_string())? = Some(Draft {
        generation,
        from,
        to: None,
        pickers: vec![],
    });
    let result = (|| {
        let monitors = source.available_monitors().map_err(|e| e.to_string())?;
        if monitors.is_empty() {
            return Err("사용 가능한 화면이 없습니다".into());
        }
        for (i, monitor) in monitors.iter().enumerate() {
            let label = format!("relation-pick-{generation}-{i}");
            app.state::<Editor>()
                .0
                .lock()
                .map_err(|e| e.to_string())?
                .as_mut()
                .ok_or("연결이 취소됐습니다")?
                .pickers
                .push(label.clone());
            let w = tauri::WebviewWindowBuilder::new(
                app,
                &label,
                tauri::WebviewUrl::App("relation-pick.html".into()),
            )
            .title(format!("{} — {label}", crate::instance::current().title()))
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .focusable(false)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;
            w.set_size(*monitor.size()).map_err(|e| e.to_string())?;
            w.set_position(*monitor.position())
                .map_err(|e| e.to_string())?;
            crate::show_without_focus(&w)?;
        }
        Ok(())
    })();
    if result.is_err() {
        cancel(app, true);
    }
    result.map(|()| generation)
}
fn picker(window: &WebviewWindow) -> Result<(u64, String), String> {
    let state = window
        .app_handle()
        .try_state::<Editor>()
        .ok_or("연결이 활성화되지 않았습니다")?;
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let d = guard.as_ref().ok_or("연결이 취소됐습니다")?;
    if !d.pickers.iter().any(|s| s == window.label()) || d.to.is_some() {
        return Err("이전 연결 요청입니다".into());
    }
    Ok((d.generation, d.from.clone()))
}
#[tauri::command]
pub fn relation_picker_info(window: WebviewWindow) -> Result<Value, String> {
    picker(&window)?;
    Ok(json!({"ready":true}))
}
#[tauri::command]
pub fn relation_pick(window: WebviewWindow, x: f64, y: f64, choose: bool) -> Result<bool, String> {
    let (generation, from) = picker(&window)?;
    if !x.is_finite() || !y.is_finite() {
        return Err("유효하지 않은 포인터 위치입니다".into());
    }
    let app = window.app_handle();
    let origin = window.inner_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let p = [origin.x as f64 + x * scale, origin.y as f64 + y * scale];
    let source = relations::endpoint_window(app, &from)
        .and_then(|w| relations::rect(&w))
        .ok_or("출발 메모가 숨겨지거나 닫혔습니다")?;
    let candidates: Vec<_> = crate::web::managed_windows(app)
        .values()
        .filter_map(|w| {
            let id = id(w.label())?;
            let host = relations::endpoint_window(app, id)?;
            if host.label() != w.label() {
                return None;
            }
            Some((id.to_owned(), relations::rect(&host)?))
        })
        .collect();
    let target = unique_target(&from, p, &candidates);
    if choose {
        let Some(to) = target else { return Ok(false) };
        let state = app.state::<Editor>();
        {
            let mut guard = state.0.lock().map_err(|e| e.to_string())?;
            let draft = guard
                .as_mut()
                .filter(|d| d.generation == generation && d.to.is_none())
                .ok_or("이전 연결 요청입니다")?;
            draft.to = Some(to.into());
        }
        let panel = tauri::WebviewWindowBuilder::new(
            app,
            "relation-settings",
            tauri::WebviewUrl::App("relation-settings.html".into()),
        )
        .title("메모 연결 설정")
        .inner_size(380., 330.)
        .resizable(false)
        .minimizable(false)
        .always_on_top(true)
        .visible(false)
        .focused(false)
        .center()
        .build();
        let panel = match panel {
            Ok(w) => w,
            Err(e) => {
                cancel(app, true);
                return Err(e.to_string());
            }
        };
        let labels = {
            let mut guard = state.0.lock().map_err(|e| e.to_string())?;
            std::mem::take(&mut guard.as_mut().ok_or("연결이 취소됐습니다")?.pickers)
        };
        for label in labels {
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.destroy();
            }
        }
        panel.show().map_err(|e| e.to_string())?;
        panel.set_focus().map_err(|e| e.to_string())?;
        return Ok(true);
    }
    let destination =
        target.and_then(|id| candidates.iter().find(|(s, _)| s == id).map(|(_, r)| *r));
    let points = relations::curve(
        source,
        destination.unwrap_or(Rect {
            x: p[0],
            y: p[1],
            w: 0.,
            h: 0.,
        }),
    );
    let labels = app
        .state::<Editor>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .filter(|d| d.generation == generation)
        .map(|d| d.pickers.clone())
        .unwrap_or_default();
    for label in labels {
        if let Some(w) = app.get_webview_window(&label) {
            let o = w.inner_position().map_err(|e| e.to_string())?;
            let s = w.inner_size().map_err(|e| e.to_string())?;
            let local = points.map(|p| p.map(|v| [v[0] - o.x as f64, v[1] - o.y as f64]));
            let highlight = destination
                .map(|r| json!({"x":r.x-o.x as f64,"y":r.y-o.y as f64,"width":r.w,"height":r.h}));
            w.emit("relation-preview", json!({"width":s.width,"height":s.height,"points":local,"target":target,"highlight":highlight,"scale":w.scale_factor().map_err(|e| e.to_string())?})).map_err(|e| e.to_string())?;
        }
    }
    Ok(target.is_some())
}
fn pair(window: &WebviewWindow) -> Result<(u64, String, String), String> {
    if window.label() != "relation-settings" {
        return Err("연결 설정창 전용입니다".into());
    }
    let state = window
        .app_handle()
        .try_state::<Editor>()
        .ok_or("연결이 활성화되지 않았습니다")?;
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let draft = guard.as_ref().ok_or("연결이 취소됐습니다")?;
    Ok((
        draft.generation,
        draft.from.clone(),
        draft.to.clone().ok_or("대상 메모가 없습니다")?,
    ))
}
fn title(app: &tauri::AppHandle, id: &str) -> String {
    if let Some(note) = app
        .state::<crate::AppNotes>()
        .lock()
        .ok()
        .and_then(|notes| notes.get(id).cloned())
    {
        return if note.title.trim().is_empty() {
            "메모".into()
        } else {
            note.title
        };
    }
    if let Some(terminal) = crate::terminal::entries(app)
        .into_iter()
        .find(|t| t.id == id)
    {
        return terminal.display_title();
    }
    if let Ok(web) = crate::web::inspect(app, id) {
        if let Some(host) = web["url"]
            .as_str()
            .and_then(|s| tauri::Url::parse(s).ok())
            .and_then(|u| u.host_str().map(str::to_owned))
        {
            return format!("웹 · {host}");
        }
    }
    "웹 메모".into()
}
#[tauri::command]
pub fn relation_editor_state(window: WebviewWindow) -> Result<Value, String> {
    let (generation, from, to) = pair(&window)?;
    Ok(
        json!({"generation":generation,"fromTitle":title(window.app_handle(),&from),"toTitle":title(window.app_handle(),&to)}),
    )
}
#[tauri::command]
pub fn relation_save(
    window: WebviewWindow,
    label: String,
    arrow: Arrow,
    generation: u64,
) -> Result<(), String> {
    let (current, from, to) = pair(&window)?;
    if current != generation {
        return Err("이전 연결 요청입니다".into());
    }
    relations::save_connection(
        window.app_handle(),
        from,
        to.clone(),
        label.trim().into(),
        arrow,
    )?;
    let app = window.app_handle().clone();
    cancel(&app, false);
    if let Some(w) = relations::endpoint_window(&app, &to) {
        if relations::rect(&w).is_some() {
            let _ = w.set_focus();
        }
    }
    Ok(())
}
#[tauri::command]
pub fn relation_cancel(view: tauri::Webview, generation: Option<u64>) -> Result<(), String> {
    let app = view.app_handle();
    let state = app
        .try_state::<Editor>()
        .ok_or("연결이 활성화되지 않았습니다")?;
    let allowed = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        let d = guard.as_ref().ok_or("진행 중인 연결이 없습니다")?;
        d.pickers.iter().any(|s| s == view.label())
            || (generation == Some(d.generation)
                && (view.label() == "relation-settings"
                    || caller(&view).is_ok_and(|id| id == d.from)))
    };
    if !allowed {
        return Err("현재 연결의 창이 아닙니다".into());
    }
    cancel(app, true);
    Ok(())
}
pub(crate) fn sweep(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<Editor>() else {
        return;
    };
    let ids = state
        .0
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| (d.from.clone(), d.to.clone())));
    if let Some((from, to)) = ids {
        if crate::stash::active(app)
            || [Some(from), to].into_iter().flatten().any(|id| {
                relations::endpoint_window(app, &id)
                    .and_then(|w| relations::rect(&w))
                    .is_none()
            })
        {
            cancel(app, false);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_requires_one_visible_owner_and_never_the_source() {
        let r = Rect {
            x: 0.,
            y: 0.,
            w: 100.,
            h: 100.,
        };
        let a = vec![("a".into(), r)];
        assert_eq!(unique_target("a", [50., 50.], &a), None);
        assert_eq!(unique_target("b", [50., 50.], &a), Some("a"));
        assert_eq!(unique_target("b", [100., 50.], &a), None);
        let ambiguous = vec![("a".into(), r), ("c".into(), r)];
        assert_eq!(unique_target("b", [50., 50.], &ambiguous), None);
    }
}
