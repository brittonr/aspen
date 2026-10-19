const CLUSTER_HARNESS_FIXTURE: &str = "tests/fixtures/cluster-harness/two-node.cluster";
const CLUSTER_HARNESS_TEST_TIMEOUT_MS: &str = "30000";

#[test]
fn cli_cluster_harness_executes_checked_fixture_and_verifies_offline() -> CliResult<()> {
    // r[verify molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
    // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
    // r[verify molten.testing.receipt_first_cluster_harness.fixture_executable_runner]
    // r[verify molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
    // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
    // r[verify molten.testing.local_multiprocess_cluster_tier.middle_tier]
    let root = temp_dir("cli-cluster-harness-success")?;
    let state_root = root.join("state");
    let run_dir = root.join("run");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLUSTER_HARNESS_FIXTURE);
    let run = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&fixture)
        .args(["--state-root"])
        .arg(&state_root)
        .args(["--run-dir"])
        .arg(&run_dir)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_success(&run, "cluster harness checked fixture run");
    assert!(stdout(&run).contains("decision=pass"));
    for artifact in [
        "artifact-index.tsv",
        "fixture-metadata.preserves",
        "derived-plan.preserves",
        "local-executable-run.preserves",
        "cluster-lifecycle-receipt.preserves",
        "drift-summary.preserves",
        "cluster-run-receipt.preserves",
        "verification.preserves",
    ] {
        assert!(run_dir.join(artifact).exists(), "missing cluster harness artifact {artifact}");
    }
    let parent_text =
        molten::preserves_rail::to_text(&read_preserves(&run_dir.join("cluster-run-receipt.preserves"))?)?;
    let drift_text = molten::preserves_rail::to_text(&read_preserves(&run_dir.join("drift-summary.preserves"))?)?;
    let node_a_startup =
        read_preserves(&run_dir.join("children/receipts/fixture-a/startup-receipt.preserves"))?;
    let node_b_startup =
        read_preserves(&run_dir.join("children/receipts/fixture-b/startup-receipt.preserves"))?;
    assert_ne!(
        molten::preserves_rail::canonical_hash(&node_a_startup)?,
        molten::preserves_rail::canonical_hash(&node_b_startup)?
    );
    assert!(parent_text.contains("child-receipts"));
    assert!(drift_text.contains("expected-equalities"));
    assert!(drift_text.contains("allowed-variances"));

    let verify = molten_cmd()
        .args(["cluster", "harness-verify", "--run-dir"])
        .arg(&run_dir)
        .output()?;
    assert_success(&verify, "cluster harness offline verification");
    assert!(stdout(&verify).contains("decision=pass"));
    Ok(())
}

#[test]
fn cli_cluster_harness_preflights_fixture_bytes_before_staging_or_children() -> CliResult<()> {
    let root = temp_dir("cli-cluster-harness-fixture-byte-bound")?;
    let prefix = "molten.cluster.nodes.v1\nnode:fixture-a\n";
    let mut source = prefix.to_string();
    source.push_str(&"\n".repeat(molten::cluster::MAX_CLUSTER_MANIFEST_BYTES - prefix.len()));
    assert_eq!(source.len(), molten::cluster::MAX_CLUSTER_MANIFEST_BYTES);
    let exact_fixture = root.join("exact.cluster");
    std::fs::write(&exact_fixture, &source)?;
    let exact_state = root.join("exact-state");
    let exact_run = root.join("exact-run");
    let exact = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&exact_fixture)
        .args(["--state-root"])
        .arg(&exact_state)
        .args(["--run-dir"])
        .arg(&exact_run)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_success(&exact, "exact one-megabyte harness fixture with actual child");
    assert!(exact_run.join("children/receipts/fixture-a/startup-receipt.preserves").exists());

    source.push('\n');
    let excess_fixture = root.join("excess.cluster");
    std::fs::write(&excess_fixture, source)?;
    let excess_state = root.join("excess-state");
    let excess_run = root.join("excess-run");
    std::fs::create_dir_all(&excess_state)?;
    std::fs::create_dir_all(&excess_run)?;
    let state_sentinel = excess_state.join("state-before-run");
    let run_sentinel = excess_run.join("output-before-run");
    std::fs::write(&state_sentinel, "retained")?;
    std::fs::write(&run_sentinel, "retained")?;
    let excess = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&excess_fixture)
        .args(["--state-root"])
        .arg(&excess_state)
        .args(["--run-dir"])
        .arg(&excess_run)
        .args(["--force", "--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_failure(&excess, "oversized fixture must deny before staging");
    assert!(stderr(&excess).contains("cluster manifest exceeds byte bound 1048576"));
    assert_eq!(std::fs::read(&state_sentinel)?, b"retained");
    assert_eq!(std::fs::read(&run_sentinel)?, b"retained");
    assert!(!excess_run.join("artifact-index.tsv").exists());
    assert!(!excess_state.join("fixture-a").exists());
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
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLUSTER_HARNESS_FIXTURE);
    let run = molten_cmd()
        .args(["cluster", "harness-run", "--fixture"])
        .arg(&fixture)
        .args(["--state-root"])
        .arg(&state_root)
        .args(["--run-dir"])
        .arg(&run_dir)
        .args(["--child-timeout-ms", CLUSTER_HARNESS_TEST_TIMEOUT_MS])
        .output()?;
    assert_success(&run, "cluster harness run before tamper");

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
    let is_tamper_explained = error.contains("non-canonical-artifact") || error.contains("content-ref-mismatch");
    assert!(is_tamper_explained, "unexpected tamper verification error: {error}");

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
