#[test]
fn cli_repro_export_profiles_fail_closed_and_unpack_diagnostics() -> CliResult<()> {
    let dir = temp_dir("cli-repro-profiles")?;
    let suite = dir.join("secret-suite.preserves");
    let report = dir.join("report.preserves");
    std::fs::write(
        &suite,
        r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-cli" 1
          <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
          <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
          <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
          [<send "a" "b" <secret "token">>]>"#,
    )?;
    let run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&report).output()?;
    assert_success(&run, "secret test run");

    assert_default_denial(&dir, &report)?;
    assert_diagnostic_case(&dir, &report)?;
    let (encrypted_out, encrypted_refs) = prepare_private_case(&dir, &report)?;
    assert_reveal_cases(&dir, &encrypted_out, encrypted_refs.as_slice())?;
    Ok(())
}

fn assert_default_denial(dir: &std::path::Path, report: &std::path::Path) -> CliResult<()> {
    let denied_out = dir.join("default-repro");
    let denied = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(report)
        .args(["--out"])
        .arg(&denied_out)
        .output()?;
    assert_failure(&denied, "default deny-sensitive export");
    assert!(stderr(&denied).contains("sensitive marker secret"));
    Ok(())
}

fn assert_diagnostic_case(dir: &std::path::Path, report: &std::path::Path) -> CliResult<()> {
    let diagnostic_out = dir.join("diagnostic-repro");
    let diagnostic = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(report)
        .args(["--out"])
        .arg(&diagnostic_out)
        .args(["--profile", "redacted-diagnostic"])
        .output()?;
    assert_success(&diagnostic, "redacted diagnostic export");
    let diagnostic_bundle = read_preserves(&diagnostic_out.join("refs.preserves"))?;
    let parsed_diagnostic = molten::harness::parse_repro_bundle(&diagnostic_bundle)?;
    assert_eq!(parsed_diagnostic.export_profile.as_deref(), Some("redacted-diagnostic"));
    assert_eq!(parsed_diagnostic.loss_classification.as_deref(), Some("diagnostic-only"));
    assert!(diagnostic_out.join("redaction-transform-receipt.preserves").exists());
    let verify = molten_cmd().args(["test", "repro", "verify"]).arg(diagnostic_out.join("refs.preserves")).output()?;
    assert_failure(&verify, "diagnostic verify fails closed");
    assert!(stderr(&verify).contains("diagnostic-only"));
    let unpacked_diagnostic = dir.join("diagnostic-unpacked");
    let unpack = molten_cmd()
        .args(["test", "repro", "unpack"])
        .arg(diagnostic_out.join("refs.preserves"))
        .args(["--out"])
        .arg(&unpacked_diagnostic)
        .output()?;
    assert_success(&unpack, "diagnostic unpack");
    assert!(unpacked_diagnostic.join("redaction-transform-receipt.preserves").exists());
    Ok(())
}

fn prepare_private_case(
    dir: &std::path::Path,
    report: &std::path::Path,
) -> CliResult<(std::path::PathBuf, Vec<String>)> {
    let encrypted_out = dir.join("encrypted-repro");
    let encrypted = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(report)
        .args(["--out"])
        .arg(&encrypted_out)
        .args(["--profile", "encrypted-private"])
        .output()?;
    assert_success(&encrypted, "encrypted private export");
    let encrypted_bundle = read_preserves(&encrypted_out.join("refs.preserves"))?;
    let parsed_encrypted = molten::harness::parse_repro_bundle(&encrypted_bundle)?;
    assert_eq!(parsed_encrypted.loss_classification.as_deref(), Some("requires-reveal"));
    let denied_unpack = molten_cmd()
        .args(["test", "repro", "unpack"])
        .arg(encrypted_out.join("refs.preserves"))
        .args(["--out"])
        .arg(dir.join("encrypted-unpack-denied"))
        .output()?;
    assert_failure(&denied_unpack, "encrypted unpack without reveal");
    assert!(stderr(&denied_unpack).contains("requires at least one passing reveal receipt"));
    if parsed_encrypted.encrypted_refs.is_empty() {
        return Err(test_error("encrypted profile did not expose encrypted refs"));
    }
    Ok((encrypted_out, parsed_encrypted.encrypted_refs))
}

fn assert_reveal_cases(
    dir: &std::path::Path,
    encrypted_out: &std::path::Path,
    encrypted_refs: &[String],
) -> CliResult<()> {
    let first_ref = encrypted_refs
        .first()
        .ok_or_else(|| test_error("encrypted profile did not expose encrypted refs"))?;
    let legacy_reveal_path =
        write_reveal_case(dir, "legacy-reveal.preserves", first_ref, None, "authorized-private-material-legacy")?;
    let legacy_unpack = molten_cmd()
        .args(["test", "repro", "unpack"])
        .arg(encrypted_out.join("refs.preserves"))
        .args(["--out"])
        .arg(dir.join("encrypted-unpack-legacy-reveal"))
        .args(["--reveal-receipt"])
        .arg(&legacy_reveal_path)
        .output()?;
    assert_failure(&legacy_unpack, "encrypted unpack with legacy reveal ref");
    assert!(stderr(&legacy_unpack).contains("does not bind an encrypted repro reference"));

    let wrong_reveal_ref =
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::string("wrong-encrypted-ref"))?;
    let stale_reveal_path = write_reveal_case(
        dir,
        "stale-reveal.preserves",
        first_ref,
        Some(wrong_reveal_ref.as_str()),
        "authorized-private-material-stale",
    )?;
    let stale_unpack = molten_cmd()
        .args(["test", "repro", "unpack"])
        .arg(encrypted_out.join("refs.preserves"))
        .args(["--out"])
        .arg(dir.join("encrypted-unpack-stale-reveal"))
        .args(["--reveal-receipt"])
        .arg(&stale_reveal_path)
        .output()?;
    assert_failure(&stale_unpack, "encrypted unpack with stale reveal ref");
    assert!(stderr(&stale_unpack).contains("not part of this repro bundle"));

    let mut reveal_paths = Vec::with_capacity(encrypted_refs.len());
    for (index, encrypted_ref) in encrypted_refs.iter().enumerate() {
        let reveal_name = format!("reveal-{index}.preserves");
        let plaintext = format!("authorized-private-material-{index}");
        let reveal_path =
            write_reveal_case(dir, &reveal_name, encrypted_ref, Some(encrypted_ref.as_str()), &plaintext)?;
        reveal_paths.push(reveal_path);
    }
    let unpacked_private = dir.join("encrypted-unpacked");
    let mut reveal_command = molten_cmd();
    reveal_command
        .args(["test", "repro", "unpack"])
        .arg(encrypted_out.join("refs.preserves"))
        .args(["--out"])
        .arg(&unpacked_private);
    for reveal_path in &reveal_paths {
        reveal_command.args(["--reveal-receipt"]).arg(reveal_path);
    }
    let revealed_unpack = reveal_command.output()?;
    assert_success(&revealed_unpack, "encrypted unpack with reveal");
    assert!(unpacked_private.join("reveal-receipt-0.preserves").exists());
    Ok(())
}

fn write_reveal_case(
    dir: &std::path::Path,
    name: &str,
    secret_ref: &str,
    encrypted_ref: Option<&str>,
    plaintext: &str,
) -> CliResult<std::path::PathBuf> {
    let reveal = molten::secrets::reveal_receipt_value(&molten::secrets::RevealReceiptInput {
        secret_ref: secret_ref.to_string(),
        encrypted_ref: encrypted_ref.map(|ref_value| ref_value.to_string()),
        requester_ref: molten::preserves_rail::canonical_hash(&molten::preserves_rail::string("cli-requester"))?,
        purpose: "export".to_string(),
        plaintext_ref: Some(molten::preserves_rail::canonical_hash(&molten::preserves_rail::string(plaintext))?),
        commitment_ref: secret_ref.to_string(),
        authority_refs: vec![molten::preserves_rail::canonical_hash(
            &molten::preserves_rail::string("reveal-authority"),
        )?],
        policy_refs: vec![molten::preserves_rail::canonical_hash(
            &molten::preserves_rail::string("reveal-policy"),
        )?],
        resource_refs: vec![molten::preserves_rail::canonical_hash(
            &molten::preserves_rail::string("reveal-resource"),
        )?],
        effect_handle_refs: vec![molten::preserves_rail::canonical_hash(
            &molten::preserves_rail::string("reveal-effect-handle"),
        )?],
        revocation_refs: Vec::new(),
    })?;
    let reveal_path = dir.join(name);
    std::fs::write(&reveal_path, molten::preserves_rail::to_text(&reveal)?)?;
    Ok(reveal_path)
}
