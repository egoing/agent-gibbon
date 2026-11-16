//! Local-restorable web windows. Trusted Welcome and remote pages use disjoint labels.
use crate::{
    capture::ImageNote,
    model::{Action, Request},
};
use serde_json::{json, Value};
mod recent;
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow};
#[derive(Clone)]
struct Record {
    label: String,
    host: WebviewWindow,
    url: Option<String>,
    created_at: std::time::SystemTime,
}
#[derive(Default)]
pub struct Windows(
    Mutex<BTreeMap<String, Record>>,
    // Navigation callbacks can precede record insertion during native startup.
    Mutex<BTreeMap<String, String>>,
    Mutex<BTreeMap<String, String>>,
);
// Preserve app-wide window enumeration for hosts containing remote child WebViews.
pub fn managed_windows(app: &tauri::AppHandle) -> std::collections::HashMap<String, WebviewWindow> {
    let mut windows = app.webview_windows();
    if let Some(state) = app.try_state::<Windows>() {
        if let Ok(records) = state.0.lock() {
            windows.extend(records.values().map(|r| (r.label.clone(), r.host.clone())));
        }
    }
    windows
}
pub fn managed_window(app: &tauri::AppHandle, label: &str) -> Option<WebviewWindow> {
    app.get_webview_window(label).or_else(|| {
        app.try_state::<Windows>()?
            .0
            .lock()
            .ok()?
            .values()
            .find(|r| r.label == label)
            .map(|r| r.host.clone())
    })
}
pub(crate) fn created_at(app: &tauri::AppHandle, id: &str) -> Option<std::time::SystemTime> {
    app.state::<Windows>()
        .0
        .lock()
        .ok()?
        .get(id)
        .map(|r| r.created_at)
}
fn creation_time(id: &str) -> std::time::SystemTime {
    id.strip_prefix("web-")
        .and_then(|value| value.parse::<u64>().ok())
        .and_then(|value| std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_nanos(value)))
        .unwrap_or_else(std::time::SystemTime::now)
}

// A mobile Safari UA is applied before the first network request. It is a
// compatibility hint, not device emulation or a guarantee of mobile content.
const MOBILE_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
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
fn record(app: &tauri::AppHandle, id: &str) -> Result<Record, String> {
    app.state::<Windows>()
        .0
        .lock()
        .map_err(err)?
        .get(id)
        .cloned()
        .ok_or("웹 창을 찾을 수 없습니다".into())
}
// Wry 0.57 macOS url() unwraps WKWebView.URL even before a document exists.
// Read the public nullable property directly and report unknown as null. Do not
// confuse the last allowed navigation target with the committed/current URL.
fn current_webview_url(view: &tauri::Webview, app: &tauri::AppHandle, id: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        let id = id.to_owned();
        let observed = app.state::<Windows>().2.lock().ok()?.get(&id).cloned();
        view.with_webview(move |platform| unsafe {
            let native = &*platform.inner().cast::<objc2::runtime::AnyObject>();
            let url: Option<objc2::rc::Retained<objc2_foundation::NSURL>> =
                objc2::msg_send![native, URL];
            let value = url
                .and_then(|url| url.absoluteString())
                .map(|s| s.to_string());
            if let Ok(mut urls) = app.state::<Windows>().2.lock() {
                if let Some(value) = value {
                    urls.insert(id, value);
                } else {
                    urls.remove(&id);
                }
            }
        })
        .ok()?;
        // Native callbacks may be deferred. Return only an actually observed
        // URL, never wait on the UI thread or substitute a navigation target.
        observed
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, id);
        view.url().ok().map(|url| url.to_string())
    }
}
pub fn inspect(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
    let r = record(app, id)?;
    let w = r.host.clone();
    let content = app.get_webview(&format!("webpage-{id}"));
    let current_url = current_webview_url(content.as_ref().unwrap_or(w.as_ref()), app, id);
    let navigation_url = app
        .state::<Windows>()
        .1
        .lock()
        .map_err(err)?
        .get(id)
        .cloned();
    Ok(
        json!({"ok":true,"instance":crate::instance::current().id,"id":id,"kind":"web","color":crate::memo_color::read(&app.state::<crate::storage::Store>().0, id)?,"title":w.title().map_err(err)?,"url":r.url,"welcome":r.url.is_none(),"current_url":current_url,"navigation_url":navigation_url,"visible":w.is_visible().map_err(err)?,"focused":w.is_focused().map_err(err)?,"always_on_top":w.is_always_on_top().map_err(err)?,"position":w.outer_position().map_err(err)?,"size":w.inner_size().map_err(err)?}),
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
// Poll only registered hosts. CLI inspect also queries the remote view, reads
// color storage and measures geometry, none of which the manager needs.
pub(crate) fn manager_entries(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    let records = app.state::<Windows>().0.lock().map_err(err)?.clone();
    records
        .into_iter()
        .map(|(id, record)| {
            Ok(json!({
                "id": id, "kind": "web",
                "title": record.url.as_deref().unwrap_or("Welcome"),
                "visible": crate::manager::window_visible(&record.host)?
            }))
        })
        .collect()
}

pub(crate) fn set_manager_visibility(
    app: &tauri::AppHandle,
    id: &str,
    visible: bool,
) -> Result<bool, String> {
    let _activity = crate::updater::activity(app)?;
    if crate::privacy::onboarding_required(app) {
        return Err("약관 확인이 필요합니다".into());
    }
    // Look up the host at click time. A removed web session must never fall
    // through to the ordinary open path, which can create a blank text memo.
    let host = record(app, id)?.host;
    if visible {
        crate::focus::discard_restore(app, host.label());
        if host.is_minimized().map_err(err)? {
            restore_minimized_without_focus(&host)?;
        }
        show(&host)?;
    } else {
        hide(&host)?;
    }
    crate::manager::window_visible(&host)
}

#[cfg(target_os = "macos")]
fn restore_minimized_without_focus(host: &WebviewWindow) -> Result<(), String> {
    // The manager command runs on the main thread. Tao consults the window's
    // focusable flag in both canBecomeKeyWindow and canBecomeMainWindow.
    let native = unsafe { &*(host.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow) };
    if !native.canBecomeKeyWindow() {
        return host.unminimize().map_err(err);
    }
    host.set_focusable(false).map_err(err)?;
    let restored = host.unminimize().map_err(err);
    let focusable = host.set_focusable(true).map_err(err);
    restored.and(focusable)
}
#[cfg(windows)]
use crate::windows_window::restore_minimized_without_focus;
#[cfg(not(any(target_os = "macos", windows)))]
fn restore_minimized_without_focus(host: &WebviewWindow) -> Result<(), String> {
    host.unminimize().map_err(err)
}

fn hide(w: &WebviewWindow) -> Result<(), String> {
    crate::stash::release(w)?;
    crate::relation_prototype::changed(w.app_handle(), w.label());
    w.hide().map_err(err)
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
    Ok(())
}
// Configure only public NSScrollView descendants owned by this remote view.
// No private WK selectors or application/global scrollbar preferences are used.
fn configure_native_scrollbars(content: &tauri::Webview) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    content
        .with_webview(|platform| unsafe {
            use objc2::{runtime::NSObjectProtocol, ClassType};
            use objc2_app_kit::{NSScrollView, NSScrollerStyle, NSView};
            fn apply(view: &NSView) {
                if view.isKindOfClass(NSScrollView::class()) {
                    // SAFETY: isKindOfClass above checked this public superclass.
                    let scroll = unsafe { &*(view as *const NSView as *const NSScrollView) };
                    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
                }
                for child in view.subviews() {
                    apply(&child);
                }
            }
            apply(&*platform.inner().cast::<NSView>());
        })
        .map_err(err)?;
    #[cfg(not(target_os = "macos"))]
    let _ = content;
    Ok(())
}
// Keep remote content below the trusted toolbar. A decorated macOS host has
// a titlebar inset between Wry child coordinates and the visible content area.
fn content_bounds(chrome: &tauri::Webview) -> Result<tauri::Rect, String> {
    // Resize events precede WebView autoresizing. Read the current native frame
    // on macOS instead of the previous WebView size, including its title inset.
    #[cfg(target_os = "macos")]
    let (size, inset) = unsafe {
        let host = chrome.window();
        let native = &*(host.ns_window().map_err(err)? as *const objc2_app_kit::NSWindow);
        let frame = native.frame();
        let content = native.contentLayoutRect();
        (
            tauri::LogicalSize::new(content.size.width, frame.size.height),
            (frame.size.height - content.size.height).max(0.0),
        )
    };
    #[cfg(not(target_os = "macos"))]
    let (size, inset) = (
        chrome
            .window()
            .inner_size()
            .map_err(err)?
            .to_logical::<f64>(chrome.window().scale_factor().map_err(err)?),
        0.0,
    );
    Ok(tauri::Rect {
        position: tauri::LogicalPosition::new(3.0, 30.0 + inset).into(),
        size: tauri::LogicalSize::new(
            (size.width - 6.0).max(1.0),
            (size.height - inset - 33.0).max(1.0),
        )
        .into(),
    })
}
pub fn create(
    app: &tauri::AppHandle,
    bounds: Option<ImageNote>,
    id: Option<&str>,
    url: Option<&str>,
) -> Result<Value, String> {
    let mut result = create_presented(app, bounds, id, url, true)?;
    if let Some(url) = url {
        // History failure must not turn an already-opened page into a failed
        // navigation. Keep it explicit in the response instead.
        result["recent_saved"] =
            json!(recent::remember(&app.state::<crate::storage::Store>().0, url).is_ok());
    }
    Ok(result)
}
pub(crate) fn create_presented(
    app: &tauri::AppHandle,
    bounds: Option<ImageNote>,
    id: Option<&str>,
    url: Option<&str>,
    present: bool,
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
    // Restored pre-palette windows retain their legacy color. New windows get
    // a color once; navigation and recreation reuse their persistent sidecar.
    if present && !contains(app, &id) {
        crate::memo_color::create(&app.state::<crate::storage::Store>().0, &id)?;
    }
    let remote = parsed.is_some();
    let label = format!("webview-{id}");
    let old = if contains(app, &id) {
        Some(record(app, &id)?)
    } else {
        None
    };
    if let Some(r) = &old {
        if url.is_none() || r.url.as_deref() == url {
            let w = r.host.clone();
            show(&w)?;
            return inspect(app, &id);
        }
    }
    let previous = old.as_ref().map(|r| r.host.clone());
    // A replacement is prepared before the current window is removed.
    // Repeated remote navigation uses the same isolated WebView.
    if let Some(w) = previous
        .as_ref()
        .filter(|_| old.as_ref().is_some_and(|r| r.url.is_some()) && remote)
    {
        app.get_webview(&format!("webpage-{id}"))
            .ok_or("웹 콘텐츠를 찾을 수 없습니다")?
            .navigate(parsed.unwrap())
            .map_err(err)?;
        app.state::<Windows>()
            .0
            .lock()
            .map_err(err)?
            .get_mut(&id)
            .unwrap()
            .url = url.map(str::to_owned);
        show(w)?;
        return inspect(app, &id);
    }
    let w = if let Some(previous) = &previous {
        previous.clone()
    } else {
        crate::focus::remember(app, &label);
        let window =
            tauri::WebviewWindowBuilder::new(app, &label, WebviewUrl::App("web.html".into()))
                .title(format!(
                    "{} — 웹 — {id}",
                    crate::instance::current().title()
                ))
                .inner_size(600.0, 400.0)
                .min_inner_size(240.0, 160.0)
                .always_on_top(true)
                .decorations(false)
                .visible(false)
                .focused(false)
                .on_navigation(|url| {
                    url.path() == "/web.html"
                        && url.port().is_none()
                        && url.query().is_none()
                        && ((url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                            || (matches!(url.scheme(), "http" | "https")
                                && url.host_str() == Some("tauri.localhost")))
                })
                .build()
                .map_err(err)?;
        if let Some(b) = bounds {
            #[cfg(windows)]
            window
                .set_position(tauri::PhysicalPosition::new(
                    b.x.round() as i32,
                    b.y.round() as i32,
                ))
                .map_err(err)?;
            #[cfg(not(windows))]
            window
                .set_position(tauri::LogicalPosition::new(b.x, b.y))
                .map_err(err)?;
            window
                .set_size(tauri::LogicalSize::new(
                    b.width.max(240.0),
                    b.height.max(160.0),
                ))
                .map_err(err)?;
        }
        let handle = app.clone();
        let content_label = format!("webpage-{id}");
        let parent_label = label.clone();
        window.on_window_event(move |event| {
            if matches!(
                event,
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. }
            ) {
                if let (Some(chrome), Some(content)) = (
                    handle.get_webview(&parent_label),
                    handle.get_webview(&content_label),
                ) {
                    if let Ok(bounds) = content_bounds(&chrome) {
                        let _ = content.set_bounds(bounds);
                    }
                }
            }
        });
        window
    };
    if let Some(target) = parsed {
        let content = tauri::webview::WebviewBuilder::new(
            format!("webpage-{id}"),
            WebviewUrl::External(target),
        )
        .user_agent(MOBILE_USER_AGENT)
        .initialization_script(include_str!("web_presentation.js"))
        .on_navigation({
            let app = app.clone();
            let id = id.clone();
            move |url| {
                if validate_url(url.as_str()).is_err() {
                    return false;
                }
                if let Ok(mut observed) = app.state::<Windows>().2.lock() {
                    observed.remove(&id);
                }
                if let Ok(mut urls) = app.state::<Windows>().1.lock() {
                    urls.insert(id.clone(), url.as_str().to_owned());
                }
                true
            }
        })
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_download(|_, _| false);
        let size = w
            .inner_size()
            .map_err(err)?
            .to_logical::<f64>(w.scale_factor().map_err(err)?);
        let content = match w.as_ref().window().add_child(
            content,
            tauri::LogicalPosition::new(0.0, 30.0),
            tauri::LogicalSize::new(size.width, (size.height - 30.0).max(1.0)),
        ) {
            Ok(content) => content,
            Err(error) => {
                if previous.is_none() {
                    let _ = w.destroy();
                }
                return Err(err(error));
            }
        };
        // Reapply after attachment: Wry now resolves the child's actual parent.
        if let Err(error) = configure_native_scrollbars(&content).and_then(|_| {
            content_bounds(w.as_ref()).and_then(|bounds| content.set_bounds(bounds).map_err(err))
        }) {
            // Preserve an existing Welcome and remove only this attempted child.
            let _ = content.close();
            if previous.is_none() {
                let _ = w.destroy();
            }
            return Err(error);
        }
    }
    app.state::<Windows>().0.lock().map_err(err)?.insert(
        id.clone(),
        Record {
            created_at: old
                .as_ref()
                .map(|r| r.created_at)
                .unwrap_or_else(|| creation_time(&id)),
            host: w.clone(),
            label,
            url: url.map(str::to_owned),
        },
    );
    if present {
        show(&w)?;
    }
    inspect(app, &id)
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
    let w = r.host.clone();
    match request.action {
        Action::Update
            if request.color.is_some()
                && request.title.is_none()
                && request.markdown.is_none()
                && request.status.is_none() =>
        {
            crate::memo_color::write(
                &app.state::<crate::storage::Store>().0,
                request.id(),
                request.color.as_deref().unwrap(),
            )?;
            app.emit_to(
                r.label.as_str(),
                "memo-color",
                crate::memo_color::read(&app.state::<crate::storage::Store>().0, request.id())?,
            )
            .map_err(err)?;
            inspect(app, request.id())
        }
        Action::Inspect => inspect(app, request.id()),
        Action::Close => {
            hide(&w)?;
            inspect(app, request.id())
        }
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
        _ => Err("웹 창에는 web/open/inspect/close 또는 색상만 지정한 update를 사용하세요".into()),
    }
}
#[tauri::command]
pub fn web_recent(view: tauri::Webview, remove: Option<String>) -> Result<Vec<String>, String> {
    let id = view
        .label()
        .strip_prefix("webview-")
        .ok_or("웹 시작 화면에서만 사용할 수 있습니다")?;
    let r = record(view.app_handle(), id)?;
    if r.label != view.label() || r.url.is_some() {
        return Err("웹 시작 화면에서만 사용할 수 있습니다".into());
    }
    let root = &view.state::<crate::storage::Store>().0;
    match remove {
        Some(url) => recent::remove(root, &url),
        None => recent::list(root),
    }
}
#[tauri::command]
pub async fn web_run(window: tauri::Webview, url: String) -> Result<Value, String> {
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
#[tauri::command]
pub fn web_chrome(view: tauri::Webview, action: String) -> Result<Value, String> {
    let id = view
        .label()
        .strip_prefix("webview-")
        .ok_or("웹 툴바에서만 실행할 수 있습니다")?;
    let r = record(view.app_handle(), id)?;
    let window = r.host.clone();
    match action.as_str() {
        "inspect" => inspect(window.app_handle(), id),
        "drag" => {
            crate::monitor_layout::manual(&window);
            crate::relation_prototype::changed(window.app_handle(), window.label());
            window.start_dragging().map_err(err)?;
            Ok(json!({"ok":true}))
        }
        action if action.starts_with("resize-") => {
            crate::monitor_layout::manual(&window);
            crate::relation_prototype::changed(window.app_handle(), window.label());
            let direction =
                serde_json::from_value(json!(action.strip_prefix("resize-").unwrap_or_default()))
                    .map_err(err)?;
            window
                .as_ref()
                .window()
                .start_resize_dragging(direction)
                .map_err(err)?;
            Ok(json!({"ok":true}))
        }
        "pin" => {
            window
                .set_always_on_top(!window.is_always_on_top().map_err(err)?)
                .map_err(err)?;
            inspect(window.app_handle(), id)
        }
        "close" => {
            // Match the current web window close contract, which preserves the session.
            crate::relation_prototype::changed(window.app_handle(), window.label());
            window.hide().map_err(err)?;
            Ok(json!({"ok":true,"url":r.url}))
        }
        _ => Err("Unknown web toolbar action".into()),
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn generated_web_creation_order_is_not_navigation_or_color_update_time() {
        assert_eq!(
            super::creation_time("web-100"),
            std::time::UNIX_EPOCH + std::time::Duration::from_nanos(100)
        );
        assert!(super::creation_time("web-100") < super::creation_time("web-200"));
        let before = std::time::SystemTime::now();
        let custom = super::creation_time("web-custom");
        assert!(custom >= before && custom <= std::time::SystemTime::now());
    }

    use super::*;
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
