
fn parse_status(
    status_file: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<StatusArtifact> {
    let Some(status_file) = status_file else {
        push_check(checks, "status-json-parse", false);
        return None;
    };
    match serde_json::from_str::<StatusArtifact>(&status_file.text) {
        Ok(status) => {
            let has_complete_metadata = status.metadata.tool_name == "cargo-octet"
                && !status.metadata.tool_version.is_empty()
                && !status.metadata.rustc_version.is_empty()
                && !status.metadata.toolchain.is_empty()
                && !status.metadata.config_hash.is_empty()
                && !status.metadata.profile_hash.is_empty();
            let has_consistent_exit_status =
                status.exit_code == 0 || status.status == "lint-failure" || status.status == "integration-failure";
            let is_tool_version_supported = status.metadata.tool_version == SUPPORTED_OCTET_TOOL_VERSION;
            push_check(checks, "status-json-parse", true);
            push_check(checks, "status-metadata-complete", has_complete_metadata);
            push_check(checks, "status-tool-version-supported", is_tool_version_supported);
            push_check(checks, "status-exit-consistent", has_consistent_exit_status);
            if !has_complete_metadata {
                push_diagnostic(diagnostics, "status metadata missing required cargo-octet fields".to_string());
            }
            if !is_tool_version_supported {
                push_diagnostic(
                    diagnostics,
                    format!("unsupported cargo-octet version `{}`", status.metadata.tool_version),
                );
            }
            if !has_consistent_exit_status {
                push_diagnostic(
                    diagnostics,
                    format!("status exit_code {} is inconsistent with status `{}`", status.exit_code, status.status),
                );
            }
            Some(status)
        }
        Err(error) => {
            push_check(checks, "status-json-parse", false);
            push_diagnostic(diagnostics, format!("malformed status.json: {error}"));
            None
        }
    }
}

fn parse_summary_lints(
    summary: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> OrderedMap<String, u64> {
    let Some(summary) = summary else {
        push_check(checks, "summary-lints-parse", false);
        return OrderedMap::new();
    };
    let mut lints = OrderedMap::new();
    let mut is_parsing_lints = false;
    for line in summary.text.lines() {
        let trimmed = line.trim();
        if trimmed == "By lint:" {
            is_parsing_lints = true;
            continue;
        }
        if is_parsing_lints && (trimmed.is_empty() || trimmed == "Index:") {
            break;
        }
        if !is_parsing_lints {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(count) = parts.next() else { continue };
        let Ok(count) = count.parse::<u64>() else { continue };
        if insert_bounded(&mut lints, name.to_string(), count, MAX_OCTET_SUMMARY_LINTS, "summary lint counts").is_err()
        {
            push_diagnostic(diagnostics, "summary lint count exceeds configured bound".to_string());
            break;
        }
    }
    push_check(checks, "summary-lints-parse", true);
    if lints.is_empty() && summary.text.contains("Findings:") && !summary.text.contains("Findings: 0") {
        push_diagnostic(diagnostics, "summary contains findings but no parseable lint counts".to_string());
    }
    lints
}

fn validate_object_corpus(
    object_corpus: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<ObjectCorpusReceipt> {
    let Some(object_corpus) = object_corpus else {
        push_check(checks, "object-corpus-json-parse", false);
        push_check(checks, "object-corpus-schema", false);
        push_check(checks, "object-corpus-nonempty", false);
        push_check(checks, "object-corpus-fingerprint", false);
        push_check(checks, "object-corpus-critical-paths", false);
        push_check(checks, SOURCE_SCOPE_OBJECT_CORPUS_CHECK, false);
        return None;
    };
    let parsed = serde_json::from_str::<ObjectCorpusReceipt>(&object_corpus.text);
    let Ok(receipt) = parsed else {
        push_check(checks, "object-corpus-json-parse", false);
        push_check(checks, "object-corpus-schema", false);
        push_check(checks, "object-corpus-nonempty", false);
        push_check(checks, "object-corpus-fingerprint", false);
        push_check(checks, "object-corpus-critical-paths", false);
        push_check(checks, SOURCE_SCOPE_OBJECT_CORPUS_CHECK, false);
        push_diagnostic(diagnostics, "malformed object corpus receipt JSON".to_string());
        return None;
    };
    let has_supported_schema =
        receipt.schema.as_deref() == Some(OCTET_OBJECT_CORPUS_SCHEMA) && receipt.schema_version == Some(1);
    let has_objects = receipt.object_count.is_some_and(|count| count > 0);
    let has_object_set_fingerprint = receipt.object_set_hash.as_deref().is_some_and(is_b3_ref);
    let coverage_paths = object_corpus_coverage_paths(&receipt);
    let has_required_source_paths = coverage_paths.as_ref().is_some_and(|paths| {
        REQUIRED_OBJECT_CORPUS_SOURCE_PATHS.iter().all(|required| paths.iter().any(|path| path == required))
    });
    let has_source_scope_paths = coverage_paths.as_ref().is_some_and(|paths| {
        SOURCE_GATE_SOURCE_SCOPE_PATHS.iter().all(|required| paths.iter().any(|path| path == required))
    });
    push_check(checks, "object-corpus-json-parse", true);
    push_check(checks, "object-corpus-schema", has_supported_schema);
    push_check(checks, "object-corpus-nonempty", has_objects);
    push_check(checks, "object-corpus-fingerprint", has_object_set_fingerprint);
    push_check(checks, "object-corpus-critical-paths", has_required_source_paths);
    push_check(checks, SOURCE_SCOPE_OBJECT_CORPUS_CHECK, has_source_scope_paths);
    if !has_supported_schema {
        push_diagnostic(diagnostics, "object corpus receipt has missing or unsupported schema".to_string());
    }
    if !has_objects {
        push_diagnostic(diagnostics, "object corpus receipt has no focused objects".to_string());
    }
    if !has_object_set_fingerprint {
        push_diagnostic(diagnostics, "object corpus receipt is missing object_set_hash fingerprint".to_string());
    }
    if !has_required_source_paths {
        push_diagnostic(diagnostics, "object corpus receipt does not cover required critical paths".to_string());
    }
    if !has_source_scope_paths {
        push_diagnostic(diagnostics, "object corpus receipt does not cover required source-gate scope paths".to_string());
    }
    if has_supported_schema
        && has_objects
        && has_object_set_fingerprint
        && has_required_source_paths
        && has_source_scope_paths
    {
        Some(receipt)
    } else {
        None
    }
}

fn octet_fingerprint_evidence_value(object_corpus: &GateFile, receipt: &ObjectCorpusReceipt) -> Result<IoValue> {
    let object_set_hash = receipt
        .object_set_hash
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing object_set_hash"))?;
    let source_paths = object_corpus_coverage_paths(receipt)
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing source_paths"))?;
    let object_count = receipt
        .object_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing object_count"))?;
    let pure_cache_blocked = receipt.pure_cache_blocked_count.unwrap_or(0);
    Ok(record("octet-fingerprint-evidence-v1", vec![
        string(crate::preserves_rail::OCTET_FINGERPRINT_EVIDENCE_SCHEMA),
        record("object-corpus", vec![string(&object_corpus.artifact_ref)]),
        record("object-set-hash", vec![string(object_set_hash)]),
        record("source-paths", vec![sequence(source_paths.iter().map(string).collect())]),
        record("object-count", vec![u64_value(object_count)]),
        record("pure-cache-blocked", vec![u64_value(pure_cache_blocked)]),
        checks_value(&[
            Check {
                name: "object-corpus-fingerprint",
                status: "pass",
            },
            Check {
                name: "critical-path-coverage",
                status: "pass",
            },
        ]),
    ]))
}

fn object_corpus_coverage_paths(receipt: &ObjectCorpusReceipt) -> Option<Vec<String>> {
    let mut paths = receipt.source_paths.clone()?;
    if let Some(command) = receipt.replay.as_ref().and_then(|replay| replay.command.as_deref()) {
        for token in command.split_whitespace().filter(|token| token.ends_with(RUST_SOURCE_EXTENSION)) {
            paths.push(token.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    Some(paths)
}

fn is_b3_ref(value: &str) -> bool {
    is_prefixed_blake3_hex_ref(value, B3_REF_PREFIX)
}

fn validate_command(
    command: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let Some(command) = command else {
        push_check(checks, "command-shape", false);
        return false;
    };
    let normalized = command.text.trim();
    let has_canonical_command_shape =
        normalized.starts_with("cargo octet check") && normalized.contains("--artifact-dir");
    push_check(checks, "command-shape", has_canonical_command_shape);
    if !has_canonical_command_shape {
        push_diagnostic(diagnostics, format!("noncanonical octet command: {normalized}"));
    }
    has_canonical_command_shape
}

fn validate_metadata_binding(
    command: Option<&GateFile>,
    status: Option<&StatusArtifact>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let Some(command) = command else {
        push_check(checks, "status-config-current", false);
        push_check(checks, "status-profile-current", false);
        return false;
    };
    let Some(status) = status else {
        push_check(checks, "status-config-current", false);
        push_check(checks, "status-profile-current", false);
        return false;
    };
    let expected = match expected_metadata_for_command(command.text.trim()) {
        Ok(expected) => expected,
        Err(message) => {
            push_check(checks, "status-config-current", false);
            push_check(checks, "status-profile-current", false);
            push_diagnostic(diagnostics, message);
            return false;
        }
    };
    let is_status_config_current = status.metadata.config_hash == expected.config_hash;
    let is_status_profile_current = status.metadata.profile_hash == expected.profile_hash;
    push_check(checks, "status-config-current", is_status_config_current);
    push_check(checks, "status-profile-current", is_status_profile_current);
    if !is_status_config_current {
        push_diagnostic(
            diagnostics,
            format!("stale octet config hash: status={} current={}", status.metadata.config_hash, expected.config_hash),
        );
    }
    if !is_status_profile_current {
        push_diagnostic(
            diagnostics,
            format!(
                "stale octet profile hash: status={} current={}",
                status.metadata.profile_hash, expected.profile_hash
            ),
        );
    }
    is_status_config_current && is_status_profile_current
}

fn expected_metadata_for_command(command: &str) -> std::result::Result<ExpectedMetadata, String> {
    let workspace_root = std::env::current_dir().map_err(|error| format!("current_dir: {error}"))?;
    let workspace_config = load_workspace_octet_config(&workspace_root)?;
    let effective = parse_effective_command(command, &workspace_config)?;
    let config_hash = current_config_hash(&workspace_root, &effective.scope_args, &effective.cargo_check_args)?;
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
