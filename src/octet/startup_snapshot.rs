//! Pure re-evaluation of a measured portable snapshot. No cwd, filesystem, or process access.
//! Success is verification-only; it is not a source execution attestation or startup capability.
use super::*;
use molten_core::node_startup::{EvidencePlan, SourceFile};

pub struct Snapshot<'a> {
    pub plan: &'a EvidencePlan,
    /// Measured member bytes in the exact role order prescribed by EvidencePlan.
    pub members: &'a [Vec<u8>],
}

// r[impl molten.startup_evidence.strict]
pub fn evaluate(snapshot: Snapshot<'_>) -> Result<OctetGateEvaluation> {
    if snapshot.members.len() != snapshot.plan.members().len() {
        return Err(MoltenError::invalid_harness("startup-evidence-member-inventory"));
    }
    for (index, bytes) in snapshot.members.iter().enumerate() {
        snapshot
            .plan
            .verify_member(index, bytes)
            .map_err(|_| MoltenError::invalid_harness("startup-evidence-member-identity"))?;
    }
    let source_files: Vec<SourceFile> = serde_json::from_slice(&snapshot.members[5])
        .map_err(|_| MoltenError::invalid_harness("startup-evidence-source-inventory"))?;
    molten_core::node_startup::validate_source_inventory(snapshot.plan, &source_files)
        .map_err(|_| MoltenError::invalid_harness("startup-evidence-source-context"))?;
    let text = |index: usize| -> Result<&str> {
        std::str::from_utf8(&snapshot.members[index]).map_err(|_| MoltenError::invalid_harness("startup-evidence-utf8"))
    };
    let command = text(6)?;
    if command.trim() != DEFAULT_GATE_COMMAND {
        return Err(MoltenError::invalid_harness("startup-evidence-command-profile"));
    }
    let expected =
        explicit_metadata(text(0)?, &snapshot.members[1], command.trim()).map_err(MoltenError::invalid_harness)?;
    let status: StatusArtifact =
        serde_json::from_str(text(7)?).map_err(|_| MoltenError::invalid_harness("startup-evidence-status"))?;
    if status.metadata.toolchain != snapshot.plan.cohort().octet_toolchain
        || status.exit_code != 0
        || status.total_findings != 0
        || status.warning_findings != 0
        || status.error_findings != 0
        || status.autofixable_findings != 0
    {
        return Err(MoltenError::invalid_harness("startup-evidence-not-strict-clean"));
    }
    let summary = text(8)?;
    let mut totals = summary.lines().map(str::trim).filter(|line| line.starts_with("Findings:"));
    if totals.next() != Some("Findings: 0") || totals.next().is_some() {
        return Err(MoltenError::invalid_harness("startup-evidence-summary-count"));
    }
    let corpus: ObjectCorpusReceipt =
        serde_json::from_str(text(9)?).map_err(|_| MoltenError::invalid_harness("startup-evidence-object-corpus"))?;
    // A replay command naming a file is not source coverage. Require actual corpus paths.
    let paths = corpus
        .source_paths
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("startup-evidence-source-coverage"))?;
    if paths.len() > molten_core::node_startup::MAX_SOURCE_FILES
        || paths.iter().any(|path| source_files.binary_search_by(|file| file.name.cmp(path)).is_err())
        || SOURCE_GATE_SOURCE_SCOPE_PATHS.iter().any(|required| !paths.iter().any(|p| p == required))
        || [
            "src/node/content.rs",
            "src/node/startup_evidence.rs",
            "src/octet/startup_snapshot.rs",
            "crates/molten-core/src/node_startup.rs",
            "crates/molten-core/src/content_store_adapter/node_service.rs",
            "src/node/parts/daemon/p018/body.rs",
            "src/node/parts/daemon/p019/body.rs",
        ]
        .iter()
        .any(|required| !paths.iter().any(|p| p == required))
    {
        return Err(MoltenError::invalid_harness("startup-evidence-source-coverage"));
    }
    let build_config: toml::Table =
        text(4)?.parse().map_err(|_| MoltenError::invalid_harness("startup-evidence-build-toolchain"))?;
    if build_config.get("toolchain").and_then(|v| v.get("channel")).and_then(toml::Value::as_str)
        != Some(snapshot.plan.cohort().build_toolchain.as_str())
    {
        return Err(MoltenError::invalid_harness("startup-evidence-build-toolchain"));
    }
    let gate_file = |index: usize| -> Result<GateFile> {
        Ok(GateFile {
            artifact_ref: content_ref_from_bytes(snapshot.members[index].as_slice()),
            text: text(index)?.into(),
        })
    };
    let files = InputFiles {
        command: Some(gate_file(6)?),
        status_file: Some(gate_file(7)?),
        summary: Some(gate_file(8)?),
        object_corpus: Some(gate_file(9)?),
    };
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let lints = parse_summary_lints(files.summary.as_ref(), &mut checks, &mut diagnostics);
    if lints.values().any(|count| *count != 0) || !diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness("startup-evidence-summary-count"));
    }
    let mut initial_checks = vec![
        Check {
            name: "artifacts-dir-present",
            status: "pass",
        },
        Check {
            name: "profile-supported",
            status: "pass",
        },
    ];
    for name in [
        "command-artifact-present",
        "status-artifact-present",
        "summary-artifact-present",
        "object-corpus-artifact-present",
    ] {
        initial_checks.push(Check { name, status: "pass" });
    }
    evaluate_octet_gate_files(
        &OctetGateInput {
            artifacts_dir: PathBuf::from("target/octet"),
            profile: STRICT_PROFILE.into(),
        },
        files,
        initial_checks,
        Vec::new(),
        Ok(expected),
    )
}

fn explicit_metadata(manifest: &str, dylint: &[u8], command: &str) -> std::result::Result<ExpectedMetadata, String> {
    let config = parse_workspace_octet_config(manifest, Path::new("Cargo.toml"))?;
    let lint_text = std::str::from_utf8(dylint).map_err(|_| "startup-evidence-dylint-utf8")?;
    let lint_config: toml::Table = lint_text.parse().map_err(|_| "startup-evidence-dylint-config")?;
    let octet = lint_config.get("octet").and_then(toml::Value::as_table).ok_or("startup-evidence-dylint-config")?;
    if lint_config.len() != 1
        || octet.len() != 1
        || !octet.get("disabled_lints").and_then(toml::Value::as_array).is_some_and(Vec::is_empty)
    {
        return Err("startup-evidence-dylint-suppression".into());
    }
    let effective = parse_effective_command(command, &config)?;
    if effective.scope_args != ["-p", "molten", "-p", "molten-node-host"]
        || effective.cargo_check_args != ["--all-targets"]
        || effective.output_format != "human"
    {
        return Err("startup-evidence-workspace-scope".into());
    }
    let files = [
        OctetFileHashEntry {
            hash: Some(b3_ref_from_bytes(manifest.as_bytes()).map_err(|error| error.to_string())?),
            path: "Cargo.toml".into(),
        },
        OctetFileHashEntry {
            hash: Some(b3_ref_from_bytes(dylint).map_err(|error| error.to_string())?),
            path: "dylint.toml".into(),
        },
    ];
    let config_hash =
        b3_full_hash(&octet_config_hash_payload(&files, &effective.scope_args, &effective.cargo_check_args)?);
    let profile_hash = current_profile_hash(
        &effective.scope_args,
        &effective.cargo_check_args,
        &effective.output_format,
        &config_hash,
    )?;
    Ok(ExpectedMetadata {
        config_hash,
        profile_hash,
    })
}

#[cfg(test)]
pub(crate) mod tests;
