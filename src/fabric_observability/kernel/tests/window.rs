use super::*;

fn reference(index: u8) -> String {
    format!("blake3:{index:064x}")
}

fn observation_profile() -> ObservationProfile {
    ObservationProfile {
        schema: OBSERVATION_PROFILE_SCHEMA.into(),
        profile_id: "synthetic-linux-source".into(),
        profile_ref: reference(1),
        bounds: ObservationBounds {
            max_descriptors: 6,
            max_labels_per_sample: 1,
            max_label_name_bytes: 32,
            max_label_value_bytes: 64,
            max_series: 6,
            max_events: 1,
            max_event_detail_bytes: 128,
            max_queued_bytes: 8192,
            max_snapshots: 1,
            max_scan_items: 1,
            max_findings: 1,
            max_diagnostics: 16,
            min_export_interval_ticks: 1,
        },
        redaction_rules: Vec::new(),
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn binding() -> EbpfBinding {
    EbpfBinding {
        machine: "synthetic-machine".into(),
        generation: "synthetic-generation".into(),
        pack: format!("onix:blake3:bpf-pack:{:064x}", 2),
        program: reference(3),
        object: reference(4),
        kernel: format!("onix:blake3:kernel-build:{:064x}", 5),
        btf: reference(6),
        schema: EBPF_WINDOW_SCHEMA.into(),
        scope: reference(7),
        accounting: reference(8),
        adapter: reference(9),
    }
}

fn mappings(signals: &[EbpfSignal], profile_ref: &str) -> Vec<EbpfMapping> {
    signals
        .iter()
        .enumerate()
        .map(|(index, signal)| EbpfMapping {
            signal: *signal,
            descriptor: MetricDescriptor {
                schema: METRIC_DESCRIPTOR_SCHEMA.into(),
                descriptor_id: format!("synthetic-signal-{index}"),
                descriptor_ref: reference(index as u8 + 20),
                profile_ref: profile_ref.to_string(),
                name: format!("synthetic-signal-{index}"),
                unit: "count".into(),
                kind: MetricKind::Counter,
                aggregation: MetricAggregation::Sum,
                allowed_label_names: Vec::new(),
                description: "Synthetic source facts, not live eBPF evidence".into(),
            },
        })
        .collect()
}

fn observation_context(profile_ref: &str) -> ObservationContext {
    ObservationContext {
        source_id: "synthetic-linux-source".into(),
        source_ref: reference(10),
        profile_ref: profile_ref.to_string(),
        scope: ClaimScope::LocalComponent,
        generation: 1,
        observed_tick: 10,
        valid_until_tick: 15,
        resource_ref: reference(7),
        evidence_refs: vec![reference(2)],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn fixture() -> (ObservationProfile, EbpfProfile, EbpfWindow, ObservationContext) {
    let profile = observation_profile();
    let binding = binding();
    let signals = [
        EbpfSignal::Process,
        EbpfSignal::Scheduler,
        EbpfSignal::Block,
        EbpfSignal::Network,
        EbpfSignal::Pressure,
        EbpfSignal::AdapterHealth,
    ];
    let mappings = mappings(&signals, &profile.profile_ref);
    let records = signals
        .iter()
        .enumerate()
        .map(|(index, signal)| EbpfRecord {
            stream: index as u32,
            sequence: 0,
            tick: 5,
            signal: *signal,
            value: index as u64 + 1,
        })
        .collect();
    let adapter = EbpfProfile {
        binding: binding.clone(),
        mappings,
        max_records: 32,
        max_bytes: 8192,
        max_streams: 8,
        window_ticks: 10,
        freshness_ticks: 5,
        required: true,
    };
    let window = EbpfWindow {
        schema: EBPF_WINDOW_SCHEMA.into(),
        binding,
        generation: 1,
        start: 0,
        end: 10,
        records,
        encoded_bytes: 0,
        accounting: Some(EbpfAccounting {
            attempts: 6,
            submitted: 6,
            producer_dropped: 0,
            consumer_dropped: 0,
            pending: 0,
        }),
        final_close_succeeded: true,
        cancelled: false,
        cross_stream_total_order_claimed: false,
    };
    let context = observation_context(&profile.profile_ref);
    (profile, adapter, window, context)
}

fn control() -> EbpfServiceControl {
    EbpfServiceControl {
        epoch: 1,
        now: 10,
        deadline: 15,
        cancelled: false,
    }
}

// r[verify linux_ebpf_observability.ordering]
// r[verify linux_ebpf_observability.verification]
#[test]
fn real_file_and_recording_projection_preserve_canonical_identity_under_interleaving() {
    let (profile, adapter, mut window, context) = fixture();
    let workspace = crate::test_support::process_workspace("ebpf-window-parity").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).unwrap();
    let leaf = crate::node_state::RelativePath::parse("window.json").unwrap();
    root.write(&leaf, &serde_json::to_vec(&window).unwrap()).unwrap();
    let first = observe_linux_ebpf(EbpfServiceInput {
        profile: &profile,
        adapter: &adapter,
        capability: Some(EbpfReadCapability {
            root: &root,
            leaf: leaf.clone(),
            content_ref: None,
        }),
        control: control(),
        context: context.clone(),
    })
    .unwrap();
    assert_eq!(first.status.artifact.kind, AdapterOutcomeKind::Exported);
    assert_eq!(first.samples.iter().map(|sample| sample.artifact.value).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
    let reference = canonical_ebpf_window_ref(&window).unwrap();
    window.records.reverse();
    assert_eq!(canonical_ebpf_window_ref(&window).unwrap(), reference);
    let mut recorded_context = first.health.artifact.context.clone();
    recorded_context.source_ref = reference;
    let recorded = canonical_ebpf_window(&profile, &adapter, &window, &recorded_context, 10).unwrap();
    root.write(&leaf, &serde_json::to_vec_pretty(&window).unwrap()).unwrap();
    let second = observe_linux_ebpf(EbpfServiceInput {
        profile: &profile,
        adapter: &adapter,
        capability: Some(EbpfReadCapability {
            root: &root,
            leaf,
            content_ref: None,
        }),
        control: control(),
        context,
    })
    .unwrap();
    let references = |samples: &[Artifact<MetricSample>]| {
        samples.iter().map(|sample| sample.artifact_ref.clone()).collect::<Vec<_>>()
    };
    assert_eq!(references(&first.samples), references(&second.samples));
    assert_eq!(references(&second.samples), references(&recorded.samples));
    assert_eq!(first.health.artifact_ref, second.health.artifact_ref);
    assert_eq!(first.status.artifact_ref, second.status.artifact_ref);
    assert_eq!(first.event.artifact_ref, second.event.artifact_ref);
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn partial_unavailable_cancelled_and_stale_required_sources_cannot_become_ready() {
    let (profile, adapter, mut window, context) = fixture();
    let missing = observe_linux_ebpf(EbpfServiceInput {
        profile: &profile,
        adapter: &adapter,
        capability: None,
        control: control(),
        context: context.clone(),
    })
    .unwrap();
    assert_eq!(missing.status.artifact.kind, AdapterOutcomeKind::Unavailable);
    assert_eq!(missing.health.artifact.state, HealthState::Unavailable);
    assert!(missing.deny_required_readiness);
    let workspace = crate::test_support::process_workspace("ebpf-window-loss").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).unwrap();
    let leaf = crate::node_state::RelativePath::parse("window.json").unwrap();
    window.accounting.as_mut().unwrap().attempts = 7;
    window.accounting.as_mut().unwrap().producer_dropped = 1;
    root.write(&leaf, &serde_json::to_vec(&window).unwrap()).unwrap();
    for (now, cancelled, expected) in [
        (10, false, AdapterOutcomeKind::Partial),
        (10, true, AdapterOutcomeKind::Cancelled),
        (16, false, AdapterOutcomeKind::Stale),
    ] {
        let observation = observe_linux_ebpf(EbpfServiceInput {
            profile: &profile,
            adapter: &adapter,
            capability: Some(EbpfReadCapability {
                root: &root,
                leaf: leaf.clone(),
                content_ref: None,
            }),
            control: EbpfServiceControl {
                epoch: 1,
                now,
                deadline: 20,
                cancelled,
            },
            context: context.clone(),
        })
        .unwrap();
        assert_eq!(observation.status.artifact.kind, expected);
        assert_eq!(observation.health.artifact.state, HealthState::Unavailable);
        assert!(observation.deny_required_readiness);
    }
}

// r[verify linux_ebpf_observability.confidentiality]
#[test]
fn unknown_schema_signals_and_private_fields_never_cross_the_window_decoder() {
    let (_, adapter, window, _) = fixture();
    for field in [
        "command_line",
        "payload",
        "packet_contents",
        "private_path",
        "socket",
        "peer",
        "secret",
        "kernel_pointer",
    ] {
        let mut changed = serde_json::to_value(&window).unwrap();
        changed["records"][0][field] = "prohibited-private-material".into();
        assert_eq!(
            decode_ebpf_window(&serde_json::to_vec(&changed).unwrap(), 8192),
            Err(EbpfEndpointFailure::Read(EbpfReadFailure::Malformed))
        );
    }
    let mut changed = serde_json::to_value(&window).unwrap();
    changed["records"][0]["signal"] = "unknown-signal".into();
    assert_eq!(
        decode_ebpf_window(&serde_json::to_vec(&changed).unwrap(), 8192),
        Err(EbpfEndpointFailure::Read(EbpfReadFailure::Malformed))
    );
    changed = serde_json::to_value(&window).unwrap();
    changed["schema"] = "unknown-schema".into();
    let decoded = decode_ebpf_window(&serde_json::to_vec(&changed).unwrap(), 8192).unwrap();
    assert_eq!(admit_ebpf_window(&adapter, &decoded, 1, 10), Err(EbpfIssue::Binding));
}
