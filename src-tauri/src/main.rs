mod capture;
mod entrance;
mod export;
mod feedback;
mod feedback_window;
mod focus;
mod instance;
mod ipc;
mod manager;
mod update_policy;
mod update_restore;
mod updater;

mod layout;
mod local_trace;
mod model;
mod opacity;
mod placement;
mod presentation;
mod privacy;
mod sizing;
mod skills;
mod source;
mod stash;
mod stash_input;
mod storage;
mod telemetry;
mod terminal;
mod tray;
mod warp;
mod web;
#[cfg(windows)]
mod windows_contact;
#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_window;
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
fn create_window(app: &tauri::AppHandle, id: &str) -> Result<tauri::WebviewWindow, String> {
    let _timing = telemetry::Timing::new("window.create");
    telemetry::checkpoint("window.create");
    let mut config = app.config().app.windows[0].clone();
    config.label = label(id);
    config.title = format!("{} — {id}", instance::current().title());
    focus::remember(app, &config.label);
    let win = tauri::WebviewWindowBuilder::from_config(app, &config)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
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
    Ok(hovered)
}
// Native alpha blends the entire window, including its background and controls.
// Called by synchronous commands and the IPC main-thread handler only.
#[tauri::command]
fn sync_opacity(window: tauri::WebviewWindow) -> Result<f64, String> {
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
        #[cfg(target_os = "macos")]
        let feedback_hovered = feedback_window::pointer_over(&window);
        #[cfg(windows)]
        let feedback_hovered = false; // Windows uses the memo's own feedback area.
        let alpha = opacity::target(
            manual,
            hovered || feedback_hovered,
            entrance::presenting(&window),
            feedback::active(&window),
            sizing::folded(&window),
        );
        let alpha = window
            .state::<opacity::Fades>()
            .lock()
            .map_err(|e| e.to_string())?
            .entry(window.label().into())
            .or_insert_with(|| opacity::Fade::at(manual.unwrap_or(1.0)))
            .sample(alpha, Instant::now());
        opacity::set_native(&window, alpha)?;
        feedback_window::set_opacity(&window, alpha)?;
        Ok(alpha)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        if let Some(alpha) = manual {
            opacity::set_native(&window, alpha)?;
            feedback_window::set_opacity(&window, alpha)?;
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
        || json!({"ok":true,"stash":stash::inspect(win.app_handle()),"feedback_surface":surface,"hovered":hovered,"instance":instance::current().id,"dev_label":instance::current().label,"id":id,"note":note,"storage_path":win.app_handle().state::<storage::Store>().path(id),"feedback_active":feedback_active,"highlight_revision":feedback::highlight_revision(win),"opacity":opacity,"global_opacity":opacity::configured(win.app_handle()),"animating":entrance::active(win),"auto_layout_excluded":layout::excluded(win),"layout_pending":sizing::pending(win),"content_height_cap":sizing::content_height(win),"layout_destination":entrance::destination(win).map(|d|json!({"x":d.position.x,"y":d.position.y,"width":d.size.width,"height":d.size.height})),"visible":visible,"focused":focused,"folded": sizing::folded(win), "always_on_top":pinned,"warp_right":win.current_monitor().ok().flatten().map(|m| i64::from(position.x) * 2 + i64::from(size.width) < i64::from(m.work_area().position.x) * 2 + i64::from(m.work_area().size.width)),"position":{"x":position.x,"y":position.y},"size":{"width":size.width,"height":size.height}}),
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
    stash::release(&window)?;
    layout::mark(&window, false);
    sizing::cancel(&window);
    feedback::clear(&window);
    feedback_window::hide(&window);
    entrance::finish(&window)?;
    window.hide().map_err(|e| e.to_string())?;
    tray::sync_visibility(window.app_handle())
}
type ClosedNotes = std::sync::Mutex<std::collections::BTreeMap<String, Value>>;
#[tauri::command]
fn close_note(window: tauri::WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let id = window
        .label()
        .strip_prefix("note-")
        .unwrap_or("main")
        .to_string();
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
    apply_with_presentation(app, request, entrance::Presentation::Cursor)
}
fn apply_direct(app: &tauri::AppHandle, request: Request) -> Result<Value, String> {
    apply_with_presentation(app, request, entrance::Presentation::Direct)
}
fn apply_with_presentation(
    app: &tauri::AppHandle,
    request: Request,
    presentation: entrance::Presentation,
) -> Result<Value, String> {
    request.validate()?;
    // Read-only diagnostics must remain observable during the installation handshake.
    if matches!(request.action, Action::Diagnostics) {
        return Ok(
            json!({"ok":true,"instance":instance::current().id,"diagnostics":telemetry::status(),"updater":updater::diagnostics(app),"privacy":{"terms_accepted":privacy::accepted(app),"terms_skipped":privacy::terms_skipped(),"onboarding_required":privacy::onboarding_required(app),"sharing_allowed":privacy::sharing(app),"notice_ready":privacy::notice_ready()}}),
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
    if matches!(request.action, Action::Web) || web::contains(app, request.id()) {
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
    if existing.is_none() && !matches!(request.action, Action::Open) {
        return Err(format!(
            "메모 창을 찾을 수 없습니다: {id}. 먼저 open --id {id}를 실행하세요"
        ));
    }
    let mut next = if existing.as_ref().is_none_or(|note| note.revision == 0)
        && matches!(request.action, Action::Open)
    {
        // The startup main placeholder is not a created note. Persisted notes,
        // including hidden and explicitly colored notes, continue the cycle.
        let created_count = state
            .lock()
            .map_err(|e| e.to_string())?
            .values()
            .filter(|note| note.revision > 0)
            .count();
        Note::with_palette_color(created_count)
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
        create_window(app, &id)?
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
            stash::release(&win)?;
            sizing::cancel(&win);
            feedback::clear(&win);
            feedback_window::hide(&win);
            entrance::finish(&win)?;
            win.hide().map_err(|e| e.to_string())?;
            tray::sync_visibility(app)?;
        }
        Action::Update
        | Action::Inspect
        | Action::List
        | Action::Diagnostics
        | Action::Probe
        | Action::Web => (),
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
    let initial_focus = if initial_open {
        focus::Targets::initial()
    } else {
        focus::Targets::default()
    };
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(false);
    let app = builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(privacy::Privacy::default())
        .manage(telemetry::Runtime::default())
        .manage(capture::Captures::default())
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
        .manage(stash::Stash::default())
        .manage(Mutex::new(BTreeMap::from([(
            "main".to_string(),
            Note::default(),
        )])))
        .invoke_handler({
            let handler: Box<dyn Fn(tauri::ipc::Invoke) -> bool + Send + Sync> =
                Box::new(tauri::generate_handler![
                    updater::updater_status,
                    updater::updater_check,
                    updater::updater_install,
                    updater::updater_notice_status,
                    updater::updater_notice_ready,
                    updater::updater_notice_install,
                    updater::updater_notice_dismiss,
                    updater::updater_ack,
                    skills::skills_list,
                    skills::skills_set,
                    privacy::privacy_status,
                    privacy::privacy_save,
                    privacy::privacy_quit,
                    local_trace::local_flow_status,
                    local_trace::local_flow_begin,
                    local_trace::local_flow_stage,
                    snapshot,
                    web::web_run,
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
                    feedback_window::feedback_snapshot,
                    feedback_window::layout_feedback,
                    feedback_window::feedback_action,
                    feedback_window::drag_feedback,
                    capture::capture_screen,
                    capture::capture_ready,
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
                    toggle_pin,
                    sync_opacity,
                    feedback::acknowledge_feedback,
                    save_note,
                    export::export_note,
                    warp::warp_note,
                    warp::poll_warp,
                    sizing::fit_note,
                    sizing::poll_fold,
                    sizing::toggle_fold,
                    sizing::chrome_fold,
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
                {
                    invoke.resolver.reject("업데이트 설치를 준비하고 있습니다");
                    return true;
                }
                let exempt = matches!(
                    command,
                    "manager_set_theme" // Local cosmetic-only manager command.
                        | "privacy_status"
                        | "privacy_save"
                        | "privacy_quit"
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
            if win.label() == manager::LABEL && privacy::onboarding_required(win.app_handle()) {
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                    win.app_handle().exit(0);
                }
                return;
            }
            if win.label().starts_with("webview-") || win.label().starts_with("webpage-") {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win.hide();
                }
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
                if matches!(
                    event,
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                ) {
                    terminal::geometry(win);
                }
                if let Some(w) = win.app_handle().get_webview_window(win.label()) {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            let _ = terminal::request_close(&w);
                        }
                        tauri::WindowEvent::Focused(false)
                            if !w.is_always_on_top().unwrap_or(true) =>
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
            if win.label() == "stash-indicator"
                || win.label().starts_with("capture-")
                || feedback_window::is_surface(win.label())
            {
                return;
            }
            if matches!(
                event,
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
            ) {
                if let Some(webview) = win.app_handle().get_webview_window(win.label()) {
                    feedback_window::sync(&webview);
                }
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
            let restored = store.load()?;
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
            terminal::restore(app.handle())?;
            for id in ids {
                let win = create_window(app.handle(), &id)?;
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
            updater::initialize(app.handle());
            tray::install(app.handle())?;
            capture::install(app.handle())?;
            stash::install(app.handle())?;
            tray::refresh(app.handle())?;
            #[cfg(target_os = "macos")]
            sizing::install_contact_monitor(app.handle());
            #[cfg(windows)]
            windows_contact::install(app.handle());
            if initial_open && !restored_update {
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
            if privacy::onboarding_required(app.handle()) {
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
    app.run(|_app, _event| {
        if matches!(_event, tauri::RunEvent::Exit) {
            telemetry::shutdown(_app);
            terminal::shutdown(_app);
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            if let Err(error) = reopen_from_icon(_app) {
                eprintln!("앱 아이콘으로 메모를 열지 못했습니다: {error}");
            }
        }
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn reopen_from_icon(app: &tauri::AppHandle) -> Result<(), String> {
    let _update_activity = updater::activity(app)?;
    let mut visible = false;
    // Only note windows count; capture overlays must not prevent reopening.
    let ids: Vec<_> = app
        .state::<AppNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .keys()
        .cloned()
        .collect();
    for id in ids {
        let Some(win) = app.get_webview_window(&label(&id)) else {
            continue;
        };
        if win.is_visible().map_err(|e| e.to_string())? {
            show_without_focus(&win)?;
            visible = true;
        }
    }
    if !visible {
        apply_direct(
            app,
            Request {
                url: None,
                action: Action::Open,
                id: None,
                color: None,
                title: None,
                markdown: None,
                change_mode: None,
                highlight: None,
                source_path: None,
                status: None,
            },
        )?;
    }
    Ok(())
}

fn parse(args: &[String]) -> Result<Request, String> {
    let action = match args.first().map(String::as_str) { Some("diagnostics") => Action::Diagnostics, Some("diagnostics-probe") => Action::Probe, Some("web") => Action::Web, Some("open") => Action::Open, Some("update") => Action::Update, Some("close") => Action::Close, Some("inspect") => Action::Inspect, Some("list") => Action::List, _ => return Err("사용법: agent-gibbon web [--id web-ID] [--url URL]; agent-gibbon diagnostics|diagnostics-probe; open|update [--id ID] [--color #RRGGBB] [--title 제목] [--status 상태] [--file 경로|- 또는 --markdown 본문] / close|inspect [--id ID] / list".into()) };
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
    let mut i = 1;
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
fn cli(args: &[String]) -> Result<Value, String> {
    let request = parse(args)?;
    // Preserve invalid local IPC paths instead of masking them as startup errors.
    ipc::directory()?;
    let stream = match ipc::connect() {
        Ok(stream) => stream,
        Err(_) if matches!(request.action, Action::Open | Action::Web) => {
            let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
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
            let start = Instant::now();
            loop {
                if let Ok(stream) = ipc::connect() {
                    break stream;
                }
                if start.elapsed() > Duration::from_secs(15) {
                    return Err("앱 시작에 실패했습니다. agent-gibbon --serve로 진단하세요".into());
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
    let result = if args.is_empty() || args == ["--serve"] {
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
            json!({"ok":true,"usage":"agent-gibbon web [--id web-ID] [--url URL]; agent-gibbon diagnostics|diagnostics-probe; open|update [--id ID] [--color #RRGGBB] [--title TEXT] [--status TEXT] [--change-mode diff|replace] [--highlight none|newly-completed] [--file PATH|- | --markdown TEXT]; agent-gibbon close|inspect [--id ID]; agent-gibbon list","note":"open/web은 앱을 시작하거나 창을 생성·복원합니다. web의 ID 생략은 새 웹 창, open의 ID 생략은 main이며 close는 숨기기입니다. 색상은 #RRGGBB입니다."}),
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
