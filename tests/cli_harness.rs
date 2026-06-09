use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use molten::error::MoltenError;
use molten::harness::failure_value;
use molten::harness::gate_receipt_summary;
use molten::harness::parse_failure;
use molten::harness::parse_gate_receipt;
use molten::harness::parse_repro_bundle;
use molten::harness::parse_repro_verify_receipt;
use molten::harness::report_summary;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::retention;
use molten::secrets::RevealReceiptInput;
use molten::secrets::reveal_receipt_value;

type CliResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn cli_happy_path_produces_gateable_report_and_repro_bundle() -> CliResult<()> {
    let dir = temp_dir("cli-happy")?;
    let report = dir.join("report.preserves");
    let repro = dir.join("repro");
    let suite = manifest_dir().join("examples/two-actor.preserves");

    let run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&report).output()?;
    assert_success(&run, "test run");
    assert!(stdout(&run).contains("report blake3:"));

    let report_value = read_preserves(&report)?;
    let summary = report_summary(&report_value)?;
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
    let receipt_value = parse_text(&stdout(&gate))?;
    let receipt = parse_gate_receipt(&receipt_value)?;
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.artifact_kind, "report");
    assert!(gate_receipt_summary(&receipt_value)?.contains("decision=pass"));

    let export = molten_cmd().args(["test", "repro", "export"]).arg(&report).args(["--out"]).arg(&repro).output()?;
    assert_success(&export, "test repro export");
    assert!(stdout(&export).contains("repro bundle written"));

    let bundle = read_preserves(&repro.join("refs.preserves"))?;
    let parsed_bundle = parse_repro_bundle(&bundle)?;
    assert_eq!(parsed_bundle.kind, molten::harness::HarnessReproBundleKind::Report);
    assert!(parsed_bundle.gate_receipt_ref.is_some());
    let embedded_value = parsed_bundle
        .gate_receipt_value
        .as_ref()
        .ok_or_else(|| test_error("sealed repro bundle missing embedded report gate receipt"))?;
    let embedded_receipt = parse_gate_receipt(embedded_value)?;
    assert_eq!(embedded_receipt.artifact_kind, "report");
    let exported_receipt = parse_gate_receipt(&read_preserves(&repro.join("gate-receipt.preserves"))?)?;
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
    let verify = parse_repro_verify_receipt(&read_preserves(&verify_receipt)?)?;
    assert_eq!(verify.decision, "pass");
    assert_eq!(verify.report_ref, receipt.report_ref);

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
        molten::preserves_rail::canonical_hash(&report_value)?
    );
    parse_repro_verify_receipt(&read_preserves(&unpacked.join("verify-receipt.preserves"))?)?;

    let bundle_receipt = dir.join("bundle.gate-receipt.preserves");
    let gate_bundle = molten_cmd()
        .args(["test", "gate", "check"])
        .arg(repro.join("refs.preserves"))
        .args(["--receipt-out"])
        .arg(&bundle_receipt)
        .output()?;
    assert_success(&gate_bundle, "test gate check repro bundle");
    assert!(stdout(&gate_bundle).contains("gate receipt blake3:"));
    let receipt = parse_gate_receipt(&read_preserves(&bundle_receipt)?)?;
    assert_eq!(receipt.artifact_kind, "repro-bundle");
    Ok(())
}

#[test]
fn cli_repro_export_profiles_fail_closed_and_unpack_diagnostics() -> CliResult<()> {
    let dir = temp_dir("cli-repro-profiles")?;
    let suite = dir.join("secret-suite.preserves");
    let report = dir.join("report.preserves");
    fs::write(
        &suite,
        r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-cli" 1
          <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
          <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
          <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
          [<send "a" "b" <secret "token">>]>"#,
    )?;
    let run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&report).output()?;
    assert_success(&run, "secret test run");

    let denied_out = dir.join("default-repro");
    let denied = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(&report)
        .args(["--out"])
        .arg(&denied_out)
        .output()?;
    assert_failure(&denied, "default deny-sensitive export");
    assert!(stderr(&denied).contains("sensitive marker secret"));

    let diagnostic_out = dir.join("diagnostic-repro");
    let diagnostic = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(&report)
        .args(["--out"])
        .arg(&diagnostic_out)
        .args(["--profile", "redacted-diagnostic"])
        .output()?;
    assert_success(&diagnostic, "redacted diagnostic export");
    let diagnostic_bundle = read_preserves(&diagnostic_out.join("refs.preserves"))?;
    let parsed_diagnostic = parse_repro_bundle(&diagnostic_bundle)?;
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

    let encrypted_out = dir.join("encrypted-repro");
    let encrypted = molten_cmd()
        .args(["test", "repro", "export"])
        .arg(&report)
        .args(["--out"])
        .arg(&encrypted_out)
        .args(["--profile", "encrypted-private"])
        .output()?;
    assert_success(&encrypted, "encrypted private export");
    let encrypted_bundle = read_preserves(&encrypted_out.join("refs.preserves"))?;
    let parsed_encrypted = parse_repro_bundle(&encrypted_bundle)?;
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
    let legacy_reveal = reveal_receipt_value(&RevealReceiptInput {
        secret_ref: parsed_encrypted.encrypted_refs[0].clone(),
        encrypted_ref: None,
        requester_ref: canonical_hash(&string("cli-requester"))?,
        purpose: "export".to_string(),
        plaintext_ref: Some(canonical_hash(&string("authorized-private-material-legacy"))?),
        commitment_ref: parsed_encrypted.encrypted_refs[0].clone(),
        authority_refs: vec![canonical_hash(&string("reveal-authority"))?],
        policy_refs: vec![canonical_hash(&string("reveal-policy"))?],
        resource_refs: vec![canonical_hash(&string("reveal-resource"))?],
        effect_handle_refs: vec![canonical_hash(&string("reveal-effect-handle"))?],
        revocation_refs: Vec::new(),
    })?;
    let legacy_reveal_path = dir.join("legacy-reveal.preserves");
    fs::write(&legacy_reveal_path, to_text(&legacy_reveal)?)?;
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

    let wrong_reveal_ref = canonical_hash(&string("wrong-encrypted-ref"))?;
    let stale_reveal = reveal_receipt_value(&RevealReceiptInput {
        secret_ref: parsed_encrypted.encrypted_refs[0].clone(),
        encrypted_ref: Some(wrong_reveal_ref),
        requester_ref: canonical_hash(&string("cli-requester"))?,
        purpose: "export".to_string(),
        plaintext_ref: Some(canonical_hash(&string("authorized-private-material-stale"))?),
        commitment_ref: parsed_encrypted.encrypted_refs[0].clone(),
        authority_refs: vec![canonical_hash(&string("reveal-authority"))?],
        policy_refs: vec![canonical_hash(&string("reveal-policy"))?],
        resource_refs: vec![canonical_hash(&string("reveal-resource"))?],
        effect_handle_refs: vec![canonical_hash(&string("reveal-effect-handle"))?],
        revocation_refs: Vec::new(),
    })?;
    let stale_reveal_path = dir.join("stale-reveal.preserves");
    fs::write(&stale_reveal_path, to_text(&stale_reveal)?)?;
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

    let mut reveal_paths = Vec::with_capacity(parsed_encrypted.encrypted_refs.len());
    for (index, encrypted_ref) in parsed_encrypted.encrypted_refs.iter().enumerate() {
        let reveal = reveal_receipt_value(&RevealReceiptInput {
            secret_ref: encrypted_ref.clone(),
            encrypted_ref: Some(encrypted_ref.clone()),
            requester_ref: canonical_hash(&string("cli-requester"))?,
            purpose: "export".to_string(),
            plaintext_ref: Some(canonical_hash(&string(format!("authorized-private-material-{index}")))?),
            commitment_ref: encrypted_ref.clone(),
            authority_refs: vec![canonical_hash(&string("reveal-authority"))?],
            policy_refs: vec![canonical_hash(&string("reveal-policy"))?],
            resource_refs: vec![canonical_hash(&string("reveal-resource"))?],
            effect_handle_refs: vec![canonical_hash(&string("reveal-effect-handle"))?],
            revocation_refs: Vec::new(),
        })?;
        let reveal_path = dir.join(format!("reveal-{index}.preserves"));
        fs::write(&reveal_path, to_text(&reveal)?)?;
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

#[test]
fn cli_failure_paths_write_canonical_failure_artifacts_to_files() -> CliResult<()> {
    let dir = temp_dir("cli-failure-files")?;
    let bad_suite = dir.join("bad-suite.preserves");
    let run_failure = dir.join("run.failure.preserves");
    fs::write(
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
    let failure = parse_failure(&read_preserves(&run_failure)?)?;
    assert_eq!(failure.phase, "preflight");
    assert_eq!(failure.kind, "invalid-harness");
    assert!(failure.message.contains("unknown actor missing"));

    let good_report = dir.join("report.preserves");
    let suite = manifest_dir().join("examples/two-actor.preserves");
    let good_run = molten_cmd().args(["test", "run"]).arg(&suite).args(["--report-out"]).arg(&good_report).output()?;
    assert_success(&good_run, "setup test run");

    let tampered_report = dir.join("tampered.report.preserves");
    let report_text = fs::read_to_string(&good_report)?;
    fs::write(&tampered_report, report_text.replacen("message-delivered", "message-tampered", 1))?;

    let replay_failure = dir.join("replay.failure.preserves");
    let failed_replay = molten_cmd()
        .args(["test", "replay"])
        .arg(&tampered_report)
        .args(["--failure-out"])
        .arg(&replay_failure)
        .output()?;
    assert_failure(&failed_replay, "failing test replay");
    let failure = parse_failure(&read_preserves(&replay_failure)?)?;
    assert_eq!(failure.phase, "replay");
    assert_eq!(failure.kind, "trace");

    let invalid_report = dir.join("invalid.report.preserves");
    fs::write(&invalid_report, "<not-a-harness-report>\n")?;

    let validate_failure = dir.join("validate.failure.preserves");
    let failed_validate = molten_cmd()
        .args(["test", "report", "validate"])
        .arg(&invalid_report)
        .args(["--failure-out"])
        .arg(&validate_failure)
        .output()?;
    assert_failure(&failed_validate, "failing test report validate");
    let failure = parse_failure(&read_preserves(&validate_failure)?)?;
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
    let failure = parse_failure(&read_preserves(&export_failure)?)?;
    assert_eq!(failure.phase, "export");
    assert_eq!(failure.kind, "invalid-harness");
    Ok(())
}

#[test]
fn cli_blob_ref_job_submit_execute_status_and_receipt_show() -> CliResult<()> {
    let dir = temp_dir("cli-job-ref")?;
    let chunks = dir.join("chunks");
    let ledger = dir.join("ledger");
    let submission = dir.join("submission.preserves");
    let receipt_path = dir.join("receipt.preserves");
    let executable = molten::chunk_store::put_bytes(
        &chunks,
        "job-executable",
        b"echo",
        molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )?;
    let input = molten::chunk_store::put_bytes(
        &chunks,
        "job-input",
        b"cli-output",
        molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )?;
    let operation_id = test_ref("cli-job-ref-operation")?;
    let authority_ref = test_ref("cli-job-ref-authority")?;
    let policy_ref = test_ref("cli-job-ref-policy")?;
    let provenance_ref = test_ref("cli-job-ref-provenance")?;
    let effect_ref = test_ref("cli-job-ref-effect")?;
    let executable_arg = format!("{}@{}@elf-executable", executable.manifest_ref, executable.total_len);
    let input_arg = format!("{}@{}@bytes", input.manifest_ref, input.total_len);

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
    let diagnostic = failure_value("preflight", &MoltenError::invalid_harness("synthetic diagnostic"), Vec::new());
    fs::write(&failure_artifact, to_text(&diagnostic)?)?;

    let failed_gate = molten_cmd().args(["test", "gate", "check"]).arg(&failure_artifact).output()?;
    assert_failure(&failed_gate, "failing test gate check");

    let stdout_failure = parse_text(&stdout(&failed_gate))?;
    let failure = parse_failure(&stdout_failure)?;
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
    let deny_text = to_text(&read_preserves(&deny_receipt)?)?;
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
    fs::create_dir_all(&workspace)?;
    fs::create_dir_all(&lib)?;
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
    let text = to_text(&receipt_value)?;
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

    let init = molten_cmd()
        .args(["node", "init", "--state-root"])
        .arg(&state_root)
        .args(["--node-id", "node:cli", "--config-out"])
        .arg(&config)
        .output()?;
    assert_success(&init, "node init");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&config)?), "node-config");

    let run = molten_cmd()
        .args(["node", "run", "--state-root"])
        .arg(&state_root)
        .args(["--startup-out"])
        .arg(&startup)
        .output()?;
    assert_success(&run, "node run");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&startup)?), "node-startup-receipt");

    let authority_ref = test_ref("node-control-authority")?;
    let policy_ref = test_ref("node-control-policy")?;
    let resource_ref = test_ref("node-control-resource")?;
    let request = molten_cmd()
        .args(["node", "control-request", "--operation", "status", "--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--out"])
        .arg(&socket_request)
        .output()?;
    assert_success(&request, "node socket status request");
    let submit = molten_cmd()
        .args(["node", "control-submit", "--state-root"])
        .arg(&state_root)
        .arg(&socket_request)
        .args(["--receipt-out"])
        .arg(&socket_queue)
        .output()?;
    assert_success(&submit, "node socket status submit");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&socket_queue)?), "node-control-queue-receipt");
    let dispatch = molten_cmd()
        .args(["node", "control-dispatch", "--state-root"])
        .arg(&state_root)
        .args(["--receipt-out"])
        .arg(&socket_receipt)
        .output()?;
    assert_success(&dispatch, "node socket status dispatch");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&socket_receipt)?), "node-control-receipt");

    let status = molten_cmd()
        .args(["node", "status", "--state-root"])
        .arg(&state_root)
        .args(["--health-out"])
        .arg(&health)
        .args(["--receipt-out"])
        .arg(&status_receipt)
        .output()?;
    assert_success(&status, "node status");
    assert!(stdout(&status).contains("node status running"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&health)?), "node-health-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&status_receipt)?), "node-control-receipt");

    let shutdown_req = molten_cmd()
        .args(["node", "control-request", "--operation", "shutdown", "--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--out"])
        .arg(&shutdown_request)
        .output()?;
    assert_success(&shutdown_req, "node socket shutdown request");
    let shutdown_submit = molten_cmd()
        .args(["node", "control-submit", "--state-root"])
        .arg(&state_root)
        .arg(&shutdown_request)
        .args(["--receipt-out"])
        .arg(&shutdown_queue)
        .output()?;
    assert_success(&shutdown_submit, "node socket shutdown submit");
    let stop = molten_cmd()
        .args(["node", "run-loop", "--state-root"])
        .arg(&state_root)
        .args(["--max-requests", "4", "--receipt-out"])
        .arg(&loop_receipt)
        .output()?;
    assert_success(&stop, "node socket shutdown loop");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&shutdown)?), "node-shutdown-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&loop_receipt)?), "node-control-loop-receipt");

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
    let text = to_text(&receipt_value)?;
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

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-ingress"])
            .output()?,
        "node ingress init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node ingress run",
    );
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
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&request)
            .output()?,
        "node ingress request",
    );
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
            .arg(&bootstrap_ref)
            .args(["--authority"])
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&envelope)
            .arg(&request)
            .output()?,
        "node ingress build",
    );
    let envelope_value = read_preserves(&envelope)?;
    assert_eq!(molten::ledger::artifact_kind(&envelope_value), "node-control-ingress-envelope");
    let envelope_ref = molten::preserves_rail::canonical_hash(&envelope_value)?;
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-publish", "--state-root"])
            .arg(&state_root)
            .arg(&envelope)
            .args(["--receipt-out"])
            .arg(&publish_receipt)
            .output()?,
        "node ingress publish",
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&publish_receipt)?), "node-control-ingress-receipt");
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-deliver", "--state-root"])
            .arg(&state_root)
            .arg(&envelope_ref)
            .args(["--receipt-out"])
            .arg(&deliver_receipt)
            .output()?,
        "node ingress deliver",
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&deliver_receipt)?), "node-control-ingress-receipt");
    let loop_out = molten_cmd()
        .args(["test", "node", "run-loop", "--state-root"])
        .arg(&state_root)
        .args(["--max-requests", "1", "--receipt-out"])
        .arg(&loop_receipt)
        .output()?;
    assert_success(&loop_out, "node ingress loop");
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

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-live"])
            .output()?,
        "node live init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node live run",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "authority-grant-fixture", "--state-root"])
            .arg(&state_root)
            .args([
                "--peer",
                "peer:cli-live",
                "--node",
                "node:cli-live",
                "--operation",
                "status",
                "--policy",
            ])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&authority_grant)
            .output()?,
        "node live authority grant",
    );
    let authority_ref = molten::preserves_rail::canonical_hash(&read_preserves(&authority_grant)?)?;
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-ticket-export", "--state-root"])
            .arg(&state_root)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&live_ticket)
            .output()?,
        "node live ticket export",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-peer-admit", "--state-root"])
            .arg(&state_root)
            .args(["--peer", "peer:cli-live", "--policy"])
            .arg(&policy_ref)
            .args(["--receipt-out"])
            .arg(&peer_admission)
            .arg(&live_ticket)
            .output()?,
        "node live peer admit",
    );
    let bootstrap_ref = molten::preserves_rail::canonical_hash(&read_preserves(&peer_admission)?)?;
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
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&request)
            .output()?,
        "node live status request",
    );
    let loopback = molten_cmd()
        .args(["test", "node", "control-ingress-live-loopback", "--state-root"])
        .arg(&state_root)
        .args([
            "--from-peer",
            "peer:cli-live",
            "--to-node",
            "node:cli-live",
            "--peer-bootstrap",
        ])
        .arg(&bootstrap_ref)
        .args(["--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--publish-receipt-out"])
        .arg(&publish_receipt)
        .args(["--receive-receipt-out"])
        .arg(&receive_receipt)
        .arg(&request)
        .output()?;
    assert_success(&loopback, "node live ingress loopback");
    assert!(stdout(&loopback).contains("enqueued=yes"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&publish_receipt)?),
        "node-control-live-transport-receipt"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&receive_receipt)?),
        "node-control-live-transport-receipt"
    );
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

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&receiver_root)
            .args(["--node-id", "node:cli-live-import"])
            .output()?,
        "receiver init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&receiver_root).output()?,
        "receiver run",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&sender_root)
            .args(["--node-id", "node:cli-live-import-sender"])
            .output()?,
        "sender init",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&bundle_sender_root)
            .args(["--node-id", "node:cli-live-bundle-sender"])
            .output()?,
        "bundle sender init",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&bundle_apply_root)
            .args(["--node-id", "node:cli-live-bundle-apply"])
            .output()?,
        "bundle apply init",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "authority-grant-fixture", "--state-root"])
            .arg(&receiver_root)
            .args([
                "--peer",
                "peer:cli-live-import",
                "--node",
                "node:cli-live-import",
                "--operation",
                "status",
                "--policy",
            ])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&authority_grant)
            .output()?,
        "receiver authority grant",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-ticket-export", "--state-root"])
            .arg(&receiver_root)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&live_ticket)
            .output()?,
        "receiver live ticket",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-peer-admit", "--state-root"])
            .arg(&receiver_root)
            .args(["--peer", "peer:cli-live-import", "--policy"])
            .arg(&policy_ref)
            .args(["--receipt-out"])
            .arg(&peer_admission)
            .arg(&live_ticket)
            .output()?,
        "receiver peer admit",
    );
    let authority_ref = molten::preserves_rail::canonical_hash(&read_preserves(&authority_grant)?)?;
    let bootstrap_ref = molten::preserves_rail::canonical_hash(&read_preserves(&peer_admission)?)?;
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
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&missing_import_request)
            .output()?,
        "missing import request",
    );
    let missing_import_send = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(&sender_root)
        .arg(&missing_import_request)
        .arg(&live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "wrong-topic",
            "--peer-bootstrap",
        ])
        .arg(&bootstrap_ref)
        .args(["--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--receipt-out"])
        .arg(&missing_import_send_receipt)
        .output()?;
    assert_success(&missing_import_send, "live send missing imports deny receipt");
    let missing_import_text = to_text(&read_preserves(&missing_import_send_receipt)?)?;
    assert!(missing_import_text.contains("live-ticket-import"));
    assert!(missing_import_text.contains("authority-grant-import"));
    assert!(missing_import_text.contains("ticket topic node-control does not match expected wrong-topic"));
    assert!(missing_import_text.contains("receiver-ticket-expected"));
    assert!(missing_import_text.contains("sender-state-root-evidence"));

    let bundle_export_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-export", "--ticket"])
        .arg(&live_ticket)
        .args(["--peer-admission"])
        .arg(&peer_admission)
        .args(["--authority-grant"])
        .arg(&authority_grant)
        .args(["--receipt"])
        .arg(&missing_import_send_receipt)
        .args(["--out"])
        .arg(&workflow_bundle)
        .args(["--receipt-out"])
        .arg(&bundle_export)
        .output()?;
    assert_success(&bundle_export_out, "live workflow bundle export");
    assert!(stdout(&bundle_export_out).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&workflow_bundle)?),
        "node-control-live-workflow-bundle"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_export)?),
        "node-control-live-workflow-bundle-export-receipt"
    );
    let bundle_verify_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-verify"])
        .arg(&workflow_bundle)
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
        .arg(&bundle_verify)
        .output()?;
    assert_success(&bundle_verify_out, "live workflow bundle verify");
    assert!(stdout(&bundle_verify_out).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_verify)?),
        "node-control-live-workflow-bundle-verify-receipt"
    );
    let bundle_verify_text = to_text(&read_preserves(&bundle_verify)?)?;
    assert!(bundle_verify_text.contains("verify-receipt-is-not-authority"));

    let bundle_gate_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-gate"])
        .arg(&workflow_bundle)
        .args(["--verify-receipt"])
        .arg(&bundle_verify)
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
        .arg(&bundle_gate)
        .output()?;
    assert_success(&bundle_gate_out, "live workflow bundle gate");
    assert!(stdout(&bundle_gate_out).contains("decision=pass"));
    assert!(stdout(&bundle_gate_out).contains("next-step=import-bundle"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_gate)?),
        "node-control-live-workflow-bundle-gate-receipt"
    );
    let bundle_gate_text = to_text(&read_preserves(&bundle_gate)?)?;
    assert!(bundle_gate_text.contains("gate-receipt-is-not-authority"));

    let bundle_apply_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-apply", "--state-root"])
        .arg(&bundle_apply_root)
        .arg(&workflow_bundle)
        .args(["--gate-receipt"])
        .arg(&bundle_gate)
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
        .arg(&bundle_apply)
        .output()?;
    assert_success(&bundle_apply_out, "live workflow bundle apply");
    assert!(stdout(&bundle_apply_out).contains("decision=pass"));
    assert!(stdout(&bundle_apply_out).contains("next-step=dry-run-or-send-request"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_apply)?),
        "node-control-live-workflow-bundle-apply-receipt"
    );
    let bundle_apply_text = to_text(&read_preserves(&bundle_apply)?)?;
    assert!(bundle_apply_text.contains("apply-receipt-is-not-authority"));

    let bundle_reconcile_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-reconcile"])
        .arg(&bundle_apply)
        .args(["--receipt-out"])
        .arg(&bundle_reconcile)
        .output()?;
    assert_success(&bundle_reconcile_out, "live workflow bundle reconcile missing receiver");
    assert!(stdout(&bundle_reconcile_out).contains("decision=deny"));
    assert!(stdout(&bundle_reconcile_out).contains("next-step=wait-or-import-receiver-ingress"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_reconcile)?),
        "node-control-live-workflow-bundle-reconcile-receipt"
    );
    let bundle_reconcile_text = to_text(&read_preserves(&bundle_reconcile)?)?;
    assert!(bundle_reconcile_text.contains("reconcile-receipt-is-not-authority"));

    let bundle_ack_export_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-export"])
        .arg(&bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(&bundle_reconcile)
        .args(["--out"])
        .arg(&bundle_ack)
        .args(["--receipt-out"])
        .arg(&bundle_ack_export)
        .output()?;
    assert_success(&bundle_ack_export_out, "live workflow bundle ack export missing receiver");
    assert!(stdout(&bundle_ack_export_out).contains("decision=deny"));
    assert!(stdout(&bundle_ack_export_out).contains("next-step=collect-receiver-evidence"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_ack)?),
        "node-control-live-workflow-bundle-ack"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_ack_export)?),
        "node-control-live-workflow-bundle-ack-export-receipt"
    );
    let bundle_ack_text = to_text(&read_preserves(&bundle_ack)?)?;
    assert!(bundle_ack_text.contains("ack-bundle-is-not-authority"));

    let bundle_ack_import_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-import", "--state-root"])
        .arg(&bundle_sender_root)
        .arg(&bundle_ack)
        .args(["--receipt-out"])
        .arg(&bundle_ack_import)
        .output()?;
    assert_success(&bundle_ack_import_out, "live workflow bundle ack import missing receiver");
    assert!(stdout(&bundle_ack_import_out).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_ack_import)?),
        "node-control-live-workflow-bundle-ack-import-receipt"
    );
    let bundle_ack_import_text = to_text(&read_preserves(&bundle_ack_import)?)?;
    assert!(bundle_ack_import_text.contains("ack-import-is-not-authority"));

    let bundle_protocol_gate_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-protocol-gate"])
        .arg(&workflow_bundle)
        .args(["--gate-receipt"])
        .arg(&bundle_gate)
        .args(["--apply-receipt"])
        .arg(&bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(&bundle_reconcile)
        .args(["--ack"])
        .arg(&bundle_ack)
        .args(["--receipt-out"])
        .arg(&bundle_protocol_gate)
        .output()?;
    assert_success(&bundle_protocol_gate_out, "live workflow bundle protocol gate missing receiver");
    assert!(stdout(&bundle_protocol_gate_out).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_protocol_gate)?),
        "protocol-session-gate-receipt"
    );
    let bundle_protocol_gate_text = to_text(&read_preserves(&bundle_protocol_gate)?)?;
    assert!(bundle_protocol_gate_text.contains("ack receiver decision deny"));
    assert!(bundle_protocol_gate_text.contains("protocol-session-gate-is-not-authority"));

    let bundle_import_out = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-import", "--state-root"])
        .arg(&bundle_sender_root)
        .arg(&workflow_bundle)
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
        .arg(&bundle_import)
        .output()?;
    assert_success(&bundle_import_out, "live workflow bundle import");
    assert!(stdout(&bundle_import_out).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&bundle_import)?),
        "node-control-live-workflow-bundle-import-receipt"
    );
    let bundle_import_send = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(&bundle_sender_root)
        .arg(&missing_import_request)
        .arg(&live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "node-control",
            "--peer-bootstrap",
        ])
        .arg(&bootstrap_ref)
        .args(["--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--receipt-out"])
        .arg(&bundle_import_send_receipt)
        .output()?;
    assert_success(&bundle_import_send, "live send after workflow bundle import");
    let bundle_send_text = to_text(&read_preserves(&bundle_import_send_receipt)?)?;
    assert!(bundle_send_text.contains("ticket has no endpoint addresses"));
    assert!(!bundle_send_text.contains("authority-grant-import"));
    assert!(!bundle_send_text.contains("peer admission unavailable in sender state root"));
    assert!(!bundle_send_text.contains("authority grant unavailable in sender state root"));

    let ticket_import_out = molten_cmd()
        .args(["test", "node", "live-ticket-import", "--state-root"])
        .arg(&sender_root)
        .arg(&live_ticket)
        .args(["--peer-admission"])
        .arg(&peer_admission)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--receipt-out",
        ])
        .arg(&ticket_import)
        .output()?;
    assert_success(&ticket_import_out, "sender live ticket import");
    assert!(stdout(&ticket_import_out).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&ticket_import)?),
        "node-control-live-ticket-import-receipt"
    );

    let grant_import_out = molten_cmd()
        .args(["test", "node", "authority-grant-import", "--state-root"])
        .arg(&sender_root)
        .arg(&authority_grant)
        .args([
            "--peer",
            "peer:cli-live-import",
            "--node",
            "node:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(&grant_import)
        .output()?;
    assert_success(&grant_import_out, "sender authority grant import");
    assert!(stdout(&grant_import_out).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&grant_import)?),
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

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-live-send"])
            .output()?,
        "node live send init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node live send run",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "authority-grant-fixture", "--state-root"])
            .arg(&state_root)
            .args([
                "--peer",
                "peer:cli-live-send",
                "--node",
                "node:cli-live-send",
                "--operation",
                "status",
                "--policy",
            ])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&authority_grant)
            .output()?,
        "node live send authority grant",
    );
    let authority_ref = molten::preserves_rail::canonical_hash(&read_preserves(&authority_grant)?)?;
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
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&request)
            .output()?,
        "node live send request",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-ticket-export", "--state-root"])
            .arg(&state_root)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&ticket)
            .output()?,
        "node live send ticket",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-peer-admit", "--state-root"])
            .arg(&state_root)
            .args(["--peer", "peer:cli-live-send", "--policy"])
            .arg(&policy_ref)
            .args(["--receipt-out"])
            .arg(&peer_admission)
            .arg(&ticket)
            .output()?,
        "node live send peer admit",
    );
    let bootstrap_ref = molten::preserves_rail::canonical_hash(&read_preserves(&peer_admission)?)?;
    let sent = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(&state_root)
        .arg(&request)
        .arg(&ticket)
        .args(["--from-peer", "peer:cli-live-send", "--peer-bootstrap"])
        .arg(&bootstrap_ref)
        .args(["--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--transport-receipt-out"])
        .arg(&transport_receipt)
        .args(["--receipt-out"])
        .arg(&send_receipt)
        .output()?;
    assert_success(&sent, "node live send deny no address");
    assert!(stdout(&sent).contains("transport_receipt=none"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&send_receipt)?), "node-control-live-send-receipt");
    assert!(!transport_receipt.exists());
    let text = to_text(&read_preserves(&send_receipt)?)?;
    assert!(text.contains("ticket has no endpoint addresses"));
    let operation_mismatch = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(&state_root)
        .arg(&request)
        .arg(&ticket)
        .args(["--from-peer", "peer:cli-live-send", "--operation-id"])
        .arg(&wrong_operation_ref)
        .args(["--peer-bootstrap"])
        .arg(&bootstrap_ref)
        .args(["--authority"])
        .arg(&authority_ref)
        .args(["--policy"])
        .arg(&policy_ref)
        .args(["--resource"])
        .arg(&resource_ref)
        .args(["--receipt-out"])
        .arg(&operation_mismatch_receipt)
        .output()?;
    assert_success(&operation_mismatch, "node live send deny operation mismatch");
    let mismatch_text = to_text(&read_preserves(&operation_mismatch_receipt)?)?;
    assert!(mismatch_text.contains("operation-id"));
    assert_success(
        &molten_cmd()
            .args(["test", "node", "serve", "--state-root"])
            .arg(&state_root)
            .args(["--max-ticks", "1", "--receipt-out"])
            .arg(&service_receipt)
            .output()?,
        "node live send service receipt",
    );
    let bundled = molten_cmd()
        .args(["test", "node", "live-workflow-bundle", "--state-root"])
        .arg(&state_root)
        .args(["--ticket"])
        .arg(&ticket)
        .args(["--peer-admission"])
        .arg(&peer_admission)
        .args(["--authority-grant"])
        .arg(&authority_grant)
        .args(["--send-receipt"])
        .arg(&send_receipt)
        .args(["--service-receipt"])
        .arg(&service_receipt)
        .args(["--receipt-out"])
        .arg(&workflow_receipt)
        .output()?;
    assert_success(&bundled, "node live workflow bundle deny");
    assert!(stdout(&bundled).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&workflow_receipt)?),
        "node-control-live-workflow-receipt"
    );
    let workflow_text = to_text(&read_preserves(&workflow_receipt)?)?;
    assert!(workflow_text.contains("missing receive receipt"));
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
    let service_text = to_text(&service_value)?;
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
    fs::create_dir_all(&artifacts)?;
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
    let receipt_text = to_text(&receipt_value)?;
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
    let requester_ref = test_ref("retention-plan-requester")?;
    let object_ref = test_ref("retention-plan-object")?;
    let peer_ref = test_ref("retention-plan-peer")?;
    let remote_ref = test_ref("retention-plan-remote")?;
    let store_admission = |kind: &str, label: &str, remote_refs: &[String]| -> CliResult<String> {
        Ok(retention::store_retention_evidence_admission(&root, &retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: retention::CLASS_DURABLE_VALUE,
            action: retention::ACTION_DELETE,
            bound_refs: &[test_ref(label)?],
            retained_refs: &[],
            remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })?
        .admission_ref)
    };
    let policy_ref = store_admission(retention::ADMISSION_KIND_POLICY, "retention-plan-policy", &[])?;
    let authority_ref = store_admission(retention::ADMISSION_KIND_AUTHORITY, "retention-plan-authority", &[])?;
    let support_ref = store_admission(retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "retention-plan-support", &[])?;
    let index_ref = store_admission(retention::ADMISSION_KIND_REFERENCE_INDEX, "retention-plan-index", &[])?;
    let remote_gc_ref = store_admission(
        retention::ADMISSION_KIND_REMOTE_GC,
        "retention-plan-remote-gc",
        std::slice::from_ref(&remote_ref),
    )?;
    let clearance =
        retention::store_retention_remote_gc_clearance(&root, &retention::RetentionRemoteGcClearanceInput {
            decision: "pass",
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: retention::CLASS_DURABLE_VALUE,
            action: retention::ACTION_DELETE,
            remote_ref: &remote_ref,
            policy_ref: &policy_ref,
            authority_ref: &authority_ref,
            evidence_refs: std::slice::from_ref(&support_ref),
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })?;
    let output = molten_cmd()
        .args(["test", "retention", "gc-plan", "--root"])
        .arg(&root)
        .args(["--subsystem", "ledger-gc", "--object-ref"])
        .arg(&object_ref)
        .args([
            "--object-kind",
            "chunk",
            "--retention-class",
            retention::CLASS_DURABLE_VALUE,
            "--action",
            "delete",
        ])
        .args(["--retention-requester"])
        .arg(&requester_ref)
        .args(["--retention-policy-ref"])
        .arg(&policy_ref)
        .args(["--retention-authority-ref"])
        .arg(&authority_ref)
        .args(["--retention-evidence-ref"])
        .arg(&support_ref)
        .args(["--retention-remote-peer-ref"])
        .arg(&peer_ref)
        .args(["--retention-remote-ref"])
        .arg(&remote_ref)
        .args(["--retention-reference-index-ref"])
        .arg(&index_ref)
        .args(["--retention-remote-gc-ref"])
        .arg(&remote_gc_ref)
        .args(["--retention-remote-clearance-ref"])
        .arg(&clearance.clearance_ref)
        .args(["--retention-reference-index-complete", "--out"])
        .arg(&plan_path)
        .output()?;
    assert_success(&output, "retention gc-plan");
    assert!(stdout(&output).contains("retention gc plan ref="));
    let plan_value = read_preserves(&plan_path)?;
    assert_eq!(molten::ledger::artifact_kind(&plan_value), "retention-gc-plan");
    let plan = retention::parse_retention_gc_plan(&plan_value)?;
    assert_eq!(plan.decision, "pass");
    assert!(plan.gates.iter().any(|gate| gate.name == "remote-clearance" && gate.decision == "pass"));
    let show = molten_cmd().args(["test", "retention", "show"]).arg(&plan_path).output()?;
    assert_success(&show, "retention show gc-plan");
    assert!(stdout(&show).contains("retention gc plan"));
    let apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&apply_output, "retention gc-apply-plan");
    assert!(stdout(&apply_output).contains("retention gc apply ref="));
    let apply_value = read_preserves(&apply_path)?;
    assert_eq!(molten::ledger::artifact_kind(&apply_value), "retention-gc-apply");
    let apply = retention::parse_retention_gc_apply(&apply_value)?;
    assert_eq!(apply.decision, "pass");
    assert_eq!(apply.plan_ref, plan.plan_ref);
    assert!(apply.retention_receipt_ref.is_some());
    assert!(apply.tombstone_ref.is_some());
    Ok(())
}

#[test]
fn cli_retention_gc_negative_regression_matrix() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-negative")?;

    let missing_plan_root = dir.join("missing-plan-root");
    let missing_plan_ref = test_ref("retention-missing-plan")?;
    let missing_plan_apply = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&missing_plan_root)
        .args(["--plan-ref"])
        .arg(&missing_plan_ref)
        .args(["--receipt-out"])
        .arg(dir.join("missing-plan-apply.preserves"))
        .output()?;
    assert_failure(&missing_plan_apply, "retention apply missing plan ref");

    let stale_plan_root = dir.join("stale-plan-root");
    let stale_candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &stale_plan_root,
        label: "stale-plan",
        object_ref: test_ref("retention-stale-object")?,
        object_kind: "artifact",
        retention_class: retention::CLASS_PUBLIC_ARTIFACT,
        action: retention::ACTION_DELETE,
    })?;
    let stale_plan = run_retention_gc_plan_cli(&stale_candidate, "ledger-gc", &dir.join("stale-plan.preserves"))?;
    retention::pin_object(&stale_plan_root, retention::RetentionPinInput {
        object_ref: stale_candidate.object_ref.clone(),
        object_kind: stale_candidate.object_kind.clone(),
        retention_class: stale_candidate.retention_class.clone(),
        source: retention::SOURCE_OPERATOR_HOLD.to_string(),
        reason: "negative CLI stale plan".to_string(),
        owner_ref: stale_candidate.requester_ref.clone(),
        expiry_ref: None,
        policy_refs: vec![stale_candidate.policy_ref.clone()],
        evidence_refs: vec![stale_candidate.support_ref.clone()],
        has_authority: true,
    })?;
    let stale_apply_path = dir.join("stale-apply.preserves");
    let stale_apply = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&stale_plan_root)
        .args(["--plan-ref"])
        .arg(&stale_plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&stale_apply_path)
        .output()?;
    assert_success(&stale_apply, "retention apply stale plan ref");
    let stale_apply_receipt = retention::parse_retention_gc_apply(&read_preserves(&stale_apply_path)?)?;
    assert_eq!(stale_apply_receipt.decision, "deny");
    assert!(stale_apply_receipt.retention_receipt_ref.is_none());
    assert!(stale_apply_receipt.tombstone_ref.is_none());
    assert!(
        stale_apply_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-apply-plan-drift")
    );
    assert!(stale_apply_receipt.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));

    let missing_apply_root = dir.join("missing-apply-ledger");
    let missing_apply_artifact =
        molten::ledger::import_artifact(&missing_apply_root, &parse_text("<artifact \"missing-apply\">")?)?;
    let missing_apply_candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &missing_apply_root,
        label: "missing-apply",
        object_ref: missing_apply_artifact.artifact_ref.clone(),
        object_kind: &missing_apply_artifact.artifact_kind,
        retention_class: retention::CLASS_PUBLIC_ARTIFACT,
        action: retention::ACTION_DELETE,
    })?;
    let missing_apply_receipt = dir.join("missing-apply-ledger-gc.preserves");
    let mut missing_apply_gc = molten_cmd();
    missing_apply_gc
        .args(["test", "ledger", "gc", "--ledger"])
        .arg(&missing_apply_root)
        .args(["--receipt-out"])
        .arg(&missing_apply_receipt);
    add_retention_args(&mut missing_apply_gc, &missing_apply_candidate);
    let missing_apply_output = missing_apply_gc.output()?;
    assert_success(&missing_apply_output, "ledger gc missing apply ref");
    assert!(stdout(&missing_apply_output).contains("decision=deny"));
    let missing_apply_text = fs::read_to_string(&missing_apply_receipt)?;
    assert!(missing_apply_text.contains("retention-gc-execute-apply-missing"));
    molten::ledger::read_artifact(&missing_apply_root, &missing_apply_candidate.object_ref)?;

    let wrong_apply_root = dir.join("wrong-apply-ledger");
    let wrong_apply_artifact =
        molten::ledger::import_artifact(&wrong_apply_root, &parse_text("<artifact \"wrong-apply\">")?)?;
    let wrong_apply_candidate = setup_retention_cli_candidate(RetentionCandidateInput {
        root: &wrong_apply_root,
        label: "wrong-apply",
        object_ref: wrong_apply_artifact.artifact_ref.clone(),
        object_kind: &wrong_apply_artifact.artifact_kind,
        retention_class: retention::CLASS_PUBLIC_ARTIFACT,
        action: retention::ACTION_DELETE,
    })?;
    let wrong_plan = run_retention_gc_plan_cli(&wrong_apply_candidate, "chunk-gc", &dir.join("wrong-plan.preserves"))?;
    let wrong_apply_path = dir.join("wrong-apply.preserves");
    let wrong_apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&wrong_apply_root)
        .args(["--plan-ref"])
        .arg(&wrong_plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&wrong_apply_path)
        .output()?;
    assert_success(&wrong_apply_output, "retention apply wrong subsystem plan");
    let wrong_apply = retention::parse_retention_gc_apply(&read_preserves(&wrong_apply_path)?)?;
    assert_eq!(wrong_apply.decision, "pass");
    let wrong_apply_receipt = dir.join("wrong-apply-ledger-gc.preserves");
    let mut wrong_apply_gc = molten_cmd();
    wrong_apply_gc
        .args(["test", "ledger", "gc", "--ledger"])
        .arg(&wrong_apply_root)
        .args(["--apply-ref"])
        .arg(&wrong_apply.apply_ref)
        .args(["--receipt-out"])
        .arg(&wrong_apply_receipt);
    add_retention_args(&mut wrong_apply_gc, &wrong_apply_candidate);
    let wrong_apply_output = wrong_apply_gc.output()?;
    assert_success(&wrong_apply_output, "ledger gc wrong apply ref");
    assert!(stdout(&wrong_apply_output).contains("decision=deny"));
    let wrong_apply_text = fs::read_to_string(&wrong_apply_receipt)?;
    assert!(wrong_apply_text.contains("retention-gc-execute-apply-scope-mismatch"));
    molten::ledger::read_artifact(&wrong_apply_root, &wrong_apply_candidate.object_ref)?;

    let audit_root = dir.join("audit-root");
    let missing_execution = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&audit_root)
        .args(["--execution-ref"])
        .arg(test_ref("missing-execution")?)
        .args(["--out"])
        .arg(dir.join("missing-execution-audit.preserves"))
        .output()?;
    assert_failure(&missing_execution, "retention audit missing execution ref");
    let denied_execution = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
        root: &audit_root,
        subsystem: "ledger-gc",
        action: retention::ACTION_DELETE,
        object_ref: &test_ref("denied-execution-object")?,
        object_kind: "artifact",
        retention_class: retention::CLASS_PUBLIC_ARTIFACT,
        apply_ref: None,
    })?;
    let denied_audit_path = dir.join("denied-execution-audit.preserves");
    let denied_audit = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&audit_root)
        .args(["--execution-ref"])
        .arg(&denied_execution.execution_ref)
        .args(["--out"])
        .arg(&denied_audit_path)
        .output()?;
    assert_success(&denied_audit, "retention audit denied execution ref");
    let denied_audit = retention::parse_retention_gc_audit(&read_preserves(&denied_audit_path)?)?;
    assert_eq!(denied_audit.decision, "deny");
    assert!(denied_audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-apply-missing"));
    assert!(denied_audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-plan-missing"));
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
        retention_class: retention::CLASS_PUBLIC_ARTIFACT,
        action: retention::ACTION_DELETE,
    })?;
    let fixture = setup_retention_gc_catalog_fixture(&candidate, "ledger-gc", &dir)?;

    let explain_path = dir.join("retention-explain.preserves");
    let explain_output = molten_cmd()
        .args(["test", "retention", "explain", "--root"])
        .arg(&retention_root)
        .args(["--object-ref"])
        .arg(&fixture.object_ref)
        .args([
            "--object-kind",
            "artifact",
            "--retention-class",
            retention::CLASS_PUBLIC_ARTIFACT,
            "--action",
            retention::ACTION_DELETE,
            "--subsystem",
            "ledger-gc",
            "--out",
        ])
        .arg(&explain_path)
        .output()?;
    assert_success(&explain_output, "retention explain candidate");
    assert!(stdout(&explain_output).contains("retention explain ref="));
    let explain = retention::parse_retention_candidate_explain(&read_preserves(&explain_path)?)?;
    assert_eq!(explain.object_ref, fixture.object_ref);
    assert_eq!(explain.admission_refs.len(), 4);
    assert_eq!(explain.gc_plan_refs, vec![fixture.plan_ref.clone()]);
    assert_eq!(explain.gc_apply_refs, vec![fixture.apply_ref.clone()]);
    assert_eq!(explain.gc_execution_refs, vec![fixture.execution_ref.clone()]);
    assert_eq!(explain.gc_audit_refs, vec![fixture.audit_ref.clone()]);
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&explain_path)?), "retention-candidate-explain");

    let bundle_dir = dir.join("retention-bundle");
    let bundle_output = molten_cmd()
        .args(["test", "retention", "bundle-export", "--root"])
        .arg(&retention_root)
        .args(["--explain"])
        .arg(&explain_path)
        .args(["--out"])
        .arg(&bundle_dir)
        .output()?;
    assert_success(&bundle_output, "retention bundle export");
    assert!(stderr(&bundle_output).contains("retention bundle ref="));
    let bundle_value = read_preserves(&bundle_dir.join("bundle.preserves"))?;
    let bundle = retention::parse_retention_candidate_bundle(&bundle_value)?;
    assert_eq!(molten::ledger::artifact_kind(&bundle_value), "retention-candidate-bundle");
    assert_eq!(bundle.explain_ref, explain.explain_ref);
    assert_eq!(bundle.artifact_refs.len(), 6);
    assert!(bundle.diagnostics.is_empty());
    assert!(bundle_dir.join("explain.preserves").exists());
    assert!(bundle_dir.join("artifacts/gc-plans").exists());
    assert!(bundle_dir.join("artifacts/gc-audits").exists());

    let verify_path = dir.join("retention-bundle-verify.preserves");
    let verify_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(&bundle_dir)
        .args(["--receipt-out"])
        .arg(&verify_path)
        .output()?;
    assert_success(&verify_output, "retention bundle verify");
    assert!(stderr(&verify_output).contains("retention bundle verify ref="));
    let verify_value = read_preserves(&verify_path)?;
    let verify = retention::parse_retention_candidate_bundle_verify(&verify_value)?;
    assert_eq!(molten::ledger::artifact_kind(&verify_value), "retention-candidate-bundle-verify");
    assert_eq!(verify.decision, "pass");
    assert_eq!(verify.bundle_ref, bundle.bundle_ref);
    assert_eq!(verify.file_refs.len(), 6);
    assert!(verify.diagnostics.is_empty());
    let verify_import = molten_cmd()
        .args(["test", "ledger", "import"])
        .arg(&verify_path)
        .args(["--ledger"])
        .arg(&ledger_root)
        .output()?;
    assert_success(&verify_import, "ledger import retention bundle verify");
    let verify_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(&registry)
        .args(["--ledger"])
        .arg(&ledger_root)
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

    let tampered_plan_path = bundle_dir
        .join("artifacts/gc-plans")
        .join(format!("{}.preserves", fixture.plan_ref.replace(':', "_")));
    fs::write(&tampered_plan_path, to_text(&record("tampered", vec![string("plan")]))?)?;
    let tampered_path = dir.join("retention-bundle-verify-tampered.preserves");
    let tampered_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(&bundle_dir)
        .args(["--receipt-out"])
        .arg(&tampered_path)
        .output()?;
    assert_success(&tampered_output, "retention bundle verify tampered");
    let tampered = retention::parse_retention_candidate_bundle_verify(&read_preserves(&tampered_path)?)?;
    assert_eq!(tampered.decision, "deny");
    assert!(
        tampered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
    );

    let search_receipt = dir.join("catalog-search-receipt.preserves");
    let search_output = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(&registry)
        .args(["--ledger"])
        .arg(&ledger_root)
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
        .arg(&registry)
        .args(["--ledger"])
        .arg(&ledger_root)
        .args(["--ledger-kind", "retention-gc-audit", "--text", "retention-gc:audit"])
        .output()?;
    assert_success(&audit_search, "catalog search retention GC audit ledger kind");
    let audit_search_stdout = stdout(&audit_search);
    assert!(audit_search_stdout.contains("retention-gc:audit"));
    assert!(audit_search_stdout.contains(&fixture.audit_ref));

    let mcp_request_path = dir.join("retention-gc-search-request.preserves");
    let mcp_response_path = dir.join("retention-gc-search-response.preserves");
    let mcp_receipt_path = dir.join("retention-gc-search-mcp-receipt.preserves");
    let mcp_request = molten::catalog_mcp::mcp_request_value("search_retention_gc", vec![
        record("stage", vec![string("audit")]),
        record("object-ref", vec![string(&fixture.object_ref)]),
        record("subsystem", vec![string("ledger-gc")]),
        record("execution-ref", vec![string(&fixture.execution_ref)]),
    ])?;
    fs::write(&mcp_request_path, to_text(&mcp_request)?)?;
    let mcp_output = molten_cmd()
        .args(["test", "catalog", "mcp-call"])
        .arg(&mcp_request_path)
        .args(["--registry"])
        .arg(&registry)
        .args(["--ledger"])
        .arg(&ledger_root)
        .args(["--out"])
        .arg(&mcp_response_path)
        .args(["--receipt-out"])
        .arg(&mcp_receipt_path)
        .output()?;
    assert_success(&mcp_output, "catalog MCP search_retention_gc");
    let mcp_response = fs::read_to_string(&mcp_response_path)?;
    assert!(mcp_response.contains("retention-gc:audit"));
    assert!(mcp_response.contains(&fixture.execution_ref));
    let mcp_receipt = molten::catalog_mcp::parse_mcp_receipt(&read_preserves(&mcp_receipt_path)?)?;
    assert_eq!(mcp_receipt.tool, "search_retention_gc");
    assert_eq!(mcp_receipt.decision, "pass");
    Ok(())
}

struct RetentionCandidateInput<'a> {
    root: &'a Path,
    label: &'a str,
    object_ref: String,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct RetentionCliCandidate {
    root: PathBuf,
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
    dir: &Path,
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
    let apply = retention::parse_retention_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(apply.decision, "pass");
    let execution = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
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
    fs::write(&execution_path, to_text(&execution.value)?)?;
    let audit = retention::audit_retention_gc_execution(retention::RetentionGcAuditInput {
        root: &candidate.root,
        execution_ref: &execution.execution_ref,
    })?;
    assert_eq!(audit.decision, "pass");
    let audit_path = dir.join("catalog-retention-audit.preserves");
    fs::write(&audit_path, to_text(&audit.value)?)?;
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
        kind: retention::ADMISSION_KIND_POLICY,
        label: "policy",
    })?;
    candidate.authority_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: retention::ADMISSION_KIND_AUTHORITY,
        label: "authority",
    })?;
    candidate.support_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
        label: "support",
    })?;
    candidate.index_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: retention::ADMISSION_KIND_REFERENCE_INDEX,
        label: "index",
    })?;
    Ok(candidate)
}

fn store_retention_cli_admission(input: RetentionAdmissionInput<'_>) -> CliResult<String> {
    Ok(retention::store_retention_evidence_admission(
        &input.candidate.root,
        &retention::RetentionEvidenceAdmissionInput {
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
    out: &Path,
) -> CliResult<retention::RetentionGcPlan> {
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
    let plan = retention::parse_retention_gc_plan(&read_preserves(out)?)?;
    assert_eq!(plan.decision, "pass");
    Ok(plan)
}

fn add_retention_args(command: &mut Command, candidate: &RetentionCliCandidate) {
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

fn write_octet_artifacts(dir: &Path) -> CliResult<()> {
    write_octet_artifacts_with(dir, OCTET_WARNING_STATUS, OCTET_WARNING_SUMMARY)
}

fn write_octet_artifacts_with(dir: &Path, status: impl AsRef<str>, summary: &str) -> CliResult<()> {
    fs::write(dir.join("command.txt"), "cargo octet check --artifact-dir target/octet\n")?;
    fs::write(dir.join("status.json"), status.as_ref())?;
    fs::write(dir.join("summary.txt"), summary)?;
    fs::write(dir.join("object-corpus-receipt.json"), OCTET_OBJECT_CORPUS)?;
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

fn file_hash(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| format!("b3:{}", blake3::hash(&bytes).to_hex()))
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

const OCTET_OBJECT_CORPUS: &str = r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":3,"source_paths":["src/job_dag.rs","src/main.rs","src/node_runtime.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":3}"#;

fn molten_cmd() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_molten"));
    command.current_dir(manifest_dir());
    command
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cleanup_stale_molten_temp_dirs() {
    static CLEAN_STALE_TEMP_DIRS: std::sync::Once = std::sync::Once::new();
    CLEAN_STALE_TEMP_DIRS.call_once(|| {
        let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
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
                    let remove_result = fs::remove_dir_all(entry.path());
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

fn temp_dir(label: &str) -> CliResult<PathBuf> {
    cleanup_stale_molten_temp_dirs();
    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn read_preserves(path: &Path) -> CliResult<preserves::IOValue> {
    Ok(parse_text(&fs::read_to_string(path)?)?)
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output, label: &str) {
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}
