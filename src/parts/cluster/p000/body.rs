type IoValue = preserves::IOValue;

pub const CLUSTER_MANIFEST_FILE: &str = "cluster.nodes";
const CLUSTER_MANIFEST_HEADER: &str = "molten.cluster.nodes.v1";
const CLUSTER_LIFECYCLE_RUN_SCHEMA: &str = "molten.testing.cluster-lifecycle-run.v1";
const NODE_ID_PREFIX: &str = "node:";
const CURRENT_DIR_COMPONENT: &str = ".";
const PARENT_DIR_COMPONENT: &str = "..";
const NODE_PATH_DASH: char = '-';
const NODE_PATH_UNDERSCORE: char = '_';
const NODE_ID_SEPARATOR: char = ':';
const CLUSTER_LIFECYCLE_PASS: &str = "pass";
const CLUSTER_LIFECYCLE_DENY: &str = "deny";
const CLUSTER_LIFECYCLE_ALREADY_RUNNING: &str = "already-running";
const CLUSTER_LIFECYCLE_SKIPPED: &str = "skipped";
const CLUSTER_LIFECYCLE_UNAVAILABLE: &str = "unavailable";
const CLUSTER_LIFECYCLE_PHASE_INIT: &str = "init";
const CLUSTER_LIFECYCLE_PHASE_START: &str = "start";
const CLUSTER_LIFECYCLE_PHASE_STATUS: &str = "status";
const CLUSTER_LIFECYCLE_PHASE_STOP: &str = "stop";
const CLUSTER_LIFECYCLE_NONE: &str = "none";
const CLUSTER_LIFECYCLE_STOP_SEPARATOR: &str = ">";
const MAX_CLUSTER_LIFECYCLE_ITEMS: usize = 512;
const _: () = assert!(MAX_CLUSTER_LIFECYCLE_ITEMS > 0);
// Lifecycle receipts encode at most this many node summaries; larger manifests cannot run.
const MAX_CLUSTER_MANIFEST_NODES: usize = MAX_CLUSTER_LIFECYCLE_ITEMS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterNodePlan {
    pub requested_node: String,
    pub node_id: String,
    pub path_component: String,
    pub state_root: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPlan {
    pub state_root: std::path::PathBuf,
    pub nodes: Vec<ClusterNodePlan>,
}

pub fn cluster_manifest_path(state_root: &std::path::Path) -> std::path::PathBuf {
    state_root.join(CLUSTER_MANIFEST_FILE)
}

pub fn plan_cluster(state_root: &std::path::Path, node_names: &[String]) -> crate::error::Result<ClusterPlan> {
    validate_cluster_state_root(state_root)?;
    if node_names.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster requires at least one --node"));
    }
    let mut seen_node_ids = std::collections::BTreeSet::new();
    let mut nodes = Vec::with_capacity(node_names.len());
    for node_name in node_names {
        let node = plan_node(state_root, node_name)?;
        if !seen_node_ids.insert(node.node_id.clone()) {
            return Err(crate::error::MoltenError::invalid_harness(format!("duplicate cluster node {}", node.node_id)));
        }
        nodes.push(node);
    }
    Ok(ClusterPlan {
        state_root: state_root.to_path_buf(),
        nodes,
    })
}

pub fn render_cluster_manifest(plan: &ClusterPlan) -> String {
    let mut rendered = String::from(CLUSTER_MANIFEST_HEADER);
    rendered.push('\n');
    for node in &plan.nodes {
        rendered.push_str(&node.node_id);
        rendered.push('\n');
    }
    rendered
}

pub fn parse_cluster_manifest(source: &str) -> crate::error::Result<Vec<String>> {
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("cluster manifest is empty"))?;
    if header != CLUSTER_MANIFEST_HEADER {
        return Err(crate::error::MoltenError::invalid_harness("cluster manifest has unsupported header"));
    }
    let mut nodes = Vec::new();
    for line in lines {
        if !line.is_empty() {
            crate::bounded::push_bounded(
                &mut nodes,
                line.to_string(),
                MAX_CLUSTER_MANIFEST_NODES,
                "cluster manifest node",
            )?;
        }
    }
    if nodes.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster manifest has no nodes"));
    }
    Ok(nodes)
}

fn validate_cluster_state_root(state_root: &std::path::Path) -> crate::error::Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster requires explicit state root"));
    }
    if state_root == std::path::Path::new(CURRENT_DIR_COMPONENT)
        || state_root == std::path::Path::new(PARENT_DIR_COMPONENT)
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "cluster state root must not be ambient current or parent directory",
        ));
    }
    Ok(())
}

fn plan_node(state_root: &std::path::Path, requested_node: &str) -> crate::error::Result<ClusterNodePlan> {
    let path_component = node_path_component(requested_node)?;
    let node_id = if requested_node.starts_with(NODE_ID_PREFIX) {
        requested_node.to_string()
    } else {
        format!("{NODE_ID_PREFIX}{requested_node}")
    };
    Ok(ClusterNodePlan {
        requested_node: requested_node.to_string(),
        node_id,
        state_root: state_root.join(&path_component),
        path_component,
    })
}

fn node_path_component(requested_node: &str) -> crate::error::Result<String> {
    if requested_node.is_empty() || requested_node.trim() != requested_node {
        return Err(crate::error::MoltenError::invalid_harness("cluster node name must be non-empty and unpadded"));
    }
    let component = requested_node.strip_prefix(NODE_ID_PREFIX).unwrap_or(requested_node);
    if component.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster node path component must be non-empty"));
    }
    if component == CURRENT_DIR_COMPONENT || component == PARENT_DIR_COMPONENT {
        return Err(crate::error::MoltenError::invalid_harness(
            "cluster node path component must not be relative syntax",
        ));
    }
    if component.contains(NODE_ID_SEPARATOR) {
        return Err(crate::error::MoltenError::invalid_harness("cluster node path component must not contain ':'"));
    }
    if !component.chars().all(is_safe_node_path_character) {
        return Err(crate::error::MoltenError::invalid_harness(
            "cluster node path component must contain only ASCII letters, digits, '-' or '_'",
        ));
    }
    Ok(component.to_string())
}

fn is_safe_node_path_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == NODE_PATH_DASH || character == NODE_PATH_UNDERSCORE
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterLifecyclePhaseObservation {
    pub phase: String,
    pub decision: String,
    pub receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterLifecycleNodeSummary {
    pub node_id: String,
    pub manifest_ref: String,
    pub config_ref: String,
    pub identity_ref: Option<String>,
    pub startup_ref: Option<String>,
    pub health_ref: Option<String>,
    pub queue_ref: Option<String>,
    pub control_ref: Option<String>,
    pub heartbeat_ref: Option<String>,
    pub shutdown_ref: Option<String>,
    pub stop_control_ref: Option<String>,
    pub already_running_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterLifecycleRunInput {
    pub workflow_id: String,
    pub manifest_ref: String,
    pub ordered_node_ids: Vec<String>,
    pub phases: Vec<ClusterLifecyclePhaseObservation>,
    pub node_summaries: Vec<ClusterLifecycleNodeSummary>,
    pub already_running_refs: Vec<String>,
    pub stop_order: Vec<String>,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterLifecycleRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

// r[impl molten.testing.cluster_lifecycle_receipt.run_receipt]
// r[impl molten.testing.cluster_lifecycle_receipt.fail_closed_validation]
pub fn build_cluster_lifecycle_run_receipt(
    input: &ClusterLifecycleRunInput,
) -> crate::error::Result<ClusterLifecycleRunReceipt> {
    let mut diagnostics = cluster_lifecycle_run_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = cluster_lifecycle_decision(&diagnostics).to_string();
    let value = cluster_lifecycle_run_value(input, &decision, &diagnostics)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ClusterLifecycleRunReceipt {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

// r[impl molten.testing.cluster_lifecycle_summary_drift.receipt_summary]
// r[impl molten.testing.cluster_lifecycle_summary_drift.negatives]
pub fn cluster_lifecycle_drift_summary(
    input: &ClusterLifecycleRunInput,
) -> crate::error::Result<crate::drift_core::EvidenceSummary> {
    let mut fields = Vec::new();
    push_lifecycle_summary_field(&mut fields, "workflow-id", &input.workflow_id, false)?;
    push_lifecycle_summary_field(&mut fields, "manifest", &input.manifest_ref, true)?;
    push_lifecycle_summary_field(&mut fields, "node-order", &join_lifecycle_values(&input.ordered_node_ids), false)?;
    push_lifecycle_summary_field(&mut fields, "stop-order", &join_lifecycle_values(&input.stop_order), false)?;
    for phase in &input.phases {
        push_lifecycle_summary_field(&mut fields, &format!("phase:{}:decision", phase.phase), &phase.decision, false)?;
        push_lifecycle_ref_fields(&mut fields, &format!("phase:{}:receipt", phase.phase), &phase.receipt_refs)?;
    }
    for summary in &input.node_summaries {
        push_lifecycle_summary_field(
            &mut fields,
            &format!("node:{}:manifest", summary.node_id),
            &summary.manifest_ref,
            true,
        )?;
        push_lifecycle_summary_field(
            &mut fields,
            &format!("node:{}:config", summary.node_id),
            &summary.config_ref,
            true,
        )?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "identity", summary.identity_ref.as_deref())?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "startup", summary.startup_ref.as_deref())?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "health", summary.health_ref.as_deref())?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "queue", summary.queue_ref.as_deref())?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "control", summary.control_ref.as_deref())?;
        push_lifecycle_optional_ref_field(
            &mut fields,
            &summary.node_id,
            "heartbeat",
            summary.heartbeat_ref.as_deref(),
        )?;
        push_lifecycle_optional_ref_field(&mut fields, &summary.node_id, "shutdown", summary.shutdown_ref.as_deref())?;
        push_lifecycle_optional_ref_field(
            &mut fields,
            &summary.node_id,
            "stop-control",
            summary.stop_control_ref.as_deref(),
        )?;
        push_lifecycle_optional_ref_field(
            &mut fields,
            &summary.node_id,
            "already-running",
            summary.already_running_ref.as_deref(),
        )?;
    }
    push_lifecycle_ref_fields(&mut fields, "already-running", &input.already_running_refs)?;
    push_lifecycle_summary_field(&mut fields, "caveats", &join_lifecycle_values(&input.caveats), false)?;
    Ok(crate::drift_core::EvidenceSummary {
        workflow: input.workflow_id.clone(),
        fields,
    })
}
