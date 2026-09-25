
pub fn parse_raft_group_manifest(value: &IoValue) -> Result<RaftGroupManifest> {
    let fields = value
        .collect_simple_record("raft-group-manifest-v1", Some(RAFT_GROUP_MANIFEST_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-group-manifest-v1 ...> with explicit profile"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_GROUP_MANIFEST_SCHEMA, "raft group manifest schema")?;
    let group_id = record_string(&fields[1], "group-id")?;
    validate_group_id(&group_id)?;
    let members = parse_string_sequence(&fields[2], "members")?;
    validate_refs(&members, "raft member ref")?;
    let state_machine = record_string(&fields[3], "state-machine")?;
    let command_schemas = parse_string_sequence(&fields[4], "command-schemas")?;
    validate_command_schema_list(&command_schemas)?;
    let read_mode = record_string(&fields[5], "read-mode")?;
    validate_read_mode(&read_mode)?;
    let snapshot_policy_ref = record_ref(&fields[6], "snapshot-policy")?;
    let policy_refs = parse_ref_sequence(&fields[7], "policy")?;
    let resource_refs = parse_ref_sequence(&fields[8], "resource")?;
    let algorithm_profile = record_string(&fields[9], "algorithm-profile")?;
    let admitted_profile_version = record_string(&fields[10], "profile-version")?;
    let read_consistency_support = parse_string_sequence(&fields[11], "read-consistency-support")?;
    let quorum_rule = record_string(&fields[12], "quorum-rule")?;
    let membership_policy_refs = parse_ref_sequence(&fields[13], "membership-policy")?;
    let placement_ref = record_optional_ref(&fields[14], "placement")?;
    let fault_model_caveats = parse_string_sequence(&fields[15], "fault-model-caveats")?;
    let required_evidence_refs = parse_ref_sequence(&fields[16], "required-evidence")?;
    let profile = ConsensusAlgorithmProfileInput {
        algorithm_profile,
        admitted_profile_version,
        read_consistency_support,
        quorum_rule,
        membership_policy_refs,
        placement_ref,
        fault_model_caveats,
        required_evidence_refs,
    };
    validate_consensus_algorithm_profile(&profile)?;
    require_check(&parse_checks(&fields[17])?, "algorithm-profile-declared", "raft group manifest")?;
    Ok(RaftGroupManifest {
        manifest_ref: canonical_hash(value)?,
        group_id,
        members,
        state_machine,
        command_schemas,
        read_mode,
        snapshot_policy_ref,
        policy_refs,
        resource_refs,
        production_status: consensus_production_status(&profile.algorithm_profile).to_string(),
        algorithm_profile: profile.algorithm_profile,
        admitted_profile_version: profile.admitted_profile_version,
        read_consistency_support: profile.read_consistency_support,
        quorum_rule: profile.quorum_rule,
        membership_policy_refs: profile.membership_policy_refs,
        placement_ref: profile.placement_ref,
        fault_model_caveats: profile.fault_model_caveats,
        required_evidence_refs: profile.required_evidence_refs,
        value: value.clone(),
    })
}

pub fn control_registry_command_value(input: &ControlRegistryCommandInput) -> Result<IoValue> {
    validate_control_command(input)?;
    Ok(record("control-registry-command-v1", vec![
        string(crate::preserves_rail::CONTROL_REGISTRY_COMMAND_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("namespace", vec![string(&input.namespace)]),
        record("name", vec![string(&input.name)]),
        record("target", vec![optional_ref_value(input.target_ref.as_deref())]),
        checks_value(&[("control-plane-only", "pass"), ("schema-admitted", "pass")]),
    ]))
}

pub fn parse_control_registry_command(value: &IoValue) -> Result<ControlRegistryCommand> {
    let fields = value
        .collect_simple_record("control-registry-command-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-command-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::CONTROL_REGISTRY_COMMAND_SCHEMA,
        "control registry command schema",
    )?;
    let input = ControlRegistryCommandInput {
        operation: record_string(&fields[1], "operation")?,
        namespace: record_string(&fields[2], "namespace")?,
        name: record_string(&fields[3], "name")?,
        target_ref: record_optional_ref(&fields[4], "target")?,
    };
    validate_control_command(&input)?;
    require_check(&parse_checks(&fields[5])?, "control-plane-only", "control registry command")?;
    Ok(ControlRegistryCommand {
        command_ref: canonical_hash(value)?,
        operation: input.operation,
        namespace: input.namespace,
        name: input.name,
        target_ref: input.target_ref,
        value: value.clone(),
    })
}

pub fn raft_command_envelope_value(input: &RaftCommandEnvelopeInput) -> Result<IoValue> {
    require_ref(&input.group_ref, "raft command group ref")?;
    validate_client_session(&input.client_session)?;
    validate_refs(&input.authority_refs, "raft command authority ref")?;
    validate_refs(&input.policy_refs, "raft command policy ref")?;
    validate_refs(&input.resource_refs, "raft command resource ref")?;
    validate_refs(&input.evidence_refs, "raft command evidence ref")?;
    ensure_count_at_most(input.evidence_refs.len(), MAX_RAFT_REFS, "raft command evidence")?;
    Ok(record("raft-command-envelope-v1", vec![
        string(crate::preserves_rail::RAFT_COMMAND_ENVELOPE_SCHEMA),
        record("group", vec![string(&input.group_ref)]),
        record("client-session", vec![string(&input.client_session)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("command", vec![input.command.clone()]),
        record("authority", vec![strings_sequence(&input.authority_refs)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        checks_value(&[("schema-admitted", "pass"), ("control-plane-only", "pass")]),
    ]))
}

pub fn parse_raft_command_envelope(value: &IoValue) -> Result<RaftCommandEnvelope> {
    let fields = value
        .collect_simple_record("raft-command-envelope-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-command-envelope-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_COMMAND_ENVELOPE_SCHEMA, "raft command envelope schema")?;
    let command = record_iovalue(&fields[4], "command")?;
    require_check(&parse_checks(&fields[9])?, "control-plane-only", "raft command envelope")?;
    Ok(RaftCommandEnvelope {
        envelope_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        client_session: record_string(&fields[2], "client-session")?,
        sequence: record_u64(&fields[3], "sequence")?,
        command,
        authority_refs: parse_ref_sequence(&fields[5], "authority")?,
        policy_refs: parse_ref_sequence(&fields[6], "policy")?,
        resource_refs: parse_ref_sequence(&fields[7], "resource")?,
        evidence_refs: parse_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn initial_control_registry_state() -> Result<ControlRegistryState> {
    control_registry_state_value(Vec::new(), Vec::new()).and_then(|value| parse_control_registry_state(&value))
}

pub fn control_registry_state_value(
    mut entries: Vec<ControlRegistryEntry>,
    mut client_sessions: Vec<ClientSessionRecord>,
) -> Result<IoValue> {
    ensure_count_at_most(entries.len(), MAX_RAFT_ENTRIES, "control registry entries")?;
    ensure_count_at_most(client_sessions.len(), MAX_RAFT_ENTRIES, "control registry client sessions")?;
    entries.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.target_ref.cmp(&right.target_ref))
    });
    client_sessions.sort_by(|left, right| {
        left.client_session
            .cmp(&right.client_session)
            .then_with(|| left.sequence.cmp(&right.sequence))
            .then_with(|| left.result_command_ref.cmp(&right.result_command_ref))
    });
    let entry_values = entries
        .iter()
        .map(|entry| record("entry", vec![string(&entry.namespace), string(&entry.name), string(&entry.target_ref)]))
        .collect();
    let session_values = client_sessions
        .iter()
        .map(|session| {
            record("session", vec![
                string(&session.client_session),
                u64_value(session.sequence),
                string(&session.result_command_ref),
            ])
        })
        .collect();
    Ok(record("control-registry-state-v1", vec![
        string(crate::preserves_rail::CONTROL_REGISTRY_STATE_SCHEMA),
        record("entries", vec![sequence(entry_values)]),
        record("client-sessions", vec![sequence(session_values)]),
        checks_value(&[
            ("deterministic-map-order", "pass"),
            ("control-plane-namespaces", "pass"),
        ]),
    ]))
}
