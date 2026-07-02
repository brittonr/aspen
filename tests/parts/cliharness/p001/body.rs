
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
