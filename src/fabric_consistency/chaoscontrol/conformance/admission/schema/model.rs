pub const EXPECTED_SCHEMA: &str = "molten.chaoscontrol-expected-cohort.v1";
pub const BUNDLE_SCHEMA: &str = "molten.chaoscontrol-conformance-bundle.v1";
pub const RECEIPT_RECORD_LABEL: &str = "external-chaoscontrol-conformance-receipt-v1";
pub const EXPECTED_RECORD_LABEL: &str = "molten-chaoscontrol-expected-v1";
pub const BUNDLE_RECORD_LABEL: &str = "molten-chaoscontrol-bundle-v1";
pub const MAX_EXPECTED_BYTES: u64 = 1_048_576;
pub const MAX_BUNDLE_BYTES: u64 = 16 * 1_048_576;
pub const MAX_OPERATIONS: u64 = 4_096;
pub const MAX_OBSERVATIONS: u64 = 16_384;
pub const MAX_PARTICIPANTS: u64 = 64;
pub const MAX_COMMAND_BYTES: u64 = 65_536;
pub const MAX_EFFECTS: u64 = 256;
pub const MAX_RECEIPT_BYTES: u64 = MAX_BUNDLE_BYTES + 2 * MAX_EXPECTED_BYTES;

pub const NON_CLAIMS: [&str; 13] = [
    "producer-attestation",
    "guest-execution",
    "snapshot-backed-replay",
    "policy",
    "authority",
    "resources",
    "durability",
    "deployment",
    "release-eligibility",
    "universal-consensus-correctness",
    "application-validity",
    "physical-fault-execution",
    "unexecuted-vm-qualification",
];

/// Independently provisioned operator policy, never inferred from the candidate bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCohort {
    pub schema: String,
    pub producer_revision: String,
    pub workload: chaoscontrol_smr::SmrWorkloadProfile,
    pub protocol: chaoscontrol_smr::observation::ProtocolProfile,
    pub group_ref: String,
    pub boundary_ref: String,
    pub scheduler_ref: String,
    pub molten_source_ref: String,
    pub molten_build_ref: String,
    pub dependency_closure_ref: String,
    pub adapter_ref: String,
    pub observer_ref: String,
    pub sources: Vec<chaoscontrol_smr::observation::guest::GuestSource>,
    pub evidence_class: chaoscontrol_smr::observation::guest::EvidenceClass,
    pub claim: String,
    pub operations: Vec<ExpectedOperation>,
    pub faults: Vec<ExpectedFault>,
}

/// Canonical command bytes and independently expected application state for one logical operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOperation {
    pub client_session: String,
    pub sequence: u64,
    pub operation_ref: String,
    pub command_index: u64,
    pub command_ref: String,
    pub command_bytes: Vec<u8>,
    pub application_state_ref: String,
    pub boundary_ref: String,
    pub transition_class_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedFault {
    pub action_id: String,
    pub class: chaoscontrol_smr::FaultClass,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoltenIdentity {
    pub source_ref: String,
    pub build_ref: String,
    pub dependency_closure_ref: String,
    pub adapter_ref: String,
    pub observer_ref: String,
}

/// A retained observer effect, not proof that a physical fault executed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEffect {
    pub action_id: String,
    pub class: chaoscontrol_smr::FaultClass,
    pub participant_ref: String,
    pub source_sequence: u64,
    pub boundary_ref: String,
    pub scheduler_ref: String,
    pub before_ref: String,
    pub after_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayEvidence {
    pub original: chaoscontrol_smr::SemanticRun,
    pub repeat: chaoscontrol_smr::SemanticRun,
    pub repeat_observations: Vec<super::wire::BoundObservation>,
    pub comparison: chaoscontrol_smr::ReplayComparison,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub schema: String,
    pub producer_revision: String,
    pub evidence_class: chaoscontrol_smr::observation::guest::EvidenceClass,
    pub claim: String,
    pub molten: MoltenIdentity,
    pub sources: Vec<chaoscontrol_smr::observation::guest::GuestSource>,
    pub execution_ref: String,
    pub observations: Vec<super::wire::BoundObservation>,
    pub drains: Vec<chaoscontrol_smr::observation::Drain>,
    pub source_ref: String,
    pub cohort: chaoscontrol_smr::observation::CohortResult,
    pub history: chaoscontrol_smr::HistoryReport,
    pub history_ref: String,
    pub observations_ref: String,
    pub proposals: Vec<chaoscontrol_smr::ProposalAttempt>,
    pub stabilization: Option<chaoscontrol_smr::StabilizationFacts>,
    pub liveness: chaoscontrol_smr::LivenessReport,
    pub terminal_status: chaoscontrol_smr::TerminalStatus,
    pub faults: Vec<chaoscontrol_smr::FaultOutcome>,
    pub fault_effects: Vec<FaultEffect>,
    pub replay: Option<ReplayEvidence>,
    pub oracle: chaoscontrol_smr::observation::OracleResult,
    pub receipt: chaoscontrol_smr::observation::ProtocolReceipt,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Imported {
    pub receipt_ref: String,
    pub value: preserves::IOValue,
    pub status: String,
    pub evidence_class: String,
}

/// Bounded classifications; no untrusted parser/provider diagnostic is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportIssue {
    Bounds,
    Decode,
    Policy,
    Identity,
    Schema,
    Claim,
    Source,
    Observation,
    History,
    Liveness,
    Fault,
    Replay,
    Oracle,
    Receipt,
}

impl std::fmt::Display for ImportIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bounds => "bounds",
            Self::Decode => "decode",
            Self::Policy => "policy",
            Self::Identity => "identity",
            Self::Schema => "schema",
            Self::Claim => "claim",
            Self::Source => "source",
            Self::Observation => "observation",
            Self::History => "history",
            Self::Liveness => "liveness",
            Self::Fault => "fault",
            Self::Replay => "replay",
            Self::Oracle => "oracle",
            Self::Receipt => "receipt",
        })
    }
}
impl std::error::Error for ImportIssue {}
