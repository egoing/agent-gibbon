//! Transient, window-local attention. Monotonic deadlines avoid stale timers.
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub struct Feedbacks(Mutex<BTreeMap<String, Instant>>);

pub fn active(window: &WebviewWindow) -> bool {
    let state = window.state::<Feedbacks>();
    state
        .0
        .lock()
        .map(|deadlines| live(deadlines.get(window.label()).copied(), Instant::now()))
        .unwrap_or(false)
}
fn live(deadline: Option<Instant>, now: Instant) -> bool {
    deadline.is_some_and(|deadline| now < deadline)
}
pub fn clear(window: &WebviewWindow) {
    if let Ok(mut deadlines) = window.state::<Feedbacks>().0.lock() {
        deadlines.remove(window.label());
    }
}
pub fn changed(
    window: &WebviewWindow,
    before: Option<&crate::model::Note>,
    after: &crate::model::Note,
) -> Result<(), String> {
    if before.is_none_or(|note| note.revision == 0 || note.markdown == after.markdown)
        || !window.is_visible().map_err(|e| e.to_string())?
    {
        return Ok(());
    }
    let first = window
        .state::<Feedbacks>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            window.label().into(),
            Instant::now() + Duration::from_secs(3),
        )
        .is_none();
    if first {
        let window = window.clone();
        std::thread::spawn(move || {
            let mut wait = Duration::from_secs(3);
            loop {
                std::thread::sleep(wait);
                let target = window.clone();
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                if window
                    .run_on_main_thread(move || {
                        let next =
                            target
                                .state::<Feedbacks>()
                                .0
                                .lock()
                                .ok()
                                .and_then(|mut deadlines| {
                                    let deadline = deadlines.get(target.label()).copied()?;
                                    let remaining =
                                        deadline.saturating_duration_since(Instant::now());
                                    if remaining.is_zero() {
                                        deadlines.remove(target.label());
                                        None
                                    } else {
                                        Some(remaining)
                                    }
                                });
                        let _ = crate::sync_opacity(target);
                        let _ = tx.send(next);
                    })
                    .is_err()
                {
                    break;
                }
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(Some(next)) => wait = next,
                    _ => break,
                }
            }
        });
    }
    crate::show_without_focus(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadlines_restart_and_are_isolated_without_stale_callbacks() {
        let start = Instant::now();
        let mut deadlines = BTreeMap::new();
        deadlines.insert("a", start + Duration::from_secs(3));
        deadlines.insert("b", start + Duration::from_secs(3));
        deadlines.insert("a", start + Duration::from_secs(5));
        assert!(live(
            deadlines.get("a").copied(),
            start + Duration::from_secs(3)
        ));
        assert!(!live(
            deadlines.get("b").copied(),
            start + Duration::from_secs(3)
        ));
        assert!(!live(
            deadlines.get("a").copied(),
            start + Duration::from_secs(5)
        ));
        deadlines.remove("a");
        assert!(!live(deadlines.get("a").copied(), start));
    }
}
