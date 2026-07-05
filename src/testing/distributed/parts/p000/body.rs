type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const DISTRIBUTED_TOPOLOGY_SCHEMA: &str = "molten.testing.distributed-simulation.topology.v1";
const DISTRIBUTED_SCHEDULER_SCHEMA: &str = "molten.testing.distributed-simulation.scheduler-profile.v1";
const DISTRIBUTED_SEED_SCHEMA: &str = "molten.testing.distributed-simulation.seed.v1";
const DISTRIBUTED_FAULT_PLAN_SCHEMA: &str = "molten.testing.distributed-simulation.fault-plan.v1";
const DISTRIBUTED_EVENT_SCHEMA: &str = "molten.testing.distributed-simulation.event.v1";
const DISTRIBUTED_FINAL_STATE_SCHEMA: &str = "molten.testing.distributed-simulation.final-state.v1";
const DISTRIBUTED_RUN_SCHEMA: &str = "molten.testing.distributed-test-run.v1";
const DISTRIBUTED_CI_MATRIX_SCHEMA: &str = "molten.testing.distributed-ci.matrix.v1";
const DISTRIBUTED_TEST_METADATA_SCHEMA: &str = "molten.testing.distributed-ci.metadata.v1";
const DISTRIBUTED_CI_GATE_SCHEMA: &str = "molten.testing.distributed-ci.gate.v1";
const COMPOSITE_FAULT_CASE_SCHEMA: &str = "molten.testing.distributed-simulation.composite-fault-case.v1";
const COMPOSITE_FAULT_SUITE_SCHEMA: &str = "molten.testing.distributed-simulation.composite-fault-suite.v1";
const GENERATED_CASE_PROMOTION_SCHEMA: &str = "molten.testing.distributed-simulation.generated-case-promotion.v1";

const MAX_DISTRIBUTED_PEERS: usize = 128;
const MAX_DISTRIBUTED_CHANNELS: usize = 512;
const MAX_DISTRIBUTED_ROLES: usize = 32;
const MAX_DISTRIBUTED_FAULTS: usize = 512;
const MAX_DISTRIBUTED_COMMANDS: usize = 1024;
const MAX_DISTRIBUTED_REFS: usize = 1024;
const MAX_DISTRIBUTED_TEXT: usize = 256;
const MAX_DISTRIBUTED_PROFILES: usize = 32;
const REQUIRED_DISTRIBUTED_PROFILE_COUNT: usize = 6;
const DISTRIBUTED_RUN_ARITY: usize = 15;
const DEFAULT_REPLAY_STATUS: &str = "deterministic-simulation-replayable";
const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const DUPLICATE_SUPPRESSED_DECISION: &str = "duplicate-suppressed";
const COMMIT_EVENT_KIND: &str = "semantic-commit";
const DENY_EVENT_KIND: &str = "deny-before-side-effects";
const REPLAY_EVENT_KIND: &str = "restart-replay-stable";
const DUPLICATE_EVENT_KIND: &str = "duplicate-operation-suppressed";
const FAULT_DELAY: &str = "delay";
const FAULT_DROP: &str = "drop";
const FAULT_DUPLICATE: &str = "duplicate";
const FAULT_REORDER: &str = "reorder";
const FAULT_PARTITION: &str = "partition";
const FAULT_REJOIN: &str = "rejoin";
const FAULT_CRASH: &str = "crash";
const FAULT_RESTART: &str = "restart";
const FAULT_RESOURCE_PRESSURE: &str = "resource-pressure";
const FAULT_STALE_EVIDENCE: &str = "stale-evidence";
const FAULT_AMBIENT_STATE_DRIFT: &str = "ambient-state-drift";
const FAULT_CORRUPTED_RECEIPT: &str = "corrupted-receipt";
const FAULT_UNAUTHORIZED_TRANSPORT: &str = "unauthorized-transport";
const PROFILE_FAST: &str = "fast";
const PROFILE_PROTOCOL: &str = "protocol";
const PROFILE_CLI: &str = "cli";
const PROFILE_VM_SMOKE: &str = "vm-smoke";
const PROFILE_VM_FAULT: &str = "vm-fault";
const PROFILE_SOAK: &str = "soak";
const RELEASE_REQUIRED: &str = "required";
const RELEASE_REQUIRED_WHEN_SUPPORTED: &str = "required-when-supported";
const RELEASE_PILOT_SCOPE: &str = "pilot-scope";
const COST_FAST: &str = "fast";
const COST_MEDIUM: &str = "medium";
const COST_HEAVY: &str = "heavy";
const METADATA_REQUIRED_FIELDS: usize = 15;

const _: () = assert!(MAX_DISTRIBUTED_PEERS > 0);
const _: () = assert!(MAX_DISTRIBUTED_CHANNELS >= MAX_DISTRIBUTED_PEERS);
const _: () = assert!(MAX_DISTRIBUTED_FAULTS > 0);
const _: () = assert!(MAX_DISTRIBUTED_COMMANDS > 0);
const _: () = assert!(MAX_DISTRIBUTED_REFS >= MAX_DISTRIBUTED_COMMANDS);
const _: () = assert!(MAX_DISTRIBUTED_TEXT > 0);
const _: () = assert!(MAX_DISTRIBUTED_PROFILES >= REQUIRED_DISTRIBUTED_PROFILE_COUNT);
const _: () = assert!(METADATA_REQUIRED_FIELDS >= REQUIRED_DISTRIBUTED_PROFILE_COUNT);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub from_peer: String,
    pub to_peer: String,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topology {
    pub peers: Vec<Peer>,
    pub channels: Vec<Channel>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProfile {
    pub id: String,
    pub policy: String,
    pub max_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationSeed {
    pub id: String,
    pub entropy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEvent {
    pub kind: String,
    pub target_kind: String,
    pub target: String,
    pub operation_id: Option<String>,
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultPlan {
    pub events: Vec<FaultEvent>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationCommand {
    pub operation_id: String,
    pub from_peer: String,
    pub to_peer: String,
    pub payload_ref: String,
    pub commit_ref: String,
    pub authority_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub resource_ref: Option<String>,
    pub transport_ref: Option<String>,
    pub requires_authority: bool,
    pub requires_quorum: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationInput {
    pub topology: Topology,
    pub scheduler: SchedulerProfile,
    pub seed: SimulationSeed,
    pub fault_plan: FaultPlan,
    pub source_ref: String,
    pub test_binary_ref: String,
    pub commands: Vec<SimulationCommand>,
    pub child_workflow_refs: Vec<String>,
    pub allowed_variance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationEventOutcome {
    pub tick: u64,
    pub operation_id: String,
    pub kind: String,
    pub decision: String,
    pub diagnostic: String,
    pub event_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationRun {
    pub decision: String,
    pub topology_ref: String,
    pub seed_ref: String,
    pub scheduler_ref: String,
    pub fault_plan_ref: String,
    pub source_ref: String,
    pub test_binary_ref: String,
    pub child_workflow_refs: Vec<String>,
    pub event_refs: Vec<String>,
    pub event_outcomes: Vec<SimulationEventOutcome>,
    pub committed_operation_ids: Vec<String>,
    pub denied_operation_ids: Vec<String>,
    pub final_state_ref: String,
    pub replay_status: String,
    pub allowed_variance_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTestRun {
    pub decision: String,
    pub topology_ref: String,
    pub seed_ref: String,
    pub scheduler_ref: String,
    pub fault_plan_ref: String,
    pub event_refs: Vec<String>,
    pub final_state_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiProfile {
    pub id: String,
    pub purpose: String,
    pub command: String,
    pub expected_artifact_kinds: Vec<String>,
    pub evidence_scope: String,
    pub cost_class: String,
    pub release_review_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiMatrix {
    pub decision: String,
    pub profiles: Vec<CiProfile>,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMetadataInput {
    pub source_ref: String,
    pub nix_input_refs: Vec<String>,
    pub test_binary_ref: String,
    pub profile_id: String,
    pub command: String,
    pub expected_artifact_kinds: Vec<String>,
    pub cost_class: String,
    pub release_review_status: String,
    pub shard_id: String,
    pub seed_ref: String,
    pub topology_ref: String,
    pub fault_plan_ref: String,
    pub receipt_refs: Vec<String>,
    pub variance_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMetadata {
    pub profile_id: String,
    pub command: String,
    pub expected_artifact_kinds: Vec<String>,
    pub cost_class: String,
    pub release_review_status: String,
    pub shard_id: String,
    pub metadata_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRun {
    pub profile_id: String,
    pub decision: String,
    pub metadata_ref: String,
    pub traceability_ref: String,
    pub positive_coverage: bool,
    pub negative_coverage: bool,
    pub retry_attempts: u64,
    pub unavailable: bool,
    pub unsupported_reason: Option<String>,
    pub variance_declared: bool,
    pub required_for_release: bool,
}

