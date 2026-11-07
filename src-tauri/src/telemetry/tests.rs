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
        name: Some("save_note".into()),
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

// Reproduces the IPC receive-thread -> UI callback boundary without native UI
// or network traffic. The callback's nested operations must share one trace.
#[test]
fn ipc_callback_keeps_wait_parent_across_threads() {
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
    let main = sentry::Hub::main();
    let previous = main.client();
    main.bind_client(Some(client));
    transport::permit(true);
    ENABLED.store(true, Ordering::SeqCst);
    DETAILED.store(true, Ordering::SeqCst);
    std::thread::spawn(|| {
        let wait = Timing::new("ipc.wait");
        let parent = wait.context();
        std::thread::spawn(move || {
            let _parent = parent.enter();
            let _apply = Timing::new("ipc.open");
            let _nested = Timing::new("markdown.render");
        })
        .join()
        .unwrap();
        drop(wait);
    })
    .join()
    .unwrap();
    ENABLED.store(false, Ordering::SeqCst);
    DETAILED.store(false, Ordering::SeqCst);
    main.bind_client(previous);
    transport::permit(false);
    let envelopes = captured.0.lock().unwrap();
    let transactions: Vec<_> = envelopes
        .iter()
        .flat_map(|e| e.items())
        .filter_map(|i| {
            if let p::EnvelopeItem::Transaction(t) = i {
                Some(t)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        transactions.len(),
        1,
        "callback must remain in the wait transaction"
    );
    let t = transactions[0];
    assert_eq!(t.name.as_deref(), Some("ipc.wait"));
    let p::Context::Trace(root) = &t.contexts["trace"] else {
        panic!("missing trace")
    };
    let apply = t
        .spans
        .iter()
        .find(|s| s.op.as_deref() == Some("ipc.open"))
        .unwrap();
    let nested = t
        .spans
        .iter()
        .find(|s| s.op.as_deref() == Some("markdown.render"))
        .unwrap();
    assert_eq!(apply.trace_id, root.trace_id);
    assert_eq!(apply.parent_span_id, Some(root.span_id));
    assert_eq!(nested.trace_id, root.trace_id);
    assert_eq!(nested.parent_span_id, Some(apply.span_id));
    assert!(t.timestamp.unwrap() >= apply.timestamp.unwrap());
}

#[derive(Default)]
struct TimingCapture(std::sync::Mutex<Vec<sentry::Envelope>>);
impl sentry::Transport for TimingCapture {
    fn send_envelope(&self, envelope: sentry::Envelope) {
        // Exercise the unchanged final egress allowlist, without any network.
        self.0.lock().unwrap().push(privacy::envelope(envelope));
    }
}
fn timing_harness(rate: f32) -> (Arc<sentry::Hub>, Arc<TimingCapture>, Arc<transport::Gate>) {
    let capture = Arc::new(TimingCapture::default());
    let client = Arc::new(sentry::Client::from_config(sentry::ClientOptions {
        dsn: Some("https://public@example.invalid/42".parse().unwrap()),
        traces_sampler: Some(Arc::new(move |_| rate)),
        default_integrations: false,
        transport: Some(Arc::new(capture.clone())),
        ..Default::default()
    }));
    let hub = Arc::new(sentry::Hub::new(
        Some(client),
        Arc::new(sentry::Scope::default()),
    ));
    let gate = Arc::new(transport::Gate::default());
    gate.change(true);
    (hub, capture, gate)
}
fn start_timing(gate: &Arc<transport::Gate>, op: &'static str) -> Timing {
    Timing::with_gate(op, false, true, gate.clone())
}
fn captured_transactions(capture: &TimingCapture) -> Vec<p::Transaction<'static>> {
    capture
        .0
        .lock()
        .unwrap()
        .iter()
        .flat_map(|e| e.items())
        .filter_map(|i| {
            if let p::EnvelopeItem::Transaction(t) = i {
                Some(t.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn timing_late_callback_finishes_child_before_root_and_cancellation_releases_lease() {
    let (hub, capture, gate) = timing_harness(1.0);
    sentry::Hub::run(hub.clone(), || {
        let wait = start_timing(&gate, "ipc.wait");
        let parent = wait.context();
        drop(wait); // Models receiver timeout while UI work remains queued.
        assert!(captured_transactions(&capture).is_empty());
        let worker_gate = gate.clone();
        std::thread::spawn(move || {
            sentry::Hub::run(hub, || {
                let _scope = parent.enter();
                let _apply = start_timing(&worker_gate, "ipc.update");
                let _nested = start_timing(&worker_gate, "storage.save");
            })
        })
        .join()
        .unwrap();
        let transactions = captured_transactions(&capture);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].spans.len(), 2);
        for child in &transactions[0].spans {
            assert!(transactions[0].timestamp.unwrap() >= child.timestamp.unwrap());
        }
        let wait = start_timing(&gate, "ipc.wait");
        let cancelled = wait.context();
        drop(wait);
        assert_eq!(captured_transactions(&capture).len(), 1);
        drop(cancelled); // Models run_on_main_thread rejection/drop or expired callback.
        assert_eq!(captured_transactions(&capture).len(), 2);
    });
}

#[test]
fn timing_concurrent_requests_restore_destination_context_including_panic() {
    let (hub, capture, gate) = timing_harness(1.0);
    let (send, receive) = std::sync::mpsc::channel();
    let mut workers = Vec::new();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    for _ in 0..2 {
        let (hub, gate, send, barrier) = (hub.clone(), gate.clone(), send.clone(), barrier.clone());
        workers.push(std::thread::spawn(move || {
            sentry::Hub::run(hub, || {
                let root = start_timing(&gate, "ipc.wait");
                send.send(root.context()).unwrap();
                barrier.wait();
            })
        }));
    }
    let first = receive.recv().unwrap();
    let second = receive.recv().unwrap();
    sentry::Hub::run(hub, || {
        let ambient = start_timing(&gate, "terminal.start");
        let ambient_trace = ambient
            .context
            .span
            .as_ref()
            .unwrap()
            .span
            .get_trace_context()
            .trace_id;
        for parent in [first, second] {
            let parent_trace = parent
                .span
                .as_ref()
                .unwrap()
                .span
                .get_trace_context()
                .trace_id;
            assert_ne!(parent_trace, ambient_trace);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _scope = parent.enter();
                let child = start_timing(&gate, "ipc.open");
                assert_eq!(
                    child
                        .context
                        .span
                        .as_ref()
                        .unwrap()
                        .span
                        .get_trace_context()
                        .trace_id,
                    parent_trace
                );
                panic!("callback panic");
            }));
            assert!(result.is_err());
            let restored = start_timing(&gate, "storage.save");
            assert_eq!(
                restored
                    .context
                    .span
                    .as_ref()
                    .unwrap()
                    .span
                    .get_trace_context()
                    .trace_id,
                ambient_trace
            );
        }
    });
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    let transactions = captured_transactions(&capture);
    assert_eq!(transactions.len(), 3);
    let waits: Vec<_> = transactions
        .iter()
        .filter(|t| t.name.as_deref() == Some("ipc.wait"))
        .collect();
    assert_eq!(waits.len(), 2);
    assert_ne!(waits[0].contexts["trace"], waits[1].contexts["trace"]);
    for wait in waits {
        assert_eq!(wait.spans.len(), 1);
        assert_eq!(wait.spans[0].status, Some(p::SpanStatus::InternalError));
        let p::Context::Trace(root) = &wait.contexts["trace"] else {
            panic!("trace")
        };
        assert_eq!(wait.spans[0].parent_span_id, Some(root.span_id));
    }
}

#[test]
fn timing_denies_revoked_disabled_and_prior_epoch_work_after_reactivation() {
    let (hub, capture, gate) = timing_harness(1.0);
    sentry::Hub::run(hub, || {
        let wait = start_timing(&gate, "ipc.wait");
        let old = wait.context();
        gate.change(false);
        {
            let _scope = old.clone().enter();
            assert!(start_timing(&gate, "ipc.open").context.span.is_none());
        }
        gate.change(true);
        {
            let _scope = old.enter();
            assert!(start_timing(&gate, "ipc.open").context.span.is_none());
        }
        drop(wait);
        assert!(captured_transactions(&capture).is_empty());
        gate.change(false);
        let disabled = start_timing(&gate, "ipc.wait");
        let old = disabled.context();
        drop(disabled);
        gate.change(true);
        {
            let _scope = old.enter();
            assert!(start_timing(&gate, "ipc.open").context.span.is_none());
        }
        {
            let disabled = Timing::with_gate("ipc.wait", false, false, gate.clone());
            let parent = disabled.context();
            drop(disabled);
            let _scope = parent.enter();
            assert!(start_timing(&gate, "ipc.open").context.span.is_none());
        }
        drop(start_timing(&gate, "ipc.wait"));
        assert_eq!(captured_transactions(&capture).len(), 1);
    });
}

#[test]
fn timing_children_inherit_unsampled_parent_without_resampling_and_polling_is_capped() {
    for rate in [0.0, 1.0] {
        let (hub, capture, gate) = timing_harness(rate);
        sentry::Hub::run(hub.clone(), || {
            let root = start_timing(&gate, "ipc.wait");
            let context = root.context();
            let worker_gate = gate.clone();
            std::thread::spawn(move || {
                sentry::Hub::run(hub, || {
                    let _scope = context.enter();
                    let _child = start_timing(&worker_gate, "ipc.inspect");
                    let _snapshot = start_timing(&worker_gate, "snapshot.request");
                })
            })
            .join()
            .unwrap();
        });
        assert_eq!(
            captured_transactions(&capture).len(),
            if rate == 0.0 { 0 } else { 1 }
        );
    }
    let sampler = options("https://public@example.invalid/42".parse().unwrap(), true)
        .traces_sampler
        .unwrap();
    let mut polling = sentry::TransactionContext::new("ipc.wait", "ipc.wait");
    polling.custom_insert("ipc.polling".into(), json!(true));
    assert!(sampler(&polling) <= 0.01);
    // Verify the root actually supplies that flag to the SDK, not just the callback.
    let (hub, _, gate) = timing_harness(1.0);
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let client = Arc::new(sentry::Client::from_config(sentry::ClientOptions {
        dsn: Some("https://public@example.invalid/42".parse().unwrap()),
        traces_sampler: Some(Arc::new(move |ctx| {
            assert_eq!(ctx.name(), "ipc.wait");
            assert_eq!(ctx.custom().unwrap().get("ipc.polling"), Some(&json!(true)));
            seen.fetch_add(1, Ordering::SeqCst);
            0.0
        })),
        default_integrations: false,
        transport: Some(Arc::new(Arc::new(TimingCapture::default()))),
        ..Default::default()
    }));
    hub.bind_client(Some(client));
    sentry::Hub::run(hub, || {
        drop(Timing::with_gate("ipc.wait", true, true, gate));
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn timing_late_finish_uses_originating_client_scope_and_restores_destination_hub() {
    let (hub, capture, gate) = timing_harness(1.0);
    hub.configure_scope(|s| s.set_tag("window.kind", "memo"));
    let parent = sentry::Hub::run(hub.clone(), || {
        let wait = start_timing(&gate, "ipc.wait");
        wait.context()
    });
    let destination = Arc::new(sentry::Hub::new_from_top(hub));
    destination.configure_scope(|s| s.set_tag("window.kind", "terminal"));
    let original_client = parent.span.as_ref().unwrap().hub.client().unwrap();
    // Rebinding the destination must not rebind the lease's originating client.
    let replacement = Arc::new(sentry::Client::from_config(sentry::ClientOptions {
        default_integrations: false,
        ..Default::default()
    }));
    destination.bind_client(Some(replacement.clone()));
    sentry::Hub::run(destination, || {
        assert!(!Arc::ptr_eq(
            &original_client,
            &sentry::Hub::current().client().unwrap()
        ));
        let _scope = parent.enter();
        let child = start_timing(&gate, "ipc.open");
        assert!(Arc::ptr_eq(
            &original_client,
            &child.context.span.as_ref().unwrap().hub.client().unwrap()
        ));
        drop(child);
    });
    let transactions = captured_transactions(&capture);
    assert_eq!(transactions.len(), 1);
    assert_eq!(
        transactions[0].tags.get("window.kind").map(String::as_str),
        Some("memo")
    );
    assert_eq!(transactions[0].spans.len(), 1);
}

#[test]
fn snapshot_summary_is_denied_at_egress_but_individual_request_survives() {
    let mut input = sentry::Envelope::new();
    input.add_item(p::Transaction {
        name: Some("snapshot".into()),
        ..Default::default()
    });
    input.add_item(p::Transaction {
        name: Some("snapshot.request".into()),
        ..Default::default()
    });
    let out = privacy::envelope(input);
    let names: Vec<_> = out
        .items()
        .filter_map(|i| {
            if let p::EnvelopeItem::Transaction(t) = i {
                t.name.as_deref()
            } else {
                None
            }
        })
        .collect();
    assert_eq!(names, vec!["snapshot.request"]);
    // A legacy UI may still send bounded snapshot samples. Accepting the input
    // preserves heartbeat/watchdog compatibility, independently of trace egress.
    assert!(valid_measurement(&Measurement {
        op: "snapshot".into(),
        duration_ms: 12.,
        count: 2,
        failure_count: 0
    }));
}

#[test]
fn every_child_span_gets_trusted_platform_build_metadata_at_final_egress() {
    let mut input = sentry::Envelope::new();
    let mut t = p::Transaction {
        name: Some("ipc.open".into()),
        ..Default::default()
    };
    for forged in [false, true] {
        let mut child = p::Span {
            op: Some("markdown.render".into()),
            ..Default::default()
        };
        if forged {
            child.tags.insert("app.os".into(), "windows".into());
            child.tags.insert("app.arch".into(), "x86".into());
            child.tags.insert("app.build".into(), "forged".into());
        }
        child.tags.insert("private".into(), "PRIVATE_NOTE".into());
        t.spans.push(child);
    }
    input.add_item(t);
    let out = privacy::envelope(input);
    let t = out
        .items()
        .find_map(|i| {
            if let p::EnvelopeItem::Transaction(t) = i {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(t.spans.len(), 2);
    for child in &t.spans {
        assert_eq!(
            child.tags.get("app.os").map(String::as_str),
            Some(std::env::consts::OS)
        );
        assert_eq!(
            child.tags.get("app.arch").map(String::as_str),
            Some(std::env::consts::ARCH)
        );
        assert_eq!(
            child.tags.get("app.build").map(String::as_str),
            Some(env!("AGENT_GIBBON_BUILD_REVISION"))
        );
        assert!(!child.tags.contains_key("private"));
    }
}

#[test]
fn legacy_snapshot_summary_never_creates_a_transaction_or_blocks_other_measurements() {
    let (hub, capture, _) = timing_harness(1.0);
    sentry::Hub::run(hub, || {
        report_measurement(
            Measurement {
                op: "snapshot".into(),
                duration_ms: 5000.,
                count: 2,
                failure_count: 1,
            },
            "memo",
        );
        assert!(captured_transactions(&capture).is_empty());
        report_measurement(
            Measurement {
                op: "save_note".into(),
                duration_ms: 2.,
                count: 1,
                failure_count: 0,
            },
            "memo",
        );
    });
    let transactions = captured_transactions(&capture);
    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].name.as_deref(), Some("save_note"));
    assert_eq!(
        transactions[0]
            .tags
            .get("measurement.origin")
            .map(String::as_str),
        Some("webview.aggregate")
    );
}

#[test]
fn terminal_blocking_worker_keeps_request_parent() {
    let (hub, capture, gate) = timing_harness(1.0);
    sentry::Hub::run(hub.clone(), || {
        let request = start_timing(&gate, "terminal.start");
        let worker_gate = gate.clone();
        let work = move || {
            sentry::Hub::run(hub, || {
                let _worker = start_timing(&worker_gate, "terminal.pty.create");
            })
        };
        tauri::async_runtime::block_on(tauri::async_runtime::spawn_blocking(work)).unwrap();
        drop(request);
    });
    let transactions = captured_transactions(&capture);
    assert_eq!(
        transactions.len(),
        1,
        "request and blocking worker must share one trace"
    );
    let t = &transactions[0];
    let p::Context::Trace(root) = &t.contexts["trace"] else {
        panic!("trace")
    };
    assert_eq!(t.spans.len(), 1);
    assert_eq!(t.spans[0].trace_id, root.trace_id);
    assert_eq!(t.spans[0].parent_span_id, Some(root.span_id));
}

#[test]
fn capture_worker_then_ui_callback_keeps_request_parent() {
    let (hub, capture, gate) = timing_harness(1.0);
    sentry::Hub::run(hub.clone(), || {
        let request = start_timing(&gate, "capture.prepare");
        std::thread::spawn(move || {
            // Represents the detached worker -> queued UI closure boundary.
            std::thread::spawn(move || {
                sentry::Hub::run(hub, || {
                    let _capture = start_timing(&gate, "capture.screen.capture");
                })
            })
            .join()
            .unwrap();
        })
        .join()
        .unwrap();
        drop(request);
    });
    let transactions = captured_transactions(&capture);
    assert_eq!(
        transactions.len(),
        1,
        "request, worker and UI callback must share one trace"
    );
    let t = &transactions[0];
    let p::Context::Trace(root) = &t.contexts["trace"] else {
        panic!("trace")
    };
    assert_eq!(t.spans.len(), 1);
    assert_eq!(t.spans[0].trace_id, root.trace_id);
    assert_eq!(t.spans[0].parent_span_id, Some(root.span_id));
}
