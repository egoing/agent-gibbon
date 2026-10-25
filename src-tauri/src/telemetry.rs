//! Only fixed diagnostics leave the app; note content and error messages stay local.
use std::sync::atomic::{AtomicUsize, Ordering};

// Never initialize the SDK without a build-time DSN: sentry::init otherwise
// falls back to the unrelated SENTRY_DSN environment variable.
fn production_dsn(disabled_build: bool, embedded: Option<&str>) -> Option<sentry::types::Dsn> {
    if disabled_build {
        return None;
    }
    embedded?.trim().parse().ok()
}

pub fn init() -> Option<sentry::ClientInitGuard> {
    let dsn = production_dsn(
        cfg!(any(debug_assertions, test)),
        option_env!("AGENT_GIBBON_SENTRY_DSN"),
    )?;
    Some(sentry::init(sentry::ClientOptions {
        dsn: Some(dsn),
        release: sentry::release_name!(),
        environment: Some("production".into()),
        send_default_pii: false,
        before_send: Some(std::sync::Arc::new(sanitize)),
        ..Default::default()
    }))
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

    const TEST_DSN: &str = "https://public@example.invalid/1";

    #[test]
    fn development_and_tests_never_enable_collection() {
        assert!(production_dsn(true, Some(TEST_DSN)).is_none());
        assert!(init().is_none());
    }

    #[test]
    fn production_requires_a_valid_embedded_dsn() {
        for value in [None, Some(""), Some("  "), Some("invalid")] {
            assert!(production_dsn(false, value).is_none());
        }
        assert!(production_dsn(false, Some(TEST_DSN)).is_some());
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
