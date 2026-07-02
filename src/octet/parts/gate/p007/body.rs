
fn parse_effective_command(
    command: &str,
    workspace_config: &WorkspaceOctetConfig,
) -> std::result::Result<EffectiveOctetCommand, String> {
    let mut tokens = Vec::with_capacity(MAX_OCTET_COMMAND_TOKENS.min(command.len()));
    for token in command.split_whitespace() {
        if tokens.len() >= MAX_OCTET_COMMAND_TOKENS {
            return Err(format!("octet command token count exceeds bound {MAX_OCTET_COMMAND_TOKENS}: {command}"));
        }
        tokens.push(token.to_string());
    }
    if tokens.len() < 3 || tokens[0] != "cargo" || tokens[1] != "octet" || tokens[2] != "check" {
        return Err(format!("cannot derive octet metadata from noncanonical command: {command}"));
    }
    let mut scope_args = Vec::new();
    let mut cargo_check_args = None;
    let mut output_format = "human".to_string();
    let mut index = 3;
    while index < tokens.len() {
        let token = &tokens[index];
        if token == "--" {
            let args_len = tokens.len().saturating_sub(index + 1);
            if args_len > MAX_OCTET_COMMAND_TOKENS {
                return Err("octet cargo-check argument count exceeds command token bound".to_string());
            }
            cargo_check_args = Some(tokens[index + 1..].to_vec());
            break;
        }
        if token == "--output-format" {
            let Some(value) = tokens.get(index + 1) else {
                return Err("missing value after --output-format in octet command".to_string());
            };
            output_format = value.clone();
            index += 2;
            continue;
        }
        if let Some(value) = token.strip_prefix("--output-format=") {
            output_format = value.to_string();
            index += 1;
            continue;
        }
        if option_takes_value(token) {
            if tokens.get(index + 1).is_none() {
                return Err(format!("missing value after {token} in octet command"));
            }
            index += 2;
            continue;
        }
        if option_with_inline_value(token) || token == "--cache" {
            index += 1;
            continue;
        }
        push_token_bounded(&mut scope_args, token.clone())?;
        index += 1;
    }
    let scope_args = if scope_args.is_empty() {
        workspace_config.default_scope.clone()
    } else {
        scope_args
    };
    let cargo_check_args = cargo_check_args.unwrap_or_else(|| workspace_config.cargo_check_args.clone());
    Ok(EffectiveOctetCommand {
        scope_args,
        cargo_check_args,
        output_format,
    })
}

fn option_takes_value(token: &str) -> bool {
    matches!(token, "--artifact-dir" | "--baseline" | "--write-baseline")
}

fn option_with_inline_value(token: &str) -> bool {
    token.starts_with("--artifact-dir=") || token.starts_with("--baseline=") || token.starts_with("--write-baseline=")
}

fn load_workspace_octet_config(workspace_root: &Path) -> std::result::Result<WorkspaceOctetConfig, String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let source =
        fs::read_to_string(&manifest_path).map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let document = source
        .parse::<toml::Table>()
        .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
    let octet = document
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("metadata"))
        .and_then(toml::Value::as_table)
        .and_then(|metadata| metadata.get("octet"))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing [workspace.metadata.octet] in {}", manifest_path.display()))?;
    Ok(WorkspaceOctetConfig {
        default_scope: string_array_field(octet, "default_scope", &manifest_path)?,
        cargo_check_args: string_array_field(octet, "cargo_check_args", &manifest_path)?,
    })
}

fn string_array_field(
    table: &toml::Table,
    key: &str,
    manifest_path: &Path,
) -> std::result::Result<Vec<String>, String> {
    let values = table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("missing `{key}` array in {}", manifest_path.display()))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("`{key}` must contain only strings in {}", manifest_path.display()))
        })
        .collect()
}

fn current_config_hash(workspace_root: &Path, scope_args: &[String], cargo_check_args: &[String]) -> String {
    let files = vec![
        file_hash_entry(workspace_root.join("Cargo.toml")),
        file_hash_entry(workspace_root.join("dylint.toml")),
    ];
    let payload = serde_json::json!({
        "files": files,
        "effective_scope_args": scope_args,
        "effective_cargo_check_args": cargo_check_args,
    });
    b3_full_hash(&payload.to_string())
}

fn current_profile_hash(
    scope_args: &[String],
    cargo_check_args: &[String],
    output_format: &str,
    config_hash: &str,
) -> String {
    let payload = serde_json::json!({
        "scope_args": scope_args,
        "cargo_check_args": cargo_check_args,
        "output_format": output_format,
        "config_hash": config_hash,
    });
    b3_full_hash(&payload.to_string())
}

fn file_hash_entry(path: PathBuf) -> serde_json::Value {
    let relative = path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");
    serde_json::json!({
        "path": relative,
        "hash": fs::read(&path).ok().and_then(|bytes| b3_ref_from_bytes(&bytes).ok()),
    })
}

fn b3_full_hash(input: &str) -> String {
    format!("b3:{}", blake3::hash(input.as_bytes()).to_hex())
}

enum DeltaKind {
    NewOrIncreased,
    Removed,
}

struct OctetBaselineReceiptInput<'a> {
    decision: &'a str,
    baseline_ref: &'a str,
    status_ref: &'a str,
    new_findings: &'a [FindingEntry],
    removed_findings: &'a [FindingEntry],
    unchanged_findings: &'a [FindingEntry],
    critical_unreviewed: &'a [FindingEntry],
    review_refs: &'a [String],
    expired: bool,
    diagnostics: &'a [String],
    checks: &'a [Check],
}

struct OctetWarningBaselineValueInput<'a> {
    run: &'a CurrentOctetRun,
    created_at: &'a str,
    expires_at: &'a str,
    target_next: u64,
    source_snapshot_ref: &'a str,
    checks: &'a [Check],
}

fn load_current_octet_run(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<CurrentOctetRun> {
    let command =
        read_required_file(artifacts_dir, COMMAND_NAME, "baseline-command-artifact-present", checks, diagnostics);
    let status_file =
        read_required_file(artifacts_dir, STATUS_NAME, "baseline-status-artifact-present", checks, diagnostics);
    let summary =
        read_required_file(artifacts_dir, SUMMARY_NAME, "baseline-summary-artifact-present", checks, diagnostics);
    let object_corpus = read_required_file(
        artifacts_dir,
        OBJECT_CORPUS_RECEIPT_NAME,
        "baseline-object-corpus-artifact-present",
        checks,
        diagnostics,
    );
    let Some(status_file) = status_file else {
        return Err(MoltenError::invalid_harness("octet baseline requires status.json"));
    };
    let Some(summary) = summary else {
        return Err(MoltenError::invalid_harness("octet baseline requires summary.txt"));
    };
    let Some(object_corpus) = object_corpus else {
        return Err(MoltenError::invalid_harness("octet baseline requires object-corpus-receipt.json"));
    };
    let Some(status) = parse_status(Some(&status_file), checks, diagnostics) else {
        return Err(MoltenError::invalid_harness("octet baseline requires parseable status.json"));
    };
    validate_command(command.as_ref(), checks, diagnostics);
    validate_metadata_binding(command.as_ref(), Some(&status), checks, diagnostics);
    let has_valid_object_corpus = validate_object_corpus(Some(&object_corpus), checks, diagnostics).is_some();
    if !has_valid_object_corpus {
        return Err(MoltenError::invalid_harness("octet baseline requires valid object corpus receipt"));
    }
    let (findings, parsed_count) = parse_summary_findings(&summary, &status);
    let unkeyed_findings = status.total_findings.saturating_sub(parsed_count);
    push_check(checks, "baseline-findings-keyed", unkeyed_findings == 0);
    if unkeyed_findings > 0 {
        push_diagnostic(diagnostics, format!("summary omitted stable keys for {unkeyed_findings} findings"));
    }
    Ok(CurrentOctetRun {
        status_ref: status_file.artifact_ref,
        summary_ref: summary.artifact_ref,
        object_corpus_ref: object_corpus.artifact_ref,
        status,
        findings,
        unkeyed_findings,
    })
}

fn octet_warning_baseline_value(input: &OctetWarningBaselineValueInput<'_>) -> IoValue {
    let critical_keys = critical_keys(&input.run.findings);
    record("octet-warning-baseline-v1", vec![
        string(OCTET_WARNING_BASELINE_SCHEMA),
        record("scope", vec![string("workspace")]),
        record("created-at", vec![string(input.created_at)]),
        record("expires-at", vec![string(input.expires_at)]),
        record("octet-config-hash", vec![string(&input.run.status.metadata.config_hash)]),
        record("octet-profile-hash", vec![string(&input.run.status.metadata.profile_hash)]),
        record("toolchain", vec![string(&input.run.status.metadata.toolchain)]),
        record("source-snapshot", vec![string(input.source_snapshot_ref)]),
        record("finding-keys", vec![sequence(input.run.findings.values().map(finding_entry_value).collect())]),
        record("critical-finding-keys", vec![sequence(critical_keys.iter().map(string).collect())]),
        record("allowed-profiles", vec![sequence(vec![string(QUARANTINE_PROFILE)])]),
        record("burn-down", vec![
            record("total", vec![u64_value(input.run.status.total_findings)]),
            record("target-next", vec![u64_value(input.target_next)]),
            record("deadline", vec![string(input.expires_at)]),
        ]),
        record("review-refs", vec![sequence(Vec::new())]),
        checks_value(input.checks),
    ])
}

fn octet_baseline_receipt_value(input: OctetBaselineReceiptInput<'_>) -> IoValue {
    record("octet-baseline-receipt-v1", vec![
        string(crate::preserves_rail::OCTET_BASELINE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("baseline", vec![string(input.baseline_ref)]),
        record("run-status", vec![string(input.status_ref)]),
        record("new-findings", vec![sequence(input.new_findings.iter().map(finding_entry_value).collect())]),
        record("removed-findings", vec![sequence(
            input.removed_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("unchanged-findings", vec![sequence(
            input.unchanged_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("critical-unreviewed", vec![sequence(
            input.critical_unreviewed.iter().map(finding_entry_value).collect(),
        )]),
        record("review-refs", vec![sequence(input.review_refs.iter().map(string).collect())]),
        record("expired", vec![bool_value(input.expired)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn parse_review_manifests(values: &[IoValue]) -> Result<Vec<ParsedReviewManifest>> {
    values.iter().map(parse_review_manifest).collect()
}
