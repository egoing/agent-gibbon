//! Transient, window-local attention. Monotonic deadlines avoid stale timers.
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub struct Feedbacks {
    deadlines: Mutex<BTreeMap<String, Instant>>,
    highlights: Mutex<BTreeMap<String, u64>>,
}

impl Feedbacks {
    pub fn restore(&self, notes: &BTreeMap<String, crate::model::Note>) -> Result<(), String> {
        let mut highlights = self.highlights.lock().map_err(|e| e.to_string())?;
        for (id, note) in notes {
            sync_highlight(&mut highlights, &crate::label(id), note);
        }
        Ok(())
    }
}

fn sync_highlight(highlights: &mut BTreeMap<String, u64>, label: &str, note: &crate::model::Note) {
    if note.has_pending_changes() {
        highlights.insert(label.into(), note.revision);
    } else {
        highlights.remove(label);
    }
}

pub fn highlight_revision(window: &WebviewWindow) -> Option<u64> {
    window
        .state::<Feedbacks>()
        .highlights
        .lock()
        .ok()?
        .get(window.label())
        .copied()
}

fn acknowledge(highlights: &mut BTreeMap<String, u64>, label: &str, revision: u64) -> bool {
    if highlights.get(label) != Some(&revision) {
        return false;
    }
    highlights.remove(label);
    true
}

#[tauri::command]
pub fn acknowledge_feedback(window: WebviewWindow, revision: u64) -> Result<bool, String> {
    // Keep the same notes -> feedback lock order as snapshot and save_note.
    let notes = window.state::<crate::AppNotes>();
    let mut notes = notes.lock().map_err(|e| e.to_string())?;
    let id = window.label().strip_prefix("note-").unwrap_or("main");
    let note = notes.get_mut(id).ok_or("메모를 찾을 수 없습니다")?;
    let state = window.state::<Feedbacks>();
    let mut highlights = state.highlights.lock().map_err(|e| e.to_string())?;
    if highlights.get(window.label()) != Some(&revision) {
        return Ok(false);
    }
    // Persist first: a failed save must leave both the baseline and highlight intact.
    window
        .state::<crate::storage::Store>()
        .acknowledge_changes(id, note)?;
    Ok(acknowledge(&mut highlights, window.label(), revision))
}

pub fn active(window: &WebviewWindow) -> bool {
    let state = window.state::<Feedbacks>();
    state
        .deadlines
        .lock()
        .map(|deadlines| live(deadlines.get(window.label()).copied(), Instant::now()))
        .unwrap_or(false)
}
fn live(deadline: Option<Instant>, now: Instant) -> bool {
    deadline.is_some_and(|deadline| now < deadline)
}
pub fn clear(window: &WebviewWindow) {
    if let Ok(mut deadlines) = window.state::<Feedbacks>().deadlines.lock() {
        deadlines.remove(window.label());
    }
}
// Direct editor saves are already reviewed; clear both retained and transient feedback.
pub fn edited(window: &WebviewWindow) -> Result<(), String> {
    window
        .state::<Feedbacks>()
        .highlights
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    clear(window);
    Ok(())
}
pub fn changed(
    window: &WebviewWindow,
    before: Option<&crate::model::Note>,
    after: &crate::model::Note,
) -> Result<(), String> {
    if before.is_none_or(|note| note.revision == 0 || note.markdown == after.markdown) {
        return Ok(());
    }
    // Replace the rendering revision while retaining the last confirmed baseline.
    // A full revert has no pending changes and removes the confirmation menu.
    {
        let state = window.state::<Feedbacks>();
        let mut highlights = state.highlights.lock().map_err(|e| e.to_string())?;
        sync_highlight(&mut highlights, window.label(), after);
    }
    if !window.is_visible().map_err(|e| e.to_string())? {
        return Ok(());
    }
    let first = window
        .state::<Feedbacks>()
        .deadlines
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
                        let next = target.state::<Feedbacks>().deadlines.lock().ok().and_then(
                            |mut deadlines| {
                                let deadline = deadlines.get(target.label()).copied()?;
                                let remaining = deadline.saturating_duration_since(Instant::now());
                                if remaining.is_zero() {
                                    deadlines.remove(target.label());
                                    None
                                } else {
                                    Some(remaining)
                                }
                            },
                        );
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
    fn restored_pending_notes_have_highlights_without_attention_and_reverts_clear_them() {
        let mut note = crate::model::Note::default();
        for body in ["원본", "변경"] {
            note.apply(
                &serde_json::from_value(serde_json::json!({
                    "action": "update", "markdown": body
                }))
                .unwrap(),
            )
            .unwrap();
        }
        let state = Feedbacks::default();
        state
            .restore(&BTreeMap::from([("a".into(), note.clone())]))
            .unwrap();
        assert_eq!(
            state.highlights.lock().unwrap().get("note-a"),
            Some(&note.revision)
        );
        assert!(state.deadlines.lock().unwrap().is_empty());
        note.save_edit("원본".into(), note.revision).unwrap();
        sync_highlight(&mut state.highlights.lock().unwrap(), "note-a", &note);
        assert!(state.highlights.lock().unwrap().is_empty());
    }

    #[test]
    fn acknowledgement_is_revision_guarded_and_does_not_touch_opacity() {
        let state = Feedbacks::default();
        let deadline = Instant::now() + Duration::from_secs(3);
        state.deadlines.lock().unwrap().insert("a".into(), deadline);
        let mut highlights = state.highlights.lock().unwrap();
        highlights.insert("a".into(), 2);
        highlights.insert("b".into(), 4);
        assert!(!acknowledge(&mut highlights, "a", 1));
        assert_eq!(highlights.get("a"), Some(&2));
        assert!(acknowledge(&mut highlights, "a", 2));
        assert_eq!(highlights.get("b"), Some(&4));
        assert_eq!(state.deadlines.lock().unwrap().get("a"), Some(&deadline));
        highlights.insert("a".into(), 3);
        assert!(!acknowledge(&mut highlights, "a", 2));
        state.deadlines.lock().unwrap().clear();
        assert_eq!(highlights.get("a"), Some(&3));
    }
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
