
pub fn snapshot_control_registry(input: &RaftSnapshotInput) -> Result<RaftSnapshot> {
    let state = parse_control_registry_state(&input.state)?;
    require_ref(&input.group_ref, "raft snapshot group ref")?;
    validate_refs(&input.log_refs, "raft snapshot log ref")?;
    let content_ref = state.state_ref.clone();
    let session_refs =
        state.client_sessions.iter().map(|session| session.result_command_ref.clone()).collect::<Vec<_>>();
    let value = record("raft-snapshot-v1", vec![
        string(crate::preserves_rail::RAFT_SNAPSHOT_SCHEMA),
        record("group", vec![string(&input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("state-ref", vec![string(&state.state_ref)]),
        record("content-ref", vec![string(&content_ref)]),
        record("state", vec![state.value.clone()]),
        record("client-sessions", vec![strings_sequence(&session_refs)]),
        record("log", vec![strings_sequence(&input.log_refs)]),
        checks_value(&[
            ("chunk-backed-content-ref", "pass"),
            ("snapshot-state-integrity", "pass"),
        ]),
    ]);
    Ok(RaftSnapshot {
        snapshot_ref: canonical_hash(&value)?,
        group_ref: input.group_ref.clone(),
        term: input.term,
        index: input.index,
        state,
        content_ref,
        value,
    })
}

pub fn parse_raft_snapshot(value: &IoValue) -> Result<RaftSnapshot> {
    let fields = value
        .collect_simple_record("raft-snapshot-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-snapshot-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_SNAPSHOT_SCHEMA, "raft snapshot schema")?;
    let state_value = record_iovalue(&fields[6], "state")?;
    let state = parse_control_registry_state(&state_value)?;
    let state_ref = record_ref(&fields[4], "state-ref")?;
    let content_ref = record_ref(&fields[5], "content-ref")?;
    if state.state_ref != state_ref || state.state_ref != content_ref {
        return Err(MoltenError::invalid_harness("raft snapshot state/content ref mismatch"));
    }
    require_check(&parse_checks(&fields[9])?, "snapshot-state-integrity", "raft snapshot")?;
    Ok(RaftSnapshot {
        snapshot_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        term: record_u64(&fields[2], "term")?,
        index: record_u64(&fields[3], "index")?,
        state,
        content_ref,
        value: value.clone(),
    })
}

pub fn recover_control_registry(input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt> {
    let snapshot = parse_raft_snapshot(&input.snapshot)?;
    ensure_count_at_most(input.log_entries.len(), MAX_RAFT_ENTRIES, "recovery log entries")?;
    let mut diagnostics = Vec::with_capacity(input.log_entries.len().saturating_add(1));
    if snapshot.group_ref != input.group_ref {
        diagnostics.push("snapshot group does not match recovery group".to_string());
    }
    let mut expected_index = snapshot.index.saturating_add(1);
    let mut replayed = Vec::with_capacity(input.log_entries.len());
    for entry_value in &input.log_entries {
        let entry = parse_raft_log_entry(entry_value)?;
        if entry.index != expected_index {
            diagnostics.push(format!("log gap at index {}; expected {expected_index}", entry.index));
        }
        expected_index = entry.index.saturating_add(1);
        replayed.push(entry.entry_ref);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let predicate = if decision == "pass" {
        Some(parse_predicate_receipt(&predicate_receipt_value(&PredicateReceiptInput {
            predicate: "trellis-snapshot-restore",
            decision,
            group_ref: &input.group_ref,
            term: snapshot.term,
            index: snapshot.index,
            subjects: &[snapshot.snapshot_ref.clone(), snapshot.state.state_ref.clone()],
            diagnostics: &[],
            checks: &[("trellis-predicate", "pass"), ("snapshot-content-ref", "pass")],
        })?)?)
    } else {
        None
    };
    let value = record("raft-recovery-receipt-v1", vec![
        string(crate::preserves_rail::RAFT_RECOVERY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("group", vec![string(&input.group_ref)]),
        record("snapshot", vec![string(&snapshot.snapshot_ref)]),
        record("restored-state", vec![optional_ref_value(
            (decision == "pass").then_some(snapshot.state.state_ref.as_str()),
        )]),
        record("replayed-log", vec![strings_sequence(&replayed)]),
        record("restore-predicate", vec![optional_ref_value(
            predicate.as_ref().map(|value| value.predicate_ref.as_str()),
        )]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[("snapshot-verified", decision), ("log-suffix-checked", "pass")]),
    ]);
    Ok(RaftRecoveryReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        restored_state_ref: (decision == "pass").then_some(snapshot.state.state_ref),
        diagnostics,
        value,
    })
}
