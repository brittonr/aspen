
/// Runs the node phases, then captures node artifacts, cleans up, and records the cleanup,
/// lifecycle, drift, and local executable run artifacts in that order.
fn record_run_evidence(
    input: &ClusterHarnessExecutionInput,
    mut planned: PlannedRun,
) -> crate::error::Result<RunEvidence> {
    let mut phases = run_lifecycle_phases(input, &planned.plan, &mut planned.artifacts)?;
    collect_child_diagnostics(&phases.child_executions, &mut phases.diagnostics);
    let mut child_receipt_refs = Vec::new();
    let node_artifacts = planned
        .plan
        .nodes
        .iter()
        .map(|node| capture_node_artifacts(node, &mut planned.artifacts, &mut child_receipt_refs))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let cleanup_observation = cleanup_state_roots(&planned.plan)?;
    let child_process_refs = phases.child_executions.iter().map(|child| child.process_ref.clone()).collect::<Vec<_>>();
    child_receipt_refs.extend(child_process_refs.iter().cloned());
    child_receipt_refs.sort();
    child_receipt_refs.dedup();
    let cleanup_input = run_cleanup_input(&planned, &phases, child_process_refs.clone(), cleanup_observation);
    let cleanup = cleanup_value(&cleanup_input)?;
    let cleanup_ref = crate::preserves_rail::canonical_hash(&cleanup)?;
    push_artifact(&mut planned.artifacts, CLEANUP_FILE, CLEANUP_KIND, cleanup)?;

    let lifecycle = build_lifecycle_artifacts(LifecycleArtifactsInput {
        fixture_ref: &planned.fixture_ref,
        node_ids: &planned.node_ids,
        nodes: &node_artifacts,
        child_executions: &phases.child_executions,
        diagnostics: &phases.diagnostics,
        phases_passed: phases.all_passed(),
        caveats: &planned.caveats,
    })?;
    push_artifact(&mut planned.artifacts, LIFECYCLE_FILE, CLUSTER_LIFECYCLE_KIND, lifecycle.lifecycle_value)?;
    push_artifact(&mut planned.artifacts, DRIFT_SUMMARY_FILE, DRIFT_SUMMARY_KIND, lifecycle.drift_value)?;
    let local_executable = local_executable_run(&planned, &node_artifacts, &cleanup_input, &cleanup_ref, &phases)?;
    if local_executable.decision != molten_core::cluster_harness::RUN_DIRECTORY_PASS {
        phases
            .diagnostics
            .extend(local_executable.diagnostics.iter().map(|item| format!("local-run:{item}")));
    }
    push_artifact(
        &mut planned.artifacts,
        LOCAL_EXECUTABLE_RUN_FILE,
        LOCAL_EXECUTABLE_RUN_KIND,
        local_executable.value,
    )?;
    Ok(RunEvidence {
        plan: planned,
        child_executions: phases.child_executions,
        diagnostics: phases.diagnostics,
        child_process_refs,
        child_receipt_refs,
        cleanup_ref,
        lifecycle_ref: lifecycle.lifecycle_ref,
        drift_ref: lifecycle.drift_ref,
        local_plan_ref: local_executable.plan_ref,
        local_run_ref: local_executable.executable_ref,
    })
}

/// The cleanup record: every child process, the nodes stopped in reverse order when stop passed,
/// orphaned children, and the ticket cleanup observation.
fn run_cleanup_input(
    planned: &PlannedRun,
    phases: &LifecyclePhases,
    child_process_refs: Vec<String>,
    cleanup_observation: CleanupObservation,
) -> ClusterHarnessCleanupInput {
    ClusterHarnessCleanupInput {
        child_process_refs,
        stopped_node_ids: if phases.is_stop_passed {
            planned.plan.nodes.iter().rev().map(|node| node.node_id.clone()).collect()
        } else {
            Vec::new()
        },
        orphaned_processes: phases
            .child_executions
            .iter()
            .filter(|child| child.orphaned)
            .map(|child| format!("{}:{}", child.phase, child.node_id))
            .collect(),
        removed_ticket_refs: cleanup_observation.removed_ticket_refs,
        remaining_ticket_paths: cleanup_observation.remaining_ticket_paths,
        cleanup_succeeded: cleanup_observation.succeeded && phases.is_stop_passed,
        caveats: planned.caveats.clone(),
    }
}

fn local_executable_run(
    planned: &PlannedRun,
    node_artifacts: &[NodeArtifacts],
    cleanup_input: &ClusterHarnessCleanupInput,
    cleanup_ref: &str,
    phases: &LifecyclePhases,
) -> crate::error::Result<crate::multinode_core::LocalMultiprocessExecutableRunReceipt> {
    crate::multinode_core::build_local_multiprocess_executable_run(
        &crate::multinode_core::LocalMultiprocessExecutableRunInput {
            plan: planned.local_plan_input.clone(),
            startup_refs: node_artifacts.iter().filter_map(|node| node.startup_ref.clone()).collect(),
            workflow_refs: node_artifacts
                .iter()
                .flat_map(|node| [node.workflow_ref.clone(), node.heartbeat_ref.clone()])
                .flatten()
                .collect(),
            shutdown_refs: node_artifacts.iter().filter_map(|node| node.shutdown_ref.clone()).collect(),
            cleanup_refs: vec![cleanup_ref.to_string()],
            ticket_status: TICKET_STATUS_CURRENT.to_string(),
            child_timed_out: phases.child_executions.iter().any(|child| child.timed_out),
            orphaned_processes: cleanup_input.orphaned_processes.clone(),
            cleanup_succeeded: cleanup_input.cleanup_succeeded,
            diagnostics: phases.diagnostics.clone(),
            caveats: planned.caveats.clone(),
        },
    )
}

/// Builds and records the parent receipt, writes the artifacts, run index, and verification, and
/// writes a failure repro bundle when the parent or the verification denies.
fn finish_run(
    input: &ClusterHarnessExecutionInput,
    evidence: RunEvidence,
) -> crate::error::Result<ClusterHarnessExecution> {
    let RunEvidence {
        plan: mut planned,
        child_executions,
        mut diagnostics,
        child_process_refs,
        child_receipt_refs,
        cleanup_ref,
        lifecycle_ref,
        drift_ref,
        local_plan_ref,
        local_run_ref,
    } = evidence;
    let diagnostic_log_refs = child_executions
        .iter()
        .map(|child| child_log_ref(child, &input.output_directory, &planned.plan))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let observed_kinds = observed_artifact_kinds(&planned.artifacts);
    diagnostics.sort();
    diagnostics.dedup();
    let parent = build_cluster_harness_parent(&ClusterHarnessParentInput {
        fixture_ref: planned.fixture_ref.clone(),
        command_plan_ref: planned.command_plan_ref.clone(),
        local_plan_ref: local_plan_ref.clone(),
        local_run_ref,
        lifecycle_ref: lifecycle_ref.clone(),
        drift_summary_ref: drift_ref.clone(),
        cleanup_ref,
        child_receipt_refs: child_receipt_refs.clone(),
        diagnostic_log_refs: diagnostic_log_refs.clone(),
        observed_artifact_kinds: observed_kinds,
        required_artifact_kinds: planned.expected_kinds,
        unsupported_pass_claim: false,
        diagnostics,
        caveats: planned.caveats,
    })?;
    push_artifact(&mut planned.artifacts, PARENT_RUN_FILE, CLUSTER_RUN_KIND, parent.value.clone())?;
    let verification = write_run_index_and_verification(input, planned.artifacts, &planned.plan)?;

    let is_passed = parent.decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS
        && verification.decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS;
    let failure_bundle_ref = if is_passed {
        None
    } else {
        Some(write_failure_bundle(input, &crate::multinode_core::FailureReproBundleInput {
            scenario_fixture_ref: planned.fixture_ref,
            topology_ref: local_plan_ref,
            scheduler_ref: planned.command_plan_ref,
            fault_plan_ref: parent.receipt_ref.clone(),
            command_refs: child_process_refs,
            node_summary_refs: vec![lifecycle_ref, drift_ref],
            receipt_refs: child_receipt_refs,
            diagnostic_refs: diagnostic_log_refs.clone(),
            log_refs: diagnostic_log_refs,
            ..sealed_failure_bundle_defaults()
        })?)
    };
    let decision = if is_passed {
        molten_core::cluster_harness::RUN_DIRECTORY_PASS
    } else {
        molten_core::cluster_harness::RUN_DIRECTORY_DENY
    };
    Ok(ClusterHarnessExecution {
        decision: decision.to_string(),
        parent_ref: parent.receipt_ref,
        verification_ref: verification.verification_ref,
        failure_bundle_ref,
        diagnostics: sorted_union(parent.diagnostics, &verification.diagnostics),
        output_directory: input.output_directory.clone(),
    })
}

/// The sorted, deduplicated kinds of the prepared artifacts plus the parent run kind itself.
fn observed_artifact_kinds(artifacts: &[PreparedArtifact]) -> Vec<String> {
    let mut observed_kinds = artifacts.iter().map(|artifact| artifact.entry.artifact_kind.clone()).collect::<Vec<_>>();
    observed_kinds.push(CLUSTER_RUN_KIND.to_string());
    observed_kinds.sort();
    observed_kinds.dedup();
    observed_kinds
}

/// `first` extended with `second`, sorted and deduplicated.
fn sorted_union(mut first: Vec<String>, second: &[String]) -> Vec<String> {
    first.extend(second.iter().cloned());
    first.sort();
    first.dedup();
    first
}

/// The fixed seed, redaction, replay, and sealing fields of a harness failure repro bundle; the
/// refs are empty and filled in by the caller.
fn sealed_failure_bundle_defaults() -> crate::multinode_core::FailureReproBundleInput {
    crate::multinode_core::FailureReproBundleInput {
        scenario_fixture_ref: String::new(),
        topology_ref: String::new(),
        scheduler_ref: String::new(),
        seed_ref: content_ref_for_text(COMMAND_PROFILE_DOMAIN, "no-ambient-randomness"),
        fault_plan_ref: String::new(),
        command_refs: Vec::new(),
        node_summary_refs: Vec::new(),
        receipt_refs: Vec::new(),
        diagnostic_refs: Vec::new(),
        log_refs: Vec::new(),
        redaction_policy_ref: content_ref_for_text(COMMAND_PROFILE_DOMAIN, "public-diagnostics-no-private-attachments"),
        replay_status: "non-replayable-local-process-observation".to_string(),
        diagnostic_only: true,
        sealed: true,
        private_attachment_refs: Vec::new(),
        reveal_receipt_refs: Vec::new(),
        claimed_payload_ref: None,
        caveats: cluster_harness_caveats(),
    }
}

/// Writes the prepared artifacts, appends their log entries, and writes the run index and its
/// verification.
fn write_run_index_and_verification(
    input: &ClusterHarnessExecutionInput,
    artifacts: Vec<PreparedArtifact>,
    plan: &crate::cluster::ClusterPlan,
) -> crate::error::Result<ClusterRunVerificationReceipt> {
    write_prepared_artifacts(&input.output_directory, &artifacts)?;
    let entries =
        append_log_entries(&input.output_directory, artifacts.into_iter().map(|item| item.entry).collect(), plan)?;
    let index_text = render_run_index(&entries);
    let index_path = input.output_directory.join(RUN_INDEX_FILE);
    std::fs::write(&index_path, &index_text).map_err(crate::error::MoltenError::from)?;
    let index_ref = content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &index_text);
    let assessment = assess_indexed_run_directory(&input.output_directory, &entries);
    let verification = cluster_run_verification_value(&index_ref, &assessment)?;
    write_preserves_path(&input.output_directory.join(VERIFICATION_FILE), &verification.value)?;
    Ok(verification)
}

/// Builds, verifies, and writes the sealed diagnostic failure repro bundle, returning its ref.
fn write_failure_bundle(
    input: &ClusterHarnessExecutionInput,
    failure_input: &crate::multinode_core::FailureReproBundleInput,
) -> crate::error::Result<String> {
    let bundle = crate::multinode_core::build_failure_repro_bundle(failure_input)?;
    let bundle_verification = crate::multinode_core::verify_failure_repro_bundle(failure_input)?;
    write_preserves_path(&input.output_directory.join(FAILURE_BUNDLE_FILE), &bundle.value)?;
    write_preserves_path(&input.output_directory.join(FAILURE_BUNDLE_VERIFICATION_FILE), &bundle_verification.value)?;
    Ok(bundle.bundle_ref)
}
