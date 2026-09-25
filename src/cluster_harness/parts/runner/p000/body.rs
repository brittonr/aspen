use super::canonical::*;

pub const DEFAULT_CLUSTER_CHILD_TIMEOUT_MS: u64 = 30_000;
pub const MAX_CLUSTER_CHILD_TIMEOUT_MS: u64 = 300_000;
const CHILD_POLL_INTERVAL_MS: u64 = 10;
const MAX_TICKET_FILES: usize = 1_024;
pub(super) const RUN_INDEX_HEADER: &str = "molten.cluster-run-index.v1";
const RUN_INDEX_FIELD_COUNT: usize = 4;
const RUN_INDEX_REF_FIELD: usize = 2;
const RUN_INDEX_FORMAT_FIELD: usize = 3;
const RUN_INDEX_ENTRY_LINE_OFFSET: usize = 2;
// One past molten-core's run-artifact cap, so an oversized index still reaches the core
// too-many-artifacts diagnostic.
pub(super) const MAX_RUN_INDEX_ENTRIES: usize = molten_core::cluster_harness::MAX_RUN_ARTIFACTS + 1;
const RUN_INDEX_FILE: &str = "artifact-index.tsv";
const VERIFICATION_FILE: &str = "verification.preserves";
const FAILURE_BUNDLE_FILE: &str = "failure-repro-bundle.preserves";
const FAILURE_BUNDLE_VERIFICATION_FILE: &str = "failure-repro-verification.preserves";
const FIXTURE_METADATA_FILE: &str = "fixture-metadata.preserves";
const COMMAND_PLAN_FILE: &str = "command-plan.preserves";
const LOCAL_PLAN_FILE: &str = "derived-plan.preserves";
const LOCAL_EXECUTABLE_RUN_FILE: &str = "local-executable-run.preserves";
const LIFECYCLE_FILE: &str = "cluster-lifecycle-receipt.preserves";
const DRIFT_SUMMARY_FILE: &str = "drift-summary.preserves";
const CLEANUP_FILE: &str = "cleanup-receipt.preserves";
const PARENT_RUN_FILE: &str = "cluster-run-receipt.preserves";
const TEXT_ARTIFACT_DOMAIN: &str = "molten.testing.cluster-harness-text-artifact.v1";
const FIXTURE_DOMAIN: &str = "molten.testing.cluster-harness-fixture.v1";
const COMMAND_PROFILE_DOMAIN: &str = "molten.testing.cluster-harness-command-profile.v1";
const EXPECTED_ARTIFACT_DOMAIN: &str = "molten.testing.cluster-harness-expected-artifact.v1";
const CLEANUP_POLICY: &str = "cleanup-required";
const TICKET_STATUS_CURRENT: &str = "current";
const WORKFLOW_ID: &str = "receipt-first-cluster-harness";
const WORKFLOW_MAX_REQUESTS: &str = "1";

#[derive(Debug, Clone)]
pub struct ClusterHarnessExecutionInput {
    pub fixture_path: std::path::PathBuf,
    pub state_root: std::path::PathBuf,
    pub output_directory: std::path::PathBuf,
    pub node_binary: std::path::PathBuf,
    pub child_timeout_ms: u64,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct ClusterHarnessExecution {
    pub decision: String,
    pub parent_ref: String,
    pub verification_ref: String,
    pub failure_bundle_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub output_directory: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct ClusterRunDirectoryVerification {
    pub decision: String,
    pub index_ref: String,
    pub receipt: ClusterRunVerificationReceipt,
}

#[derive(Debug)]
struct ChildExecution {
    node_id: String,
    phase: String,
    process_ref: String,
    value: IoValue,
    succeeded: bool,
    timed_out: bool,
    orphaned: bool,
    diagnostic: Option<String>,
}

#[derive(Debug, Default)]
struct NodeArtifacts {
    config_ref: Option<String>,
    identity_ref: Option<String>,
    startup_ref: Option<String>,
    workflow_ref: Option<String>,
    heartbeat_ref: Option<String>,
    health_ref: Option<String>,
    control_ref: Option<String>,
    shutdown_ref: Option<String>,
    stop_control_ref: Option<String>,
}

#[derive(Debug)]
struct PreparedArtifact {
    entry: molten_core::cluster_harness::RunArtifactIndexEntry,
    value: IoValue,
}

// r[impl molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
// r[impl molten.testing.receipt_first_cluster_harness.fixture_executable_runner]
// r[impl molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
// r[impl molten.testing.local_multiprocess_cluster_tier.middle_tier]
pub fn execute_cluster_harness(input: &ClusterHarnessExecutionInput) -> crate::error::Result<ClusterHarnessExecution> {
    validate_execution_input(input)?;
    prepare_output_roots(input)?;
    let planned = plan_run(input)?;
    let evidence = record_run_evidence(input, planned)?;
    finish_run(input, evidence)
}

/// The cluster plan, fixture identity, and plan artifacts one harness run executes against.
struct PlannedRun {
    plan: crate::cluster::ClusterPlan,
    node_ids: Vec<String>,
    fixture_ref: String,
    caveats: Vec<String>,
    expected_kinds: Vec<String>,
    command_plan_ref: String,
    local_plan_input: crate::multinode_core::LocalMultiprocessPlanInput,
    artifacts: Vec<PreparedArtifact>,
}

/// Reads and plans the fixture, and prepares the fixture metadata, command plan, and local plan
/// artifacts.
fn plan_run(input: &ClusterHarnessExecutionInput) -> crate::error::Result<PlannedRun> {
    let fixture_source = std::fs::read_to_string(&input.fixture_path).map_err(crate::error::MoltenError::from)?;
    let node_names = crate::cluster::parse_cluster_manifest(&fixture_source)?;
    let plan = crate::cluster::plan_cluster(&input.state_root, &node_names)?;
    let node_ids = plan.nodes.iter().map(|node| node.node_id.clone()).collect::<Vec<_>>();
    let fixture_ref = content_ref_for_text(FIXTURE_DOMAIN, &fixture_source);
    let caveats = cluster_harness_caveats();
    let expected_kinds = expected_artifact_kinds();
    let fixture_value = fixture_metadata_value(&fixture_ref, &node_ids, &caveats)?;
    let command_plan = command_plan_value(&fixture_ref, &node_ids, input.child_timeout_ms, &expected_kinds)?;
    let command_plan_ref = crate::preserves_rail::canonical_hash(&command_plan)?;
    let local_plan_input = local_plan_input(&plan, &fixture_ref, &command_plan_ref, &expected_kinds, &caveats);
    let local_plan = crate::multinode_core::build_local_multiprocess_plan(&local_plan_input)?;

    let mut artifacts = Vec::new();
    push_artifact(&mut artifacts, FIXTURE_METADATA_FILE, FIXTURE_METADATA_KIND, fixture_value)?;
    push_artifact(&mut artifacts, COMMAND_PLAN_FILE, COMMAND_PLAN_KIND, command_plan)?;
    push_artifact(&mut artifacts, LOCAL_PLAN_FILE, LOCAL_PLAN_KIND, local_plan.value.clone())?;
    Ok(PlannedRun {
        plan,
        node_ids,
        fixture_ref,
        caveats,
        expected_kinds,
        command_plan_ref,
        local_plan_input,
        artifacts,
    })
}

/// The child executions, skip diagnostics, and per-phase outcomes of the node lifecycle phases.
struct LifecyclePhases {
    child_executions: Vec<ChildExecution>,
    diagnostics: Vec<String>,
    is_init_passed: bool,
    is_start_passed: bool,
    is_workflow_passed: bool,
    is_status_passed: bool,
    is_stop_passed: bool,
}

impl LifecyclePhases {
    const fn all_passed(&self) -> bool {
        self.is_init_passed
            && self.is_start_passed
            && self.is_workflow_passed
            && self.is_status_passed
            && self.is_stop_passed
    }
}

/// Runs init, start, workflow, and status on every node in order, each only after the previous
/// phase passed, then stops the started nodes in reverse order.
fn run_lifecycle_phases(
    input: &ClusterHarnessExecutionInput,
    plan: &crate::cluster::ClusterPlan,
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
) -> crate::error::Result<LifecyclePhases> {
    let step = |phase| PhaseStep { input, plan, phase };
    let mut child_executions = Vec::new();
    let mut diagnostics = Vec::new();
    let is_init_passed = execute_phase_for_nodes(step("init"), &mut child_executions, artifacts, init_node_args)?;
    let is_start_passed = if is_init_passed {
        execute_phase_for_nodes(step("start"), &mut child_executions, artifacts, start_node_args)?
    } else {
        diagnostics.push("cluster-harness-start-skipped-after-init-failure".to_string());
        false
    };
    let is_workflow_passed = if is_start_passed {
        execute_phase_for_nodes(step("workflow"), &mut child_executions, artifacts, workflow_node_args)?
    } else {
        diagnostics.push("cluster-harness-workflow-skipped-after-start-failure".to_string());
        false
    };
    let is_status_passed = if is_workflow_passed {
        execute_phase_for_nodes(step("status"), &mut child_executions, artifacts, status_node_args)?
    } else {
        diagnostics.push("cluster-harness-status-skipped-after-workflow-failure".to_string());
        false
    };
    let is_stop_passed = if is_start_passed {
        execute_phase_for_nodes_reverse(step("stop"), &mut child_executions, artifacts, stop_node_args)?
    } else {
        true
    };
    Ok(LifecyclePhases {
        child_executions,
        diagnostics,
        is_init_passed,
        is_start_passed,
        is_workflow_passed,
        is_status_passed,
        is_stop_passed,
    })
}

fn init_node_args(node: &crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("node"),
        std::ffi::OsString::from("init"),
        std::ffi::OsString::from("--state-root"),
        node.state_root.as_os_str().to_os_string(),
        std::ffi::OsString::from("--node-id"),
        std::ffi::OsString::from(&node.node_id),
    ]
}

fn start_node_args(node: &crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("node"),
        std::ffi::OsString::from("run"),
        std::ffi::OsString::from("--state-root"),
        node.state_root.as_os_str().to_os_string(),
    ]
}

fn workflow_node_args(node: &crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("node"),
        std::ffi::OsString::from("run-loop"),
        std::ffi::OsString::from("--state-root"),
        node.state_root.as_os_str().to_os_string(),
        std::ffi::OsString::from("--max-requests"),
        std::ffi::OsString::from(WORKFLOW_MAX_REQUESTS),
        std::ffi::OsString::from("--receipt-out"),
        node.state_root.join("cluster-harness-workflow.preserves").into_os_string(),
        std::ffi::OsString::from("--heartbeat-out"),
        node.state_root.join("cluster-harness-heartbeat.preserves").into_os_string(),
    ]
}

fn status_node_args(node: &crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("node"),
        std::ffi::OsString::from("status"),
        std::ffi::OsString::from("--state-root"),
        node.state_root.as_os_str().to_os_string(),
    ]
}

fn stop_node_args(node: &crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("node"),
        std::ffi::OsString::from("stop"),
        std::ffi::OsString::from("--state-root"),
        node.state_root.as_os_str().to_os_string(),
    ]
}

/// Everything the parent receipt, run index, and failure bundle are built from once the nodes have
/// run.
struct RunEvidence {
    plan: PlannedRun,
    child_executions: Vec<ChildExecution>,
    diagnostics: Vec<String>,
    child_process_refs: Vec<String>,
    child_receipt_refs: Vec<String>,
    cleanup_ref: String,
    lifecycle_ref: String,
    drift_ref: String,
    local_plan_ref: String,
    local_run_ref: String,
}
