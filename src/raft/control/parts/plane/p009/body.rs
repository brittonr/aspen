
const CONSENSUS_ENGINE_REGISTRY_SCHEMA: &str = "molten.consensus.engine-registry.v1";
const CONSENSUS_ENGINE_DESCRIPTOR_SCHEMA: &str = "molten.consensus.engine-descriptor.v1";
const CONSENSUS_ENGINE_ADMISSION_RECEIPT_SCHEMA: &str = "molten.consensus.engine-admission-receipt.v1";
const CONSENSUS_ENGINE_RECEIPT_SCHEMA: &str = "molten.consensus.engine-receipt.v1";
const CONSENSUS_ENGINE_SWITCHOVER_RECEIPT_SCHEMA: &str = "molten.consensus.engine-switchover-receipt.v1";
const CONSENSUS_ENGINE_EPOCH_GATE_SCHEMA: &str = "molten.consensus.engine-epoch-gate.v1";
const CONSENSUS_ENGINE_CONFORMANCE_RECEIPT_SCHEMA: &str = "molten.consensus.engine-conformance-receipt.v1";

const CONSENSUS_ENGINE_REGISTRY_FIELD_COUNT: usize = 3;
const CONSENSUS_ENGINE_DESCRIPTOR_FIELD_COUNT: usize = 13;
const CONSENSUS_ENGINE_ADMISSION_FIELD_COUNT: usize = 12;
const CONSENSUS_ENGINE_RECEIPT_FIELD_COUNT: usize = 13;
const CONSENSUS_ENGINE_SWITCHOVER_FIELD_COUNT: usize = 16;
const CONSENSUS_ENGINE_EPOCH_GATE_FIELD_COUNT: usize = 10;
const CONSENSUS_ENGINE_CONFORMANCE_FIELD_COUNT: usize = 11;
pub const DEFAULT_CONSENSUS_ENGINE_REGISTRY_LEN: usize = 3;

pub const INITIAL_CONSENSUS_ENGINE_EPOCH: u64 = 1;
pub const NEXT_CONSENSUS_ENGINE_EPOCH_STEP: u64 = 1;

const CONSENSUS_ENVIRONMENT_PRODUCTION: &str = "production";
const ENGINE_STATUS_DISABLED: &str = "disabled";
const ENGINE_DECISION_PASS: &str = "pass";
const ENGINE_DECISION_DENY: &str = "deny";
const ENGINE_DECISION_DIAGNOSTIC: &str = "diagnostic";

const ENGINE_CAPABILITY_PROPOSAL: &str = "proposal";
const ENGINE_CAPABILITY_LINEARIZABLE_READ: &str = "linearizable-read";
const ENGINE_CAPABILITY_LOCAL_STALE_READ: &str = "local-stale-read";
const ENGINE_CAPABILITY_SNAPSHOT: &str = "snapshot";
const ENGINE_CAPABILITY_RECOVERY: &str = "recovery";
const ENGINE_CAPABILITY_MEMBERSHIP_CONFIG: &str = "membership-config-transition";
const ENGINE_CAPABILITY_PLACEMENT_VALIDATION: &str = "placement-validation";
const ENGINE_CAPABILITY_READBACK_SUMMARY: &str = "readback-summary";
const ENGINE_CAPABILITY_SWITCHOVER: &str = "switchover-plan";

const CURRENTNESS_CLASS_RAFT_QUORUM_COMMIT: &str = "raft-quorum-commit";
const CURRENTNESS_CLASS_READ_INDEX: &str = "read-index-currentness";
const MEMBERSHIP_CAPABILITY_JOINT_CONSENSUS: &str = "joint-consensus-reviewed";
const MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED: &str = "deny-unsupported-transition";

const NORMALIZED_RECEIPT_KIND_COMMIT: &str = "commit";
const NORMALIZED_RECEIPT_KIND_READ: &str = "read";

const CONFORMANCE_CASE_PROPOSAL: &str = "proposal";
const CONFORMANCE_CASE_DUPLICATE_DENIAL: &str = "duplicate-operation-denial";
const CONFORMANCE_CASE_LINEARIZABLE_READ: &str = "linearizable-read-freshness";
const CONFORMANCE_CASE_LOCAL_STALE_READ: &str = "local-stale-classification";
const CONFORMANCE_CASE_SNAPSHOT_RECOVERY: &str = "snapshot-recovery";
const CONFORMANCE_CASE_MEMBERSHIP_DENIAL: &str = "membership-config-transition-denial";
const CONFORMANCE_CASE_CANONICAL_REPLAY: &str = "canonical-replay";
const CONFORMANCE_CASE_NORMALIZED_RECEIPT: &str = "normalized-receipt-shape";

const SUPPORTED_SWITCHOVER_ROLLBACK_POSTURES: &[&str] = &["rollback-supported", "rollback-denied-with-review"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineDescriptor {
    pub descriptor_ref: String,
    pub profile_id: String,
    pub profile_version: String,
    pub implementation_id: String,
    pub enabled: bool,
    pub supported_read_consistency_modes: Vec<String>,
    pub capabilities: Vec<String>,
    pub currentness_evidence_classes: Vec<String>,
    pub membership_capabilities: Vec<String>,
    pub production_admission_status: String,
    pub required_evidence_refs: Vec<String>,
    pub conformance_receipt_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineRegistry {
    pub registry_ref: String,
    pub entries: Vec<ConsensusEngineDescriptor>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineAdmissionInput {
    pub algorithm_profile: String,
    pub profile_version: String,
    pub requested_environment: String,
    pub requested_read_consistency: String,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineAdmissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub descriptor: Option<ConsensusEngineDescriptor>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub receipt_kind: String,
    pub engine_profile: String,
    pub profile_version: String,
    pub engine_epoch: u64,
    pub state_ref: Option<String>,
    pub source_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineSwitchoverInput {
    pub source_profile: String,
    pub source_version: String,
    pub target_profile: String,
    pub target_version: String,
    pub active_engine_epoch: u64,
    pub target_engine_epoch: u64,
    pub source_state_ref: String,
    pub target_bootstrap_state_ref: String,
    pub membership_refs: Vec<String>,
    pub placement_refs: Vec<String>,
    pub replay_conformance_refs: Vec<String>,
    pub currentness_evidence_refs: Vec<String>,
    pub operator_approval_refs: Vec<String>,
    pub rollback_posture: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineSwitchoverReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub source_profile: String,
    pub target_profile: String,
    pub target_engine_epoch: u64,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineEpochGateInput {
    pub operation: String,
    pub active_profile: String,
    pub active_engine_epoch: u64,
    pub presented_profile: String,
    pub presented_engine_epoch: u64,
    pub activation_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineEpochGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineConformanceInput {
    pub algorithm_profile: String,
    pub profile_version: String,
    pub fixture_id: String,
    pub passed_cases: Vec<String>,
    pub expected_state_ref: String,
    pub actual_state_ref: String,
    pub normalized_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusEngineConformanceReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub fixture_id: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

pub trait ControlPlaneConsensusEngine {
    fn descriptor(&self) -> Result<ConsensusEngineDescriptor>;
    fn readback_summary(&self) -> Result<String>;
    fn propose(&self, runtime: &mut ControlRegistryRuntime, envelope_value: &IoValue) -> Result<ControlRegistryProposal>;
    fn read(&self, input: &ControlRegistryReadInput) -> Result<RaftReadReceipt>;
    fn snapshot(&self, input: &RaftSnapshotInput) -> Result<RaftSnapshot>;
    fn recover(&self, input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaftControlPlaneEngine;

impl ControlPlaneConsensusEngine for RaftControlPlaneEngine {
    fn descriptor(&self) -> Result<ConsensusEngineDescriptor> {
        default_raft_engine_descriptor()
    }

    fn readback_summary(&self) -> Result<String> {
        Ok(consensus_engine_readback_summary(&self.descriptor()?))
    }

    fn propose(&self, runtime: &mut ControlRegistryRuntime, envelope_value: &IoValue) -> Result<ControlRegistryProposal> {
        propose_control_registry_command(runtime, envelope_value)
    }

    fn read(&self, input: &ControlRegistryReadInput) -> Result<RaftReadReceipt> {
        read_control_registry(input)
    }

    fn snapshot(&self, input: &RaftSnapshotInput) -> Result<RaftSnapshot> {
        snapshot_control_registry(input)
    }

    fn recover(&self, input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt> {
        recover_control_registry(input)
    }
}

// r[impl molten.consensus.engine_registry]
pub fn default_consensus_engine_registry() -> Result<ConsensusEngineRegistry> {
    let entries = vec![
        default_raft_engine_descriptor()?,
        leaderless_experimental_engine_descriptor()?,
        disabled_fixture_engine_descriptor()?,
    ];
    parse_consensus_engine_registry(&consensus_engine_registry_value(&entries)?)
}

pub fn default_raft_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        implementation_id: "in-process-raft-control-registry-v1",
        enabled: true,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE, READ_CONSISTENCY_LOCAL_STALE],
        capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL,
            ENGINE_CAPABILITY_LINEARIZABLE_READ,
            ENGINE_CAPABILITY_LOCAL_STALE_READ,
            ENGINE_CAPABILITY_SNAPSHOT,
            ENGINE_CAPABILITY_RECOVERY,
            ENGINE_CAPABILITY_MEMBERSHIP_CONFIG,
            ENGINE_CAPABILITY_PLACEMENT_VALIDATION,
            ENGINE_CAPABILITY_READBACK_SUMMARY,
            ENGINE_CAPABILITY_SWITCHOVER,
        ],
        currentness_evidence_classes: vec![CURRENTNESS_CLASS_RAFT_QUORUM_COMMIT, CURRENTNESS_CLASS_READ_INDEX],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_JOINT_CONSENSUS, MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: PRODUCTION_STATUS_ADMITTED,
        required_evidence_refs: vec![
            synthetic_ref("raft-implementation-evidence")?,
            synthetic_ref("raft-proof-model-evidence")?,
            synthetic_ref("raft-simulation-evidence")?,
            synthetic_ref("raft-placement-evidence")?,
            synthetic_ref("raft-membership-evidence")?,
        ],
        conformance_receipt_refs: vec![synthetic_ref("raft-conformance-receipt")?],
        caveats: default_consensus_caveats(),
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

fn leaderless_experimental_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL,
        profile_version: CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL,
        implementation_id: "in-process-leaderless-quorum-diagnostic-v1",
        enabled: true,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE, READ_CONSISTENCY_LOCAL_STALE],
        capabilities: vec![ENGINE_CAPABILITY_LINEARIZABLE_READ, ENGINE_CAPABILITY_LOCAL_STALE_READ],
        currentness_evidence_classes: vec!["leaderless-quorum-certificate"],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: PRODUCTION_STATUS_EXPERIMENTAL,
        required_evidence_refs: Vec::new(),
        conformance_receipt_refs: Vec::new(),
        caveats: default_consensus_caveats(),
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

fn disabled_fixture_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: "disabled-fixture-engine",
        profile_version: "disabled-fixture-v1",
        implementation_id: "disabled-fixture-implementation-v1",
        enabled: false,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE],
        capabilities: vec![ENGINE_CAPABILITY_PROPOSAL],
        currentness_evidence_classes: vec!["fixture-currentness"],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: ENGINE_STATUS_DISABLED,
        required_evidence_refs: vec![synthetic_ref("disabled-evidence")?],
        conformance_receipt_refs: vec![synthetic_ref("disabled-conformance")?],
        caveats: vec!["disabled fixture only".to_string()],
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

struct ConsensusEngineDescriptorInput<'a> {
    profile_id: &'a str,
    profile_version: &'a str,
    implementation_id: &'a str,
    enabled: bool,
    supported_read_consistency_modes: Vec<&'a str>,
    capabilities: Vec<&'a str>,
    currentness_evidence_classes: Vec<&'a str>,
    membership_capabilities: Vec<&'a str>,
    production_admission_status: &'a str,
    required_evidence_refs: Vec<String>,
    conformance_receipt_refs: Vec<String>,
    caveats: Vec<String>,
}

fn consensus_engine_descriptor_value(input: &ConsensusEngineDescriptorInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.profile_id, "consensus engine profile")?;
    validate_non_empty(input.profile_version, "consensus engine profile version")?;
    validate_non_empty(input.implementation_id, "consensus engine implementation id")?;
    validate_refs(&input.required_evidence_refs, "consensus engine evidence ref")?;
    validate_refs(&input.conformance_receipt_refs, "consensus engine conformance ref")?;
    validate_caveats(&input.caveats)?;
    Ok(record("consensus-engine-descriptor-v1", vec![
        string(CONSENSUS_ENGINE_DESCRIPTOR_SCHEMA),
        record("profile", vec![string(input.profile_id)]),
        record("version", vec![string(input.profile_version)]),
        record("implementation", vec![string(input.implementation_id)]),
        record("enabled", vec![bool_value(input.enabled)]),
        record("read-consistency", vec![strings_sequence(&borrowed_strings(&input.supported_read_consistency_modes))]),
        record("capabilities", vec![strings_sequence(&borrowed_strings(&input.capabilities))]),
        record("currentness", vec![strings_sequence(&borrowed_strings(&input.currentness_evidence_classes))]),
        record("membership", vec![strings_sequence(&borrowed_strings(&input.membership_capabilities))]),
        record("production-status", vec![string(input.production_admission_status)]),
        record("evidence", vec![strings_sequence(&input.required_evidence_refs)]),
        record("conformance", vec![strings_sequence(&input.conformance_receipt_refs)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
    ]))
}

fn borrowed_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn parse_consensus_engine_descriptor(value: &IoValue) -> Result<ConsensusEngineDescriptor> {
    let fields = value
        .collect_simple_record("consensus-engine-descriptor-v1", Some(CONSENSUS_ENGINE_DESCRIPTOR_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-descriptor-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_DESCRIPTOR_SCHEMA, "consensus engine descriptor schema")?;
    let profile_id = record_string(&fields[1], "profile")?;
    let profile_version = record_string(&fields[2], "version")?;
    let implementation_id = record_string(&fields[3], "implementation")?;
    let enabled = record_bool(&fields[4], "enabled")?;
    let supported_read_consistency_modes = parse_string_sequence(&fields[5], "read-consistency")?;
    validate_read_consistency_support(&supported_read_consistency_modes)?;
    let capabilities = parse_string_sequence(&fields[6], "capabilities")?;
    let currentness_evidence_classes = parse_string_sequence(&fields[7], "currentness")?;
    let membership_capabilities = parse_string_sequence(&fields[8], "membership")?;
    let production_admission_status = record_string(&fields[9], "production-status")?;
    let required_evidence_refs = parse_ref_sequence(&fields[10], "evidence")?;
    let conformance_receipt_refs = parse_ref_sequence(&fields[11], "conformance")?;
    let caveats = parse_string_sequence(&fields[12], "caveats")?;
    validate_engine_descriptor_core(
        &profile_id,
        &profile_version,
        &implementation_id,
        &capabilities,
        &currentness_evidence_classes,
        &membership_capabilities,
        &caveats,
    )?;
    Ok(ConsensusEngineDescriptor {
        descriptor_ref: canonical_hash(value)?,
        profile_id,
        profile_version,
        implementation_id,
        enabled,
        supported_read_consistency_modes,
        capabilities,
        currentness_evidence_classes,
        membership_capabilities,
        production_admission_status,
        required_evidence_refs,
        conformance_receipt_refs,
        caveats,
        value: value.clone(),
    })
}

fn validate_engine_descriptor_core(
    profile_id: &str,
    profile_version: &str,
    implementation_id: &str,
    capabilities: &[String],
    currentness: &[String],
    membership: &[String],
    caveats: &[String],
) -> Result<()> {
    validate_non_empty(profile_id, "consensus engine profile")?;
    validate_non_empty(profile_version, "consensus engine profile version")?;
    validate_non_empty(implementation_id, "consensus engine implementation id")?;
    validate_string_items(capabilities, "consensus engine capability")?;
    validate_string_items(currentness, "consensus engine currentness class")?;
    validate_string_items(membership, "consensus engine membership capability")?;
    validate_caveats(caveats)
}

fn consensus_engine_registry_value(entries: &[ConsensusEngineDescriptor]) -> Result<IoValue> {
    ensure_count_at_most(entries.len(), MAX_RAFT_REFS, "consensus engine registry entries")?;
    if entries.is_empty() {
        return Err(MoltenError::invalid_harness("consensus engine registry requires entries"));
    }
    Ok(record("consensus-engine-registry-v1", vec![
        string(CONSENSUS_ENGINE_REGISTRY_SCHEMA),
        record("entries", vec![sequence(entries.iter().map(|entry| entry.value.clone()).collect())]),
        checks_value(&[("explicit-engine-registry", ENGINE_DECISION_PASS), ("fail-closed-resolution", ENGINE_DECISION_PASS)]),
    ]))
}

pub fn parse_consensus_engine_registry(value: &IoValue) -> Result<ConsensusEngineRegistry> {
    let fields = value
        .collect_simple_record("consensus-engine-registry-v1", Some(CONSENSUS_ENGINE_REGISTRY_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-registry-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_REGISTRY_SCHEMA, "consensus engine registry schema")?;
    let values = field_sequence(&fields[1], "entries")?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, "consensus engine registry entries")?;
    let mut entries = Vec::with_capacity(values.len());
    for value in values {
        entries.push(parse_consensus_engine_descriptor(&value_to_iovalue(&value))?);
    }
    require_unique_engine_keys(&entries)?;
    require_check(&parse_checks(&fields[2])?, "explicit-engine-registry", "consensus engine registry")?;
    Ok(ConsensusEngineRegistry {
        registry_ref: canonical_hash(value)?,
        entries,
        value: value.clone(),
    })
}

fn require_unique_engine_keys(entries: &[ConsensusEngineDescriptor]) -> Result<()> {
    let mut keys = std::collections::BTreeSet::new();
    for entry in entries {
        let key = engine_key(&entry.profile_id, &entry.profile_version);
        if !keys.insert(key.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate consensus engine registry entry {key}")));
        }
    }
    Ok(())
}

fn engine_key(profile_id: &str, profile_version: &str) -> String {
    format!("{profile_id}@{profile_version}")
}

// r[impl molten.consensus.engine_admission_policy]
// r[impl molten.testing.consensus_registry_negative_fixtures]
pub fn admit_consensus_engine(
    registry: &ConsensusEngineRegistry,
    input: &ConsensusEngineAdmissionInput,
) -> Result<ConsensusEngineAdmissionReceipt> {
    let descriptor = matching_engine_descriptor(registry, &input.algorithm_profile, &input.profile_version);
    let mut diagnostics = Vec::new();
    if descriptor.is_none() {
        if registry.entries.iter().any(|entry| entry.profile_id == input.algorithm_profile) {
            diagnostics.push(format!(
                "consensus engine profile version mismatch for {} version {}",
                input.algorithm_profile, input.profile_version
            ));
        } else {
            diagnostics.push(format!(
                "unsupported consensus engine profile {} version {}",
                input.algorithm_profile, input.profile_version
            ));
        }
    }
    if let Some(descriptor) = descriptor {
        collect_engine_admission_diagnostics(descriptor, input, &mut diagnostics)?;
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus engine admission diagnostics")?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_admission_receipt_value(input, descriptor, decision, &diagnostics)?;
    Ok(ConsensusEngineAdmissionReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        descriptor: descriptor.cloned(),
        diagnostics,
        value,
    })
}

fn matching_engine_descriptor<'a>(
    registry: &'a ConsensusEngineRegistry,
    profile_id: &str,
    profile_version: &str,
) -> Option<&'a ConsensusEngineDescriptor> {
    registry
        .entries
        .iter()
        .find(|entry| entry.profile_id == profile_id && entry.profile_version == profile_version)
}

fn collect_engine_admission_diagnostics(
    descriptor: &ConsensusEngineDescriptor,
    input: &ConsensusEngineAdmissionInput,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if !descriptor.enabled {
        diagnostics.push(format!("consensus engine {} is disabled", descriptor.profile_id));
    }
    if input.requested_environment == CONSENSUS_ENVIRONMENT_PRODUCTION
        && descriptor.production_admission_status != PRODUCTION_STATUS_ADMITTED
    {
        diagnostics.push(format!(
            "consensus engine {} is not admitted for production runtime; status {}",
            descriptor.profile_id, descriptor.production_admission_status
        ));
    }
    if descriptor.required_evidence_refs.is_empty() {
        diagnostics.push(format!("consensus engine {} missing proof/model evidence", descriptor.profile_id));
    }
    if descriptor.conformance_receipt_refs.is_empty() {
        diagnostics.push(format!("consensus engine {} missing conformance refs", descriptor.profile_id));
    }
    if !descriptor
        .supported_read_consistency_modes
        .iter()
        .any(|mode| mode == &input.requested_read_consistency)
    {
        diagnostics.push(format!(
            "unsupported read consistency mode {} for consensus engine {}",
            input.requested_read_consistency, descriptor.profile_id
        ));
    }
    for capability in &input.required_capabilities {
        if !descriptor.capabilities.iter().any(|value| value == capability) {
            diagnostics.push(format!(
                "unsupported consensus engine capability {capability} for {}",
                descriptor.profile_id
            ));
        }
    }
    ensure_count_at_most(input.required_capabilities.len(), MAX_RAFT_REFS, "consensus engine required capabilities")
}

fn consensus_engine_admission_receipt_value(
    input: &ConsensusEngineAdmissionInput,
    descriptor: Option<&ConsensusEngineDescriptor>,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_read_consistency_mode(&input.requested_read_consistency)?;
    validate_string_items(&input.required_capabilities, "consensus engine required capability")?;
    Ok(record("consensus-engine-admission-receipt-v1", vec![
        string(CONSENSUS_ENGINE_ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&input.algorithm_profile)]),
        record("version", vec![string(&input.profile_version)]),
        record("environment", vec![string(&input.requested_environment)]),
        record("read-consistency", vec![string(&input.requested_read_consistency)]),
        record("capabilities", vec![strings_sequence(&input.required_capabilities)]),
        record("descriptor", vec![optional_ref_value(descriptor.map(|entry| entry.descriptor_ref.as_str()))]),
        record("implementation", vec![string(descriptor.map_or("none", |entry| entry.implementation_id.as_str()))]),
        record("evidence", vec![strings_sequence(descriptor.map_or(&[][..], |entry| entry.required_evidence_refs.as_slice()))]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("engine-registry-resolved", decision),
            ("production-admission-policy", decision),
            ("conformance-evidence-bound", if descriptor.is_some_and(|entry| !entry.conformance_receipt_refs.is_empty()) { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY }),
        ]),
    ]))
}

pub fn parse_consensus_engine_admission_receipt(value: &IoValue) -> Result<ConsensusEngineAdmissionReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-admission-receipt-v1", Some(CONSENSUS_ENGINE_ADMISSION_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-admission-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_ADMISSION_RECEIPT_SCHEMA, "consensus engine admission schema")?;
    require_check(&parse_checks(&fields[11])?, "engine-registry-resolved", "consensus engine admission")?;
    Ok(ConsensusEngineAdmissionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        descriptor: None,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn resolve_control_registry_engine(manifest: &RaftGroupManifest) -> Result<ConsensusEngineAdmissionReceipt> {
    let registry = default_consensus_engine_registry()?;
    admit_consensus_engine(&registry, &ConsensusEngineAdmissionInput {
        algorithm_profile: manifest.algorithm_profile.clone(),
        profile_version: manifest.admitted_profile_version.clone(),
        requested_environment: CONSENSUS_ENVIRONMENT_PRODUCTION.to_string(),
        requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        required_capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL.to_string(),
            ENGINE_CAPABILITY_LINEARIZABLE_READ.to_string(),
            ENGINE_CAPABILITY_SNAPSHOT.to_string(),
            ENGINE_CAPABILITY_RECOVERY.to_string(),
        ],
    })
}

pub fn consensus_engine_readback_summary(descriptor: &ConsensusEngineDescriptor) -> String {
    format!(
        "consensus-engine profile={} version={} implementation={} enabled={} production={} capabilities={} currentness={} conformance={} caveats={}",
        descriptor.profile_id,
        descriptor.profile_version,
        descriptor.implementation_id,
        descriptor.enabled,
        descriptor.production_admission_status,
        descriptor.capabilities.join(","),
        descriptor.currentness_evidence_classes.join(","),
        descriptor.conformance_receipt_refs.len(),
        descriptor.caveats.join(",")
    )
}

// r[impl molten.consensus.engine_interface]
// r[impl molten.consensus.engine_portable_state]
pub fn normalized_raft_commit_receipt_value(receipt: &RaftCommitReceipt, engine_epoch: u64) -> Result<IoValue> {
    let state_ref = receipt.log_entry_ref.as_deref();
    consensus_engine_receipt_value(&ConsensusEngineReceiptValueInput {
        receipt_kind: NORMALIZED_RECEIPT_KIND_COMMIT,
        decision: &receipt.decision,
        engine_profile: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        engine_epoch,
        group_ref: &receipt.group_ref,
        operation_ref: &receipt.command_ref,
        state_ref,
        currentness_ref: receipt.log_entry_ref.as_deref(),
        source_receipt_ref: Some(&receipt.receipt_ref),
        evidence_refs: receipt.log_entry_ref.as_slice(),
        diagnostics: &[],
    })
}

pub fn normalized_raft_read_receipt_value(receipt: &RaftReadReceipt, engine_epoch: u64) -> Result<IoValue> {
    let read_fields = read_receipt_fields(&receipt.value)?;
    consensus_engine_receipt_value(&ConsensusEngineReceiptValueInput {
        receipt_kind: NORMALIZED_RECEIPT_KIND_READ,
        decision: &receipt.decision,
        engine_profile: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        engine_epoch,
        group_ref: &read_fields.group_ref,
        operation_ref: receipt.target_ref.as_deref().unwrap_or(&read_fields.state_ref),
        state_ref: Some(&read_fields.state_ref),
        currentness_ref: Some(&receipt.receipt_ref),
        source_receipt_ref: Some(&receipt.receipt_ref),
        evidence_refs: read_fields.read_index_predicate_ref.as_slice(),
        diagnostics: &receipt.diagnostics,
    })
}

struct ReadReceiptFields {
    group_ref: String,
    state_ref: String,
    read_index_predicate_ref: Option<String>,
}

fn read_receipt_fields(value: &IoValue) -> Result<ReadReceiptFields> {
    let fields = value
        .collect_simple_record("raft-read-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-read-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_READ_RECEIPT_SCHEMA, "raft read receipt schema")?;
    Ok(ReadReceiptFields {
        group_ref: record_ref(&fields[2], "group")?,
        state_ref: record_ref(&fields[3], "state")?,
        read_index_predicate_ref: record_optional_ref(&fields[10], "read-index-predicate")?,
    })
}

struct ConsensusEngineReceiptValueInput<'a> {
    receipt_kind: &'a str,
    decision: &'a str,
    engine_profile: &'a str,
    profile_version: &'a str,
    engine_epoch: u64,
    group_ref: &'a str,
    operation_ref: &'a str,
    state_ref: Option<&'a str>,
    currentness_ref: Option<&'a str>,
    source_receipt_ref: Option<&'a str>,
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

fn consensus_engine_receipt_value(input: &ConsensusEngineReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_algorithm_name(input.engine_profile)?;
    validate_non_empty(input.profile_version, "consensus engine receipt profile version")?;
    validate_non_empty(input.receipt_kind, "consensus engine receipt kind")?;
    require_ref(input.group_ref, "consensus engine receipt group ref")?;
    require_ref(input.operation_ref, "consensus engine receipt operation ref")?;
    if let Some(reference) = input.state_ref {
        require_ref(reference, "consensus engine receipt state ref")?;
    }
    if let Some(reference) = input.currentness_ref {
        require_ref(reference, "consensus engine receipt currentness ref")?;
    }
    if let Some(reference) = input.source_receipt_ref {
        require_ref(reference, "consensus engine source receipt ref")?;
    }
    validate_refs(input.evidence_refs, "consensus engine receipt evidence ref")?;
    validate_diagnostic_strings(input.diagnostics, "consensus engine receipt diagnostic")?;
    Ok(record("consensus-engine-receipt-v1", vec![
        string(CONSENSUS_ENGINE_RECEIPT_SCHEMA),
        record("kind", vec![string(input.receipt_kind)]),
        record("decision", vec![string(input.decision)]),
        record("engine-profile", vec![string(input.engine_profile)]),
        record("profile-version", vec![string(input.profile_version)]),
        record("engine-epoch", vec![u64_value(input.engine_epoch)]),
        record("group", vec![string(input.group_ref)]),
        record("operation", vec![string(input.operation_ref)]),
        record("state", vec![optional_ref_value(input.state_ref)]),
        record("currentness", vec![optional_ref_value(input.currentness_ref)]),
        record("source-receipt", vec![optional_ref_value(input.source_receipt_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
    ]))
}

pub fn parse_consensus_engine_receipt(value: &IoValue) -> Result<ConsensusEngineReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-receipt-v1", Some(CONSENSUS_ENGINE_RECEIPT_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_RECEIPT_SCHEMA, "consensus engine receipt schema")?;
    let receipt_kind = record_string(&fields[1], "kind")?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    Ok(ConsensusEngineReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        receipt_kind,
        engine_profile: record_string(&fields[3], "engine-profile")?,
        profile_version: record_string(&fields[4], "profile-version")?,
        engine_epoch: record_u64(&fields[5], "engine-epoch")?,
        state_ref: record_optional_ref(&fields[8], "state")?,
        source_receipt_ref: record_optional_ref(&fields[10], "source-receipt")?,
        diagnostics: parse_string_sequence(&fields[12], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.consensus.engine_switchover_receipts]
// r[impl molten.testing.consensus_switchover_fixtures]
pub fn consensus_engine_switchover_receipt(
    input: &ConsensusEngineSwitchoverInput,
) -> Result<ConsensusEngineSwitchoverReceipt> {
    validate_switchover_input(input)?;
    let registry = default_consensus_engine_registry()?;
    let target_admission = admit_consensus_engine(&registry, &ConsensusEngineAdmissionInput {
        algorithm_profile: input.target_profile.clone(),
        profile_version: input.target_version.clone(),
        requested_environment: CONSENSUS_ENVIRONMENT_PRODUCTION.to_string(),
        requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        required_capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL.to_string(),
            ENGINE_CAPABILITY_LINEARIZABLE_READ.to_string(),
            ENGINE_CAPABILITY_SWITCHOVER.to_string(),
        ],
    })?;
    let mut diagnostics = switchover_diagnostics(input, &target_admission)?;
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus switchover diagnostics")?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_switchover_receipt_value(input, decision, &diagnostics, &target_admission.receipt_ref)?;
    Ok(ConsensusEngineSwitchoverReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        source_profile: input.source_profile.clone(),
        target_profile: input.target_profile.clone(),
        target_engine_epoch: input.target_engine_epoch,
        diagnostics: std::mem::take(&mut diagnostics),
        value,
    })
}

fn validate_switchover_input(input: &ConsensusEngineSwitchoverInput) -> Result<()> {
    validate_algorithm_name(&input.source_profile)?;
    validate_algorithm_name(&input.target_profile)?;
    validate_non_empty(&input.source_version, "source consensus profile version")?;
    validate_non_empty(&input.target_version, "target consensus profile version")?;
    require_ref(&input.source_state_ref, "switchover source state ref")?;
    require_ref(&input.target_bootstrap_state_ref, "switchover target bootstrap state ref")?;
    validate_refs(&input.membership_refs, "switchover membership ref")?;
    validate_refs(&input.placement_refs, "switchover placement ref")?;
    validate_refs(&input.replay_conformance_refs, "switchover replay conformance ref")?;
    validate_refs(&input.currentness_evidence_refs, "switchover currentness ref")?;
    validate_refs(&input.operator_approval_refs, "switchover operator approval ref")?;
    validate_non_empty(&input.rollback_posture, "switchover rollback posture")
}

fn switchover_diagnostics(
    input: &ConsensusEngineSwitchoverInput,
    target_admission: &ConsensusEngineAdmissionReceipt,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.source_profile == input.target_profile && input.source_version == input.target_version {
        diagnostics.push("switchover target must differ from source profile/version".to_string());
    }
    if input.target_engine_epoch <= input.active_engine_epoch {
        diagnostics.push(format!(
            "target engine epoch {} must advance active epoch {}",
            input.target_engine_epoch, input.active_engine_epoch
        ));
    }
    if input.membership_refs.is_empty() {
        diagnostics.push("switchover requires membership/config refs".to_string());
    }
    if input.placement_refs.is_empty() {
        diagnostics.push("switchover requires placement refs".to_string());
    }
    if input.replay_conformance_refs.is_empty() {
        diagnostics.push("switchover requires replay/conformance evidence".to_string());
    }
    if input.currentness_evidence_refs.is_empty() {
        diagnostics.push("switchover requires current source-state evidence".to_string());
    }
    if input.operator_approval_refs.is_empty() {
        diagnostics.push("switchover requires operator approval refs".to_string());
    }
    if !SUPPORTED_SWITCHOVER_ROLLBACK_POSTURES.contains(&input.rollback_posture.as_str()) {
        diagnostics.push(format!("unsupported switchover rollback posture {}", input.rollback_posture));
    }
    if target_admission.decision != ENGINE_DECISION_PASS {
        diagnostics.push(format!("target engine admission denied: {}", target_admission.diagnostics.join(";")));
    }
    Ok(diagnostics)
}

fn consensus_engine_switchover_receipt_value(
    input: &ConsensusEngineSwitchoverInput,
    decision: &str,
    diagnostics: &[String],
    target_admission_ref: &str,
) -> Result<IoValue> {
    Ok(record("consensus-engine-switchover-receipt-v1", vec![
        string(CONSENSUS_ENGINE_SWITCHOVER_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("source", vec![string(&input.source_profile), string(&input.source_version)]),
        record("target", vec![string(&input.target_profile), string(&input.target_version)]),
        record("active-epoch", vec![u64_value(input.active_engine_epoch)]),
        record("target-epoch", vec![u64_value(input.target_engine_epoch)]),
        record("source-state", vec![string(&input.source_state_ref)]),
        record("target-bootstrap", vec![string(&input.target_bootstrap_state_ref)]),
        record("membership", vec![strings_sequence(&input.membership_refs)]),
        record("placement", vec![strings_sequence(&input.placement_refs)]),
        record("replay-conformance", vec![strings_sequence(&input.replay_conformance_refs)]),
        record("currentness", vec![strings_sequence(&input.currentness_evidence_refs)]),
        record("operator-approval", vec![strings_sequence(&input.operator_approval_refs)]),
        record("rollback", vec![string(&input.rollback_posture)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("target-engine-admitted", decision),
            ("fencing-epoch-advanced", decision),
            ("replay-conformance-bound", if input.replay_conformance_refs.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
            ("rollback-posture-declared", ENGINE_DECISION_PASS),
            ("target-admission-receipt", if target_admission_ref.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_switchover_receipt(value: &IoValue) -> Result<ConsensusEngineSwitchoverReceipt> {
    let fields = value
        .collect_simple_record(
            "consensus-engine-switchover-receipt-v1",
            Some(CONSENSUS_ENGINE_SWITCHOVER_FIELD_COUNT),
        )
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-switchover-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_SWITCHOVER_RECEIPT_SCHEMA, "consensus switchover schema")?;
    require_check(&parse_checks(&fields[15])?, "fencing-epoch-advanced", "consensus switchover receipt")?;
    let source_fields = value_to_iovalue(&fields[2]);
    let source = source_fields
        .collect_simple_record("source", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected consensus switchover source"))?;
    let target_fields = value_to_iovalue(&fields[3]);
    let target = target_fields
        .collect_simple_record("target", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected consensus switchover target"))?;
    Ok(ConsensusEngineSwitchoverReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        source_profile: required_string(&source[0], "source profile")?,
        target_profile: required_string(&target[0], "target profile")?,
        target_engine_epoch: record_u64(&fields[5], "target-epoch")?,
        diagnostics: parse_string_sequence(&fields[14], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.consensus.engine_switchover_fencing]
pub fn consensus_engine_epoch_gate(input: &ConsensusEngineEpochGateInput) -> Result<ConsensusEngineEpochGateReceipt> {
    validate_epoch_gate_input(input)?;
    let diagnostics = epoch_gate_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_epoch_gate_value(input, decision, &diagnostics)?;
    Ok(ConsensusEngineEpochGateReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        diagnostics,
        value,
    })
}

fn validate_epoch_gate_input(input: &ConsensusEngineEpochGateInput) -> Result<()> {
    validate_non_empty(&input.operation, "consensus engine epoch gate operation")?;
    validate_algorithm_name(&input.active_profile)?;
    validate_algorithm_name(&input.presented_profile)?;
    if let Some(reference) = &input.activation_receipt_ref {
        require_ref(reference, "engine activation receipt ref")?;
    }
    Ok(())
}

fn epoch_gate_diagnostics(input: &ConsensusEngineEpochGateInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.presented_profile != input.active_profile {
        diagnostics.push(format!(
            "inactive consensus engine profile {}; active profile {}",
            input.presented_profile, input.active_profile
        ));
    }
    if input.presented_engine_epoch < input.active_engine_epoch {
        diagnostics.push(format!(
            "stale engine epoch {}; active epoch {}",
            input.presented_engine_epoch, input.active_engine_epoch
        ));
    }
    if input.presented_engine_epoch > input.active_engine_epoch && input.activation_receipt_ref.is_none() {
        diagnostics.push(format!(
            "target engine epoch {} is not activated by a committed switchover receipt",
            input.presented_engine_epoch
        ));
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus engine epoch diagnostics")?;
    Ok(diagnostics)
}

fn consensus_engine_epoch_gate_value(
    input: &ConsensusEngineEpochGateInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-engine-epoch-gate-v1", vec![
        string(CONSENSUS_ENGINE_EPOCH_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("operation", vec![string(&input.operation)]),
        record("active-profile", vec![string(&input.active_profile)]),
        record("active-epoch", vec![u64_value(input.active_engine_epoch)]),
        record("presented-profile", vec![string(&input.presented_profile)]),
        record("presented-epoch", vec![u64_value(input.presented_engine_epoch)]),
        record("activation", vec![optional_ref_value(input.activation_receipt_ref.as_deref())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("active-engine-epoch-bound", decision),
            ("stale-writer-fenced", if diagnostics.iter().any(|value| value.contains("stale engine epoch")) { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
            ("target-read-activation", if diagnostics.iter().any(|value| value.contains("not activated")) { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_epoch_gate(value: &IoValue) -> Result<ConsensusEngineEpochGateReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-epoch-gate-v1", Some(CONSENSUS_ENGINE_EPOCH_GATE_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-epoch-gate-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_EPOCH_GATE_SCHEMA, "consensus engine epoch gate schema")?;
    require_check(&parse_checks(&fields[9])?, "active-engine-epoch-bound", "consensus engine epoch gate")?;
    Ok(ConsensusEngineEpochGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        diagnostics: parse_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.testing.consensus_engine_conformance]
pub fn consensus_engine_conformance_receipt(
    input: &ConsensusEngineConformanceInput,
) -> Result<ConsensusEngineConformanceReceipt> {
    validate_conformance_input(input)?;
    let diagnostics = conformance_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_conformance_receipt_value(input, decision, &diagnostics)?;
    Ok(ConsensusEngineConformanceReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        fixture_id: input.fixture_id.clone(),
        diagnostics,
        value,
    })
}

fn validate_conformance_input(input: &ConsensusEngineConformanceInput) -> Result<()> {
    validate_algorithm_name(&input.algorithm_profile)?;
    validate_non_empty(&input.profile_version, "consensus conformance profile version")?;
    validate_non_empty(&input.fixture_id, "consensus conformance fixture id")?;
    validate_string_items(&input.passed_cases, "consensus conformance case")?;
    require_ref(&input.expected_state_ref, "consensus conformance expected state ref")?;
    require_ref(&input.actual_state_ref, "consensus conformance actual state ref")?;
    validate_refs(&input.normalized_receipt_refs, "consensus conformance normalized receipt ref")
}

fn conformance_diagnostics(input: &ConsensusEngineConformanceInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for required in required_conformance_cases() {
        if !input.passed_cases.iter().any(|value| value == required) {
            diagnostics.push(format!("missing consensus engine conformance case {required}"));
        }
    }
    if input.expected_state_ref != input.actual_state_ref {
        diagnostics.push(format!(
            "consensus engine replay state mismatch expected {} actual {}",
            input.expected_state_ref, input.actual_state_ref
        ));
    }
    if input.normalized_receipt_refs.is_empty() {
        diagnostics.push("consensus engine conformance requires normalized receipt refs".to_string());
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus conformance diagnostics")?;
    Ok(diagnostics)
}

fn required_conformance_cases() -> &'static [&'static str] {
    &[
        CONFORMANCE_CASE_PROPOSAL,
        CONFORMANCE_CASE_DUPLICATE_DENIAL,
        CONFORMANCE_CASE_LINEARIZABLE_READ,
        CONFORMANCE_CASE_LOCAL_STALE_READ,
        CONFORMANCE_CASE_SNAPSHOT_RECOVERY,
        CONFORMANCE_CASE_MEMBERSHIP_DENIAL,
        CONFORMANCE_CASE_CANONICAL_REPLAY,
        CONFORMANCE_CASE_NORMALIZED_RECEIPT,
    ]
}

fn consensus_engine_conformance_receipt_value(
    input: &ConsensusEngineConformanceInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-engine-conformance-receipt-v1", vec![
        string(CONSENSUS_ENGINE_CONFORMANCE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&input.algorithm_profile)]),
        record("version", vec![string(&input.profile_version)]),
        record("fixture", vec![string(&input.fixture_id)]),
        record("cases", vec![strings_sequence(&input.passed_cases)]),
        record("expected-state", vec![string(&input.expected_state_ref)]),
        record("actual-state", vec![string(&input.actual_state_ref)]),
        record("normalized", vec![strings_sequence(&input.normalized_receipt_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("deterministic-engine-conformance", decision),
            ("canonical-replay-state", if input.expected_state_ref == input.actual_state_ref { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY }),
            ("normalized-receipt-shape", if input.normalized_receipt_refs.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_conformance_receipt(value: &IoValue) -> Result<ConsensusEngineConformanceReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-conformance-receipt-v1", Some(CONSENSUS_ENGINE_CONFORMANCE_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-conformance-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_CONFORMANCE_RECEIPT_SCHEMA, "consensus conformance schema")?;
    require_check(&parse_checks(&fields[10])?, "deterministic-engine-conformance", "consensus conformance receipt")?;
    Ok(ConsensusEngineConformanceReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        fixture_id: record_string(&fields[4], "fixture")?,
        diagnostics: parse_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

fn validate_string_items(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} list must not be empty")));
    }
    for value in values {
        validate_non_empty(value, label)?;
    }
    Ok(())
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        ENGINE_DECISION_PASS | ENGINE_DECISION_DENY | ENGINE_DECISION_DIAGNOSTIC => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported consensus engine decision {value}"))),
    }
}
