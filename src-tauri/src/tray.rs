// 메뉴 막대에서 보이는 창을 앞으로 가져오거나 숨긴 창을 개별 복원한다.
use crate::{
    apply_direct,
    model::{Action, Request},
    show_without_focus, AppNotes,
};
use tauri::{
    menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

type TrayMenu = std::sync::Mutex<Option<Menu<tauri::Wry>>>;

fn menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    if let Some(label) = &crate::instance::current().label {
        menu.append(
            &MenuItem::with_id(app, "dev-instance", label, false, None::<&str>)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    menu.append(
        &MenuItem::with_id(
            app,
            "capture-image",
            "영역 캡처 · Ctrl+Shift+1",
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    menu.append(
        &MenuItem::with_id(
            app,
            "capture-text",
            "영역 텍스트 메모 · Ctrl+Shift+2",
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let front = MenuItem::with_id(app, "bring-visible", "열린 메모 앞으로", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    menu.append(&front).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let state = app.state::<AppNotes>();
    let notes = state.lock().map_err(|e| e.to_string())?;
    for (id, note) in notes.iter() {
        let title: String = note.title.chars().take(30).collect();
        let text = if title.is_empty() {
            id.clone()
        } else {
            format!("{title} ({id})")
        };
        let visible = crate::window(app, id)?
            .is_visible()
            .map_err(|e| e.to_string())?;

        let bytes: &[u8] = if note.capture.is_some() {
            include_bytes!("../icons/tabler/photo.png")
        } else {
            include_bytes!("../icons/tabler/file-text.png")
        };
        let image = memo_icon(bytes, visible)?;
        let item = IconMenuItem::with_id(
            app,
            format!("note:{id}"),
            text,
            true,
            Some(image),
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;
        menu.append(&item).map_err(|e| e.to_string())?;
    }
    menu.append(&PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    menu.append(
        &MenuItem::with_id(app, "quit-app", "Agent Gibbon 종료", true, None::<&str>)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    template_icons(app, &menu)?;
    Ok(menu)
}

pub fn refresh(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("agent-gibbon") {
        let menu = menu(app)?;
        tray.set_menu(Some(menu.clone()))
            .map_err(|e| e.to_string())?;
        *app.state::<TrayMenu>().lock().map_err(|e| e.to_string())? = Some(menu);
    }
    Ok(())
}

// Update marks in place so background work never dismisses an open menu.
pub fn sync_visibility(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<TrayMenu>() else {
        return Ok(());
    };
    let guard = state.lock().map_err(|e| e.to_string())?;
    if let Some(menu) = guard.as_ref() {
        for item in menu.items().map_err(|e| e.to_string())? {
            if let Some(check) = item.as_icon_menuitem() {
                if let Some(id) = check.id().as_ref().strip_prefix("note:") {
                    let visible = crate::window(app, id)?
                        .is_visible()
                        .map_err(|e| e.to_string())?;
                    let notes = app.state::<AppNotes>();
                    let notes = notes.lock().map_err(|e| e.to_string())?;
                    if let Some(note) = notes.get(id) {
                        let bytes: &[u8] = if note.capture.is_some() {
                            include_bytes!("../icons/tabler/photo.png")
                        } else {
                            include_bytes!("../icons/tabler/file-text.png")
                        };
                        check
                            .set_icon(Some(memo_icon(bytes, visible)?))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
    if let Some(menu) = guard.as_ref() {
        template_icons(app, menu)?;
    }
    Ok(())
}

fn memo_icon(bytes: &[u8], visible: bool) -> Result<tauri::image::Image<'static>, String> {
    let image = tauri::image::Image::from_bytes(bytes).map_err(|e| e.to_string())?;
    let mut rgba = image.rgba().to_vec();
    if !visible {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = (pixel[3] as f32 * 0.45) as u8;
        }
    }
    Ok(tauri::image::Image::new_owned(
        rgba,
        image.width(),
        image.height(),
    ))
}

fn template_icons(app: &tauri::AppHandle, menu: &Menu<tauri::Wry>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let ids = menu
            .items()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|item| item.id().as_ref().strip_prefix("note:").map(str::to_owned))
            .collect::<Vec<_>>();
        let app_handle = app.clone();
        let menu = menu.clone();
        app.run_on_main_thread(move || {
            fn native<T: tauri::menu::ContextMenu>(menu: &T) -> *mut std::ffi::c_void {
                menu.inner_context().ns_menu()
            }
            // The menu owns these objects and all AppKit calls run on the main thread.
            unsafe {
                use objc2::{class, msg_send, runtime::AnyObject};
                let raw = native(&menu) as *mut AnyObject;
                let count: isize = msg_send![raw, numberOfItems];
                for index in 0..count {
                    let item: *mut AnyObject = msg_send![raw, itemAtIndex: index];
                    if let Some(Some(id)) = ids.get(index as usize) {
                        let Ok(visible) = crate::window(&app_handle, id).and_then(|win| win.is_visible().map_err(|e| e.to_string())) else { continue; };
                        if visible {
                            // Remove the faded attributed title entirely so AppKit restores
                            // its normal text color, including selection highlighting.
                            let _: () = msg_send![item, setAttributedTitle: std::ptr::null::<AnyObject>()];
                        } else {
                        let title: *mut AnyObject = msg_send![item, title];
                        let color: *mut AnyObject = msg_send![class!(NSColor), labelColor];
                        let color: *mut AnyObject = msg_send![color, colorWithAlphaComponent: 0.45f64];
                        let key: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c"NSColor".as_ptr()];
                        let attrs: *mut AnyObject = msg_send![class!(NSDictionary), dictionaryWithObject: color, forKey: key];
                        let attributed: *mut AnyObject = msg_send![class!(NSAttributedString), alloc];
                        let attributed: *mut AnyObject = msg_send![attributed, initWithString: title, attributes: attrs];
                        let _: () = msg_send![item, setAttributedTitle: attributed];
                        let _: () = msg_send![attributed, release];
                        }
                    }
                    let image: *mut AnyObject = msg_send![item, image];
                    if !image.is_null() {
                        let _: () = msg_send![image, setTemplate: true];
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, menu);
    Ok(())
}

fn icon() -> tauri::image::Image<'static> {
    // A crisp template note icon, 18 logical pixels at 2x density.
    let mut pixels = vec![0; 36 * 36 * 4];
    for y in 4..32 {
        for x in 6..30 {
            let border = !(8..28).contains(&x) || !(6..30).contains(&y);
            let line = (12..=24).contains(&x)
                && (y == 12 || y == 13 || y == 18 || y == 19 || y == 24 || y == 25);
            if border || line {
                pixels[(y * 36 + x) * 4 + 3] = 255;
            }
        }
    }
    tauri::image::Image::new_owned(pixels, 36, 36)
}

pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    let initial_menu = menu(app)?;
    app.manage(TrayMenu::new(Some(initial_menu.clone())));
    TrayIconBuilder::with_id("agent-gibbon")
        .icon(icon())
        .icon_as_template(true)
        .tooltip(format!(
            "{} — 메모 열기",
            crate::instance::current().title()
        ))
        .menu(&initial_menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit-app" {
                app.exit(0);
                return;
            }
            let result = if event.id.as_ref() == "capture-image" {
                crate::capture::trigger(app, crate::capture::SelectionMode::Image);
                Ok(())
            } else if event.id.as_ref() == "capture-text" {
                crate::capture::trigger(app, crate::capture::SelectionMode::Text);
                Ok(())
            } else if event.id.as_ref() == "bring-visible" {
                app.webview_windows().values().try_for_each(|win| {
                    if !crate::feedback_window::is_surface(win.label())
                        && win.is_visible().map_err(|e| e.to_string())?
                    {
                        crate::layout::mark(win, false);
                        show_without_focus(win)?;
                    }
                    Ok::<_, String>(())
                })
            } else if let Some(id) = event.id.as_ref().strip_prefix("note:") {
                apply_direct(
                    app,
                    Request {
                        source_path: None,
                        action: Action::Open,
                        id: Some(id.into()),
                        color: None,
                        title: None,
                        markdown: None,
                        status: None,
                    },
                )
                .map(|_| ())
            } else {
                Ok(())
            };
            // Visibility is the source of truth for the textual open-state mark.
            let result = result.and(sync_visibility(app));
            if let Err(error) = result {
                if let Some(tray) = app.tray_by_id("agent-gibbon") {
                    let _ = tray.set_tooltip(Some(format!("Agent Gibbon: {error}")));
                }
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;
    Ok(())
}
