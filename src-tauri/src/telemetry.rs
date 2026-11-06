//! One allowlisted egress for errors, traces and logs. Sessions stay disabled.
mod journal;
mod privacy;
mod transport;
mod watchdog;

use sentry::{protocol as p, TransactionOrSpan};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};
use tauri::{Emitter, Manager};

static ENABLED: AtomicBool = AtomicBool::new(false);
static DETAILED: AtomicBool = AtomicBool::new(false);
static OPERATIONS: AtomicUsize = AtomicUsize::new(0);
static HEARTBEATS: AtomicUsize = AtomicUsize::new(0);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static DIAGNOSTICS: AtomicUsize = AtomicUsize::new(0);
const MAX_DIAGNOSTICS: usize = 120;
struct Budget {
    start: Option<Instant>,
    used: [u32; 4],
}
static BUDGET: std::sync::Mutex<Budget> = std::sync::Mutex::new(Budget {
    start: None,
    used: [0; 4],
});
fn take_budget(bucket: usize) -> bool {
    let Ok(mut b) = BUDGET.lock() else {
        return false;
    };
    let now = Instant::now();
    if b.start
        .is_none_or(|start| now.duration_since(start) >= Duration::from_secs(60))
    {
        b.start = Some(now);
        b.used = [0; 4];
    }
    if b.used[bucket] >= [120, 20, 10, 2][bucket] {
        return false;
    }
    b.used[bucket] += 1;
    true
}
const MESSAGES: &[&str] = &[
    "WebView uncaught error",
    "WebView unhandled promise rejection",
    "Agent Gibbon internal error",
    "Agent Gibbon diagnostic probe",
    "Native UI stalled",
    "Native UI recovered",
    "WebView stalled",
    "WebView recovered",
];
pub const OPERATIONS_ALLOWED: &[&str] = &[
    "app.startup",
    "app.ready",
    "app.shutdown",
    "ipc.open",
    "ipc.update",
    "ipc.close",
    "ipc.inspect",
    "ipc.list",
    "ipc.wait",
    "window.create",
    "capture.prepare",
    "capture.finish",
    "capture.cancel",
    "capture.failed",
    "capture.lifecycle.wait",
    "capture.screen.enumerate",
    "capture.screen.capture",
    "capture.overlay.create",
    "capture.preview.copy",
    "capture.selection.validate",
    "capture.image.encode",
    "capture.cleanup.request",
    "capture.cleanup.wait",
    "storage.save",
    "storage.load",
    "markdown.render",
    "terminal.start",
    "terminal.window.open",
    "terminal.window.create",
    "terminal.window.position",
    "terminal.window.show",
    "terminal.command.prepare",
    "terminal.pty.create",
    "terminal.process.spawn",
    "terminal.io.attach",
    "terminal.output.filter",
    "terminal.output.dispatch",
    "terminal.session.lock",
    "terminal.input.write",
    "terminal.ui.ready",
    "terminal.first_output",
    "terminal.output.parse",
    "manager.terminal.open",
    "manager.terminal.delete",
    "terminal.write",
    "terminal.resize",
    "terminal.output",
    "terminal.hide",
    "terminal.info",
    "terminal.welcome",
    "terminal.folder",
    "terminal.rename",
    "terminal.close",
    "snapshot",
    "snapshot.request",
    "note.state.notes_wait",
    "note.state.notes_copy",
    "note.state.opacity",
    "note.state.feedback",
    "note.state.position",
    "note.state.size",
    "note.state.visibility",
    "note.state.focus",
    "note.state.pin",
    "note.state.response",
    "save_note",
    "export_note",
    "toggle_pin",
    "hide_note",
    "close_note",
    "warp_note",
    "toggle_fold",
    "capture_screen",
    "finish_capture",
    "copy_note_image",
    "manager_notes",
    "manager_image",
    "manager_terminal",
    "webview.long_task",
    "webview.long_frame",
    "webview.event_loop_lag",
    "webview.interaction",
    "diagnostic.probe",
];

fn allowed(op: &str) -> bool {
    OPERATIONS_ALLOWED.contains(&op)
}
fn dsn(
    test: bool,
    debug: bool,
    opt_in: bool,
    embedded: Option<&str>,
) -> Option<sentry::types::Dsn> {
    if test || (debug && !opt_in) {
        return None;
    }
    embedded?.trim().parse().ok()
}
fn diagnostic_mode() -> bool {
    cfg!(debug_assertions) && std::env::var("AGENT_GIBBON_DIAGNOSTICS").as_deref() == Ok("1")
}
fn sample_rate() -> f32 {
    std::env::var("AGENT_GIBBON_TRACE_SAMPLE_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .unwrap_or(if diagnostic_mode() { 1.0 } else { 0.2 })
}
fn options(dsn: sentry::types::Dsn, detailed: bool) -> sentry::ClientOptions {
    let rate = sample_rate();
    sentry::ClientOptions {
        dsn: Some(dsn),
        release: Some(
            format!(
                "agent-gibbon@{}+{}",
                env!("CARGO_PKG_VERSION"),
                env!("AGENT_GIBBON_BUILD_REVISION")
            )
            .into(),
        ),
        environment: Some(
            if diagnostic_mode() {
                "development"
            } else {
                "production"
            }
            .into(),
        ),
        send_default_pii: false,
        attach_stacktrace: true,
        in_app_include: vec!["agent_gibbon"],
        enable_logs: detailed,
        // Release Health stays off until its aggregate retention is confirmed.
        auto_session_tracking: false,
        max_breadcrumbs: 40,
        traces_sampler: Some(Arc::new(move |ctx| {
            if !detailed {
                return 0.0;
            }
            if matches!(ctx.name(), "app.startup" | "diagnostic.probe") {
                1.0
            } else if matches!(
                ctx.name(),
                "terminal.write"
                    | "terminal.output"
                    | "snapshot"
                    | "snapshot.request"
                    | "ipc.inspect"
                    | "ipc.list"
                    | "terminal.info"
                    | "terminal.output.parse"
            ) {
                rate.min(0.01)
            } else {
                rate
            }
        })),
        before_send: Some(Arc::new(privacy::event)),
        before_breadcrumb: Some(Arc::new(privacy::breadcrumb)),
        before_send_log: Some(Arc::new(privacy::log)),
        transport: Some(Arc::new(privacy::Factory)),
        shutdown_timeout: Duration::from_secs(2),
        ..Default::default()
    }
}
pub fn init(consented: bool) -> Option<sentry::ClientInitGuard> {
    if !consented {
        return None;
    }
    if std::env::var("AGENT_GIBBON_TELEMETRY").as_deref() == Ok("off") {
        return None;
    }
    let dsn = dsn(
        cfg!(test),
        cfg!(debug_assertions),
        diagnostic_mode(),
        option_env!("AGENT_GIBBON_SENTRY_DSN"),
    )?;
    let guard = sentry::Hub::run(sentry::Hub::main(), || sentry::init(options(dsn, true)));
    ENABLED.store(true, Ordering::Relaxed);
    DETAILED.store(true, Ordering::Relaxed);
    sentry::Hub::run(sentry::Hub::main(), || {
        sentry::configure_scope(|s| {
            s.set_tag("app.arch", std::env::consts::ARCH);
            s.set_tag("app.os", std::env::consts::OS);
            s.set_tag("app.build", env!("AGENT_GIBBON_BUILD_REVISION"));
        })
    });
    log("app.ready", 0.0);
    Some(guard)
}
#[derive(Default)]
pub struct Runtime(std::sync::Mutex<Option<sentry::ClientInitGuard>>);
pub fn revoke(app: &tauri::AppHandle) {
    ENABLED.store(false, Ordering::SeqCst);
    DETAILED.store(false, Ordering::SeqCst);
    transport::permit(false);
    let old = app
        .state::<Runtime>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    sentry::Hub::main().bind_client(None);
    drop(old);
    let _ = app.emit("privacy-changed", status());
}
pub fn refresh(app: &tauri::AppHandle) {
    let allowed = crate::privacy::sharing(app)
        && std::env::var("AGENT_GIBBON_TELEMETRY").as_deref() != Ok("off");
    if !allowed {
        revoke(app);
        return;
    }
    let runtime = app.state::<Runtime>();
    let mut guard = runtime.0.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        transport::permit(true);
        *guard = init(true);
        if guard.is_none() {
            transport::permit(false);
        }
    }
    drop(guard);
    install(app);
}
fn bind_client() {
    if enabled() {
        sentry::Hub::current().bind_client(sentry::Hub::main().client());
    }
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
pub fn detailed() -> bool {
    enabled() && DETAILED.load(Ordering::Relaxed)
}

thread_local! { static PARENTS: RefCell<Vec<TransactionOrSpan>> = const {RefCell::new(Vec::new())}; }
/// Nested Rust timings are child spans, never named with note IDs, paths or commands.
pub struct Timing {
    local: crate::local_trace::Call,
    _scope: crate::local_trace::Scope,
    span: Option<TransactionOrSpan>,
    op: &'static str,
    start: Instant,
}
fn timing_span(parent: Option<&TransactionOrSpan>, op: &'static str) -> TransactionOrSpan {
    parent
        .map(|parent| TransactionOrSpan::Span(parent.start_child(op, op)))
        .unwrap_or_else(|| {
            TransactionOrSpan::Transaction(sentry::start_transaction(
                sentry::TransactionContext::new(op, op),
            ))
        })
}
impl Timing {
    pub fn new(op: &'static str) -> Self {
        let local = crate::local_trace::Call::begin(op, crate::local_trace::Stage::Dispatch);
        let scope = local.scope();
        let span = (detailed() && allowed(op)).then(|| {
            bind_client();
            let span = PARENTS.with(|p| timing_span(p.borrow().last(), op));
            span.set_tag("measurement.origin", "rust");
            PARENTS.with(|p| p.borrow_mut().push(span.clone()));
            span
        });
        Self {
            local,
            _scope: scope,
            span,
            op,
            start: Instant::now(),
        }
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.local.complete();
        }
        if let Some(span) = self.span.take() {
            PARENTS.with(|p| {
                p.borrow_mut().pop();
            });
            let ms = self.start.elapsed().as_secs_f64() * 1000.;
            span.set_data("duration_ms", json!(ms));
            if std::thread::panicking() {
                span.set_status(p::SpanStatus::InternalError);
            }
            span.finish();
            OPERATIONS.fetch_add(1, Ordering::Relaxed);
            if ms >= 100. {
                log(self.op, ms);
            }
        }
    }
}
fn log(op: &str, ms: f64) {
    if !detailed() || !allowed(op) || !take_budget(0) {
        return;
    }
    DIAGNOSTICS.fetch_add(1, Ordering::Relaxed);
    bind_client();
    sentry::Hub::current().capture_log(p::Log {
        level: p::LogLevel::Info,
        body: "Agent Gibbon operation".into(),
        trace_id: PARENTS.with(|parents| {
            parents
                .borrow()
                .last()
                .map(|s| s.get_trace_context().trace_id)
        }),
        timestamp: SystemTime::now(),
        severity_number: None,
        attributes: p::Map::from_iter([
            ("operation".into(), p::LogAttribute(json!(op))),
            ("duration_ms".into(), p::LogAttribute(json!(ms))),
        ]),
    });
    sentry::add_breadcrumb(p::Breadcrumb {
        category: Some("app.operation".into()),
        message: Some(op.into()),
        data: p::Map::from_iter([("duration_ms".into(), json!(ms))]),
        ..Default::default()
    });
}
/// Static stage marker: makes a stage that never finishes visible in bounded
/// structured logs without including any window identifier or operation input.
pub fn checkpoint(op: &'static str) {
    log(op, 0.0);
}
pub fn diagnostic(message: &'static str, kind: &'static str, elapsed_ms: f64) {
    if !detailed() || !MESSAGES.contains(&message) {
        return;
    }
    journal::record(message, kind, elapsed_ms);
    if !take_budget(2) {
        return;
    }
    bind_client();
    sentry::with_scope(
        |s| {
            s.set_tag("window.kind", kind);
            s.set_extra("duration_ms", json!(elapsed_ms));
        },
        || {
            sentry::capture_message(message, sentry::Level::Warning);
        },
    );
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    op: String,
    duration_ms: f64,
    #[serde(default = "one")]
    count: u32,
    #[serde(default)]
    failure_count: u32,
}
fn one() -> u32 {
    1
}
fn valid_measurement(m: &Measurement) -> bool {
    allowed(&m.op)
        && m.duration_ms.is_finite()
        && (0.0..=600_000.).contains(&m.duration_ms)
        && (1..=10_000).contains(&m.count)
        && m.failure_count <= m.count
}
#[tauri::command]
pub fn telemetry_status() -> Value {
    status()
}
pub fn status() -> Value {
    json!({"enabled":enabled(),"detailed":detailed(),"environment":if cfg!(debug_assertions){"development"}else{"production"},"traces_sample_rate":if detailed(){sample_rate()}else{0.0},"logs":detailed(),"release_health":false,"revision":env!("AGENT_GIBBON_BUILD_REVISION"),"operations":OPERATIONS.load(Ordering::Relaxed),"heartbeats":HEARTBEATS.load(Ordering::Relaxed),"frontend_errors":ERRORS.load(Ordering::Relaxed),"diagnostic_limit":MAX_DIAGNOSTICS,"watchdog":watchdog::status(),"journal":journal::status()})
}
#[tauri::command]
pub fn telemetry_heartbeat(
    window: tauri::WebviewWindow,
    measurements: Vec<Measurement>,
) -> Result<(), String> {
    if !detailed() {
        return Ok(());
    }
    if measurements.len() > 32 || measurements.iter().any(|m| !valid_measurement(m)) {
        return Err("Invalid diagnostic measurement".into());
    }
    HEARTBEATS.fetch_add(1, Ordering::Relaxed);
    watchdog::beat(window.app_handle(), window.label());
    let kind = watchdog::kind(window.label());
    for m in measurements {
        let started = SystemTime::now()
            .checked_sub(Duration::from_secs_f64(m.duration_ms / 1000.))
            .unwrap_or(SystemTime::now());
        let t = sentry::start_transaction_with_timestamp(
            sentry::TransactionContext::new(&m.op, &m.op),
            started,
        );
        t.set_tag("window.kind", kind);
        t.set_tag("measurement.origin", "webview.aggregate");
        t.set_data("duration_ms", json!(m.duration_ms));
        t.set_data("count", json!(m.count));
        t.set_data("failure_count", json!(m.failure_count));
        t.set_status(if m.failure_count > 0 {
            p::SpanStatus::InternalError
        } else {
            p::SpanStatus::Ok
        });
        t.finish();
        if m.duration_ms >= 100. {
            log(&m.op, m.duration_ms);
        }
    }
    Ok(())
}
const SOURCES: &[&str] = &[
    "app.js",
    "telemetry.js",
    "terminal.js",
    "terminal-ime.js",
    "toolbar.js",
    "manager.js",
    "privacy.js",
    "feedback.js",
    "capture.js",
    "stash.js",
];
#[tauri::command]
pub fn report_frontend_error(
    kind: &str,
    source: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
) {
    let message = match kind {
        "error" => "WebView uncaught error",
        "unhandledrejection" => "WebView unhandled promise rejection",
        _ => return,
    };
    if !enabled() || !take_budget(1) {
        return;
    }
    ERRORS.fetch_add(1, Ordering::Relaxed);
    let mut event = p::Event {
        message: Some(message.into()),
        platform: "javascript".into(),
        level: sentry::Level::Error,
        ..Default::default()
    };
    let frames = source
        .filter(|s| SOURCES.contains(&s.as_str()))
        .map(|source| p::Frame {
            filename: Some(source),
            lineno: line.filter(|n| *n <= 10_000_000).map(u64::from),
            colno: column.filter(|n| *n <= 10_000_000).map(u64::from),
            ..Default::default()
        })
        .into_iter()
        .collect();
    event.exception.values.push(p::Exception {
        ty: "WebViewError".into(),
        value: Some(message.into()),
        stacktrace: Some(p::Stacktrace {
            frames,
            ..Default::default()
        }),
        ..Default::default()
    });
    bind_client();
    sentry::capture_event(event);
}

pub fn probe() -> Value {
    if !enabled() {
        return json!({"ok":false,"error":"Sentry diagnostics are disabled"});
    }
    if !take_budget(3) {
        return json!({"ok":false,"error":"Diagnostic probe rate limited"});
    }
    bind_client();
    let t = Timing::new("diagnostic.probe");
    log("diagnostic.probe", 0.0);
    let event = sentry::capture_message("Agent Gibbon diagnostic probe", sentry::Level::Info);
    drop(t);
    // This proves queue drain only, never claim server ingestion from flush alone.
    let flushed = sentry::Hub::current()
        .client()
        .is_some_and(|c| c.flush(Some(Duration::from_secs(3))));
    json!({"ok":true,"event_id":event.to_string(),"queue_flushed":flushed,"note":"Verify server ingestion separately"})
}
pub fn install(app: &tauri::AppHandle) {
    if detailed() {
        journal::start(app);
        watchdog::install(app);
    }
}
pub fn shutdown(app: &tauri::AppHandle) {
    watchdog::stop(app);
    log("app.shutdown", 0.0);
}

#[cfg(test)]
mod tests;
