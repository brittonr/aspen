
fn stale_marker_denial(dir: &std::path::Path, base: &BaseOutputs, bundles: &BundleChecks) -> CliResult<()> {
    std::fs::write(dir.join("after-nextest.txt"), "/nix/store/stale-molten-nextest\n")?;
    let stale_verify = dir.join("nix-dogfood-verify-stale.preserves");
    let verify_stale = molten_cmd()
        .args(["dogfood", "nix-release-verify", "--output-path"])
        .arg(dir)
        .args(["--evidence"])
        .arg(&base.nix_evidence)
        .args(["--receipt-out"])
        .arg(&stale_verify)
        .output()?;
    assert_success(&verify_stale, "dogfood nix-release-verify stale marker");
    let stale_receipt = molten::operator_dogfood::parse_nix_dogfood_verify_receipt(&read_preserves(&stale_verify)?)?;
    assert_eq!(stale_receipt.decision, "deny");
    assert!(
        stale_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("nextest-marker-ref mismatch"))
    );
    let stale_bundle_verify_path = dir.join("release-evidence-bundle-verify-stale.preserves");
    let verify_stale_bundle = molten_cmd()
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&bundles.path)
        .args(["--receipt-out"])
        .arg(&stale_bundle_verify_path)
        .output()?;
    assert_success(&verify_stale_bundle, "dogfood release-bundle-verify stale marker");
    let stale_bundle_receipt = molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(
        &read_preserves(&stale_bundle_verify_path)?,
    )?;
    assert_eq!(stale_bundle_receipt.decision, "deny");
    assert!(
        stale_bundle_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("nextest-marker-ref mismatch"))
    );
    Ok(())
}

fn manifest_arg(root: &std::path::Path, name: &str, bytes: &[u8], kind: &str) -> CliResult<String> {
    let stored = molten::chunk_store::put_bytes(root, name, bytes, molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE)?;
    Ok(format!("{}@{}@{}", stored.manifest_ref, stored.total_len, kind))
}

#[test]
fn cli_blob_ref_job_submit_execute_status_and_receipt_show() -> CliResult<()> {
    let dir = temp_dir("cli-job-ref")?;
    let chunks = dir.join("chunks");
    let ledger = dir.join("ledger");
    let submission = dir.join("submission.preserves");
    let receipt_path = dir.join("receipt.preserves");
    let operation_id = test_ref("cli-job-ref-operation")?;
    let authority_ref = test_ref("cli-job-ref-authority")?;
    let policy_ref = test_ref("cli-job-ref-policy")?;
    let provenance_ref = test_ref("cli-job-ref-provenance")?;
    let effect_ref = test_ref("cli-job-ref-effect")?;
    let executable_arg = manifest_arg(&chunks, "job-executable", b"echo", "elf-executable")?;
    let input_arg = manifest_arg(&chunks, "job-input", b"cli-output", "bytes")?;

    let submit = molten_cmd()
        .args(["test", "job", "ref-submit", "--job-id", "cli-job-ref", "--operation-id"])
        .arg(&operation_id)
        .args(["--executable"])
        .arg(&executable_arg)
        .args(["--input"])
        .arg(&input_arg)
        .args(["--context-ref"])
        .arg(&authority_ref)
        .args(["--policy-ref"])
        .arg(&policy_ref)
        .args(["--provenance-ref"])
        .arg(&provenance_ref)
        .args(["--effect-ref"])
        .arg(&effect_ref)
        .args(["--out"])
        .arg(&submission)
        .output()?;
    assert_success(&submit, "job ref-submit");
    let submission_value = read_preserves(&submission)?;
    assert_eq!(molten::job_dag::parse_job_ref_submission_value(&submission_value)?.job_id, "cli-job-ref");

    let execute = molten_cmd()
        .args(["test", "job", "ref-execute"])
        .arg(&submission)
        .args(["--chunks"])
        .arg(&chunks)
        .args(["--ledger"])
        .arg(&ledger)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .output()?;
    assert_success(&execute, "job ref-execute");
    assert!(stdout(&execute).contains("job ref receipt blake3:"));
    let receipt_value = read_preserves(&receipt_path)?;
    let receipt = molten::job_dag::parse_blob_ref_job_receipt_value(&receipt_value)?;
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.output_refs.len(), 1);
    assert!(molten::job_dag::receipt_summary(&receipt_value)?.contains("decision=pass"));

    let status = molten_cmd()
        .args(["test", "job", "status", "--ledger"])
        .arg(&ledger)
        .args(["--job", "cli-job-ref"])
        .output()?;
    assert_success(&status, "job status");
    assert!(stdout(&status).contains("blob-ref-worker-execute"));

    let receipt_ref = molten::preserves_rail::canonical_hash(&receipt_value)?;
    let show = molten_cmd()
        .args(["test", "job", "receipt-show"])
        .arg(&receipt_ref)
        .args(["--ledger"])
        .arg(&ledger)
        .output()?;
    assert_success(&show, "job receipt-show");
    assert!(stdout(&show).contains("blob-ref-worker-execute"));
    Ok(())
}

#[test]
fn cli_gate_rejection_emits_canonical_failure_to_stdout_without_failure_out() -> CliResult<()> {
    let dir = temp_dir("cli-failure-stdout")?;
    let failure_artifact = dir.join("diagnostic.failure.preserves");
    let diagnostic = molten::harness::failure_value(
        "preflight",
        &molten::error::MoltenError::invalid_harness("synthetic diagnostic"),
        Vec::new(),
    );
    std::fs::write(&failure_artifact, molten::preserves_rail::to_text(&diagnostic)?)?;

    let failed_gate = molten_cmd().args(["test", "gate", "check"]).arg(&failure_artifact).output()?;
    assert_failure(&failed_gate, "failing test gate check");

    let stdout_failure = molten::preserves_rail::parse_text(&stdout(&failed_gate))?;
    let failure = molten::harness::parse_failure(&stdout_failure)?;
    assert_eq!(failure.phase, "validate");
    assert_eq!(failure.kind, "invalid-harness");
    assert!(failure.message.contains("cannot satisfy pass evidence gate"));
    Ok(())
}

#[test]
fn cli_octet_baseline_allows_identical_noncritical_warning_and_denies_new_warning() -> CliResult<()> {
    let dir = temp_dir("cli-octet-baseline")?;
    let baseline = dir.join("baseline.preserves");
    let pass_receipt = dir.join("baseline-pass.preserves");
    let deny_receipt = dir.join("baseline-deny.preserves");
    write_octet_artifacts_with(&dir, octet_noncritical_status(1), OCTET_NONCRITICAL_SUMMARY_ONE)?;

    let write = molten_cmd()
        .args(["test", "octet", "baseline", "write", "--artifacts"])
        .arg(&dir)
        .args(["--out"])
        .arg(&baseline)
        .args([
            "--created-at",
            "2026-05-31T00:00:00Z",
            "--expires-at",
            "9999-01-01T00:00:00Z",
        ])
        .output()?;
    assert_success(&write, "octet baseline write");
    assert!(stdout(&write).contains("octet warning baseline"));

    let pass = molten_cmd()
        .args(["test", "octet", "baseline", "check", "--artifacts"])
        .arg(&dir)
        .args(["--baseline"])
        .arg(&baseline)
        .args(["--as-of", "2026-05-31T00:00:00Z", "--receipt-out"])
        .arg(&pass_receipt)
        .output()?;
    assert_success(&pass, "octet baseline check pass");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&pass_receipt)?), "octet-baseline-receipt");

    write_octet_artifacts_with(&dir, octet_noncritical_status(2), OCTET_NONCRITICAL_SUMMARY_TWO)?;
    let deny = molten_cmd()
        .args(["test", "octet", "baseline", "check", "--artifacts"])
        .arg(&dir)
        .args(["--baseline"])
        .arg(&baseline)
        .args(["--as-of", "2026-05-31T00:00:00Z", "--receipt-out"])
        .arg(&deny_receipt)
        .output()?;
    assert_failure(&deny, "octet baseline check deny");
    let deny_text = molten::preserves_rail::to_text(&read_preserves(&deny_receipt)?)?;
    assert!(deny_text.contains("<decision \"deny\">"));
    assert!(deny_text.contains("new or increased octet findings"));
    Ok(())
}

#[test]
fn cli_octet_remediation_plan_writes_baseline_receipt() -> CliResult<()> {
    let dir = temp_dir("cli-octet-remediation")?;
    let workspace = dir.join("workspace");
    let lib = dir.join("lib");
    let receipt = dir.join("remediation-plan.preserves");
    std::fs::create_dir_all(&workspace)?;
    std::fs::create_dir_all(&lib)?;
    write_octet_artifacts_with(&workspace, octet_noncritical_status(1), OCTET_NONCRITICAL_SUMMARY_ONE)?;
    write_octet_artifacts_with(&lib, octet_noncritical_status(1), OCTET_NONCRITICAL_SUMMARY_ONE)?;

    let plan = molten_cmd()
        .args(["test", "octet", "remediation", "plan", "--artifacts"])
        .arg(&workspace)
        .args(["--lib-artifacts"])
        .arg(&lib)
        .args(["--receipt-out"])
        .arg(&receipt)
        .output()?;

    assert_success(&plan, "octet remediation plan");
    assert!(stdout(&plan).contains("octet remediation plan receipt=blake3:"));
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&receipt_value), "octet-remediation-plan");
    let text = molten::preserves_rail::to_text(&receipt_value)?;
    assert!(text.contains("critical-deny-classes"));
    assert!(text.contains("no-suppression-policy"));
    Ok(())
}

#[test]
fn cli_node_init_run_status_and_stop_write_receipts() -> CliResult<()> {
    let dir = temp_dir("cli-node-daemon")?;
    let state_root = dir.join("state");
    let config = dir.join("node-config.preserves");
    let startup = dir.join("node-startup.preserves");
    let health = dir.join("node-health.preserves");
    let status_receipt = dir.join("node-status-control.preserves");
    let socket_request = dir.join("node-socket-status-request.preserves");
    let socket_queue = dir.join("node-socket-status-queue.preserves");
    let socket_receipt = dir.join("node-socket-status-control.preserves");
    let shutdown_request = dir.join("node-socket-shutdown-request.preserves");
    let shutdown_queue = dir.join("node-socket-shutdown-queue.preserves");
    let shutdown = state_root.join("shutdown-receipt.preserves");
    let loop_receipt = dir.join("node-control-loop.preserves");

    start_case(StartArgs {
        root: &state_root,
        config: &config,
        startup: &startup,
    })?;
    let authority_ref = test_ref("node-control-authority")?;
    let policy_ref = test_ref("node-control-policy")?;
    let resource_ref = test_ref("node-control-resource")?;

    write_op(OpArgs {
        name: "status",
        out: &socket_request,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
        label: "node socket status request",
    })?;
    submit_op(&state_root, &socket_request, &socket_queue, "node socket status submit")?;
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&socket_queue)?), "node-control-queue-receipt");
    dispatch_op(&state_root, &socket_receipt, "node socket status dispatch")?;
    expect_running(&state_root, &health, &status_receipt)?;

    write_op(OpArgs {
        name: "shutdown",
        out: &shutdown_request,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
        label: "node socket shutdown request",
    })?;
    submit_op(&state_root, &shutdown_request, &shutdown_queue, "node socket shutdown submit")?;
    expect_stop_loop(&state_root, &shutdown, &loop_receipt)?;

    let stopped = molten_cmd().args(["node", "status", "--state-root"]).arg(&state_root).output()?;
    assert_success(&stopped, "node stopped status");
    assert!(stdout(&stopped).contains("node status stopped"));
    Ok(())
}

#[test]
fn cli_node_profile_backed_init_binds_startup_metadata() -> CliResult<()> {
    let dir = temp_dir("cli-node-profile-backed")?;
    let state_root = dir.join("state");
    let config = dir.join("node-config.preserves");
    let startup = dir.join("node-startup.preserves");
    let profile_resolution = dir.join("node-profile-resolution.preserves");
    let profile_ref = test_ref("checked-node-profile")?;
    let profile_state_root_ref = test_ref("profile-state-root")?;
    let policy_ref = test_ref("profile-policy")?;
    let capability_ref = test_ref("profile-capability")?;
    let resource_ref = test_ref("profile-resource")?;
    let effect_ref = test_ref("profile-effect")?;
    let mut init = molten_cmd();
    init.args(["node", "init", "--state-root"])
        .arg(&state_root)
        .args(["--node-id", "node:profile", "--config-out"])
        .arg(&config)
        .args(["--profile-resolution-out"])
        .arg(&profile_resolution)
        .args(["--profile-ref", &profile_ref, "--actual-profile-ref", &profile_ref])
        .args(["--profile-tier", "pilot", "--profile-identity", "pilot-node"])
        .args(["--profile-state-root-ref", &profile_state_root_ref])
        .args(["--policy-ref", &policy_ref, "--capability-ref", &capability_ref])
        .args(["--resource-ref", &resource_ref, "--effect-profile-ref", &effect_ref]);
    for adapter in molten::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let adapter_ref = test_ref(&format!("adapter-{adapter}"))?;
        init.arg("--adapter-profile").arg(format!("{adapter}={adapter_ref}"));
    }
    let output = init.output()?;
    assert_success(&output, "node profile-backed init");
    assert!(stdout(&output).contains("profile_resolution="));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&config)?), "node-config");
    let resolution_value = read_preserves(&profile_resolution)?;
    let resolution_ref = molten::preserves_rail::canonical_hash(&resolution_value)?;
    assert!(molten::preserves_rail::to_text(&resolution_value)?.contains("node-profile-config-resolution-v1"));

    let run = molten_cmd()
        .args(["node", "run", "--state-root"])
        .arg(&state_root)
        .args(["--startup-out"])
        .arg(&startup)
        .output()?;
    assert_success(&run, "node profile-backed run");
    let startup_receipt = molten::node_runtime::parse_node_startup_receipt(&read_preserves(&startup)?)?;
    assert!(startup_receipt.profile_metadata_refs.contains(&profile_ref));
    assert!(startup_receipt.profile_metadata_refs.contains(&resolution_ref));
    Ok(())
}
