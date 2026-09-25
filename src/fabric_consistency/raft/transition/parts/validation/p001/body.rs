
fn validate_state_entry<'a>(
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    request_refs: &mut std::collections::BTreeSet<&'a str>,
) -> crate::error::Result<()> {
    if entry.index != expected_index || entry.term == 0 {
        return Err(crate::error::MoltenError::invalid_harness("Raft log indices or terms are invalid"));
    }
    validate_content_ref(&entry.request_ref, "Raft state request ref")?;
    validate_content_ref(&entry.command_ref, "Raft state command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft state command schema ref")?;
    if !request_refs.insert(entry.request_ref.as_str()) {
        return Err(crate::error::MoltenError::invalid_harness("Raft log contains a duplicate request ref"));
    }
    Ok(())
}

fn validate_role(state: &ReplicaState) -> crate::error::Result<()> {
    if state.role == ReplicaRole::Leader && state.leader_id.as_deref() != Some(state.node_id.as_str()) {
        return Err(crate::error::MoltenError::invalid_harness("Raft leader state lacks self leader identity"));
    }
    if state.role != ReplicaRole::Leader && state.leader_id.as_deref() == Some(state.node_id.as_str()) {
        return Err(crate::error::MoltenError::invalid_harness("non-leader Raft state claims self leadership"));
    }
    if state.role != ReplicaRole::Follower && state.current_term == 0 {
        return Err(crate::error::MoltenError::invalid_harness("candidate or leader cannot occupy term zero"));
    }
    Ok(())
}
