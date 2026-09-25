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

    let mut child_executions = Vec::new();
    let mut diagnostics = Vec::new();
    let is_init_passed = execute_phase_for_nodes(
        PhaseStep {
            input,
            plan: &plan,
            phase: "init",
        },
        &mut child_executions,
        &mut artifacts,
        |node| {
            vec![
                std::ffi::OsString::from("node"),
                std::ffi::OsString::from("init"),
                std::ffi::OsString::from("--state-root"),
                node.state_root.as_os_str().to_os_string(),
                std::ffi::OsString::from("--node-id"),
                std::ffi::OsString::from(&node.node_id),
            ]
        },
    )?;
    let is_start_passed = if is_init_passed {
        execute_phase_for_nodes(
            PhaseStep {
                input,
                plan: &plan,
                phase: "start",
            },
            &mut child_executions,
            &mut artifacts,
            |node| {
                vec![
                    std::ffi::OsString::from("node"),
                    std::ffi::OsString::from("run"),
                    std::ffi::OsString::from("--state-root"),
                    node.state_root.as_os_str().to_os_string(),
                ]
            },
        )?
    } else {
        diagnostics.push("cluster-harness-start-skipped-after-init-failure".to_string());
        false
    };
    let is_workflow_passed = if is_start_passed {
        execute_phase_for_nodes(
            PhaseStep {
                input,
                plan: &plan,
                phase: "workflow",
            },
            &mut child_executions,
            &mut artifacts,
            |node| {
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
            },
        )?
    } else {
        diagnostics.push("cluster-harness-workflow-skipped-after-start-failure".to_string());
        false
    };
    let is_status_passed = if is_workflow_passed {
        execute_phase_for_nodes(
            PhaseStep {
                input,
                plan: &plan,
                phase: "status",
            },
            &mut child_executions,
            &mut artifacts,
            |node| {
                vec![
                    std::ffi::OsString::from("node"),
                    std::ffi::OsString::from("status"),
                    std::ffi::OsString::from("--state-root"),
                    node.state_root.as_os_str().to_os_string(),
                ]
            },
        )?
    } else {
        diagnostics.push("cluster-harness-status-skipped-after-workflow-failure".to_string());
        false
    };

    let is_stop_passed = if is_start_passed {
        execute_phase_for_nodes_reverse(
            PhaseStep {
                input,
                plan: &plan,
                phase: "stop",
            },
            &mut child_executions,
            &mut artifacts,
            |node| {
                vec![
                    std::ffi::OsString::from("node"),
                    std::ffi::OsString::from("stop"),
                    std::ffi::OsString::from("--state-root"),
                    node.state_root.as_os_str().to_os_string(),
                ]
            },
        )?
    } else {
        true
    };

    collect_child_diagnostics(&child_executions, &mut diagnostics);
    let mut child_receipt_refs = Vec::new();
    let node_artifacts = plan
        .nodes
        .iter()
        .map(|node| capture_node_artifacts(node, &mut artifacts, &mut child_receipt_refs))
        .collect::<crate::error::Result<Vec<_>>>()?;

    let cleanup_observation = cleanup_state_roots(&plan)?;
    let child_process_refs = child_executions.iter().map(|child| child.process_ref.clone()).collect::<Vec<_>>();
    child_receipt_refs.extend(child_process_refs.iter().cloned());
    child_receipt_refs.sort();
    child_receipt_refs.dedup();
    let cleanup_input = ClusterHarnessCleanupInput {
        child_process_refs: child_process_refs.clone(),
        stopped_node_ids: if is_stop_passed {
            plan.nodes.iter().rev().map(|node| node.node_id.clone()).collect()
        } else {
            Vec::new()
        },
        orphaned_processes: child_executions
            .iter()
            .filter(|child| child.orphaned)
            .map(|child| format!("{}:{}", child.phase, child.node_id))
            .collect(),
        removed_ticket_refs: cleanup_observation.removed_ticket_refs,
        remaining_ticket_paths: cleanup_observation.remaining_ticket_paths,
        cleanup_succeeded: cleanup_observation.succeeded && is_stop_passed,
        caveats: caveats.clone(),
    };
    let cleanup = cleanup_value(&cleanup_input)?;
    let cleanup_ref = crate::preserves_rail::canonical_hash(&cleanup)?;
    push_artifact(&mut artifacts, CLEANUP_FILE, CLEANUP_KIND, cleanup)?;

    let lifecycle = build_lifecycle_artifacts(LifecycleArtifactsInput {
        fixture_ref: &fixture_ref,
        node_ids: &node_ids,
        nodes: &node_artifacts,
        child_executions: &child_executions,
        diagnostics: &diagnostics,
        phases_passed: is_init_passed && is_start_passed && is_workflow_passed && is_status_passed && is_stop_passed,
        caveats: &caveats,
    })?;
    push_artifact(&mut artifacts, LIFECYCLE_FILE, CLUSTER_LIFECYCLE_KIND, lifecycle.lifecycle_value)?;
    push_artifact(&mut artifacts, DRIFT_SUMMARY_FILE, DRIFT_SUMMARY_KIND, lifecycle.drift_value)?;

    let local_executable = crate::multinode_core::build_local_multiprocess_executable_run(
        &crate::multinode_core::LocalMultiprocessExecutableRunInput {
            plan: local_plan_input,
            startup_refs: node_artifacts.iter().filter_map(|node| node.startup_ref.clone()).collect(),
            workflow_refs: node_artifacts
                .iter()
                .flat_map(|node| [node.workflow_ref.clone(), node.heartbeat_ref.clone()])
                .flatten()
                .collect(),
            shutdown_refs: node_artifacts.iter().filter_map(|node| node.shutdown_ref.clone()).collect(),
            cleanup_refs: vec![cleanup_ref.clone()],
            ticket_status: TICKET_STATUS_CURRENT.to_string(),
            child_timed_out: child_executions.iter().any(|child| child.timed_out),
            orphaned_processes: cleanup_input.orphaned_processes.clone(),
            cleanup_succeeded: cleanup_input.cleanup_succeeded,
            diagnostics: diagnostics.clone(),
            caveats: caveats.clone(),
        },
    )?;
    if local_executable.decision != molten_core::cluster_harness::RUN_DIRECTORY_PASS {
        diagnostics.extend(local_executable.diagnostics.iter().map(|item| format!("local-run:{item}")));
    }
    push_artifact(&mut artifacts, LOCAL_EXECUTABLE_RUN_FILE, LOCAL_EXECUTABLE_RUN_KIND, local_executable.value)?;

    let diagnostic_log_refs = child_executions
        .iter()
        .map(|child| child_log_ref(child, &input.output_directory, &plan))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let mut observed_kinds = artifacts.iter().map(|artifact| artifact.entry.artifact_kind.clone()).collect::<Vec<_>>();
    observed_kinds.push(CLUSTER_RUN_KIND.to_string());
    observed_kinds.sort();
    observed_kinds.dedup();
    diagnostics.sort();
    diagnostics.dedup();
    let parent = build_cluster_harness_parent(&ClusterHarnessParentInput {
        fixture_ref: fixture_ref.clone(),
        command_plan_ref: command_plan_ref.clone(),
        local_plan_ref: local_executable.plan_ref.clone(),
        local_run_ref: local_executable.executable_ref.clone(),
        lifecycle_ref: lifecycle.lifecycle_ref.clone(),
        drift_summary_ref: lifecycle.drift_ref.clone(),
        cleanup_ref,
        child_receipt_refs: child_receipt_refs.clone(),
        diagnostic_log_refs: diagnostic_log_refs.clone(),
        observed_artifact_kinds: observed_kinds,
        required_artifact_kinds: expected_kinds,
        unsupported_pass_claim: false,
        diagnostics,
        caveats,
    })?;
    push_artifact(&mut artifacts, PARENT_RUN_FILE, CLUSTER_RUN_KIND, parent.value.clone())?;

    write_prepared_artifacts(&input.output_directory, &artifacts)?;
    let entries =
        append_log_entries(&input.output_directory, artifacts.into_iter().map(|item| item.entry).collect(), &plan)?;
    let index_text = render_run_index(&entries);
    let index_path = input.output_directory.join(RUN_INDEX_FILE);
    std::fs::write(&index_path, &index_text).map_err(crate::error::MoltenError::from)?;
    let index_ref = content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &index_text);
    let assessment = assess_indexed_run_directory(&input.output_directory, &entries);
    let verification = cluster_run_verification_value(&index_ref, &assessment)?;
    write_preserves_path(&input.output_directory.join(VERIFICATION_FILE), &verification.value)?;

    let mut failure_bundle_ref = None;
    if parent.decision != molten_core::cluster_harness::RUN_DIRECTORY_PASS
        || verification.decision != molten_core::cluster_harness::RUN_DIRECTORY_PASS
    {
        let failure_input = crate::multinode_core::FailureReproBundleInput {
            scenario_fixture_ref: fixture_ref,
            topology_ref: local_executable.plan_ref,
            scheduler_ref: command_plan_ref,
            seed_ref: content_ref_for_text(COMMAND_PROFILE_DOMAIN, "no-ambient-randomness"),
            fault_plan_ref: parent.receipt_ref.clone(),
            command_refs: child_process_refs,
            node_summary_refs: vec![lifecycle.lifecycle_ref, lifecycle.drift_ref],
            receipt_refs: child_receipt_refs,
            diagnostic_refs: diagnostic_log_refs.clone(),
            log_refs: diagnostic_log_refs,
            redaction_policy_ref: content_ref_for_text(
                COMMAND_PROFILE_DOMAIN,
                "public-diagnostics-no-private-attachments",
            ),
            replay_status: "non-replayable-local-process-observation".to_string(),
            diagnostic_only: true,
            sealed: true,
            private_attachment_refs: Vec::new(),
            reveal_receipt_refs: Vec::new(),
            claimed_payload_ref: None,
            caveats: cluster_harness_caveats(),
        };
        let bundle = crate::multinode_core::build_failure_repro_bundle(&failure_input)?;
        let bundle_verification = crate::multinode_core::verify_failure_repro_bundle(&failure_input)?;
        write_preserves_path(&input.output_directory.join(FAILURE_BUNDLE_FILE), &bundle.value)?;
        write_preserves_path(
            &input.output_directory.join(FAILURE_BUNDLE_VERIFICATION_FILE),
            &bundle_verification.value,
        )?;
        failure_bundle_ref = Some(bundle.bundle_ref);
    }

    let decision = if parent.decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS
        && verification.decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS
    {
        molten_core::cluster_harness::RUN_DIRECTORY_PASS.to_string()
    } else {
        molten_core::cluster_harness::RUN_DIRECTORY_DENY.to_string()
    };
    let mut final_diagnostics = parent.diagnostics;
    final_diagnostics.extend(verification.diagnostics.clone());
    final_diagnostics.sort();
    final_diagnostics.dedup();
    Ok(ClusterHarnessExecution {
        decision,
        parent_ref: parent.receipt_ref,
        verification_ref: verification.verification_ref,
        failure_bundle_ref,
        diagnostics: final_diagnostics,
        output_directory: input.output_directory.clone(),
    })
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
pub fn verify_cluster_run_directory(
    run_directory: &std::path::Path,
) -> crate::error::Result<ClusterRunDirectoryVerification> {
    let index_path = run_directory.join(RUN_INDEX_FILE);
    let index_text = std::fs::read_to_string(&index_path).map_err(crate::error::MoltenError::from)?;
    let entries = parse_run_index(&index_text)?;
    let index_ref = content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &index_text);
    let mut assessment = assess_indexed_run_directory(run_directory, &entries);
    let expected_receipt = cluster_run_verification_value(&index_ref, &assessment)?;
    let companion_path = run_directory.join(VERIFICATION_FILE);
    if assessment.decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS {
        match read_preserves_path(&companion_path) {
            Ok(companion) if companion == expected_receipt.value => {}
            Ok(_) => add_verification_companion_diagnostic(
                &mut assessment,
                "cluster-run-verification-companion-mismatch",
                "mismatched",
            ),
            Err(_) => add_verification_companion_diagnostic(
                &mut assessment,
                "cluster-run-verification-companion-missing",
                "missing",
            ),
        }
    }
    let receipt = cluster_run_verification_value(&index_ref, &assessment)?;
    Ok(ClusterRunDirectoryVerification {
        decision: receipt.decision.clone(),
        index_ref,
        receipt,
    })
}

fn validate_execution_input(input: &ClusterHarnessExecutionInput) -> crate::error::Result<()> {
    if input.child_timeout_ms == 0 || input.child_timeout_ms > MAX_CLUSTER_CHILD_TIMEOUT_MS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cluster child timeout must be between 1 and {MAX_CLUSTER_CHILD_TIMEOUT_MS} milliseconds"
        )));
    }
    for (label, path) in [
        ("fixture", &input.fixture_path),
        ("state root", &input.state_root),
        ("output directory", &input.output_directory),
        ("node binary", &input.node_binary),
    ] {
        if path.as_os_str().is_empty() {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "cluster harness requires explicit {label}"
            )));
        }
    }
    if input.state_root == input.output_directory
        || input.state_root.starts_with(&input.output_directory)
        || input.output_directory.starts_with(&input.state_root)
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "cluster harness state root and output directory must be isolated",
        ));
    }
    Ok(())
}

fn prepare_output_roots(input: &ClusterHarnessExecutionInput) -> crate::error::Result<()> {
    for path in [&input.state_root, &input.output_directory] {
        if path.exists() {
            if !input.force {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "cluster harness path already exists: {}; pass --force to replace it",
                    path.display()
                )));
            }
            std::fs::remove_dir_all(path).map_err(crate::error::MoltenError::from)?;
        }
        std::fs::create_dir_all(path).map_err(crate::error::MoltenError::from)?;
    }
    Ok(())
}

fn expected_artifact_kinds() -> Vec<String> {
    molten_core::cluster_harness::REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS
        .iter()
        .map(|kind| (*kind).to_string())
        .collect()
}

fn cluster_harness_caveats() -> Vec<String> {
    vec![
        "local multiprocess evidence is not VM or production evidence".to_string(),
        "diagnostic logs are adjuncts and cannot replace canonical receipts".to_string(),
        "node lifecycle observations do not establish distributed consensus or live transport correctness".to_string(),
    ]
}

fn local_plan_input(
    plan: &crate::cluster::ClusterPlan,
    fixture_ref: &str,
    command_plan_ref: &str,
    expected_kinds: &[String],
    caveats: &[String],
) -> crate::multinode_core::LocalMultiprocessPlanInput {
    crate::multinode_core::LocalMultiprocessPlanInput {
        fixture_ref: fixture_ref.to_string(),
        nodes: plan
            .nodes
            .iter()
            .map(|node| crate::multinode_core::LocalProcessNodePlan {
                node_id: node.node_id.clone(),
                state_root_handle: format!("state-root:{}", node.path_component),
                transport_handle: format!("local-process:{}", node.path_component),
            })
            .collect(),
        command_plan_ref: command_plan_ref.to_string(),
        expected_receipt_refs: expected_kinds
            .iter()
            .map(|kind| content_ref_for_text(EXPECTED_ARTIFACT_DOMAIN, kind))
            .collect(),
        cleanup_policy: CLEANUP_POLICY.to_string(),
        caveats: caveats.to_vec(),
    }
}

#[derive(Clone, Copy)]
struct PhaseStep<'a> {
    input: &'a ClusterHarnessExecutionInput,
    plan: &'a crate::cluster::ClusterPlan,
    phase: &'a str,
}

fn execute_phase_for_nodes<F>(
    step: PhaseStep<'_>,
    executions: &mut impl crate::bounded::VecSink<ChildExecution>,
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
    arguments: F,
) -> crate::error::Result<bool>
where
    F: Fn(&crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString>,
{
    let PhaseStep { input, plan, phase } = step;
    let mut is_passed = true;
    for node in &plan.nodes {
        let execution = execute_child(input, node, phase, arguments(node))?;
        is_passed &= execution.succeeded;
        push_artifact(
            artifacts,
            &format!("children/processes/{phase}-{}.preserves", node.path_component),
            CHILD_PROCESS_KIND,
            execution.value.clone(),
        )?;
        executions.push_item(execution);
    }
    Ok(is_passed)
}

fn execute_phase_for_nodes_reverse<F>(
    step: PhaseStep<'_>,
    executions: &mut impl crate::bounded::VecSink<ChildExecution>,
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
    arguments: F,
) -> crate::error::Result<bool>
where
    F: Fn(&crate::cluster::ClusterNodePlan) -> Vec<std::ffi::OsString>,
{
    let PhaseStep { input, plan, phase } = step;
    let mut is_passed = true;
    for node in plan.nodes.iter().rev() {
        let execution = execute_child(input, node, phase, arguments(node))?;
        is_passed &= execution.succeeded;
        push_artifact(
            artifacts,
            &format!("children/processes/{phase}-{}.preserves", node.path_component),
            CHILD_PROCESS_KIND,
            execution.value.clone(),
        )?;
        executions.push_item(execution);
    }
    Ok(is_passed)
}

fn execute_child(
    input: &ClusterHarnessExecutionInput,
    node: &crate::cluster::ClusterNodePlan,
    phase: &str,
    arguments: Vec<std::ffi::OsString>,
) -> crate::error::Result<ChildExecution> {
    let mut command = std::process::Command::new(&input.node_binary);
    command.args(&arguments).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return finalize_child_execution(input, node, phase, ChildProcessObservation {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("spawn failed: {error}"),
                timed_out: false,
                orphaned: false,
                succeeded: false,
            });
        }
    };
    let mut deadline =
        crate::fabric_time::SupervisionDeadline::after(std::time::Duration::from_millis(input.child_timeout_ms))?;
    let mut is_timed_out = false;
    let mut is_orphaned = false;
    let mut process_error = None;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                process_error = Some(format!("child status failed: {error}"));
                if child.kill().is_err() {
                    is_orphaned = true;
                }
                break;
            }
        }
        if deadline.is_expired()? {
            is_timed_out = true;
            if child.kill().is_err() {
                is_orphaned = true;
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(CHILD_POLL_INTERVAL_MS));
    }
    match child.wait_with_output() {
        Ok(output) => {
            let is_process_status_ok = process_error.is_none();
            let stderr = process_error.map_or_else(
                || String::from_utf8_lossy(&output.stderr).into_owned(),
                |error| format!("{error}\n{}", String::from_utf8_lossy(&output.stderr)),
            );
            finalize_child_execution(input, node, phase, ChildProcessObservation {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr,
                timed_out: is_timed_out,
                orphaned: is_orphaned,
                succeeded: output.status.success() && !is_timed_out && !is_orphaned && is_process_status_ok,
            })
        }
        Err(error) => finalize_child_execution(input, node, phase, ChildProcessObservation {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("child wait failed: {error}"),
            timed_out: is_timed_out,
            orphaned: true,
            succeeded: false,
        }),
    }
}

struct ChildProcessObservation {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    orphaned: bool,
    succeeded: bool,
}

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
            node_summaries: node_ids
                .iter()
                .zip(nodes)
                .map(|(node_id, node)| {
                    let config_ref = node.config_ref.clone().ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("complete cluster node has no config ref")
                    })?;
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
                .collect::<crate::error::Result<Vec<_>>>()?,
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

fn collect_ticket_paths(
    root: &std::path::Path,
    current: &std::path::Path,
    paths: &mut impl crate::bounded::VecSink<std::path::PathBuf>,
) -> crate::error::Result<()> {
    if !current.exists() {
        return Ok(());
    }
    let mut pending = vec![current.to_path_buf()];
    while let Some(next) = pending.pop() {
        if paths.item_count() >= MAX_TICKET_FILES {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "cluster harness ticket file count exceeds bound {MAX_TICKET_FILES} under {}",
                root.display()
            )));
        }
        if next.is_dir() {
            let entries = std::fs::read_dir(&next)
                .map_err(crate::error::MoltenError::from)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(crate::error::MoltenError::from)?;
            // Push in reverse so the stack keeps the depth-first order of the
            // earlier recursion.
            for entry in entries.into_iter().rev() {
                pending.push(entry.path());
            }
        } else if next.is_file()
            && next.file_name().is_some_and(|name| name.to_string_lossy().to_ascii_lowercase().contains("ticket"))
        {
            paths.push_item(next);
        }
    }
    Ok(())
}

fn push_artifact(
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
    relative_path: &str,
    kind: &str,
    value: IoValue,
) -> crate::error::Result<()> {
    let expected_ref = crate::preserves_rail::canonical_hash(&value)?;
    artifacts.push_item(PreparedArtifact {
        entry: molten_core::cluster_harness::RunArtifactIndexEntry {
            relative_path: relative_path.to_string(),
            artifact_kind: kind.to_string(),
            expected_ref,
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
        },
        value,
    });
    Ok(())
}

fn write_prepared_artifacts(
    output_directory: &std::path::Path,
    artifacts: &[PreparedArtifact],
) -> crate::error::Result<()> {
    for artifact in artifacts {
        write_preserves_path(&output_directory.join(&artifact.entry.relative_path), &artifact.value)?;
    }
    Ok(())
}

fn append_log_entries(
    output_directory: &std::path::Path,
    mut entries: Vec<molten_core::cluster_harness::RunArtifactIndexEntry>,
    plan: &crate::cluster::ClusterPlan,
) -> crate::error::Result<Vec<molten_core::cluster_harness::RunArtifactIndexEntry>> {
    for phase in ["init", "start", "workflow", "status", "stop"] {
        for node in &plan.nodes {
            let relative_path = format!("logs/{phase}-{}.log", node.path_component);
            let path = output_directory.join(&relative_path);
            if !path.exists() {
                continue;
            }
            let text = std::fs::read_to_string(path).map_err(crate::error::MoltenError::from)?;
            entries.push(molten_core::cluster_harness::RunArtifactIndexEntry {
                relative_path,
                artifact_kind: DIAGNOSTIC_LOG_KIND.to_string(),
                expected_ref: content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text),
                format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
            });
        }
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn child_log_ref(
    child: &ChildExecution,
    output_directory: &std::path::Path,
    plan: &crate::cluster::ClusterPlan,
) -> crate::error::Result<String> {
    let component = plan
        .nodes
        .iter()
        .find(|node| node.node_id == child.node_id)
        .map(|node| node.path_component.as_str())
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness(format!("missing cluster plan node {}", child.node_id))
        })?;
    let text = std::fs::read_to_string(output_directory.join(format!("logs/{}-{component}.log", child.phase)))
        .map_err(crate::error::MoltenError::from)?;
    Ok(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text))
}

fn render_run_index(entries: &[molten_core::cluster_harness::RunArtifactIndexEntry]) -> String {
    let mut output = String::from(RUN_INDEX_HEADER);
    output.push('\n');
    for entry in entries {
        output.push_str(&entry.relative_path);
        output.push('\t');
        output.push_str(&entry.artifact_kind);
        output.push('\t');
        output.push_str(&entry.expected_ref);
        output.push('\t');
        output.push_str(&entry.format);
        output.push('\n');
    }
    output
}

pub(super) fn parse_run_index(
    source: &str,
) -> crate::error::Result<Vec<molten_core::cluster_harness::RunArtifactIndexEntry>> {
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("cluster run index is empty"))?;
    if header != RUN_INDEX_HEADER {
        return Err(crate::error::MoltenError::invalid_harness("cluster run index has unsupported header"));
    }
    let mut entries = Vec::new();
    for (line_index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != RUN_INDEX_FIELD_COUNT {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "cluster run index line {} must have four tab-separated fields",
                line_index.saturating_add(RUN_INDEX_ENTRY_LINE_OFFSET)
            )));
        }
        crate::bounded::push_bounded(
            &mut entries,
            molten_core::cluster_harness::RunArtifactIndexEntry {
                relative_path: fields[0].to_string(),
                artifact_kind: fields[1].to_string(),
                expected_ref: fields[RUN_INDEX_REF_FIELD].to_string(),
                format: fields[RUN_INDEX_FORMAT_FIELD].to_string(),
            },
            MAX_RUN_INDEX_ENTRIES,
            "cluster run index entry",
        )?;
    }
    Ok(entries)
}

fn assess_indexed_run_directory(
    run_directory: &std::path::Path,
    entries: &[molten_core::cluster_harness::RunArtifactIndexEntry],
) -> molten_core::cluster_harness::RunDirectoryAssessment {
    let mut observations =
        entries.iter().map(|entry| observe_indexed_artifact(run_directory, entry)).collect::<Vec<_>>();
    let indexed_paths =
        entries.iter().map(|entry| entry.relative_path.as_str()).collect::<std::collections::BTreeSet<_>>();
    if let Ok(files) = collect_run_files(run_directory) {
        for relative_path in files {
            if indexed_paths.contains(relative_path.as_str()) || allowed_unindexed_file(&relative_path) {
                continue;
            }
            observations.push(observe_unexpected_artifact(run_directory, &relative_path));
        }
    }
    molten_core::cluster_harness::assess_run_directory(entries, &observations)
}

fn observe_indexed_artifact(
    run_directory: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    let path = run_directory.join(&entry.relative_path);
    match entry.format.as_str() {
        molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES => observe_preserves_artifact(&path, entry),
        molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT => observe_text_artifact(&path, entry),
        _ => missing_observation(entry),
    }
}

fn observe_preserves_artifact(
    path: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    if !is_regular_file_without_symlink(path) {
        return missing_observation(entry);
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return missing_observation(entry);
    };
    let Ok(value) = crate::preserves_rail::parse_text(&text) else {
        return molten_core::cluster_harness::RunArtifactObservation {
            relative_path: entry.relative_path.clone(),
            artifact_kind: entry.artifact_kind.clone(),
            observed_ref: None,
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
            canonical: false,
            pass_eligible: false,
        };
    };
    let observed_ref = crate::preserves_rail::canonical_hash(&value).ok();
    let actual_kind = crate::ledger::artifact_kind(&value).to_string();
    let is_canonical = crate::preserves_rail::to_text(&value).is_ok_and(|rendered| rendered == text);
    let is_pass_eligible = artifact_decision(&value, &actual_kind)
        .ok()
        .flatten()
        .is_none_or(|decision| decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS);
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: entry.relative_path.clone(),
        artifact_kind: actual_kind,
        observed_ref,
        format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
        canonical: is_canonical,
        pass_eligible: is_pass_eligible,
    }
}

fn observe_text_artifact(
    path: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    if !is_regular_file_without_symlink(path) {
        return missing_observation(entry);
    }
    match std::fs::read_to_string(path) {
        Ok(text) => molten_core::cluster_harness::RunArtifactObservation {
            relative_path: entry.relative_path.clone(),
            artifact_kind: entry.artifact_kind.clone(),
            observed_ref: Some(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text)),
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
            canonical: true,
            pass_eligible: true,
        },
        Err(_) => missing_observation(entry),
    }
}

fn missing_observation(
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: entry.relative_path.clone(),
        artifact_kind: entry.artifact_kind.clone(),
        observed_ref: None,
        format: entry.format.clone(),
        canonical: false,
        pass_eligible: false,
    }
}

fn observe_unexpected_artifact(
    run_directory: &std::path::Path,
    relative_path: &str,
) -> molten_core::cluster_harness::RunArtifactObservation {
    let path = run_directory.join(relative_path);
    if relative_path.ends_with(".preserves")
        && let Ok(value) = read_preserves_path(&path)
    {
        return molten_core::cluster_harness::RunArtifactObservation {
            relative_path: relative_path.to_string(),
            artifact_kind: crate::ledger::artifact_kind(&value).to_string(),
            observed_ref: crate::preserves_rail::canonical_hash(&value).ok(),
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
            canonical: true,
            pass_eligible: false,
        };
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: relative_path.to_string(),
        artifact_kind: "unexpected".to_string(),
        observed_ref: Some(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text)),
        format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
        canonical: true,
        pass_eligible: false,
    }
}

fn collect_run_files(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let mut files = Vec::new();
    collect_run_files_from(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_run_files_from(
    root: &std::path::Path,
    current: &std::path::Path,
    files: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    let mut pending = vec![current.to_path_buf()];
    while let Some(next) = pending.pop() {
        if next.is_dir() {
            for entry in std::fs::read_dir(&next).map_err(crate::error::MoltenError::from)? {
                let entry = entry.map_err(crate::error::MoltenError::from)?;
                let file_type = entry.file_type().map_err(crate::error::MoltenError::from)?;
                if file_type.is_dir() {
                    pending.push(entry.path());
                } else if file_type.is_file() || file_type.is_symlink() {
                    let relative = entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| crate::error::MoltenError::invalid_harness("cluster run file escaped root"))?
                        .to_string_lossy()
                        .replace(std::path::MAIN_SEPARATOR, "/");
                    files.push_item(relative);
                }
            }
        }
    }
    Ok(())
}

fn allowed_unindexed_file(relative_path: &str) -> bool {
    matches!(
        relative_path,
        RUN_INDEX_FILE | VERIFICATION_FILE | FAILURE_BUNDLE_FILE | FAILURE_BUNDLE_VERIFICATION_FILE
    )
}

fn add_verification_companion_diagnostic(
    assessment: &mut molten_core::cluster_harness::RunDirectoryAssessment,
    diagnostic: &str,
    observed: &str,
) {
    assessment.decision = molten_core::cluster_harness::RUN_DIRECTORY_DENY.to_string();
    assessment.diagnostics.push(diagnostic.to_string());
    assessment.diagnostics.sort();
    assessment.diagnostics.dedup();
    assessment.first_divergence = Some(molten_core::cluster_harness::FirstDivergence {
        relative_path: VERIFICATION_FILE.to_string(),
        artifact_kind: VERIFICATION_KIND.to_string(),
        expected: "matching-verification-receipt".to_string(),
        observed: observed.to_string(),
        reason: diagnostic.to_string(),
        diagnostic_only: true,
    });
}

fn write_preserves_path(path: &std::path::Path, value: &IoValue) -> crate::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::write(path, crate::preserves_rail::to_text(value)?).map_err(crate::error::MoltenError::from)
}

fn read_preserves_path(path: &std::path::Path) -> crate::error::Result<IoValue> {
    if !is_regular_file_without_symlink(path) {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "expected regular non-symlink Preserves file at {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(crate::error::MoltenError::from)?;
    crate::preserves_rail::parse_text(&text)
}

fn is_regular_file_without_symlink(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
}
