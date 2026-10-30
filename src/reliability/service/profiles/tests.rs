use super::*;

fn content_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn transport_json() -> serde_json::Value {
    serde_json::json!({
        "schema": crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA,
        "profile_id": "coordination-transport",
        "profile_ref": content_ref("transport"),
        "adapter_kind": "iroh-live",
        "capabilities": ["bidirectional-streams"],
        "limits": {
            "max_listeners": 1, "max_sessions": 8, "max_streams_per_session": 2,
            "max_frame_bytes": 1024, "max_datagram_bytes": 128, "max_queued_events": 8,
            "max_queued_bytes": 4096, "max_inflight_bytes": 4096, "operation_deadline_ticks": 100
        },
        "non_claims": crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.map(|value| value.as_str())
    })
}

fn durable_json() -> serde_json::Value {
    serde_json::json!({
        "schema": crate::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA,
        "profile_id": "coordination-durable", "profile_ref": content_ref("durable"),
        "adapter_kind": "live-redb", "supported_levels": ["machine-loss"],
        "max_namespaces": 1, "max_log_records": 100, "max_ordered_entries": 100,
        "max_operation_bytes": 1024, "max_namespace_bytes": 4096, "max_batch_operations": 8,
        "max_snapshots": 4, "max_effect_transactions": 4,
        "non_claims": crate::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.map(|value| value.as_str())
    })
}

#[test]
fn transport_codec_admits_iroh_profile_and_enforces_limits() {
    let input = transport_json();
    let admitted = serde_json::from_value::<TransportProfileDto>(input.clone()).unwrap().into_profile().unwrap();
    assert_eq!(admitted.profile.adapter_kind, crate::fabric_transport::TransportAdapterKind::IrohLive);
    assert_eq!(admitted.profile.limits.max_frame_bytes, 1024);
    let mut invalid = input;
    invalid["limits"]["max_frame_bytes"] = serde_json::json!(4097);
    assert!(serde_json::from_value::<TransportProfileDto>(invalid).unwrap().into_profile().is_err());
}

#[test]
fn protocol_codec_rejects_mismatched_profile_and_unsupported_capability() {
    let profile = serde_json::from_value::<TransportProfileDto>(transport_json()).unwrap().into_profile().unwrap();
    let input = serde_json::json!({
        "schema": crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA, "protocol_id": "coordination", "version": "v1",
        "alpn": "molten/coordination/1", "extension_id": "coordination", "service_id": "coordination",
        "generation": 1, "listener_limit": 1, "requested_capabilities": ["bidirectional-streams"],
        "framing": { "profile_id": "bounded-frame", "profile_ref": content_ref("framing"),
            "max_frame_bytes": 1024, "length_prefix_bytes": 8, "payload_hash_required": true },
        "cleanup_policy": { "kind": "bounded-drain", "grace_ticks": 2 },
        "registration_authority_ref": content_ref("separately-signed-grant"),
        "profile_ref": profile.profile.profile_ref
    });
    let descriptor = serde_json::from_value::<ProtocolDescriptorDto>(input.clone())
        .unwrap()
        .into_descriptor(&profile)
        .unwrap();
    assert_eq!(descriptor.framing.max_frame_bytes, 1024);
    let mut mismatched = input.clone();
    mismatched["profile_ref"] = serde_json::json!(content_ref("different-profile"));
    assert!(
        serde_json::from_value::<ProtocolDescriptorDto>(mismatched)
            .unwrap()
            .into_descriptor(&profile)
            .is_err()
    );
    let mut unsupported = input;
    unsupported["requested_capabilities"] = serde_json::json!(["datagrams"]);
    assert!(
        serde_json::from_value::<ProtocolDescriptorDto>(unsupported)
            .unwrap()
            .into_descriptor(&profile)
            .is_err()
    );
}

#[test]
fn durable_codec_enforces_namespace_atomicity_binding() {
    let profile = serde_json::from_value::<DurableStateProfileDto>(durable_json()).unwrap().into_profile().unwrap();
    assert_eq!(profile.profile.adapter_kind, crate::fabric_durability::DurableAdapterKind::LiveRedb);
    let input = serde_json::json!({
        "schema": crate::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA, "profile_ref": profile.profile.profile_ref,
        "adapter_id": "redb", "namespace_id": "coordination", "generation": 1,
        "value_schema_ref": content_ref("value-schema"), "retention_authority_ref": null, "quota_bytes": 4096,
        "atomicity_domain": { "domain_id": "coordination-domain", "adapter_id": "redb",
            "namespace_id": "coordination", "generation": 1, "object_classes": ["log-record", "ordered-value"],
            "max_operations": 8, "max_bytes": 4096, "supported_levels": ["machine-loss"] }
    });
    let admitted = serde_json::from_value::<DurableNamespaceDescriptorDto>(input.clone())
        .unwrap()
        .into_descriptor(&profile)
        .unwrap();
    assert_eq!(admitted.atomicity_domain.generation, admitted.generation);
    let mut invalid = input;
    invalid["atomicity_domain"]["generation"] = serde_json::json!(2);
    assert!(
        serde_json::from_value::<DurableNamespaceDescriptorDto>(invalid)
            .unwrap()
            .into_descriptor(&profile)
            .is_err()
    );
    let mut zero = durable_json();
    zero["max_operation_bytes"] = serde_json::json!(0);
    assert!(serde_json::from_value::<DurableStateProfileDto>(zero).unwrap().into_profile().is_err());
}

#[test]
fn time_codec_admits_real_scheduler_policy_and_rejects_inconsistent_limits() {
    let input = serde_json::json!({
        "schema": crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA, "profile_id": "coordination-time",
        "profile_ref": content_ref("time"), "kind": "live",
        "supported_domains": ["wall-clock", "monotonic", "logical", "virtual"],
        "max_duration_ticks": 1000, "max_uncertainty_ticks": 10, "max_timers": 8, "max_runnables": 8,
        "max_entropy_request_bytes": 32, "max_entropy_total_bytes": 128,
        "max_scheduler_concurrency": 2, "max_scheduler_queue_depth": 8, "fairness_bound_turns": null,
        "scheduler_policy": { "ordering": "fifo", "replay": "recorded-choice-required", "overload": "reject" },
        "evidence_mode": "aggregate", "non_claims": crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.map(|value| value.as_str())
    });
    let admitted = serde_json::from_value::<TimeProfileDescriptorDto>(input.clone())
        .unwrap()
        .into_descriptor()
        .unwrap();
    assert_eq!(admitted.profile.kind, crate::fabric_time::TimeProfileKind::Live);
    assert_eq!(
        admitted.profile.scheduler_policy.replay,
        crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired
    );
    let mut invalid = input;
    invalid["max_entropy_request_bytes"] = serde_json::json!(129);
    assert!(serde_json::from_value::<TimeProfileDescriptorDto>(invalid).unwrap().into_descriptor().is_err());
}

#[test]
fn serde_bounds_and_closed_vocabulary_fail_without_authority_defaults() {
    let mut input = transport_json();
    input["profile_id"] = serde_json::json!("x".repeat(257));
    assert!(serde_json::from_value::<TransportProfileDto>(input).is_err());
    let mut input = transport_json();
    input["non_claims"] = serde_json::json!(vec!["does-not-prove-consensus"; 4097]);
    assert!(serde_json::from_value::<TransportProfileDto>(input).is_err());
    let mut input = transport_json();
    input["adapter_kind"] = serde_json::json!("trusted-transport");
    assert!(serde_json::from_value::<TransportProfileDto>(input).unwrap().into_profile().is_err());
    let mut input = transport_json();
    input["authority"] = serde_json::json!(true);
    assert!(serde_json::from_value::<TransportProfileDto>(input).is_err());
    assert!(serde_json::from_value::<EndpointAdmissionStateDto>(serde_json::json!({})).is_err());
}

#[test]
fn endpoint_codec_preserves_inactive_facts_and_rejects_expired_cohort() {
    let input = serde_json::json!({
        "registration_active": false, "transport_capability_active": false,
        "protocol_capability_active": false, "profile_active": false, "listener_ready": false
    });
    let state = serde_json::from_value::<EndpointAdmissionStateDto>(input).unwrap().into_state().unwrap();
    assert!(!state.registration_active);
    assert!(!state.transport_capability_active);
    assert!(!state.listener_ready);
    let cohort = EndpointValidityCohortDto {
        cohort_ref: content_ref("cohort"),
        not_before_tick: 5,
        expires_at_tick: 5,
    };
    assert!(cohort.into_cohort().is_err());
}

#[test]
fn experimental_recipe_binds_limits_and_preserves_explicit_authority() {
    let input = ExperimentalInput {
        service_id: "coordination".into(),
        extension_id: "coordination-extension".into(),
        protocol_id: "coordination-protocol".into(),
        adapter_id: "coordination-redb".into(),
        namespace_id: "coordination".into(),
        registration_authority_ref: content_ref("signed-grant"),
        value_schema_ref: content_ref("coordination-value-schema"),
        operation_deadline_ticks: 1000,
        max_duration_ticks: 10000,
    };
    let profiles = experimental(input.clone()).unwrap();
    assert_eq!(profiles.protocol.registration_authority_ref, input.registration_authority_ref);
    assert_eq!(profiles.namespace.value_schema_ref, input.value_schema_ref);
    let mut changed = input.clone();
    changed.operation_deadline_ticks += 1;
    let changed_profiles = experimental(changed).unwrap();
    assert_ne!(profiles.transport.profile_ref, changed_profiles.transport.profile_ref);
    let mut malformed = input.clone();
    malformed.registration_authority_ref.clear();
    assert!(experimental(malformed).is_err());
    let mut unbounded = input;
    unbounded.max_duration_ticks = 0;
    assert!(experimental(unbounded).is_err());
}
