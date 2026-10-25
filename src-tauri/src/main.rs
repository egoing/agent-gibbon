mod entrance;
mod feedback;
mod ipc;
mod model;
mod placement;
mod sizing;
mod source;
mod tray;
mod warp;
use model::{Action, Note, Request, MAX_REQUEST};
use serde_json::{json, Value};
use std::os::unix::process::CommandExt;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, State};

type AppNotes = Mutex<BTreeMap<String, Note>>;
fn label(id: &str) -> String {
    if id == "main" {
        "main".into()
    } else {
        format!("note-{id}")
    }
}
fn window(app: &tauri::AppHandle, id: &str) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window(&label(id)).ok_or_else(|| {
        format!("메모 창을 찾을 수 없습니다: {id}. 먼저 open --id {id}를 실행하세요")
    })
}
fn create_window(app: &tauri::AppHandle, id: &str) -> Result<tauri::WebviewWindow, String> {
    let mut config = app.config().app.windows[0].clone();
    config.label = label(id);
    config.title = format!("Floatnote — {id}");
    tauri::WebviewWindowBuilder::from_config(app, &config)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())
}
// Window APIs and monitor work areas use physical pixels. Convert the new
// window's logical size per monitor so mixed-DPI displays retain its size.
fn place_new_window(
    app: &tauri::AppHandle,
    win: &tauri::WebviewWindow,
    requested_size: Option<tauri::LogicalSize<f64>>,
) -> Result<entrance::Destination, String> {
    use std::hash::{BuildHasher, Hasher};
    let random = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let mut occupied = Vec::new();
    for other in app.webview_windows().values() {
        if other.label() == win.label() || !other.is_visible().map_err(|e| e.to_string())? {
            continue;
        }
        let rect = if let Some(destination) = entrance::destination(other) {
            destination.rect()
        } else {
            let p = other.outer_position().map_err(|e| e.to_string())?;
            let s = other.outer_size().map_err(|e| e.to_string())?;
            placement::Rect {
                x: p.x.into(),
                y: p.y.into(),
                width: s.width.into(),
                height: s.height.into(),
            }
        };
        occupied.push(rect);
    }
    let mut monitors = win.available_monitors().map_err(|e| e.to_string())?;
    if let Some(current) = win.current_monitor().map_err(|e| e.to_string())? {
        if let Some(index) = monitors
            .iter()
            .position(|m| m.position() == current.position())
        {
            monitors.swap(0, index);
        }
    }
    let mut fallback = None;
    for monitor in monitors {
        let work = monitor.work_area();
        let gap = (8.0 * monitor.scale_factor()).round() as i64;
        let area = placement::Rect {
            x: i64::from(work.position.x) + gap,
            y: i64::from(work.position.y) + gap,
            width: i64::from(work.size.width) - 2 * gap,
            height: i64::from(work.size.height) - 2 * gap,
        };
        let logical = requested_size.unwrap_or(tauri::LogicalSize::new(
            f64::from(size.width) / scale,
            f64::from(size.height) / scale,
        ));
        let target = (
            (logical.width * monitor.scale_factor()).round() as i64,
            (logical.height * monitor.scale_factor()).round() as i64,
        );
        if let Some((score, rect)) = placement::least_overlap(area, target, &occupied) {
            if fallback
                .as_ref()
                .is_none_or(|(best, _, _, _)| score < *best)
            {
                fallback = Some((score, rect, work.position.y, monitor.scale_factor()));
            }
        }
        if let Some(rect) = placement::find_space(area, target, random, &occupied, gap) {
            win.set_position(tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32))
                .map_err(|e| e.to_string())?;
            win.set_size(tauri::PhysicalSize::new(
                rect.width as u32,
                rect.height as u32,
            ))
            .map_err(|e| e.to_string())?;
            warp::remember(app, win, rect.y, work.position.y, monitor.scale_factor())?;
            return Ok(entrance::Destination {
                position: tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32),
                size: tauri::PhysicalSize::new(rect.width as u32, rect.height as u32),
                scale: monitor.scale_factor(),
            });
        }
    }
    if let Some((_, rect, work_y, scale)) = fallback {
        win.set_position(tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32))
            .map_err(|e| e.to_string())?;
        win.set_size(tauri::PhysicalSize::new(
            rect.width as u32,
            rect.height as u32,
        ))
        .map_err(|e| e.to_string())?;
        warp::remember(app, win, rect.y, work_y, scale)?;
        return Ok(entrance::Destination {
            position: tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32),
            size: tauri::PhysicalSize::new(rect.width as u32, rect.height as u32),
            scale,
        });
    }
    Err("패널을 배치할 수 있는 화면이 없습니다".into())
}
// Called only on Tauri's main thread. macOS `show()` makes the window key;
// ordering it front directly preserves the user's active application.
fn show_without_focus(win: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let ptr = win.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: Tauri owns this live NSWindow; main-thread callers retain win
        // throughout the synchronous call and never store the borrowed pointer.
        unsafe {
            sync_opacity(win.clone())?;
            (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    win.show().map_err(|e| e.to_string())
}
// Native alpha blends the entire window, including its background and controls.
// Called by synchronous commands and the IPC main-thread handler only.
#[tauri::command]
fn sync_opacity(window: tauri::WebviewWindow) -> Result<f64, String> {
    #[cfg(target_os = "macos")]
    {
        let cursor = window.cursor_position().map_err(|e| e.to_string())?;
        let p = window.outer_position().map_err(|e| e.to_string())?;
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let hovered = cursor.x >= f64::from(p.x)
            && cursor.y >= f64::from(p.y)
            && cursor.x < f64::from(p.x) + f64::from(size.width)
            && cursor.y < f64::from(p.y) + f64::from(size.height);
        let alpha = if hovered || entrance::active(&window) || feedback::active(&window) {
            1.0
        } else if sizing::folded(&window) {
            0.8
        } else {
            0.5
        };
        let ptr = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: Tauri owns the live NSWindow, borrowed only on the main thread.
        unsafe {
            let native = &*(ptr as *const objc2_app_kit::NSWindow);
            if native.alphaValue() != alpha {
                native.setAlphaValue(alpha);
            }
        }
        Ok(alpha)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(1.0)
    }
}
fn inspect_window(win: &tauri::WebviewWindow, id: &str, note: &Note) -> Result<Value, String> {
    let opacity = sync_opacity(win.clone())?;
    let feedback_active = feedback::active(win);
    let position = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    Ok(
        json!({"ok":true,"id":id,"note":note,"feedback_active":feedback_active,"highlight_revision":feedback::highlight_revision(win),"opacity":opacity,"animating":entrance::active(win),"visible":win.is_visible().map_err(|e| e.to_string())?,"focused":win.is_focused().map_err(|e| e.to_string())?,"folded": sizing::folded(win), "always_on_top":win.is_always_on_top().map_err(|e| e.to_string())?,"position":{"x":position.x,"y":position.y},"size":{"width":size.width,"height":size.height}}),
    )
}
#[tauri::command]
fn snapshot(window: tauri::WebviewWindow, notes: State<AppNotes>) -> Result<Value, String> {
    // Tauri injects the calling window; the frontend cannot select another note.
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    let notes = notes.lock().map_err(|e| e.to_string())?;
    inspect_window(&window, id, notes.get(id).ok_or("메모를 찾을 수 없습니다")?)
}
#[tauri::command]
fn save_note(
    window: tauri::WebviewWindow,
    notes: State<AppNotes>,
    markdown: String,
    expected_revision: u64,
) -> Result<Note, String> {
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    let mut notes = notes.lock().map_err(|e| e.to_string())?;
    let note = notes.get_mut(id).ok_or("메모를 찾을 수 없습니다")?;
    let before = note.clone();
    note.save_edit(markdown, expected_revision)?;
    feedback::changed(&window, Some(&before), note)?;
    Ok(note.clone())
}
#[tauri::command]
fn hide_note(window: tauri::WebviewWindow) -> Result<(), String> {
    feedback::clear(&window);
    sizing::cancel(&window);
    entrance::finish(&window)?;
    window.hide().map_err(|e| e.to_string())
}
#[tauri::command]
fn toggle_pin(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .set_always_on_top(!window.is_always_on_top().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn apply(app: &tauri::AppHandle, request: Request) -> Result<Value, String> {
    request.validate()?;
    let state = app.state::<AppNotes>();
    if matches!(request.action, Action::List) {
        let notes = state.lock().map_err(|e| e.to_string())?;
        let mut windows = Vec::new();
        for (id, note) in notes.iter() {
            let win = window(app, id)?;
            windows.push(
                json!({"id":id,"title":note.title,"status":note.status,"color":note.color,
                "visible":win.is_visible().map_err(|e| e.to_string())?,
                "always_on_top":win.is_always_on_top().map_err(|e| e.to_string())?}),
            );
        }
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
    let mut next = existing.clone().unwrap_or_default();
    next.apply(&request)?;
    let win = if existing.is_some() {
        window(app, &id)?
    } else {
        create_window(app, &id)?
    };
    feedback::changed(&win, existing.as_ref(), &next)?;
    match request.action {
        Action::Open => {
            if existing.as_ref().is_none_or(|note| note.revision == 0) {
                sizing::prepare(&win)?;
            } else if let Err(error) = entrance::show(&win, None) {
                return Err(error);
            }
        }
        Action::Close => {
            feedback::clear(&win);
            sizing::cancel(&win);
            entrance::finish(&win)?;
            win.hide().map_err(|e| e.to_string())?;
        }
        Action::Update | Action::Inspect | Action::List => (),
    }
    let menu_changed = existing
        .as_ref()
        .is_none_or(|previous| previous.title != next.title);
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
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(false);
    let app = builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(warp::Warps::default())
        .manage(feedback::Feedbacks::default())
        .manage(entrance::Entrances::default())
        .manage(sizing::Sizes::default())
        .manage(Mutex::new(BTreeMap::from([(
            "main".to_string(),
            Note::default(),
        )])))
        .invoke_handler(tauri::generate_handler![
            snapshot,
            entrance::drag_note,
            hide_note,
            toggle_pin,
            sync_opacity,
            feedback::acknowledge_feedback,
            save_note,
            warp::warp_note,
            warp::poll_warp,
            sizing::fit_note,
            sizing::poll_fold,
            sizing::toggle_fold,
            sizing::manual_resize
        ])
        .on_window_event(|win, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(webview) = win.app_handle().get_webview_window(win.label()) {
                    feedback::clear(&webview);
                    sizing::cancel(&webview);
                    let _ = entrance::finish(&webview);
                }
                let _ = win.hide();
            }
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray::install(app.handle())?;
            if initial_open {
                apply(
                    app.handle(),
                    Request {
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
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for connection in endpoint.listener.incoming() {
                    let Ok(mut stream) = connection else { continue };
                    let response = match ipc::read_request(&mut stream) {
                        Err(e) => ipc::error(e),
                        Ok(request) => {
                            let (tx, rx) = mpsc::sync_channel(1);
                            let target = handle.clone();
                            let deadline = Instant::now() + Duration::from_secs(5);
                            match handle.run_on_main_thread(move || {
                                let result = if Instant::now() > deadline {
                                    ipc::error("요청 처리 기한이 만료되었습니다")
                                } else {
                                    apply(&target, request).unwrap_or_else(ipc::error)
                                };
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
    app.run(|_, _| {});
    Ok(())
}

fn parse(args: &[String]) -> Result<Request, String> {
    let action = match args.first().map(String::as_str) { Some("open") => Action::Open, Some("update") => Action::Update, Some("close") => Action::Close, Some("inspect") => Action::Inspect, Some("list") => Action::List, _ => return Err("사용법: floatnote open|update [--id ID] [--color #RRGGBB] [--title 제목] [--status 상태] [--file 경로|- 또는 --markdown 본문] / close|inspect [--id ID] / list".into()) };
    let mut request = Request {
        action,
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
    let stream = match ipc::connect() {
        Ok(stream) => stream,
        Err(_) if matches!(request.action, Action::Open) => {
            let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
            child
                .arg("--serve")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            // Detach the GUI from short-lived agent/terminal command sessions.
            // SAFETY: setsid is async-signal-safe and touches no Rust state after fork.
            unsafe {
                child.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            child.spawn().map_err(|e| e.to_string())?;
            let start = Instant::now();
            loop {
                if let Ok(stream) = ipc::connect() {
                    break stream;
                }
                if start.elapsed() > Duration::from_secs(15) {
                    return Err("앱 시작에 실패했습니다. floatnote --serve로 진단하세요".into());
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
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = if args.is_empty() || args == ["--serve"] {
        match ipc::Endpoint::bind() {
            Ok(Some(endpoint)) => serve(endpoint, args.is_empty()).map(|_| json!({"ok":true})),
            Ok(None) if args.is_empty() => cli(&["open".into()]),
            Ok(None) => Ok(json!({"ok":true,"already_running":true})),
            Err(e) => Err(e),
        }
    } else if args == ["--help"] || args == ["-h"] {
        Ok(
            json!({"ok":true,"usage":"floatnote open|update [--id ID] [--color #RRGGBB] [--title TEXT] [--status TEXT] [--file PATH|- | --markdown TEXT]; floatnote close|inspect [--id ID]; floatnote list","note":"open만 앱을 시작하거나 창을 생성·복원합니다. ID 생략 시 main, close는 숨기기입니다. 색상은 #RRGGBB입니다."}),
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
}
