//! Each visit owns its ordinary desktop snapshot. The last overview layout lives in memory.
use super::{arrangement::Arrangement, *};
use crate::monitor_layout::{self, RecoveryPoint};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use tauri::{WebviewUrl, WebviewWindowBuilder};

const BACKDROP: &str = "memo-backdrop";
const CONTROLS: &str = "memo-overview";
const BAR_WIDTH: f64 = 180.;
const BAR_HEIGHT: f64 = 68.;

#[derive(Clone)]
struct Original {
    host: Arc<HostIdentity>,
    frame: Destination,
    excluded: bool,
    recovery: RecoveryPoint,
}

// A recreated host may reuse the same label. Its WebView resource table is new.
struct HostIdentity;
impl tauri::Resource for HostIdentity {}

fn host_identity(window: &WebviewWindow) -> Arc<HostIdentity> {
    let mut resources = window.resources_table();
    let existing = resources
        .names()
        .find(|(_, name)| name == std::any::type_name::<HostIdentity>())
        .map(|(id, _)| id);
    if let Some(host) = existing.and_then(|id| resources.get::<HostIdentity>(id).ok()) {
        return host;
    }
    let host = Arc::new(HostIdentity);
    resources.add_arc(host.clone());
    host
}

#[derive(Clone)]
struct RememberedLayout {
    work: Destination,
    screens: Vec<tauri::Monitor>,
    frames: BTreeMap<String, (Arc<HostIdentity>, Destination)>,
}

fn same_screen(a: &tauri::Monitor, b: &tauri::Monitor) -> bool {
    a.name() == b.name()
        && a.position() == b.position()
        && a.size() == b.size()
        && a.work_area().position == b.work_area().position
        && a.work_area().size == b.work_area().size
        && a.scale_factor() == b.scale_factor()
}

impl RememberedLayout {
    fn compatible(&self, monitor: &tauri::Monitor, screens: &[tauri::Monitor]) -> bool {
        self.work.position == monitor.work_area().position
            && self.work.size == monitor.work_area().size
            && self.work.scale == monitor.scale_factor()
            && self.screens.len() == screens.len()
            && self
                .screens
                .iter()
                .all(|old| screens.iter().any(|now| same_screen(old, now)))
    }

    fn frame(&self, label: &str, host: &Arc<HostIdentity>) -> Option<Destination> {
        let (previous, frame) = self.frames.get(label)?;
        Arc::ptr_eq(previous, host).then_some(*frame)
    }
}

#[derive(Clone)]
struct CapturePause {
    generation: u64,
    frame: Option<String>,
}

#[derive(Clone)]
struct Session {
    id: u32,
    paused_capture: Option<CapturePause>,
    work: Destination,
    originals: BTreeMap<String, Original>,
}

#[derive(Default)]
pub(crate) struct Overview {
    session: Mutex<Option<Session>>,
    remembered: Mutex<Option<RememberedLayout>>,
    sequence: AtomicU32,
}

impl Overview {
    fn current(&self, generation: u32) -> Option<Session> {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .filter(|session| session.id == generation)
            .cloned()
    }

    fn finish(
        &self,
        generation: u32,
        restore: impl FnOnce(&Session) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(session) = self.current(generation) {
            restore(&session)?;
            self.session.lock().unwrap().take_if(|s| s.id == generation);
        }
        Ok(())
    }
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub(crate) fn install(app: &tauri::AppHandle) -> Result<(), String> {
    app.manage(Overview::default());
    for (label, title) in [(BACKDROP, "메모 배경"), (CONTROLS, "메모 모아보기")] {
        crate::focus::remember(app, label);
        let builder =
            WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("{label}.html").into()))
                .title(format!("{} — {title}", crate::instance::current().title()))
                .inner_size(BAR_WIDTH, BAR_HEIGHT)
                .visible(false)
                .focused(false)
                .focusable(label == CONTROLS)
                .accept_first_mouse(true)
                .always_on_top(label == CONTROLS)
                .skip_taskbar(true)
                .transparent(true)
                .shadow(false)
                .decorations(false)
                .resizable(false);
        let builder = if label == BACKDROP {
            builder
                .background_color(tauri::window::Color(0, 0, 0, 0))
                .on_page_load(|window, event| {
                    if event.event() == tauri::webview::PageLoadEvent::Finished {
                        let app = window.app_handle().clone();
                        let _ = window.clone().run_on_main_thread(move || {
                            // A new document starts transparent. Recover a show that targeted
                            // the previous document only while this visit is still visible.
                            if window.is_visible().unwrap_or(false) && active(&app) {
                                if let Err(e) = set_backdrop_visible(&window, true) {
                                    eprintln!("모아보기 배경 표시 실패: {e}");
                                }
                            }
                        });
                    }
                })
        } else {
            builder
        };
        let window = builder.build().map_err(error)?;
        #[cfg(any(target_os = "macos", windows))]
        crate::focus::restore(&window);
        let app = app.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(e) = leave(&app) {
                    eprintln!("메모 보기 종료 실패: {e}");
                }
            }
        });
    }
    Ok(())
}

fn utility(app: &tauri::AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label)
        .ok_or_else(|| format!("{label} 창을 찾을 수 없습니다"))
}

pub(crate) fn active(app: &tauri::AppHandle) -> bool {
    app.try_state::<Overview>()
        .is_some_and(|state| state.session.lock().unwrap().is_some())
}

pub(crate) fn protects(window: &WebviewWindow) -> bool {
    window.try_state::<Overview>().is_some_and(|state| {
        state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.originals.contains_key(window.label()))
    })
}

pub(super) fn forget(window: &WebviewWindow) {
    if let Some(state) = window.try_state::<Overview>() {
        if let Some(s) = state.session.lock().unwrap().as_mut() {
            s.originals.remove(window.label());
        }
    }
}

pub(super) fn current_frame(w: &WebviewWindow) -> Result<Destination, String> {
    Ok(Destination {
        position: w.outer_position().map_err(error)?,
        size: w.outer_size().map_err(error)?,
        scale: w.scale_factor().map_err(error)?,
    })
}

pub(crate) fn enter(app: &tauri::AppHandle, monitor: tauri::Monitor) -> Result<(), String> {
    let state = app.state::<Overview>();
    if crate::capture::update_blocked(app) || state.session.lock().unwrap().is_some() {
        return Ok(());
    }
    let windows = visible_hosts(app)?;
    if windows.is_empty() {
        return Ok(());
    }
    let backdrop = utility(app, BACKDROP)?;
    let controls = utility(app, CONTROLS)?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let screens = backdrop.available_monitors().unwrap_or_default();
    let previous = state.remembered.lock().unwrap().clone();
    let remembered = previous
        .as_ref()
        .filter(|layout| layout.compatible(&monitor, &screens));
    let mut originals = BTreeMap::new();
    for window in &windows {
        let host = host_identity(window);
        // Immediate re-entry can interrupt the last close while it is still restoring.
        if previous
            .as_ref()
            .and_then(|layout| layout.frame(window.label(), &host))
            .is_some()
        {
            entrance::finish(window)?;
        }
        originals.insert(
            window.label().to_owned(),
            Original {
                host,
                frame: current_frame(window)?,
                excluded: excluded(window),
                recovery: monitor_layout::recovery_point(window),
            },
        );
    }
    let id = state
        .sequence
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    let mut repositioned = Vec::new();
    let show = (|| -> Result<(), String> {
        entrance::place(
            &backdrop,
            Destination {
                position: work.position,
                size: work.size,
                scale,
            },
        )?;
        controls
            .set_position(tauri::PhysicalPosition::new(
                (f64::from(work.position.x) + (f64::from(work.size.width) - BAR_WIDTH * scale) / 2.)
                    .round() as i32,
                (f64::from(work.position.y) + f64::from(work.size.height)
                    - (BAR_HEIGHT + 16.) * scale)
                    .round() as i32,
            ))
            .map_err(error)?;
        controls
            .eval(format!(
                "window.overviewGeneration = {id}; window.beginOverview?.({id})"
            ))
            .map_err(error)?;
        if let Some(panel) = app.get_webview_window("layout-undo") {
            panel.hide().map_err(error)?;
        }
        app.state::<Undo>().0.lock().unwrap().take();
        crate::stash::show_frame(&backdrop)?;

        for window in &windows {
            entrance::cancel(window)?;
            crate::sizing::cancel(window);
            let original = &originals[window.label()];
            if let Some(frame) =
                remembered.and_then(|layout| layout.frame(window.label(), &original.host))
            {
                repositioned.push((window, original.frame));
                entrance::place(window, frame)?;
            }
            crate::stash::show_frame(window)?;
            crate::opacity::set_native(window, 1.0)?;
        }
        crate::stash::show_frame(&controls)
    })();
    if let Err(e) = show {
        let _ = controls.hide();
        let _ = hide_backdrop(&backdrop);
        for (window, frame) in repositioned {
            if let Err(rollback) = entrance::place(window, frame) {
                eprintln!("모아보기 진입 위치 복구 실패: {rollback}");
            }
        }
        return Err(e);
    }
    *state.session.lock().unwrap() = Some(Session {
        id,
        paused_capture: None,
        work: Destination {
            position: work.position,
            size: work.size,
            scale,
        },
        originals,
    });
    if let Err(e) = set_backdrop_visible(&backdrop, true) {
        finish_visit(app, id, false)?;
        return Err(e);
    }
    crate::relation_prototype::focus_changed();
    // Clear pending idle fades only after the visit owns every participating host.
    for window in windows {
        crate::sync_opacity(window)?;
    }
    Ok(())
}

fn arrange_session(app: &tauri::AppHandle, session: &Session) -> Result<(), String> {
    let windows = visible_hosts(app)?
        .into_iter()
        .filter(|w| session.originals.contains_key(w.label()))
        .collect::<Vec<_>>();
    let work = session.work;
    let scale = work.scale;
    let mut area = native_rect(work.position, work.size, scale);
    area.height -= ((BAR_HEIGHT + 16.) * scale / unit(scale)).ceil() as i64;
    let frames = planned_frames(&windows, area, scale, Arrangement::Grid)?;
    let before = windows
        .iter()
        .map(current_frame)
        .collect::<Result<Vec<_>, _>>()?;
    if let Err(e) = move_hosts(app, &windows, frames, scale, work.position.y) {
        for (window, frame) in windows.iter().zip(before) {
            entrance::cancel(window)?;
            entrance::place(window, frame)?;
        }
        return Err(e);
    }
    Ok(())
}

fn set_backdrop_visible(backdrop: &WebviewWindow, visible: bool) -> Result<(), String> {
    backdrop
        .eval(format!(
            "document.documentElement.toggleAttribute('data-visible', {visible})"
        ))
        .map_err(error)
}

fn hide_backdrop(backdrop: &WebviewWindow) -> Result<(), String> {
    backdrop.hide().map_err(error)?;
    set_backdrop_visible(backdrop, false)
}

fn close(app: &tauri::AppHandle, session: &Session, animate: bool) -> Result<(), String> {
    for (label, original) in &session.originals {
        let Some(w) = crate::web::managed_window(app, label) else {
            continue;
        };
        if !Arc::ptr_eq(&host_identity(&w), &original.host) {
            continue;
        }
        entrance::cancel(&w)?;
        crate::sizing::cancel(&w);
        let live = current_frame(&w)?;
        if live.position != original.frame.position || live.size != original.frame.size {
            if animate {
                entrance::transition(&w, original.frame)?;
            } else {
                entrance::place(&w, original.frame)?;
            }
        }
        monitor_layout::restore_destination(&w, original.recovery.clone(), original.frame)?;
        w.state::<Exclusions>()
            .0
            .lock()
            .unwrap()
            .insert(label.clone(), original.excluded);
        if let Some(m) = w
            .monitor_from_point(
                original.frame.position.x as f64,
                original.frame.position.y as f64,
            )
            .map_err(error)?
        {
            crate::warp::remember(
                app,
                &w,
                original.frame.position.y.into(),
                m.work_area().position.y,
                original.frame.scale,
            )?;
        }
    }
    hide_backdrop(&utility(app, BACKDROP)?)?;
    utility(app, CONTROLS)?.hide().map_err(error)?;
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Action {
    Arrange,
    Close,
}

#[tauri::command]
pub(crate) fn overview_action(
    window: WebviewWindow,
    action: Action,
    generation: u32,
) -> Result<(), String> {
    if window.label() != CONTROLS {
        return Err("메모 모아보기에서만 실행할 수 있습니다".into());
    }
    let app = window.app_handle();
    let _activity = crate::updater::activity(app)?;
    let state = app.state::<Overview>();
    // A click queued just before capture hid the controls belongs to the
    // suspended visit. Keep its layout intact until selection has ended.
    if state
        .current(generation)
        .is_some_and(|s| s.paused_capture.is_some())
    {
        return Ok(());
    }
    match action {
        Action::Arrange => {
            if let Some(session) = state.current(generation) {
                arrange_session(app, &session)?;
            }
            Ok(())
        }
        Action::Close => finish_visit(app, generation, true),
    }
}

fn remember_layout(app: &tauri::AppHandle, session: &Session) -> Result<RememberedLayout, String> {
    let screens = utility(app, BACKDROP)?
        .available_monitors()
        .map_err(error)?;
    let mut frames = BTreeMap::new();
    for window in visible_hosts(app)? {
        let frame = current_frame(&window)?;
        let cx = i64::from(frame.position.x) + i64::from(frame.size.width) / 2;
        let cy = i64::from(frame.position.y) + i64::from(frame.size.height) / 2;
        if screens.iter().any(|screen| {
            let area = screen.work_area();
            let x = i64::from(area.position.x);
            let y = i64::from(area.position.y);
            (x..x + i64::from(area.size.width)).contains(&cx)
                && (y..y + i64::from(area.size.height)).contains(&cy)
                && screen.scale_factor() == frame.scale
        }) {
            frames.insert(window.label().to_owned(), (host_identity(&window), frame));
        }
    }
    Ok(RememberedLayout {
        work: session.work,
        screens,
        frames,
    })
}

fn finish_visit(app: &tauri::AppHandle, generation: u32, animate: bool) -> Result<(), String> {
    let state = app.state::<Overview>();
    let mut labels = Vec::new();
    state.finish(generation, |session| {
        let remembered = remember_layout(app, session);
        close(app, session, animate)?;
        match remembered {
            Ok(layout) => *state.remembered.lock().unwrap() = Some(layout),
            Err(e) => {
                state.remembered.lock().unwrap().take();
                eprintln!("모아보기 배치 기억 실패: {e}");
            }
        }
        labels.extend(session.originals.keys().cloned());
        Ok(())
    })?;
    crate::relation_prototype::focus_changed();
    for label in labels {
        if let Some(window) = crate::web::managed_window(app, &label) {
            crate::sync_opacity(window)?;
        }
    }
    Ok(())
}

/// Capture workers call this synchronously so screenshot acquisition starts only
/// after both native surfaces have been hidden. Native access stays on the UI thread.
pub(crate) fn capture_visibility(
    app: &tauri::AppHandle,
    generation: u64,
    suspend: bool,
) -> Result<(), String> {
    let Some(state) = app.try_state::<Overview>() else {
        return Ok(());
    };
    if state.session.lock().unwrap().is_none() {
        return Ok(());
    }
    let handle = app.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = sender.send(set_capture_visibility(&handle, generation, suspend));
    })
    .map_err(error)?;
    receiver.recv().map_err(error)?
}

fn set_capture_visibility(
    app: &tauri::AppHandle,
    generation: u64,
    suspend: bool,
) -> Result<(), String> {
    let state = app.state::<Overview>();
    {
        let mut guard = state.session.lock().unwrap();
        let Some(session) = guard.as_mut() else {
            return Ok(());
        };
        if suspend {
            session.paused_capture = Some(CapturePause {
                generation,
                frame: None,
            });
        } else if session.paused_capture.as_ref().map(|p| p.generation) != Some(generation) {
            return Ok(());
        }
    }
    let backdrop = utility(app, BACKDROP)?;
    let controls = utility(app, CONTROLS)?;
    if suspend {
        for window in [&controls, &backdrop] {
            #[cfg(target_os = "macos")]
            {
                // Match show_frame's native ordering. The screenshot is acquired
                // in this same main-thread turn, before queued UI events can run.
                let ptr = window.ns_window().map_err(error)?;
                unsafe { (&*(ptr as *const objc2_app_kit::NSWindow)).orderOut(None) };
            }
            #[cfg(not(target_os = "macos"))]
            window.hide().map_err(error)?;
            if window.is_visible().map_err(error)? {
                return Err("모아보기 창을 숨기지 못했습니다".into());
            }
        }
        return set_backdrop_visible(&backdrop, false);
    }
    crate::stash::show_frame(&backdrop)?;
    for window in visible_hosts(app)? {
        crate::stash::show_frame(&window)?;
    }
    crate::stash::show_frame(&controls)?;
    set_backdrop_visible(&backdrop, true)?;
    if let Some(session) = state.session.lock().unwrap().as_mut() {
        if session.paused_capture.as_ref().map(|p| p.generation) == Some(generation) {
            session.paused_capture = None;
        }
    }
    Ok(())
}

pub(crate) fn await_capture_frame(app: &tauri::AppHandle, generation: u64, label: &str) {
    if let Some(state) = app.try_state::<Overview>() {
        let mut guard = state.session.lock().unwrap();
        if let Some(pause) = guard.as_mut().and_then(|s| s.paused_capture.as_mut()) {
            if pause.generation == generation {
                pause.frame = Some(label.into());
            }
        }
    }
}

pub(crate) fn capture_frame_finished(app: &tauri::AppHandle, label: &str) {
    let generation = app.try_state::<Overview>().and_then(|state| {
        let guard = state.session.lock().unwrap();
        let pause = guard.as_ref()?.paused_capture.as_ref()?;
        (pause.frame.as_deref() == Some(label)).then_some(pause.generation)
    });
    if let Some(generation) = generation {
        let handle = app.clone();
        if let Err(e) = app.run_on_main_thread(move || {
            if let Err(e) = set_capture_visibility(&handle, generation, false) {
                eprintln!("모아보기 재표시 실패: {e}");
            }
        }) {
            eprintln!("모아보기 재표시 요청 실패: {e}");
        }
    }
}

/// Other explicit layout or stash commands leave overview before taking ownership.
pub(crate) fn leave(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<Overview>() else {
        return Ok(());
    };
    let id = state.session.lock().unwrap().as_ref().map(|s| s.id);
    if let Some(id) = id {
        finish_visit(app, id, false)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: i32, width: u32) -> Destination {
        Destination {
            position: tauri::PhysicalPosition::new(x, -37),
            size: tauri::PhysicalSize::new(width, 174),
            scale: 2.0,
        }
    }

    fn visit(id: u32, frame: Destination) -> Session {
        Session {
            id,
            paused_capture: None,
            work: frame,
            originals: BTreeMap::from([(
                "note-example".into(),
                Original {
                    host: Arc::new(HostIdentity),
                    frame,
                    excluded: true,
                    recovery: RecoveryPoint::default(),
                },
            )]),
        }
    }

    #[test]
    fn manual_frames_and_failed_close_do_not_replace_the_entry_snapshot() {
        let state = Overview::default();
        *state.session.lock().unwrap() = Some(visit(1, frame(-124, 381)));
        let mut live = frame(40, 280); // grid result
        live.position.x += 51; // user drag
        assert_eq!(live.position.x, 91);
        assert!(state.finish(1, |_| Err("창 접근 실패".into())).is_err());
        state
            .finish(1, |s| {
                live = s.originals["note-example"].frame;
                Ok(())
            })
            .unwrap();
        assert_eq!((live.position.x, live.size.width), (-124, 381));
        assert!(state.current(1).is_none());
        *state.session.lock().unwrap() = Some(visit(2, frame(252, 177)));
        state.finish(1, |_| panic!("이전 세션의 닫기")).unwrap();
        assert!(state.current(1).is_none());
        let snapshot = state.current(2).unwrap().originals["note-example"].frame;
        assert_eq!((snapshot.position.x, snapshot.size.width), (252, 177));
    }
}
