
fn cluster_lifecycle_run_diagnostics(input: &ClusterLifecycleRunInput) -> crate::error::Result<Vec<String>> {
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

fn collect_ordered_lifecycle_nodes(
    node_ids: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<std::collections::BTreeSet<String>> {
    let mut seen = std::collections::BTreeSet::new();
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
    ordered_nodes: &std::collections::BTreeSet<String>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<std::collections::BTreeSet<String>> {
    let mut seen = std::collections::BTreeSet::new();
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
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
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
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    if reference.is_none() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-{label}:{}", summary.node_id))?;
    }
    Ok(())
}

fn collect_phase_diagnostics(
    phase: &ClusterLifecyclePhaseObservation,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    collect_lifecycle_text_diagnostic("phase", &phase.phase, diagnostics)?;
    collect_lifecycle_decision_diagnostic(&phase.phase, &phase.decision, diagnostics)?;
    collect_lifecycle_ref_diagnostics("phase-receipt", &phase.receipt_refs, diagnostics)?;
    if phase.receipt_refs.is_empty() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-phase-receipts:{}", phase.phase))?;
    }
    Ok(())
}

fn collect_stop_order_diagnostics(
    input: &ClusterLifecycleRunInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
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

fn collect_lifecycle_decision_diagnostic(
    phase: &str,
    decision: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
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

fn collect_lifecycle_text_diagnostic(
    label: &str,
    value: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    if value.trim().is_empty() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-missing-{label}"))?;
    }
    Ok(())
}

fn collect_lifecycle_ref_diagnostics(
    label: &str,
    refs: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    if refs.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
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
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    if let Some(reference) = reference {
        collect_lifecycle_ref_diagnostic(label, reference, diagnostics)?;
    }
    Ok(())
}

fn collect_lifecycle_ref_diagnostic(
    label: &str,
    reference: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    if crate::preserves_rail::validate_content_ref(reference).is_err() {
        push_lifecycle_diagnostic(diagnostics, format!("cluster-lifecycle-invalid-{label}-ref"))?;
    }
    Ok(())
}

fn push_lifecycle_if(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    condition: bool,
    diagnostic: &'static str,
) -> crate::error::Result<()> {
    if condition {
        push_lifecycle_diagnostic(diagnostics, diagnostic)?;
    }
    Ok(())
}

fn push_lifecycle_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: impl Into<String>,
) -> crate::error::Result<()> {
    if diagnostics.item_count() >= MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cluster lifecycle diagnostic count exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}"
        )));
    }
    diagnostics.push_item(diagnostic.into());
    Ok(())
}

fn cluster_lifecycle_decision(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        CLUSTER_LIFECYCLE_PASS
    } else {
        CLUSTER_LIFECYCLE_DENY
    }
}
