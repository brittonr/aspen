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

fn octet_noncritical_status(total: u64) -> CliResult<String> {
    let (config_hash, profile_hash) = current_octet_hashes()?;
    Ok(format!(
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
    ))
}

#[derive(serde::Serialize)]
struct CliOctetConfigPayload<'a> {
    effective_cargo_check_args: &'a [&'a str],
    effective_scope_args: &'a [&'a str],
    files: &'a [CliOctetFileHashEntry<'a>],
}

#[derive(serde::Serialize)]
struct CliOctetFileHashEntry<'a> {
    hash: Option<String>,
    path: &'a str,
}

#[derive(serde::Serialize)]
struct CliOctetProfilePayload<'a> {
    cargo_check_args: &'a [&'a str],
    config_hash: &'a str,
    output_format: &'a str,
    scope_args: &'a [&'a str],
}

fn current_octet_hashes() -> CliResult<(String, String)> {
    let cargo_check_args = ["--all-targets"];
    let scope_args = ["-p", "molten", "-p", "molten-node-host"];
    let files = [
        CliOctetFileHashEntry {
            hash: file_hash(&manifest_dir().join("Cargo.toml")),
            path: "Cargo.toml",
        },
        CliOctetFileHashEntry {
            hash: file_hash(&manifest_dir().join("dylint.toml")),
            path: "dylint.toml",
        },
    ];
    let config_payload = serde_json::to_string(&CliOctetConfigPayload {
        effective_cargo_check_args: &cargo_check_args,
        effective_scope_args: &scope_args,
        files: &files,
    })?;
    let config_hash = b3_full_hash(&config_payload);
    let profile_payload = serde_json::to_string(&CliOctetProfilePayload {
        cargo_check_args: &cargo_check_args,
        config_hash: &config_hash,
        output_format: "human",
        scope_args: &scope_args,
    })?;
    let profile_hash = b3_full_hash(&profile_payload);
    Ok((config_hash, profile_hash))
}

fn file_hash(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| format!("b3:{}", blake3::hash(&bytes).to_hex()))
}

fn b3_full_hash(input: &str) -> String {
    format!("b3:{}", blake3::hash(input.as_bytes()).to_hex())
}

const OCTET_NONCRITICAL_SUMMARY_ONE: &str = "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n";

const OCTET_NONCRITICAL_SUMMARY_TWO: &str = "--- octet summary ---\nStatus: warning-only\nFindings: 2\nWarnings: 2\nErrors: 0\n\nBy lint:\n  function_length 1\n  bool_naming 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n  F2 bool_naming molten src/example.rs:20\n";

const OCTET_OBJECT_CORPUS: &str = r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":6,"source_paths":["src/job/dag.rs","src/main.rs","src/node/daemon.rs","src/node/runtime.rs","src/octet/gate.rs","src/upgrades/mod.rs"],"object_set_hash":"b3:0000000000000000000000000000000000000000000000000000000000000000","pure_cache_blocked_count":6}"#;
