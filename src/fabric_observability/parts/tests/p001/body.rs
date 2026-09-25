
/// The tracing shell exports an event as its canonical reference, and a Prometheus shell refuses
/// events.
fn assert_tracing_event_export_only(profile: &ObservationProfile) {
    let tracing_adapter = adapter(ObservationAdapterClass::Tracing);
    let event = ObservationEvent {
        schema: OBSERVATION_EVENT_SCHEMA.to_string(),
        event_ref: test_ref("adapter-state-change"),
        event_kind: "adapter-state-change".to_string(),
        severity: EventSeverity::Info,
        context: context(),
        detail: "exporter admitted".to_string(),
        attributes: vec![MetricLabel {
            name: "service".to_string(),
            value: "extension-a".to_string(),
            class: LabelClass::Public,
        }],
    };
    let canonical_event = canonical_observation_event(profile, &event, OBSERVED_TICK).expect("canonical event");
    let event_request = AdapterDeliveryRequest {
        operation_ref: test_ref("event-export-operation"),
        adapter_ref: tracing_adapter.adapter_ref.clone(),
        payload_ref: canonical_event.artifact_ref.clone(),
        payload_bytes: u64::try_from(canonical_event.artifact_ref.len()).expect("event reference length"),
        submitted_tick: OBSERVED_TICK,
        deadline_tick: OBSERVED_TICK + ADAPTER_TIMEOUT_TICKS,
    };
    let mut event_sink = success_sink();
    let tracing_event = execute_event_export(
        AdapterDelivery {
            profile,
            adapter: &tracing_adapter,
            request: &event_request,
            state: &shell_state(true, 0),
            last_export_tick: None,
        },
        &event,
        &mut event_sink,
    )
    .expect("tracing event export");
    assert_eq!(tracing_event.payload, tracing_event.payload_ref.as_bytes());
    assert!(
        execute_event_export(
            AdapterDelivery {
                profile,
                adapter: &adapter(ObservationAdapterClass::Prometheus),
                request: &event_request,
                state: &shell_state(true, 0),
                last_export_tick: None
            },
            &event,
            &mut event_sink
        )
        .is_err()
    );
}

// r[verify molten.fabric_observability.failure_semantics]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn exporter_unavailable_backpressure_and_sink_failure_are_terminal_without_hidden_retry() {
    let profile = profile();
    let snapshot = snapshot();
    let adapter = adapter(ObservationAdapterClass::Prometheus);
    let request = export_request(&adapter, &snapshot, ExportFormat::Prometheus);
    let mut sink = success_sink();
    let unavailable = execute_snapshot_export(
        AdapterDelivery {
            profile: &profile,
            adapter: &adapter,
            request: &request,
            state: &shell_state(false, 0),
            last_export_tick: None,
        },
        &snapshot,
        ExportFormat::Prometheus,
        &mut sink,
    )
    .expect("unavailable outcome");
    assert_eq!(unavailable.outcome.artifact.kind, AdapterOutcomeKind::Unavailable);
    assert_eq!(sink.calls, 0);

    let mut pressure_sink = success_sink();
    let pressure = execute_snapshot_export(
        AdapterDelivery {
            profile: &profile,
            adapter: &adapter,
            request: &request,
            state: &shell_state(true, MAX_QUEUED_BYTES),
            last_export_tick: None,
        },
        &snapshot,
        ExportFormat::Prometheus,
        &mut pressure_sink,
    )
    .expect("backpressure outcome");
    assert_eq!(pressure.outcome.artifact.kind, AdapterOutcomeKind::Backpressure);
    assert_eq!(pressure_sink.calls, 0);

    let mut failed_sink = RecordingSink {
        calls: 0,
        completion: SinkCompletion {
            completed_tick: OBSERVED_TICK,
            dropped_observations: 0,
            failure: Some(AdapterFailureClass::AdapterFailure),
        },
        payload: Vec::new(),
    };
    let failed = execute_snapshot_export(
        AdapterDelivery {
            profile: &profile,
            adapter: &adapter,
            request: &request,
            state: &shell_state(true, 0),
            last_export_tick: None,
        },
        &snapshot,
        ExportFormat::Prometheus,
        &mut failed_sink,
    )
    .expect("failed outcome");
    assert_eq!(failed.outcome.artifact.kind, AdapterOutcomeKind::Failed);
    assert_eq!(failed_sink.calls, 1);
    assert!(failed.payload.is_empty());
}

fn shell_state(available: bool, queued_bytes: u64) -> ExportShellState {
    ExportShellState {
        available,
        queued_bytes,
        cancelled: false,
    }
}

fn integrity_plan(bytes: &[u8]) -> IntegrityPlan {
    IntegrityPlan {
        schema: INTEGRITY_PLAN_SCHEMA.to_string(),
        plan_ref: test_ref("integrity-plan"),
        profile_ref: profile().profile_ref,
        scope_ref: test_ref("integrity-scope"),
        generation: GENERATION_ONE,
        read_only: true,
        require_complete: true,
        max_items: MAX_SCAN_ITEMS,
        max_findings: MAX_FINDINGS,
        targets: vec![IntegrityTarget {
            item_ref: test_ref("integrity-item"),
            kind: IntegrityTargetKind::DurableRecord,
            expected_content_ref: Some(crate::preserves_rail::content_ref_from_bytes(bytes)),
            expected_length: Some(u64::try_from(bytes.len()).expect("fixture length")),
        }],
        resource_ref: test_ref("integrity-resource"),
        policy_refs: vec![test_ref("integrity-policy")],
        evidence_refs: vec![test_ref("integrity-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

// r[verify molten.fabric_observability.integrity_readonly]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn capability_rooted_durable_scan_detects_corruption_partial_and_overbound_without_mutation() {
    let original = b"durable-record";
    let workspace = temp_dir("observability-durable-scan");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Ledger, &workspace)
            .expect("ledger namespace");
    let path = crate::node_state::NodeStatePath::parse("record.bin").expect("record path");
    namespace.write(&path, original).expect("write record");
    let plan = integrity_plan(original);
    let binding = DurableScanBinding {
        item_ref: plan.targets[0].item_ref.clone(),
        path: path.clone(),
    };
    let control = ScanShellControl {
        max_items: MAX_SCAN_ITEMS,
        max_item_bytes: MAX_SCAN_BYTES,
        cancelled: false,
    };
    let passed = scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &control)
        .expect("scan pass");
    assert_eq!(passed.result.artifact.decision, IntegrityDecision::Pass);
    assert_eq!(namespace.read(&path, MAX_SCAN_BYTES).expect("readback"), original);

    let corrupt = b"corrupt-record";
    namespace.write(&path, corrupt).expect("write corruption fixture");
    let failed = scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &control)
        .expect("scan corruption");
    assert_eq!(failed.result.artifact.decision, IntegrityDecision::Fail);
    assert!(failed.result.artifact.findings.iter().any(|finding| finding.class == FindingClass::ContentMismatch));
    assert_eq!(namespace.read(&path, MAX_SCAN_BYTES).expect("corrupt readback"), corrupt);

    let cancelled =
        scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &ScanShellControl {
            cancelled: true,
            ..control.clone()
        })
        .expect("cancelled scan");
    assert_eq!(cancelled.result.artifact.decision, IntegrityDecision::Cancelled);

    let overbound = scan_durable_namespace(&profile(), &plan, &namespace, &[binding], &ScanShellControl {
        max_item_bytes: 1,
        ..control
    })
    .expect("overbound scan");
    assert!(overbound.result.artifact.findings.iter().any(|finding| finding.class == FindingClass::OverBound));
    std::fs::remove_dir_all(workspace).expect("remove integrity scan workspace");
}

struct FixtureContentSource {
    values: std::collections::BTreeMap<String, Vec<u8>>,
}

impl ReadOnlyContentSource for FixtureContentSource {
    fn observe_bounded(&self, item_ref: &str, max_bytes: u64) -> ReadOnlyContentObservation {
        match self.values.get(item_ref) {
            Some(bytes) if u64::try_from(bytes.len()).is_ok_and(|length| length <= max_bytes) => {
                ReadOnlyContentObservation {
                    status: ScanItemStatus::Present,
                    bytes: Some(bytes.clone()),
                    evidence_refs: vec![test_ref("content-source-evidence")],
                }
            }
            Some(_) => ReadOnlyContentObservation {
                status: ScanItemStatus::OverBound,
                bytes: None,
                evidence_refs: vec![test_ref("content-source-evidence")],
            },
            None => ReadOnlyContentObservation {
                status: ScanItemStatus::Missing,
                bytes: None,
                evidence_refs: vec![test_ref("content-source-evidence")],
            },
        }
    }
}

// r[verify molten.fabric_observability.adapter_contract]
// r[verify molten.fabric_observability.integrity_readonly]
#[test]
fn content_verification_and_simulation_sources_share_read_only_scan_semantics() {
    let bytes = b"content-value";
    let mut plan = integrity_plan(bytes);
    plan.targets[0].kind = IntegrityTargetKind::Content;
    let source = FixtureContentSource {
        values: std::collections::BTreeMap::from([(plan.targets[0].item_ref.clone(), bytes.to_vec())]),
    };
    let execution = scan_content_source(&profile(), &plan, &source, &ScanShellControl {
        max_items: MAX_SCAN_ITEMS,
        max_item_bytes: MAX_SCAN_BYTES,
        cancelled: false,
    })
    .expect("content scan");
    assert_eq!(execution.result.artifact.decision, IntegrityDecision::Pass);
}
