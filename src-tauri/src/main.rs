#[cfg(target_os = "macos")]
mod app_activation;
mod autostart;
mod capture;
mod entrance;
mod export;
mod feedback;
mod feedback_window;
mod first_frame;
mod focus;
mod instance;
mod ipc;
mod manager;
mod update_policy;
mod update_restore;
mod updater;

mod language;
#[cfg(target_os = "macos")]
mod language_menu;
mod layout;
mod local_trace;
mod memo_color;
mod model;
mod monitor_layout;
mod opacity;
mod placement;
mod presentation;
mod privacy;
mod relation_editor;
mod relation_prototype;
mod session_restore;
mod sizing;
mod skills;
mod source;
mod stash;
mod stash_input;
mod storage;
mod telemetry;
mod terminal;
#[cfg(target_os = "macos")]
mod toolbar_pointer;
mod tray;
mod warp;
mod web;
#[cfg(windows)]
mod windows_contact;
#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_window;
mod workspace;
use model::{Action, Note, Request, MAX_REQUEST};
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State};

type AppNotes = Mutex<BTreeMap<String, Note>>;
type PointerStates = Mutex<BTreeMap<String, bool>>;
fn label(id: &str) -> String {
    if id == "main" {
        "main".into()
    } else {
        format!("note-{id}")
    }
}
// The configured main memo is the template for all trusted memo hosts.
fn memo_window_config(app: &tauri::AppHandle) -> &tauri::utils::config::WindowConfig {
    &app.config().app.windows[0]
}
fn create_window(
    app: &tauri::AppHandle,
    id: &str,
    region: bool,
) -> Result<tauri::WebviewWindow, String> {
    #[cfg(target_os = "macos")]
    let _passive = app_activation::passive_creation();
    let _timing = telemetry::Timing::new("window.create");
    telemetry::checkpoint("window.create");
    let mut config = memo_window_config(app).clone();
    config.label = label(id);
    config.title = format!("{} — {id}", instance::current().title());
    focus::remember(app, &config.label);
    let builder =
        tauri::WebviewWindowBuilder::from_config(app, &config).map_err(|e| e.to_string())?;
    let win = first_frame::prepare_builder(builder, region)
        .build()
        .map_err(|e| e.to_string())?;
    // Use the same OS-owned external shadow for text and capture memo hosts.
    // No CSS gutter/paint may cover image pixels or transparent letterboxing.
    // Limit this explicit setting to macOS: Windows adds a white 1px border
    // when set_shadow(true) is called on an undecorated window.
    #[cfg(target_os = "macos")]
    win.set_shadow(true).map_err(|e| e.to_string())?;
    // Region-created notes also have a stable place if later re-included.
    #[cfg(windows)]
    {
        windows_window::install_resize_observer(&win)?;
        focus::restore(&win);
    }
    layout::register(&win, true);
    Ok(win)
}
// Called only on Tauri's main thread. macOS `show()` makes the window key;
// ordering it front directly preserves the user's active application.
fn show_without_focus(win: &tauri::WebviewWindow) -> Result<(), String> {
    opacity::reset_for_reveal(win)?;
    sync_opacity(win.clone())?;
    if stash::owns(win) {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let ptr = win.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: Tauri owns this live NSWindow; main-thread callers retain win
        // throughout the synchronous call and never store the borrowed pointer.
        unsafe {
            (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
            focus::restore(win);
        }
        tray::sync_visibility(win.app_handle())
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(windows)]
        {
            focus::show_without_focus(win)?;
            sync_opacity(win.clone())?;
        }
        #[cfg(not(windows))]
        win.show().map_err(|e| e.to_string())?;
        tray::sync_visibility(win.app_handle())
    }
}
fn pointer_over(window: &tauri::WebviewWindow) -> Result<bool, String> {
    if !window.is_visible().map_err(|e| e.to_string())? {
        return Ok(false);
    }
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    let p = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    Ok(cursor.x >= f64::from(p.x)
        && cursor.y >= f64::from(p.y)
        && cursor.x < f64::from(p.x) + f64::from(size.width)
        && cursor.y < f64::from(p.y) + f64::from(size.height))
}
fn sync_pointer(window: &tauri::WebviewWindow) -> Result<bool, String> {
    let hovered = pointer_over(window)?;
    let changed = window
        .state::<PointerStates>()
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), hovered)
        != Some(hovered);
    if changed {
        // Emitter::emit broadcasts even when called on a window. Hover belongs
        // only to this memo; other WebViews must never receive its boolean.
        window
            .emit_to(window.label(), "note-pointer-over", hovered)
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    toolbar_pointer::sync(window)?;
    Ok(hovered)
}
// Native alpha blends the entire window, including its background and controls.
// Called by synchronous commands and the IPC main-thread handler only.
#[tauri::command]
fn sync_opacity(window: tauri::WebviewWindow) -> Result<f64, String> {
    if layout::overview::protects(&window)
        || matches!(window.label(), "memo-backdrop" | "memo-overview")
    {
        sync_pointer(&window)?;
        #[cfg(any(target_os = "macos", windows))]
        window
            .state::<opacity::Fades>()
            .lock()
            .unwrap()
            .remove(window.label());
        opacity::set_native(&window, 1.0)?;
        return Ok(1.0);
    }
    if stash::owns(&window)
        || window.label() == "stash-indicator"
        || window.label().starts_with("capture-")
    {
        #[cfg(target_os = "macos")]
        unsafe {
            return Ok((&*(window.ns_window().map_err(|e| e.to_string())?
                as *const objc2_app_kit::NSWindow))
                .alphaValue());
        }
        #[cfg(windows)]
        return windows_window::opacity(&window);
        #[cfg(not(any(target_os = "macos", windows)))]
        return Ok(1.0);
    }
    if window.label() == manager::LABEL {
        #[cfg(windows)]
        windows_window::set_opacity(&window, 1.0)?;
        #[cfg(target_os = "macos")]
        {
            window
                .state::<opacity::Fades>()
                .lock()
                .map_err(|e| e.to_string())?
                .remove(manager::LABEL);
            let ptr = window.ns_window().map_err(|e| e.to_string())?;
            // SAFETY: Tauri owns this live NSWindow; called on the main thread.
            unsafe {
                (&*(ptr as *const objc2_app_kit::NSWindow)).setAlphaValue(1.0);
            }
        }
        return Ok(1.0);
    }
    let hovered = sync_pointer(&window)?;
    let manual = opacity::configured(window.app_handle());
    #[cfg(any(target_os = "macos", windows))]
    {
        let presenting = entrance::presenting(&window);
        let moving = entrance::active(&window) && !presenting;
        let attention = feedback::active(&window);
        let folded = sizing::folded(&window);
        let alpha = window
            .state::<opacity::Fades>()
            .lock()
            .map_err(|e| e.to_string())?
            .entry(window.label().into())
            .or_insert_with(|| opacity::Fade::at(manual.unwrap_or(1.0)))
            .sample_memo(
                manual,
                hovered || presenting,
                moving,
                attention,
                folded,
                Instant::now(),
            );
        opacity::set_native(&window, alpha)?;
        Ok(alpha)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        if let Some(alpha) = manual {
            opacity::set_native(&window, alpha)?;
            return Ok(alpha);
        }
        let _ = (window, hovered);
        Ok(1.0)
    }
}
fn inspect_window(win: &tauri::WebviewWindow, id: &str, note: &Note) -> Result<Value, String> {
    let mut trace = local_trace::Call::new(false);
    inspect_window_traced(win, id, note, &mut trace)
}
fn state_stage<T>(
    trace: &local_trace::Call,
    stage: local_trace::Stage,
    operation: &'static str,
    work: impl FnOnce() -> T,
) -> T {
    trace.stage(stage);
    let _timing = telemetry::Timing::new(operation);
    work()
}
fn inspect_window_traced(
    win: &tauri::WebviewWindow,
    id: &str,
    note: &Note,
    trace: &mut local_trace::Call,
) -> Result<Value, String> {
    use local_trace::Stage;
    let opacity = state_stage(trace, Stage::Opacity, "note.state.opacity", || {
        sync_opacity(win.clone())
    })?;
    let feedback_active = state_stage(trace, Stage::Feedback, "note.state.feedback", || {
        feedback::active(win)
    });
    let position = state_stage(trace, Stage::Position, "note.state.position", || {
        win.outer_position().map_err(|e| e.to_string())
    })?;
    let size = state_stage(trace, Stage::Size, "note.state.size", || {
        win.outer_size().map_err(|e| e.to_string())
    })?;
    let visible = state_stage(trace, Stage::Visibility, "note.state.visibility", || {
        win.is_visible().map_err(|e| e.to_string())
    })?;
    let focused = state_stage(trace, Stage::Focus, "note.state.focus", || {
        win.is_focused().map_err(|e| e.to_string())
    })?;
    let pinned = state_stage(trace, Stage::Pin, "note.state.pin", || {
        win.is_always_on_top().map_err(|e| e.to_string())
    })?;
    let surface = state_stage(trace, Stage::Feedback, "note.state.feedback", || {
        feedback_window::inspect(win)
    })?;
    let hovered = state_stage(trace, Stage::Opacity, "note.state.opacity", || {
        pointer_over(win)
    })?;
    let result = state_stage(
        trace,
        Stage::Response,
        "note.state.response",
        || json!({"ok":true,"stash":stash::inspect(win.app_handle()),"feedback_surface":surface,"hovered":hovered,"instance":instance::current().id,"dev_label":instance::current().label,"id":id,"note":note,"storage_path":win.app_handle().state::<storage::Store>().path(id),"feedback_active":feedback_active,"capture_toolbar_below":sizing::image_toolbar_below(win),"highlight_revision":feedback::highlight_revision(win),"opacity":opacity,"global_opacity":opacity::configured(win.app_handle()),"animating":entrance::active(win),"auto_layout_excluded":layout::excluded(win),"layout_pending":sizing::pending(win),"content_height_cap":sizing::content_height(win),"layout_destination":entrance::destination(win).map(|d|json!({"x":d.position.x,"y":d.position.y,"width":d.size.width,"height":d.size.height})),"visible":visible,"focused":focused,"folded": sizing::folded(win), "always_on_top":pinned,"warp_right":win.current_monitor().ok().flatten().map(|m| i64::from(position.x) * 2 + i64::from(size.width) < i64::from(m.work_area().position.x) * 2 + i64::from(m.work_area().size.width)),"position":{"x":position.x,"y":position.y},"size":{"width":size.width,"height":size.height}}),
    );
    trace.complete();
    Ok(result)
}
#[tauri::command]
fn snapshot(
    window: tauri::WebviewWindow,
    notes: State<AppNotes>,
    local_flow: Option<u64>,
) -> Result<Value, String> {
    let _scope = local_trace::webview_scope(&window, local_flow);
    let _timing = telemetry::Timing::new("snapshot.request");
    // Tauri injects the calling window; the frontend cannot select another note.
    let mut trace = local_trace::Call::new(true);
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    trace.stage(local_trace::Stage::NotesWait);
    let note = {
        let notes = {
            let _wait = telemetry::Timing::new("note.state.notes_wait");
            notes.lock().map_err(|e| e.to_string())?
        };
        trace.stage(local_trace::Stage::NotesCopy);
        let _copy = telemetry::Timing::new("note.state.notes_copy");
        notes.get(id).cloned().ok_or("메모를 찾을 수 없습니다")?
    };
    trace.cause(note.local_mutation);
    // Native getters and feedback inspection must not hold AppNotes.
    inspect_window_traced(&window, id, &note, &mut trace)
}
#[tauri::command]
fn save_note(
    window: tauri::WebviewWindow,
    notes: State<AppNotes>,
    markdown: String,
    expected_revision: u64,
    local_flow: Option<u64>,
) -> Result<Note, String> {
    let _scope = local_trace::webview_scope(&window, local_flow);
    let _timing = telemetry::Timing::new("save_note");
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    let mut notes = notes.lock().map_err(|e| e.to_string())?;
    let note = notes.get_mut(id).ok_or("메모를 찾을 수 없습니다")?;
    window.app_handle().state::<storage::Store>().save_edit(
        id,
        note,
        markdown,
        expected_revision,
    )?;
    note.local_mutation = local_trace::current();
    layout::mark(&window, false);
    feedback::edited(&window)?;
    Ok(note.clone())
}
#[tauri::command]
fn hide_note(window: tauri::WebviewWindow) -> Result<(), String> {
    relation_prototype::changed(window.app_handle(), window.label());
    stash::release(&window)?;
    layout::mark(&window, false);
    sizing::cancel(&window);
    feedback::clear(&window);
    entrance::finish(&window)?;
    window.hide().map_err(|e| e.to_string())?;
    tray::sync_visibility(window.app_handle())
}
type ClosedNotes = std::sync::Mutex<std::collections::BTreeMap<String, Value>>;
fn memo_id_from_window(label: &str) -> Option<&str> {
    if label == "main" {
        Some("main")
    } else {
        label.strip_prefix("note-").filter(|id| !id.is_empty())
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn close_hit_rect(window: tauri::WebviewWindow, rect: [f64; 4]) -> Result<(), String> {
    if memo_id_from_window(window.label()).is_none() {
        return Err("메모 창이 아닙니다".into());
    }
    extern "C" {
        fn jajan_close_hit_rect(web: *mut std::ffi::c_void, x: f64, y: f64, w: f64, h: f64);
    }
    let [x, y, w, h] = rect;
    window
        .with_webview(move |web| unsafe {
            // with_webview supplies the live WKWebView on AppKit's main thread.
            jajan_close_hit_rect(web.inner().cast(), x, y, w, h);
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn close_note(window: tauri::WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let id = memo_id_from_window(window.label())
        .ok_or("메모 창이 아닙니다")?
        .to_string();
    #[cfg(target_os = "macos")]
    {
        // AppKit chooses a new key/main window synchronously in orderOut:.
        // A visible background manager must not be promoted by this close.
        app_activation::preserve_focus();
        let manager = app.get_webview_window(manager::LABEL);
        if let Some(manager) = &manager {
            app_activation::prepare_close(&window, manager)?;
        }
        let manager = manager.filter(|manager| {
            window.is_focused().unwrap_or(false) && manager.is_visible().unwrap_or(false)
        });
        if let Some(manager) = &manager {
            manager.set_focusable(false).map_err(|e| e.to_string())?;
        }
        let hidden = hide_note(window.clone());
        if let Some(manager) = manager {
            manager.set_focusable(true).map_err(|e| e.to_string())?;
        }
        hidden?;
    }
    #[cfg(not(target_os = "macos"))]
    hide_note(window.clone())?;
    let note = app
        .state::<AppNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .get(&id)
        .cloned()
        .ok_or("메모가 없습니다")?;
    let mut state = inspect_window(&window, &id, &note)?;
    state["visible"] = json!(false);
    state["closed"] = json!(true);
    state["feedback_surface"] = Value::Null;
    feedback_window::destroy(&window)?;
    app.state::<ClosedNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id, state);
    window.destroy().map_err(|e| e.to_string())?;
    tray::sync_visibility(app)
}
#[tauri::command]
fn toggle_pin(window: tauri::WebviewWindow) -> Result<(), String> {
    layout::mark(&window, false);
    window
        .set_always_on_top(!window.is_always_on_top().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn apply(app: &tauri::AppHandle, request: Request) -> Result<Value, String> {
    apply_with_presentation(app, request, entrance::Presentation::Cursor, None)
}
fn apply_direct(app: &tauri::AppHandle, request: Request) -> Result<Value, String> {
    apply_with_presentation(app, request, entrance::Presentation::Direct, None)
}
fn apply_with_presentation(
    app: &tauri::AppHandle,
    request: Request,
    presentation: entrance::Presentation,
    expected: Option<manager::RowKind>,
) -> Result<Value, String> {
    request.validate()?;
    // Read-only diagnostics must remain observable during the installation handshake.
    if matches!(request.action, Action::Diagnostics) {
        return Ok(
            json!({"ok":true,"instance":instance::current().id,"diagnostics":telemetry::status(),"updater":updater::diagnostics(app),"autostart":autostart::diagnostics(app),"monitor_layout":monitor_layout::inspect(app),"privacy":{"terms_accepted":privacy::accepted(app),"terms_skipped":privacy::terms_skipped(),"onboarding_required":privacy::onboarding_required(app),"sharing_allowed":privacy::sharing(app),"notice_ready":privacy::notice_ready()}}),
        );
    }
    let _activity = updater::activity(app)?;
    if privacy::onboarding_required(app)
        && !matches!(request.action, Action::Diagnostics | Action::Probe)
    {
        manager::open(app)?;
        return Ok(
            json!({"ok":false,"onboarding_required":true,"error":"Review the Terms of Use in the manager; sharing is optional","instance":instance::current().id}),
        );
    }
    if matches!(request.action, Action::Probe) {
        return Ok(telemetry::probe());
    }
    if let Action::Workspace(operation) = &request.action {
        return workspace::execute(app, operation);
    }
    if matches!(request.action, Action::Web) || web::contains(app, request.id()) {
        if expected.is_some() {
            return Err("메모 종류가 변경되었습니다".into());
        }
        return web::handle(app, &request);
    }
    let _timing = telemetry::Timing::new(match request.action {
        Action::Open => "ipc.open",
        Action::Update => "ipc.update",
        Action::Close => "ipc.close",
        Action::Inspect => "ipc.inspect",
        Action::List => "ipc.list",
        _ => unreachable!(),
    });
    if terminal::contains(app, request.id()) {
        if expected == Some(manager::RowKind::Note) {
            return Err("메모 종류가 변경되었습니다".into());
        }
        return terminal::handle(app, &request);
    }
    let state = app.state::<AppNotes>();
    if matches!(request.action, Action::List) {
        let notes = state.lock().map_err(|e| e.to_string())?;
        let mut windows = Vec::new();
        for (id, note) in notes.iter() {
            let win = app.get_webview_window(&label(id));
            windows.push(
                json!({"id":id,"title":note.title,"status":note.status,"color":note.color,
                "visible":win.as_ref().is_some_and(|w|w.is_visible().unwrap_or(false)),
                "always_on_top":win.as_ref().is_none_or(|w|w.is_always_on_top().unwrap_or(true))}),
            );
        }
        for agent in terminal::entries(app) {
            windows.push(terminal::inspect(app, &agent.id)?);
        }
        windows.extend(web::entries(app)?);
        return Ok(json!({"ok":true,"windows":windows}));
    }
    let id = request.id().to_string();
    // No lock is held while creating a WebView: its startup may invoke snapshot.
    let existing = state.lock().map_err(|e| e.to_string())?.get(&id).cloned();
    if expected == Some(manager::RowKind::Terminal) || (expected.is_some() && existing.is_none()) {
        return Err("메모가 없거나 종류가 변경되었습니다".into());
    }
    if existing.is_none() && !matches!(request.action, Action::Open) {
        return Err(format!(
            "메모 창을 찾을 수 없습니다: {id}. 먼저 open --id {id}를 실행하세요"
        ));
    }
    let mut next = if existing.as_ref().is_none_or(|note| note.revision == 0)
        && matches!(request.action, Action::Open)
    {
        Note::with_random_color()
    } else {
        existing.clone().unwrap_or_default()
    };
    next.apply(&request)?;
    if matches!(request.action, Action::Open | Action::Update) {
        next.local_mutation = local_trace::current();
    }
    if matches!(request.action, Action::Open) {
        focus::remember(app, &label(&id));
    }
    if matches!(request.action, Action::Open | Action::Update) {
        app.state::<storage::Store>().save(&id, &next)?;
    }
    if !matches!(request.action, Action::Open) {
        let closed_state = app.state::<ClosedNotes>();
        let mut closed = closed_state.lock().map_err(|e| e.to_string())?;
        if let Some(response) = closed.get_mut(&id) {
            response["note"] = serde_json::to_value(&next).map_err(|e| e.to_string())?;
            state.lock().map_err(|e| e.to_string())?.insert(id, next);
            return Ok(response.clone());
        }
    }
    app.state::<ClosedNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&id);
    let win = if let Some(win) = app.get_webview_window(&label(&id)) {
        win
    } else {
        create_window(app, &id, false)?
    };
    feedback::changed(&win, existing.as_ref(), &next)?;
    if matches!(request.action, Action::Open | Action::Close)
        || (matches!(request.action, Action::Update)
            && (request.markdown.is_some()
                || request.title.is_some()
                || request.status.is_some()
                || request.color.is_some()))
    {
        layout::mark(&win, false);
    }
    match request.action {
        Action::Open => {
            sizing::prepare(&win, presentation)?;
        }
        Action::Close => {
            relation_prototype::changed(app, win.label());
            stash::release(&win)?;
            sizing::cancel(&win);
            feedback::clear(&win);
            entrance::finish(&win)?;
            win.hide().map_err(|e| e.to_string())?;
            tray::sync_visibility(app)?;
        }
        Action::Update
        | Action::Inspect
        | Action::List
        | Action::Diagnostics
        | Action::Probe
        | Action::Web
        | Action::Workspace(_) => (),
    }
    // Existing metadata is recollected before the next menu open. Replacing a
    // tracking native menu here would dismiss it during background updates.
    let menu_changed = existing.is_none();
    let response = inspect_window(&win, &id, &next);
    state.lock().map_err(|e| e.to_string())?.insert(id, next);
    if menu_changed {
        if let Err(error) = tray::refresh(app) {
            eprintln!("메뉴 막대 갱신 실패: {error}");
        }
    }
    response
}
fn serve(endpoint: ipc::Endpoint, initial_open: bool) -> Result<(), String> {
    // CLI starts with --serve too: restored visible windows still need the
    // foreground target captured before Tao/Wry can activate this process.
    #[cfg(target_os = "macos")]
    let mut launch_creation = Some(app_activation::passive_creation());
    let initial_focus = focus::Targets::initial();
    let builder = tauri::Builder::default().plugin(
        tauri::plugin::Builder::<_, ()>::new("language")
            .setup(|app, _| {
                language::initialize(app)?;
                Ok(())
            })
            .build(),
    );
    #[cfg(target_os = "macos")]
    let builder = builder
        .activate_ignoring_other_apps(false)
        .plugin(
            tauri::plugin::Builder::<_, ()>::new("memo-hide")
                .on_window_ready(stash::configure_native_hide)
                .build(),
        )
        .manage(toolbar_pointer::Pointers::default());
    let app = builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(privacy::Privacy::default())
        .manage(autostart::Autostart::default())
        .manage(telemetry::Runtime::default())
        .manage(capture::Captures::default())
        .manage(workspace::Sharing::default())
        .manage(workspace::Placements::default())
        .manage(terminal::cli::Requests::default())
        .manage(first_frame::Pending::default())
        .manage(terminal::Terminals::default())
        .manage(web::Windows::default())
        .manage(ClosedNotes::default())
        .manage(PointerStates::default())
        .manage(initial_focus)
        .manage(warp::Warps::default())
        .manage(feedback::Feedbacks::default())
        .manage(feedback_window::Surfaces::default())
        .manage(entrance::Entrances::default())
        .manage(updater::Updates::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(layout::Exclusions::default())
        .manage(layout::OpenOrder::default())
        .manage(sizing::Sizes::default())
        .manage(layout::Undo::default())
        .manage(layout::LastLayout::default())
        .manage(stash::Stash::default())
        .invoke_handler({
            let handler: Box<dyn Fn(tauri::ipc::Invoke) -> bool + Send + Sync> =
                Box::new(tauri::generate_handler![
                    language::language_status,
                    language::language_save,
                    layout::undo_arrangement,
                    layout::overview::overview_action,
                    capture::permission_guide,
                    updater::updater_status,
                    updater::updater_check,
                    updater::updater_install,
                    updater::updater_test_prepare,
                    updater::updater_test_install,
                    updater::updater_test_roundtrip,
                    updater::updater_notice_status,
                    updater::updater_notice_ready,
                    updater::updater_notice_install,
                    updater::updater_notice_dismiss,
                    updater::updater_ack,
                    skills::skills_list,
                    skills::skills_set,
                    privacy::privacy_status,
                    autostart::autostart_status,
                    autostart::autostart_save,
                    autostart::autostart_open_settings,
                    privacy::privacy_save,
                    privacy::privacy_quit,
                    local_trace::local_flow_status,
                    local_trace::local_flow_begin,
                    local_trace::local_flow_stage,
                    snapshot,
                    relation_prototype::relation_ready,
                    relation_editor::relation_status,
                    relation_editor::relation_begin,
                    relation_editor::relation_picker_info,
                    relation_editor::relation_pick,
                    relation_editor::relation_editor_state,
                    relation_editor::relation_save,
                    relation_editor::relation_cancel,
                    web::web_run,
                    web::web_recent,
                    web::web_chrome,
                    terminal::terminal_start,
                    terminal::terminal_write,
                    terminal::terminal_resize,
                    terminal::terminal_info,
                    terminal::welcome::terminal_welcome,
                    terminal::welcome::terminal_folder,
                    terminal::welcome::terminal_rename,
                    terminal::welcome::terminal_close,
                    manager::manager_set_theme,
                    manager::manager_notes,
                    manager::manager_set_visibility,
                    manager::manager_terminal,
                    manager::manager_image,
                    layout::content_action,
                    feedback_window::present_feedback,
                    feedback_window::memo_frame_focus,
                    feedback_window::feedback_action,
                    capture::capture_ready,
                    first_frame::region_frame_placement,
                    first_frame::region_frame_ready,
                    capture::capture_failed,
                    capture::finish_capture,
                    capture::cancel_capture,
                    capture::note_image,
                    capture::copy_note_image,
                    telemetry::report_frontend_error,
                    telemetry::telemetry_status,
                    telemetry::telemetry_heartbeat,
                    entrance::drag_note,
                    hide_note,
                    close_note,
                    #[cfg(target_os = "macos")]
                    close_hit_rect,
                    toggle_pin,
                    sync_opacity,
                    feedback::acknowledge_feedback,
                    save_note,
                    export::export_note,
                    warp::warp_note,
                    warp::poll_warp,
                    sizing::fit_note,
                    sizing::enter_note_editor,
                    sizing::poll_fold,
                    sizing::toggle_fold,
                    sizing::chrome_fold,
                    sizing::chrome_drag,
                    sizing::chrome_work_area,
                    sizing::manual_resize
                ]);
            move |invoke: tauri::ipc::Invoke| {
                // Remote pages never receive application commands, even if an
                // origin is incorrectly classified as local by the WebView runtime.
                if invoke.message.webview_ref().label().starts_with("webpage-") {
                    invoke
                        .resolver
                        .reject("External web content has no application IPC access");
                    return true;
                }
                let command = invoke.message.command();
                if updater::frozen(invoke.message.webview_ref().app_handle())
                    && !command.starts_with("updater_")
                    && command != "language_status"
                {
                    invoke.resolver.reject("업데이트 설치를 준비하고 있습니다");
                    return true;
                }
                let exempt = matches!(
                    command,
                    "language_status" | "language_save" | "manager_set_theme" // Local cosmetic-only manager command.
                        | "privacy_status"
                        | "privacy_save"
                        | "privacy_quit"
                        | "autostart_status"
                        | "autostart_save"
                        | "autostart_open_settings"
                        | "telemetry_status"
                        | "report_frontend_error"
                );
                if !exempt
                    && privacy::onboarding_required(invoke.message.webview_ref().app_handle())
                {
                    invoke
                        .resolver
                        .reject("Review the Terms of Use in the manager first");
                    true
                } else {
                    handler(invoke)
                }
            }
        })
        .on_window_event(|win, event| {
            monitor_layout::native_event(win, event);
            if matches!(event, tauri::WindowEvent::Destroyed) {
                first_frame::cancel(win.app_handle(), win.label());
                if win.label().starts_with("terminal-") {
                    terminal::cli::cancel_pending(win.app_handle(), win.label());
                }
            }
            if win.label() == "relation-settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    relation_editor::cancel(win.app_handle(), true);
                    return;
                }
            }
            if matches!(event, tauri::WindowEvent::Moved(_)) {
                relation_prototype::moved(win.app_handle(), win.label());
            } else if matches!(
                event,
                tauri::WindowEvent::Resized(_)
                    | tauri::WindowEvent::ScaleFactorChanged { .. }
                    | tauri::WindowEvent::Destroyed
            ) {
                relation_prototype::changed(win.app_handle(), win.label());
            }
            if matches!(event, tauri::WindowEvent::Focused(_)) {
                relation_prototype::focus_changed();
                if let Some(window) = win.app_handle().get_webview_window(win.label()) {
                    feedback_window::sync_frame_focus(&window);
                }
            }
            if win.label() == manager::LABEL && privacy::onboarding_required(win.app_handle()) {
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                    win.app_handle().exit(0);
                }
                return;
            }
            if win.label().starts_with("webview-") || win.label().starts_with("webpage-") {
                // The web host handles close with its own session identity.
                return;
            }
            if win.label() == "updater-notice" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(window) = win.app_handle().get_webview_window(win.label()) {
                        if let Err(error) = updater::close_notice(&window) {
                            eprintln!("업데이트 안내를 닫지 못했습니다: {error}");
                        }
                    }
                }
                return;
            }
            if win.label().starts_with("terminal-") {
                if let Some(w) = win.app_handle().get_webview_window(win.label()) {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            let _ = terminal::request_close(&w);
                        }
                        tauri::WindowEvent::Focused(false)
                            if !w.is_always_on_top().unwrap_or(true)
                                && !layout::overview::protects(&w) =>
                        {
                            let _ = terminal::terminal_hide(w);
                        }
                        _ => (),
                    }
                }
                return;
            }
            if win.label() == manager::LABEL {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = win.app_handle().get_webview_window(win.label()) {
                        let _ = stash::release(&w);
                    }
                    let _ = win.hide();
                }
                return;
            }
            if memo_id_from_window(win.label()).is_none() {
                return;
            }
            #[cfg(any(target_os = "macos", windows))]
            if let tauri::WindowEvent::Resized(_) = event {
                if let Some(webview) = win.app_handle().get_webview_window(win.label()) {
                    if let Err(error) = sizing::native_resize(&webview) {
                        eprintln!("수동 크기 기록 실패: {error}");
                    }
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(webview) = win.app_handle().get_webview_window(win.label()) {
                    let _ = close_note(webview);
                }
            }
        })
        .setup(move |app| {
            app.manage(Mutex::new(BTreeMap::from([(
                "main".to_owned(),
                Note::default(),
            )])));
            #[cfg(target_os = "macos")]
            language_menu::install(app.handle())?;
            local_trace::start(app.handle());
            privacy::initialize(app.handle())?;
            opacity::initialize(app.handle())?;
            skills::synchronize(app.handle().clone());
            let _startup = telemetry::Timing::new("app.startup");
            #[cfg(any(target_os = "macos", windows))]
            {
                app.manage(opacity::Fades::default());
                opacity::start(app.handle().clone());
            }
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            if let Some(win) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                win.set_shadow(true)?;
                #[cfg(windows)]
                windows_window::install_resize_observer(&win)?;
                win.set_title(&format!("{} — main", instance::current().title()))?;
            }
            let instance = instance::current().id.as_deref();
            let data_dir = app.path().app_data_dir()?;
            let root = if instance.is_some() {
                data_dir.clone()
            } else {
                app.path().document_dir()?
            };
            let store = storage::Store::new(root, instance);
            store.migrate_from(&data_dir.join("notes").join(instance.unwrap_or("default")))?;
            let mut restored = store.load()?;
            if !restored.contains_key("main") {
                let welcome = Note::default();
                store.save("main", &welcome)?;
                restored.insert("main".to_owned(), welcome);
            }
            app.state::<feedback::Feedbacks>().restore(&restored)?;
            let ids: Vec<_> = restored
                .keys()
                .filter(|id| id.as_str() != "main")
                .cloned()
                .collect();
            app.state::<AppNotes>()
                .lock()
                .map_err(|e| e.to_string())?
                .extend(restored);
            app.manage(store);
            monitor_layout::initialize(app.handle());
            terminal::restore(app.handle())?;
            for id in ids {
                let win = create_window(app.handle(), &id, false)?;
                let capture = app
                    .state::<AppNotes>()
                    .lock()
                    .map_err(|e| e.to_string())?
                    .get(&id)
                    .and_then(|n| n.capture.clone());
                if let Some(bounds) = capture {
                    capture::place(&win, &bounds)?;
                }
            }
            let restored_update = match update_restore::restore(app.handle()) {
                Ok(restored) => restored,
                Err(error) => {
                    eprintln!("업데이트 창 복원 실패: {error}");
                    true
                }
            };
            let mut session_enabled = true;
            let restored_session = if restored_update {
                false
            } else {
                match session_restore::restore(app.handle()) {
                    Ok(restored) => restored,
                    Err(error) => {
                        session_enabled = false;
                        eprintln!(
                            "시작 창 복원 실패 (원본 보존, 이번 실행의 상태 저장 중지): {error}"
                        );
                        true
                    }
                }
            };
            // Explicit macOS app launch opens management even when windows were restored.
            #[cfg(target_os = "macos")]
            let _ = restored_session;
            session_restore::start(app.handle(), session_enabled);
            updater::initialize(app.handle());
            relation_prototype::install(app.handle());
            tray::install(app.handle())?;
            capture::install(app.handle())?;
            stash::install(app.handle())?;
            tray::refresh(app.handle())?;
            #[cfg(target_os = "macos")]
            sizing::install_contact_monitor(app.handle());
            #[cfg(windows)]
            windows_contact::install(app.handle());
            #[cfg(not(target_os = "macos"))]
            if initial_open && !autostart::login_launch() && !restored_update && !restored_session {
                apply_direct(
                    app.handle(),
                    Request {
                        url: None,
                        change_mode: None,
                        highlight: None,
                        source_path: None,
                        action: Action::Open,
                        id: None,
                        color: None,
                        title: None,
                        markdown: None,
                        status: None,
                    },
                )?;
            }
            telemetry::install(app.handle());
            #[cfg(feature = "verification-roundtrip")]
            manager::open(app.handle())?;
            #[cfg(not(target_os = "macos"))]
            if privacy::onboarding_required(app.handle()) && !autostart::login_launch() {
                manager::open(app.handle())?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for connection in endpoint.listener.incoming() {
                    let Ok(mut stream) = connection else { continue };
                    let response = match ipc::read_request(&mut stream) {
                        Err(e) => ipc::error(e),
                        Ok(request) => {
                            let (tx, rx) = mpsc::sync_channel(1);
                            let _wait = telemetry::Timing::ipc_wait(matches!(
                                request.action,
                                Action::Inspect | Action::List
                            ));
                            let sentry_parent = _wait.context();
                            let target = handle.clone();
                            let deadline = Instant::now() + Duration::from_secs(5);
                            let mut local_call = local_trace::Call::ipc();
                            match handle.run_on_main_thread(move || {
                                let _sentry_scope = sentry_parent.enter();
                                let _scope = local_call.scope();
                                local_call.stage(local_trace::Stage::UiExecute);
                                let result = if Instant::now() > deadline {
                                    ipc::error("요청 처리 기한이 만료되었습니다")
                                } else {
                                    apply(&target, request).unwrap_or_else(ipc::error)
                                };
                                local_call.finish(result["ok"] == true);
                                let _ = tx.send(result);
                            }) {
                                Err(e) => ipc::error(e),
                                Ok(()) => {
                                    rx.recv_timeout(Duration::from_secs(6)).unwrap_or_else(|_| {
                                        ipc::error("앱 응답 시간이 초과되었습니다")
                                    })
                                }
                            }
                        }
                    };
                    let _ = serde_json::to_writer(&mut stream, &response);
                    let _ = stream.flush();
                }
                drop(endpoint);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    app.run(move |_app, _event| {
        // Complete the launch transaction after Tao activation and Tauri setup.
        #[cfg(any(target_os = "macos", windows))]
        if matches!(_event, tauri::RunEvent::Ready) {
            #[cfg(target_os = "macos")]
            drop(launch_creation.take());
            if let Some(main) = _app.get_webview_window("main") {
                focus::restore(&main);
            }
            #[cfg(target_os = "macos")]
            app_activation::install(_app);
            #[cfg(target_os = "macos")]
            if !autostart::login_launch() && (initial_open || privacy::onboarding_required(_app)) {
                // Explicit app launch/required onboarding focus only after Tao
                // has finished launch, outside the passive startup transaction.
                if let Err(error) = manager::open(_app) {
                    eprintln!("시작 관리 창을 열지 못했습니다: {error}");
                }
            }
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::ExitRequested {
            code: None,
            ref api,
            ..
        } = _event
        {
            if _app.webview_windows().is_empty() {
                // Closing the last memo leaves the tray and app switcher available.
                api.prevent_exit();
                return;
            }
        }
        if matches!(_event, tauri::RunEvent::ExitRequested { .. }) {
            session_restore::checkpoint(_app);
        }
        if matches!(_event, tauri::RunEvent::Exit) {
            telemetry::shutdown(_app);
            terminal::shutdown(_app);
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            if !autostart::current_login_event() {
                if let Err(error) = reopen_from_icon(_app) {
                    eprintln!("앱 아이콘으로 메모를 열지 못했습니다: {error}");
                }
            }
        }
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn reopen_from_icon(app: &tauri::AppHandle) -> Result<(), String> {
    let _update_activity = updater::activity(app)?;
    // Dock/application Reopen is an explicit selection, independent of passive creation.
    manager::open(app)
}

fn parse(args: &[String]) -> Result<Request, String> {
    let action = match args.first().map(String::as_str) { Some("workspace") => Action::Workspace(workspace::parse(&args[1..])?), Some("diagnostics") => Action::Diagnostics, Some("diagnostics-probe") => Action::Probe, Some("web") => Action::Web, Some("open") => Action::Open, Some("update") => Action::Update, Some("close") => Action::Close, Some("inspect") => Action::Inspect, Some("list") => Action::List, _ => return Err("사용법: jajan web [--id web-ID] [--url URL]; jajan diagnostics|diagnostics-probe; open|update [--id ID] [--color #RRGGBB] [--title 제목] [--status 상태] [--file 경로|- 또는 --markdown 본문] / close|inspect [--id ID] / list".into()) };
    let mut request = Request {
        url: None,
        action,
        change_mode: None,
        highlight: None,
        source_path: None,
        id: None,
        color: None,
        title: None,
        status: None,
        markdown: None,
    };
    let mut i = if matches!(request.action, Action::Workspace(_)) {
        args.len()
    } else {
        1
    };
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).ok_or("옵션 값이 없습니다")?;
        match key.as_str() {
            "--change-mode" if request.change_mode.is_none() => {
                request.change_mode = Some(match value.as_str() {
                    "diff" => model::ChangeMode::Diff,
                    "replace" => model::ChangeMode::Replace,
                    _ => return Err("change-mode는 diff 또는 replace여야 합니다".into()),
                })
            }
            "--highlight" if request.highlight.is_none() => {
                request.highlight = Some(match value.as_str() {
                    "none" => model::Highlight::None,
                    "newly-completed" => model::Highlight::NewlyCompleted,
                    _ => return Err("highlight는 none 또는 newly-completed여야 합니다".into()),
                })
            }
            "--url" if request.url.is_none() => request.url = Some(value.clone()),
            "--id" if request.id.is_none() => request.id = Some(value.clone()),
            "--color" if request.color.is_none() => request.color = Some(value.clone()),
            "--title" if request.title.is_none() => request.title = Some(value.clone()),
            "--status" if request.status.is_none() => request.status = Some(value.clone()),
            "--markdown" if request.markdown.is_none() => request.markdown = Some(value.clone()),
            "--file" if request.markdown.is_none() => {
                let input: Box<dyn Read> = if value == "-" {
                    Box::new(std::io::stdin())
                } else {
                    let path = std::fs::canonicalize(value).map_err(|e| e.to_string())?;
                    request.source_path = Some(
                        path.to_str()
                            .ok_or("파일 경로는 UTF-8이어야 합니다")?
                            .to_owned(),
                    );
                    Box::new(std::fs::File::open(path).map_err(|e| e.to_string())?)
                };
                let mut text = String::new();
                input
                    .take(MAX_REQUEST + 1)
                    .read_to_string(&mut text)
                    .map_err(|e| e.to_string())?;
                if text.len() as u64 > MAX_REQUEST {
                    return Err("본문은 1MiB 이하여야 합니다".into());
                }
                request.markdown = Some(text);
            }
            _ => return Err(format!("알 수 없거나 중복된 옵션: {key}")),
        }
        i += 2;
    }
    request.validate()?;
    if serde_json::to_vec(&request)
        .map_err(|e| e.to_string())?
        .len() as u64
        > MAX_REQUEST
    {
        return Err("요청은 1MiB 이하여야 합니다".into());
    }
    Ok(request)
}
#[cfg(any(target_os = "macos", test))]
fn enclosing_app(executable: &std::path::Path) -> Option<&std::path::Path> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    (macos.ends_with("MacOS") && contents.ends_with("Contents") && bundle.extension()? == "app")
        .then_some(bundle)
}

fn launch_cli_server() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    if crate::instance::current().id.is_none() {
        if let Some(bundle) = enclosing_app(&executable) {
            let result = Command::new("/usr/bin/open")
                .arg("-g")
                .arg(bundle)
                .args(["--args", "--serve"])
                .output()
                .map_err(|e| e.to_string())?;
            return if result.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "설치 앱을 열지 못했습니다: {}",
                    String::from_utf8_lossy(&result.stderr).trim()
                ))
            };
        }
    }
    let mut child = Command::new(executable);
    child
        .arg("--serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Detach the GUI from short-lived agent/terminal command sessions.
    // SAFETY: setsid is async-signal-safe and touches no Rust state after fork.
    #[cfg(unix)]
    unsafe {
        child.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(windows)]
    windows_process::spawn(&mut child).map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    child.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
#[test]
fn launcher_recognizes_only_bundle_executables() {
    use std::path::Path;
    assert_eq!(
        enclosing_app(Path::new("/Applications/Jajan.app/Contents/MacOS/jajan")),
        Some(Path::new("/Applications/Jajan.app"))
    );
    assert_eq!(
        enclosing_app(Path::new("/work/src-tauri/target/debug/jajan")),
        None
    );
    assert_eq!(
        enclosing_app(Path::new("/Applications/Other.app/random/MacOS/jajan")),
        None
    );
}

fn cli(args: &[String]) -> Result<Value, String> {
    let request = parse(args)?;
    // Preserve invalid local IPC paths instead of masking them as startup errors.
    ipc::directory()?;
    let stream = match ipc::connect() {
        Ok(stream) => stream,
        Err(_)
            if matches!(
                request.action,
                Action::Open | Action::Web | Action::Workspace(_)
            ) =>
        {
            launch_cli_server()?;
            let start = Instant::now();
            loop {
                if let Ok(stream) = ipc::connect() {
                    break stream;
                }
                if start.elapsed() > Duration::from_secs(15) {
                    return Err("앱 시작에 실패했습니다. jajan --serve로 진단하세요".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        Err(_) => return Err("앱이 실행 중이지 않습니다. 먼저 open을 실행하세요".into()),
    };
    let response = ipc::exchange(stream, &request)?;
    if matches!(request.action, Action::Open) && response["ok"] == true {
        // Wait only for initial measurement/show, never for pointer movement.
        let inspect = Request {
            url: None,
            change_mode: None,
            highlight: None,
            source_path: None,
            action: Action::Inspect,
            id: request.id,
            color: None,
            title: None,
            markdown: None,
            status: None,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let state = ipc::exchange(ipc::connect()?, &inspect)?;
            if state["visible"] == true || Instant::now() >= deadline {
                return Ok(state);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    Ok(response)
}
fn main() {
    let helper_args: Vec<String> = std::env::args().skip(1).collect();
    if let [mode, path] = helper_args.as_slice() {
        if mode == "--terminal-exec" {
            let code = terminal::cli::helper(path).unwrap_or_else(|error| {
                eprintln!("{error}");
                127
            });
            std::process::exit(code);
        }
    }

    if let Err(error) = instance::init() {
        println!("{}", ipc::error(error));
        std::process::exit(1);
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    // Explorer launches should leave only the detached GUI, not a console window.
    #[cfg(windows)]
    let args = if args.is_empty() {
        vec!["open".to_string()]
    } else {
        args
    };
    let result = if args == ["--login-startup"] {
        // No CLI open request, new document, or manager focus at login.
        #[cfg(windows)]
        unsafe {
            #[link(name = "kernel32")]
            extern "system" {
                fn FreeConsole() -> i32;
            }
            FreeConsole();
        }
        match ipc::Endpoint::bind() {
            Ok(Some(endpoint)) => serve(endpoint, false).map(|_| json!({"ok":true})),
            Ok(None) => Ok(json!({"ok":true,"already_running":true})),
            Err(e) => Err(e),
        }
    } else if args.is_empty() || args == ["--serve"] {
        match ipc::Endpoint::bind() {
            Ok(Some(endpoint)) => serve(endpoint, args.is_empty()).map(|_| json!({"ok":true})),
            Ok(None) if args.is_empty() => cli(&["open".into()]),
            Ok(None) => Ok(json!({"ok":true,"already_running":true})),
            Err(e) => Err(e),
        }
    } else if args == ["--instance-info"] {
        Ok(
            json!({"ok":true,"instance":instance::current().id,"dev_label":instance::current().label,"manifest_dir":env!("CARGO_MANIFEST_DIR")}),
        )
    } else if args == ["--help"] || args == ["-h"] {
        Ok(
            json!({"ok":true,"workspace":workspace::USAGE,"usage":"jajan web [--id web-ID] [--url URL]; jajan diagnostics|diagnostics-probe; open|update [--id ID] [--color #RRGGBB] [--title TEXT] [--status TEXT] [--change-mode diff|replace] [--highlight none|newly-completed] [--file PATH|- | --markdown TEXT]; jajan close|inspect [--id ID]; jajan list","note":"open/web은 앱을 시작하거나 창을 생성·복원합니다. web의 ID 생략은 새 웹 창, open의 ID 생략은 main이며 close는 숨기기입니다. 색상은 #RRGGBB입니다."}),
        )
    } else {
        cli(&args)
    };
    let value = result.unwrap_or_else(ipc::error);
    println!("{value}");
    if value["ok"] != true {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_mode_options_parse_and_reject_unsupported_combinations() {
        let valid = [
            "update",
            "--markdown",
            "- [x] A",
            "--change-mode",
            "replace",
            "--highlight",
            "newly-completed",
        ];
        let request = parse(&valid.map(String::from)).unwrap();
        assert_eq!(request.change_mode, Some(model::ChangeMode::Replace));
        assert_eq!(request.highlight, Some(model::Highlight::NewlyCompleted));
        for args in [
            vec!["update", "--markdown", "x", "--change-mode", "invalid"],
            vec![
                "update",
                "--markdown",
                "x",
                "--change-mode",
                "replace",
                "--change-mode",
                "diff",
            ],
            vec![
                "update",
                "--markdown",
                "x",
                "--highlight",
                "newly-completed",
            ],
            vec!["update", "--change-mode", "replace"],
            vec!["open", "--markdown", "x", "--change-mode", "replace"],
        ] {
            assert!(parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }

    #[test]
    fn retired_agent_cli_and_project_ids_are_rejected() {
        assert!(parse(&["agent".into()]).is_err());
        for action in ["open", "update", "inspect", "close"] {
            assert!(parse(&[action.into(), "--id".into(), "agent-123".into()]).is_err());
        }
        assert!(serde_json::from_str::<Action>("\"agent\"").is_err());
    }

    #[test]
    fn invalid_commands_and_duplicate_content_fail() {
        for args in [
            vec!["oops"],
            vec!["close", "--title", "x"],
            vec!["update", "--markdown", "a", "--markdown", "b"],
            vec!["open", "--status"],
        ] {
            assert!(parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }
    #[test]
    fn named_window_commands_and_list_parse() {
        let args = ["open", "--id", "job-1", "--color", "#AABBCC"];
        let request = parse(&args.map(String::from)).unwrap();
        assert_eq!(request.id(), "job-1");
        assert_eq!(request.color.as_deref(), Some("#AABBCC"));
        for command in ["open", "update", "close", "inspect"] {
            assert!(parse(&[command.into(), "--id".into(), "job-1".into()]).is_ok());
        }
        assert!(parse(&["list".into()]).is_ok());
        for args in [
            vec!["open", "--id", "x", "--id", "y"],
            vec!["open", "--color", "#112233", "--color", "#445566"],
            vec!["list", "--id", "main"],
            vec!["inspect", "--color", "#112233"],
            vec!["open", "--id", ""],
            vec!["open", "--color", "red"],
        ] {
            assert!(parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }

    #[test]
    fn permission_window_never_resolves_to_main_note() {
        assert_eq!(super::memo_id_from_window("permission-guide"), None);
        assert_eq!(super::memo_id_from_window("main"), Some("main"));
        assert_eq!(super::memo_id_from_window("note-example"), Some("example"));
    }
    #[test]
    fn window_labels_do_not_collide() {
        assert_eq!(label("main"), "main");
        assert_ne!(label("main"), label("note-main"));
        assert_ne!(label("x"), label("note-x"));
    }
    #[test]
    fn web_cli_opens_welcome_without_a_url() {
        let request = parse(&["web", "--id", "web-review"].map(String::from))
            .expect("web 명령은 URL 없이 Welcome을 열 수 있어야 합니다");
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["action"], "web");
        assert_eq!(request.id(), "web-review");
        assert!(value.get("url").is_none_or(serde_json::Value::is_null));
    }

    #[test]
    fn web_cli_accepts_http_https_and_local_development_pages() {
        for url in [
            "https://example.com/path?q=hello#section",
            "http://127.0.0.1:3000/",
            "http://localhost:8080/",
            "http://[::1]:3000/",
        ] {
            let request = parse(&["web", "--id", "web-review", "--url", url].map(String::from))
                .unwrap_or_else(|error| panic!("유효한 웹 URL {url}: {error}"));
            let value = serde_json::to_value(request).unwrap();
            assert_eq!(value["action"], "web");
            assert_eq!(value["url"], url);
        }
    }

    #[test]
    fn web_cli_rejects_unsafe_urls_before_connecting_or_starting_the_app() {
        for url in [
            "",
            "example.com",
            "javascript:alert(1)",
            "data:text/html,hello",
            "file:///etc/passwd",
            "tauri://localhost/",
            "https://tauri.localhost/",
            "http://ipc.localhost/",
            "https://user:password@example.com/",
            "https://example.com/\npath",
            "https://example.com/\tpath",
        ] {
            let error = match parse(&["web", "--id", "web-review", "--url", url].map(String::from))
            {
                Ok(_) => panic!("위험하거나 모호한 URL을 허용했습니다: {url:?}"),
                Err(error) => error,
            };
            assert!(
                error.contains("URL"),
                "URL 검증 오류여야 합니다: {url:?}: {error}"
            );
        }
    }

    #[test]
    fn web_cli_rejects_note_content_and_duplicate_url_options() {
        for args in [
            vec!["web", "--markdown", "본문"],
            vec!["web", "--color", "#112233"],
            vec![
                "web",
                "--url",
                "https://example.com/",
                "--url",
                "https://example.org/",
            ],
            vec!["open", "--url", "https://example.com/"],
            vec!["update", "--url", "https://example.com/"],
            vec!["web", "--id", "main"],
            vec!["web", "--id", "terminal-123"],
        ] {
            assert!(parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }
}
