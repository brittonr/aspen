pub const FABRIC_SIMULATION_EXECUTION_DIFFERENTIAL_SCHEMA: &str =
    "molten.fabric-simulation.execution-differential.v1";
pub const FABRIC_SIMULATION_DIFFERENTIAL_EVIDENCE_SCHEMA: &str =
    "molten.fabric-simulation.differential-evidence.v1";
pub const EXECUTION_DIFFERENTIAL_NON_CLAIMS: [&str; 4] = [
    "agreement-does-not-prove-live-equivalence",
    "differential-does-not-prove-performance",
    "incomplete-is-not-agreement",
    "differential-does-not-grant-release-eligibility",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionDifferential {
    pub report_ref: String,
    pub result_kind: &'static str,
    pub workload_ref: String,
    pub simulation_side_ref: String,
    pub live_side_ref: String,
    pub decision: LiveDifferentialDecision,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferentialEvidence {
    pub evidence_ref: String,
    pub structural_ref: String,
    pub execution_ref: String,
    pub workload_ref: String,
    pub value: preserves::IOValue,
}

pub struct ExecutionDifferentialInput<'a> {
    pub workload: &'a LiveWorkload,
    pub simulation_side_ref: &'a str,
    pub live_side_ref: &'a str,
    pub simulated: &'a [Option<OutcomeClass>],
    pub live: Option<&'a [Option<OutcomeClass>]>,
}

pub struct DifferentialEvidenceInput<'a> {
    pub structural: &'a Differential,
    pub execution: &'a ExecutionDifferential,
}
