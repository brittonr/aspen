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

fn write_config_lint_root(dir: &std::path::Path, drift: bool) -> CliResult<()> {
    std::fs::create_dir_all(dir.join("docs"))?;
    let cargo_revision = "d913dc01e765c9b297df5fcc57dfa06aac39bc74";
    let nix_revision = if drift {
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    } else {
        cargo_revision
    };
    let hook_entry = if drift {
        "entry: nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- validate --root .\n"
    } else {
        "entry: sh -c 'nix run path:${CAIRN_FLAKE:-../cairn#cairn} -- validate --root .'\n"
    };
    let toolchain = if drift {
        "[toolchain]\nchannel = \"nightly\"\n"
    } else {
        "[toolchain]\nchannel = \"nightly-2026-05-26\"\n"
    };

    std::fs::write(dir.join(".pre-commit-config.yaml"), hook_entry)?;
    std::fs::write(
        dir.join("flake.nix"),
        format!(
            "localGitSources = {{\n  \"ssh://git@github.com/OnixResearch/basalt.git#{nix_revision}\" = basalt-src;\n}};\n"
        ),
    )?;
    std::fs::write(dir.join("rust-toolchain.toml"), toolchain)?;
    std::fs::write(dir.join("README.md"), "portable config lint\n")?;
    std::fs::write(dir.join("docs/proof-workflow.md"), "release refs are explicit\n")?;
    std::fs::write(
        dir.join("Cargo.lock"),
        format!(
            "[[package]]\nname = \"basalt\"\nsource = \"git+ssh://git@github.com/OnixResearch/basalt.git#{cargo_revision}\"\n"
        ),
    )?;
    Ok(())
}

#[test]
fn cli_config_lint_accepts_relocatable_pinned_config() -> CliResult<()> {
    let dir = temp_dir("cli-config-lint")?;
    let receipt = dir.join("config-portability.preserves");
    let summary = dir.join("config-portability.txt");
    write_config_lint_root(&dir, false)?;

    let output = molten_cmd()
        .args(["test", "traceability", "config-lint", "--root"])
        .arg(&dir)
        .args(["--out"])
        .arg(&receipt)
        .args(["--summary-out"])
        .arg(&summary)
        .output()?;

    assert_success(&output, "config lint");
    assert!(stderr(&output).contains("config-portability report=blake3:"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("config-portability-report-v1"));
    assert!(receipt_text.contains("compared-source-pins"));
    assert!(std::fs::read_to_string(&summary)?.contains("decision=pass"));
    Ok(())
}

#[test]
fn cli_config_lint_denies_home_path_floating_toolchain_and_pin_drift() -> CliResult<()> {
    let dir = temp_dir("cli-config-lint-negative")?;
    let receipt = dir.join("config-portability.preserves");
    write_config_lint_root(&dir, true)?;

    let output = molten_cmd()
        .args(["test", "traceability", "config-lint", "--root"])
        .arg(&dir)
        .args(["--out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "config lint negative");
    let stderr_text = stderr(&output);
    assert!(stderr_text.contains("user-home-path:.pre-commit-config.yaml"));
    assert!(stderr_text.contains("floating-release-toolchain:rust-toolchain.toml"));
    assert!(stderr_text.contains("source-pin-drift:basalt"));
    assert!(std::fs::read_to_string(&receipt)?.contains("source-pin-drift:basalt"));
    Ok(())
}

#[test]
fn cli_effective_config_writes_canonical_artifact_before_summary() -> CliResult<()> {
    let dir = temp_dir("cli-effective-config")?;
    let receipt = dir.join("effective-config.preserves");
    let summary = dir.join("effective-config.txt");
    let profile_ref = "blake3:8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65";
    let cli_ref = "blake3:2ded4d8475648207836b950368aa4e1037b11b9aeb6f5b939482ad4d859664f7";

    let output = molten_cmd()
        .args(["test", "traceability", "effective-config", "--profile-ref", profile_ref])
        .args(["--field", &format!("node.id|node:local|profile|{profile_ref}|")])
        .args(["--field", &format!("state.root|target/node|cli-override|{cli_ref}|operator override")])
        .args(["--out"])
        .arg(&receipt)
        .args(["--summary-out"])
        .arg(&summary)
        .output()?;

    assert_success(&output, "effective config readback");
    assert!(stderr(&output).contains("effective-config ref=blake3:"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("effective-config-readback-v1"));
    assert!(receipt_text.contains("selected-source-class"));
    assert!(std::fs::read_to_string(&summary)?.contains("effective-config ref=blake3:"));
    Ok(())
}

#[test]
fn cli_effective_config_denies_release_fixture_default_and_stale_ref() -> CliResult<()> {
    let dir = temp_dir("cli-effective-config-negative")?;
    let receipt = dir.join("effective-config.preserves");

    let output = molten_cmd()
        .args(["test", "traceability", "effective-config", "--release-mode"])
        .args(["--profile-ref", "not-a-ref"])
        .args(["--field", "max.events|16|default|none|local fixture"])
        .args(["--out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&output, "effective config negative");
    let stderr_text = stderr(&output);
    assert!(stderr_text.contains("fixture-default-in-release:max.events"));
    assert!(stderr_text.contains("stale-ref:effective config profile:not-a-ref"));
    assert!(std::fs::read_to_string(&receipt)?.contains("fixture-default-in-release:max.events"));
    Ok(())
}

#[test]
fn cli_context_profile_expands_refs_for_command_core() -> CliResult<()> {
    let dir = temp_dir("cli-context-profile")?;
    let receipt = dir.join("context-profile.preserves");
    let summary = dir.join("context-profile.txt");
    let policy_ref = "blake3:8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65";
    let authority_ref = "blake3:2ded4d8475648207836b950368aa4e1037b11b9aeb6f5b939482ad4d859664f7";
    let resource_ref = "blake3:e6cfe6b85e63f1eb8bbaf271586411e55885b51611497587164fd2c0adf0aed3";
    let evidence_ref = "blake3:555b0d27ee2e8a2b36c5886b126f1c118e909de89261cd61a399982aec392c67";
    let extra_evidence_ref = "blake3:9492775b4c3722f1da4b1d955f04e8358341cebbbe8b8bd48596d1ce2ab1e0a8";

    let output = molten_cmd()
        .args([
            "test",
            "traceability",
            "context-profile",
            "--profile-id",
            "operator:node-control",
            "--profile-tier",
            "pilot",
            "--allowed-operation",
            "node.install",
            "--operation",
            "node.install",
            "--require-policy",
            "--require-authority",
            "--require-resource",
            "--require-evidence",
            "--policy-ref",
            policy_ref,
            "--authority-ref",
            authority_ref,
            "--resource-ref",
            resource_ref,
            "--evidence-ref",
            evidence_ref,
            "--override-evidence-ref",
            extra_evidence_ref,
            "--out",
        ])
        .arg(&receipt)
        .args(["--summary-out"])
        .arg(&summary)
        .output()?;

    assert_success(&output, "context profile expansion");
    assert!(stderr(&output).contains("context-profile expansion=blake3:"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("context-profile-expansion-v1"));
    assert!(receipt_text.contains(extra_evidence_ref));
    assert!(std::fs::read_to_string(&summary)?.contains("decision=pass"));
    Ok(())
}

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
