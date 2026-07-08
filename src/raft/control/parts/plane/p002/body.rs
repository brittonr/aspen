
pub fn parse_control_registry_state(value: &IoValue) -> Result<ControlRegistryState> {
    let fields = value
        .collect_simple_record("control-registry-state-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-state-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::CONTROL_REGISTRY_STATE_SCHEMA, "control registry state schema")?;
    let entries = parse_registry_entries(&fields[1])?;
    let client_sessions = parse_client_sessions(&fields[2])?;
    require_check(&parse_checks(&fields[3])?, "deterministic-map-order", "control registry state")?;
    Ok(ControlRegistryState {
        state_ref: canonical_hash(value)?,
        entries,
        client_sessions,
        value: value.clone(),
    })
}

// r[impl molten.consensus.leaderless_profile_boundary]
pub fn new_control_registry_runtime(manifest_value: &IoValue) -> Result<ControlRegistryRuntime> {
    let manifest = parse_raft_group_manifest(manifest_value)?;
    if manifest.state_machine != CONTROL_REGISTRY_STATE_MACHINE {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported raft state machine {}; expected {CONTROL_REGISTRY_STATE_MACHINE}",
            manifest.state_machine
        )));
    }
    if manifest.algorithm_profile != CONSENSUS_PROFILE_RAFT {
        return Err(MoltenError::invalid_harness(format!(
            "consensus profile {} is not admitted for production runtime; status {}",
            manifest.algorithm_profile, manifest.production_status
        )));
    }
    Ok(ControlRegistryRuntime {
        manifest,
        term: 1,
        committed_index: 0,
        last_log_ref: None,
        state: initial_control_registry_state()?,
        log_entries: Vec::new(),
        commit_receipts: Vec::new(),
        registry_receipts: Vec::new(),
        predicate_receipts: Vec::new(),
    })
}

pub fn propose_control_registry_command(
    runtime: &mut ControlRegistryRuntime,
    envelope_value: &IoValue,
) -> Result<ControlRegistryProposal> {
    let envelope = parse_raft_command_envelope(envelope_value)?;
    let (command, diagnostics) = admitted_command(&envelope.command);
    if let Some(duplicate) = duplicate_sequence(runtime, &envelope) {
        match duplicate {
            DuplicateSequence::Replay(existing) => {
                let commit_receipt = deny_commit_receipt(runtime, &envelope, "duplicate-client-sequence", &[])?;
                return Ok(ControlRegistryProposal {
                    decision: existing.decision.clone(),
                    duplicate: true,
                    envelope,
                    predicates: Vec::new(),
                    log_entry: None,
                    commit_receipt,
                    registry_receipt: existing,
                });
            }
            DuplicateSequence::Conflict(session) => {
                let diagnostics = vec![format!(
                    "conflicting duplicate client sequence {} for {}; prior command {}",
                    envelope.sequence, envelope.client_session, session.result_command_ref
                )];
                let commit_receipt = deny_commit_receipt(runtime, &envelope, "duplicate-client-sequence", &diagnostics)?;
                let registry_receipt = deny_duplicate_registry_receipt(runtime, &envelope, command.as_ref(), &diagnostics)?;
                return Ok(ControlRegistryProposal {
                    decision: "deny".to_string(),
                    duplicate: true,
                    envelope,
                    predicates: Vec::new(),
                    log_entry: None,
                    commit_receipt,
                    registry_receipt,
                });
            }
        }
    }
    let admission = proposal_diagnostics(ProposalDecisionInput {
        runtime,
        envelope: &envelope,
        command: command.as_ref(),
        diagnostics,
    })?;
    if !admission.is_empty() {
        let commit_receipt = deny_commit_receipt(runtime, &envelope, "proposal-deny", &admission)?;
        let registry_receipt = deny_registry_receipt(runtime, &envelope, command.as_ref(), &admission)?;
        return Ok(ControlRegistryProposal {
            decision: "deny".to_string(),
            duplicate: false,
            envelope,
            predicates: Vec::new(),
            log_entry: None,
            commit_receipt,
            registry_receipt,
        });
    }
    let command =
        command.ok_or_else(|| MoltenError::invalid_harness("missing admitted command after admission pass"))?;
    let draft = pass_draft(runtime, &envelope)?;
    let registry_receipt = apply_admitted_command(runtime, &envelope, &command, &draft.log_entry)?;
    runtime.committed_index = draft.next_index;
    runtime.last_log_ref = Some(draft.log_entry.entry_ref.clone());
    runtime.log_entries.push(draft.log_entry.clone());
    runtime.commit_receipts.push(draft.commit_receipt.clone());
    runtime.predicate_receipts.push(draft.append_predicate.clone());
    runtime.predicate_receipts.push(draft.commit_predicate.clone());
    runtime.predicate_receipts.push(draft.advancement_predicate.clone());
    runtime.registry_receipts.push(registry_receipt.clone());
    Ok(ControlRegistryProposal {
        decision: "pass".to_string(),
        duplicate: false,
        envelope,
        predicates: vec![
            draft.append_predicate,
            draft.commit_predicate,
            draft.advancement_predicate,
        ],
        log_entry: Some(draft.log_entry),
        commit_receipt: draft.commit_receipt,
        registry_receipt,
    })
}

// r[impl molten.consensus.read_consistency_modes]
pub fn read_control_registry(input: &ControlRegistryReadInput) -> Result<RaftReadReceipt> {
    validate_read_consistency_mode(&input.read_consistency_mode)?;
    let state = parse_control_registry_state(&input.state)?;
    let mut diagnostics = Vec::new();
    if input.authority_refs.is_empty() {
        diagnostics.push("missing read authority evidence".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("missing read resource evidence".to_string());
    }
    if input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE && input.read_index != input.committed_index {
        diagnostics
            .push(format!("stale read-index {}; expected committed index {}", input.read_index, input.committed_index));
    }
    validate_refs(&input.authority_refs, "raft read authority ref")?;
    validate_refs(&input.resource_refs, "raft read resource ref")?;
    let target = find_entry(&state, &input.namespace, &input.name).map(|entry| entry.target_ref.clone());
    if target.is_none() {
        diagnostics.push("control registry entry not found".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let predicate = if decision == "pass" && input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE {
        Some(parse_predicate_receipt(&predicate_receipt_value(&PredicateReceiptInput {
            predicate: "trellis-read-index-freshness",
            decision,
            group_ref: &input.group_ref,
            term: input.committed_term,
            index: input.committed_index,
            subjects: std::slice::from_ref(&state.state_ref),
            diagnostics: &[],
            checks: &[("trellis-predicate", "pass"), ("read-index-current", "pass")],
        })?)?)
    } else {
        None
    };
    let receipt = read_receipt_value(&ReadReceiptValueInput {
        decision,
        group_ref: &input.group_ref,
        state_ref: &state.state_ref,
        committed_term: input.committed_term,
        committed_index: input.committed_index,
        namespace: &input.namespace,
        name: &input.name,
        target_ref: target.as_deref(),
        read_consistency_mode: &input.read_consistency_mode,
        read_index_predicate_ref: predicate.as_ref().map(|value| value.predicate_ref.as_str()),
        authority_refs: &input.authority_refs,
        resource_refs: &input.resource_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(RaftReadReceipt {
        receipt_ref: canonical_hash(&receipt)?,
        decision: decision.to_string(),
        read_consistency_mode: input.read_consistency_mode.clone(),
        target_ref: target,
        diagnostics,
        value: receipt,
    })
}

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
