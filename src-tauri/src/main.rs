mod ipc;
mod model;
use model::{Action, Note, Request, MAX_REQUEST};
use serde_json::{json, Value};
use std::os::unix::process::CommandExt;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, State};

type AppNote = Mutex<Note>;
fn window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or("메모 창을 찾을 수 없습니다".into())
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
            (&*(ptr as *const objc2_app_kit::NSWindow)).orderFrontRegardless();
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    win.show().map_err(|e| e.to_string())
}
#[tauri::command]
fn snapshot(app: tauri::AppHandle, note: State<AppNote>) -> Result<Value, String> {
    let win = window(&app)?;
    let position = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    Ok(
        json!({"ok":true,"note":*note.lock().map_err(|e| e.to_string())?,"visible":win.is_visible().map_err(|e| e.to_string())?,"focused":win.is_focused().map_err(|e| e.to_string())?,"always_on_top":win.is_always_on_top().map_err(|e| e.to_string())?,"position":{"x":position.x,"y":position.y},"size":{"width":size.width,"height":size.height}}),
    )
}
#[tauri::command]
fn hide_note(app: tauri::AppHandle) -> Result<(), String> {
    window(&app)?.hide().map_err(|e| e.to_string())
}
#[tauri::command]
fn toggle_pin(app: tauri::AppHandle) -> Result<(), String> {
    let win = window(&app)?;
    win.set_always_on_top(!win.is_always_on_top().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn apply(app: &tauri::AppHandle, request: Request) -> Result<Value, String> {
    let state = app.state::<AppNote>();
    // Validate and prepare before touching native window state.
    let mut next = state.lock().map_err(|e| e.to_string())?.clone();
    next.apply(&request)?;
    let win = window(app)?;
    match request.action {
        Action::Open => {
            show_without_focus(&win)?;
        }
        Action::Close => win.hide().map_err(|e| e.to_string())?,
        Action::Update | Action::Inspect => (),
    }
    *state.lock().map_err(|e| e.to_string())? = next;
    snapshot(app.clone(), state)
}
fn serve(endpoint: ipc::Endpoint, initial_open: bool) -> Result<(), String> {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(false);
    let app = builder
        .manage(Mutex::new(Note::default()))
        .invoke_handler(tauri::generate_handler![snapshot, hide_note, toggle_pin])
        .on_window_event(|win, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win.hide();
            }
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if initial_open {
                show_without_focus(&window(app.handle())?)?;
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
    let action = match args.first().map(String::as_str) { Some("open") => Action::Open, Some("update") => Action::Update, Some("close") => Action::Close, Some("inspect") => Action::Inspect, _ => return Err("사용법: floatnote open|update [--title 제목] [--status 상태] [--file 경로|- 또는 --markdown 본문] / close / inspect".into()) };
    let mut request = Request {
        action,
        title: None,
        status: None,
        markdown: None,
    };
    let mut i = 1;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).ok_or("옵션 값이 없습니다")?;
        match key.as_str() {
            "--title" if request.title.is_none() => request.title = Some(value.clone()),
            "--status" if request.status.is_none() => request.status = Some(value.clone()),
            "--markdown" if request.markdown.is_none() => request.markdown = Some(value.clone()),
            "--file" if request.markdown.is_none() => {
                let input: Box<dyn Read> = if value == "-" {
                    Box::new(std::io::stdin())
                } else {
                    Box::new(std::fs::File::open(value).map_err(|e| e.to_string())?)
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
    Note::default().apply(&request)?;
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
    ipc::exchange(stream, &request)
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
            json!({"ok":true,"usage":"floatnote open|update [--title TEXT] [--status TEXT] [--file PATH|- | --markdown TEXT]; floatnote close|inspect","note":"open만 앱을 시작하거나 창을 복원합니다. close는 숨기기입니다."}),
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
}
