
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

pub const CONSENSUS_ENVIRONMENT_PRODUCTION: &str = "production";
pub const CONSENSUS_ENVIRONMENT_MODEL: &str = "model";
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

pub trait ConsensusEngineReadback {
    fn descriptor(&self) -> Result<ConsensusEngineDescriptor>;
    fn readback_summary(&self) -> Result<String>;
}

pub trait ConsensusEngineProposal {
    fn propose_transition(
        &self,
        runtime: &ControlRegistryRuntime,
        envelope_value: &IoValue,
    ) -> Result<ControlRegistryTransition>;
}

pub trait ConsensusEngineRead {
    fn read(&self, input: &ControlRegistryReadInput) -> Result<RaftReadReceipt>;
}

pub trait ConsensusEngineSnapshot {
    fn snapshot(&self, input: &RaftSnapshotInput) -> Result<RaftSnapshot>;
}

pub trait ConsensusEngineRecovery {
    fn recover(&self, input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt>;
}

pub trait ControlPlaneConsensusEngine:
    ConsensusEngineReadback + ConsensusEngineProposal + ConsensusEngineRead + ConsensusEngineSnapshot + ConsensusEngineRecovery
{
    fn propose(&self, runtime: &mut ControlRegistryRuntime, envelope_value: &IoValue) -> Result<ControlRegistryProposal> {
        let transition = self.propose_transition(runtime, envelope_value)?;
        apply_control_registry_transition(runtime, &transition);
        Ok(transition.proposal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaftControlPlaneEngine;

impl ConsensusEngineReadback for RaftControlPlaneEngine {
    fn descriptor(&self) -> Result<ConsensusEngineDescriptor> {
        default_raft_engine_descriptor()
    }

    fn readback_summary(&self) -> Result<String> {
        Ok(consensus_engine_readback_summary(&self.descriptor()?))
    }
}

impl ConsensusEngineProposal for RaftControlPlaneEngine {
    fn propose_transition(
        &self,
        runtime: &ControlRegistryRuntime,
        envelope_value: &IoValue,
    ) -> Result<ControlRegistryTransition> {
        propose_control_registry_transition_core(runtime, envelope_value)
    }
}

impl ConsensusEngineRead for RaftControlPlaneEngine {
    fn read(&self, input: &ControlRegistryReadInput) -> Result<RaftReadReceipt> {
        read_control_registry(input)
    }
}

impl ConsensusEngineSnapshot for RaftControlPlaneEngine {
    fn snapshot(&self, input: &RaftSnapshotInput) -> Result<RaftSnapshot> {
        snapshot_control_registry(input)
    }
}

impl ConsensusEngineRecovery for RaftControlPlaneEngine {
    fn recover(&self, input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt> {
        recover_control_registry(input)
    }
}

impl ControlPlaneConsensusEngine for RaftControlPlaneEngine {}

pub fn unsupported_consensus_capability(engine_profile: &str, capability: &str) -> Result<()> {
    Err(MoltenError::invalid_harness(format!(
        "consensus engine {engine_profile} does not support capability {capability}"
    )))
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
