use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::error::MoltenError;
use crate::error::Result;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterNodePlan {
    pub requested_node: String,
    pub node_id: String,
    pub path_component: String,
    pub state_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPlan {
    pub state_root: PathBuf,
    pub nodes: Vec<ClusterNodePlan>,
}

pub fn cluster_manifest_path(state_root: &Path) -> PathBuf {
    state_root.join(CLUSTER_MANIFEST_FILE)
}

pub fn plan_cluster(state_root: &Path, node_names: &[String]) -> Result<ClusterPlan> {
    validate_cluster_state_root(state_root)?;
    if node_names.is_empty() {
        return Err(MoltenError::invalid_harness("cluster requires at least one --node"));
    }
    let mut seen_node_ids = BTreeSet::new();
    let mut nodes = Vec::with_capacity(node_names.len());
    for node_name in node_names {
        let node = plan_node(state_root, node_name)?;
        if !seen_node_ids.insert(node.node_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate cluster node {}", node.node_id)));
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

pub fn parse_cluster_manifest(source: &str) -> Result<Vec<String>> {
    let mut lines = source.lines();
    let header = lines.next().ok_or_else(|| MoltenError::invalid_harness("cluster manifest is empty"))?;
    if header != CLUSTER_MANIFEST_HEADER {
        return Err(MoltenError::invalid_harness("cluster manifest has unsupported header"));
    }
    let mut nodes = Vec::new();
    for line in lines {
        if !line.is_empty() {
            nodes.push(line.to_string());
        }
    }
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("cluster manifest has no nodes"));
    }
    Ok(nodes)
}

fn validate_cluster_state_root(state_root: &Path) -> Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("cluster requires explicit state root"));
    }
    if state_root == Path::new(CURRENT_DIR_COMPONENT) || state_root == Path::new(PARENT_DIR_COMPONENT) {
        return Err(MoltenError::invalid_harness("cluster state root must not be ambient current or parent directory"));
    }
    Ok(())
}

fn plan_node(state_root: &Path, requested_node: &str) -> Result<ClusterNodePlan> {
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

fn node_path_component(requested_node: &str) -> Result<String> {
    if requested_node.is_empty() || requested_node.trim() != requested_node {
        return Err(MoltenError::invalid_harness("cluster node name must be non-empty and unpadded"));
    }
    let component = requested_node.strip_prefix(NODE_ID_PREFIX).unwrap_or(requested_node);
    if component.is_empty() {
        return Err(MoltenError::invalid_harness("cluster node path component must be non-empty"));
    }
    if component == CURRENT_DIR_COMPONENT || component == PARENT_DIR_COMPONENT {
        return Err(MoltenError::invalid_harness("cluster node path component must not be relative syntax"));
    }
    if component.contains(NODE_ID_SEPARATOR) {
        return Err(MoltenError::invalid_harness("cluster node path component must not contain ':'"));
    }
    if !component.chars().all(is_safe_node_path_character) {
        return Err(MoltenError::invalid_harness(
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
pub fn build_cluster_lifecycle_run_receipt(input: &ClusterLifecycleRunInput) -> Result<ClusterLifecycleRunReceipt> {
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
pub fn cluster_lifecycle_drift_summary(input: &ClusterLifecycleRunInput) -> Result<crate::drift_core::EvidenceSummary> {
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

fn cluster_lifecycle_run_diagnostics(input: &ClusterLifecycleRunInput) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics.clone();
    collect_lifecycle_text_diagnostic("workflow-id", &input.workflow_id, &mut diagnostics)?;
    collect_lifecycle_ref_diagnostic("manifest", &input.manifest_ref, &mut diagnostics)?;
    push_lifecycle_if(&mut diagnostics, input.ordered_node_ids.is_empty(), "cluster-lifecycle-missing-node-order")?;
    push_lifecycle_if(&mut diagnostics, input.phases.is_empty(), "cluster-lifecycle-missing-phases")?;
    push_lifecycle_if(&mut diagnostics, input.caveats.is_empty(), "cluster-lifecycle-missing-caveats")?;
    let ordered_nodes = collect_ordered_lifecycle_nodes(&input.ordered_node_ids, &mut diagnostics)?;
    let summary_nodes = collect_summary_node_diagnostics(input, &ordered_nodes, &mut diagnostics)?;
    for node_id in &input.ordered_node_ids {
        if !summary_nodes.contains(node_id.as_str()) {
            push_lifecycle_diagnostic(&mut diagnostics, format!("cluster-lifecycle-missing-node-summary:{node_id}"))?;
        }
    }
    for phase in &input.phases {
        collect_phase_diagnostics(phase, &mut diagnostics)?;
    }
    collect_lifecycle_ref_diagnostics("already-running", &input.already_running_refs, &mut diagnostics)?;
    collect_stop_order_diagnostics(input, &mut diagnostics)?;
    push_lifecycle_if(
        &mut diagnostics,
        !cluster_lifecycle_has_canonical_receipts(input),
        "cluster-lifecycle-stdout-only-evidence",
    )?;
    Ok(diagnostics)
}

fn collect_ordered_lifecycle_nodes(node_ids: &[String], diagnostics: &mut Vec<String>) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    for node_id in node_ids {
        collect_lifecycle_text_diagnostic("node-id", node_id, diagnostics)?;
        if !seen.insert(node_id.clone()) {
            push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-duplicate-node-order:{node_id}"))?;
        }
    }
    Ok(seen)
}

fn collect_summary_node_diagnostics(
    input: &ClusterLifecycleRunInput,
    ordered_nodes: &BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    let has_init = cluster_lifecycle_has_phase(input, CLUSTER_LIFECYCLE_PHASE_INIT);
    let has_start = cluster_lifecycle_has_phase(input, CLUSTER_LIFECYCLE_PHASE_START);
    let has_status = cluster_lifecycle_has_phase(input, CLUSTER_LIFECYCLE_PHASE_STATUS);
    let has_stop = cluster_lifecycle_has_phase(input, CLUSTER_LIFECYCLE_PHASE_STOP);
    for summary in &input.node_summaries {
        collect_lifecycle_text_diagnostic("node-summary-id", &summary.node_id, diagnostics)?;
        if !seen.insert(summary.node_id.clone()) {
            push_lifecycle_diagnostic(
                diagnostics,
                format!("cluster-lifecycle-duplicate-node-summary:{}", summary.node_id),
            )?;
        }
        if !ordered_nodes.contains(summary.node_id.as_str()) {
            push_lifecycle_diagnostic(
                diagnostics,
                format!("cluster-lifecycle-unordered-node-summary:{}", summary.node_id),
            )?;
        }
        if summary.manifest_ref != input.manifest_ref {
            push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-stale-manifest:{}", summary.node_id))?;
        }
        collect_lifecycle_ref_diagnostic("node-manifest", &summary.manifest_ref, diagnostics)?;
        collect_lifecycle_ref_diagnostic("node-config", &summary.config_ref, diagnostics)?;
        collect_summary_optional_ref_diagnostics(summary, diagnostics)?;
        if has_init {
            collect_required_optional_summary_ref(summary, "identity", summary.identity_ref.as_deref(), diagnostics)?;
        }
        if has_start {
            collect_required_optional_summary_ref(summary, "startup", summary.startup_ref.as_deref(), diagnostics)?;
            collect_required_optional_summary_ref(summary, "heartbeat", summary.heartbeat_ref.as_deref(), diagnostics)?;
        }
        if has_status {
            collect_required_optional_summary_ref(summary, "health", summary.health_ref.as_deref(), diagnostics)?;
            collect_required_optional_summary_ref(summary, "control", summary.control_ref.as_deref(), diagnostics)?;
        }
        if has_stop {
            collect_required_optional_summary_ref(summary, "shutdown", summary.shutdown_ref.as_deref(), diagnostics)?;
            collect_required_optional_summary_ref(
                summary,
                "stop-control",
                summary.stop_control_ref.as_deref(),
                diagnostics,
            )?;
        }
    }
    Ok(seen)
}

fn collect_summary_optional_ref_diagnostics(
    summary: &ClusterLifecycleNodeSummary,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    for (label, reference) in [
        ("identity", summary.identity_ref.as_deref()),
        ("startup", summary.startup_ref.as_deref()),
        ("health", summary.health_ref.as_deref()),
        ("queue", summary.queue_ref.as_deref()),
        ("control", summary.control_ref.as_deref()),
        ("heartbeat", summary.heartbeat_ref.as_deref()),
        ("shutdown", summary.shutdown_ref.as_deref()),
        ("stop-control", summary.stop_control_ref.as_deref()),
        ("already-running", summary.already_running_ref.as_deref()),
    ] {
        collect_lifecycle_optional_ref_diagnostic(label, reference, diagnostics)?;
    }
    Ok(())
}

fn collect_required_optional_summary_ref(
    summary: &ClusterLifecycleNodeSummary,
    label: &str,
    reference: Option<&str>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if reference.is_none() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-{label}:{}", summary.node_id))?;
    }
    Ok(())
}

fn collect_phase_diagnostics(phase: &ClusterLifecyclePhaseObservation, diagnostics: &mut Vec<String>) -> Result<()> {
    collect_lifecycle_text_diagnostic("phase", &phase.phase, diagnostics)?;
    collect_lifecycle_decision_diagnostic(&phase.phase, &phase.decision, diagnostics)?;
    collect_lifecycle_ref_diagnostics("phase-receipt", &phase.receipt_refs, diagnostics)?;
    if phase.receipt_refs.is_empty() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-phase-receipts:{}", phase.phase))?;
    }
    Ok(())
}

fn collect_stop_order_diagnostics(input: &ClusterLifecycleRunInput, diagnostics: &mut Vec<String>) -> Result<()> {
    let has_stop = cluster_lifecycle_has_phase(input, CLUSTER_LIFECYCLE_PHASE_STOP);
    for node_id in &input.stop_order {
        collect_lifecycle_text_diagnostic("stop-order-node", node_id, diagnostics)?;
    }
    if has_stop && input.stop_order.is_empty() {
        push_lifecycle_diagnostic(diagnostics, "cluster-lifecycle-missing-stop-order")?;
    }
    if input.stop_order.is_empty() {
        return Ok(());
    }
    let expected = input.ordered_node_ids.iter().rev().cloned().collect::<Vec<_>>();
    if input.stop_order != expected {
        push_lifecycle_diagnostic(diagnostics, "cluster-lifecycle-stop-order-drift")?;
    }
    Ok(())
}

fn cluster_lifecycle_has_canonical_receipts(input: &ClusterLifecycleRunInput) -> bool {
    input.phases.iter().any(|phase| !phase.receipt_refs.is_empty())
        || !input.already_running_refs.is_empty()
        || input.node_summaries.iter().any(summary_has_lifecycle_receipts)
}

fn summary_has_lifecycle_receipts(summary: &ClusterLifecycleNodeSummary) -> bool {
    [
        &summary.identity_ref,
        &summary.startup_ref,
        &summary.health_ref,
        &summary.queue_ref,
        &summary.control_ref,
        &summary.heartbeat_ref,
        &summary.shutdown_ref,
        &summary.stop_control_ref,
        &summary.already_running_ref,
    ]
    .iter()
    .any(|reference| reference.is_some())
}

fn cluster_lifecycle_has_phase(input: &ClusterLifecycleRunInput, phase_name: &str) -> bool {
    input.phases.iter().any(|phase| phase.phase == phase_name)
}

fn collect_lifecycle_decision_diagnostic(phase: &str, decision: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match decision {
        CLUSTER_LIFECYCLE_PASS
        | CLUSTER_LIFECYCLE_DENY
        | CLUSTER_LIFECYCLE_ALREADY_RUNNING
        | CLUSTER_LIFECYCLE_SKIPPED
        | CLUSTER_LIFECYCLE_UNAVAILABLE => Ok(()),
        other => {
            push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-unsupported-decision:{phase}:{other}"))
        }
    }
}

fn collect_lifecycle_text_diagnostic(label: &str, value: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    if value.trim().is_empty() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-{label}"))?;
    }
    Ok(())
}

fn collect_lifecycle_ref_diagnostics(label: &str, refs: &[String], diagnostics: &mut Vec<String>) -> Result<()> {
    if refs.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "cluster lifecycle {label} ref count {} exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}",
            refs.len()
        )));
    }
    for reference in refs {
        collect_lifecycle_ref_diagnostic(label, reference, diagnostics)?;
    }
    Ok(())
}

fn collect_lifecycle_optional_ref_diagnostic(
    label: &str,
    reference: Option<&str>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if let Some(reference) = reference {
        collect_lifecycle_ref_diagnostic(label, reference, diagnostics)?;
    }
    Ok(())
}

fn collect_lifecycle_ref_diagnostic(label: &str, reference: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    if crate::preserves_rail::validate_content_ref(reference).is_err() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-invalid-{label}-ref"))?;
    }
    Ok(())
}

fn push_lifecycle_if(diagnostics: &mut Vec<String>, condition: bool, diagnostic: &'static str) -> Result<()> {
    if condition {
        push_lifecycle_diagnostic(diagnostics, diagnostic)?;
    }
    Ok(())
}

fn push_lifecycle_diagnostic(diagnostics: &mut Vec<String>, diagnostic: impl Into<String>) -> Result<()> {
    if diagnostics.len() >= MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "cluster lifecycle diagnostic count exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}"
        )));
    }
    diagnostics.push(diagnostic.into());
    Ok(())
}

fn cluster_lifecycle_decision(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        CLUSTER_LIFECYCLE_PASS
    } else {
        CLUSTER_LIFECYCLE_DENY
    }
}

fn cluster_lifecycle_run_value(
    input: &ClusterLifecycleRunInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("cluster-lifecycle-run-v1", vec![
        string(CLUSTER_LIFECYCLE_RUN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("workflow", vec![string(&input.workflow_id)]),
        record("manifest", vec![string(&input.manifest_ref)]),
        record("nodes", vec![strings_sequence(&input.ordered_node_ids)]),
        record("phases", vec![sequence(cluster_lifecycle_phase_values(&input.phases)?)]),
        record("node-summaries", vec![sequence(cluster_lifecycle_node_values(&input.node_summaries)?)]),
        record("already-running", vec![refs_sequence(&input.already_running_refs)]),
        record("stop-order", vec![strings_sequence(&input.stop_order)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        record("checks", vec![sequence(vec![
            check_value(
                "manifest-bound",
                cluster_lifecycle_decision_status(diagnostics, "cluster-lifecycle-stale-manifest"),
            ),
            check_value(
                "phase-receipts-bound",
                cluster_lifecycle_decision_status(diagnostics, "cluster-lifecycle-missing-phase-receipts"),
            ),
            check_value(
                "stdout-not-evidence",
                cluster_lifecycle_decision_status(diagnostics, "cluster-lifecycle-stdout-only-evidence"),
            ),
            check_value(
                "stop-order-bound",
                cluster_lifecycle_decision_status(diagnostics, "cluster-lifecycle-stop-order-drift"),
            ),
        ])]),
    ]))
}

fn cluster_lifecycle_decision_status(diagnostics: &[String], prefix: &str) -> &'static str {
    if diagnostics.iter().any(|diagnostic| diagnostic.starts_with(prefix)) {
        CLUSTER_LIFECYCLE_DENY
    } else {
        CLUSTER_LIFECYCLE_PASS
    }
}

fn cluster_lifecycle_phase_values(phases: &[ClusterLifecyclePhaseObservation]) -> Result<Vec<IoValue>> {
    if phases.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "cluster lifecycle phase count {} exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}",
            phases.len()
        )));
    }
    Ok(phases
        .iter()
        .map(|phase| {
            record("phase", vec![
                record("name", vec![string(&phase.phase)]),
                record("decision", vec![string(&phase.decision)]),
                record("receipts", vec![refs_sequence(&phase.receipt_refs)]),
            ])
        })
        .collect())
}

fn cluster_lifecycle_node_values(summaries: &[ClusterLifecycleNodeSummary]) -> Result<Vec<IoValue>> {
    if summaries.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "cluster lifecycle node summary count {} exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}",
            summaries.len()
        )));
    }
    Ok(summaries
        .iter()
        .map(|summary| {
            record("node", vec![
                record("id", vec![string(&summary.node_id)]),
                record("manifest", vec![string(&summary.manifest_ref)]),
                record("config", vec![string(&summary.config_ref)]),
                optional_ref_field("identity", summary.identity_ref.as_deref()),
                optional_ref_field("startup", summary.startup_ref.as_deref()),
                optional_ref_field("health", summary.health_ref.as_deref()),
                optional_ref_field("queue", summary.queue_ref.as_deref()),
                optional_ref_field("control", summary.control_ref.as_deref()),
                optional_ref_field("heartbeat", summary.heartbeat_ref.as_deref()),
                optional_ref_field("shutdown", summary.shutdown_ref.as_deref()),
                optional_ref_field("stop-control", summary.stop_control_ref.as_deref()),
                optional_ref_field("already-running", summary.already_running_ref.as_deref()),
            ])
        })
        .collect())
}

fn optional_ref_field(label: &'static str, reference: Option<&str>) -> IoValue {
    record(label, vec![optional_ref_value(reference)])
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map(string).unwrap_or_else(|| record("none", Vec::new()))
}

fn push_lifecycle_optional_ref_field(
    fields: &mut Vec<crate::drift_core::EvidenceField>,
    node_id: &str,
    label: &str,
    reference: Option<&str>,
) -> Result<()> {
    let path = format!("node:{node_id}:{label}");
    match reference {
        Some(reference) => push_lifecycle_summary_field(fields, &path, reference, true),
        None => push_lifecycle_summary_field(fields, &path, CLUSTER_LIFECYCLE_NONE, false),
    }
}

fn push_lifecycle_ref_fields(
    fields: &mut Vec<crate::drift_core::EvidenceField>,
    prefix: &str,
    refs: &[String],
) -> Result<()> {
    if refs.is_empty() {
        push_lifecycle_summary_field(fields, prefix, CLUSTER_LIFECYCLE_NONE, false)?;
        return Ok(());
    }
    for (index, reference) in refs.iter().enumerate() {
        push_lifecycle_summary_field(fields, &format!("{prefix}:{index}"), reference, true)?;
    }
    Ok(())
}

fn push_lifecycle_summary_field(
    fields: &mut Vec<crate::drift_core::EvidenceField>,
    path: &str,
    value: &str,
    is_ref: bool,
) -> Result<()> {
    if fields.len() >= MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "cluster lifecycle drift field count exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}"
        )));
    }
    fields.push(crate::drift_core::EvidenceField {
        path: path.to_string(),
        value: value.to_string(),
        is_ref,
    });
    Ok(())
}

fn join_lifecycle_values(values: &[String]) -> String {
    if values.is_empty() {
        return CLUSTER_LIFECYCLE_NONE.to_string();
    }
    values.join(CLUSTER_LIFECYCLE_STOP_SEPARATOR)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(format!("cluster-lifecycle:{label}").as_bytes())
    }

    fn phase(name: &str) -> ClusterLifecyclePhaseObservation {
        ClusterLifecyclePhaseObservation {
            phase: name.to_string(),
            decision: CLUSTER_LIFECYCLE_PASS.to_string(),
            receipt_refs: vec![local_ref(&format!("phase:{name}"))],
        }
    }

    fn lifecycle_node(node_id: &str, manifest_ref: &str) -> ClusterLifecycleNodeSummary {
        ClusterLifecycleNodeSummary {
            node_id: node_id.to_string(),
            manifest_ref: manifest_ref.to_string(),
            config_ref: local_ref(&format!("{node_id}:config")),
            identity_ref: Some(local_ref(&format!("{node_id}:identity"))),
            startup_ref: Some(local_ref(&format!("{node_id}:startup"))),
            health_ref: Some(local_ref(&format!("{node_id}:health"))),
            queue_ref: Some(local_ref(&format!("{node_id}:queue"))),
            control_ref: Some(local_ref(&format!("{node_id}:control"))),
            heartbeat_ref: Some(local_ref(&format!("{node_id}:heartbeat"))),
            shutdown_ref: Some(local_ref(&format!("{node_id}:shutdown"))),
            stop_control_ref: Some(local_ref(&format!("{node_id}:stop-control"))),
            already_running_ref: Some(local_ref(&format!("{node_id}:already-running"))),
        }
    }

    fn lifecycle_input() -> ClusterLifecycleRunInput {
        let manifest_ref = local_ref("manifest");
        ClusterLifecycleRunInput {
            workflow_id: "cluster-two-node-lifecycle".to_string(),
            manifest_ref: manifest_ref.clone(),
            ordered_node_ids: vec!["node:node-a".to_string(), "node:node-b".to_string()],
            phases: vec![
                phase(CLUSTER_LIFECYCLE_PHASE_INIT),
                phase(CLUSTER_LIFECYCLE_PHASE_START),
                phase(CLUSTER_LIFECYCLE_PHASE_STATUS),
                phase(CLUSTER_LIFECYCLE_PHASE_STOP),
            ],
            node_summaries: vec![
                lifecycle_node("node:node-a", &manifest_ref),
                lifecycle_node("node:node-b", &manifest_ref),
            ],
            already_running_refs: vec![local_ref("already-running-observation")],
            stop_order: vec!["node:node-b".to_string(), "node:node-a".to_string()],
            diagnostics: Vec::new(),
            caveats: vec!["cluster lifecycle run evidence is local wrapper evidence only".to_string()],
        }
    }

    #[test]
    fn cluster_lifecycle_run_receipt_binds_complete_two_node_workflow() {
        // r[verify molten.testing.cluster_lifecycle_receipt.run_receipt]
        let receipt = build_cluster_lifecycle_run_receipt(&lifecycle_input()).expect("cluster lifecycle receipt");
        let rendered = crate::preserves_rail::to_text(&receipt.value).expect("render lifecycle receipt");

        assert_eq!(receipt.decision, CLUSTER_LIFECYCLE_PASS);
        assert!(receipt.diagnostics.is_empty());
        assert!(receipt.receipt_ref.starts_with("blake3:"));
        assert!(rendered.contains("cluster-lifecycle-run-v1"));
        assert!(rendered.contains("stdout-not-evidence"));
    }

    #[test]
    fn cluster_lifecycle_run_receipt_denies_missing_stale_and_stdout_only_evidence() {
        // r[verify molten.testing.cluster_lifecycle_receipt.fail_closed_validation]
        let mut input = lifecycle_input();
        for phase in &mut input.phases {
            phase.receipt_refs.clear();
        }
        input.node_summaries[0].identity_ref = None;
        input.node_summaries[0].startup_ref = None;
        input.node_summaries[0].health_ref = None;
        input.node_summaries[0].queue_ref = None;
        input.node_summaries[0].control_ref = None;
        input.node_summaries[0].heartbeat_ref = None;
        input.node_summaries[0].shutdown_ref = None;
        input.node_summaries[0].stop_control_ref = None;
        input.node_summaries[0].already_running_ref = None;
        input.node_summaries[1].node_id = input.node_summaries[0].node_id.clone();
        input.node_summaries[1].manifest_ref = local_ref("stale-manifest");
        input.node_summaries[1].identity_ref = None;
        input.node_summaries[1].startup_ref = None;
        input.node_summaries[1].health_ref = None;
        input.node_summaries[1].queue_ref = None;
        input.node_summaries[1].control_ref = None;
        input.node_summaries[1].heartbeat_ref = None;
        input.node_summaries[1].shutdown_ref = None;
        input.node_summaries[1].stop_control_ref = None;
        input.node_summaries[1].already_running_ref = None;
        input.already_running_refs.clear();
        input.stop_order.reverse();
        let receipt = build_cluster_lifecycle_run_receipt(&input).expect("denied lifecycle receipt");

        assert_eq!(receipt.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-missing-phase-receipts:")));
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-stale-manifest:")));
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-duplicate-node-summary:")));
        assert!(receipt.diagnostics.iter().any(|item| item == "cluster-lifecycle-stop-order-drift"));
        assert!(receipt.diagnostics.iter().any(|item| item == "cluster-lifecycle-stdout-only-evidence"));
    }

    #[test]
    fn cluster_lifecycle_drift_summary_compares_stable_fields() {
        // r[verify molten.testing.cluster_lifecycle_summary_drift.receipt_summary]
        let first = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("first lifecycle summary");
        let second = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("second lifecycle summary");
        let comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: first,
            right: second,
            allowed_variances: Vec::new(),
        })
        .expect("stable lifecycle drift comparison");

        assert_eq!(comparison.decision, CLUSTER_LIFECYCLE_PASS);
        assert!(comparison.diagnostics.is_empty());
    }

    #[test]
    fn cluster_lifecycle_drift_summary_denies_child_node_and_field_kind_drift() {
        // r[verify molten.testing.cluster_lifecycle_summary_drift.negatives]
        let left = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("left summary");
        let mut changed_child_input = lifecycle_input();
        changed_child_input.node_summaries[0].startup_ref = Some(local_ref("node-a:startup:changed"));
        let changed_child = cluster_lifecycle_drift_summary(&changed_child_input).expect("changed child summary");
        let child_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: left.clone(),
            right: changed_child,
            allowed_variances: Vec::new(),
        })
        .expect("changed child comparison");

        let mut changed_order_input = lifecycle_input();
        changed_order_input.ordered_node_ids.reverse();
        let changed_order = cluster_lifecycle_drift_summary(&changed_order_input).expect("changed order summary");
        let order_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: left.clone(),
            right: changed_order,
            allowed_variances: Vec::new(),
        })
        .expect("changed order comparison");

        let mut missing_field_input = lifecycle_input();
        missing_field_input.node_summaries[0].startup_ref = None;
        let missing_field = cluster_lifecycle_drift_summary(&missing_field_input).expect("missing field summary");
        let field_kind_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left,
            right: missing_field,
            allowed_variances: Vec::new(),
        })
        .expect("field kind comparison");

        assert_eq!(child_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(child_comparison.diagnostics.iter().any(|diagnostic| diagnostic.path == "node:node:node-a:startup"));
        assert_eq!(order_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(order_comparison.diagnostics.iter().any(|diagnostic| diagnostic.path == "node-order"));
        assert_eq!(field_kind_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(field_kind_comparison.diagnostics.iter().any(|diagnostic| diagnostic.kind == "field-kind-drift"));
    }

    #[test]
    fn plans_cluster_nodes_and_round_trips_manifest() {
        const EXPECTED_CLUSTER_NODE_COUNT: usize = 2;

        let root = PathBuf::from("target/cluster");
        let plan = plan_cluster(&root, &node_names(&["node-a", "node_b"])).expect("cluster plan");
        assert_eq!(plan.nodes.len(), EXPECTED_CLUSTER_NODE_COUNT);
        assert_eq!(plan.nodes[0].node_id, "node:node-a");
        assert_eq!(plan.nodes[0].path_component, "node-a");
        assert_eq!(plan.nodes[0].state_root, root.join("node-a"));
        assert_eq!(plan.nodes[1].node_id, "node:node_b");
        assert_eq!(cluster_manifest_path(&root), root.join(CLUSTER_MANIFEST_FILE));

        let manifest = render_cluster_manifest(&plan);
        let parsed = parse_cluster_manifest(&manifest).expect("parse manifest");
        let reparsed_plan = plan_cluster(&root, &parsed).expect("reparsed plan");
        let reparsed_node_ids: Vec<&str> = reparsed_plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        let planned_node_ids: Vec<&str> = plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        assert_eq!(reparsed_node_ids, planned_node_ids);
        assert_eq!(reparsed_plan.state_root, plan.state_root);
    }

    #[test]
    fn denies_empty_duplicate_and_unsafe_nodes() {
        let root = PathBuf::from("target/cluster");
        let empty = plan_cluster(&root, &[]).expect_err("empty denied");
        assert!(empty.to_string().contains("at least one"));

        let duplicate = plan_cluster(&root, &node_names(&["node-a", "node:node-a"])).expect_err("duplicate denied");
        assert!(duplicate.to_string().contains("duplicate cluster node"));

        let relative = plan_cluster(&root, &node_names(&["../node-a"])).expect_err("relative denied");
        assert!(relative.to_string().contains("ASCII letters"));

        let colon = plan_cluster(&root, &node_names(&["node:a:b"])).expect_err("colon denied");
        assert!(colon.to_string().contains("must not contain ':'"));

        let current_root = plan_cluster(Path::new("."), &node_names(&["node-a"])).expect_err("current root denied");
        assert!(current_root.to_string().contains("must not be ambient"));

        let parent_root = plan_cluster(Path::new(".."), &node_names(&["node-a"])).expect_err("parent root denied");
        assert!(parent_root.to_string().contains("must not be ambient"));
    }

    #[test]
    fn denies_malformed_manifests() {
        let empty = parse_cluster_manifest("").expect_err("empty manifest denied");
        assert!(empty.to_string().contains("manifest is empty"));

        let header = parse_cluster_manifest("not-a-cluster\nnode:node-a\n").expect_err("bad header denied");
        assert!(header.to_string().contains("unsupported header"));

        let no_nodes = parse_cluster_manifest("molten.cluster.nodes.v1\n").expect_err("empty nodes denied");
        assert!(no_nodes.to_string().contains("no nodes"));
    }
}
