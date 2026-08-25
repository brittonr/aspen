fn release_candidate_gate_ref(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn release_candidate_gate_command(
    out: &std::path::Path,
    rust_source_ref: &str,
) -> std::process::Command {
    let source_ref = release_candidate_gate_ref("reviewed-candidate-source");
    let artifact_ref = release_candidate_gate_ref("candidate-evidence-artifact");
    let binding = format!("{artifact_ref}@{source_ref}");
    let rust_binding = format!("{artifact_ref}@{rust_source_ref}");
    let mut command = molten_cmd();
    command
        .args(["test", "prod-soak", "release-candidate-gate"])
        .args(["--candidate", "molten-limited-internal-pilot"])
        .args(["--source-ref", &source_ref])
        .args(["--rust-validation-binding", &rust_binding])
        .args(["--nextest-binding", &binding])
        .args(["--nix-check-binding", &binding])
        .args(["--cairn-validation-binding", &binding])
        .args(["--octet-binding", &binding])
        .args(["--dogfood-binding", &binding])
        .args(["--bundle-verify-binding", &binding])
        .args(["--promotion-binding", &binding])
        .args(["--export-verify-binding", &binding])
        .args(["--pilot-decision-binding", &binding])
        .arg("--out")
        .arg(out);
    command
}

// r[verify molten.prod_release_candidate.evidence_source_binding]
#[test]
fn release_candidate_cli_records_one_candidate_for_all_evidence() -> CliResult<()> {
    let dir = temp_dir("release-candidate-binding-pass")?;
    let out = dir.join("candidate.preserves");
    let source_ref = release_candidate_gate_ref("reviewed-candidate-source");
    let output = release_candidate_gate_command(&out, &source_ref).output()?;

    assert_success(&output, "candidate-bound release gate");
    let text = std::fs::read_to_string(out)?;
    assert!(text.contains("prod-release-candidate-gate-v2"));
    assert!(text.contains("candidate-evidence"));
    assert!(text.contains(&source_ref));
    assert!(text.contains("all-evidence-candidate-bound"));
    Ok(())
}

// r[verify molten.prod_release_candidate.evidence_source_binding]
#[test]
fn release_candidate_cli_denies_mismatched_or_malformed_binding() -> CliResult<()> {
    let dir = temp_dir("release-candidate-binding-deny")?;
    let mismatch_out = dir.join("mismatch.preserves");
    let other_source_ref = release_candidate_gate_ref("other-candidate-source");
    let mismatch = release_candidate_gate_command(&mismatch_out, &other_source_ref).output()?;

    assert!(!mismatch.status.success());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("Rust validation candidate source mismatch"));

    let malformed_out = dir.join("malformed.preserves");
    let source_ref = release_candidate_gate_ref("reviewed-candidate-source");
    let mut malformed = release_candidate_gate_command(&malformed_out, &source_ref);
    malformed.args(["--rust-validation-binding", "missing-source-member"]);
    let malformed = malformed.output()?;
    assert!(!malformed.status.success());
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("must use ARTIFACT_REF@SOURCE_REF"));
    Ok(())
}
