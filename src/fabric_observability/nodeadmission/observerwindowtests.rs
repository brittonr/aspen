use crate::fabric_observability as observation;
use crate::node_state::RelativePath;

fn bytes() -> (Vec<u8>, Vec<u8>) {
    let reference = crate::preserves_rail::content_ref_from_bytes(b"fixture binding");
    let binding = serde_json::json!({
        "machine": "fixture-machine", "generation": "fixture-generation",
        "pack": format!("onix:blake3:bpf-pack:{:064x}", 1),
        "kernel": format!("onix:blake3:kernel-build:{:064x}", 2),
        "program": reference, "object": reference, "btf": reference,
        "schema": observation::EBPF_WINDOW_SCHEMA,
        "scope": reference, "accounting": reference, "adapter": reference,
    });
    let profile = serde_json::json!({
        "schema": "molten.linux-ebpf-observation.profile.v1",
        "profile_ref": reference, "binding": binding,
        "mappings": [{"signal":"process", "descriptor_ref":reference, "aggregation":"sum", "labels":[]}],
        "max_records":32, "max_bytes":4096, "max_streams":8,
        "max_queue_records":32, "max_queue_bytes":4096, "max_series":1, "max_events":1,
        "window_ticks":10, "freshness_ticks":5, "required":true,
        "loss_policy":"incomplete-on-any-loss-or-missing-accounting", "event_detail":"status-only",
        "redaction":"deny-unmapped-and-all-free-text", "scope":"local-component",
        "non_claims":observation::REQUIRED_OBSERVABILITY_NON_CLAIMS.map(observation::ObservabilityNonClaim::as_str),
    });
    let window = serde_json::json!({
        "schema": observation::EBPF_WINDOW_SCHEMA, "binding":binding, "generation":7, "start":10, "end":20,
        "records":[{"stream":0,"sequence":0,"tick":15,"signal":"process","value":2}],
        "accounting":{"attempts":1,"submitted":1,"producer_dropped":0,"consumer_dropped":0,"pending":0},
        "final_close_succeeded":true, "cancelled":false, "cross_stream_total_order_claimed":false,
    });
    (serde_json::to_vec(&profile).unwrap(), serde_json::to_vec(&window).unwrap())
}

fn setup() -> super::Fixture {
    let (profile, window) = bytes();
    let reference = crate::preserves_rail::content_ref_from_bytes(b"fixture source");
    let f = super::fixture_for(super::super::NodeObservationSelection {
        source_validation_ref: reference.clone(),
        checked_profile_path: "checked/profile.json".into(),
        checked_profile_ref: crate::preserves_rail::content_ref_from_bytes(&profile),
        endpoint_directory: "receipts/provider".into(),
        endpoint_leaf: "snapshot".into(),
        endpoint_content_ref: crate::preserves_rail::content_ref_from_bytes(&window),
        epoch: 7,
        authorization_tick: 1,
        boot_id: "12345678-1234-1234-1234-123456789abc".into(),
        onix_receipt_ref: reference,
    });
    for (directory, path, bytes) in [
        ("checked", "checked/profile.json", profile),
        ("receipts/provider", "receipts/provider/snapshot", window),
    ] {
        f.root.create_dir_all(&RelativePath::parse(directory).unwrap()).unwrap();
        f.root.write(&RelativePath::parse(path).unwrap(), &bytes).unwrap();
    }
    f
}

fn admitted(f: &super::Fixture) -> super::super::ObserverAuthorityLease {
    super::super::observerlease::admit(&f.root, &f.selection, f.trust.not_before + 60).unwrap()
}

fn tick(f: &super::Fixture) -> super::super::ObserverWindowTick<'_> {
    super::super::ObserverWindowTick {
        boot_id: &f.selection.boot_id,
        unix_seconds: f.trust.not_before + 60,
        monotonic_ns: 20,
        deadline_ns: 25,
        cancelled: false,
    }
}

// r[verify linux_ebpf_observability.boundary]
#[test]
fn selected_provider_bytes_project_only_local_health() {
    let f = setup();
    let good = admitted(&f).poll_window(tick(&f)).unwrap();
    assert_eq!(good.health.artifact.state, observation::HealthState::Healthy);
    assert_eq!(good.health.artifact.context.scope, observation::ClaimScope::LocalComponent);
    assert!(good.health.artifact.context.evidence_refs.contains(&f.selection.endpoint_content_ref));
    assert!(!good.deny_required_readiness);
    assert!(super::super::observerlease::admit(&f.root, &f.selection, tick(&f).unix_seconds).is_err());
}

// r[verify linux_ebpf_observability.admission]
#[test]
fn replaced_profile_or_window_never_satisfies_required_readiness() {
    let (mut profile, mut window) = bytes();
    profile.push(b'\n');
    window.push(b'\n');
    let f = setup();
    let lease = admitted(&f);
    f.root.write(&RelativePath::parse("checked/profile.json").unwrap(), &profile).unwrap();
    assert!(lease.poll_window(tick(&f)).is_err());
    let f = setup();
    let lease = admitted(&f);
    f.root.write(&RelativePath::parse("receipts/provider/snapshot").unwrap(), &window).unwrap();
    let refused = lease.poll_window(tick(&f)).unwrap();
    assert_eq!(refused.status.artifact.kind, observation::AdapterOutcomeKind::Corrupt);
    assert!(refused.deny_required_readiness);
    assert!(refused.samples.is_empty());
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn wrong_boot_cancellation_and_expired_poll_deny_required_window() {
    let f = setup();
    let other_boot = super::super::ObserverWindowTick {
        boot_id: "other boot",
        ..tick(&f)
    };
    assert!(admitted(&f).poll_window(other_boot).is_err());
    for (cancelled, deadline_ns, expected) in [
        (true, 25, observation::AdapterOutcomeKind::Cancelled),
        (false, 19, observation::AdapterOutcomeKind::Timeout),
    ] {
        let f = setup();
        let observed = admitted(&f)
            .poll_window(super::super::ObserverWindowTick {
                cancelled,
                deadline_ns,
                ..tick(&f)
            })
            .unwrap();
        assert_eq!(observed.status.artifact.kind, expected);
        assert!(observed.deny_required_readiness);
        assert!(observed.samples.is_empty());
    }
}
