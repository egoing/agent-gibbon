//! Separate thread distinguishes a blocked native UI from a silent visible WebView.
use super::*;
use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicBool, Mutex},
    thread,
};
const STALL: Duration = Duration::from_secs(12);
const GRACE: Duration = Duration::from_secs(30);
#[derive(Default)]
pub struct Watchdog {
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
}
struct View {
    last: Instant,
    created: Instant,
    visible: bool,
    reported: bool,
    kind: &'static str,
}
struct State {
    native: Instant,
    native_reported: bool,
    views: BTreeMap<String, View>,
    stalls: usize,
    recoveries: usize,
}
impl Default for State {
    fn default() -> Self {
        Self {
            native: Instant::now(),
            native_reported: false,
            views: BTreeMap::new(),
            stalls: 0,
            recoveries: 0,
        }
    }
}
impl Default for View {
    fn default() -> Self {
        Self {
            last: Instant::now(),
            created: Instant::now(),
            visible: false,
            reported: false,
            kind: "memo",
        }
    }
}
pub fn kind(label: &str) -> &'static str {
    if label.starts_with("terminal-") {
        "terminal"
    } else if label.starts_with("agent-") {
        "agent"
    } else if label == "manager" {
        "manager"
    } else if label.starts_with("feedback-") {
        "feedback"
    } else if label.starts_with("capture-") {
        "capture"
    } else if label == "stash-indicator" {
        "stash"
    } else {
        "memo"
    }
}
// Only surfaces loading telemetry.js participate. Decorative WebViews are excluded.
fn monitored(label: &str) -> bool {
    label == "main"
        || label.starts_with("note-")
        || label.starts_with("terminal-")
        || label.starts_with("agent-")
        || label == "manager"
}
fn decisions(state: &mut State, now: Instant) -> Vec<(&'static str, &'static str, f64)> {
    let mut events = Vec::new();
    let lag = now.saturating_duration_since(state.native);
    if lag > STALL {
        if !state.native_reported {
            state.native_reported = true;
            state.stalls += 1;
            events.push(("Native UI stalled", "native", lag.as_secs_f64() * 1000.));
        }
        return events;
    }
    if state.native_reported {
        state.native_reported = false;
        state.recoveries += 1;
        events.push(("Native UI recovered", "native", 0.));
    }
    for v in state.views.values_mut() {
        let age = now.saturating_duration_since(v.created);
        let lag = now.saturating_duration_since(v.last);
        if v.visible && age > GRACE && lag > STALL {
            if !v.reported {
                v.reported = true;
                state.stalls += 1;
                events.push(("WebView stalled", v.kind, lag.as_secs_f64() * 1000.));
            }
        } else if v.reported && lag <= STALL {
            v.reported = false;
            state.recoveries += 1;
            events.push(("WebView recovered", v.kind, 0.));
        }
    }
    events
}
pub fn beat(app: &tauri::AppHandle, label: &str) {
    if let Some(w) = app.try_state::<Watchdog>() {
        if let Ok(mut s) = w.state.lock() {
            if let Some(v) = s.views.get_mut(label) {
                v.last = Instant::now();
            }
        }
    }
}
pub fn status() -> Value {
    json!({"stall_threshold_ms":STALL.as_millis(),"startup_grace_ms":GRACE.as_millis()})
}
pub fn install(app: &tauri::AppHandle) {
    if let Some(w) = app.try_state::<Watchdog>() {
        if let Ok(mut s) = w.state.lock() {
            s.native = Instant::now();
            s.native_reported = false;
            s.views.clear();
        }
        return;
    }
    let w = Watchdog::default();
    let state = w.state.clone();
    let stop = w.stop.clone();
    let pending = w.pending.clone();
    app.manage(w);
    let app = app.clone();
    thread::spawn(move || {
        let mut previous = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(2));
            let now = Instant::now();
            // Machine sleep / suspended process is not a UI freeze.
            let resumed = now.duration_since(previous) > Duration::from_secs(8);
            previous = now;
            if !detailed() {
                if let Ok(mut s) = state.lock() {
                    s.native = now;
                    s.native_reported = false;
                    s.views.clear();
                }
                continue;
            }
            if let Ok(mut s) = state.lock() {
                if resumed {
                    s.native = now;
                    for v in s.views.values_mut() {
                        v.last = now;
                        v.created = now;
                        v.reported = false;
                    }
                } else {
                    for (message, kind, ms) in decisions(&mut s, now) {
                        diagnostic(message, kind, ms);
                    }
                }
            }
            // At most one outstanding native event, even during a permanent deadlock.
            if pending.swap(true, Ordering::Relaxed) {
                continue;
            }
            let state = state.clone();
            let pending_done = pending.clone();
            let target = app.clone();
            if app
                .run_on_main_thread(move || {
                    if !detailed() {
                        pending_done.store(false, Ordering::Relaxed);
                        return;
                    }
                    let now = Instant::now();
                    let windows = crate::web::managed_windows(&target);
                    if let Ok(mut s) = state.lock() {
                        s.native = now;
                        s.views.retain(|label, _| windows.contains_key(label));
                        for (label, window) in windows {
                            if monitored(&label) {
                                let v = s.views.entry(label.clone()).or_default();
                                v.kind = kind(&label);
                                v.visible = window.is_visible().unwrap_or(false);
                            }
                        }
                    }
                    pending_done.store(false, Ordering::Relaxed);
                })
                .is_err()
            {
                pending.store(false, Ordering::Relaxed);
                break;
            }
        }
    });
}
pub fn stop(app: &tauri::AppHandle) {
    if let Some(w) = app.try_state::<Watchdog>() {
        w.stop.store(true, Ordering::Relaxed);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_native_and_webview_and_reports_once() {
        let start = Instant::now();
        let mut s = State {
            native: start,
            ..Default::default()
        };
        assert_eq!(
            decisions(&mut s, start + Duration::from_secs(13))[0].0,
            "Native UI stalled"
        );
        assert!(decisions(&mut s, start + Duration::from_secs(15)).is_empty());
        s.native = start + Duration::from_secs(40);
        s.views.insert(
            "local-private-note-id".into(),
            View {
                last: start,
                created: start,
                visible: true,
                kind: "memo",
                reported: false,
            },
        );
        let events = decisions(&mut s, start + Duration::from_secs(40));
        assert_eq!(events[0].0, "Native UI recovered");
        assert_eq!(events[1].0, "WebView stalled");
        assert!(decisions(&mut s, start + Duration::from_secs(41)).is_empty());
        s.views.get_mut("local-private-note-id").unwrap().last = start + Duration::from_secs(41);
        assert_eq!(
            decisions(&mut s, start + Duration::from_secs(41))[0].0,
            "WebView recovered"
        );
    }
    #[test]
    fn hidden_and_new_views_do_not_stall() {
        let start = Instant::now();
        let mut s = State {
            native: start + Duration::from_secs(20),
            ..Default::default()
        };
        s.views.insert(
            "test".into(),
            View {
                last: start,
                created: start,
                visible: true,
                ..Default::default()
            },
        );
        assert!(decisions(&mut s, start + Duration::from_secs(20)).is_empty());
        s.native = start + Duration::from_secs(60);
        s.views.get_mut("test").unwrap().visible = false;
        assert!(decisions(&mut s, start + Duration::from_secs(60)).is_empty());
    }
}
