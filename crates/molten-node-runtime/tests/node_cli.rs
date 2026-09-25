#![cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use molten_node_runtime::{content_store_adapter, ledger, node_runtime, preserves_rail};

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn init_writes_config_but_run_denies_before_startup_effects() -> CliResult<()> {
    let (_guard, dir) = temp_dir()?;
    let state_root = dir.join("state");
    let config = dir.join("node-config.preserves");
    let startup = dir.join("node-startup.preserves");

    let init = molten_node_cmd()
        .args(["init", "--state-root"])
        .arg(&state_root)
        .args(["--node-id", "node:cli", "--config-out"])
        .arg(&config)
        .output()?;
    assert_success(&init, "node init");
    assert_eq!(ledger::artifact_kind(&read_preserves(&config)?), "node-config");
    assert!(state_root.join("config.preserves").is_file());
    assert!(state_root.join("identity-receipt.preserves").is_file());
    let state_before = snapshot_node_state(&state_root)?;

    let run = molten_node_cmd()
        .args(["run", "--state-root"])
        .arg(&state_root)
        .args(["--startup-out"])
        .arg(&startup)
        .output()?;
    assert_failure(&run, "node run without real startup evidence");
    assert!(stderr(&run).contains("node-startup-source-gate-required"));
    assert!(run.stdout.is_empty());
    assert!(!startup.exists());
    assert!(!state_root.join("startup-receipt.preserves").exists());
    assert_eq!(snapshot_node_state(&state_root)?, state_before);
    Ok(())
}

#[test]
fn profile_backed_init_preserves_metadata_when_run_denies() -> CliResult<()> {
    let (_guard, dir) = temp_dir()?;
    let state_root = dir.join("state");
    let config = dir.join("node-config.preserves");
    let startup = dir.join("node-startup.preserves");
    let profile_resolution = dir.join("node-profile-resolution.preserves");
    let profile_ref = test_ref("checked-node-profile")?;
    let profile_state_root_ref = test_ref("profile-state-root")?;
    let policy_ref = test_ref("profile-policy")?;
    let capability_ref = test_ref("profile-capability")?;
    let resource_ref = test_ref("profile-resource")?;
    let effect_ref = test_ref("profile-effect")?;
    let mut init = molten_node_cmd();
    init.args(["init", "--state-root"])
        .arg(&state_root)
        .args(["--node-id", "node:profile", "--config-out"])
        .arg(&config)
        .args(["--profile-resolution-out"])
        .arg(&profile_resolution)
        .args(["--profile-ref", &profile_ref, "--actual-profile-ref", &profile_ref])
        .args(["--profile-tier", "pilot", "--profile-identity", "pilot-node"])
        .args(["--profile-state-root-ref", &profile_state_root_ref])
        .args(["--policy-ref", &policy_ref, "--capability-ref", &capability_ref])
        .args(["--resource-ref", &resource_ref, "--effect-profile-ref", &effect_ref]);
    for adapter in node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let adapter_ref = test_ref(&format!("adapter-{adapter}"))?;
        init.arg("--adapter-profile").arg(format!("{adapter}={adapter_ref}"));
    }
    let output = init.output()?;
    assert_success(&output, "node profile-backed init");
    assert!(stdout(&output).contains("profile_resolution="));
    assert_eq!(ledger::artifact_kind(&read_preserves(&config)?), "node-config");
    let resolution_value = read_preserves(&profile_resolution)?;
    let resolution_bytes = std::fs::read(&profile_resolution)?;
    assert!(preserves_rail::to_text(&resolution_value)?.contains("node-profile-config-resolution-v1"));
    let state_before = snapshot_node_state(&state_root)?;

    let run = molten_node_cmd()
        .args(["run", "--state-root"])
        .arg(&state_root)
        .args(["--startup-out"])
        .arg(&startup)
        .output()?;
    assert_failure(&run, "node profile-backed run without real startup evidence");
    assert!(stderr(&run).contains("node-startup-source-gate-required"));
    assert!(run.stdout.is_empty());
    assert!(!startup.exists());
    assert!(!state_root.join("startup-receipt.preserves").exists());
    assert_eq!(ledger::artifact_kind(&read_preserves(&config)?), "node-config");
    assert_eq!(std::fs::read(&profile_resolution)?, resolution_bytes);
    assert_eq!(snapshot_node_state(&state_root)?, state_before);
    Ok(())
}

#[test]
fn control_request_and_deny_receipt_work_offline() -> CliResult<()> {
    let (_guard, dir) = temp_dir()?;
    let request = dir.join("node-control-request.preserves");
    let receipt = dir.join("node-control-receipt.preserves");
    let provenance = dir.join("node-control-provenance.preserves");
    let payload_ref = test_ref("node-control-payload")?;
    let startup_ref = test_ref("node-startup")?;

    let provenance_out = molten_node_cmd()
        .args(["provenance-fixture", "--artifact-ref"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&provenance)
        .output()?;
    assert_success(&provenance_out, "node provenance fixture");
    assert_eq!(ledger::artifact_kind(&read_preserves(&provenance)?), "provenance-record");

    let request_out = molten_node_cmd()
        .args(["control-request", "--operation", "gate", "--payload"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&request)
        .output()?;
    assert_success(&request_out, "node control request");
    assert_eq!(ledger::artifact_kind(&read_preserves(&request)?), "node-control-request");

    let deny = molten_node_cmd()
        .args(["control-deny"])
        .arg(&request)
        .args(["--startup"])
        .arg(&startup_ref)
        .args(["--diagnostic", "missing authority/resource", "--receipt-out"])
        .arg(&receipt)
        .output()?;
    assert_success(&deny, "node control deny");
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(ledger::artifact_kind(&receipt_value), "node-control-receipt");
    let text = preserves_rail::to_text(&receipt_value)?;
    assert!(text.contains("missing authority/resource"));
    Ok(())
}

#[test]
fn content_serve_denies_without_real_startup_evidence_before_effects() -> CliResult<()> {
    let (_guard, dir) = temp_dir()?;
    let state_root = dir.join("node-state");
    let content_config = dir.join("content-config.json");
    let service_receipt = dir.join("service.preserves");
    let control_receipt = dir.join("control.preserves");
    let config = serde_json::json!({
        "schema": content_store_adapter::NODE_CONTENT_SCHEMA,
        "manifest_ref": format!("blake3:{}", "a".repeat(64)),
        "readers": ["b".repeat(64)],
        "bind_addr": "192.0.2.1:17888",
        "tick_ms": 250
    });
    std::fs::write(&content_config, serde_json::to_vec(&config)?)?;

    let served = molten_node_cmd()
        .args(["serve", "--state-root"])
        .arg(&state_root)
        .args(["--content-config"])
        .arg(&content_config)
        .args(["--max-ticks", "1", "--service-receipt-out"])
        .arg(&service_receipt)
        .args(["--receipt-out"])
        .arg(&control_receipt)
        .output()?;
    assert_failure(&served, "node content serve without real startup evidence");
    assert!(stderr(&served).contains("node-startup-source-gate-required"));
    assert!(served.stdout.is_empty());
    assert!(!state_root.exists());
    assert!(!service_receipt.exists());
    assert!(!control_receipt.exists());
    Ok(())
}

fn temp_dir() -> CliResult<(cap_tempfile::TempDir, PathBuf)> {
    let dir = cap_tempfile::tempdir(cap_tempfile::ambient_authority())?;
    let path = std::fs::read_link(format!("/proc/self/fd/{}", dir.as_raw_fd()))?;
    Ok((dir, path))
}

fn molten_node_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_molten-node"))
}

fn test_ref(label: &str) -> CliResult<String> {
    Ok(preserves_rail::canonical_hash(&preserves_rail::record("cli-test-ref", vec![
        preserves_rail::string(label),
    ]))?)
}

fn read_preserves(path: &Path) -> CliResult<preserves::IOValue> {
    Ok(preserves_rail::parse_text(&std::fs::read_to_string(path)?)?)
}

fn snapshot_node_state(root: &Path) -> CliResult<Vec<(PathBuf, Option<Vec<u8>>)>> {
    fn visit(root: &Path, dir: &Path, entries: &mut Vec<(PathBuf, Option<Vec<u8>>)>) -> CliResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(root)?.to_path_buf();
            if entry.file_type()?.is_dir() {
                entries.push((relative, None));
                visit(root, &path, entries)?;
            } else {
                entries.push((relative, Some(std::fs::read(path)?)));
            }
        }
        Ok(())
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

fn assert_success(output: &Output, label: &str) {
    assert!(output.status.success(), "{label} failed\nstdout:\n{}\nstderr:\n{}", stdout(output), stderr(output));
}

fn assert_failure(output: &Output, label: &str) {
    assert!(!output.status.success(), "{label} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}", stdout(output), stderr(output));
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
