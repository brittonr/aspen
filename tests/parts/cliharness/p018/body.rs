const CLUSTER_HARNESS_FIXTURE: &str = "tests/fixtures/cluster-harness/two-node.cluster";
const CLUSTER_HARNESS_TEST_TIMEOUT_MS: &str = "30000";

#[test]
fn cli_cluster_harness_records_child_failure_and_verifies_offline() -> CliResult<()> {
    // r[verify molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
    // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
    // r[verify molten.testing.receipt_first_cluster_harness.fixture_executable_runner]
    // r[verify molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
    // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
    // r[verify molten.testing.local_multiprocess_cluster_tier.middle_tier]
    let root = temp_dir("cli-cluster-harness-child-denied")?;
    let state_root = root.join("state");
    let run_dir = root.join("run");
    let missing_binary = root.join("missing-node-binary");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLUSTER_HARNESS_FIXTURE);
    let run = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&fixture)
        .args(["--state-root"])
        .arg(&state_root)
        .args(["--run-dir"])
        .arg(&run_dir)
        .args(["--node-binary"])
        .arg(&missing_binary)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_failure(&run, "cluster harness checked fixture cannot spawn unavailable children");
    assert!(stdout(&run).contains("decision=deny"));
    for artifact in [
        "artifact-index.tsv",
        "fixture-metadata.preserves",
        "derived-plan.preserves",
        "local-executable-run.preserves",
        "cluster-lifecycle-receipt.preserves",
        "drift-summary.preserves",
        "cluster-run-receipt.preserves",
        "verification.preserves",
        "failure-repro-bundle.preserves",
        "failure-repro-verification.preserves",
    ] {
        assert!(run_dir.join(artifact).exists(), "missing cluster harness artifact {artifact}");
    }
    for node in ["fixture-a", "fixture-b"] {
        assert!(!run_dir.join("children/receipts").join(node).exists());
        assert!(!state_root.join(node).join("startup-receipt.preserves").exists());
        let log = std::fs::read_to_string(run_dir.join(format!("logs/init-{node}.log")))?;
        assert!(log.contains("spawn failed"));
    }

    let verify = molten_cmd()
        .args(["cluster", "harness-verify", "--run-dir"])
        .arg(&run_dir)
        .output()?;
    assert_failure(&verify, "cluster harness offline verification of denied run");
    assert!(stdout(&verify).contains("decision=deny"));
    Ok(())
}

#[test]
fn cli_cluster_harness_exports_sealed_bundle_when_children_cannot_spawn() -> CliResult<()> {
    // r[verify molten.testing.receipt_first_cluster_harness.fixture_executable_runner]
    // r[verify molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
    // r[verify molten.testing.cluster_failure_repro_bundles.bundle_schema]
    // r[verify molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass]
    let root = temp_dir("cli-cluster-harness-spawn-failure")?;
    let state_root = root.join("state");
    let run_dir = root.join("run");
    let missing_binary = root.join("missing-node-binary");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLUSTER_HARNESS_FIXTURE);
    let run = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&fixture)
        .args(["--state-root"])
        .arg(&state_root)
        .args(["--run-dir"])
        .arg(&run_dir)
        .args(["--node-binary"])
        .arg(&missing_binary)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_failure(&run, "cluster harness spawn failure");
    let bundle = read_preserves(&run_dir.join("failure-repro-bundle.preserves"))?;
    let bundle_text = molten::preserves_rail::to_text(&bundle)?;
    assert!(bundle_text.contains("multinode-failure-repro-bundle-v1"));
    assert!(bundle_text.contains("sealed #t"));
    assert!(run_dir.join("failure-repro-verification.preserves").exists());
    assert!(run_dir.join("cleanup-receipt.preserves").exists());
    Ok(())
}

#[test]
fn cli_cluster_harness_offline_verifier_denies_tampered_artifact() -> CliResult<()> {
    // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
    // r[verify molten.testing.receipt_first_cluster_harness.failure_triage]
    // r[verify molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
    let root = temp_dir("cli-cluster-harness-tamper")?;
    let state_root = root.join("state");
    let run_dir = root.join("run");
    let missing_binary = root.join("missing-node-binary");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLUSTER_HARNESS_FIXTURE);
    let run = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&fixture)
        .args(["--state-root"])
        .arg(&state_root)
        .args(["--run-dir"])
        .arg(&run_dir)
        .args(["--node-binary"])
        .arg(&missing_binary)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_failure(&run, "cluster harness denied run before tamper");
    assert!(stdout(&run).contains("decision=deny"));

    let drift_path = run_dir.join("drift-summary.preserves");
    let canonical_drift_text = std::fs::read_to_string(&drift_path)?;
    let mut drift_text = canonical_drift_text.clone();
    drift_text.push('\n');
    std::fs::write(&drift_path, drift_text)?;
    let verify = molten_cmd()
        .args(["cluster", "harness-verify", "--run-dir"])
        .arg(&run_dir)
        .output()?;
    assert_failure(&verify, "cluster harness tamper verification");
    let error = stderr(&verify);
    assert!(error.contains("non-canonical-artifact") || error.contains("content-ref-mismatch"));

    #[cfg(unix)]
    {
        let outside = root.join("outside-drift.preserves");
        std::fs::write(&outside, canonical_drift_text)?;
        std::fs::remove_file(&drift_path)?;
        std::os::unix::fs::symlink(&outside, &drift_path)?;
        let symlink_verify = molten_cmd()
            .args(["cluster", "harness-verify", "--run-dir"])
            .arg(&run_dir)
            .output()?;
        assert_failure(&symlink_verify, "cluster harness symlink verification");
        assert!(stderr(&symlink_verify).contains("unreadable-artifact"));
    }
    Ok(())
}
