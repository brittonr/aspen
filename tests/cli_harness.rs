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
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

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
