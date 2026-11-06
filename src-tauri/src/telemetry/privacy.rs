//! SDK event hooks do not cover transactions. Filter *every* envelope at egress.
use super::*;
use sentry::Envelope;

fn safe_tags(tags: &mut p::Map<String, String>) {
    tags.retain(|k, v| match k.as_str() {
        "app.arch" => matches!(v.as_str(), "aarch64" | "x86_64" | "x86"),
        "app.os" => matches!(v.as_str(), "macos" | "windows" | "linux"),
        "app.build" => v == env!("AGENT_GIBBON_BUILD_REVISION"),
        "measurement.origin" => matches!(v.as_str(), "rust" | "webview"),
        "window.kind" => matches!(
            v.as_str(),
            "memo" | "terminal" | "agent" | "manager" | "feedback" | "capture" | "stash" | "native"
        ),
        _ => false,
    });
}
fn safe_data(data: &mut p::Map<String, Value>) {
    data.retain(|k, v| {
        matches!(k.as_str(), "duration_ms" | "count" | "failure_count")
            && v.as_f64()
                .is_some_and(|v| v.is_finite() && (0.0..=600_000.).contains(&v))
    });
}
fn contexts(contexts: &mut p::Map<String, p::Context>) {
    contexts.retain(|k, c| match (k.as_str(), c) {
        ("trace", p::Context::Trace(t)) => {
            if !t.op.as_deref().is_some_and(allowed) {
                t.op = None;
            }
            t.description = None;
            safe_data(&mut t.data);
            true
        }
        // OS/runtime version and architecture are useful, device names and paths are not.
        ("os", p::Context::Os(o)) => {
            o.other.clear();
            true
        }
        ("runtime", p::Context::Runtime(r)) => {
            r.other.clear();
            true
        }
        _ => false,
    });
}
pub fn breadcrumb(mut b: p::Breadcrumb) -> Option<p::Breadcrumb> {
    if b.category.as_deref() != Some("app.operation") || !b.message.as_deref().is_some_and(allowed)
    {
        return None;
    }
    safe_data(&mut b.data);
    Some(b)
}
pub fn log(mut l: p::Log) -> Option<p::Log> {
    if l.body != "Agent Gibbon operation" {
        return None;
    }
    l.attributes.retain(|k, v| match k.as_str() {
        "operation" => v.0.as_str().is_some_and(allowed),
        "duration_ms" => {
            v.0.as_f64()
                .is_some_and(|n| n.is_finite() && (0.0..=600_000.).contains(&n))
        }
        _ => false,
    });
    if !l.attributes.contains_key("operation") {
        return None;
    }
    l.attributes.insert(
        "sentry.release".into(),
        p::LogAttribute(json!(format!(
            "agent-gibbon@{}+{}",
            env!("CARGO_PKG_VERSION"),
            env!("AGENT_GIBBON_BUILD_REVISION")
        ))),
    );
    l.attributes.insert(
        "sentry.environment".into(),
        p::LogAttribute(json!(if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        })),
    );
    l.attributes.insert(
        "app.os".into(),
        p::LogAttribute(json!(std::env::consts::OS)),
    );
    l.attributes.insert(
        "app.arch".into(),
        p::LogAttribute(json!(std::env::consts::ARCH)),
    );
    Some(l)
}
pub fn event(mut e: p::Event<'static>) -> Option<p::Event<'static>> {
    e.message = e.message.map(|s| {
        if MESSAGES.contains(&s.as_str()) {
            s
        } else {
            "Agent Gibbon internal error".into()
        }
    });
    for exception in e.exception.values.iter_mut() {
        exception.value = Some("Error details withheld for privacy".into());
        if let Some(stack) = exception.stacktrace.as_mut() {
            for f in &mut stack.frames {
                f.vars.clear();
                f.abs_path = None;
                f.filename = f
                    .filename
                    .take()
                    .filter(|name| SOURCES.contains(&name.as_str()));
                f.pre_context.clear();
                f.post_context.clear();
                f.context_line = None;
            }
        }
    }
    e.threads.values.clear();
    let debug = e.debug_meta.to_mut();
    for image in &mut debug.images {
        let mut value = serde_json::to_value(&*image).unwrap_or(Value::Null);
        if let Some(obj) = value.as_object_mut() {
            for key in ["code_file", "debug_file", "name"] {
                if let Some(path) = obj.get(key).and_then(Value::as_str).map(str::to_owned) {
                    let base = path.rsplit(['/', '\\']).next().unwrap_or("");
                    obj.insert(key.into(), json!(base));
                }
            }
        }
        if let Ok(clean) = serde_json::from_value(value) {
            *image = clean;
        }
    }
    e.breadcrumbs.values = std::mem::take(&mut e.breadcrumbs.values)
        .into_iter()
        .filter_map(breadcrumb)
        .collect();
    safe_data(&mut e.extra);
    safe_tags(&mut e.tags);
    contexts(&mut e.contexts);
    e.user = None;
    e.server_name = None;
    e.request = None;
    e.logentry = None;
    e.transaction = e.transaction.filter(|n| allowed(n));
    e.fingerprint = std::borrow::Cow::Borrowed(&[]);
    Some(e)
}
fn transaction(mut t: p::Transaction<'static>) -> Option<p::Transaction<'static>> {
    if !t.name.as_deref().is_some_and(allowed) {
        return None;
    }
    t.user = None;
    t.request = None;
    t.server_name = None;
    t.extra.clear();
    safe_tags(&mut t.tags);
    t.tags.insert("app.os".into(), std::env::consts::OS.into());
    t.tags
        .insert("app.arch".into(), std::env::consts::ARCH.into());
    t.tags.insert(
        "app.build".into(),
        env!("AGENT_GIBBON_BUILD_REVISION").into(),
    );
    contexts(&mut t.contexts);
    t.spans.retain(|s| s.op.as_deref().is_some_and(allowed));
    for s in &mut t.spans {
        s.description = s.op.clone();
        safe_tags(&mut s.tags);
        safe_data(&mut s.data);
    }
    Some(t)
}
pub fn envelope(input: Envelope) -> Envelope {
    // Rebuild headers: no propagated arbitrary dynamic sampling context or DSN.
    let mut out = Envelope::new();
    for item in input.items() {
        match item {
            p::EnvelopeItem::Event(e) => {
                if let Some(e) = event(e.clone()) {
                    out.add_item(e);
                }
            }
            p::EnvelopeItem::Transaction(t) => {
                if let Some(t) = transaction(t.clone()) {
                    out.add_item(t);
                }
            }
            p::EnvelopeItem::ItemContainer(p::ItemContainer::Logs(logs)) => {
                let logs: Vec<_> = logs.iter().cloned().filter_map(log).collect();
                if !logs.is_empty() {
                    out.add_item(logs);
                }
            }
            // Sessions/aggregates, attachments, replay, dumps and unknown types are denied.
            _ => (),
        }
    }
    out
}
pub use super::transport::Factory;
