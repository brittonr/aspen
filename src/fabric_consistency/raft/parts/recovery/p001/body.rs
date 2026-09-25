
fn install_recovery_snapshot(
    state: &mut ReplicaState,
    durable_commit_index: u64,
    snapshot: Option<ReplicaSnapshot>,
) -> crate::error::Result<()> {
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.group_binding_ref != state.profile.group_binding_ref
        || snapshot.membership_ref != state.membership.membership_ref
        || snapshot.config_epoch != state.membership.config_epoch
        || snapshot.fencing_epoch != state.profile.fencing_epoch
        || snapshot.last_included_index > durable_commit_index
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "live Raft recovered snapshot binding or boundary mismatch",
        ));
    }
    for index in snapshot.completed_requests.values() {
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(crate::error::MoltenError::invalid_harness(
                "recovered snapshot completed request index is out of range",
            ));
        }
    }
    state.completed_requests.clone_from(&snapshot.completed_requests);
    state.log.retain(|entry| entry.index > snapshot.last_included_index);
    state.snapshot = Some(snapshot);
    Ok(())
}

fn support_last_log_index(state: &ReplicaState) -> u64 {
    state.log.last().map_or_else(
        || state.snapshot.as_ref().map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index),
        |entry| entry.index,
    )
}

fn recovery_ref(
    state: &ReplicaState,
    durable_record_refs: &[String],
    snapshot_bytes: Option<&[u8]>,
) -> crate::error::Result<String> {
    let snapshot_content_ref = snapshot_bytes.map(crate::preserves_rail::content_ref_from_bytes);
    let snapshot_value = snapshot_content_ref.as_deref().map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |reference| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
    );
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-recovery-plan-v1", vec![
        crate::preserves_rail::string(&state.profile.group_binding_ref),
        crate::preserves_rail::string(&state.node_id),
        crate::preserves_rail::u64_value(state.profile.service_generation),
        crate::preserves_rail::u64_value(state.current_term),
        crate::preserves_rail::u64_value(state.commit_index),
        crate::preserves_rail::sequence(durable_record_refs.iter().map(crate::preserves_rail::string).collect()),
        snapshot_value,
    ]))
}
