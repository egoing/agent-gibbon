use super::*;

#[test]
fn transmission_requires_embedded_dsn_and_explicit_development_opt_in() {
    let embedded = Some("https://public@example.invalid/42");
    assert!(dsn(true, false, true, embedded).is_none());
    assert!(dsn(false, true, false, embedded).is_none());
    assert!(dsn(false, true, true, embedded).is_some());
    assert!(dsn(false, false, false, embedded).is_some());
    assert!(dsn(false, false, true, None).is_none());
    assert!(dsn(false, false, true, Some("not a dsn")).is_none());
    assert!(init(false).is_none());
}
#[test]
fn rejects_unknown_and_nonfinite_frontend_measurements() {
    for (op, ms, count) in [
        ("private command", 12., 1),
        ("snapshot", f64::NAN, 1),
        ("snapshot", f64::INFINITY, 1),
        ("snapshot", -1., 1),
        ("snapshot", 1., 10001),
    ] {
        assert!(!valid_measurement(&Measurement {
            op: op.into(),
            duration_ms: ms,
            count,
            failure_count: 0
        }));
    }
    assert!(valid_measurement(&Measurement {
        op: "snapshot".into(),
        duration_ms: 12.,
        count: 3,
        failure_count: 0
    }));
    assert!(serde_json::from_value::<Measurement>(
        json!({"op":"snapshot","duration_ms":1,"private":"secret"})
    )
    .is_err());
}
#[test]
fn whole_envelope_scrubs_error_trace_log_and_denies_sessions_and_attachments() {
    let secret = "PRIVATE_NOTE_COMMAND_PATH_TOKEN";
    let mut e:p::Event=serde_json::from_value(json!({
        "message":secret,"user":{"username":secret},"request":{"url":"https://example.invalid/PRIVATE_NOTE_COMMAND_PATH_TOKEN"},
        "tags":{"secret":secret,"window.kind":"terminal"},"extra":{"secret":secret,"duration_ms":33},
        "contexts":{"private":{"secret":secret}},
        "breadcrumbs":{"values":[{"message":secret},{"category":"app.operation","message":"storage.save","data":{"secret":secret,"duration_ms":5}}]},
        "exception":{"values":[{"type":"panic","value":secret,"stacktrace":{"frames":[{"function":"application_function","abs_path":secret,"vars":{"secret":secret},"pre_context":[secret],"context_line":secret,"post_context":[secret]}]}}]}
    })).unwrap();
    e.transaction = Some(secret.into());
    let mut t = p::Transaction {
        name: Some("ipc.open".into()),
        ..Default::default()
    };
    t.user = Some(p::User {
        username: Some(secret.into()),
        ..Default::default()
    });
    t.extra.insert("secret".into(), json!(secret));
    t.spans.push(p::Span {
        op: Some("storage.save".into()),
        description: Some(secret.into()),
        data: p::Map::from_iter([
            ("secret".into(), json!(secret)),
            ("duration_ms".into(), json!(25)),
        ]),
        ..Default::default()
    });
    let log = p::Log {
        level: p::LogLevel::Info,
        body: "Agent Gibbon operation".into(),
        trace_id: None,
        timestamp: SystemTime::now(),
        severity_number: None,
        attributes: p::Map::from_iter([
            ("operation".into(), p::LogAttribute(json!("storage.save"))),
            ("secret".into(), p::LogAttribute(json!(secret))),
        ]),
    };
    let mut input = sentry::Envelope::new();
    input.add_item(e);
    input.add_item(t);
    input.add_item(vec![log]);
    // An unknown transaction and arbitrary log are denied rather than renamed.
    input.add_item(p::Transaction {
        name: Some(secret.into()),
        ..Default::default()
    });
    let session:p::SessionUpdate=serde_json::from_value(json!({"sid":"00112233-4455-6677-8899-aabbccddeeff","did":secret,"started":"2026-11-06T00:00:00Z","status":"ok","errors":0,"attrs":{"release":"test","user_agent":secret,"ip_address":"127.0.0.1"}})).unwrap();
    input.add_item(session);
    input.add_item(p::Attachment {
        buffer: secret.as_bytes().to_vec(),
        filename: secret.into(),
        content_type: None,
        ty: None,
    });
    let out = privacy::envelope(input);
    let mut bytes = Vec::new();
    out.to_writer(&mut bytes).unwrap();
    let payload = String::from_utf8(bytes).unwrap();
    assert!(!payload.contains(secret), "{payload}");
    assert!(payload.contains("storage.save"));
    assert!(payload.contains("application_function"));
    assert!(payload.contains("duration_ms"));
    assert_eq!(out.items().count(), 3);
    assert!(!payload.contains("00112233-4455-6677-8899-aabbccddeeff"));
}
#[test]
fn foreign_breadcrumbs_and_logs_are_not_allowed() {
    assert!(privacy::breadcrumb(p::Breadcrumb {
        category: Some("console".into()),
        message: Some("private".into()),
        ..Default::default()
    })
    .is_none());
    let l = p::Log {
        level: p::LogLevel::Info,
        body: "secret".into(),
        trace_id: None,
        timestamp: SystemTime::now(),
        severity_number: None,
        attributes: p::Map::new(),
    };
    assert!(privacy::log(l).is_none());
}

#[test]
fn basic_release_and_opted_in_diagnostics_have_separate_features() {
    let dsn = "https://public@example.invalid/42".parse().unwrap();
    let basic = options(dsn, false);
    assert!(!basic.enable_logs && !basic.auto_session_tracking);
    assert_eq!(
        basic.traces_sampler.unwrap()(&sentry::TransactionContext::new(
            "diagnostic.probe",
            "diagnostic.probe"
        )),
        0.0
    );
    let detailed = options("https://public@example.invalid/42".parse().unwrap(), true);
    assert!(detailed.enable_logs);
    assert!(!detailed.auto_session_tracking);
    let sampler = detailed.traces_sampler.unwrap();
    assert_eq!(
        sampler(&sentry::TransactionContext::new(
            "diagnostic.probe",
            "diagnostic.probe"
        )),
        1.0
    );
    assert!(
        sampler(&sentry::TransactionContext::new(
            "terminal.output.parse",
            "terminal.output.parse"
        )) <= 0.01
    );
}

#[test]
fn aggregate_tag_survives_but_local_correlation_never_leaves_envelope() {
    let mut input = sentry::Envelope::new();
    let mut t = p::Transaction {
        name: Some("snapshot".into()),
        ..Default::default()
    };
    t.tags
        .insert("measurement.origin".into(), "webview.aggregate".into());
    for key in ["local_flow", "parent", "cause", "local_mutation"] {
        t.extra.insert(key.into(), json!(123));
    }
    input.add_item(t);
    let mut bytes = Vec::new();
    privacy::envelope(input).to_writer(&mut bytes).unwrap();
    let payload = String::from_utf8(bytes).unwrap();
    assert!(payload.contains("webview.aggregate"));
    for key in ["local_flow", "parent", "cause", "local_mutation"] {
        assert!(!payload.contains(key));
    }
}

#[test]
fn snapshot_request_is_sampled_and_children_use_the_same_trace() {
    let config = options("https://public@example.invalid/42".parse().unwrap(), true);
    let sampler = config.traces_sampler.unwrap();
    assert!(
        sampler(&sentry::TransactionContext::new(
            "snapshot.request",
            "snapshot.request"
        )) <= 0.01
    );
    #[derive(Default)]
    struct Capture(std::sync::Mutex<Vec<sentry::Envelope>>);
    impl sentry::Transport for Capture {
        fn send_envelope(&self, envelope: sentry::Envelope) {
            self.0.lock().unwrap().push(envelope);
        }
    }
    let captured = Arc::new(Capture::default());
    let client = Arc::new(sentry::Client::from_config(sentry::ClientOptions {
        dsn: Some("https://public@example.invalid/42".parse().unwrap()),
        traces_sample_rate: 1.0,
        default_integrations: false,
        transport: Some(Arc::new(captured.clone())),
        ..Default::default()
    }));
    let hub = Arc::new(sentry::Hub::new(
        Some(client),
        Arc::new(sentry::Scope::default()),
    ));
    sentry::Hub::run(hub, || {
        let root = timing_span(None, "snapshot.request");
        for op in [
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
        ] {
            assert!(allowed(op));
            let child = timing_span(Some(&root), op);
            assert_eq!(
                child.get_trace_context().trace_id,
                root.get_trace_context().trace_id
            );
            child.finish();
        }
        root.finish();
    });
    let mut envelopes = captured.0.lock().unwrap();
    assert_eq!(envelopes.len(), 1);
    let filtered = privacy::envelope(envelopes.pop().unwrap());
    let t = filtered
        .items()
        .find_map(|item| match item {
            p::EnvelopeItem::Transaction(t) => Some(t),
            _ => None,
        })
        .unwrap();
    let p::Context::Trace(context) = &t.contexts["trace"] else {
        panic!("missing trace");
    };
    assert_eq!(t.spans.len(), 10);
    for span in &t.spans {
        assert_eq!(span.parent_span_id, Some(context.span_id));
        assert_eq!(span.trace_id, context.trace_id);
    }
}
