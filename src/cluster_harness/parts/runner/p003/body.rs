
fn finalize_child_execution(
    input: &ClusterHarnessExecutionInput,
    node: &crate::cluster::ClusterNodePlan,
    phase: &str,
    observation: ChildProcessObservation,
) -> crate::error::Result<ChildExecution> {
    let log = format!(
        "phase={phase}\nnode={}\nsuccess={}\ntimed_out={}\norphaned={}\nexit_code={}\nstdout:\n{}\nstderr:\n{}",
        node.node_id,
        observation.succeeded,
        observation.timed_out,
        observation.orphaned,
        observation.exit_code.map_or_else(|| "none".to_string(), |code| code.to_string()),
        observation.stdout,
        observation.stderr,
    );
    let log_path = input.output_directory.join(format!("logs/{phase}-{}.log", node.path_component));
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::write(&log_path, &log).map_err(crate::error::MoltenError::from)?;
    let diagnostic_log_ref = content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &log);
    let command_profile_ref = content_ref_for_text(COMMAND_PROFILE_DOMAIN, &format!("{phase}:{}", node.node_id));
    let value = child_process_value(&ClusterHarnessChildProcessInput {
        node_id: node.node_id.clone(),
        phase: phase.to_string(),
        command_profile_ref,
        diagnostic_log_ref,
        exit_code: observation.exit_code,
        timed_out: observation.timed_out,
        orphaned: observation.orphaned,
        succeeded: observation.succeeded,
    })?;
    let process_ref = crate::preserves_rail::canonical_hash(&value)?;
    let diagnostic = if observation.succeeded {
        None
    } else {
        Some(format!("cluster-harness-child-failed:{phase}:{}", node.node_id))
    };
    Ok(ChildExecution {
        node_id: node.node_id.clone(),
        phase: phase.to_string(),
        process_ref,
        value,
        succeeded: observation.succeeded,
        timed_out: observation.timed_out,
        orphaned: observation.orphaned,
        diagnostic,
    })
}

fn collect_child_diagnostics(executions: &[ChildExecution], diagnostics: &mut impl crate::bounded::VecSink<String>) {
    diagnostics.extend_items(executions.iter().filter_map(|child| child.diagnostic.clone()));
}

fn capture_node_artifacts(
    node: &crate::cluster::ClusterNodePlan,
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
    child_receipt_refs: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<NodeArtifacts> {
    let mut observed = NodeArtifacts::default();
    for (file, assign) in [
        ("config.preserves", NodeArtifactField::Config),
        ("identity-receipt.preserves", NodeArtifactField::Identity),
        ("startup-receipt.preserves", NodeArtifactField::Startup),
        ("cluster-harness-workflow.preserves", NodeArtifactField::Workflow),
        ("cluster-harness-heartbeat.preserves", NodeArtifactField::Heartbeat),
        ("health-receipt.preserves", NodeArtifactField::Health),
        ("status-control-receipt.preserves", NodeArtifactField::Control),
        ("shutdown-receipt.preserves", NodeArtifactField::Shutdown),
        ("stop-control-receipt.preserves", NodeArtifactField::StopControl),
    ] {
        let source = node.state_root.join(file);
        if !source.exists() {
            continue;
        }
        let value = read_preserves_path(&source)?;
        let reference = crate::preserves_rail::canonical_hash(&value)?;
        let kind = crate::ledger::artifact_kind(&value);
        let relative = format!("children/receipts/{}/{file}", node.path_component);
        push_artifact(artifacts, &relative, kind, value)?;
        child_receipt_refs.push_item(reference.clone());
        observed.assign(assign, reference);
    }
    Ok(observed)
}

#[derive(Debug, Clone, Copy)]
enum NodeArtifactField {
    Config,
    Identity,
    Startup,
    Workflow,
    Heartbeat,
    Health,
    Control,
    Shutdown,
    StopControl,
}

impl NodeArtifacts {
    fn assign(&mut self, field: NodeArtifactField, reference: String) {
        match field {
            NodeArtifactField::Config => self.config_ref = Some(reference),
            NodeArtifactField::Identity => self.identity_ref = Some(reference),
            NodeArtifactField::Startup => self.startup_ref = Some(reference),
            NodeArtifactField::Workflow => self.workflow_ref = Some(reference),
            NodeArtifactField::Heartbeat => self.heartbeat_ref = Some(reference),
            NodeArtifactField::Health => self.health_ref = Some(reference),
            NodeArtifactField::Control => self.control_ref = Some(reference),
            NodeArtifactField::Shutdown => self.shutdown_ref = Some(reference),
            NodeArtifactField::StopControl => self.stop_control_ref = Some(reference),
        }
    }

    fn complete(&self) -> bool {
        self.config_ref.is_some()
            && self.identity_ref.is_some()
            && self.startup_ref.is_some()
            && self.workflow_ref.is_some()
            && self.heartbeat_ref.is_some()
            && self.health_ref.is_some()
            && self.control_ref.is_some()
            && self.shutdown_ref.is_some()
            && self.stop_control_ref.is_some()
    }
}

struct LifecycleArtifacts {
    lifecycle_ref: String,
    lifecycle_value: IoValue,
    drift_ref: String,
    drift_value: IoValue,
}

struct LifecycleArtifactsInput<'a> {
    fixture_ref: &'a str,
    node_ids: &'a [String],
    nodes: &'a [NodeArtifacts],
    child_executions: &'a [ChildExecution],
    diagnostics: &'a [String],
    phases_passed: bool,
    caveats: &'a [String],
}

fn build_lifecycle_artifacts(input: LifecycleArtifactsInput<'_>) -> crate::error::Result<LifecycleArtifacts> {
    let LifecycleArtifactsInput {
        fixture_ref,
        node_ids,
        nodes,
        child_executions,
        diagnostics,
        phases_passed,
        caveats,
    } = input;
    let is_complete = phases_passed && nodes.len() == node_ids.len() && nodes.iter().all(NodeArtifacts::complete);
    let (lifecycle_value, drift_summary) = if is_complete {
        let phase = |name: &str| crate::cluster::ClusterLifecyclePhaseObservation {
            phase: name.to_string(),
            decision: molten_core::cluster_harness::RUN_DIRECTORY_PASS.to_string(),
            receipt_refs: child_executions
                .iter()
                .filter(|child| child.phase == name)
                .map(|child| child.process_ref.clone())
                .collect(),
        };
        let input = crate::cluster::ClusterLifecycleRunInput {
            workflow_id: WORKFLOW_ID.to_string(),
            manifest_ref: fixture_ref.to_string(),
            ordered_node_ids: node_ids.to_vec(),
            phases: vec![
                phase("init"),
                phase("start"),
                phase("workflow"),
                phase("status"),
                phase("stop"),
            ],
            node_summaries: lifecycle_node_summaries(fixture_ref, node_ids, nodes)?,
            already_running_refs: Vec::new(),
            stop_order: node_ids.iter().rev().cloned().collect(),
            diagnostics: diagnostics.to_vec(),
            caveats: caveats.to_vec(),
        };
        let lifecycle = crate::cluster::build_cluster_lifecycle_run_receipt(&input)?;
        (lifecycle.value, crate::cluster::cluster_lifecycle_drift_summary(&input)?)
    } else {
        let mut unavailable = diagnostics.to_vec();
        unavailable.push("cluster-harness-lifecycle-evidence-incomplete".to_string());
        unavailable.sort();
        unavailable.dedup();
        let lifecycle = unavailable_cluster_lifecycle_value(fixture_ref, node_ids, &unavailable, caveats)?;
        let summary = crate::drift_core::EvidenceSummary {
            workflow: WORKFLOW_ID.to_string(),
            fields: vec![crate::drift_core::EvidenceField {
                path: "lifecycle-status".to_string(),
                value: molten_core::cluster_harness::RUN_DIRECTORY_DENY.to_string(),
                is_ref: false,
            }],
        };
        (lifecycle, summary)
    };
    let lifecycle_ref = crate::preserves_rail::canonical_hash(&lifecycle_value)?;
    let drift_value = drift_summary_value(&drift_summary)?;
    let drift_ref = crate::preserves_rail::canonical_hash(&drift_value)?;
    Ok(LifecycleArtifacts {
        lifecycle_ref,
        lifecycle_value,
        drift_ref,
        drift_value,
    })
}

fn lifecycle_node_summaries(
    fixture_ref: &str,
    node_ids: &[String],
    nodes: &[NodeArtifacts],
) -> crate::error::Result<Vec<crate::cluster::ClusterLifecycleNodeSummary>> {
    node_ids
        .iter()
        .zip(nodes)
        .map(|(node_id, node)| {
            let config_ref = node
                .config_ref
                .clone()
                .ok_or_else(|| crate::error::MoltenError::invalid_harness("complete cluster node has no config ref"))?;
            Ok(crate::cluster::ClusterLifecycleNodeSummary {
                node_id: node_id.clone(),
                manifest_ref: fixture_ref.to_string(),
                config_ref,
                identity_ref: node.identity_ref.clone(),
                startup_ref: node.startup_ref.clone(),
                health_ref: node.health_ref.clone(),
                queue_ref: None,
                control_ref: node.control_ref.clone(),
                heartbeat_ref: node.heartbeat_ref.clone(),
                shutdown_ref: node.shutdown_ref.clone(),
                stop_control_ref: node.stop_control_ref.clone(),
                already_running_ref: None,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()
}

struct CleanupObservation {
    removed_ticket_refs: Vec<String>,
    remaining_ticket_paths: Vec<String>,
    succeeded: bool,
}

fn cleanup_state_roots(plan: &crate::cluster::ClusterPlan) -> crate::error::Result<CleanupObservation> {
    let mut ticket_paths = Vec::new();
    for node in &plan.nodes {
        collect_ticket_paths(&node.state_root, &node.state_root, &mut ticket_paths)?;
    }
    let mut removed_ticket_refs = Vec::with_capacity(ticket_paths.len());
    let mut is_succeeded = true;
    for path in &ticket_paths {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        removed_ticket_refs.push(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text));
        if std::fs::remove_file(path).is_err() {
            is_succeeded = false;
        }
    }
    let mut remaining = Vec::new();
    for node in &plan.nodes {
        collect_ticket_paths(&node.state_root, &node.state_root, &mut remaining)?;
    }
    let remaining_ticket_paths = remaining.into_iter().map(|path| path.display().to_string()).collect::<Vec<_>>();
    Ok(CleanupObservation {
        removed_ticket_refs,
        succeeded: is_succeeded && remaining_ticket_paths.is_empty(),
        remaining_ticket_paths,
    })
}
