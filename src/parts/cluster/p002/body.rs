
fn cluster_lifecycle_run_value(
    input: &ClusterLifecycleRunInput,
    decision: &str,
    diagnostics: &[String],
) -> crate::error::Result<IoValue> {
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

fn cluster_lifecycle_phase_values(phases: &[ClusterLifecyclePhaseObservation]) -> crate::error::Result<Vec<IoValue>> {
    if phases.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
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

fn cluster_lifecycle_node_values(summaries: &[ClusterLifecycleNodeSummary]) -> crate::error::Result<Vec<IoValue>> {
    if summaries.len() > MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
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
    fields: &mut impl crate::bounded::VecSink<crate::drift_core::EvidenceField>,
    node_id: &str,
    label: &str,
    reference: Option<&str>,
) -> crate::error::Result<()> {
    let path = format!("node:{node_id}:{label}");
    match reference {
        Some(reference) => push_lifecycle_summary_field(fields, &path, reference, true),
        None => push_lifecycle_summary_field(fields, &path, CLUSTER_LIFECYCLE_NONE, false),
    }
}

fn push_lifecycle_ref_fields(
    fields: &mut impl crate::bounded::VecSink<crate::drift_core::EvidenceField>,
    prefix: &str,
    refs: &[String],
) -> crate::error::Result<()> {
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
    fields: &mut impl crate::bounded::VecSink<crate::drift_core::EvidenceField>,
    path: &str,
    value: &str,
    is_ref: bool,
) -> crate::error::Result<()> {
    if fields.item_count() >= MAX_CLUSTER_LIFECYCLE_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cluster lifecycle drift field count exceeds bound {MAX_CLUSTER_LIFECYCLE_ITEMS}"
        )));
    }
    fields.push_item(crate::drift_core::EvidenceField {
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
