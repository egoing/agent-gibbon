// 메뉴 막대에서 보이는 창을 앞으로 가져오거나 숨긴 창을 개별 복원한다.
use crate::{
    apply_direct,
    model::{Action, Request},
    AppNotes,
};
use tauri::{
    menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

type TrayMenu = std::sync::Mutex<Option<Menu<tauri::Wry>>>;

const RECENT_LIMIT: usize = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
struct MemoMenuEntry {
    id: String,
    title: String,
    kind: crate::presentation::Kind,
    updated: u64,
}
impl MemoMenuEntry {
    fn event_id(&self) -> String {
        format!(
            "{}:{}",
            if self.kind.is_terminal() {
                "agent"
            } else {
                "note"
            },
            self.id
        )
    }
    fn window_label(&self) -> String {
        if self.kind.is_terminal() {
            self.id.clone()
        } else {
            crate::label(&self.id)
        }
    }
    fn icon(&self) -> &'static [u8] {
        use crate::presentation::Kind;
        match self.kind {
            Kind::Note => include_bytes!("../icons/tabler/file-text.png"),
            Kind::Capture => include_bytes!("../icons/tabler/photo.png"),
            Kind::Terminal => include_bytes!("../icons/tabler/terminal-2.png"),
        }
    }
}

// Copy only owned metadata, then release all data locks before native calls.
fn memo_entries(notes: &AppNotes) -> Result<Vec<MemoMenuEntry>, String> {
    let notes = notes.lock().map_err(|e| e.to_string())?;
    Ok(notes
        .iter()
        .map(|(id, note)| {
            let kind = if note.capture.is_some() {
                crate::presentation::Kind::Capture
            } else {
                crate::presentation::Kind::Note
            };
            MemoMenuEntry {
                id: id.clone(),
                title: crate::presentation::list_title(&note.title, id, kind),
                kind,
                updated: note.updated_at.max(creation_ms(id)),
            }
        })
        .collect())
}
fn entries(app: &tauri::AppHandle) -> Result<Vec<MemoMenuEntry>, String> {
    let mut entries = memo_entries(&app.state::<AppNotes>())?;
    // entries() releases Terminals before any menu/window API is used.
    entries.extend(crate::terminal::entries(app).into_iter().map(|agent| {
        let kind = crate::presentation::Kind::Terminal;
        MemoMenuEntry {
            title: crate::presentation::list_title(&agent.title, &agent.id, kind),
            updated: agent
                .updated_at
                .max(agent.path_updated)
                .max(creation_ms(&agent.id)),
            id: agent.id,
            kind,
        }
    }));
    Ok(entries)
}
// Generated region/terminal IDs encode creation time in nanoseconds. This also
// covers legacy captures whose persisted updated_at is zero; other IDs are opaque.
fn creation_ms(id: &str) -> u64 {
    id.strip_prefix("terminal-")
        .or_else(|| id.strip_prefix("image-"))
        .or_else(|| id.strip_prefix("text-"))
        .and_then(|stamp| stamp.parse::<u128>().ok())
        .and_then(|stamp| u64::try_from(stamp / 1_000_000).ok())
        .unwrap_or(0)
}
fn recent(mut entries: Vec<MemoMenuEntry>) -> (Vec<MemoMenuEntry>, usize) {
    entries.sort_by(|a, b| {
        b.updated
            .cmp(&a.updated)
            .then_with(|| a.event_id().cmp(&b.event_id()))
    });
    let remaining = entries.len().saturating_sub(RECENT_LIMIT);
    entries.truncate(RECENT_LIMIT);
    (entries, remaining)
}
#[cfg(any(target_os = "macos", test))]
fn event_window_label(event: &str) -> Option<String> {
    event
        .strip_prefix("note:")
        .map(crate::label)
        .or_else(|| event.strip_prefix("agent:").map(str::to_owned))
}

fn menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    if let Some(branch) = &crate::instance::current().branch {
        menu.append(
            &MenuItem::with_id(app, "dev-instance", branch, false, None::<&str>)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    menu.append(
        &MenuItem::with_id(app, "open-manager", "메모 관리", true, None::<&str>)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    menu.append(
        &MenuItem::with_id(
            app,
            "capture-image",
            format!(
                "영역 캡처\t{}",
                crate::presentation::tray_shortcut("Ctrl+Shift+1")
            ),
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
            format!(
                "영역 텍스트 메모\t{}",
                crate::presentation::tray_shortcut("Ctrl+Shift+2")
            ),
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    menu.append(
        &MenuItem::with_id(
            app,
            "capture-terminal",
            format!(
                "영역 터미널\t{}",
                crate::presentation::tray_shortcut("Ctrl+Shift+3")
            ),
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    menu.append(
        &MenuItem::with_id(
            app,
            "stash-toggle",
            format!(
                "숨김\t{}",
                crate::presentation::tray_shortcut(&crate::stash::shortcut(app))
            ),
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let (entries, remaining) = recent(entries(app)?);
    for entry in entries {
        let visible = app
            .get_webview_window(&entry.window_label())
            .is_some_and(|w| w.is_visible().unwrap_or(false));
        let item = IconMenuItem::with_id(
            app,
            entry.event_id(),
            &entry.title,
            true,
            Some(memo_icon(entry.icon(), visible)?),
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;
        menu.append(&item).map_err(|e| e.to_string())?;
    }
    if remaining > 0 {
        menu.append(
            &MenuItem::with_id(
                app,
                "more-notes",
                format!("더보기({remaining})"),
                true,
                None::<&str>,
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
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
        // Drop the old native menu only after releasing TrayMenu as well.
        let previous = app
            .state::<TrayMenu>()
            .lock()
            .map_err(|e| e.to_string())?
            .replace(menu);
        drop(previous);
    }
    Ok(())
}

// Update marks in place so background work never dismisses an open menu.
pub fn sync_visibility(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<TrayMenu>() else {
        return Ok(());
    };
    let menu = state.lock().map_err(|e| e.to_string())?.clone();
    let entries = entries(app)?;
    if let Some(menu) = menu.as_ref() {
        for item in menu.items().map_err(|e| e.to_string())? {
            if let Some(check) = item.as_icon_menuitem() {
                if let Some(entry) = entries
                    .iter()
                    .find(|entry| entry.event_id() == check.id().as_ref())
                {
                    let visible = app
                        .get_webview_window(&entry.window_label())
                        .is_some_and(|w| w.is_visible().unwrap_or(false));
                    check
                        .set_icon(Some(memo_icon(entry.icon(), visible)?))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    if let Some(menu) = menu.as_ref() {
        template_icons(app, menu)?;
    }
    Ok(())
}

fn memo_icon(bytes: &[u8], visible: bool) -> Result<tauri::image::Image<'static>, String> {
    let image = tauri::image::Image::from_bytes(bytes).map_err(|e| e.to_string())?;
    let mut rgba = image.rgba().to_vec();
    if !visible {
        for pixel in rgba.as_chunks_mut::<4>().0 {
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
            .map(|item| event_window_label(item.id().as_ref()))
            .collect::<Vec<_>>();
        let shortcut_rows = menu
            .items()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|item| {
                matches!(
                    item.id().as_ref(),
                    "capture-image" | "capture-text" | "capture-terminal" | "stash-toggle"
                )
            })
            .collect::<Vec<_>>();
        let app_handle = app.clone();
        let menu = menu.clone();
        app.run_on_main_thread(move || {
            fn native<T: tauri::menu::ContextMenu>(menu: &T) -> *mut std::ffi::c_void {
                menu.inner_context().ns_menu()
            }
            unsafe fn align_shortcut(item: *mut objc2::runtime::AnyObject) {
                use objc2::{class, msg_send, runtime::AnyObject};
                #[repr(C)] struct Range { location: usize, length: usize }
                unsafe impl objc2::encode::Encode for Range {
                    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct("_NSRange", &[<usize as objc2::encode::Encode>::ENCODING, <usize as objc2::encode::Encode>::ENCODING]);
                }
                let title: *mut AnyObject = msg_send![item, title];
                let length: usize = msg_send![title, length];
                let tab: *mut AnyObject = msg_send![class!(NSTextTab), alloc];
                let options: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
                let tab: *mut AnyObject = msg_send![tab, initWithTextAlignment: 2isize, location: 310f64, options: options];
                let tabs: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: tab];
                let paragraph: *mut AnyObject = msg_send![class!(NSMutableParagraphStyle), alloc];
                let paragraph: *mut AnyObject = msg_send![paragraph, init];
                let _: () = msg_send![paragraph, setTabStops: tabs];
                let key: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c"NSParagraphStyle".as_ptr()];
                let attributed: *mut AnyObject = msg_send![class!(NSMutableAttributedString), alloc];
                let attributed: *mut AnyObject = msg_send![attributed, initWithString: title];
                let _: () = msg_send![attributed, addAttribute: key, value: paragraph, range: Range { location: 0, length }];
                // De-emphasize only the shortcut column; keep the action label
                // and all input/accelerator contracts untouched.
                let utf8: *const std::ffi::c_char = msg_send![title, UTF8String];
                if !utf8.is_null() {
                    let text = std::ffi::CStr::from_ptr(utf8).to_string_lossy();
                    if let Some((prefix, _)) = text.split_once('\t') {
                        let start = prefix.encode_utf16().count() + 1;
                        let color: *mut AnyObject = msg_send![class!(NSColor), labelColor];
                        let color: *mut AnyObject = msg_send![color, colorWithAlphaComponent: 0.55f64];
                        let color_key: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c"NSColor".as_ptr()];
                        let _: () = msg_send![attributed, addAttribute: color_key, value: color, range: Range { location: start, length: length - start }];
                    }
                }
                let _: () = msg_send![item, setAttributedTitle: attributed];
                let _: () = msg_send![attributed, release];
                let _: () = msg_send![paragraph, release];
                let _: () = msg_send![tab, release];
            }
            // The menu owns these objects and all AppKit calls run on the main thread.
            unsafe {
                use objc2::{class, msg_send, runtime::AnyObject};
                let raw = native(&menu) as *mut AnyObject;
                let count: isize = msg_send![raw, numberOfItems];
                for index in 0..count {
                    let item: *mut AnyObject = msg_send![raw, itemAtIndex: index];
                    if let Some(Some(id)) = ids.get(index as usize) {
                        let visible = app_handle.get_webview_window(id).is_some_and(|win| win.is_visible().unwrap_or(false));
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
                    if shortcut_rows.get(index as usize) == Some(&true) { align_shortcut(item); }
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
        .on_tray_icon_event(|tray, event| {
            if matches!(event, tauri::tray::TrayIconEvent::Enter { .. }) {
                if let Err(error) = refresh(tray.app_handle()) {
                    eprintln!("메뉴 열기 전 갱신 실패: {error}");
                }
            }
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit-app" {
                app.exit(0);
                return;
            }
            let result = if event.id.as_ref() == "more-notes" {
                crate::manager::open_list(app)
            } else if event.id.as_ref() == "open-manager" {
                crate::manager::open(app)
            } else if event.id.as_ref() == "stash-toggle" {
                crate::stash::trigger(app);
                Ok(())
            } else if event.id.as_ref() == "capture-image" {
                crate::capture::trigger(app, crate::capture::SelectionMode::Image);
                Ok(())
            } else if event.id.as_ref() == "capture-text" {
                crate::capture::trigger(app, crate::capture::SelectionMode::Text);
                Ok(())
            } else if event.id.as_ref() == "capture-terminal" {
                crate::capture::trigger(app, crate::capture::SelectionMode::Terminal);
                Ok(())
            } else if let Some(id) = event.id.as_ref().strip_prefix("agent:") {
                crate::terminal::open(app, id)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_menu_wait_allows_ui_snapshot_to_acquire_notes() {
        use std::{
            collections::BTreeMap,
            sync::{mpsc, Arc},
            time::Duration,
        };
        let notes = Arc::new(AppNotes::new(BTreeMap::from([(
            "capture".into(),
            crate::Note::default(),
        )])));
        let worker_notes = notes.clone();
        let (queued, ui_request) = mpsc::channel();
        let (ack, ui_reply) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let entries = memo_entries(&worker_notes).unwrap();
            // Model the blocking set_icon call: its worker waits for the UI,
            // while the UI must first process an incoming snapshot.
            queued.send(()).unwrap();
            let serviced = ui_reply.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(entries[0].id, "capture");
            serviced
        });
        ui_request.recv_timeout(Duration::from_secs(2)).unwrap();
        let snapshot_available = notes.try_lock().is_ok();
        ack.send(snapshot_available).unwrap();
        assert!(
            worker.join().unwrap(),
            "native menu update held AppNotes while waiting for UI snapshot"
        );
    }

    #[test]
    fn menu_metadata_is_owned_and_does_not_copy_memo_content() {
        use std::collections::BTreeMap;
        let note = crate::Note {
            title: "가".repeat(40),
            markdown: "private body".into(),
            ..crate::Note::default()
        };
        let notes = AppNotes::new(BTreeMap::from([("a".into(), note)]));
        let entries = memo_entries(&notes).unwrap();
        notes.lock().unwrap().clear();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "가".repeat(30));
        assert_eq!(entries[0].kind, crate::presentation::Kind::Note);
    }
    fn entry(id: &str, kind: crate::presentation::Kind, updated: u64) -> MemoMenuEntry {
        MemoMenuEntry {
            id: id.into(),
            title: "제목".into(),
            kind,
            updated,
        }
    }
    #[test]
    fn recent_boundary_zero_thirty_thirty_one_and_mixed_ties() {
        use crate::presentation::Kind::*;
        assert_eq!(recent(vec![]), (vec![], 0));
        let all = (0..31)
            .map(|i| entry(&format!("{i:02}"), [Note, Capture, Terminal][i % 3], 100))
            .collect::<Vec<_>>();
        let (thirty, remaining) = recent(all[..30].to_vec());
        assert_eq!(thirty.len(), 30);
        assert_eq!(remaining, 0);
        let (a, remaining) = recent(all.clone());
        let (b, _) = recent(all.into_iter().rev().collect());
        assert_eq!(a, b);
        assert_eq!(a.len(), 30);
        assert_eq!(remaining, 1);
        assert!(a
            .windows(2)
            .all(|pair| pair[0].event_id() < pair[1].event_id()));
        assert!(a.iter().any(|e| e.kind == Terminal));
    }
    #[test]
    fn content_updates_reorder_and_legacy_time_is_compatible() {
        use crate::presentation::Kind::*;
        let a = entry("note", Note, 1);
        let mut b = entry(
            "terminal-2000000",
            Terminal,
            creation_ms("terminal-2000000"),
        );
        assert_eq!(recent(vec![a.clone(), b.clone()]).0[0].id, b.id);
        b.updated = 0;
        assert_eq!(recent(vec![b, a]).0[0].id, "note");
        for id in ["legacy", "agent-invalid", "terminal-", "note-99999"] {
            assert_eq!(creation_ms(id), 0);
        }
        assert_eq!(creation_ms("terminal-1793960823000000000"), 1793960823000);
    }
    #[test]
    fn legacy_region_notes_with_zero_update_time_use_creation_fallback() {
        use std::collections::BTreeMap;
        let notes = AppNotes::new(BTreeMap::from([
            ("image-1793960823000000000".into(), crate::Note::default()),
            ("text-1793960824000000000".into(), crate::Note::default()),
        ]));
        let (entries, _) = recent(memo_entries(&notes).unwrap());
        assert_eq!(entries[0].id, "text-1793960824000000000");
        assert_eq!(entries[1].updated, 1793960823000);
        assert!(notes
            .lock()
            .unwrap()
            .values()
            .all(|note| note.updated_at == 0));
    }
    #[test]
    fn all_kinds_have_icons_event_routing_and_visibility_alpha() {
        use crate::presentation::Kind::*;
        for kind in [Note, Capture, Terminal] {
            let entry = entry("internal", kind, 0);
            assert_eq!(
                event_window_label(&entry.event_id()),
                Some(entry.window_label())
            );
            let shown = memo_icon(entry.icon(), true).unwrap();
            let hidden = memo_icon(entry.icon(), false).unwrap();
            assert!(shown.rgba().as_chunks::<4>().0.iter().any(|p| p[3] > 0));
            for (a, b) in shown
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .zip(hidden.rgba().as_chunks::<4>().0.iter())
            {
                assert_eq!(b[3], (a[3] as f32 * 0.45) as u8);
            }
        }
    }
}
