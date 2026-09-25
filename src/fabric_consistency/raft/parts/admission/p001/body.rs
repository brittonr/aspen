
fn initial_state(
    group: crate::fabric_consistency::ConsistencyGroupBinding,
    node_id: String,
    membership: super::StaticMembership,
    profile: super::ReplicaProfile,
) -> crate::error::Result<super::ReplicaState> {
    debug_assert_eq!(profile.group_binding_ref, group.binding_ref);
    let active_election_timer_ref = super::election_timer_ref(
        &profile.group_binding_ref,
        &node_id,
        profile.service_generation,
        super::INITIAL_TERM,
        super::INITIAL_ELECTION_TIMER_SEQUENCE,
    )?;
    Ok(super::ReplicaState {
        profile,
        node_id,
        membership,
        role: super::ReplicaRole::Follower,
        lifecycle: super::ReplicaLifecycle::Running,
        current_term: super::INITIAL_TERM,
        election_timer_sequence: super::INITIAL_ELECTION_TIMER_SEQUENCE,
        active_election_timer_ref,
        voted_for: None,
        leader_id: None,
        log: Vec::new(),
        commit_index: super::INITIAL_COMMIT_INDEX,
        last_applied: super::INITIAL_COMMIT_INDEX,
        snapshot: None,
        completed_requests: std::collections::BTreeMap::new(),
        pending_reads: std::collections::BTreeMap::new(),
        votes_received: std::collections::BTreeSet::new(),
        next_index: std::collections::BTreeMap::new(),
        match_index: std::collections::BTreeMap::new(),
        quorum_confirmed_term: None,
    })
}

fn validate_identifier(value: &str, label: &str) -> crate::error::Result<()> {
    if value.is_empty() || value.len() > MAX_REPLICA_IDENTIFIER_BYTES {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} must be non-empty and at most {MAX_REPLICA_IDENTIFIER_BYTES} bytes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} contains unsupported characters")));
    }
    Ok(())
}

fn validate_content_ref(value: &str, label: &str) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("invalid {label}: {error}")))
}
