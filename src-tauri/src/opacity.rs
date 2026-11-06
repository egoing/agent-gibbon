#[cfg(any(target_os = "macos", windows))]
use std::collections::BTreeMap;
#[cfg(any(target_os = "macos", windows))]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

#[cfg(any(target_os = "macos", windows))]
pub type Fades = Mutex<BTreeMap<String, Fade>>;
#[cfg(any(target_os = "macos", windows, test))]
pub struct Fade {
    current: f64,
    target: f64,
    from: f64,
    started: Option<Instant>,
}
#[cfg(any(target_os = "macos", windows, test))]
impl Default for Fade {
    fn default() -> Self {
        Self {
            current: 1.,
            target: 1.,
            from: 1.,
            started: None,
        }
    }
}
#[cfg(any(target_os = "macos", windows, test))]
impl Fade {
    pub fn sample(&mut self, target: f64, now: Instant) -> f64 {
        if target >= self.current {
            self.current = target;
            self.started = None;
        } else {
            if self.started.is_none() || self.target != target {
                self.started = Some(now);
                self.from = self.current;
            }
            let t = ((now.duration_since(self.started.unwrap()).as_secs_f64() - 2.) / 0.6)
                .clamp(0., 1.);
            self.current = self.from + (target - self.from) * t * t * (3. - 2. * t);
            if t >= 1. {
                self.current = target;
                self.started = None;
            }
        }
        self.target = target;
        self.current
    }
}
// Only pending fades schedule main-thread work; idle windows do not animate.
#[cfg(any(target_os = "macos", windows))]
pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let pending = Arc::new(AtomicBool::new(false));
        loop {
            std::thread::sleep(Duration::from_millis(16));
            if pending.load(Ordering::Acquire) {
                continue;
            }
            let labels: Vec<_> = app
                .state::<Fades>()
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, fade)| fade.started.is_some())
                .map(|(label, _)| label.clone())
                .collect();
            if labels.is_empty() {
                continue;
            }
            pending.store(true, Ordering::Release);
            let finished = pending.clone();
            let handle = app.clone();
            if app
                .run_on_main_thread(move || {
                    for label in labels {
                        if let Some(window) = handle.get_webview_window(&label) {
                            let _ = crate::sync_opacity(window);
                        } else {
                            handle.state::<Fades>().lock().unwrap().remove(&label);
                        }
                    }
                    finished.store(false, Ordering::Release);
                })
                .is_err()
            {
                break;
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waits_two_seconds_then_eases_and_interrupts_immediately() {
        let now = Instant::now();
        let mut fade = Fade::default();
        assert_eq!(fade.sample(0.5, now), 1.);
        assert_eq!(fade.sample(0.5, now + Duration::from_millis(1999)), 1.);
        assert!((fade.sample(0.5, now + Duration::from_millis(2300)) - 0.75).abs() < 0.0001);
        assert_eq!(fade.sample(1., now + Duration::from_millis(2301)), 1.);
        assert_eq!(fade.sample(0.5, now + Duration::from_millis(2400)), 1.);
        assert_eq!(fade.sample(0.5, now + Duration::from_millis(4399)), 1.);
        assert_eq!(fade.sample(0.5, now + Duration::from_millis(5000)), 0.5);
        assert!(fade.started.is_none());
    }
}

// An explicit wheel choice overrides automatic dimming, without changing any
// note document. None retains the previous behavior until the first gesture.
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Choice {
    percent: Option<u8>,
}
fn settings_path(root: &Path, instance: Option<&str>) -> PathBuf {
    root.join("appearance")
        .join(instance.unwrap_or("installed"))
        .join("opacity.json")
}
fn load(path: &Path) -> Result<Choice, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Choice::default()),
        Err(e) => return Err(e.to_string()),
    };
    if bytes.len() > 1024 {
        return Err("불투명도 설정 크기 초과".into());
    }
    let choice: Choice = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if choice.percent.is_some_and(|p| !(10..=100).contains(&p)) {
        return Err("불투명도 설정 범위 오류".into());
    }
    Ok(choice)
}
fn save(path: &Path, choice: Choice) -> Result<(), String> {
    let parent = path.parent().ok_or("불투명도 설정 경로 오류")?;
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut file, &choice).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Default)]
struct Wheel {
    remainder: f64,
    last: Option<Instant>,
}
impl Wheel {
    // Native adapters normalize wheel notches (and high-resolution fractions)
    // before entering here. Never carry partial motion across chord sessions.
    fn steps(&mut self, units: f64, chord: bool, now: Instant) -> i32 {
        if !chord || !units.is_finite() {
            *self = Self::default();
            return 0;
        }
        if self
            .last
            .is_none_or(|last| now.duration_since(last) > Duration::from_millis(400))
        {
            self.remainder = 0.;
        }
        self.last = Some(now);
        self.remainder += units.clamp(-20., 20.);
        let steps = self.remainder.trunc() as i32;
        self.remainder -= f64::from(steps);
        steps
    }
}
struct Local {
    path: PathBuf,
    choice: Choice,
    wheel: Wheel,
}
impl Local {
    fn adjust(&mut self, steps: i32) -> Result<bool, String> {
        let next = adjusted(self.choice, steps);
        if next.percent == self.choice.percent {
            return Ok(false);
        }
        save(&self.path, next)?;
        self.choice = next;
        Ok(true)
    }
}
pub struct Global(Mutex<Local>);
pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let path = settings_path(
        &app.path().app_data_dir().map_err(|e| e.to_string())?,
        crate::instance::current().id.as_deref(),
    );
    let choice = load(&path).unwrap_or_else(|e| {
        eprintln!("불투명도 설정 읽기 실패: {e}");
        Choice::default()
    });
    app.manage(Global(Mutex::new(Local {
        path,
        choice,
        wheel: Wheel::default(),
    })));
    Ok(())
}
pub fn configured(app: &tauri::AppHandle) -> Option<f64> {
    app.state::<Global>()
        .0
        .lock()
        .unwrap()
        .choice
        .percent
        .map(|p| f64::from(p) / 100.)
}
pub fn reset_wheel(app: &tauri::AppHandle) {
    app.state::<Global>().0.lock().unwrap().wheel = Wheel::default();
}
fn adjusted(choice: Choice, steps: i32) -> Choice {
    Choice {
        percent: Some(
            (i32::from(choice.percent.unwrap_or(50)) + steps.clamp(-20, 20) * 5).clamp(10, 100)
                as u8,
        ),
    }
}
// Runs on the main thread, including the Windows hook's queued dispatch. Persist
// before publishing so an I/O failure never pretends the setting was saved.
pub fn wheel(app: &tauri::AppHandle, units: f64, chord: bool) {
    if !crate::stash::input_owned(app) {
        return;
    }
    let result = (|| -> Result<bool, String> {
        let state = app.state::<Global>();
        let mut local = state.0.lock().map_err(|e| e.to_string())?;
        let steps = local.wheel.steps(units, chord, Instant::now());
        if steps == 0 {
            return Ok(false);
        }
        local.adjust(steps)
    })();
    match result {
        Ok(true) => refresh(app),
        Ok(false) => {}
        Err(e) => eprintln!("불투명도 저장 실패: {e}"),
    }
}
pub fn refresh(app: &tauri::AppHandle) {
    for window in app.webview_windows().into_values() {
        if is_memo(window.label()) {
            if let Err(e) = crate::sync_opacity(window) {
                eprintln!("불투명도 적용 실패: {e}");
            }
        }
    }
}
fn is_memo(label: &str) -> bool {
    label != crate::manager::LABEL
        && label != "stash-indicator"
        && !label.starts_with("capture-")
        && !crate::feedback_window::is_surface(label)
}
// Native whole-window alpha includes controls and WebView backgrounds. Main
// thread only. No focus, z-order, geometry or visibility mutations.
pub fn set_native(window: &tauri::WebviewWindow, value: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    unsafe {
        let native =
            &*(window.ns_window().map_err(|e| e.to_string())? as *const objc2_app_kit::NSWindow);
        if native.alphaValue() != value {
            native.setAlphaValue(value);
        }
    }
    #[cfg(windows)]
    crate::windows_window::set_opacity(window, value)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = (window, value);
    Ok(())
}
#[cfg(test)]
mod global_tests {
    use super::*;
    #[test]
    fn bounds_and_first_choice_preserve_legacy_until_used() {
        assert!(Choice::default().percent.is_none());
        assert_eq!(adjusted(Choice::default(), 1).percent, Some(55));
        assert_eq!(adjusted(Choice::default(), -1).percent, Some(45));
        assert_eq!(
            adjusted(Choice { percent: Some(95) }, 20).percent,
            Some(100)
        );
        assert_eq!(
            adjusted(Choice { percent: Some(15) }, -20).percent,
            Some(10)
        );
        assert_eq!(adjusted(Choice { percent: Some(10) }, -1).percent, Some(10));
        assert_eq!(
            adjusted(Choice { percent: Some(100) }, 1).percent,
            Some(100)
        );
    }
    #[test]
    fn accumulates_fractions_and_resets_between_chords_and_idle() {
        let mut w = Wheel::default();
        let t = Instant::now();
        for _ in 0..3 {
            assert_eq!(w.steps(0.25, true, t), 0);
        }
        assert_eq!(w.steps(0.25, true, t), 1);
        assert_eq!(w.steps(-2., true, t), -2);
        assert_eq!(w.steps(0.75, true, t), 0);
        assert_eq!(w.steps(0., false, t), 0);
        assert_eq!(w.steps(0.25, true, t), 0);
        assert_eq!(w.steps(0.75, true, t + Duration::from_secs(1)), 0);
        assert_eq!(w.steps(0.25, true, t + Duration::from_secs(1)), 1);
        assert_eq!(w.steps(f64::INFINITY, true, t + Duration::from_secs(1)), 0);
    }
    #[test]
    fn saves_reloads_and_isolates_instances_and_rejects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let a = settings_path(dir.path(), Some("dev-a"));
        let b = settings_path(dir.path(), Some("dev-b"));
        let installed = settings_path(dir.path(), None);
        assert!(load(&a).unwrap().percent.is_none());
        save(&a, Choice { percent: Some(35) }).unwrap();
        assert_eq!(load(&a).unwrap().percent, Some(35));
        assert!(load(&b).unwrap().percent.is_none());
        assert!(load(&installed).unwrap().percent.is_none());
        save(&a, Choice { percent: Some(100) }).unwrap();
        assert_eq!(load(&a).unwrap().percent, Some(100));
        for invalid in [
            r#"{"percent":0}"#,
            r#"{"percent":101}"#,
            r#"{"percent":-1}"#,
            r#"{"percent":null,"extra":1}"#,
            "broken",
        ] {
            std::fs::write(&a, invalid).unwrap();
            assert!(load(&a).is_err(), "{invalid}");
        }
        assert_eq!(std::fs::read_dir(a.parent().unwrap()).unwrap().count(), 1);
    }
    #[test]
    fn failed_save_keeps_previous_file_and_memory_choice() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, "unchanged").unwrap();
        let mut local = Local {
            path: blocker.join("opacity.json"),
            choice: Choice { percent: Some(50) },
            wheel: Wheel::default(),
        };
        assert!(local.adjust(-1).is_err());
        assert_eq!(local.choice.percent, Some(50));
        assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "unchanged");
    }
    #[test]
    fn global_target_excludes_auxiliary_windows() {
        for label in ["main", "note-demo", "agent-demo"] {
            assert!(is_memo(label));
        }
        for label in [
            crate::manager::LABEL,
            "stash-indicator",
            "capture-test",
            "feedback-main",
        ] {
            assert!(!is_memo(label));
        }
    }
}
