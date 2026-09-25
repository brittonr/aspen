
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
