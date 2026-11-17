//! Region windows wait for placement before reading geometry and theme. Only
//! the local document for that window can acknowledge its first layout.
use crate::capture::SelectionMode;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tauri::{
    async_runtime::{channel, Receiver, Sender},
    Manager, WebviewWindow,
};

#[derive(Default)]
pub struct Pending(Mutex<BTreeMap<String, Frame>>);

struct Frame {
    generation: Arc<()>,
    mode: SelectionMode,
    sender: Option<Sender<()>>,
    receiver: Option<Receiver<()>>,
}

impl Pending {
    fn register(&self, label: &str, mode: SelectionMode) {
        let (sender, receiver) = channel(1);
        self.0.lock().unwrap().insert(
            label.into(),
            Frame {
                generation: Arc::new(()),
                mode,
                sender: Some(sender),
                receiver: Some(receiver),
            },
        );
    }

    fn receiver(&self, label: &str) -> Option<Receiver<()>> {
        self.0.lock().unwrap().get_mut(label)?.receiver.take()
    }

    fn placed(&self, label: &str) -> Result<(), String> {
        let mut frames = self.0.lock().unwrap();
        if let Some(sender) = frames.get_mut(label).and_then(|frame| frame.sender.take()) {
            sender.try_send(()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn generation(&self, label: &str) -> Option<Arc<()>> {
        self.0
            .lock()
            .unwrap()
            .get(label)
            .map(|f| f.generation.clone())
    }

    fn take_ready(&self, label: &str, generation: &Arc<()>) -> Option<SelectionMode> {
        let mut frames = self.0.lock().unwrap();
        let frame = frames.get(label)?;
        if !Arc::ptr_eq(generation, &frame.generation)
            || frame.sender.is_some()
            || frame.receiver.is_some()
        {
            return None;
        }
        frames.remove(label).map(|frame| frame.mode)
    }

    fn cancel(&self, label: &str) {
        self.0.lock().unwrap().remove(label);
    }
}

// Mark only newly drawn transparent surfaces, before any document content paints.
pub fn prepare_builder<'a, M: Manager<tauri::Wry>>(
    builder: tauri::WebviewWindowBuilder<'a, tauri::Wry, M>,
    region: bool,
) -> tauri::WebviewWindowBuilder<'a, tauri::Wry, M> {
    if region {
        builder.initialization_script("window.gibbonPreparingRegion = true")
    } else {
        builder
    }
}

pub fn creating<T>(
    app: &tauri::AppHandle,
    label: &str,
    mode: SelectionMode,
    build: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    app.state::<Pending>().register(label, mode);
    let result = build();
    if result.is_err() {
        cancel(app, label);
    }
    result
}

pub fn cancel(app: &tauri::AppHandle, label: &str) {
    app.state::<Pending>().cancel(label);
    crate::layout::overview::capture_frame_finished(app, label);
}

pub fn placed(window: &WebviewWindow) -> Result<(), String> {
    window.state::<Pending>().placed(window.label())
}

#[tauri::command]
pub async fn region_frame_placement(window: WebviewWindow) -> Result<bool, String> {
    let receiver = window.state::<Pending>().receiver(window.label());
    if let Some(mut receiver) = receiver {
        return Ok(receiver.recv().await.is_some());
    }
    Ok(false)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn region_frame_ready(window: WebviewWindow) -> Result<(), String> {
    let Some(generation) = window.state::<Pending>().generation(window.label()) else {
        return Ok(());
    };
    show_prepared(&window, &generation)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn region_frame_ready(window: WebviewWindow) -> Result<(), String> {
    use block2::RcBlock;
    use objc2::{class, msg_send, rc::Retained, runtime::AnyObject};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let Some(generation) = window.state::<Pending>().generation(window.label()) else {
        return Ok(());
    };
    let (done, mut result) = channel(1);
    let target = window.clone();
    window
        .with_webview(move |platform| {
            // SAFETY: with_webview supplies the live WKWebView on the UI thread.
            // WebKit copies the completion block and calls it on the main thread.
            // Only the local host is sampled; the one-point image is discarded.
            unsafe {
                let config: Retained<AnyObject> = msg_send![class!(WKSnapshotConfiguration), new];
                let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(1., 1.));
                let _: () = msg_send![&*config, setRect: rect];
                let _: () = msg_send![&*config, setAfterScreenUpdates: objc2::runtime::Bool::YES];
                let completion =
                    RcBlock::new(move |image: *mut AnyObject, _error: *mut AnyObject| {
                        if image.is_null() {
                            // A snapshot error must still leave the prepared local UI reachable.
                            eprintln!("WebKit 첫 화면 갱신을 확인하지 못했습니다");
                        }
                        let _ = done.try_send(show_prepared(&target, &generation));
                    });
                let view = &*platform.inner().cast::<AnyObject>();
                let _: () = msg_send![view, takeSnapshotWithConfiguration: &*config,
                                        completionHandler: &*completion];
            }
        })
        .map_err(|e| {
            crate::layout::overview::capture_frame_finished(window.app_handle(), window.label());
            e.to_string()
        })?;
    result.recv().await.ok_or_else(|| {
        crate::layout::overview::capture_frame_finished(window.app_handle(), window.label());
        "첫 화면 준비가 중단되었습니다".to_string()
    })?
}

fn show_prepared(window: &WebviewWindow, generation: &Arc<()>) -> Result<(), String> {
    let Some(mode) = window
        .state::<Pending>()
        .take_ready(window.label(), generation)
    else {
        return Ok(());
    };
    match mode {
        SelectionMode::Text => crate::capture::show_text(window),
        SelectionMode::Image => {
            let shown = crate::show_without_focus(window);
            crate::layout::overview::capture_frame_finished(window.app_handle(), window.label());
            shown
        }
        SelectionMode::Terminal => crate::terminal::show(window),
        SelectionMode::Web => crate::web::show(window),
    }?;
    crate::tray::refresh(window.app_handle())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_orders_reveal_once_and_only_the_placed_window() {
        let frames = Pending::default();
        for placed_first in [true, false] {
            frames.register("new", SelectionMode::Image);
            let generation = frames.generation("new").unwrap();
            if placed_first {
                frames.placed("new").unwrap();
            }
            assert!(frames.take_ready("new", &generation).is_none());
            let mut placement = frames.receiver("new").unwrap();
            if !placed_first {
                assert!(frames.take_ready("new", &generation).is_none());
                frames.placed("new").unwrap();
            }
            assert!(placement.try_recv().is_ok());
            assert!(frames.take_ready("other", &generation).is_none());
            assert!(matches!(
                frames.take_ready("new", &generation),
                Some(SelectionMode::Image)
            ));
            assert!(frames.take_ready("new", &generation).is_none());
        }
    }

    #[test]
    fn hide_or_destroy_cancels_even_an_already_delivered_layout_request() {
        let frames = Pending::default();
        for placed in [false, true] {
            frames.register("new", SelectionMode::Text);
            let generation = frames.generation("new").unwrap();
            let mut placement = frames.receiver("new").unwrap();
            if placed {
                frames.placed("new").unwrap();
            }
            frames.cancel("new");
            assert_eq!(placement.try_recv().is_ok(), placed);
            assert!(frames.take_ready("new", &generation).is_none());
            assert!(frames.receiver("restored").is_none());
            frames.register("new", SelectionMode::Web);
            let mut next = frames.receiver("new").unwrap();
            frames.placed("new").unwrap();
            assert!(next.try_recv().is_ok());
            assert!(frames.take_ready("new", &generation).is_none());
            let current = frames.generation("new").unwrap();
            assert!(matches!(
                frames.take_ready("new", &current),
                Some(SelectionMode::Web)
            ));
        }
    }
}
