//! Admit the complete deduplication window before opening service storage or transport.

pub(super) fn admit_configuration(config: &super::decoder::Configuration) -> crate::error::Result<()> {
    let transport = config.transport.profile.clone().into_profile()?;
    let protocol = config.transport.protocol.clone().into_descriptor(&transport)?;
    admit(&config.replica.decode(), protocol.framing.max_frame_bytes)
}

pub(super) fn admit(
    profile: &crate::fabric_consistency::raft::ReplicaProfile,
    max_frame_bytes: u64,
) -> crate::error::Result<()> {
    crate::fabric_consistency::raft::validate_retention_bounds(profile)?;
    // Each canonical request reference alone occupies 71 bytes. Reject impossible windows before
    // allocating their projection; retention validation also caps the window at 4096 entries.
    if profile.request_retention_entries.saturating_mul(71) > max_frame_bytes {
        return Err(incompatible());
    }
    let message = largest_message(profile.request_retention_entries)?;
    if u64::try_from(message.bytes.len()).map_or(true, |bytes| bytes > max_frame_bytes) {
        return Err(incompatible());
    }
    Ok(())
}

fn incompatible() -> crate::error::Failure {
    crate::error::Failure::invalid_harness(
        "complete registry snapshot retention window exceeds the admitted protocol frame bound",
    )
}

// All references have the canonical fixed width. Use maximum-width integers, including applied
// positions late in a long-running service, and the wire codec's maximum 256-byte node identifiers.
// The actual InstallSnapshot encoder includes both the snapshot and its outer Raft envelope. The
// transport's eight-byte length prefix is separate from its admitted payload-byte bound.
fn largest_message(retention: u64) -> crate::error::Result<crate::fabric_consistency::raft::ReplicaMessage> {
    let reference = format!("blake3:{:064x}", 0);
    let mut snapshot = crate::fabric_consistency::raft::ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: reference.clone(),
        membership_ref: reference.clone(),
        config_epoch: u64::MAX,
        fencing_epoch: u64::MAX,
        last_included_index: u64::MAX,
        last_included_term: u64::MAX,
        application_state_ref: reference.clone(),
        completed_requests: (0..retention).map(|offset| (format!("blake3:{offset:064x}"), u64::MAX - offset)).collect(),
    };
    snapshot.snapshot_ref = crate::fabric_consistency::raft::snapshot_ref(&snapshot)?;
    let leader = "a".repeat(256);
    crate::fabric_consistency::raft::replica_message(&crate::fabric_consistency::raft::ReplicaMessageEnvelope {
        group_binding_ref: reference,
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
}
