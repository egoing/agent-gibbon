//! Only fixed diagnostics leave the app; note content and error messages stay local.
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn init() -> sentry::ClientInitGuard {
    let dsn = std::env::var("AGENT_GIBBON_SENTRY_DSN")
        .ok()
        .or_else(|| std::env::var("FLOATNOTE_SENTRY_DSN").ok())
        .or_else(|| option_env!("AGENT_GIBBON_SENTRY_DSN").map(str::to_owned))
        .or_else(|| option_env!("FLOATNOTE_SENTRY_DSN").map(str::to_owned))
        .unwrap_or_else(|| include_str!("../sentry-dsn.txt").trim().to_owned())
        .parse()
        .ok();
    sentry::init(sentry::ClientOptions {
        dsn,
        release: sentry::release_name!(),
        environment: Some(
            if cfg!(debug_assertions) {
                "development"
            } else {
                "production"
            }
            .into(),
        ),
        send_default_pii: false,
        before_send: Some(std::sync::Arc::new(sanitize)),
        ..Default::default()
    })
}

fn sanitize(
    mut event: sentry::protocol::Event<'static>,
) -> Option<sentry::protocol::Event<'static>> {
    event.message = event.message.map(|message| match message.as_str() {
        "WebView uncaught error"
        | "WebView unhandled promise rejection"
        | "Agent Gibbon Sentry connection test" => message,
        _ => "Agent Gibbon internal error".into(),
    });
    for exception in event.exception.values.iter_mut() {
        exception.value = Some("Error details withheld for privacy".into());
        if let Some(stack) = exception.stacktrace.as_mut() {
            for frame in &mut stack.frames {
                frame.vars.clear();
                frame.abs_path = None;
                frame.pre_context.clear();
                frame.post_context.clear();
                frame.context_line = None;
            }
        }
    }
    event.breadcrumbs.values.clear();
    event.extra.clear();
    event.user = None;
    event.server_name = None;
    Some(event)
}

#[tauri::command]
pub fn report_frontend_error(kind: &str) {
    // Accept no free-form error text; cap reports across all windows per process.
    static REPORTS: AtomicUsize = AtomicUsize::new(0);
    let message = match kind {
        "error" => "WebView uncaught error",
        "unhandledrejection" => "WebView unhandled promise rejection",
        _ => return,
    };
    if REPORTS.fetch_add(1, Ordering::Relaxed) < 20 {
        sentry::capture_message(message, sentry::Level::Error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Sends one fixed diagnostic to Sentry; run explicitly to verify delivery"]
    fn live_sentry_delivery() {
        let guard = init();
        assert!(guard.is_enabled());
        let event =
            sentry::capture_message("Agent Gibbon Sentry connection test", sentry::Level::Error);
        assert!(!event.is_nil());
        assert!(guard.flush(Some(std::time::Duration::from_secs(10))));
    }

    #[test]
    fn removes_private_error_payloads() {
        let mut event = sentry::protocol::Event {
            message: Some("private note".into()),
            server_name: Some("private host".into()),
            ..Default::default()
        };
        event.extra.insert("note".into(), "private note".into());
        event.exception.values.push(sentry::protocol::Exception {
            ty: "panic".into(),
            value: Some("private note".into()),
            ..Default::default()
        });
        let event = sanitize(event).unwrap();
        let encoded = serde_json::to_string(&event).unwrap();
        assert!(!encoded.contains("private"));
        assert!(event.extra.is_empty());
        assert!(event.server_name.is_none());
    }
}
