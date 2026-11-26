
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
    // The checked-in evidence-matrix row binds exactly this canonical receipt
    // and the content refs of the inputs this test wrote.
    let receipt_ref = molten::preserves_rail::canonical_hash(&molten::preserves_rail::parse_text(&receipt_text)?)?;
    let mut artifact_refs = [&nextest_config, &cargo_metadata, &binaries_metadata, &junit]
        .into_iter()
        .map(|path| Ok(molten::preserves_rail::content_ref_from_bytes(&std::fs::read(path)?)))
        .collect::<CliResult<Vec<_>>>()?;
    artifact_refs.sort();
    assert_evidence_matrix_row(
        "cargo test --test cliharness cli_ci_run_receipt_binds_nextest_metadata_and_junit_view",
        &artifact_refs,
        &receipt_ref,
    );
    Ok(())
}

/// The row of `tests/evidence-matrix.ncl` whose command is `command` (one field per line).
fn assert_evidence_matrix_row(command: &str, artifact_refs: &[String], receipt_ref: &str) {
    let matrix = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/evidence-matrix.ncl"));
    let lines: Vec<_> = matrix.lines().map(str::trim).collect();
    let marker = format!("command = \"{command}\",");
    let rows: Vec<_> = lines.iter().enumerate().filter(|(_, line)| **line == marker).map(|(index, _)| index).collect();
    assert_eq!(rows.len(), 1, "exactly one evidence-matrix row must name {command}");
    let quoted = |line: &str| -> Vec<String> {
        line.split('"').skip(1).step_by(2).filter(|text| text.starts_with("blake3:")).map(str::to_string).collect()
    };
    assert_eq!(quoted(lines[rows[0] + 1]), artifact_refs, "matrix artifact refs for {command}");
    assert_eq!(quoted(lines[rows[0] + 2]), [receipt_ref.to_string()], "matrix receipt ref for {command}");
}

// The hidden distinct-process peer entry of the native capability producers
// starts from the real CLI: a transport listener peer binds loopback Iroh and
// hands off a canonical endpoint descriptor in its run directory.
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn cli_evidence_qualify_peer_starts_a_transport_listener_peer() -> CliResult<()> {
    let dir = temp_dir("cli-qualify-peer")?;
    let run = dir.join("peer-run");
    std::fs::create_dir(&run)?;
    let mut child = molten_cmd()
        .args(["evidence", "qualify-peer", "--role", "transport-listener", "--run-directory"])
        .arg(&run)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let endpoint = run.join("endpoint.preserves");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !endpoint.is_file() && std::time::Instant::now() < deadline && child.try_wait()?.is_none() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let handoff = std::fs::read(&endpoint);
    child.kill()?;
    child.wait()?;
    let handoff = molten::preserves_rail::strict_canonical_decode(&handoff?)?.value;
    let parsed = molten::fabric_transport::parse_endpoint(&handoff)?;
    assert!(parsed.descriptor.public_endpoint_identity.starts_with("iroh:"));
    assert_eq!(parsed.descriptor.profile_id, "iroh-cross-process-v1");
    Ok(())
}

#[test]
fn cli_evidence_qualify_peer_rejects_unknown_roles_and_missing_run_directories() -> CliResult<()> {
    let dir = temp_dir("cli-qualify-peer-negative")?;
    let run = dir.join("peer-run");
    std::fs::create_dir(&run)?;
    let unknown = molten_cmd()
        .args(["evidence", "qualify-peer", "--role", "caller-invented-role", "--run-directory"])
        .arg(&run)
        .output()?;
    assert_failure(&unknown, "unknown qualification peer role");
    assert!(stderr(&unknown).contains("unsupported qualification peer role"));
    let absent = dir.join("absent");
    let missing = molten_cmd()
        .args(["evidence", "qualify-peer", "--role", "transport-listener", "--run-directory"])
        .arg(&absent)
        .output()?;
    assert_failure(&missing, "missing qualification peer run directory");
    assert!(!absent.exists());
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
