// Maximum-width snapshot admission must precede service effects.
fn maximum_width_snapshot(retention: u64) -> crate::fabric_consistency::raft::ReplicaMessage {
    let mut snapshot = crate::fabric_consistency::raft::ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: reference("group"),
        membership_ref: reference("membership"),
        config_epoch: u64::MAX,
        fencing_epoch: u64::MAX,
        last_included_index: u64::MAX,
        last_included_term: u64::MAX,
        application_state_ref: reference("application-state"),
        completed_requests: (0..retention)
            .map(|offset| (reference(&format!("long-running-request-{offset}")), u64::MAX - offset))
            .collect(),
    };
    snapshot.snapshot_ref = crate::fabric_consistency::raft::snapshot_ref(&snapshot).unwrap();
    let leader = "a".repeat(256);
    crate::fabric_consistency::raft::replica_message(&crate::fabric_consistency::raft::ReplicaMessageEnvelope {
        group_binding_ref: snapshot.group_binding_ref.clone(),
        service_generation: u64::MAX,
        from: leader.clone(),
        to: "b".repeat(256),
        message: crate::fabric_consistency::raft::Message::InstallSnapshot {
            term: u64::MAX,
            leader_id: leader,
            snapshot: Box::new(snapshot),
            config_epoch: u64::MAX,
            fencing_epoch: u64::MAX,
        },
    })
    .unwrap()
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn snapshot_frame_admission_honors_maximum_width_encoded_boundary() {
    let output = super::materialize(input()).unwrap();
    let profile = output.configurations["node-1"].replica.decode();
    let message = maximum_width_snapshot(profile.request_retention_entries);
    let bytes = u64::try_from(message.bytes.len()).unwrap();
    super::super::snapshot::admit(&profile, bytes).unwrap();
    assert!(super::super::snapshot::admit(&profile, bytes - 1).is_err());
    let mut larger = profile.clone();
    larger.request_retention_entries += 1;
    assert!(super::super::snapshot::admit(&larger, bytes).is_err());
    let config = &output.configurations["node-1"];
    let transport = config.transport.profile.clone().into_profile().unwrap();
    let protocol = config.transport.protocol.clone().into_descriptor(&transport).unwrap();
    assert!(bytes <= protocol.framing.max_frame_bytes);
    assert_eq!(
        crate::fabric_consistency::raft::parse_replica_message(&message.bytes).unwrap().message.term(),
        u64::MAX
    );
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn incompatible_configurable_retention_is_denied_before_service_effects() {
    let input = input();
    let group = input.group.open(&input.mechanisms.registration_authority_ref).unwrap();
    let output = super::materialize(input).unwrap();
    let mut config = output.configurations["node-1"].clone();
    let transport = config.transport.profile.clone().into_profile().unwrap();
    let protocol = config.transport.protocol.clone().into_descriptor(&transport).unwrap();
    // 700 references alone fit: this denial must account for the real completed-request records,
    // maximum-width positions, snapshot and outer envelope, not just reference bytes.
    for retention in [700, 1024] {
        config.replica.request_retention_entries = retention;
        let mut definition = serde_json::to_value(&config.replica).unwrap();
        definition.as_object_mut().unwrap().remove("profile_ref");
        config.replica.profile_ref = super::identity(&serde_json::json!({
            "schema": "molten.experimental-replica-definition.v1", "profile": definition
        }))
        .unwrap();
        let encoded = maximum_width_snapshot(retention);
        assert!(u64::try_from(encoded.bytes.len()).unwrap() > protocol.framing.max_frame_bytes);
        let error = super::validation::configuration(&config, &group).unwrap_err();
        assert!(error.to_string().contains("snapshot retention window exceeds the admitted protocol frame bound"));
    }
}
