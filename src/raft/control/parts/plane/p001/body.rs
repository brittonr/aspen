
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryStoreStatus {
    pub log_count: u64,
    pub snapshot_count: u64,
    pub session_count: u64,
    pub receipt_count: u64,
}

struct RegistryMaps {
    entries: OrderedMap<ControlRegistryKey, String>,
    sessions: OrderedMap<ClientSequenceKey, ClientSessionRecord>,
}

struct ProposalDecisionInput<'a> {
    runtime: &'a ControlRegistryRuntime,
    envelope: &'a RaftCommandEnvelope,
    command: Option<&'a ControlRegistryCommand>,
    diagnostics: Vec<String>,
}

enum DuplicateSequence {
    Replay(ControlRegistryReceipt),
    Conflict(ClientSessionRecord),
}

struct PassDraft {
    next_index: u64,
    append_predicate: RaftPredicateReceipt,
    commit_predicate: RaftPredicateReceipt,
    advancement_predicate: RaftPredicateReceipt,
    log_entry: RaftLogEntry,
    commit_receipt: RaftCommitReceipt,
}

struct PassCommitInput<'a> {
    runtime: &'a ControlRegistryRuntime,
    envelope: &'a RaftCommandEnvelope,
    index: u64,
    log_entry: &'a RaftLogEntry,
    append_predicate: &'a RaftPredicateReceipt,
    commit_predicate: &'a RaftPredicateReceipt,
}

struct PredicateReceiptInput<'a> {
    predicate: &'a str,
    decision: &'a str,
    group_ref: &'a str,
    term: u64,
    index: u64,
    subjects: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct LogEntryValueInput<'a> {
    group_ref: &'a str,
    term: u64,
    index: u64,
    prior_log_ref: Option<&'a str>,
    command_ref: &'a str,
    command: &'a IoValue,
    append_predicate_ref: &'a str,
}

struct CommitReceiptValueInput<'a> {
    decision: &'a str,
    group_ref: &'a str,
    term: u64,
    index: u64,
    command_ref: &'a str,
    log_entry_ref: Option<&'a str>,
    quorum_refs: &'a [String],
    append_predicate_ref: Option<&'a str>,
    commit_predicate_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

struct RegistryReceiptValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    command_ref: &'a str,
    log_entry_ref: Option<&'a str>,
    state_before_ref: &'a str,
    state_after_ref: Option<&'a str>,
    client_session: &'a str,
    sequence: u64,
    duplicate: bool,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    diagnostics: &'a [String],
}

struct ReadReceiptValueInput<'a> {
    decision: &'a str,
    group_ref: &'a str,
    state_ref: &'a str,
    committed_term: u64,
    committed_index: u64,
    namespace: &'a str,
    name: &'a str,
    target_ref: Option<&'a str>,
    read_consistency_mode: &'a str,
    read_index_predicate_ref: Option<&'a str>,
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    diagnostics: &'a [String],
}

// r[impl molten.consensus.algorithm_profile_manifest]
pub fn default_raft_algorithm_profile_input(input: &RaftGroupManifestInput) -> Result<ConsensusAlgorithmProfileInput> {
    Ok(ConsensusAlgorithmProfileInput {
        algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
        admitted_profile_version: CONSENSUS_PROFILE_VERSION_RAFT.to_string(),
        read_consistency_support: vec![
            READ_CONSISTENCY_LINEARIZABLE.to_string(),
            READ_CONSISTENCY_LOCAL_STALE.to_string(),
        ],
        quorum_rule: QUORUM_RULE_MAJORITY_READ_INDEX.to_string(),
        membership_policy_refs: input.policy_refs.clone(),
        placement_ref: Some(synthetic_ref("raft-placement")?),
        fault_model_caveats: default_consensus_caveats(),
        required_evidence_refs: vec![input.snapshot_policy_ref.clone()],
    })
}

// r[impl molten.consensus.leaderless_profile_boundary]
pub fn leaderless_experimental_algorithm_profile_input(
    membership_policy_refs: Vec<String>,
    placement_ref: Option<String>,
    required_evidence_refs: Vec<String>,
) -> ConsensusAlgorithmProfileInput {
    ConsensusAlgorithmProfileInput {
        algorithm_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
        admitted_profile_version: CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL.to_string(),
        read_consistency_support: vec![
            READ_CONSISTENCY_LINEARIZABLE.to_string(),
            READ_CONSISTENCY_LOCAL_STALE.to_string(),
        ],
        quorum_rule: QUORUM_RULE_LEADERLESS_MAJORITY.to_string(),
        membership_policy_refs,
        placement_ref,
        fault_model_caveats: default_consensus_caveats(),
        required_evidence_refs,
    }
}

pub fn raft_group_manifest_value(input: &RaftGroupManifestInput) -> Result<IoValue> {
    let profile = default_raft_algorithm_profile_input(input)?;
    raft_group_manifest_value_with_profile(input, &profile)
}

// r[impl molten.consensus.algorithm_profile_manifest]
pub fn raft_group_manifest_value_with_profile(
    input: &RaftGroupManifestInput,
    profile: &ConsensusAlgorithmProfileInput,
) -> Result<IoValue> {
    validate_group_id(&input.group_id)?;
    validate_refs(&input.members, "raft member ref")?;
    validate_non_empty(&input.state_machine, "raft state machine")?;
    validate_command_schema_list(&input.command_schemas)?;
    validate_read_mode(&input.read_mode)?;
    require_ref(&input.snapshot_policy_ref, "raft snapshot policy ref")?;
    validate_refs(&input.policy_refs, "raft policy ref")?;
    validate_refs(&input.resource_refs, "raft resource ref")?;
    ensure_count_at_most(input.members.len(), MAX_RAFT_MEMBERS, "raft members")?;
    validate_consensus_algorithm_profile(profile)?;
    Ok(record("raft-group-manifest-v1", vec![
        string(crate::preserves_rail::RAFT_GROUP_MANIFEST_SCHEMA),
        record("group-id", vec![string(&input.group_id)]),
        record("members", vec![strings_sequence(&input.members)]),
        record("state-machine", vec![string(&input.state_machine)]),
        record("command-schemas", vec![strings_sequence(&input.command_schemas)]),
        record("read-mode", vec![string(&input.read_mode)]),
        record("snapshot-policy", vec![string(&input.snapshot_policy_ref)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        record("algorithm-profile", vec![string(&profile.algorithm_profile)]),
        record("profile-version", vec![string(&profile.admitted_profile_version)]),
        record("read-consistency-support", vec![strings_sequence(&profile.read_consistency_support)]),
        record("quorum-rule", vec![string(&profile.quorum_rule)]),
        record("membership-policy", vec![strings_sequence(&profile.membership_policy_refs)]),
        record("placement", vec![optional_ref_value(profile.placement_ref.as_deref())]),
        record("fault-model-caveats", vec![strings_sequence(&profile.fault_model_caveats)]),
        record("required-evidence", vec![strings_sequence(&profile.required_evidence_refs)]),
        checks_value(&manifest_checks(profile)),
    ]))
}

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
