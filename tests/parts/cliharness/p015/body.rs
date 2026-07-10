fn nextest_profile_config() -> String {
    r#"
[profile.default]
retries = 0
flaky-result = "fail"
junit = { path = "junit.xml" }

[profile.ci]
inherits = "default"
default-filter = 'package(molten)'

[profile.deterministic]
inherits = "default"
default-filter = 'package(molten) & not test(/live|vm|dogfood|soak|exploratory/)'

[profile.exploratory]
inherits = "default"
default-filter = 'package(molten)'
retries = 1
flaky-result = "pass"

[profile.fast-core]
inherits = "deterministic"
default-filter = 'package(molten) & test(/hardening|bounded|preserves|profile|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)'

[profile.harness]
inherits = "deterministic"
default-filter = 'package(molten) & test(/harness|replay|repro|gate|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)'

[profile.cli]
inherits = "ci"
default-filter = 'package(molten) & test(/cli|cliharness|command|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)'

[profile.distributed-simulation]
inherits = "deterministic"
default-filter = 'package(molten) & test(/distributed|simulation|fault|two_peer|remote/) & not test(/live|vm|dogfood|soak|exploratory/)'

[profile.vm-platform]
inherits = "ci"
default-filter = 'package(molten) & test(/vm|nixos|platform/)'

[profile.dogfood-soak]
inherits = "ci"
default-filter = 'package(molten) & test(/dogfood|soak|release/)'
"#
    .trim_start()
    .to_string()
}

#[test]
fn cli_nextest_profile_matrix_binds_filters_retry_and_junit_readback() -> CliResult<()> {
    let dir = temp_dir("cli-nextest-profile-matrix")?;
    let nextest_config = dir.join("nextest.toml");
    let receipt = dir.join("nextest-profile-matrix.preserves");
    let summary = dir.join("nextest-profile-matrix.txt");

    std::fs::write(&nextest_config, nextest_profile_config())?;

    let output = molten_cmd()
        .args(["test", "traceability", "nextest-profile-matrix", "--nextest-config"])
        .arg(&nextest_config)
        .args(["--out"])
        .arg(&receipt)
        .args(["--summary-out"])
        .arg(&summary)
        .output()?;

    assert_success(&output, "nextest profile matrix");
    assert!(stderr(&output).contains("nextest-profile-matrix ref=blake3:"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("nextest-profile-matrix-v1"));
    assert!(receipt_text.contains("filter-expression"));
    assert!(receipt_text.contains("retry-policy"));
    assert!(receipt_text.contains("expected-junit-path"));
    assert!(std::fs::read_to_string(&summary)?.contains("decision=pass"));
    Ok(())
}

#[test]
fn cli_nextest_profile_matrix_denies_missing_filter() -> CliResult<()> {
    let dir = temp_dir("cli-nextest-profile-matrix-negative")?;
    let nextest_config = dir.join("nextest.toml");
    let receipt = dir.join("nextest-profile-matrix.preserves");
    let config = nextest_profile_config().replace(
        "default-filter = 'package(molten) & test(/hardening|bounded|preserves|profile|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)'\n",
        "default-filter = ''\n",
    );

    std::fs::write(&nextest_config, config)?;

    let output = molten_cmd()
        .args(["test", "traceability", "nextest-profile-matrix", "--nextest-config"])
        .arg(&nextest_config)
        .args(["--out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "nextest profile matrix missing filter");
    assert!(stderr(&output).contains("missing-filter:fast-core"));
    assert!(std::fs::read_to_string(&receipt)?.contains("missing-filter:fast-core"));
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
