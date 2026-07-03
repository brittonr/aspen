
fn apply_admitted_command(
    runtime: &mut ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: &ControlRegistryCommand,
    log_entry: &RaftLogEntry,
) -> Result<ControlRegistryReceipt> {
    let before_ref = runtime.state.state_ref.clone();
    let mut maps = state_maps(&runtime.state)?;
    match command.operation.as_str() {
        "remove" => {
            maps.entries.remove(&ControlRegistryKey {
                namespace: command.namespace.clone(),
                name: command.name.clone(),
            });
        }
        _ => {
            let target_ref = command
                .target_ref
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("admitted set command missing target ref"))?;
            maps.entries.insert(
                ControlRegistryKey {
                    namespace: command.namespace.clone(),
                    name: command.name.clone(),
                },
                target_ref,
            );
        }
    }
    maps.sessions.insert(
        ClientSequenceKey {
            client_session: envelope.client_session.clone(),
            sequence: envelope.sequence,
        },
        ClientSessionRecord {
            client_session: envelope.client_session.clone(),
            sequence: envelope.sequence,
            result_command_ref: envelope.envelope_ref.clone(),
        },
    );
    runtime.state = parse_control_registry_state(&control_registry_state_value(
        entries_from_map(&maps.entries),
        sessions_from_map(&maps.sessions),
    )?)?;
    let receipt_value = registry_receipt_value(&RegistryReceiptValueInput {
        decision: "pass",
        operation: &command.operation,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: Some(&log_entry.entry_ref),
        state_before_ref: &before_ref,
        state_after_ref: Some(&runtime.state.state_ref),
        client_session: &envelope.client_session,
        sequence: envelope.sequence,
        duplicate: false,
        authority_refs: &envelope.authority_refs,
        policy_refs: &envelope.policy_refs,
        resource_refs: &envelope.resource_refs,
        diagnostics: &[],
    })?;
    parse_registry_receipt(&receipt_value)
}

fn deny_commit_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    reason: &str,
    diagnostics: &[String],
) -> Result<RaftCommitReceipt> {
    let mut all_diagnostics = Vec::with_capacity(diagnostics.len() + 1);
    all_diagnostics.push(reason.to_string());
    all_diagnostics.extend(diagnostics.iter().cloned());
    let value = commit_receipt_value(&CommitReceiptValueInput {
        decision: "deny",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: runtime.committed_index,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: None,
        quorum_refs: &[],
        append_predicate_ref: None,
        commit_predicate_ref: None,
        diagnostics: &all_diagnostics,
    })?;
    parse_commit_receipt(&value)
}

fn deny_registry_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: Option<&ControlRegistryCommand>,
    diagnostics: &[String],
) -> Result<ControlRegistryReceipt> {
    deny_registry_receipt_with_duplicate(runtime, envelope, command, diagnostics, false)
}

fn deny_duplicate_registry_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: Option<&ControlRegistryCommand>,
    diagnostics: &[String],
) -> Result<ControlRegistryReceipt> {
    deny_registry_receipt_with_duplicate(runtime, envelope, command, diagnostics, true)
}

fn deny_registry_receipt_with_duplicate(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: Option<&ControlRegistryCommand>,
    diagnostics: &[String],
    duplicate: bool,
) -> Result<ControlRegistryReceipt> {
    let operation = command.map(|value| value.operation.as_str()).unwrap_or("deny");
    let value = registry_receipt_value(&RegistryReceiptValueInput {
        decision: "deny",
        operation,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: None,
        state_before_ref: &runtime.state.state_ref,
        state_after_ref: None,
        client_session: &envelope.client_session,
        sequence: envelope.sequence,
        duplicate,
        authority_refs: &envelope.authority_refs,
        policy_refs: &envelope.policy_refs,
        resource_refs: &envelope.resource_refs,
        diagnostics,
    })?;
    parse_registry_receipt(&value)
}

fn raft_log_entry_value(input: &LogEntryValueInput<'_>) -> Result<IoValue> {
    require_ref(input.group_ref, "raft log group ref")?;
    require_ref(input.command_ref, "raft log command ref")?;
    require_ref(input.append_predicate_ref, "raft log append predicate ref")?;
    if let Some(reference) = input.prior_log_ref {
        require_ref(reference, "raft log prior ref")?;
    }
    Ok(record("raft-log-entry-v1", vec![
        string(crate::preserves_rail::RAFT_LOG_ENTRY_SCHEMA),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("prior-log", vec![optional_ref_value(input.prior_log_ref)]),
        record("command-ref", vec![string(input.command_ref)]),
        record("command", vec![input.command.clone()]),
        record("append-predicate", vec![string(input.append_predicate_ref)]),
        checks_value(&[("append-consistency", "pass"), ("command-ref-binding", "pass")]),
    ]))
}

fn parse_raft_log_entry(value: &IoValue) -> Result<RaftLogEntry> {
    let fields = value
        .collect_simple_record("raft-log-entry-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-log-entry-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_LOG_ENTRY_SCHEMA, "raft log entry schema")?;
    let command = record_iovalue(&fields[6], "command")?;
    require_check(&parse_checks(&fields[8])?, "append-consistency", "raft log entry")?;
    Ok(RaftLogEntry {
        entry_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        term: record_u64(&fields[2], "term")?,
        index: record_u64(&fields[3], "index")?,
        prior_log_ref: record_optional_ref(&fields[4], "prior-log")?,
        command_ref: record_ref(&fields[5], "command-ref")?,
        command,
        append_predicate_ref: record_ref(&fields[7], "append-predicate")?,
        value: value.clone(),
    })
}

fn commit_receipt_value(input: &CommitReceiptValueInput<'_>) -> Result<IoValue> {
    require_ref(input.group_ref, "raft commit group ref")?;
    require_ref(input.command_ref, "raft commit command ref")?;
    validate_refs(input.quorum_refs, "raft commit quorum ref")?;
    Ok(record("raft-commit-receipt-v1", vec![
        string(crate::preserves_rail::RAFT_COMMIT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("command", vec![string(input.command_ref)]),
        record("log-entry", vec![optional_ref_value(input.log_entry_ref)]),
        record("quorum", vec![strings_sequence(input.quorum_refs)]),
        record("append-predicate", vec![optional_ref_value(input.append_predicate_ref)]),
        record("commit-predicate", vec![optional_ref_value(input.commit_predicate_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[("quorum-commit", input.decision), ("decision-before-apply", "pass")]),
    ]))
}

fn parse_commit_receipt(value: &IoValue) -> Result<RaftCommitReceipt> {
    let fields = value
        .collect_simple_record("raft-commit-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-commit-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_COMMIT_RECEIPT_SCHEMA, "raft commit receipt schema")?;
    Ok(RaftCommitReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        group_ref: record_ref(&fields[2], "group")?,
        term: record_u64(&fields[3], "term")?,
        index: record_u64(&fields[4], "index")?,
        command_ref: record_ref(&fields[5], "command")?,
        log_entry_ref: record_optional_ref(&fields[6], "log-entry")?,
        value: value.clone(),
    })
}

fn registry_receipt_value(input: &RegistryReceiptValueInput<'_>) -> Result<IoValue> {
    require_ref(input.command_ref, "control registry command ref")?;
    require_ref(input.state_before_ref, "control registry state-before ref")?;
    if let Some(reference) = input.state_after_ref {
        require_ref(reference, "control registry state-after ref")?;
    }
    Ok(record("control-registry-receipt-v1", vec![
        string(crate::preserves_rail::CONTROL_REGISTRY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("command", vec![string(input.command_ref)]),
        record("log-entry", vec![optional_ref_value(input.log_entry_ref)]),
        record("state-before", vec![string(input.state_before_ref)]),
        record("state-after", vec![optional_ref_value(input.state_after_ref)]),
        record("client-session", vec![string(input.client_session)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("duplicate", vec![bool_value(input.duplicate)]),
        record("authority", vec![strings_sequence(input.authority_refs)]),
        record("policy", vec![strings_sequence(input.policy_refs)]),
        record("resource", vec![strings_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("deterministic-apply", input.decision),
            ("client-session-idempotency", "pass"),
        ]),
    ]))
}

fn parse_registry_receipt(value: &IoValue) -> Result<ControlRegistryReceipt> {
    let fields = value
        .collect_simple_record("control-registry-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::CONTROL_REGISTRY_RECEIPT_SCHEMA,
        "control registry receipt schema",
    )?;
    Ok(ControlRegistryReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        operation: record_string(&fields[2], "operation")?,
        command_ref: record_ref(&fields[3], "command")?,
        state_before_ref: record_ref(&fields[5], "state-before")?,
        state_after_ref: record_optional_ref(&fields[6], "state-after")?,
        duplicate: record_bool(&fields[9], "duplicate")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    })
}

fn predicate_receipt_value(input: &PredicateReceiptInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.predicate, "raft predicate")?;
    require_ref(input.group_ref, "raft predicate group ref")?;
    validate_refs(input.subjects, "raft predicate subject ref")?;
    Ok(record("raft-predicate-receipt-v1", vec![
        string(crate::preserves_rail::RAFT_PREDICATE_RECEIPT_SCHEMA),
        record("predicate", vec![string(input.predicate)]),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("subjects", vec![strings_sequence(input.subjects)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(input.checks),
    ]))
}

fn parse_predicate_receipt(value: &IoValue) -> Result<RaftPredicateReceipt> {
    let fields = value
        .collect_simple_record("raft-predicate-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-predicate-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_PREDICATE_RECEIPT_SCHEMA, "raft predicate receipt schema")?;
    require_check(&parse_checks(&fields[8])?, "trellis-predicate", "raft predicate receipt")?;
    Ok(RaftPredicateReceipt {
        predicate_ref: canonical_hash(value)?,
        predicate: record_string(&fields[1], "predicate")?,
        decision: record_string(&fields[2], "decision")?,
        value: value.clone(),
    })
}

fn read_receipt_value(input: &ReadReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(record("raft-read-receipt-v1", vec![
        string(crate::preserves_rail::RAFT_READ_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("state", vec![string(input.state_ref)]),
        record("committed-term", vec![u64_value(input.committed_term)]),
        record("committed-index", vec![u64_value(input.committed_index)]),
        record("namespace", vec![string(input.namespace)]),
        record("name", vec![string(input.name)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("read-index-predicate", vec![optional_ref_value(input.read_index_predicate_ref)]),
        record("authority", vec![strings_sequence(input.authority_refs)]),
        record("resource", vec![strings_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("read-index-bound", input.decision),
            ("authority-resource-gated", "pass"),
        ]),
    ]))
}
