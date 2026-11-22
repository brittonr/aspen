fn bytes(required: bool) -> (Vec<u8>, Vec<u8>) {
    let reference = crate::preserves_rail::content_ref_from_bytes(b"fixture binding");
    let binding = serde_json::json!({
        "machine": "fixture-machine", "generation": "fixture-generation",
        "pack": format!("onix:blake3:bpf-pack:{:064x}", 1),
        "kernel": format!("onix:blake3:kernel-build:{:064x}", 2),
        "program": reference, "object": reference, "btf": reference,
        "schema": crate::fabric_observability::EBPF_WINDOW_SCHEMA,
        "scope": reference, "accounting": reference, "adapter": reference,
    });
    let profile = serde_json::json!({
        "schema": "molten.linux-ebpf-observation.profile.v1",
        "profile_ref": reference, "binding": binding,
        "mappings": [{"signal":"process", "descriptor_ref":reference, "aggregation":"sum", "labels":[]}],
        "max_records":32, "max_bytes":4096, "max_streams":8,
        "max_queue_records":32, "max_queue_bytes":4096, "max_series":1, "max_events":1,
        "window_ticks":10, "freshness_ticks":5, "required":required,
        "loss_policy":"incomplete-on-any-loss-or-missing-accounting", "event_detail":"status-only",
        "redaction":"deny-unmapped-and-all-free-text", "scope":"local-component",
        "non_claims":crate::fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.map(crate::fabric_observability::ObservabilityNonClaim::as_str),
    });
    let window = serde_json::json!({
        "schema": crate::fabric_observability::EBPF_WINDOW_SCHEMA, "binding":binding, "generation":7, "start":10, "end":20,
        "records":[{"stream":0,"sequence":0,"tick":15,"signal":"process","value":2}],
        "accounting":{"attempts":1,"submitted":1,"producer_dropped":0,"consumer_dropped":0,"pending":0},
        "final_close_succeeded":true, "cancelled":false, "cross_stream_total_order_claimed":false,
    });
    (serde_json::to_vec(&profile).unwrap(), serde_json::to_vec(&window).unwrap())
}

fn setup(required: bool) -> super::Fixture {
    let (profile, window) = bytes(required);
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
        f.root.create_dir_all(&crate::node_state::RelativePath::parse(directory).unwrap()).unwrap();
        f.root.write(&crate::node_state::RelativePath::parse(path).unwrap(), &bytes).unwrap();
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
    let f = setup(true);
    let good = admitted(&f).poll_window(tick(&f)).unwrap().observation;
    assert_eq!(good.health.artifact.state, crate::fabric_observability::HealthState::Healthy);
    assert_eq!(good.health.artifact.context.scope, crate::fabric_observability::ClaimScope::LocalComponent);
    assert!(good.health.artifact.context.evidence_refs.contains(&f.selection.endpoint_content_ref));
    assert!(!good.deny_required_readiness);
    assert!(super::super::observerlease::admit(&f.root, &f.selection, tick(&f).unix_seconds).is_err());
}

// r[verify linux_ebpf_observability.admission]
#[test]
fn replaced_profile_or_window_never_satisfies_required_readiness() {
    let (mut profile, mut window) = bytes(true);
    profile.push(b'\n');
    window.push(b'\n');
    let f = setup(true);
    let lease = admitted(&f);
    f.root
        .write(&crate::node_state::RelativePath::parse("checked/profile.json").unwrap(), &profile)
        .unwrap();
    assert!(lease.poll_window(tick(&f)).is_err());
    let f = setup(true);
    let lease = admitted(&f);
    f.root
        .write(&crate::node_state::RelativePath::parse("receipts/provider/snapshot").unwrap(), &window)
        .unwrap();
    let refused = lease.poll_window(tick(&f)).unwrap();
    assert!(refused.readiness.is_none());
    assert_eq!(refused.observation.status.artifact.kind, crate::fabric_observability::AdapterOutcomeKind::Corrupt);
    assert!(refused.observation.deny_required_readiness);
    assert!(refused.observation.samples.is_empty());
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn wrong_boot_cancellation_and_expired_poll_deny_required_window() {
    let f = setup(true);
    let other_boot = super::super::ObserverWindowTick {
        boot_id: "other boot",
        ..tick(&f)
    };
    assert!(admitted(&f).poll_window(other_boot).is_err());
    for (cancelled, deadline_ns, expected) in [
        (true, 25, crate::fabric_observability::AdapterOutcomeKind::Cancelled),
        (false, 19, crate::fabric_observability::AdapterOutcomeKind::Timeout),
    ] {
        let f = setup(true);
        let observed = admitted(&f)
            .poll_window(super::super::ObserverWindowTick {
                cancelled,
                deadline_ns,
                ..tick(&f)
            })
            .unwrap();
        assert!(observed.readiness.is_none());
        assert_eq!(observed.observation.status.artifact.kind, expected);
        assert!(observed.observation.deny_required_readiness);
        assert!(observed.observation.samples.is_empty());
    }
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn required_readiness_expires_and_rechecks_authority() {
    let mut f = setup(true);
    let now = tick(&f).unix_seconds;
    let window = admitted(&f).poll_window(tick(&f)).unwrap();
    let readiness = window.readiness.expect("complete required window guards publication");
    assert!(readiness.validate(now, 19).is_err());
    readiness.validate(now, 25).expect("last valid monotonic tick");
    assert!(readiness.validate(now, 26).is_err());
    assert!(readiness.validate(f.trust.expires, 20).is_err());

    let token = basalt::ucan::CompactToken::new(&f.credential.token).unwrap();
    f.trust.revoked.push(basalt::ucan::proof_reference(&token).as_bytes().try_into().unwrap());
    super::provision(&f);
    assert!(readiness.validate(now, 25).is_err());
}

#[test]
fn optional_unavailable_selection_reports_status_without_gating_startup() {
    let f = setup(false);
    let observed = admitted(&f)
        .poll_window(super::super::ObserverWindowTick {
            cancelled: true,
            ..tick(&f)
        })
        .unwrap();
    assert_eq!(
        observed.observation.status.artifact.kind,
        crate::fabric_observability::AdapterOutcomeKind::Cancelled
    );
    assert!(!observed.observation.deny_required_readiness);
    assert!(observed.readiness.is_none());
}
