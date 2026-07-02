type CliResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[test]
fn cli_happy_path_produces_gateable_report_and_repro_bundle() -> CliResult<()> {
    let dir = temp_dir("cli-happy")?;
    let report = dir.join("report.preserves");
    let suite = manifest_dir().join("examples/two-actor.preserves");

    let run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&report).output()?;
    assert_success(&run, "test run");
    assert!(stdout(&run).contains("report blake3:"));

    let report_value = read_preserves(&report)?;
    let summary = molten::harness::report_summary(&report_value)?;
    assert!(summary.contains("status=pass"));
    assert!(summary.contains("replay_status=deterministic"));

    let replay = molten_cmd().args(["test", "replay"]).arg(&report).output()?;
    assert_success(&replay, "test replay");
    assert!(stdout(&replay).contains("replay ok"));

    let validate = molten_cmd().args(["test", "report", "validate"]).arg(&report).output()?;
    assert_success(&validate, "test report validate");
    assert!(stdout(&validate).contains("report validate ok"));

    let gate = molten_cmd().args(["test", "gate", "check"]).arg(&report).output()?;
    assert_success(&gate, "test gate check");
    let receipt_value = molten::preserves_rail::parse_text(&stdout(&gate))?;
    let receipt = molten::harness::parse_gate_receipt(&receipt_value)?;
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.artifact_kind, "report");
    assert!(molten::harness::gate_receipt_summary(&receipt_value)?.contains("decision=pass"));

    assert_report_repro_flow(&dir, &report, &report_value, &receipt.report_ref)?;
    Ok(())
}

fn assert_report_repro_flow(
    dir: &std::path::Path,
    report: &std::path::Path,
    report_value: &preserves::IOValue,
    report_ref: &str,
) -> CliResult<()> {
    let repro = dir.join("repro");
    let export = molten_cmd().args(["test", "repro", "export"]).arg(report).args(["--out"]).arg(&repro).output()?;
    assert_success(&export, "test repro export");
    assert!(stdout(&export).contains("repro bundle written"));

    let bundle = read_preserves(&repro.join("refs.preserves"))?;
    let parsed_bundle = molten::harness::parse_repro_bundle(&bundle)?;
    assert_eq!(parsed_bundle.kind, molten::harness::HarnessReproBundleKind::Report);
    assert!(parsed_bundle.gate_receipt_ref.is_some());
    let embedded_value = parsed_bundle
        .gate_receipt_value
        .as_ref()
        .ok_or_else(|| test_error("sealed repro bundle missing embedded report gate receipt"))?;
    let embedded_receipt = molten::harness::parse_gate_receipt(embedded_value)?;
    assert_eq!(embedded_receipt.artifact_kind, "report");
    let exported_receipt =
        molten::harness::parse_gate_receipt(&read_preserves(&repro.join("gate-receipt.preserves"))?)?;
    assert_eq!(exported_receipt.receipt_ref, embedded_receipt.receipt_ref);

    let verify_receipt = dir.join("verify-receipt.preserves");
    let verify_bundle = molten_cmd()
        .args(["test", "repro", "verify"])
        .arg(repro.join("refs.preserves"))
        .args(["--receipt-out"])
        .arg(&verify_receipt)
        .output()?;
    assert_success(&verify_bundle, "test repro verify");
    assert!(stdout(&verify_bundle).contains("repro verify receipt blake3:"));
    let verify = molten::harness::parse_repro_verify_receipt(&read_preserves(&verify_receipt)?)?;
    assert_eq!(verify.decision, "pass");
    assert_eq!(verify.report_ref, report_ref);

    let unpacked = dir.join("unpacked");
    let unpack = molten_cmd()
        .args(["test", "repro", "unpack"])
        .arg(repro.join("refs.preserves"))
        .args(["--out"])
        .arg(&unpacked)
        .output()?;
    assert_success(&unpack, "test repro unpack");
    assert!(stdout(&unpack).contains("repro bundle unpacked"));
    assert_eq!(
        molten::preserves_rail::canonical_hash(&read_preserves(&unpacked.join("report.preserves"))?)?,
        molten::preserves_rail::canonical_hash(report_value)?
    );
    molten::harness::parse_repro_verify_receipt(&read_preserves(&unpacked.join("verify-receipt.preserves"))?)?;

    let bundle_receipt = dir.join("bundle.gate-receipt.preserves");
    let gate_bundle = molten_cmd()
        .args(["test", "gate", "check"])
        .arg(repro.join("refs.preserves"))
        .args(["--receipt-out"])
        .arg(&bundle_receipt)
        .output()?;
    assert_success(&gate_bundle, "test gate check repro bundle");
    assert!(stdout(&gate_bundle).contains("gate receipt blake3:"));
    let receipt = molten::harness::parse_gate_receipt(&read_preserves(&bundle_receipt)?)?;
    assert_eq!(receipt.artifact_kind, "repro-bundle");
    Ok(())
}

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

#[test]
fn cli_failure_paths_write_canonical_failure_artifacts_to_files() -> CliResult<()> {
    let dir = temp_dir("cli-failure-files")?;
    let bad_suite = dir.join("bad-suite.preserves");
    let run_failure = dir.join("run.failure.preserves");
    std::fs::write(
        &bad_suite,
        r#"<harness-suite-v1 "molten.harness.suite.v1" "bad" 1
          <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
          <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
          [<send "producer" "missing" "hello">]>"#,
    )?;

    let failed_run = molten_cmd()
        .args(["test", "run"])
        .arg(&bad_suite)
        .args(["--report-out"])
        .arg(&run_failure)
        .output()?;
    assert_failure(&failed_run, "failing test run");
    let failure = molten::harness::parse_failure(&read_preserves(&run_failure)?)?;
    assert_eq!(failure.phase, "preflight");
    assert_eq!(failure.kind, "invalid-harness");
    assert!(failure.message.contains("unknown actor missing"));

    let good_report = dir.join("report.preserves");
    let suite = manifest_dir().join("examples/two-actor.preserves");
    let good_run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&good_report).output()?;
    assert_success(&good_run, "setup test run");

    let tampered_report = dir.join("tampered.report.preserves");
    let report_text = std::fs::read_to_string(&good_report)?;
    std::fs::write(&tampered_report, report_text.replacen("message-delivered", "message-tampered", 1))?;

    let replay_failure = dir.join("replay.failure.preserves");
    let failed_replay = molten_cmd()
        .args(["test", "replay"])
        .arg(&tampered_report)
        .args(["--failure-out"])
        .arg(&replay_failure)
        .output()?;
    assert_failure(&failed_replay, "failing test replay");
    let failure = molten::harness::parse_failure(&read_preserves(&replay_failure)?)?;
    assert_eq!(failure.phase, "replay");
    assert_eq!(failure.kind, "trace");

    let invalid_report = dir.join("invalid.report.preserves");
    std::fs::write(&invalid_report, "<not-a-harness-report>\n")?;

    let validate_failure = dir.join("validate.failure.preserves");
    let failed_validate = molten_cmd()
        .args(["test", "report", "validate"])
        .arg(&invalid_report)
        .args(["--failure-out"])
        .arg(&validate_failure)
        .output()?;
    assert_failure(&failed_validate, "failing test report validate");
    let failure = molten::harness::parse_failure(&read_preserves(&validate_failure)?)?;
    assert_eq!(failure.phase, "validate");
    assert_eq!(failure.kind, "invalid-harness");
    assert!(failure.message.contains("expected <harness-report-v1"));

    let export_failure = dir.join("export.failure.preserves");
    let failed_export = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(&invalid_report)
        .args(["--out"])
        .arg(dir.join("invalid-repro"))
        .args(["--failure-out"])
        .arg(&export_failure)
        .output()?;
    assert_failure(&failed_export, "failing test repro export");
    let failure = molten::harness::parse_failure(&read_preserves(&export_failure)?)?;
    assert_eq!(failure.phase, "export");
    assert_eq!(failure.kind, "invalid-harness");
    Ok(())
}

#[test]
fn cli_dogfood_receipts_and_nix_negative_verify_work() -> CliResult<()> {
    let dir = temp_dir("dogfood-receipts")?;
    let base = base_outputs(&dir)?;
    receipt_ops(&base)?;
    nix_bundle(&dir, &base)?;
    let keys = key_set(&dir)?;
    let members = member_files(&dir, &base, &keys)?;
    let bundles = bundle_checks(&dir, &base, &keys, &members)?;
    let promotion = promotion_summary(&dir, &keys, &bundles)?;
    missing_member_summary(&dir, &keys)?;
    let archive = archive_case(&dir, &promotion)?;
    archive_denials(&dir, &archive)?;
    wrong_signer_denial(&dir, &base, &members)?;
    revoked_key_denial(&dir, &keys, &bundles)?;
    stale_marker_denial(&dir, &base, &bundles)?;
    Ok(())
}

struct BaseOutputs {
    ledger: std::path::PathBuf,
    report: std::path::PathBuf,
    gate: std::path::PathBuf,
    replay_verify: std::path::PathBuf,
    replay_index: std::path::PathBuf,
    nix_evidence: std::path::PathBuf,
    nix_verify: std::path::PathBuf,
    bundle: std::path::PathBuf,
    report_ref: String,
    gate_ref: String,
}

struct Keys {
    ledger: std::path::PathBuf,
    key_ref: String,
}

struct MemberFiles {
    report: std::path::PathBuf,
    gate: std::path::PathBuf,
    replay_verify: std::path::PathBuf,
    replay_index: std::path::PathBuf,
    nix_evidence: std::path::PathBuf,
    nix_verify: std::path::PathBuf,
}

struct BundleChecks {
    path: std::path::PathBuf,
    keyring_verify: std::path::PathBuf,
}

struct PromotionSummary {
    summary_ref: String,
}

struct PromotionRefs {
    promotion_ref: String,
    summary_ref: String,
}

struct ArchiveCase {
    manifest: std::path::PathBuf,
    member_refs: Vec<(String, String)>,
}

fn base_outputs(dir: &std::path::Path) -> CliResult<BaseOutputs> {
    let state_root = dir.join("state");
    let report = dir.join("dogfood-report.preserves");
    let gate = dir.join("release-gate.preserves");
    let replay_verify = dir.join("replay-verify.preserves");
    let replay_index = dir.join("replay-evidence-index.preserves");
    let run = molten_cmd()
        .args(["dogfood", "local-node", "--state-root"])
        .arg(&state_root)
        .args(["--out"])
        .arg(&report)
        .args(["--release-gate-out"])
        .arg(&gate)
        .args(["--replay-verify-out"])
        .arg(&replay_verify)
        .args(["--replay-index-out"])
        .arg(&replay_index)
        .output()?;
    assert_success(&run, "dogfood local-node");
    assert!(stdout(&run).contains("decision=pass"));

    let report_value = read_preserves(&report)?;
    let parsed_report = molten::operator_dogfood::parse_dogfood_report(&report_value)?;
    assert_eq!(parsed_report.decision, "pass");
    let gate_ref = molten::preserves_rail::canonical_hash(&read_preserves(&gate)?)?;
    assert!(std::fs::read_to_string(&replay_verify)?.contains("deterministic-replay-verify-v1"));
    assert!(std::fs::read_to_string(&replay_index)?.contains("deterministic-replay-index-v1"));
    Ok(BaseOutputs {
        ledger: state_root.join("ledger"),
        report,
        gate,
        replay_verify,
        replay_index,
        nix_evidence: dir.join("nix-dogfood-evidence.preserves"),
        nix_verify: dir.join("nix-dogfood-verify.preserves"),
        bundle: dir.join("release-evidence-bundle.preserves"),
        report_ref: parsed_report.report_ref,
        gate_ref,
    })
}

fn receipt_ops(base: &BaseOutputs) -> CliResult<()> {
    let list = molten_cmd().args(["receipts", "list", "--ledger"]).arg(&base.ledger).output()?;
    assert_success(&list, "receipts list");
    assert!(stdout(&list).contains(&base.report_ref));
    assert!(stdout(&list).contains("dogfood-report"));

    let show = molten_cmd()
        .args(["receipts", "show"])
        .arg(&base.report_ref)
        .args(["--ledger"])
        .arg(&base.ledger)
        .output()?;
    assert_success(&show, "receipts show");
    assert!(stdout(&show).contains("operator dogfood report"));

    let validate = molten_cmd()
        .args(["receipts", "validate"])
        .arg(&base.report_ref)
        .args(["--ledger"])
        .arg(&base.ledger)
        .output()?;
    assert_success(&validate, "receipts validate");
    assert!(stdout(&validate).contains("receipts validate ok"));

    let exported = base.report.with_file_name("exported-dogfood-report.preserves");
    let export = molten_cmd()
        .args(["receipts", "export"])
        .arg(&base.report_ref)
        .args(["--ledger"])
        .arg(&base.ledger)
        .args(["--out"])
        .arg(&exported)
        .output()?;
    assert_success(&export, "receipts export");
    assert!(stdout(&export).contains("redaction=pass"));
    assert_eq!(molten::preserves_rail::canonical_hash(&read_preserves(&exported)?)?, base.report_ref);
    Ok(())
}

fn nix_bundle(dir: &std::path::Path, base: &BaseOutputs) -> CliResult<()> {
    std::fs::write(
        dir.join("dogfood-summary.txt"),
        format!("dogfood local-node decision=pass report={} release-gate={}\n", base.report_ref, base.gate_ref),
    )?;
    std::fs::write(dir.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n")?;
    let export_nix = molten_cmd()
        .args(["dogfood", "nix-release-export", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&base.nix_evidence)
        .output()?;
    assert_success(&export_nix, "dogfood nix-release-export");
    let verify_nix = molten_cmd()
        .args(["dogfood", "nix-release-verify", "--output-path"])
        .arg(dir)
        .args(["--evidence"])
        .arg(&base.nix_evidence)
        .args(["--receipt-out"])
        .arg(&base.nix_verify)
        .output()?;
    assert_success(&verify_nix, "dogfood nix-release-verify");
    std::fs::write(dir.join("nix-dogfood-verify.txt"), stdout(&verify_nix))?;
    let verify_receipt =
        molten::operator_dogfood::parse_nix_dogfood_verify_receipt(&read_preserves(&base.nix_verify)?)?;
    assert_eq!(verify_receipt.decision, "pass");

    let bundle_verify = dir.join("release-evidence-bundle-verify.preserves");
    let export_bundle = molten_cmd()
        .args(["dogfood", "release-bundle-export", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&base.bundle)
        .output()?;
    assert_success(&export_bundle, "dogfood release-bundle-export");
    let verify_bundle = molten_cmd()
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&bundle_verify)
        .output()?;
    assert_success(&verify_bundle, "dogfood release-bundle-verify");
    std::fs::write(dir.join("release-evidence-bundle-verify.txt"), stdout(&verify_bundle))?;
    let parsed =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&bundle_verify)?)?;
    assert_eq!(parsed.decision, "pass");
    Ok(())
}

fn key_set(dir: &std::path::Path) -> CliResult<Keys> {
    let ledger = dir.join("signed-keyring");
    let key_import = molten_cmd()
        .args(["receipts", "key", "import", "--ledger"])
        .arg(&ledger)
        .args([
            "--key-id",
            "release-key-1",
            "--signer",
            "release-signer",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
        ])
        .output()?;
    assert_success(&key_import, "receipts key import");
    let key_import_stdout = stdout(&key_import);
    std::fs::write(dir.join("signed-keyring-import.txt"), &key_import_stdout)?;
    let key_ref = key_import_stdout
        .split_whitespace()
        .find_map(|field| field.strip_prefix("key="))
        .ok_or_else(|| test_error("key import output did not include key ref"))?
        .to_string();
    let key_list = molten_cmd().args(["receipts", "key", "list", "--ledger"]).arg(&ledger).output()?;
    assert_success(&key_list, "receipts key list");
    assert!(stdout(&key_list).contains("release-key-1"));
    let key_show = molten_cmd()
        .args(["receipts", "key", "show"])
        .arg(&key_ref)
        .args(["--ledger"])
        .arg(&ledger)
        .output()?;
    assert_success(&key_show, "receipts key show");
    assert!(stdout(&key_show).contains("evidence-only=pass"));
    rotate_seed_key(&ledger)?;
    Ok(Keys { ledger, key_ref })
}

fn rotate_seed_key(ledger: &std::path::Path) -> CliResult<()> {
    let rotate_seed = molten_cmd()
        .args(["receipts", "key", "import", "--ledger"])
        .arg(ledger)
        .args([
            "--key-id",
            "rotate-key-1",
            "--signer",
            "rotate-signer",
            "--trust-root",
            "rotate-root",
            "--key",
            "rotate-key",
        ])
        .output()?;
    assert_success(&rotate_seed, "receipts key import rotate seed");
    let rotate_key_ref = stdout(&rotate_seed)
        .split_whitespace()
        .find_map(|field| field.strip_prefix("key="))
        .ok_or_else(|| test_error("rotate seed output did not include key ref"))?
        .to_string();
    let rotate_success = molten_cmd()
        .args(["receipts", "key", "rotate"])
        .arg(&rotate_key_ref)
        .args(["--ledger"])
        .arg(ledger)
        .args(["--new-key-id", "rotate-key-2", "--new-key", "rotate-key-2"])
        .output()?;
    assert_success(&rotate_success, "receipts key rotate");
    assert!(stdout(&rotate_success).contains("new-key-id=rotate-key-2"));
    Ok(())
}

fn member_files(dir: &std::path::Path, base: &BaseOutputs, keys: &Keys) -> CliResult<MemberFiles> {
    let members = MemberFiles {
        report: dir.join("dogfood-report.signed.preserves"),
        gate: dir.join("release-gate.signed.preserves"),
        replay_verify: dir.join("replay-verify.signed.preserves"),
        replay_index: dir.join("replay-evidence-index.signed.preserves"),
        nix_evidence: dir.join("nix-dogfood-evidence.signed.preserves"),
        nix_verify: dir.join("nix-dogfood-verify.signed.preserves"),
    };
    for (receipt_path, signed_path) in [
        (&base.report, &members.report),
        (&base.gate, &members.gate),
        (&base.replay_verify, &members.replay_verify),
        (&base.replay_index, &members.replay_index),
        (&base.nix_evidence, &members.nix_evidence),
        (&base.nix_verify, &members.nix_verify),
    ] {
        let signed = molten_cmd()
            .args(["receipts", "sign"])
            .arg(receipt_path)
            .args(["--out"])
            .arg(signed_path)
            .args([
                "--signer",
                "release-signer",
                "--purpose",
                "release-evidence",
                "--trust-root",
                "release-root",
                "--key",
                "release-key",
            ])
            .output()?;
        assert_success(&signed, "receipts sign release member");
    }
    verify_member_file(base, keys, &members.report)?;
    Ok(members)
}

fn verify_member_file(base: &BaseOutputs, keys: &Keys, signed_report: &std::path::Path) -> CliResult<()> {
    let verify_signed = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(signed_report)
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
            "--signer",
            "release-signer",
            "--subject-ref",
        ])
        .arg(&base.report_ref)
        .output()?;
    assert_success(&verify_signed, "receipts verify-signed release member");
    assert!(stdout(&verify_signed).contains("evidence-only=pass"));
    let verify_signed_keyring = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(signed_report)
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args([
            "--key-ref",
            &keys.key_ref,
            "--signer",
            "release-signer",
            "--subject-ref",
        ])
        .arg(&base.report_ref)
        .output()?;
    assert_success(&verify_signed_keyring, "receipts verify-signed release member with keyring");
    assert!(stdout(&verify_signed_keyring).contains("keyring=current"));
    Ok(())
}

fn bundle_checks(
    dir: &std::path::Path,
    base: &BaseOutputs,
    keys: &Keys,
    members: &MemberFiles,
) -> CliResult<BundleChecks> {
    let direct = dir.join("release-evidence-bundle-verify-signed.preserves");
    let mut verify_direct = molten_cmd();
    verify_direct
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&direct)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key",
            "release-key",
            "--signed-signer",
            "release-signer",
        ]);
    add_member_args(&mut verify_direct, members);
    let direct_output = verify_direct.output()?;
    assert_success(&direct_output, "dogfood release-bundle-verify signed members");
    let direct_receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&direct)?)?;
    assert_eq!(direct_receipt.decision, "pass");

    let keyring_verify = dir.join("release-evidence-bundle-verify-keyring.preserves");
    let mut verify_keyring = molten_cmd();
    verify_keyring
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&keyring_verify)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args(["--signed-signer", "release-signer"]);
    add_member_args(&mut verify_keyring, members);
    let keyring_output = verify_keyring.output()?;
    assert_success(&keyring_output, "dogfood release-bundle-verify signed keyring members");
    let keyring_receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&keyring_verify)?)?;
    assert_eq!(keyring_receipt.decision, "pass");
    Ok(BundleChecks {
        path: base.bundle.clone(),
        keyring_verify,
    })
}

fn add_member_args(command: &mut std::process::Command, members: &MemberFiles) {
    for signed_path in [
        &members.report,
        &members.gate,
        &members.replay_verify,
        &members.replay_index,
        &members.nix_evidence,
        &members.nix_verify,
    ] {
        command.args(["--signed-member"]).arg(signed_path);
    }
}

fn promotion_summary(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<PromotionSummary> {
    let promotion_receipt_path = dir.join("release-promotion-gate.preserves");
    let promotion = molten_cmd()
        .args(["dogfood", "release-promote", "--output-path"])
        .arg(dir)
        .args(["--bundle-verify"])
        .arg(&bundles.keyring_verify)
        .args(["--receipt-out"])
        .arg(&promotion_receipt_path)
        .args(["--signed-key-ledger"])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args([
            "--signed-signer",
            "release-signer",
            "--signed-trust-root",
            "release-root",
            "--source-evidence",
            "source:cli-dogfood-fixture",
            "--octet-evidence",
            "octet:clean-fixture",
            "--cairn-evidence",
            "cairn:strict-fixture",
        ])
        .output()?;
    assert_success(&promotion, "dogfood release-promote");
    assert!(stdout(&promotion).contains("decision=pass"));
    std::fs::write(dir.join("release-promotion-gate.txt"), stdout(&promotion))?;
    let promotion_receipt =
        molten::operator_dogfood::parse_release_promotion_gate_receipt(&read_preserves(&promotion_receipt_path)?)?;
    assert_eq!(promotion_receipt.decision, "pass");
    sign_and_verify_promotion(dir, keys, &promotion_receipt_path, &promotion_receipt.receipt_ref)?;
    let refs = write_promotion_summary(
        dir,
        keys,
        "release-promotion-summary.preserves",
        Some("release-promotion-summary.txt"),
        "pass",
    )?;
    assert_eq!(refs.promotion_ref, promotion_receipt.receipt_ref);
    Ok(PromotionSummary {
        summary_ref: refs.summary_ref,
    })
}

fn sign_and_verify_promotion(
    dir: &std::path::Path,
    keys: &Keys,
    receipt_path: &std::path::Path,
    receipt_ref: &str,
) -> CliResult<()> {
    let signed_path = dir.join("release-promotion-gate.signed.preserves");
    let sign_promotion = molten_cmd()
        .args(["receipts", "sign"])
        .arg(receipt_path)
        .args(["--out"])
        .arg(&signed_path)
        .args([
            "--signer",
            "release-signer",
            "--purpose",
            "release-promotion",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
        ])
        .output()?;
    assert_success(&sign_promotion, "sign release promotion receipt");
    let verify_signed_promotion = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(&signed_path)
        .args([
            "--purpose",
            "release-promotion",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--key-ref"])
        .arg(&keys.key_ref)
        .args(["--signer", "release-signer", "--subject-ref"])
        .arg(receipt_ref)
        .output()?;
    assert_success(&verify_signed_promotion, "verify signed release promotion receipt");
    std::fs::write(dir.join("release-promotion-gate-signed-verify.txt"), stdout(&verify_signed_promotion))?;
    Ok(())
}

fn write_promotion_summary(
    dir: &std::path::Path,
    keys: &Keys,
    name: &str,
    stdout_name: Option<&str>,
    expected_decision: &str,
) -> CliResult<PromotionRefs> {
    let summary_path = dir.join(name);
    let summary = molten_cmd()
        .args(["dogfood", "release-promotion-summary", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&summary_path)
        .args(["--signed-key-ledger"])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args([
            "--signed-signer",
            "release-signer",
            "--signed-trust-root",
            "release-root",
        ])
        .output()?;
    assert_success(&summary, "dogfood release-promotion-summary");
    if let Some(stdout_name) = stdout_name {
        std::fs::write(dir.join(stdout_name), stdout(&summary))?;
    }
    let parsed = molten::operator_dogfood::parse_release_promotion_summary(&read_preserves(&summary_path)?)?;
    assert_eq!(parsed.decision, expected_decision);
    Ok(PromotionRefs {
        promotion_ref: parsed.promotion_ref,
        summary_ref: parsed.summary_ref,
    })
}

fn missing_member_summary(dir: &std::path::Path, keys: &Keys) -> CliResult<()> {
    let signed_path = dir.join("release-promotion-gate.signed.preserves");
    let missing_path = dir.join("release-promotion-gate.signed.missing");
    std::fs::rename(&signed_path, &missing_path)?;
    let _refs = write_promotion_summary(dir, keys, "release-promotion-summary-missing-signed.preserves", None, "deny")?;
    std::fs::rename(&missing_path, &signed_path)?;
    Ok(())
}

fn archive_case(dir: &std::path::Path, promotion: &PromotionSummary) -> CliResult<ArchiveCase> {
    let archive_path = dir.join("release-evidence.tar.zst");
    let manifest = dir.join("release-export-manifest.preserves");
    let release_export = molten_cmd()
        .args(["dogfood", "release-export", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&archive_path)
        .args(["--manifest-out"])
        .arg(&manifest)
        .output()?;
    assert_success(&release_export, "dogfood release-export");
    assert!(stdout(&release_export).contains("release-export manifest="));
    let parsed_export = molten::operator_dogfood::parse_release_export_manifest(&read_preserves(&manifest)?)?;
    assert_eq!(parsed_export.promotion_summary_ref, promotion.summary_ref);

    let verify_path = dir.join("release-export-verify.preserves");
    let verify = molten_cmd()
        .args(["dogfood", "release-export-verify", "--bundle"])
        .arg(&archive_path)
        .args(["--receipt-out"])
        .arg(&verify_path)
        .output()?;
    assert_success(&verify, "dogfood release-export-verify");
    assert!(stdout(&verify).contains("decision=pass"));
    let parsed_verify = molten::operator_dogfood::parse_release_export_verify_receipt(&read_preserves(&verify_path)?)?;
    assert_eq!(parsed_verify.decision, "pass");
    Ok(ArchiveCase {
        manifest,
        member_refs: parsed_export.member_refs,
    })
}

fn archive_denials(dir: &std::path::Path, archive: &ArchiveCase) -> CliResult<()> {
    let missing_manifest = dir.join("release-evidence-missing-manifest.tar.zst");
    write_release_export_test_archive(dir, &missing_manifest, None, &archive.member_refs)?;
    verify_archive_deny(dir, &missing_manifest, "release-export-verify-missing-manifest.preserves")?;

    let extra = dir.join("release-evidence-extra.tar.zst");
    write_release_export_test_archive_with_extra(
        dir,
        &extra,
        &archive.manifest,
        &archive.member_refs,
        ExtraArchiveMember {
            name: "unexpected.txt",
            bytes: b"extra evidence",
        },
    )?;
    verify_archive_deny(dir, &extra, "release-export-verify-extra.preserves")?;

    let tampered = dir.join("release-evidence-tampered.tar.zst");
    write_release_export_test_archive_with_tamper(dir, &tampered, &archive.manifest, &archive.member_refs)?;
    verify_archive_deny(dir, &tampered, "release-export-verify-tampered.preserves")?;

    let duplicate = dir.join("release-evidence-duplicate.tar.zst");
    write_release_export_test_archive_with_duplicate(dir, &duplicate, &archive.manifest, &archive.member_refs)?;
    verify_archive_deny(dir, &duplicate, "release-export-verify-duplicate.preserves")?;
    Ok(())
}

fn verify_archive_deny(dir: &std::path::Path, archive_path: &std::path::Path, receipt_name: &str) -> CliResult<()> {
    let receipt_path = dir.join(receipt_name);
    let verify = molten_cmd()
        .args(["dogfood", "release-export-verify", "--bundle"])
        .arg(archive_path)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .output()?;
    assert_success(&verify, "dogfood release-export-verify emits deny receipt");
    let parsed = molten::operator_dogfood::parse_release_export_verify_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(parsed.decision, "deny");
    Ok(())
}

fn wrong_signer_denial(dir: &std::path::Path, base: &BaseOutputs, members: &MemberFiles) -> CliResult<()> {
    let receipt_path = dir.join("release-evidence-bundle-verify-wrong-signer.preserves");
    let mut verify = molten_cmd();
    verify
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key",
            "release-key",
            "--signed-signer",
            "wrong-signer",
        ]);
    add_member_args(&mut verify, members);
    let output = verify.output()?;
    assert_success(&output, "dogfood release-bundle-verify wrong signer");
    let receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("signer")));
    Ok(())
}

fn revoked_key_denial(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<()> {
    let revoke_key = molten_cmd()
        .args(["receipts", "key", "revoke"])
        .arg(&keys.key_ref)
        .args(["--ledger"])
        .arg(&keys.ledger)
        .args(["--reason", "test-revoked"])
        .output()?;
    assert_success(&revoke_key, "receipts key revoke");
    let revoked_verify = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(dir.join("dogfood-report.signed.preserves"))
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--key-ref"])
        .arg(&keys.key_ref)
        .output()?;
    assert_failure(&revoked_verify, "revoked key denies signed receipt");
    assert!(stderr(&revoked_verify).contains("revoked"));
    revoked_promotion_denial(dir, keys, bundles)?;
    let rotate_key = molten_cmd()
        .args(["receipts", "key", "rotate"])
        .arg(&keys.key_ref)
        .args(["--ledger"])
        .arg(&keys.ledger)
        .args(["--new-key-id", "release-key-2", "--new-key", "release-key-2"])
        .output()?;
    assert_failure(&rotate_key, "cannot rotate already revoked key");
    assert!(stderr(&rotate_key).contains("already revoked"));
    Ok(())
}

fn revoked_promotion_denial(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<()> {
    let receipt_path = dir.join("release-promotion-gate-revoked.preserves");
    let revoked_promotion = molten_cmd()
        .args(["dogfood", "release-promote", "--output-path"])
        .arg(dir)
        .args(["--bundle-verify"])
        .arg(&bundles.keyring_verify)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .args(["--signed-key-ledger"])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args([
            "--signed-signer",
            "release-signer",
            "--signed-trust-root",
            "release-root",
            "--source-evidence",
            "source:cli-dogfood-fixture",
            "--octet-evidence",
            "octet:clean-fixture",
            "--cairn-evidence",
            "cairn:strict-fixture",
        ])
        .output()?;
    assert_success(&revoked_promotion, "dogfood release-promote revoked key emits deny receipt");
    let receipt = molten::operator_dogfood::parse_release_promotion_gate_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(receipt.decision, "deny");
    Ok(())
}

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
        .args(["--authority-context-ref"])
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
fn cli_node_control_request_and_deny_receipt_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-control")?;
    let request = dir.join("node-control-request.preserves");
    let receipt = dir.join("node-control-receipt.preserves");
    let provenance = dir.join("node-control-provenance.preserves");
    let payload_ref = test_ref("node-control-payload")?;
    let startup_ref = test_ref("node-startup")?;

    let provenance_out = molten_cmd()
        .args(["test", "node", "provenance-fixture", "--artifact-ref"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&provenance)
        .output()?;
    assert_success(&provenance_out, "node provenance fixture");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&provenance)?), "provenance-record");

    let request_out = molten_cmd()
        .args(["test", "node", "control-request", "--operation", "gate", "--payload"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&request)
        .output()?;
    assert_success(&request_out, "node control request");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&request)?), "node-control-request");

    let deny = molten_cmd()
        .args(["test", "node", "control-deny"])
        .arg(&request)
        .args(["--startup"])
        .arg(&startup_ref)
        .args(["--diagnostic", "missing authority/resource", "--receipt-out"])
        .arg(&receipt)
        .output()?;
    assert_success(&deny, "node control deny");
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&receipt_value), "node-control-receipt");
    let text = molten::preserves_rail::to_text(&receipt_value)?;
    assert!(text.contains("missing authority/resource"));
    Ok(())
}

#[test]
fn cli_node_control_ingress_build_publish_deliver_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-control-ingress")?;
    let state_root = dir.join("node-state");
    let request = dir.join("request.preserves");
    let envelope = dir.join("ingress-envelope.preserves");
    let publish_receipt = dir.join("ingress-publish.preserves");
    let deliver_receipt = dir.join("ingress-deliver.preserves");
    let loop_receipt = dir.join("loop.preserves");
    let authority_ref = test_ref("ingress-authority")?;
    let policy_ref = test_ref("ingress-policy")?;
    let resource_ref = test_ref("ingress-resource")?;
    let bootstrap_ref = test_ref("ingress-bootstrap")?;

    start_state(&state_root, "node:cli-ingress", "node ingress init", "node ingress run")?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node ingress request")?;
    let envelope_ref = write_envelope(EnvelopeArgs {
        path: &envelope,
        request: &request,
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
        label: "node ingress build",
    })?;
    publish_envelope(&state_root, &envelope, &publish_receipt, "node ingress publish")?;
    deliver_envelope(&state_root, &envelope_ref, &deliver_receipt, "node ingress deliver")?;
    let loop_out = run_once(&state_root, &loop_receipt, "node ingress loop")?;
    assert!(stdout(&loop_out).contains("processed=1"));
    Ok(())
}

#[test]
fn cli_node_live_ingress_loopback_enqueues_request() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-ingress")?;
    let state_root = dir.join("node-state");
    let request = dir.join("status.preserves");
    let publish_receipt = dir.join("publish.preserves");
    let receive_receipt = dir.join("receive.preserves");
    let authority_grant = dir.join("authority-grant.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let policy_ref = test_ref("live-policy")?;
    let resource_ref = test_ref("live-resource")?;

    start_state(&state_root, "node:cli-live", "node live init", "node live run")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: &state_root,
        grant: &authority_grant,
        peer: "peer:cli-live",
        node: "node:cli-live",
        policy_ref: &policy_ref,
        label: "node live authority grant",
    })?;
    ticket_export(&state_root, &live_ticket, &policy_ref, "node live ticket export")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: &state_root,
        receipt: &peer_admission,
        peer: "peer:cli-live",
        policy_ref: &policy_ref,
        ticket: &live_ticket,
        label: "node live peer admit",
    })?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node live status request")?;
    run_loopback(LoopbackArgs {
        root: &state_root,
        request: &request,
        publish: &publish_receipt,
        receive: &receive_receipt,
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
    })?;
    Ok(())
}

#[test]
fn cli_node_live_ticket_and_authority_import_receipts_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-import")?;
    let receiver_root = dir.join("receiver-node");
    let sender_root = dir.join("sender-node");
    let bundle_sender_root = dir.join("bundle-sender-node");
    let bundle_apply_root = dir.join("bundle-apply-node");
    let authority_grant = dir.join("authority-grant.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let missing_import_request = dir.join("missing-import-request.preserves");
    let missing_import_send_receipt = dir.join("missing-import-send.preserves");
    let workflow_bundle = dir.join("workflow-bundle.preserves");
    let bundle_export = dir.join("workflow-bundle-export.preserves");
    let bundle_verify = dir.join("workflow-bundle-verify.preserves");
    let bundle_gate = dir.join("workflow-bundle-gate.preserves");
    let bundle_apply = dir.join("workflow-bundle-apply.preserves");
    let bundle_reconcile = dir.join("workflow-bundle-reconcile.preserves");
    let bundle_ack = dir.join("workflow-bundle-ack.preserves");
    let bundle_ack_export = dir.join("workflow-bundle-ack-export.preserves");
    let bundle_ack_import = dir.join("workflow-bundle-ack-import.preserves");
    let bundle_protocol_gate = dir.join("workflow-bundle-protocol-gate.preserves");
    let bundle_import = dir.join("workflow-bundle-import.preserves");
    let bundle_import_send_receipt = dir.join("bundle-import-send.preserves");
    let ticket_import = dir.join("ticket-import.preserves");
    let grant_import = dir.join("grant-import.preserves");
    let policy_ref = test_ref("live-import-policy")?;
    let resource_ref = test_ref("live-import-resource")?;
    let case = LiveImportCase {
        receiver_root: &receiver_root,
        sender_root: &sender_root,
        bundle_sender_root: &bundle_sender_root,
        bundle_apply_root: &bundle_apply_root,
        authority_grant: &authority_grant,
        live_ticket: &live_ticket,
        peer_admission: &peer_admission,
        missing_import_request: &missing_import_request,
        missing_import_send_receipt: &missing_import_send_receipt,
        workflow_bundle: &workflow_bundle,
        bundle_export: &bundle_export,
        bundle_verify: &bundle_verify,
        bundle_gate: &bundle_gate,
        bundle_apply: &bundle_apply,
        bundle_reconcile: &bundle_reconcile,
        bundle_ack: &bundle_ack,
        bundle_ack_export: &bundle_ack_export,
        bundle_ack_import: &bundle_ack_import,
        bundle_protocol_gate: &bundle_protocol_gate,
        bundle_import: &bundle_import,
        bundle_import_send_receipt: &bundle_import_send_receipt,
        ticket_import: &ticket_import,
        grant_import: &grant_import,
        policy_ref: policy_ref.as_str(),
        resource_ref: resource_ref.as_str(),
    };

    let refs = prepare_live_import_case(&case)?;
    expect_missing_imports(&case, &refs)?;
    export_and_verify_case(&case)?;
    gate_and_apply_case(&case)?;
    review_missing_receiver(&case)?;
    import_case_and_retry(&case, &refs)?;
    import_ticket_and_grant(&case)?;
    Ok(())
}

struct LiveImportCase<'a> {
    receiver_root: &'a std::path::Path,
    sender_root: &'a std::path::Path,
    bundle_sender_root: &'a std::path::Path,
    bundle_apply_root: &'a std::path::Path,
    authority_grant: &'a std::path::Path,
    live_ticket: &'a std::path::Path,
    peer_admission: &'a std::path::Path,
    missing_import_request: &'a std::path::Path,
    missing_import_send_receipt: &'a std::path::Path,
    workflow_bundle: &'a std::path::Path,
    bundle_export: &'a std::path::Path,
    bundle_verify: &'a std::path::Path,
    bundle_gate: &'a std::path::Path,
    bundle_apply: &'a std::path::Path,
    bundle_reconcile: &'a std::path::Path,
    bundle_ack: &'a std::path::Path,
    bundle_ack_export: &'a std::path::Path,
    bundle_ack_import: &'a std::path::Path,
    bundle_protocol_gate: &'a std::path::Path,
    bundle_import: &'a std::path::Path,
    bundle_import_send_receipt: &'a std::path::Path,
    ticket_import: &'a std::path::Path,
    grant_import: &'a std::path::Path,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

struct LiveImportRefs {
    authority_ref: String,
    bootstrap_ref: String,
}

fn init_node(root: &std::path::Path, node_id: &str, label: &str) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(root)
            .args(["--node-id", node_id])
            .output()?,
        label,
    );
    Ok(())
}

fn prepare_live_import_case(case: &LiveImportCase<'_>) -> CliResult<LiveImportRefs> {
    start_state(case.receiver_root, "node:cli-live-import", "receiver init", "receiver run")?;
    init_node(case.sender_root, "node:cli-live-import-sender", "sender init")?;
    init_node(case.bundle_sender_root, "node:cli-live-bundle-sender", "bundle sender init")?;
    init_node(case.bundle_apply_root, "node:cli-live-bundle-apply", "bundle apply init")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: case.receiver_root,
        grant: case.authority_grant,
        peer: "peer:cli-live-import",
        node: "node:cli-live-import",
        policy_ref: case.policy_ref,
        label: "receiver authority grant",
    })?;
    ticket_export(case.receiver_root, case.live_ticket, case.policy_ref, "receiver live ticket")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: case.receiver_root,
        receipt: case.peer_admission,
        peer: "peer:cli-live-import",
        policy_ref: case.policy_ref,
        ticket: case.live_ticket,
        label: "receiver peer admit",
    })?;
    write_status_request(
        case.missing_import_request,
        &authority_ref,
        case.policy_ref,
        case.resource_ref,
        "missing import request",
    )?;
    Ok(LiveImportRefs {
        authority_ref,
        bootstrap_ref,
    })
}

fn expect_missing_imports(case: &LiveImportCase<'_>, refs: &LiveImportRefs) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(case.sender_root)
        .arg(case.missing_import_request)
        .arg(case.live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "wrong-topic",
            "--peer-bootstrap",
        ])
        .arg(&refs.bootstrap_ref)
        .args(["--authority"])
        .arg(&refs.authority_ref)
        .args(["--policy"])
        .arg(case.policy_ref)
        .args(["--resource"])
        .arg(case.resource_ref)
        .args(["--receipt-out"])
        .arg(case.missing_import_send_receipt)
        .output()?;
    assert_success(&output, "live send missing imports deny receipt");
    let text = molten::preserves_rail::to_text(&read_preserves(case.missing_import_send_receipt)?)?;
    assert!(text.contains("live-ticket-import"));
    assert!(text.contains("authority-grant-import"));
    assert!(text.contains("ticket topic node-control does not match expected wrong-topic"));
    assert!(text.contains("receiver-ticket-expected"));
    assert!(text.contains("sender-state-root-evidence"));
    Ok(())
}

fn export_and_verify_case(case: &LiveImportCase<'_>) -> CliResult<()> {
    let exported = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-export", "--ticket"])
        .arg(case.live_ticket)
        .args(["--peer-admission"])
        .arg(case.peer_admission)
        .args(["--authority-grant"])
        .arg(case.authority_grant)
        .args(["--receipt"])
        .arg(case.missing_import_send_receipt)
        .args(["--out"])
        .arg(case.workflow_bundle)
        .args(["--receipt-out"])
        .arg(case.bundle_export)
        .output()?;
    assert_success(&exported, "live workflow bundle export");
    assert!(stdout(&exported).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.workflow_bundle)?),
        "node-control-live-workflow-bundle"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_export)?),
        "node-control-live-workflow-bundle-export-receipt"
    );

    let verified = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-verify"])
        .arg(case.workflow_bundle)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_verify)
        .output()?;
    assert_success(&verified, "live workflow bundle verify");
    assert!(stdout(&verified).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_verify)?),
        "node-control-live-workflow-bundle-verify-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_verify)?)?;
    assert!(text.contains("verify-receipt-is-not-authority"));
    Ok(())
}

fn gate_and_apply_case(case: &LiveImportCase<'_>) -> CliResult<()> {
    let gated = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-gate"])
        .arg(case.workflow_bundle)
        .args(["--verify-receipt"])
        .arg(case.bundle_verify)
        .args([
            "--require-verify-receipt",
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_gate)
        .output()?;
    assert_success(&gated, "live workflow bundle gate");
    assert!(stdout(&gated).contains("decision=pass"));
    assert!(stdout(&gated).contains("next-step=import-bundle"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_gate)?),
        "node-control-live-workflow-bundle-gate-receipt"
    );
    let gate_text = molten::preserves_rail::to_text(&read_preserves(case.bundle_gate)?)?;
    assert!(gate_text.contains("gate-receipt-is-not-authority"));

    let applied = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-apply", "--state-root"])
        .arg(case.bundle_apply_root)
        .arg(case.workflow_bundle)
        .args(["--gate-receipt"])
        .arg(case.bundle_gate)
        .args([
            "--require-gate-receipt",
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_apply)
        .output()?;
    assert_success(&applied, "live workflow bundle apply");
    assert!(stdout(&applied).contains("decision=pass"));
    assert!(stdout(&applied).contains("next-step=dry-run-or-send-request"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_apply)?),
        "node-control-live-workflow-bundle-apply-receipt"
    );
    let apply_text = molten::preserves_rail::to_text(&read_preserves(case.bundle_apply)?)?;
    assert!(apply_text.contains("apply-receipt-is-not-authority"));
    Ok(())
}

fn review_missing_receiver(case: &LiveImportCase<'_>) -> CliResult<()> {
    reconcile_missing_receiver(case)?;
    export_missing_ack(case)?;
    import_missing_ack(case)?;
    gate_missing_protocol(case)?;
    Ok(())
}

fn reconcile_missing_receiver(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-reconcile"])
        .arg(case.bundle_apply)
        .args(["--receipt-out"])
        .arg(case.bundle_reconcile)
        .output()?;
    assert_success(&output, "live workflow bundle reconcile missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert!(stdout(&output).contains("next-step=wait-or-import-receiver-ingress"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_reconcile)?),
        "node-control-live-workflow-bundle-reconcile-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_reconcile)?)?;
    assert!(text.contains("reconcile-receipt-is-not-authority"));
    Ok(())
}

fn export_missing_ack(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-export"])
        .arg(case.bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(case.bundle_reconcile)
        .args(["--out"])
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_ack_export)
        .output()?;
    assert_success(&output, "live workflow bundle ack export missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert!(stdout(&output).contains("next-step=collect-receiver-evidence"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack)?),
        "node-control-live-workflow-bundle-ack"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack_export)?),
        "node-control-live-workflow-bundle-ack-export-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_ack)?)?;
    assert!(text.contains("ack-bundle-is-not-authority"));
    Ok(())
}

fn import_missing_ack(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-import", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_ack_import)
        .output()?;
    assert_success(&output, "live workflow bundle ack import missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack_import)?),
        "node-control-live-workflow-bundle-ack-import-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_ack_import)?)?;
    assert!(text.contains("ack-import-is-not-authority"));
    Ok(())
}

fn gate_missing_protocol(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-protocol-gate"])
        .arg(case.workflow_bundle)
        .args(["--gate-receipt"])
        .arg(case.bundle_gate)
        .args(["--apply-receipt"])
        .arg(case.bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(case.bundle_reconcile)
        .args(["--ack"])
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_protocol_gate)
        .output()?;
    assert_success(&output, "live workflow bundle protocol gate missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_protocol_gate)?),
        "protocol-session-gate-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_protocol_gate)?)?;
    assert!(text.contains("ack receiver decision deny"));
    assert!(text.contains("protocol-session-gate-is-not-authority"));
    Ok(())
}

fn import_case_and_retry(case: &LiveImportCase<'_>, refs: &LiveImportRefs) -> CliResult<()> {
    let imported = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-import", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.workflow_bundle)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_import)
        .output()?;
    assert_success(&imported, "live workflow bundle import");
    assert!(stdout(&imported).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_import)?),
        "node-control-live-workflow-bundle-import-receipt"
    );

    let sent = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.missing_import_request)
        .arg(case.live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "node-control",
            "--peer-bootstrap",
        ])
        .arg(&refs.bootstrap_ref)
        .args(["--authority"])
        .arg(&refs.authority_ref)
        .args(["--policy"])
        .arg(case.policy_ref)
        .args(["--resource"])
        .arg(case.resource_ref)
        .args(["--receipt-out"])
        .arg(case.bundle_import_send_receipt)
        .output()?;
    assert_success(&sent, "live send after workflow bundle import");
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_import_send_receipt)?)?;
    assert!(text.contains("ticket has no endpoint addresses"));
    assert!(!text.contains("authority-grant-import"));
    assert!(!text.contains("peer admission unavailable in sender state root"));
    assert!(!text.contains("authority grant unavailable in sender state root"));
    Ok(())
}

fn import_ticket_and_grant(case: &LiveImportCase<'_>) -> CliResult<()> {
    let ticket = molten_cmd()
        .args(["test", "node", "live-ticket-import", "--state-root"])
        .arg(case.sender_root)
        .arg(case.live_ticket)
        .args(["--peer-admission"])
        .arg(case.peer_admission)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--receipt-out",
        ])
        .arg(case.ticket_import)
        .output()?;
    assert_success(&ticket, "sender live ticket import");
    assert!(stdout(&ticket).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.ticket_import)?),
        "node-control-live-ticket-import-receipt"
    );

    let grant = molten_cmd()
        .args(["test", "node", "authority-grant-import", "--state-root"])
        .arg(case.sender_root)
        .arg(case.authority_grant)
        .args([
            "--peer",
            "peer:cli-live-import",
            "--node",
            "node:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.grant_import)
        .output()?;
    assert_success(&grant, "sender authority grant import");
    assert!(stdout(&grant).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.grant_import)?),
        "node-control-authority-grant-import-receipt"
    );
    Ok(())
}

#[test]
fn cli_node_live_send_denies_offline_ticket_without_addresses() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-send")?;
    let state_root = dir.join("node-state");
    let request = dir.join("status.preserves");
    let ticket = dir.join("live-ticket.preserves");
    let send_receipt = dir.join("send.preserves");
    let operation_mismatch_receipt = dir.join("operation-mismatch.preserves");
    let transport_receipt = dir.join("transport.preserves");
    let authority_grant = dir.join("authority-grant.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let service_receipt = dir.join("service.preserves");
    let workflow_receipt = dir.join("workflow.preserves");
    let policy_ref = test_ref("live-send-policy")?;
    let resource_ref = test_ref("live-send-resource")?;
    let wrong_operation_ref = test_ref("wrong-live-send-operation")?;

    start_state(&state_root, "node:cli-live-send", "node live send init", "node live send run")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: &state_root,
        grant: &authority_grant,
        peer: "peer:cli-live-send",
        node: "node:cli-live-send",
        policy_ref: &policy_ref,
        label: "node live send authority grant",
    })?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node live send request")?;
    ticket_export(&state_root, &ticket, &policy_ref, "node live send ticket")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: &state_root,
        receipt: &peer_admission,
        peer: "peer:cli-live-send",
        policy_ref: &policy_ref,
        ticket: &ticket,
        label: "node live send peer admit",
    })?;
    let send_args = SendArgs {
        root: &state_root,
        request: &request,
        ticket: &ticket,
        peer: "peer:cli-live-send",
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
    };
    expect_no_address(&send_args, &transport_receipt, &send_receipt)?;
    expect_mismatch(&send_args, &wrong_operation_ref, &operation_mismatch_receipt)?;
    assert_success(
        &molten_cmd()
            .args(["test", "node", "serve", "--state-root"])
            .arg(&state_root)
            .args(["--max-ticks", "1", "--receipt-out"])
            .arg(&service_receipt)
            .output()?,
        "node live send service receipt",
    );
    expect_missing_receive(BundleArgs {
        root: &state_root,
        ticket: &ticket,
        peer_admission: &peer_admission,
        authority_grant: &authority_grant,
        send_receipt: &send_receipt,
        service_receipt: &service_receipt,
        receipt: &workflow_receipt,
    })?;
    Ok(())
}

#[test]
fn cli_node_serve_live_iroh_empty_listener_receipt() -> CliResult<()> {
    let dir = temp_dir("cli-node-serve-live")?;
    let state_root = dir.join("node-state");
    let listener_receipt = dir.join("listener.preserves");
    let service_receipt = dir.join("service.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-serve-live"])
            .output()?,
        "node serve live init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node serve live run",
    );
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--live-iroh", "--live-max-events", "0", "--service-receipt-out"])
        .arg(&service_receipt)
        .args(["--live-ticket-out"])
        .arg(&live_ticket)
        .args(["--receipt-out"])
        .arg(&listener_receipt)
        .output()?;
    assert_success(&served, "node serve live listener");
    assert!(stdout(&served).contains("node serve live-iroh"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&listener_receipt)?),
        "node-control-live-listener-receipt"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&service_receipt)?),
        "node-control-service-run-receipt"
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&live_ticket)?), "node-control-live-ticket");
    Ok(())
}

#[test]
fn cli_node_supervisor_policy_fixture_and_serve_receipt() -> CliResult<()> {
    let dir = temp_dir("cli-node-supervisor-policy")?;
    let state_root = dir.join("node-state");
    let policy = dir.join("supervisor-policy.preserves");
    let service_receipt = dir.join("service.preserves");
    let policy_ref = test_ref("supervisor-policy")?;

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-supervisor"])
            .output()?,
        "node supervisor init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node supervisor run",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "supervisor-policy-fixture", "--state-root"])
            .arg(&state_root)
            .args([
                "--max-restarts",
                "1",
                "--restart-window-ticks",
                "2",
                "--heartbeat-timeout-ticks",
                "2",
                "--shutdown-drain-ticks",
                "2",
                "--allow-stale-lock-recovery",
                "--policy",
            ])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&policy)
            .output()?,
        "node supervisor policy fixture",
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&policy)?), "node-control-supervisor-policy");
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--max-ticks", "1", "--supervisor-policy"])
        .arg(&policy)
        .args(["--receipt-out"])
        .arg(&service_receipt)
        .output()?;
    assert_success(&served, "node supervisor serve");
    let service_value = read_preserves(&service_receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&service_value), "node-control-service-run-receipt");
    let service_text = molten::preserves_rail::to_text(&service_value)?;
    assert!(service_text.contains("supervisor-policy"));
    assert!(service_text.contains("supervisor-receipts"));
    Ok(())
}

#[test]
fn cli_node_serve_drains_shutdown_request() -> CliResult<()> {
    let dir = temp_dir("cli-node-serve")?;
    let state_root = dir.join("node-state");
    let request = dir.join("shutdown.preserves");
    let queue_receipt = dir.join("queue.preserves");
    let service_receipt = dir.join("service.preserves");
    let authority_ref = test_ref("serve-authority")?;
    let policy_ref = test_ref("serve-policy")?;
    let resource_ref = test_ref("serve-resource")?;

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-serve"])
            .output()?,
        "node serve init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node serve run",
    );
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-request",
                "--operation",
                "shutdown",
                "--authority",
            ])
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&request)
            .output()?,
        "node serve shutdown request",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-submit", "--state-root"])
            .arg(&state_root)
            .arg(&request)
            .args(["--receipt-out"])
            .arg(&queue_receipt)
            .output()?,
        "node serve submit",
    );
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--max-ticks", "2", "--max-requests-per-tick", "1", "--receipt-out"])
        .arg(&service_receipt)
        .output()?;
    assert_success(&served, "node serve");
    assert!(stdout(&served).contains("node serve decision=pass"));
    assert!(stdout(&served).contains("stopped=yes"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&service_receipt)?),
        "node-control-service-run-receipt"
    );
    Ok(())
}

#[test]
fn cli_octet_artifacts_imports_raw_artifacts_to_ledger() -> CliResult<()> {
    let dir = temp_dir("cli-octet-artifacts-import")?;
    let artifacts = dir.join("artifacts");
    let ledger_root = dir.join("ledger");
    let receipt = dir.join("octet-artifact-ledger.preserves");
    std::fs::create_dir_all(&artifacts)?;
    write_octet_artifacts(&artifacts)?;

    let imported = molten_cmd()
        .args(["test", "octet", "artifacts", "import", "--artifacts"])
        .arg(&artifacts)
        .args(["--ledger"])
        .arg(&ledger_root)
        .args(["--receipt-out"])
        .arg(&receipt)
        .output()?;

    assert_success(&imported, "octet artifacts import");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&receipt)?), "octet-artifact-ledger-receipt");
    let entries = molten::ledger::list_artifacts(&ledger_root)?;
    let mut kinds = Vec::with_capacity(entries.len());
    for entry in entries {
        let value = molten::ledger::read_artifact(&ledger_root, &entry.artifact_ref)?;
        kinds.push(molten::ledger::artifact_kind(&value).to_string());
    }
    assert!(kinds.iter().any(|kind| kind == "octet-status-artifact"));
    assert!(kinds.iter().any(|kind| kind == "octet-object-corpus-artifact"));
    assert!(kinds.iter().any(|kind| kind == "octet-fingerprint-evidence"));
    Ok(())
}

#[test]
fn cli_octet_gate_writes_canonical_deny_receipt_for_warning_only() -> CliResult<()> {
    let dir = temp_dir("cli-octet-deny")?;
    let receipt = dir.join("octet-gate.preserves");
    write_octet_artifacts(&dir)?;

    let denied = molten_cmd()
        .args(["test", "octet", "gate", "--artifacts"])
        .arg(&dir)
        .args(["--profile", "strict-ci", "--receipt-out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&denied, "warning-only octet gate");
    assert!(stdout(&denied).contains("octet gate receipt blake3:"));
    assert!(stderr(&denied).contains("octet gate denied"));
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&receipt_value), "octet-gate-receipt");
    let receipt_text = molten::preserves_rail::to_text(&receipt_value)?;
    assert!(receipt_text.contains("<decision \"deny\">"));
    assert!(receipt_text.contains("warning-only"));
    Ok(())
}

#[test]
fn cli_retention_gc_plan_lists_gates_before_mutation() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-plan")?;
    let root = dir.join("retention-state");
    let plan_path = dir.join("plan.preserves");
    let apply_path = dir.join("apply.preserves");
    let refs = build_refs(&root)?;
    let plan = run_plan(&root, &refs, &plan_path)?;

    let show = molten_cmd().args(["test", "retention", "show"]).arg(&plan_path).output()?;
    assert_success(&show, "retention show gc-plan");
    assert!(stdout(&show).contains("retention gc plan"));

    let apply = run_apply(&root, &plan, &apply_path)?;
    assert_eq!(apply.plan_ref, plan.plan_ref);
    assert!(apply.retention_receipt_ref.is_some());
    assert!(apply.tombstone_ref.is_some());
    Ok(())
}

struct Refs {
    requester: String,
    object: String,
    peer: String,
    remote: String,
    policy: String,
    authority: String,
    support: String,
    index: String,
    remote_gc: String,
    clearance: String,
}

fn build_refs(root: &std::path::Path) -> CliResult<Refs> {
    let mut refs = Refs {
        requester: test_ref("retention-plan-requester")?,
        object: test_ref("retention-plan-object")?,
        peer: test_ref("retention-plan-peer")?,
        remote: test_ref("retention-plan-remote")?,
        policy: String::new(),
        authority: String::new(),
        support: String::new(),
        index: String::new(),
        remote_gc: String::new(),
        clearance: String::new(),
    };
    refs.policy = admission(root, &refs, molten::retention::ADMISSION_KIND_POLICY, "retention-plan-policy", &[])?;
    refs.authority =
        admission(root, &refs, molten::retention::ADMISSION_KIND_AUTHORITY, "retention-plan-authority", &[])?;
    refs.support =
        admission(root, &refs, molten::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "retention-plan-support", &[])?;
    refs.index =
        admission(root, &refs, molten::retention::ADMISSION_KIND_REFERENCE_INDEX, "retention-plan-index", &[])?;
    refs.remote_gc = admission(
        root,
        &refs,
        molten::retention::ADMISSION_KIND_REMOTE_GC,
        "retention-plan-remote-gc",
        std::slice::from_ref(&refs.remote),
    )?;
    refs.clearance = clearance(root, &refs)?;
    Ok(refs)
}

fn admission(
    root: &std::path::Path,
    refs: &Refs,
    kind: &str,
    label: &str,
    remote_refs: &[String],
) -> CliResult<String> {
    Ok(molten::retention::store_retention_evidence_admission(
        root,
        &molten::retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref: &refs.requester,
            object_ref: &refs.object,
            object_kind: "chunk",
            retention_class: molten::retention::CLASS_DURABLE_VALUE,
            action: molten::retention::ACTION_DELETE,
            bound_refs: &[test_ref(label)?],
            retained_refs: &[],
            remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        },
    )?
    .admission_ref)
}

fn clearance(root: &std::path::Path, refs: &Refs) -> CliResult<String> {
    Ok(molten::retention::store_retention_remote_gc_clearance(
        root,
        &molten::retention::RetentionRemoteGcClearanceInput {
            decision: "pass",
            requester_ref: &refs.requester,
            peer_ref: &refs.peer,
            object_ref: &refs.object,
            object_kind: "chunk",
            retention_class: molten::retention::CLASS_DURABLE_VALUE,
            action: molten::retention::ACTION_DELETE,
            remote_ref: &refs.remote,
            policy_ref: &refs.policy,
            authority_ref: &refs.authority,
            evidence_refs: std::slice::from_ref(&refs.support),
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        },
    )?
    .clearance_ref)
}

fn run_plan(
    root: &std::path::Path,
    refs: &Refs,
    out: &std::path::Path,
) -> CliResult<molten::retention::RetentionGcPlan> {
    let mut command = molten_cmd();
    command
        .args(["test", "retention", "gc-plan", "--root"])
        .arg(root)
        .args(["--subsystem", "ledger-gc", "--object-ref"])
        .arg(&refs.object)
        .args([
            "--object-kind",
            "chunk",
            "--retention-class",
            molten::retention::CLASS_DURABLE_VALUE,
            "--action",
            "delete",
        ]);
    add_refs(&mut command, refs);
    command.args(["--out"]).arg(out);
    let output = command.output()?;
    assert_success(&output, "retention gc-plan");
    assert!(stdout(&output).contains("retention gc plan ref="));
    let value = read_preserves(out)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "retention-gc-plan");
    let plan = molten::retention::parse_retention_gc_plan(&value)?;
    assert_eq!(plan.decision, "pass");
    assert!(plan.gates.iter().any(|gate| gate.name == "remote-clearance" && gate.decision == "pass"));
    Ok(plan)
}

fn add_refs(command: &mut std::process::Command, refs: &Refs) {
    command
        .args(["--retention-requester"])
        .arg(&refs.requester)
        .args(["--retention-policy-ref"])
        .arg(&refs.policy)
        .args(["--retention-authority-ref"])
        .arg(&refs.authority)
        .args(["--retention-evidence-ref"])
        .arg(&refs.support)
        .args(["--retention-remote-peer-ref"])
        .arg(&refs.peer)
        .args(["--retention-remote-ref"])
        .arg(&refs.remote)
        .args(["--retention-reference-index-ref"])
        .arg(&refs.index)
        .args(["--retention-remote-gc-ref"])
        .arg(&refs.remote_gc)
        .args(["--retention-remote-clearance-ref"])
        .arg(&refs.clearance)
        .args(["--retention-reference-index-complete"]);
}

fn run_apply(
    root: &std::path::Path,
    plan: &molten::retention::RetentionGcPlan,
    out: &std::path::Path,
) -> CliResult<molten::retention::RetentionGcApply> {
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(out)
        .output()?;
    assert_success(&output, "retention gc-apply-plan");
    assert!(stdout(&output).contains("retention gc apply ref="));
    let value = read_preserves(out)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "retention-gc-apply");
    let apply = molten::retention::parse_retention_gc_apply(&value)?;
    assert_eq!(apply.decision, "pass");
    Ok(apply)
}

#[test]
fn cli_retention_gc_negative_regression_matrix() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-negative")?;
    missing_plan_case(&dir)?;
    stale_plan_case(&dir)?;
    missing_apply_case(&dir)?;
    wrong_apply_case(&dir)?;
    audit_case(&dir)?;
    Ok(())
}

fn missing_plan_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("missing-plan-root");
    let missing_plan_ref = test_ref("retention-missing-plan")?;
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&missing_plan_ref)
        .args(["--receipt-out"])
        .arg(dir.join("missing-plan-apply.preserves"))
        .output()?;
    assert_failure(&output, "retention apply missing plan ref");
    Ok(())
}

fn stale_plan_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("stale-plan-root");
    let candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &root,
        label: "stale-plan",
        object_ref: test_ref("retention-stale-object")?,
        object_kind: "artifact",
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let plan = run_retention_gc_plan_cli(&candidate, "ledger-gc", &dir.join("stale-plan.preserves"))?;
    molten::retention::pin_object(&root, molten::retention::PinInput {
        object_ref: candidate.object_ref.clone(),
        object_kind: candidate.object_kind.clone(),
        retention_class: candidate.retention_class.clone(),
        source: molten::retention::SOURCE_OPERATOR_HOLD.to_string(),
        reason: "negative CLI stale plan".to_string(),
        owner_ref: candidate.requester_ref.clone(),
        expiry_ref: None,
        policy_refs: vec![candidate.policy_ref.clone()],
        evidence_refs: vec![candidate.support_ref.clone()],
        has_authority: true,
    })?;
    let apply_path = dir.join("stale-apply.preserves");
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&output, "retention apply stale plan ref");
    let receipt = molten::retention::parse_retention_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.retention_receipt_ref.is_none());
    assert!(receipt.tombstone_ref.is_none());
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-drift"));
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
    Ok(())
}

fn missing_apply_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("missing-apply-ledger");
    let artifact =
        molten::ledger::import_artifact(&root, &molten::preserves_rail::parse_text("<artifact \"missing-apply\">")?)?;
    let candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &root,
        label: "missing-apply",
        object_ref: artifact.artifact_ref.clone(),
        object_kind: &artifact.artifact_kind,
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let receipt = dir.join("missing-apply-ledger-gc.preserves");
    let mut command = molten_cmd();
    command.args(["test", "ledger", "gc", "--ledger"]).arg(&root).args(["--receipt-out"]).arg(&receipt);
    add_retention_args(&mut command, &candidate);
    let output = command.output()?;
    assert_success(&output, "ledger gc missing apply ref");
    assert!(stdout(&output).contains("decision=deny"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("retention-gc-execute-apply-missing"));
    molten::ledger::read_artifact(&root, &candidate.object_ref)?;
    Ok(())
}

fn wrong_apply_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("wrong-apply-ledger");
    let artifact =
        molten::ledger::import_artifact(&root, &molten::preserves_rail::parse_text("<artifact \"wrong-apply\">")?)?;
    let candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &root,
        label: "wrong-apply",
        object_ref: artifact.artifact_ref.clone(),
        object_kind: &artifact.artifact_kind,
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let plan = run_retention_gc_plan_cli(&candidate, "chunk-gc", &dir.join("wrong-plan.preserves"))?;
    let apply_path = dir.join("wrong-apply.preserves");
    let apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&apply_output, "retention apply wrong subsystem plan");
    let apply = molten::retention::parse_retention_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(apply.decision, "pass");
    let receipt = dir.join("wrong-apply-ledger-gc.preserves");
    let mut command = molten_cmd();
    command
        .args(["test", "ledger", "gc", "--ledger"])
        .arg(&root)
        .args(["--apply-ref"])
        .arg(&apply.apply_ref)
        .args(["--receipt-out"])
        .arg(&receipt);
    add_retention_args(&mut command, &candidate);
    let output = command.output()?;
    assert_success(&output, "ledger gc wrong apply ref");
    assert!(stdout(&output).contains("decision=deny"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("retention-gc-execute-apply-scope-mismatch"));
    molten::ledger::read_artifact(&root, &candidate.object_ref)?;
    Ok(())
}

fn audit_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("audit-root");
    let missing = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&root)
        .args(["--execution-ref"])
        .arg(test_ref("missing-execution")?)
        .args(["--out"])
        .arg(dir.join("missing-execution-audit.preserves"))
        .output()?;
    assert_failure(&missing, "retention audit missing execution ref");
    let execution =
        molten::retention::store_retention_gc_execution_gate(molten::retention::RetentionGcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: molten::retention::ACTION_DELETE,
            object_ref: &test_ref("denied-execution-object")?,
            object_kind: "artifact",
            retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
            apply_ref: None,
        })?;
    let audit_path = dir.join("denied-execution-audit.preserves");
    let output = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&root)
        .args(["--execution-ref"])
        .arg(&execution.execution_ref)
        .args(["--out"])
        .arg(&audit_path)
        .output()?;
    assert_success(&output, "retention audit denied execution ref");
    let audit = molten::retention::parse_retention_gc_audit(&read_preserves(&audit_path)?)?;
    assert_eq!(audit.decision, "deny");
    assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-apply-missing"));
    assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-plan-missing"));
    Ok(())
}

#[test]
fn cli_catalog_discovers_retention_gc_audit_chains() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-catalog")?;
    let registry = dir.join("registry");
    let ledger_root = dir.join("ledger");
    let retention_root = dir.join("retention-root");
    let candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &retention_root,
        label: "catalog-audit",
        object_ref: test_ref("retention-catalog-audit-object")?,
        object_kind: "artifact",
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let fixture = setup_retention_gc_catalog_fixture(&candidate, "ledger-gc", &dir)?;

    let (explain_path, explain_ref) = run_explain(&retention_root, &dir, &fixture)?;
    let (bundle_dir, bundle_ref) = run_bundle(&retention_root, &dir, &explain_path, &explain_ref)?;
    check_profile(&registry, &ledger_root, &bundle_dir)?;
    run_verify(&registry, &ledger_root, &dir, &bundle_dir, &bundle_ref)?;
    run_tamper(&dir, &bundle_dir, &fixture)?;
    run_search(&dir, &registry, &ledger_root, &fixture)?;
    run_mcp(&dir, &registry, &ledger_root, &fixture)?;
    Ok(())
}

fn run_explain(
    retention_root: &std::path::Path,
    dir: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<(std::path::PathBuf, String)> {
    let explain_path = dir.join("retention-explain.preserves");
    let explain_output = molten_cmd()
        .args(["test", "retention", "explain", "--root"])
        .arg(retention_root)
        .args(["--object-ref"])
        .arg(&fixture.object_ref)
        .args([
            "--object-kind",
            "artifact",
            "--retention-class",
            molten::retention::CLASS_PUBLIC_ARTIFACT,
            "--action",
            molten::retention::ACTION_DELETE,
            "--subsystem",
            "ledger-gc",
            "--out",
        ])
        .arg(&explain_path)
        .output()?;
    assert_success(&explain_output, "retention explain candidate");
    assert!(stdout(&explain_output).contains("retention explain ref="));
    let explain = molten::retention::parse_retention_candidate_explain(&read_preserves(&explain_path)?)?;
    assert_eq!(explain.object_ref, fixture.object_ref);
    assert_eq!(explain.admission_refs.len(), 4);
    assert_eq!(explain.gc_plan_refs, vec![fixture.plan_ref.clone()]);
    assert_eq!(explain.gc_apply_refs, vec![fixture.apply_ref.clone()]);
    assert_eq!(explain.gc_execution_refs, vec![fixture.execution_ref.clone()]);
    assert_eq!(explain.gc_audit_refs, vec![fixture.audit_ref.clone()]);
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&explain_path)?), "retention-candidate-explain");
    Ok((explain_path, explain.explain_ref))
}

fn run_bundle(
    retention_root: &std::path::Path,
    dir: &std::path::Path,
    explain_path: &std::path::Path,
    explain_ref: &str,
) -> CliResult<(std::path::PathBuf, String)> {
    let bundle_dir = dir.join("retention-bundle");
    let bundle_output = molten_cmd()
        .args(["test", "retention", "bundle-export", "--root"])
        .arg(retention_root)
        .args(["--explain"])
        .arg(explain_path)
        .args(["--out"])
        .arg(&bundle_dir)
        .args(["--profile", "public"])
        .output()?;
    assert_success(&bundle_output, "retention bundle export");
    assert!(stderr(&bundle_output).contains("retention bundle ref="));
    let bundle_value = read_preserves(&bundle_dir.join("bundle.preserves"))?;
    let bundle = molten::retention::parse_retention_candidate_bundle(&bundle_value)?;
    assert_eq!(molten::ledger::artifact_kind(&bundle_value), "retention-candidate-bundle");
    assert_eq!(bundle.explain_ref, explain_ref);
    assert_eq!(bundle.artifact_refs.len(), 6);
    assert!(bundle.diagnostics.is_empty());
    assert!(bundle_dir.join("explain.preserves").exists());
    assert!(bundle_dir.join("artifacts/gc-plans").exists());
    assert!(bundle_dir.join("artifacts/gc-audits").exists());
    Ok((bundle_dir, bundle.bundle_ref))
}

fn check_profile(
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    bundle_dir: &std::path::Path,
) -> CliResult<()> {
    let bundle_profile = molten::retention::parse_retention_candidate_bundle_profile(&read_preserves(
        &bundle_dir.join("bundle-profile.preserves"),
    )?)?;
    assert_eq!(bundle_profile.profile, "public");
    assert_eq!(bundle_profile.decision, "pass");
    assert!(bundle_profile.marker_refs.is_empty());
    let bundle_profile_path = bundle_dir.join("bundle-profile.preserves");
    let profile_import = molten_cmd()
        .args(["test", "ledger", "import"])
        .arg(&bundle_profile_path)
        .args(["--ledger"])
        .arg(ledger_root)
        .output()?;
    assert_success(&profile_import, "ledger import retention bundle profile");
    let profile_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args([
            "--ledger-kind",
            "retention-candidate-bundle-profile",
            "--text",
            "retention-candidate:bundle-profile",
        ])
        .output()?;
    assert_success(&profile_search, "catalog search retention bundle profile");
    assert!(stdout(&profile_search).contains("retention-candidate:bundle-profile"));
    Ok(())
}

fn run_verify(
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    dir: &std::path::Path,
    bundle_dir: &std::path::Path,
    bundle_ref: &str,
) -> CliResult<()> {
    let verify_path = dir.join("retention-bundle-verify.preserves");
    let verify_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(bundle_dir)
        .args(["--receipt-out"])
        .arg(&verify_path)
        .output()?;
    assert_success(&verify_output, "retention bundle verify");
    assert!(stderr(&verify_output).contains("retention bundle verify ref="));
    let verify_value = read_preserves(&verify_path)?;
    let verify = molten::retention::parse_retention_candidate_bundle_verify(&verify_value)?;
    assert_eq!(molten::ledger::artifact_kind(&verify_value), "retention-candidate-bundle-verify");
    assert_eq!(verify.decision, "pass");
    assert_eq!(verify.bundle_ref, bundle_ref);
    assert_eq!(verify.file_refs.len(), 6);
    assert!(verify.diagnostics.is_empty());
    let verify_import = molten_cmd()
        .args(["test", "ledger", "import"])
        .arg(&verify_path)
        .args(["--ledger"])
        .arg(ledger_root)
        .output()?;
    assert_success(&verify_import, "ledger import retention bundle verify");
    let verify_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args([
            "--ledger-kind",
            "retention-candidate-bundle-verify",
            "--text",
            "retention-candidate:bundle-verify",
        ])
        .output()?;
    assert_success(&verify_search, "catalog search retention bundle verify");
    let verify_search_stdout = stdout(&verify_search);
    assert!(verify_search_stdout.contains("retention-candidate:bundle-verify"));
    assert!(verify_search_stdout.contains(&verify.verify_ref));
    Ok(())
}

fn run_tamper(
    dir: &std::path::Path,
    bundle_dir: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let tampered_plan_path = bundle_dir
        .join("artifacts/gc-plans")
        .join(format!("{}.preserves", fixture.plan_ref.replace(':', "_")));
    std::fs::write(
        &tampered_plan_path,
        molten::preserves_rail::to_text(&molten::preserves_rail::record("tampered", vec![
            molten::preserves_rail::string("plan"),
        ]))?,
    )?;
    let tampered_path = dir.join("retention-bundle-verify-tampered.preserves");
    let tampered_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(bundle_dir)
        .args(["--receipt-out"])
        .arg(&tampered_path)
        .output()?;
    assert_success(&tampered_output, "retention bundle verify tampered");
    let tampered = molten::retention::parse_retention_candidate_bundle_verify(&read_preserves(&tampered_path)?)?;
    assert_eq!(tampered.decision, "deny");
    assert!(
        tampered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
    );
    Ok(())
}

fn run_search(
    dir: &std::path::Path,
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let search_receipt = dir.join("catalog-search-receipt.preserves");
    let search_output = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--text"])
        .arg(format!("retention-gc-object:{}", fixture.object_ref))
        .args(["--receipt-out"])
        .arg(&search_receipt)
        .output()?;
    assert_success(&search_output, "catalog search retention GC object");
    let search_stdout = stdout(&search_output);
    assert!(search_stdout.contains("retention-gc:plan"));
    assert!(search_stdout.contains("retention-gc:apply"));
    assert!(search_stdout.contains("retention-gc:execute"));
    assert!(search_stdout.contains("retention-gc:audit"));
    assert!(search_stdout.contains(&fixture.plan_ref));
    assert!(search_stdout.contains(&fixture.apply_ref));
    assert!(search_stdout.contains(&fixture.execution_ref));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&search_receipt)?), "catalog-receipt");

    let audit_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--ledger-kind", "retention-gc-audit", "--text", "retention-gc:audit"])
        .output()?;
    assert_success(&audit_search, "catalog search retention GC audit ledger kind");
    let audit_search_stdout = stdout(&audit_search);
    assert!(audit_search_stdout.contains("retention-gc:audit"));
    assert!(audit_search_stdout.contains(&fixture.audit_ref));
    Ok(())
}

fn run_mcp(
    dir: &std::path::Path,
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let mcp_request_path = dir.join("retention-gc-search-request.preserves");
    let mcp_response_path = dir.join("retention-gc-search-response.preserves");
    let mcp_receipt_path = dir.join("retention-gc-search-mcp-receipt.preserves");
    let mcp_request = molten::catalog_mcp::mcp_request_value("search_retention_gc", vec![
        molten::preserves_rail::record("stage", vec![molten::preserves_rail::string("audit")]),
        molten::preserves_rail::record("object-ref", vec![molten::preserves_rail::string(&fixture.object_ref)]),
        molten::preserves_rail::record("subsystem", vec![molten::preserves_rail::string("ledger-gc")]),
        molten::preserves_rail::record("execution-ref", vec![molten::preserves_rail::string(&fixture.execution_ref)]),
    ])?;
    std::fs::write(&mcp_request_path, molten::preserves_rail::to_text(&mcp_request)?)?;
    let mcp_output = molten_cmd()
        .args(["test", "catalog", "mcp-call"])
        .arg(&mcp_request_path)
        .args(["--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--out"])
        .arg(&mcp_response_path)
        .args(["--receipt-out"])
        .arg(&mcp_receipt_path)
        .output()?;
    assert_success(&mcp_output, "catalog MCP search_retention_gc");
    let mcp_response = std::fs::read_to_string(&mcp_response_path)?;
    assert!(mcp_response.contains("retention-gc:audit"));
    assert!(mcp_response.contains(&fixture.execution_ref));
    let mcp_receipt = molten::catalog_mcp::parse_mcp_receipt(&read_preserves(&mcp_receipt_path)?)?;
    assert_eq!(mcp_receipt.tool, "search_retention_gc");
    assert_eq!(mcp_receipt.decision, "pass");
    Ok(())
}

struct RetentionCandidateInput<'a> {
    root: &'a std::path::Path,
    label: &'a str,
    object_ref: String,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct RetentionCliCandidate {
    root: std::path::PathBuf,
    object_ref: String,
    object_kind: String,
    retention_class: String,
    action: String,
    requester_ref: String,
    policy_ref: String,
    authority_ref: String,
    support_ref: String,
    index_ref: String,
}

struct RetentionAdmissionInput<'a> {
    candidate: &'a RetentionCliCandidate,
    kind: &'a str,
    label: &'a str,
}

struct RetentionGcCatalogFixture {
    object_ref: String,
    plan_ref: String,
    apply_ref: String,
    execution_ref: String,
    audit_ref: String,
}

fn setup_retention_gc_catalog_fixture(
    candidate: &RetentionCliCandidate,
    subsystem: &str,
    dir: &std::path::Path,
) -> CliResult<RetentionGcCatalogFixture> {
    let plan_path = dir.join("catalog-retention-plan.preserves");
    let plan = run_retention_gc_plan_cli(candidate, subsystem, &plan_path)?;
    let apply_path = dir.join("catalog-retention-apply.preserves");
    let apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&candidate.root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&apply_output, "retention gc-apply-plan catalog fixture");
    let apply = molten::retention::parse_retention_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(apply.decision, "pass");
    let execution =
        molten::retention::store_retention_gc_execution_gate(molten::retention::RetentionGcExecutionGateInput {
            root: &candidate.root,
            subsystem,
            action: &candidate.action,
            object_ref: &candidate.object_ref,
            object_kind: &candidate.object_kind,
            retention_class: &candidate.retention_class,
            apply_ref: Some(&apply.apply_ref),
        })?;
    assert_eq!(execution.decision, "pass");
    let execution_path = dir.join("catalog-retention-execution.preserves");
    std::fs::write(&execution_path, molten::preserves_rail::to_text(&execution.value)?)?;
    let audit = molten::retention::audit_retention_gc_execution(molten::retention::RetentionGcAuditInput {
        root: &candidate.root,
        execution_ref: &execution.execution_ref,
    })?;
    assert_eq!(audit.decision, "pass");
    let audit_path = dir.join("catalog-retention-audit.preserves");
    std::fs::write(&audit_path, molten::preserves_rail::to_text(&audit.value)?)?;
    let ledger_root = dir.join("ledger");
    for artifact in [&plan_path, &apply_path, &execution_path, &audit_path] {
        let output = molten_cmd()
            .args(["test", "ledger", "import"])
            .arg(artifact)
            .args(["--ledger"])
            .arg(&ledger_root)
            .output()?;
        assert_success(&output, "ledger import retention GC catalog fixture");
    }
    Ok(RetentionGcCatalogFixture {
        object_ref: candidate.object_ref.clone(),
        plan_ref: plan.plan_ref,
        apply_ref: apply.apply_ref,
        execution_ref: execution.execution_ref,
        audit_ref: audit.audit_ref,
    })
}

fn setup_retention_cli_candidate(input: RetentionCandidateInput<'_>) -> CliResult<RetentionCliCandidate> {
    let requester_ref = test_ref(&format!("{}-requester", input.label))?;
    let mut candidate = RetentionCliCandidate {
        root: input.root.to_path_buf(),
        object_ref: input.object_ref,
        object_kind: input.object_kind.to_string(),
        retention_class: input.retention_class.to_string(),
        action: input.action.to_string(),
        requester_ref,
        policy_ref: String::new(),
        authority_ref: String::new(),
        support_ref: String::new(),
        index_ref: String::new(),
    };
    candidate.policy_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_POLICY,
        label: "policy",
    })?;
    candidate.authority_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_AUTHORITY,
        label: "authority",
    })?;
    candidate.support_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
        label: "support",
    })?;
    candidate.index_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_REFERENCE_INDEX,
        label: "index",
    })?;
    Ok(candidate)
}

fn store_retention_cli_admission(input: RetentionAdmissionInput<'_>) -> CliResult<String> {
    Ok(molten::retention::store_retention_evidence_admission(
        &input.candidate.root,
        &molten::retention::RetentionEvidenceAdmissionInput {
            kind: input.kind,
            decision: "pass",
            requester_ref: &input.candidate.requester_ref,
            object_ref: &input.candidate.object_ref,
            object_kind: &input.candidate.object_kind,
            retention_class: &input.candidate.retention_class,
            action: &input.candidate.action,
            bound_refs: &[test_ref(&format!("{}-{}", input.candidate.object_ref, input.label))?],
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        },
    )?
    .admission_ref)
}

fn run_retention_gc_plan_cli(
    candidate: &RetentionCliCandidate,
    subsystem: &str,
    out: &std::path::Path,
) -> CliResult<molten::retention::RetentionGcPlan> {
    let mut command = molten_cmd();
    command
        .args(["test", "retention", "gc-plan", "--root"])
        .arg(&candidate.root)
        .args(["--subsystem", subsystem, "--object-ref"])
        .arg(&candidate.object_ref)
        .args(["--object-kind"])
        .arg(&candidate.object_kind)
        .args(["--retention-class"])
        .arg(&candidate.retention_class)
        .args(["--action"])
        .arg(&candidate.action)
        .args(["--out"])
        .arg(out);
    add_retention_args(&mut command, candidate);
    let output = command.output()?;
    assert_success(&output, "retention gc-plan regression fixture");
    let plan = molten::retention::parse_retention_gc_plan(&read_preserves(out)?)?;
    assert_eq!(plan.decision, "pass");
    Ok(plan)
}

fn add_retention_args(command: &mut std::process::Command, candidate: &RetentionCliCandidate) {
    command
        .args(["--retention-requester"])
        .arg(&candidate.requester_ref)
        .args(["--retention-policy-ref"])
        .arg(&candidate.policy_ref)
        .args(["--retention-authority-ref"])
        .arg(&candidate.authority_ref)
        .args(["--retention-evidence-ref"])
        .arg(&candidate.support_ref)
        .args(["--retention-reference-index-ref"])
        .arg(&candidate.index_ref)
        .args(["--retention-reference-index-complete"]);
}

fn write_octet_artifacts(dir: &std::path::Path) -> CliResult<()> {
    write_octet_artifacts_with(dir, OCTET_WARNING_STATUS, OCTET_WARNING_SUMMARY)
}

fn write_octet_artifacts_with(dir: &std::path::Path, status: impl AsRef<str>, summary: &str) -> CliResult<()> {
    std::fs::write(dir.join("command.txt"), "cargo octet check --artifact-dir target/octet\n")?;
    std::fs::write(dir.join("status.json"), status.as_ref())?;
    std::fs::write(dir.join("summary.txt"), summary)?;
    std::fs::write(dir.join("object-corpus-receipt.json"), OCTET_OBJECT_CORPUS)?;
    Ok(())
}

const OCTET_WARNING_STATUS: &str = r#"{
  "status": "warning-only",
  "exit_code": 0,
  "output_format": "human",
  "metadata": {
    "tool_name": "cargo-octet",
    "tool_version": "0.1.0",
    "rustc_version": "rustc 1.96.0-nightly",
    "toolchain": "nightly-2026-03-21-x86_64-unknown-linux-gnu",
    "profile_name": "workspace-metadata",
    "profile_hash": "b3:profile",
    "config_hash": "b3:config"
  },
  "total_findings": 1,
  "warning_findings": 1,
  "error_findings": 0,
  "autofixable_findings": 0,
  "cargo_process_exit": {"classification": "success", "code": 0}
}"#;

const OCTET_WARNING_SUMMARY: &str = "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  no_unwrap 1\n\nIndex:\n";

fn octet_noncritical_status(total: u64) -> String {
    let (config_hash, profile_hash) = current_octet_hashes();
    format!(
        r#"{{
  "status": "warning-only",
  "exit_code": 0,
  "output_format": "human",
  "metadata": {{
    "tool_name": "cargo-octet",
    "tool_version": "0.1.0",
    "rustc_version": "rustc 1.96.0-nightly",
    "toolchain": "nightly-2026-03-21-x86_64-unknown-linux-gnu",
    "profile_name": "workspace-metadata",
    "profile_hash": "{profile_hash}",
    "config_hash": "{config_hash}"
  }},
  "total_findings": {total},
  "warning_findings": {total},
  "error_findings": 0,
  "autofixable_findings": 0,
  "cargo_process_exit": {{"classification": "success", "code": 0}}
}}"#
    )
}

fn current_octet_hashes() -> (String, String) {
    let cargo_toml = manifest_dir().join("Cargo.toml");
    let cargo_hash = file_hash(&cargo_toml);
    let dylint_hash = file_hash(&manifest_dir().join("dylint.toml"));
    let files = vec![
        serde_json::json!({"path": "Cargo.toml", "hash": cargo_hash}),
        serde_json::json!({"path": "dylint.toml", "hash": dylint_hash}),
    ];
    let config_payload = serde_json::json!({
        "files": files,
        "effective_scope_args": ["-p", "molten"],
        "effective_cargo_check_args": ["--all-targets"],
    });
    let config_hash = b3_full_hash(&config_payload.to_string());
    let profile_payload = serde_json::json!({
        "scope_args": ["-p", "molten"],
        "cargo_check_args": ["--all-targets"],
        "output_format": "human",
        "config_hash": config_hash,
    });
    let profile_hash = b3_full_hash(&profile_payload.to_string());
    (config_hash, profile_hash)
}

fn file_hash(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| format!("b3:{}", blake3::hash(&bytes).to_hex()))
}

fn b3_full_hash(input: &str) -> String {
    format!("b3:{}", blake3::hash(input.as_bytes()).to_hex())
}

fn test_ref(label: &str) -> CliResult<String> {
    Ok(molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("cli-test-ref", vec![
        molten::preserves_rail::string(label),
    ]))?)
}

const OCTET_NONCRITICAL_SUMMARY_ONE: &str = "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n";

const OCTET_NONCRITICAL_SUMMARY_TWO: &str = "--- octet summary ---\nStatus: warning-only\nFindings: 2\nWarnings: 2\nErrors: 0\n\nBy lint:\n  function_length 1\n  bool_naming 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n  F2 bool_naming molten src/example.rs:20\n";

const OCTET_OBJECT_CORPUS: &str = r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":3,"source_paths":["src/job/dag.rs","src/main.rs","src/node/runtime.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":3}"#;

fn molten_cmd() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_molten"));
    command.current_dir(manifest_dir());
    command
}

struct StartArgs<'a> {
    root: &'a std::path::Path,
    config: &'a std::path::Path,
    startup: &'a std::path::Path,
}

fn start_case(args: StartArgs<'_>) -> CliResult<()> {
    let init = molten_cmd()
        .args(["node", "init", "--state-root"])
        .arg(args.root)
        .args(["--node-id", "node:cli", "--config-out"])
        .arg(args.config)
        .output()?;
    assert_success(&init, "node init");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.config)?), "node-config");

    let run = molten_cmd()
        .args(["node", "run", "--state-root"])
        .arg(args.root)
        .args(["--startup-out"])
        .arg(args.startup)
        .output()?;
    assert_success(&run, "node run");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.startup)?), "node-startup-receipt");
    Ok(())
}

struct OpArgs<'a> {
    name: &'a str,
    out: &'a std::path::Path,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
    label: &'a str,
}

fn write_op(args: OpArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-request", "--operation"])
        .arg(args.name)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref)
        .args(["--out"])
        .arg(args.out)
        .output()?;
    assert_success(&output, args.label);
    Ok(())
}

fn submit_op(
    root: &std::path::Path,
    request: &std::path::Path,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-submit", "--state-root"])
        .arg(root)
        .arg(request)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    Ok(())
}

fn dispatch_op(root: &std::path::Path, receipt: &std::path::Path, label: &str) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-dispatch", "--state-root"])
        .arg(root)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-receipt");
    Ok(())
}

fn expect_running(root: &std::path::Path, health: &std::path::Path, receipt: &std::path::Path) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "status", "--state-root"])
        .arg(root)
        .args(["--health-out"])
        .arg(health)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node status");
    assert!(stdout(&output).contains("node status running"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(health)?), "node-health-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-receipt");
    Ok(())
}

fn expect_stop_loop(root: &std::path::Path, shutdown: &std::path::Path, receipt: &std::path::Path) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "run-loop", "--state-root"])
        .arg(root)
        .args(["--max-requests", "4", "--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node socket shutdown loop");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(shutdown)?), "node-shutdown-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-loop-receipt");
    Ok(())
}

fn start_state(root: &std::path::Path, node_id: &str, init_label: &str, run_label: &str) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(root)
            .args(["--node-id", node_id])
            .output()?,
        init_label,
    );
    assert_success(&molten_cmd().args(["test", "node", "run", "--state-root"]).arg(root).output()?, run_label);
    Ok(())
}

fn write_status_request(
    path: &std::path::Path,
    authority_ref: &str,
    policy_ref: &str,
    resource_ref: &str,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-request",
                "--operation",
                "status",
                "--authority",
            ])
            .arg(authority_ref)
            .args(["--policy"])
            .arg(policy_ref)
            .args(["--resource"])
            .arg(resource_ref)
            .args(["--out"])
            .arg(path)
            .output()?,
        label,
    );
    Ok(())
}

struct GrantArgs<'a> {
    root: &'a std::path::Path,
    grant: &'a std::path::Path,
    peer: &'a str,
    node: &'a str,
    policy_ref: &'a str,
    label: &'a str,
}

fn grant_fixture(args: GrantArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "authority-grant-fixture", "--state-root"])
            .arg(args.root)
            .args([
                "--peer",
                args.peer,
                "--node",
                args.node,
                "--operation",
                "status",
                "--policy",
            ])
            .arg(args.policy_ref)
            .args(["--out"])
            .arg(args.grant)
            .output()?,
        args.label,
    );
    Ok(molten::preserves_rail::canonical_hash(&read_preserves(args.grant)?)?)
}

fn ticket_export(root: &std::path::Path, ticket: &std::path::Path, policy_ref: &str, label: &str) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-ticket-export", "--state-root"])
            .arg(root)
            .args(["--policy"])
            .arg(policy_ref)
            .args(["--out"])
            .arg(ticket)
            .output()?,
        label,
    );
    Ok(())
}

struct AdmitArgs<'a> {
    root: &'a std::path::Path,
    receipt: &'a std::path::Path,
    peer: &'a str,
    policy_ref: &'a str,
    ticket: &'a std::path::Path,
    label: &'a str,
}

fn peer_admit(args: AdmitArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-peer-admit", "--state-root"])
            .arg(args.root)
            .args(["--peer", args.peer, "--policy"])
            .arg(args.policy_ref)
            .args(["--receipt-out"])
            .arg(args.receipt)
            .arg(args.ticket)
            .output()?,
        args.label,
    );
    Ok(molten::preserves_rail::canonical_hash(&read_preserves(args.receipt)?)?)
}

struct SendArgs<'a> {
    root: &'a std::path::Path,
    request: &'a std::path::Path,
    ticket: &'a std::path::Path,
    peer: &'a str,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

fn send_cmd(args: &SendArgs<'_>) -> std::process::Command {
    let mut command = molten_cmd();
    command
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(args.root)
        .arg(args.request)
        .arg(args.ticket)
        .args(["--from-peer", args.peer, "--peer-bootstrap"])
        .arg(args.bootstrap_ref)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref);
    command
}

fn expect_no_address(
    args: &SendArgs<'_>,
    transport_receipt: &std::path::Path,
    receipt: &std::path::Path,
) -> CliResult<()> {
    let sent = send_cmd(args)
        .args(["--transport-receipt-out"])
        .arg(transport_receipt)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&sent, "node live send deny no address");
    assert!(stdout(&sent).contains("transport_receipt=none"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-live-send-receipt");
    assert!(!transport_receipt.exists());
    let text = molten::preserves_rail::to_text(&read_preserves(receipt)?)?;
    assert!(text.contains("ticket has no endpoint addresses"));
    Ok(())
}

fn expect_mismatch(args: &SendArgs<'_>, operation_ref: &str, receipt: &std::path::Path) -> CliResult<()> {
    let output = send_cmd(args)
        .args(["--operation-id"])
        .arg(operation_ref)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node live send deny operation mismatch");
    let text = molten::preserves_rail::to_text(&read_preserves(receipt)?)?;
    assert!(text.contains("operation-id"));
    Ok(())
}

struct BundleArgs<'a> {
    root: &'a std::path::Path,
    ticket: &'a std::path::Path,
    peer_admission: &'a std::path::Path,
    authority_grant: &'a std::path::Path,
    send_receipt: &'a std::path::Path,
    service_receipt: &'a std::path::Path,
    receipt: &'a std::path::Path,
}

fn expect_missing_receive(args: BundleArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle", "--state-root"])
        .arg(args.root)
        .args(["--ticket"])
        .arg(args.ticket)
        .args(["--peer-admission"])
        .arg(args.peer_admission)
        .args(["--authority-grant"])
        .arg(args.authority_grant)
        .args(["--send-receipt"])
        .arg(args.send_receipt)
        .args(["--service-receipt"])
        .arg(args.service_receipt)
        .args(["--receipt-out"])
        .arg(args.receipt)
        .output()?;
    assert_success(&output, "node live workflow bundle deny");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.receipt)?), "node-control-live-workflow-receipt");
    let text = molten::preserves_rail::to_text(&read_preserves(args.receipt)?)?;
    assert!(text.contains("missing receive receipt"));
    Ok(())
}

struct LoopbackArgs<'a> {
    root: &'a std::path::Path,
    request: &'a std::path::Path,
    publish: &'a std::path::Path,
    receive: &'a std::path::Path,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

fn run_loopback(args: LoopbackArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "control-ingress-live-loopback", "--state-root"])
        .arg(args.root)
        .args([
            "--from-peer",
            "peer:cli-live",
            "--to-node",
            "node:cli-live",
            "--peer-bootstrap",
        ])
        .arg(args.bootstrap_ref)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref)
        .args(["--publish-receipt-out"])
        .arg(args.publish)
        .args(["--receive-receipt-out"])
        .arg(args.receive)
        .arg(args.request)
        .output()?;
    assert_success(&output, "node live ingress loopback");
    assert!(stdout(&output).contains("enqueued=yes"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.publish)?), "node-control-live-transport-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.receive)?), "node-control-live-transport-receipt");
    Ok(())
}

struct EnvelopeArgs<'a> {
    path: &'a std::path::Path,
    request: &'a std::path::Path,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
    label: &'a str,
}

fn write_envelope(args: EnvelopeArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-ingress-build",
                "--from-peer",
                "peer:cli",
                "--to-node",
                "node:cli-ingress",
                "--peer-bootstrap",
            ])
            .arg(args.bootstrap_ref)
            .args(["--authority"])
            .arg(args.authority_ref)
            .args(["--policy"])
            .arg(args.policy_ref)
            .args(["--resource"])
            .arg(args.resource_ref)
            .args(["--out"])
            .arg(args.path)
            .arg(args.request)
            .output()?,
        args.label,
    );
    let value = read_preserves(args.path)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "node-control-ingress-envelope");
    Ok(molten::preserves_rail::canonical_hash(&value)?)
}

fn publish_envelope(
    root: &std::path::Path,
    envelope: &std::path::Path,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-publish", "--state-root"])
            .arg(root)
            .arg(envelope)
            .args(["--receipt-out"])
            .arg(receipt)
            .output()?,
        label,
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-ingress-receipt");
    Ok(())
}

fn deliver_envelope(
    root: &std::path::Path,
    envelope_ref: &str,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-deliver", "--state-root"])
            .arg(root)
            .arg(envelope_ref)
            .args(["--receipt-out"])
            .arg(receipt)
            .output()?,
        label,
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-ingress-receipt");
    Ok(())
}

fn run_once(root: &std::path::Path, receipt: &std::path::Path, label: &str) -> CliResult<std::process::Output> {
    let output = molten_cmd()
        .args(["test", "node", "run-loop", "--state-root"])
        .arg(root)
        .args(["--max-requests", "1", "--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    Ok(output)
}

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cleanup_stale_molten_temp_dirs() {
    static CLEAN_STALE_TEMP_DIRS: std::sync::Once = std::sync::Once::new();
    CLEAN_STALE_TEMP_DIRS.call_once(|| {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry_result in entries {
            let Ok(entry) = entry_result else {
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    continue;
                };
                if is_stale_molten_temp_dir(name) {
                    let remove_result = std::fs::remove_dir_all(entry.path());
                    if remove_result.is_err() {
                        continue;
                    }
                }
            }
        }
    });
}

fn is_stale_molten_temp_dir(name: &str) -> bool {
    name.starts_with("molten-") && live_process_token_count(name) == 0
}

fn live_process_token_count(name: &str) -> usize {
    let current_pid = u64::from(std::process::id());
    name.split('-')
        .filter_map(|token| token.parse::<u64>().ok())
        .filter(|pid| *pid == current_pid || std::path::Path::new("/proc").join(pid.to_string()).exists())
        .count()
}

fn write_release_export_test_archive(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: Option<&std::path::Path>,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    if let Some(manifest_path) = manifest_path {
        append_release_export_test_bytes(
            &mut builder,
            "release-export-manifest.preserves",
            &std::fs::read(manifest_path)?,
        )?;
    }
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

struct ExtraArchiveMember<'a> {
    name: &'a str,
    bytes: &'a [u8],
}

fn write_release_export_test_archive_with_extra(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
    extra: ExtraArchiveMember<'_>,
) -> CliResult<()> {
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    append_release_export_test_bytes(&mut builder, extra.name, extra.bytes)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn write_release_export_test_archive_with_tamper(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    let first = member_refs.first().ok_or_else(|| test_error("release export test needs a member"))?;
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        if name == &first.0 {
            append_release_export_test_bytes(&mut builder, name, b"tampered release evidence")?;
        } else {
            append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
        }
    }
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn write_release_export_test_archive_with_duplicate(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    let first = member_refs.first().ok_or_else(|| test_error("release export test needs a member"))?;
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    append_release_export_test_bytes(&mut builder, &first.0, &std::fs::read(output_dir.join(&first.0))?)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_release_export_test_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> CliResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder.append_data(&mut header, name, std::io::Cursor::new(bytes))?;
    Ok(())
}

fn temp_dir(label: &str) -> CliResult<std::path::PathBuf> {
    cleanup_stale_molten_temp_dirs();
    let nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn read_preserves(path: &std::path::Path) -> CliResult<preserves::IOValue> {
    Ok(molten::preserves_rail::parse_text(&std::fs::read_to_string(path)?)?)
}

fn assert_success(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &std::process::Output, label: &str) {
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}
