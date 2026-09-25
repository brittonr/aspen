
#[test]
fn cli_context_profile_denies_unsupported_scope_and_conflicting_authority() -> CliResult<()> {
    let dir = temp_dir("cli-context-profile-negative")?;
    let receipt = dir.join("context-profile.preserves");
    let policy_ref = "blake3:8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65";
    let authority_ref = "blake3:2ded4d8475648207836b950368aa4e1037b11b9aeb6f5b939482ad4d859664f7";
    let other_authority_ref = "blake3:bb8a0de843b8375ddbfea5424f28cfd6c662402ce40c7fdd28268b7fcaa09e96";

    let output = molten_cmd()
        .args([
            "test",
            "traceability",
            "context-profile",
            "--profile-id",
            "operator:catalog-read",
            "--allowed-operation",
            "catalog.search",
            "--operation",
            "retention.delete",
            "--require-authority",
            "--policy-ref",
            policy_ref,
            "--authority-ref",
            authority_ref,
            "--override-authority-ref",
            other_authority_ref,
            "--out",
        ])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "context profile negative");
    let stderr_text = stderr(&output);
    assert!(stderr_text.contains("unsupported-operation-scope:retention.delete"));
    assert!(stderr_text.contains("conflicting-authority-override"));
    assert!(std::fs::read_to_string(&receipt)?.contains("conflicting-authority-override"));
    Ok(())
}

#[test]
fn cli_ci_run_receipt_binds_nextest_metadata_and_junit_view() -> CliResult<()> {
    let dir = temp_dir("cli-ci-run-receipt")?;
    let nextest_config = dir.join("nextest.toml");
    let cargo_metadata = dir.join("cargo-metadata.json");
    let binaries_metadata = dir.join("binaries-metadata.json");
    let junit = dir.join("junit.xml");
    let receipt = dir.join("ci-test-run.preserves");

    std::fs::write(&nextest_config, "[profile.ci]\n")?;
    std::fs::write(&cargo_metadata, "{\"packages\":[]}")?;
    std::fs::write(&binaries_metadata, "{\"rust-suites\":[]}")?;
    std::fs::write(&junit, r#"<testsuite tests="2" failures="0" errors="0"></testsuite>"#)?;

    let output = molten_cmd()
        .args([
            "test",
            "traceability",
            "ci-run-receipt",
            "--source-marker",
            "nix-source-fixture",
            "--profile-id",
            "ci",
            "--command-surface",
            "cargo nextest run --profile ci",
            "--nextest-config",
        ])
        .arg(&nextest_config)
        .args(["--cargo-metadata"])
        .arg(&cargo_metadata)
        .args(["--binaries-metadata"])
        .arg(&binaries_metadata)
        .args(["--junit"])
        .arg(&junit)
        .args(["--caveat", "JUnit is a rendered view", "--out"])
        .arg(&receipt)
        .output()?;

    assert_success(&output, "ci run receipt");
    assert!(stderr(&output).contains("ci-test-run receipt=blake3:"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("ci-test-run-receipt-v1"));
    assert!(receipt_text.contains("<profile-id \"ci\">"));
    assert!(receipt_text.contains("<passed 2>"));
    assert!(receipt_text.contains("<skipped 0>"));
    Ok(())
}

#[test]
fn cli_ci_run_receipt_rejects_junit_missing_required_tests() -> CliResult<()> {
    let dir = temp_dir("cli-ci-run-receipt-missing-tests")?;
    let nextest_config = dir.join("nextest.toml");
    let cargo_metadata = dir.join("cargo-metadata.json");
    let binaries_metadata = dir.join("binaries-metadata.json");
    let junit = dir.join("junit.xml");
    let receipt = dir.join("ci-test-run.preserves");

    std::fs::write(&nextest_config, "[profile.ci]\n")?;
    std::fs::write(&cargo_metadata, "{\"packages\":[]}")?;
    std::fs::write(&binaries_metadata, "{\"rust-suites\":[]}")?;
    std::fs::write(&junit, r#"<testsuite failures="0" errors="0"></testsuite>"#)?;

    let output = molten_cmd()
        .args([
            "test",
            "traceability",
            "ci-run-receipt",
            "--source-marker",
            "nix-source-fixture",
            "--profile-id",
            "ci",
            "--command-surface",
            "cargo nextest run --profile ci",
            "--nextest-config",
        ])
        .arg(&nextest_config)
        .args(["--cargo-metadata"])
        .arg(&cargo_metadata)
        .args(["--binaries-metadata"])
        .arg(&binaries_metadata)
        .args(["--junit"])
        .arg(&junit)
        .args(["--out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "ci run receipt missing tests attribute");
    assert!(!receipt.exists());
    assert!(stderr(&output).contains("JUnit missing tests attribute"));
    Ok(())
}

#[test]
fn cli_ci_run_receipt_rejects_junit_only_missing_metadata() -> CliResult<()> {
    let dir = temp_dir("cli-ci-run-receipt-negative")?;
    let nextest_config = dir.join("missing-nextest.toml");
    let cargo_metadata = dir.join("cargo-metadata.json");
    let binaries_metadata = dir.join("binaries-metadata.json");
    let junit = dir.join("junit.xml");
    let receipt = dir.join("ci-test-run.preserves");

    std::fs::write(&cargo_metadata, "{\"packages\":[]}")?;
    std::fs::write(&binaries_metadata, "{\"rust-suites\":[]}")?;
    std::fs::write(&junit, r#"<testsuite tests="1" failures="0" errors="0" skipped="0"></testsuite>"#)?;

    let output = molten_cmd()
        .args([
            "test",
            "traceability",
            "ci-run-receipt",
            "--source-marker",
            "nix-source-fixture",
            "--profile-id",
            "ci",
            "--command-surface",
            "cargo nextest run --profile ci",
            "--nextest-config",
        ])
        .arg(&nextest_config)
        .args(["--cargo-metadata"])
        .arg(&cargo_metadata)
        .args(["--binaries-metadata"])
        .arg(&binaries_metadata)
        .args(["--junit"])
        .arg(&junit)
        .args(["--out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "ci run receipt missing metadata");
    assert!(!receipt.exists());
    assert!(stderr(&output).contains("io error"));
    Ok(())
}
