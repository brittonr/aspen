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
    let receipt = molten::harness::parse_receipt(&receipt_value)?;
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.artifact_kind, "report");
    assert!(molten::harness::receipt_summary(&receipt_value)?.contains("decision=pass"));

    assert_report_repro_flow(&dir, &report, &report_value, &receipt.report_ref)?;
    Ok(())
}

#[test]
fn cli_replay_fixture_proves_deterministic_pass_and_tamper_denial() -> CliResult<()> {
    let dir = temp_dir("cli-replay-fixture")?;
    let fixture = dir.join("fixture.preserves");
    let pass_receipt = dir.join("pass.receipt.preserves");
    let tampered_fixture = dir.join("tampered.fixture.preserves");
    let deny_receipt = dir.join("deny.receipt.preserves");

    let record = molten_cmd()
        .args(["test", "replay-fixture", "record", "--out"])
        .arg(&fixture)
        .output()?;
    assert_success(&record, "replay fixture record");
    assert!(stdout(&record).contains("deterministic replay fixture written"));

    let pass = molten_cmd()
        .args(["test", "replay-fixture", "verify"])
        .arg(&fixture)
        .args(["--receipt-out"])
        .arg(&pass_receipt)
        .output()?;
    assert_success(&pass, "replay fixture verify pass");
    assert!(stdout(&pass).contains("decision=pass"));
    assert!(stdout(&pass).contains("divergence=none"));
    let pass_text = std::fs::read_to_string(&pass_receipt)?;
    assert!(pass_text.contains("deterministic-replay-verify-v1"));
    assert!(pass_text.contains("ordered-boundary-comparison"));

    let tamper = molten_cmd()
        .args(["test", "replay-fixture", "tamper"])
        .arg(&fixture)
        .args(["--kind", "effect-response", "--out"])
        .arg(&tampered_fixture)
        .output()?;
    assert_success(&tamper, "replay fixture tamper");
    assert!(stdout(&tamper).contains("tampered fixture"));
    assert!(std::fs::read_to_string(&tampered_fixture)?.contains("deterministic-fixture-record-v1"));

    let deny = molten_cmd()
        .args(["test", "replay-fixture", "verify"])
        .arg(&tampered_fixture)
        .args(["--receipt-out"])
        .arg(&deny_receipt)
        .output()?;
    assert_success(&deny, "replay fixture verify deny");
    assert!(stdout(&deny).contains("decision=deny"));
    assert!(stdout(&deny).contains("divergence=effect-response"));
    let deny_text = std::fs::read_to_string(&deny_receipt)?;
    assert!(deny_text.contains("first-divergence-ref"));
    assert!(deny_text.contains("recorded-effects-only"));
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
    assert_eq!(parsed_bundle.kind, molten::harness::ReproBundleKind::Report);
    assert!(parsed_bundle.gate_receipt_ref.is_some());
    let embedded_value = parsed_bundle
        .receipt_value
        .as_ref()
        .ok_or_else(|| test_error("sealed repro bundle missing embedded report gate receipt"))?;
    let embedded_receipt = molten::harness::parse_receipt(embedded_value)?;
    assert_eq!(embedded_receipt.artifact_kind, "report");
    let exported_receipt = molten::harness::parse_receipt(&read_preserves(&repro.join("gate-receipt.preserves"))?)?;
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
    let receipt = molten::harness::parse_receipt(&read_preserves(&bundle_receipt)?)?;
    assert_eq!(receipt.artifact_kind, "repro-bundle");
    Ok(())
}
