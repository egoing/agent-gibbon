use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

pub type Fades = Mutex<BTreeMap<String, Fade>>;
pub struct Fade {
    current: f64,
    target: f64,
    from: f64,
    started: Option<Instant>,
}
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
pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(16));
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
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            for label in labels {
                if let Some(window) = handle.get_webview_window(&label) {
                    let _ = crate::sync_opacity(window);
                } else {
                    handle.state::<Fades>().lock().unwrap().remove(&label);
                }
            }
        });
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
