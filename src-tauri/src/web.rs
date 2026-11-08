//! Session-only web windows. Trusted Welcome and remote pages use disjoint labels.
use crate::{
    capture::ImageNote,
    model::{Action, Request},
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindow};
#[derive(Clone)]
struct Record {
    label: String,
    url: Option<String>,
}
#[derive(Default)]
pub struct Windows(Mutex<BTreeMap<String, Record>>);
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn toolbar_label(id: &str) -> String {
    format!("webtoolbar-{id}")
}
fn web_id(label: &str) -> Option<&str> {
    label
        .strip_prefix("webview-")
        .or_else(|| label.strip_prefix("webpage-"))
        .or_else(|| label.strip_prefix("webtoolbar-"))
}
pub fn validate_url(input: &str) -> Result<tauri::Url, String> {
    let invalid = || "URL은 자격증명 없는 HTTP(S) 웹 주소여야 합니다".to_string();
    if input.is_empty()
        || input.len() > 8192
        || input.chars().any(|c| c.is_control() || c.is_whitespace())
        || input.contains('\\')
    {
        return Err(invalid());
    }
    let url = tauri::Url::parse(input).map_err(|_| invalid())?;
    let host = url
        .host_str()
        .ok_or_else(invalid)?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || ["tauri.localhost", "ipc.localhost", "asset.localhost"]
            .iter()
            .any(|reserved| host == *reserved || host.ends_with(&format!(".{reserved}")))
    {
        return Err(invalid());
    }
    Ok(url)
}
pub fn contains(app: &tauri::AppHandle, id: &str) -> bool {
    app.state::<Windows>().0.lock().unwrap().contains_key(id)
}
pub fn url_for_restore(app: &tauri::AppHandle, label: &str) -> Option<String> {
    let id = web_id(label)?;
    let record = app.state::<Windows>().0.lock().ok()?.get(id)?.clone();
    record.url.as_ref()?;
    let current = app
        .get_webview_window(&record.label)?
        .url()
        .ok()?
        .to_string();
    validate_url(&current).ok().map(|url| url.to_string())
}
pub fn contains_label(app: &tauri::AppHandle, label: &str) -> bool {
    web_id(label).is_some_and(|id| contains(app, id))
}
fn record(app: &tauri::AppHandle, id: &str) -> Result<Record, String> {
    app.state::<Windows>()
        .0
        .lock()
        .map_err(err)?
        .get(id)
        .cloned()
        .ok_or("웹 창을 찾을 수 없습니다".into())
}
pub fn inspect(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    let r = record(app, id)?;
    let w = app
        .get_webview_window(&r.label)
        .ok_or("웹 창을 찾을 수 없습니다")?;
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":"web","title":w.title().map_err(err)?,"url":r.url,"welcome":r.url.is_none(),"current_url":w.url().map_err(err)?.as_str(),"visible":w.is_visible().map_err(err)?,"focused":w.is_focused().map_err(err)?,"always_on_top":w.is_always_on_top().map_err(err)?,"position":w.outer_position().map_err(err)?,"size":w.inner_size().map_err(err)?}),
    )
}
pub fn entries(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    let ids: Vec<_> = app
        .state::<Windows>()
        .0
        .lock()
        .map_err(err)?
        .keys()
        .cloned()
        .collect();
    ids.iter().map(|id| inspect(app, id)).collect()
}
fn show(w: &WebviewWindow) -> Result<(), String> {
    crate::stash::release(w)?;
    #[cfg(target_os = "macos")]
    unsafe {
        (&*(w.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
    }
    #[cfg(windows)]
    crate::focus::show_without_focus(w)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    w.show().map_err(err)?;
    #[cfg(any(target_os = "macos", windows))]
    crate::focus::restore(w);
    if let Some(id) = web_id(w.label()) {
        if let Some(toolbar) = w.app_handle().get_webview_window(&toolbar_label(id)) {
            sync_toolbar(w, &toolbar)?;
        }
    }
    Ok(())
}

fn sync_toolbar(parent: &WebviewWindow, toolbar: &WebviewWindow) -> Result<(), String> {
    if !parent.is_visible().map_err(err)? || parent.is_minimized().map_err(err)? {
        if toolbar.is_visible().map_err(err)? {
            toolbar.hide().map_err(err)?;
        }
        return Ok(());
    }
    let parent_position = parent.inner_position().map_err(err)?;
    let parent_size = parent.inner_size().map_err(err)?;
    let toolbar_size = toolbar.outer_size().map_err(err)?;
    let position = tauri::PhysicalPosition::new(
        parent_position.x + parent_size.width as i32 - toolbar_size.width as i32,
        parent_position.y,
    );
    if toolbar.outer_position().map_err(err)? != position {
        toolbar.set_position(position).map_err(err)?;
    }
    let pinned = parent.is_always_on_top().map_err(err)?;
    if toolbar.is_always_on_top().map_err(err)? != pinned {
        toolbar.set_always_on_top(pinned).map_err(err)?;
    }
    if !toolbar.is_visible().map_err(err)? {
        #[cfg(target_os = "macos")]
        {
            let ptr = toolbar.ns_window().map_err(err)?;
            // SAFETY: Tauri owns the live companion window.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
            }
            crate::focus::restore(toolbar);
        }
        #[cfg(windows)]
        crate::focus::show_without_focus(toolbar)?;
        #[cfg(not(any(target_os = "macos", windows)))]
        toolbar.show().map_err(err)?;
    }
    Ok(())
}

fn ensure_toolbar(app: &tauri::AppHandle, id: &str, parent: &WebviewWindow) -> Result<(), String> {
    let label = toolbar_label(id);
    if let Some(toolbar) = app.get_webview_window(&label) {
        return sync_toolbar(parent, &toolbar);
    }
    crate::focus::remember(app, &label);
    let toolbar =
        tauri::WebviewWindowBuilder::new(app, &label, WebviewUrl::App("web-toolbar.html".into()))
            .title(format!("웹 도구 — {id}"))
            .inner_size(54.0, 27.0)
            .decorations(false)
            .transparent(true)
            .resizable(false)
            .visible(false)
            .focused(false)
            .skip_taskbar(true)
            .accept_first_mouse(true)
            .minimizable(false)
            .maximizable(false)
            .shadow(false)
            .parent(parent)
            .map_err(err)?
            .build()
            .map_err(err)?;
    sync_toolbar(parent, &toolbar)
}

fn destroy_toolbar(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    if let Some(toolbar) = app.get_webview_window(&toolbar_label(id)) {
        toolbar.destroy().map_err(err)?;
    }
    Ok(())
}

pub fn geometry(window: &WebviewWindow) {
    if let Some(id) = web_id(window.label()) {
        if let Some(toolbar) = window.app_handle().get_webview_window(&toolbar_label(id)) {
            if let Err(error) = sync_toolbar(window, &toolbar) {
                eprintln!("웹 도구 모음 배치 실패: {error}");
            }
        }
    }
}
pub fn create(
    app: &tauri::AppHandle,
    bounds: Option<ImageNote>,
    id: Option<&str>,
    url: Option<&str>,
) -> Result<Value, String> {
    create_with_visibility(app, bounds, id, url, true)
}

pub fn restore_session(app: &tauri::AppHandle, id: &str, url: Option<&str>) -> Result<(), String> {
    create_with_visibility(app, None, Some(id), url, false).map(|_| ())
}

fn create_with_visibility(
    app: &tauri::AppHandle,
    bounds: Option<ImageNote>,
    id: Option<&str>,
    url: Option<&str>,
    visible: bool,
) -> Result<Value, String> {
    let id = id.map(str::to_owned).unwrap_or_else(|| {
        format!(
            "web-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        )
    });
    let parsed = url.map(validate_url).transpose()?;
    let remote = parsed.is_some();
    let label = format!("{}-{id}", if remote { "webpage" } else { "webview" });
    let old = if contains(app, &id) {
        Some(record(app, &id)?)
    } else {
        None
    };
    if let Some(r) = &old {
        if url.is_none() || r.url.as_deref() == url {
            let w = app
                .get_webview_window(&r.label)
                .ok_or("웹 창을 찾을 수 없습니다")?;
            if visible {
                show(&w)?;
            }
            ensure_toolbar(app, &id, &w)?;
            return inspect(app, &id);
        }
    }
    let previous = old.as_ref().and_then(|r| app.get_webview_window(&r.label));
    // A replacement is prepared before the current window is removed.
    // Repeated remote navigation uses the same isolated WebView.
    if let Some(w) = previous
        .as_ref()
        .filter(|_| old.as_ref().is_some_and(|r| r.url.is_some()) && remote)
    {
        w.navigate(parsed.unwrap()).map_err(err)?;
        app.state::<Windows>()
            .0
            .lock()
            .map_err(err)?
            .get_mut(&id)
            .unwrap()
            .url = url.map(str::to_owned);
        if visible {
            show(w)?;
        }
        ensure_toolbar(app, &id, w)?;
        return inspect(app, &id);
    }
    crate::focus::remember(app, &label);
    let target = parsed
        .map(WebviewUrl::External)
        .unwrap_or_else(|| WebviewUrl::App("web.html".into()));
    let mut builder = tauri::WebviewWindowBuilder::new(app, &label, target)
        .title(format!(
            "{} — 웹 — {id}",
            crate::instance::current().title()
        ))
        .inner_size(600.0, 400.0)
        .min_inner_size(240.0, 160.0)
        .always_on_top(true)
        .visible(false)
        .focused(false)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_download(|_, _| false);
    if remote {
        builder = builder.on_navigation(|url| validate_url(url.as_str()).is_ok());
    } else {
        builder = builder.on_navigation(|url| {
            url.path() == "/web.html"
                && url.port().is_none()
                && url.query().is_none()
                && ((url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (matches!(url.scheme(), "http" | "https")
                        && url.host_str() == Some("tauri.localhost")))
        });
    }
    let w = builder.build().map_err(err)?;
    if let Some(p) = &previous {
        w.set_position(p.outer_position().map_err(err)?)
            .map_err(err)?;
        w.set_size(p.inner_size().map_err(err)?).map_err(err)?;
        w.set_always_on_top(p.is_always_on_top().map_err(err)?)
            .map_err(err)?;
    } else if let Some(b) = bounds {
        #[cfg(windows)]
        w.set_position(tauri::PhysicalPosition::new(
            b.x.round() as i32,
            b.y.round() as i32,
        ))
        .map_err(err)?;
        #[cfg(not(windows))]
        w.set_position(tauri::LogicalPosition::new(b.x, b.y))
            .map_err(err)?;
        w.set_size(tauri::LogicalSize::new(
            b.width.max(240.0),
            b.height.max(160.0),
        ))
        .map_err(err)?;
    }
    app.state::<Windows>().0.lock().map_err(err)?.insert(
        id.clone(),
        Record {
            label,
            url: url.map(str::to_owned),
        },
    );
    if visible {
        show(&w)?;
    }
    if previous.is_some() {
        destroy_toolbar(app, &id)?;
        if let Some(p) = previous {
            p.destroy().map_err(err)?;
        }
    }
    ensure_toolbar(app, &id, &w)?;
    if !visible {
        // WebView construction can activate the app on macOS. Restoring an
        // update snapshot must not steal the user's foreground application.
        crate::focus::restore(&w);
    }
    inspect(app, &id)
}

pub fn close(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    let r = record(app, id)?;
    let window = app
        .get_webview_window(&r.label)
        .ok_or("웹 창을 찾을 수 없습니다")?;
    crate::stash::release(&window)?;
    destroy_toolbar(app, id)?;
    window.destroy().map_err(err)?;
    app.state::<Windows>().0.lock().map_err(err)?.remove(id);
    crate::tray::refresh(app)?;
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":"web","visible":false,"closed":true}),
    )
}

pub fn close_window(app: &tauri::AppHandle, label: &str) -> Result<Value, String> {
    let id = web_id(label).ok_or("웹 창을 찾을 수 없습니다")?;
    close(app, id)
}

#[tauri::command]
pub fn web_toolbar_info(window: WebviewWindow) -> Result<Value, String> {
    let id = window
        .label()
        .strip_prefix("webtoolbar-")
        .ok_or("웹 도구 창이 아닙니다")?;
    let record = record(window.app_handle(), id)?;
    let owner = window
        .app_handle()
        .get_webview_window(&record.label)
        .ok_or("웹 창을 찾을 수 없습니다")?;
    Ok(json!({"id":id,"always_on_top":owner.is_always_on_top().map_err(err)?}))
}

#[tauri::command]
pub fn web_toggle_pin(window: WebviewWindow) -> Result<bool, String> {
    let id = window
        .label()
        .strip_prefix("webtoolbar-")
        .ok_or("웹 도구 창이 아닙니다")?;
    let record = record(window.app_handle(), id)?;
    let owner = window
        .app_handle()
        .get_webview_window(&record.label)
        .ok_or("웹 창을 찾을 수 없습니다")?;
    let pinned = !owner.is_always_on_top().map_err(err)?;
    owner.set_always_on_top(pinned).map_err(err)?;
    window.set_always_on_top(pinned).map_err(err)?;
    Ok(pinned)
}

#[tauri::command]
pub fn web_close(window: WebviewWindow) -> Result<Value, String> {
    let id = window
        .label()
        .strip_prefix("webtoolbar-")
        .ok_or("웹 도구 창이 아닙니다")?;
    close(window.app_handle(), id)
}
pub fn handle(app: &tauri::AppHandle, request: &Request) -> Result<Value, String> {
    if matches!(request.action, Action::Web)
        && request.id.as_ref().is_some_and(|id| {
            app.state::<crate::AppNotes>()
                .lock()
                .unwrap()
                .contains_key(id)
        })
    {
        return Err("같은 ID의 메모가 이미 있습니다. 다른 웹 창 ID를 사용하세요".into());
    }
    if matches!(request.action, Action::Web) {
        return create(app, None, request.id.as_deref(), request.url.as_deref());
    }
    let r = record(app, request.id())?;
    let w = app
        .get_webview_window(&r.label)
        .ok_or("웹 창을 찾을 수 없습니다")?;
    match request.action {
        Action::Inspect => inspect(app, request.id()),
        Action::Close => close(app, request.id()),
        Action::Open => {
            if request.title.is_some()
                || request.markdown.is_some()
                || request.color.is_some()
                || request.status.is_some()
            {
                return Err("웹 창의 내용은 web 명령으로 변경하세요".into());
            }
            show(&w)?;
            inspect(app, request.id())
        }
        _ => Err("웹 창에는 web/open/inspect/close를 사용하세요".into()),
    }
}
#[tauri::command]
pub async fn web_run(window: WebviewWindow, url: String) -> Result<Value, String> {
    let id = window
        .label()
        .strip_prefix("webview-")
        .ok_or("웹 Welcome에서만 실행할 수 있습니다")?;
    let r = record(window.app_handle(), id)?;
    if r.label != window.label() || r.url.is_some() {
        return Err("웹 Welcome에서만 실행할 수 있습니다".into());
    }
    let id = id.to_owned();
    let app = window.app_handle().clone();
    // Native window construction/order/focus must run on the main thread.
    // The asynchronous command avoids WebView2's synchronous IPC deadlock.
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(create(&handle, None, Some(&id), Some(&url)));
        })
        .map_err(err)?;
        rx.recv().map_err(err)?
    })
    .await
    .map_err(err)?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companion_toolbar_label_is_separate_from_remote_content() {
        assert_eq!(toolbar_label("web-1"), "webtoolbar-web-1");
        assert_eq!(web_id("webpage-web-1"), Some("web-1"));
        assert_eq!(web_id("webview-web-1"), Some("web-1"));
        assert_eq!(web_id("webtoolbar-web-1"), Some("web-1"));
        assert_eq!(web_id("note-web-1"), None);
    }

    #[test]
    fn navigation_validation_rejects_internal_and_credential_urls() {
        for url in [
            "https://tauri.localhost/",
            "http://IPC.localhost./",
            "https://x.asset.localhost/",
            "tauri://localhost/web.html",
            "file:///tmp/a",
            "javascript:alert(1)",
            "https://user:p@example.com/",
        ] {
            assert!(validate_url(url).is_err(), "{url}");
        }
        for url in [
            "https://example.com/",
            "http://localhost:3000/",
            "http://127.0.0.1/",
        ] {
            assert!(validate_url(url).is_ok(), "{url}");
        }
    }
}
