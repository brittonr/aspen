type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Result<T> = crate::error::Result<T>;

const MULTINODE_SCENARIO_FIXTURE_SCHEMA: &str = "molten.testing.multinode.scenario-fixture.v1";
const MULTINODE_SCENARIO_METADATA_SCHEMA: &str = "molten.testing.multinode.scenario-metadata.v1";
const MULTINODE_TOPOLOGY_PROFILE_SCHEMA: &str = "molten.testing.multinode.topology-profile.v1";
const MULTINODE_TOPOLOGY_MATRIX_SCHEMA: &str = "molten.testing.multinode.topology-profile-matrix.v1";
const MULTINODE_TOPOLOGY_MEMBERSHIP_GATE_SCHEMA: &str = "molten.testing.multinode.topology-membership-gate.v1";
const MULTINODE_RECONCILIATION_SCHEMA: &str = "molten.testing.multinode.reconciliation-gate.v1";
const LOCAL_MULTIPROCESS_PLAN_SCHEMA: &str = "molten.testing.multinode.local-multiprocess-plan.v1";
const LOCAL_MULTIPROCESS_RUN_SCHEMA: &str = "molten.testing.multinode.local-multiprocess-run.v1";
const LOCAL_MULTIPROCESS_EXECUTABLE_RUN_SCHEMA: &str = "molten.testing.multinode.local-multiprocess-executable-run.v1";
const THREE_NODE_QUORUM_GATE_SCHEMA: &str = "molten.testing.multinode.three-node-quorum-gate.v1";
const VM_SCENARIO_GATE_SCHEMA: &str = "molten.testing.multinode.vm-scenario-gate.v1";
const VM_FAILURE_REPRO_EXPORT_SCHEMA: &str = "molten.testing.multinode.vm-failure-repro-export.v1";
const GENERATED_DISTRIBUTED_CASE_SCHEMA: &str = "molten.testing.distributed-simulation.generated-case.v1";
const GENERATED_DISTRIBUTED_REPRO_SCHEMA: &str = "molten.testing.distributed-simulation.generated-repro.v1";
const MULTINODE_FAILURE_REPRO_PAYLOAD_SCHEMA: &str = "molten.testing.multinode.failure-repro-payload.v1";
const MULTINODE_FAILURE_REPRO_BUNDLE_SCHEMA: &str = "molten.testing.multinode.failure-repro-bundle.v1";
const MULTINODE_FAILURE_REPRO_VERIFY_SCHEMA: &str = "molten.testing.multinode.failure-repro-verify.v1";
const MULTINODE_FAILURE_REPRO_PASS_GATE_SCHEMA: &str = "molten.testing.multinode.failure-repro-pass-gate.v1";
const LIVE_TRANSPORT_VM_GATE_SCHEMA: &str = "molten.testing.nixos-vm.live-transport-gate.v1";
const VM_FAULT_SUPPORT_MATRIX_SCHEMA: &str = "molten.testing.nixos-vm.fault-support-matrix.v1";

const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const DIAGNOSTIC_ONLY: &str = "diagnostic-only";
const NON_REPLAYABLE_VM: &str = "non-replayable-vm-observation";
const SUPPORTED: &str = "supported";
const UNAVAILABLE: &str = "unavailable";
const PROFILE_PAIRWISE_TRANSPORT: &str = "pairwise-transport";
const PROFILE_CONTROL_QUORUM: &str = "control-quorum";
const PROFILE_RESTART_REJOIN: &str = "restart-rejoin";
const PROFILE_SUBSCRIBER_PEER: &str = "subscriber-peer";
const PROFILE_THREE_NODE_QUORUM: &str = "three-node-quorum";
const PROFILE_WRONG_TOPOLOGY: &str = "wrong-topology-negative";
const ROLE_SENDER: &str = "sender";
const ROLE_RECEIVER: &str = "receiver";
const ROLE_VOTER: &str = "voter";
const ROLE_SUBSCRIBER: &str = "subscriber";
const ROLE_TRANSPORT_ONLY: &str = "transport-only";
const ROLE_RESTARTING_MEMBER: &str = "restarting-member";
const MEMBERSHIP_VOTER: &str = "voter";
const MEMBERSHIP_SUBSCRIBER: &str = "subscriber";
const MEMBERSHIP_TRANSPORT_ONLY: &str = "transport-only";
const CLEANUP_POLICY_REQUIRED: &str = "cleanup-required";
const TICKET_STATUS_CURRENT: &str = "current";
const MAX_MULTINODE_ITEMS: usize = 512;
const REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT: usize = 6;

const _: () = assert!(MAX_MULTINODE_ITEMS > 0);
const _: () = assert!(REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioFixture {
    pub scenario_id: String,
    pub purpose: String,
    pub evidence_scope: String,
    pub topology_profile_id: String,
    pub execution_profile_id: String,
    pub command_surface: String,
    pub expected_artifact_kinds: Vec<String>,
    pub topology_ref: String,
    pub seed_ref: String,
    pub fault_plan_ref: String,
    pub receipt_refs: Vec<String>,
    pub variance_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
    pub unavailable_policy: String,
    pub unsupported_claims_pass: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioMetadata {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub fixture_ref: String,
    pub topology_profile_ref: String,
    pub metadata_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyRole {
    pub node_id: String,
    pub role: String,
    pub membership: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyLink {
    pub from: String,
    pub to: String,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyProfile {
    pub id: String,
    pub roles: Vec<TopologyRole>,
    pub allowed_links: Vec<TopologyLink>,
    pub evidence_scope: String,
    pub required_receipt_kinds: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMatrix {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub profile_refs: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMembershipClaim {
    pub profile_id: String,
    pub topology_ref: String,
    pub scenario_topology_ref: String,
    pub node_roles: Vec<TopologyRole>,
    pub quorum_ref: Option<String>,
    pub transport_only_authority_claim: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMembershipGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCommitEvidence {
    pub operation_id: String,
    pub commit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    pub node_id: String,
    pub topology_ref: String,
    pub scenario_fixture_ref: String,
    pub receipt_refs: Vec<String>,
    pub queue_ref: String,
    pub ledger_ref: String,
    pub dispatch_ref: String,
    pub ack_ref: String,
    pub protocol_ref: String,
    pub semantic_commits: Vec<SemanticCommitEvidence>,
    pub diagnostic_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationEqualityClass {
    pub name: String,
    pub refs: Vec<String>,
    pub variance_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationInput {
    pub topology_ref: String,
    pub scenario_fixture_ref: String,
    pub required_receipt_refs: Vec<String>,
    pub node_summaries: Vec<NodeSummary>,
    pub equality_classes: Vec<ReconciliationEqualityClass>,
    pub allowed_variance_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalProcessNodePlan {
    pub node_id: String,
    pub state_root_handle: String,
    pub transport_handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessPlanInput {
    pub fixture_ref: String,
    pub nodes: Vec<LocalProcessNodePlan>,
    pub command_plan_ref: String,
    pub expected_receipt_refs: Vec<String>,
    pub cleanup_policy: String,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessPlan {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub plan_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessRunInput {
    pub plan_ref: String,
    pub startup_refs: Vec<String>,
    pub workflow_refs: Vec<String>,
    pub shutdown_refs: Vec<String>,
    pub cleanup_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessExecutableRunInput {
    pub plan: LocalMultiprocessPlanInput,
    pub startup_refs: Vec<String>,
    pub workflow_refs: Vec<String>,
    pub shutdown_refs: Vec<String>,
    pub cleanup_refs: Vec<String>,
    pub ticket_status: String,
    pub child_timed_out: bool,
    pub orphaned_processes: Vec<String>,
    pub cleanup_succeeded: bool,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessExecutableRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub plan_ref: String,
    pub run_ref: String,
    pub executable_ref: String,
    pub value: IoValue,
}

