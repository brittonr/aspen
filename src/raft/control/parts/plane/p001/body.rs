
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

// r[impl molten.consensus.cluster_config_selection]
pub fn validate_cluster_consensus_config(config: &ClusterConsensusConfig) -> Result<()> {
    validate_algorithm_name(&config.algorithm_profile)?;
    if let Some(profile_version) = &config.profile_version {
        validate_non_empty(profile_version, "cluster consensus profile version")?;
    }
    if let Some(placement_ref) = &config.placement_ref {
        require_ref(placement_ref, "cluster consensus placement ref")?;
    }
    validate_refs(&config.required_evidence_refs, "cluster consensus required evidence ref")
}

// r[impl molten.consensus.cluster_config_selection]
pub fn consensus_algorithm_profile_from_cluster_config(
    input: &RaftGroupManifestInput,
    config: &ClusterConsensusConfig,
) -> Result<ConsensusAlgorithmProfileInput> {
    validate_cluster_consensus_config(config)?;
    let profile = match config.algorithm_profile.as_str() {
        CONSENSUS_PROFILE_RAFT => raft_algorithm_profile_from_cluster_config(input, config)?,
        CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => leaderless_algorithm_profile_from_cluster_config(input, config),
        value => return Err(MoltenError::invalid_harness(format!("unsupported consensus algorithm profile {value}"))),
    };
    validate_consensus_algorithm_profile(&profile)?;
    Ok(profile)
}

fn raft_algorithm_profile_from_cluster_config(
    input: &RaftGroupManifestInput,
    config: &ClusterConsensusConfig,
) -> Result<ConsensusAlgorithmProfileInput> {
    let mut profile = default_raft_algorithm_profile_input(input)?;
    if let Some(profile_version) = &config.profile_version {
        profile.admitted_profile_version = profile_version.clone();
    }
    if let Some(placement_ref) = &config.placement_ref {
        profile.placement_ref = Some(placement_ref.clone());
    }
    if !config.required_evidence_refs.is_empty() {
        profile.required_evidence_refs = config.required_evidence_refs.clone();
    }
    Ok(profile)
}

fn leaderless_algorithm_profile_from_cluster_config(
    input: &RaftGroupManifestInput,
    config: &ClusterConsensusConfig,
) -> ConsensusAlgorithmProfileInput {
    let mut profile = leaderless_experimental_algorithm_profile_input(
        input.policy_refs.clone(),
        config.placement_ref.clone(),
        config.required_evidence_refs.clone(),
    );
    if let Some(profile_version) = &config.profile_version {
        profile.admitted_profile_version = profile_version.clone();
    }
    profile
}

pub fn raft_group_manifest_value(input: &RaftGroupManifestInput) -> Result<IoValue> {
    let profile = default_raft_algorithm_profile_input(input)?;
    raft_group_manifest_value_with_profile(input, &profile)
}

// r[impl molten.consensus.cluster_config_selection]
pub fn raft_group_manifest_value_with_cluster_config(
    input: &RaftGroupManifestInput,
    config: &ClusterConsensusConfig,
) -> Result<IoValue> {
    let profile = consensus_algorithm_profile_from_cluster_config(input, config)?;
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
