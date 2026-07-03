
pub fn persist_control_registry_runtime(
    root: &Path,
    runtime: &ControlRegistryRuntime,
    snapshot: &RaftSnapshot,
) -> Result<()> {
    let db = ensure_store_tables(root)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut logs = write_txn.open_table(STORE_LOGS).map_err(store_error)?;
        for entry in &runtime.log_entries {
            let bytes = canonical_bytes(&entry.value)?;
            logs.insert(entry.entry_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    {
        let mut snapshots = write_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?;
        let bytes = canonical_bytes(&snapshot.value)?;
        snapshots.insert(snapshot.snapshot_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut sessions = write_txn.open_table(STORE_SESSIONS).map_err(store_error)?;
        for session in &runtime.state.client_sessions {
            let value = session_record_value(session);
            let bytes = canonical_bytes(&value)?;
            sessions.insert(session.result_command_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        for receipt in &runtime.registry_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.receipt_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
        for receipt in &runtime.commit_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.receipt_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
        for receipt in &runtime.predicate_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.predicate_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    write_txn.commit().map_err(store_error)
}

pub fn control_registry_store_status(root: &Path) -> Result<ControlRegistryStoreStatus> {
    let db = ensure_store_tables(root)?;
    let read_txn = db.begin_read().map_err(store_error)?;
    let log_count = read_txn.open_table(STORE_LOGS).map_err(store_error)?.len().map_err(store_error)?;
    let snapshot_count = read_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?.len().map_err(store_error)?;
    let session_count = read_txn.open_table(STORE_SESSIONS).map_err(store_error)?.len().map_err(store_error)?;
    let receipt_count = read_txn.open_table(STORE_RECEIPTS).map_err(store_error)?.len().map_err(store_error)?;
    Ok(ControlRegistryStoreStatus {
        log_count,
        snapshot_count,
        session_count,
        receipt_count,
    })
}

pub fn control_registry_fixture_manifest_value() -> Result<IoValue> {
    raft_group_manifest_value(&RaftGroupManifestInput {
        group_id: DEFAULT_GROUP_ID.to_string(),
        members: vec![
            synthetic_ref("node-a")?,
            synthetic_ref("node-b")?,
            synthetic_ref("node-c")?,
        ],
        state_machine: CONTROL_REGISTRY_STATE_MACHINE.to_string(),
        command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
        read_mode: READ_MODE_READ_INDEX.to_string(),
        snapshot_policy_ref: synthetic_ref("snapshot-policy")?,
        policy_refs: vec![synthetic_ref("raft-policy")?],
        resource_refs: vec![synthetic_ref("raft-resource")?],
    })
}

pub fn run_control_registry_fixture() -> Result<ControlRegistryRuntime> {
    let manifest_value = control_registry_fixture_manifest_value()?;
    let mut runtime = new_control_registry_runtime(&manifest_value)?;
    let commands = vec![
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "install-protocol".to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            target_ref: Some(synthetic_ref("protocol-install")?),
        })?,
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-policy-version".to_string(),
            namespace: "policy".to_string(),
            name: "service-runtime".to_string(),
            target_ref: Some(synthetic_ref("policy-v1")?),
        })?,
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-receipt-index".to_string(),
            namespace: "receipt-index".to_string(),
            name: "harness-gate".to_string(),
            target_ref: Some(synthetic_ref("checkpoint")?),
        })?,
    ];
    for (index, command) in commands.into_iter().enumerate() {
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:fixture".to_string(),
            sequence: u64::try_from(index + 1)
                .map_err(|error| MoltenError::invalid_harness(format!("fixture sequence overflow: {error}")))?,
            command,
            authority_refs: vec![synthetic_ref("authority")?],
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![synthetic_ref("proposal-evidence")?],
        })?;
        propose_control_registry_command(&mut runtime, &envelope)?;
    }
    Ok(runtime)
}

pub fn control_registry_summary(runtime: &ControlRegistryRuntime) -> String {
    format!(
        "raft-control-registry group={} committed={} entries={} state={}",
        runtime.manifest.group_id,
        runtime.committed_index,
        runtime.state.entries.len(),
        runtime.state.state_ref
    )
}

fn pass_draft(runtime: &ControlRegistryRuntime, envelope: &RaftCommandEnvelope) -> Result<PassDraft> {
    let next_index = runtime
        .committed_index
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("raft committed index overflow"))?;
    let append_predicate = pass_predicate(
        "trellis-append-consistency",
        runtime,
        next_index,
        std::slice::from_ref(&envelope.envelope_ref),
        &[("trellis-predicate", "pass"), ("prior-log-binding", "pass")],
    )?;
    let log_entry_value = raft_log_entry_value(&LogEntryValueInput {
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        prior_log_ref: runtime.last_log_ref.as_deref(),
        command_ref: &envelope.envelope_ref,
        command: &envelope.value,
        append_predicate_ref: &append_predicate.predicate_ref,
    })?;
    let log_entry = parse_raft_log_entry(&log_entry_value)?;
    let commit_predicate =
        pass_predicate("trellis-quorum-commit", runtime, next_index, std::slice::from_ref(&log_entry.entry_ref), &[
            ("trellis-predicate", "pass"),
            ("quorum-members", "pass"),
        ])?;
    let advancement_subjects = [runtime.state.state_ref.clone(), log_entry.entry_ref.clone()];
    let advancement_predicate =
        pass_predicate("trellis-commit-advancement", runtime, next_index, &advancement_subjects, &[
            ("trellis-predicate", "pass"),
            ("monotonic-index", "pass"),
        ])?;
    let commit_receipt = pass_commit(PassCommitInput {
        runtime,
        envelope,
        index: next_index,
        log_entry: &log_entry,
        append_predicate: &append_predicate,
        commit_predicate: &commit_predicate,
    })?;
    Ok(PassDraft {
        next_index,
        append_predicate,
        commit_predicate,
        advancement_predicate,
        log_entry,
        commit_receipt,
    })
}

fn pass_predicate(
    predicate: &str,
    runtime: &ControlRegistryRuntime,
    index: u64,
    subjects: &[String],
    checks: &[(&str, &str)],
) -> Result<RaftPredicateReceipt> {
    let value = predicate_receipt_value(&PredicateReceiptInput {
        predicate,
        decision: "pass",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index,
        subjects,
        diagnostics: &[],
        checks,
    })?;
    parse_predicate_receipt(&value)
}

fn pass_commit(input: PassCommitInput<'_>) -> Result<RaftCommitReceipt> {
    let value = commit_receipt_value(&CommitReceiptValueInput {
        decision: "pass",
        group_ref: &input.runtime.manifest.manifest_ref,
        term: input.runtime.term,
        index: input.index,
        command_ref: &input.envelope.envelope_ref,
        log_entry_ref: Some(&input.log_entry.entry_ref),
        quorum_refs: &input.runtime.manifest.members,
        append_predicate_ref: Some(&input.append_predicate.predicate_ref),
        commit_predicate_ref: Some(&input.commit_predicate.predicate_ref),
        diagnostics: &[],
    })?;
    parse_commit_receipt(&value)
}

fn admitted_command(command: &IoValue) -> (Option<ControlRegistryCommand>, Vec<String>) {
    if let Ok(parsed) = parse_control_registry_command(command) {
        return (Some(parsed), Vec::new());
    }
    let text = crate::preserves_rail::to_text(command).unwrap_or_else(|_| "<unrenderable>".to_string());
    let forbidden = [
        "actor-message",
        "protocol-message",
        "remote-dataspace-envelope",
        "gossip",
        "docs",
        "blob-transfer",
        "chunk-manifest",
        "global-script",
        "comm",
        "choreography-step",
    ];
    if forbidden.iter().any(|marker| text.contains(marker)) {
        return (None, vec!["non-control-plane payload rejected from Raft".to_string()]);
    }
    (None, vec!["unknown Raft command schema".to_string()])
}

fn proposal_diagnostics(input: ProposalDecisionInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics;
    if input.envelope.group_ref != input.runtime.manifest.manifest_ref {
        diagnostics.push("command group does not match runtime manifest".to_string());
    }
    if input.envelope.authority_refs.is_empty() {
        diagnostics.push("missing proposal authority evidence".to_string());
    }
    if input.envelope.policy_refs.is_empty() {
        diagnostics.push("missing proposal policy evidence".to_string());
    }
    if input.envelope.resource_refs.is_empty() {
        diagnostics.push("missing proposal resource evidence".to_string());
    }
    if input.envelope.evidence_refs.is_empty() {
        diagnostics.push("missing proposal evidence".to_string());
    }
    if let Some(command) = input.command {
        let schema = operation_schema(&command.operation);
        if !input.runtime.manifest.command_schemas.iter().any(|value| value == schema) {
            diagnostics.push(format!("command schema {schema} is not admitted by manifest"));
        }
    }
    validate_refs(&input.envelope.authority_refs, "proposal authority ref")?;
    validate_refs(&input.envelope.policy_refs, "proposal policy ref")?;
    validate_refs(&input.envelope.resource_refs, "proposal resource ref")?;
    validate_refs(&input.envelope.evidence_refs, "proposal evidence ref")?;
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "raft proposal diagnostics")?;
    Ok(diagnostics)
}

fn duplicate_sequence(runtime: &ControlRegistryRuntime, envelope: &RaftCommandEnvelope) -> Option<DuplicateSequence> {
    if let Some(session) = runtime
        .state
        .client_sessions
        .iter()
        .find(|session| session.client_session == envelope.client_session && session.sequence == envelope.sequence)
    {
        if session.result_command_ref != envelope.envelope_ref {
            return Some(DuplicateSequence::Conflict(session.clone()));
        }
        return runtime
            .registry_receipts
            .iter()
            .find(|receipt| receipt.command_ref == envelope.envelope_ref)
            .cloned()
            .map(DuplicateSequence::Replay)
            .or_else(|| Some(DuplicateSequence::Conflict(session.clone())));
    }
    runtime
        .state
        .client_sessions
        .iter()
        .filter(|session| session.client_session == envelope.client_session && envelope.sequence <= session.sequence)
        .max_by_key(|session| session.sequence)
        .cloned()
        .map(DuplicateSequence::Conflict)
}
