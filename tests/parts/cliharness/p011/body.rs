
fn store_retention_cli_admission(input: RetentionAdmissionInput<'_>) -> CliResult<String> {
    Ok(molten::retention::store_evidence_admission(
        &input.candidate.root,
        &molten::retention::EvidenceAdmissionInput {
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

fn run_gc_plan_cli(
    candidate: &RetentionCliCandidate,
    subsystem: &str,
    out: &std::path::Path,
) -> CliResult<molten::retention::GcPlan> {
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
    let plan = molten::retention::parse_gc_plan(&read_preserves(out)?)?;
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

const OCTET_OBJECT_CORPUS: &str = r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":6,"source_paths":["src/job/dag.rs","src/main.rs","src/node/daemon.rs","src/node/runtime.rs","src/octet/gate.rs","src/upgrades/mod.rs"],"object_set_hash":"b3:0000000000000000000000000000000000000000000000000000000000000000","pure_cache_blocked_count":6}"#;

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
