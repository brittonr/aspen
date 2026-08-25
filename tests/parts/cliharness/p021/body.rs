fn release_profile_ref(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn release_profile_command(out: &std::path::Path, include_candidate: bool) -> std::process::Command {
    let generated_ref = release_profile_ref("generated-profile");
    let policy_hash = release_profile_ref("valence-policy").replace("blake3:", "");
    let mut command = molten_cmd();
    command
        .args(["test", "gate", "release-profile"])
        .args(["--profile-id", "candidate-bound-release-profile"])
        .args(["--tier", "release"])
        .args(["--source-gate-ref", &release_profile_ref("source-gate")])
        .args(["--policy-ref", &release_profile_ref("policy")])
        .args(["--octet-ref", &release_profile_ref("octet")])
        .args(["--cairn-ref", &release_profile_ref("cairn")])
        .args(["--stack-provenance-ref", &release_profile_ref("stack-provenance")])
        .args(["--production-profile-ref", &release_profile_ref("production-profile")])
        .args(["--expected-generated-export-ref", &generated_ref])
        .args(["--actual-generated-export-ref", &generated_ref])
        .arg("--stack-provenance-required")
        .args(["--accepted-valence-policy-hash", &policy_hash])
        .args(["--caveat", "release-profile-command-fixture-only"])
        .arg("--out")
        .arg(out);
    if include_candidate {
        command.args(["--candidate-ref", &release_profile_ref("candidate")]);
    }
    command
}

// r[verify molten.prod_release_profile.executable_gate]
// r[verify molten.prod_ops.release_profile.candidate_binding]
#[test]
fn release_profile_cli_emits_candidate_bound_pass_value() -> CliResult<()> {
    let dir = temp_dir("release-profile-cli-pass")?;
    let out = dir.join("release-profile-pass.preserves");
    let output = release_profile_command(&out, true).output()?;

    assert_success(&output, "release profile gate pass");
    let text = std::fs::read_to_string(out)?;
    assert!(text.contains("release-profile-validation-v1"));
    assert!(text.contains("candidate-ref"));
    assert!(text.contains("pass"));
    Ok(())
}

// r[verify molten.prod_release_profile.executable_gate]
// r[verify molten.prod_ops.release_profile.candidate_binding]
#[test]
fn release_profile_cli_preserves_deny_value_for_missing_candidate() -> CliResult<()> {
    let dir = temp_dir("release-profile-cli-deny")?;
    let out = dir.join("release-profile-deny.preserves");
    let output = release_profile_command(&out, false).output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing-release-candidate-ref"));
    let text = std::fs::read_to_string(out)?;
    assert!(text.contains("release-profile-validation-v1"));
    assert!(text.contains("missing-release-candidate-ref"));
    assert!(text.contains("deny"));
    Ok(())
}
