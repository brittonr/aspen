use super::*;

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

// r[verify linux_ebpf_observability.admission]
// r[verify linux_ebpf_observability.failure]
#[test]
fn substituted_bytes_spend_read_capability() {
    let (profile, adapter, window, _) = fixture();
    let workspace = crate::test_support::process_workspace("ebpf-copied-window").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).unwrap();
    let leaf = crate::node_state::RelativePath::parse("window.json").unwrap();
    let admitted = serde_json::to_vec(&window).unwrap();
    let selected = crate::preserves_rail::content_ref_from_bytes(&admitted);
    root.write(&leaf, &admitted).unwrap();
    let open = || {
        EbpfEndpointReader::open(&root, leaf.clone(), &profile, &adapter, EbpfEndpointSelection {
            epoch: window.generation,
            deadline: 15,
            content_ref: Some(&selected),
        })
        .unwrap()
    };
    let mut reader = open();
    let mut changed = window.clone();
    changed.records[0].value += 1;
    let replacement = serde_json::to_vec(&changed).unwrap();
    let decoded = decode_ebpf_window(&replacement, adapter.max_bytes as u64).unwrap();
    admit_ebpf_window(&adapter, &decoded, window.generation, 10).expect("otherwise valid same-cohort window");
    root.write(&leaf, &replacement).unwrap();
    assert!(matches!(reader.poll(10, false), Err(EbpfEndpointFailure::Admission(EbpfIssue::Binding))));

    root.write(&leaf, &admitted).unwrap();
    assert!(matches!(reader.poll(10, false), Err(EbpfEndpointFailure::Read(EbpfReadFailure::Closed))));
    let readback = open().poll(10, false).expect("fresh capability over the exact bytes");
    assert_eq!(readback.wire_ref, selected);
    assert_eq!(readback.endpoint_ref, canonical_ebpf_window_ref(&window).unwrap());
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn missing_window_spends_read_capability() {
    let (profile, adapter, window, _) = fixture();
    let workspace = crate::test_support::process_workspace("ebpf-missing-window").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).unwrap();
    let leaf = crate::node_state::RelativePath::parse("window.json").unwrap();
    let bytes = serde_json::to_vec(&window).unwrap();
    let selected = crate::preserves_rail::content_ref_from_bytes(&bytes);
    let open = || {
        EbpfEndpointReader::open(&root, leaf.clone(), &profile, &adapter, EbpfEndpointSelection {
            epoch: window.generation,
            deadline: 15,
            content_ref: Some(&selected),
        })
        .unwrap()
    };
    let mut reader = open();
    assert!(matches!(reader.poll(10, false), Err(EbpfEndpointFailure::Read(EbpfReadFailure::Missing))));
    root.write(&leaf, &bytes).unwrap();
    assert!(matches!(reader.poll(10, false), Err(EbpfEndpointFailure::Read(EbpfReadFailure::Closed))));
    assert_eq!(open().poll(10, false).expect("new read capability").wire_ref, selected);
}

// r[verify linux_ebpf_observability.failure]
// r[verify linux_ebpf_observability.confidentiality]
#[test]
fn oversized_sum_is_unavailable_before_metric_projection() {
    let (profile, mut adapter, mut window, context) = fixture();
    let workspace = crate::test_support::process_workspace("ebpf-metric-range").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).unwrap();
    let leaf = crate::node_state::RelativePath::parse("window.json").unwrap();
    window.records[0].value = i64::MAX as u64;
    window.records[1].signal = window.records[0].signal;
    for (increment, required) in [(0, true), (1, true), (1, false)] {
        window.records[1].value = increment;
        adapter.required = required;
        root.write(&leaf, &serde_json::to_vec(&window).unwrap()).unwrap();
        let observation = observe_linux_ebpf(EbpfServiceInput {
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
        .expect("the service returns a typed observation outcome");
        if increment == 0 {
            assert_eq!(observation.status.artifact.kind, AdapterOutcomeKind::Exported);
            assert!(observation.samples.iter().any(|sample| sample.artifact.value == i64::MAX));
            assert!(!observation.deny_required_readiness);
        } else {
            assert_eq!(observation.status.artifact.kind, AdapterOutcomeKind::Corrupt);
            assert!(observation.samples.is_empty());
            assert_eq!(observation.health.artifact.state, HealthState::Unavailable);
            assert_eq!(observation.deny_required_readiness, required);
        }
    }
}
