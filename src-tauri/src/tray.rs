// 메뉴 막대에서 보이는 창을 앞으로 가져오거나 숨긴 창을 개별 복원한다.
use crate::{
    apply,
    model::{Action, Request},
    show_without_focus, AppNotes,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
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
        let item = MenuItem::with_id(app, format!("note:{id}"), text, true, None::<&str>)
            .map_err(|e| e.to_string())?;
        menu.append(&item).map_err(|e| e.to_string())?;
    }
    Ok(menu)
}

pub fn refresh(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("floatnote") {
        tray.set_menu(Some(menu(app)?)).map_err(|e| e.to_string())?;
    }
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
    TrayIconBuilder::with_id("floatnote")
        .icon(icon())
        .icon_as_template(true)
        .tooltip("Floatnote — 메모 열기")
        .menu(&menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = if event.id.as_ref() == "bring-visible" {
                app.webview_windows().values().try_for_each(|win| {
                    if win.is_visible().map_err(|e| e.to_string())? {
                        show_without_focus(win)?;
                    }
                    Ok::<_, String>(())
                })
            } else if let Some(id) = event.id.as_ref().strip_prefix("note:") {
                apply(
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
            if let Err(error) = result {
                if let Some(tray) = app.tray_by_id("floatnote") {
                    let _ = tray.set_tooltip(Some(format!("Floatnote: {error}")));
                }
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;
    Ok(())
}
