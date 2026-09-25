
fn run_scan(input: ScanInput) -> Outcome<()> {
    let sources = collect_spec_sources(&input.root)?;
    let mut requirements = molten::requirement_traceability::requirements_from_sources(&sources)?;
    if input.changed_only {
        requirements.retain(|requirement| requirement.changed);
    }
    let raw_coverage = parse_coverage_inputs(&input.root, input.coverage, input.exemptions)?;
    let receipt_coverage = parse_receipt_inputs(&input.root, input.receipts)?;
    let coverage = molten::requirement_traceability::merge_coverage_inputs(
        raw_coverage.into_iter().chain(receipt_coverage).collect(),
    )?;
    let manifest = molten::requirement_traceability::build_traceability_manifest(
        &molten::requirement_traceability::TraceabilityInput {
            requirements,
            coverage,
            require_receipt_backed: input.require_receipt_backed,
        },
    )?;
    write_optional_preserves(input.out.as_ref(), &manifest.value)?;
    let summary = molten::requirement_traceability::render_summary(&manifest.summary)?;
    write_optional_text(input.summary_out.as_ref(), &summary)?;
    if let Some(readback_out) = input.readback_out.as_ref() {
        let readback = molten::requirement_traceability::build_proof_readback(&manifest)?;
        let rendered = molten::requirement_traceability::render_proof_readback(&readback)?;
        write_optional_text(Some(readback_out), &rendered)?;
    }
    println!(
        "traceability ref={} decision={} covered={} missing-negative={} stale={} compatibility-only={}",
        manifest.manifest_ref,
        manifest.decision,
        manifest.summary.covered.len(),
        manifest.summary.missing_negative.len(),
        manifest.summary.stale_reference.len(),
        manifest.summary.compatibility_only.len()
    );
    if manifest.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "requirement traceability denied: {}",
            summary.replace('\n', "; ")
        )))
    }
}

fn run_verification_run(input: VerificationRunCommandInput) -> Outcome<()> {
    let receipt = molten::requirement_traceability::build_verification_run_receipt(
        &molten::requirement_traceability::VerificationRunInput {
            requirement_id: input.requirement,
            coverage_kind: input.coverage_kind,
            target: input.target,
            argv: input.argv,
            profile_ref: input.profile_ref,
            toolchain_refs: input.toolchain_refs,
            exit_status: input.exit_status,
            stdout_ref: input.stdout_ref,
            stderr_ref: input.stderr_ref,
            artifact_refs: input.artifact_refs,
        },
    )?;
    write_optional_preserves(input.out.as_ref(), &receipt.value)?;
    eprintln!(
        "verification-run receipt={} decision={} requirement={} kind={}",
        receipt.receipt_ref, receipt.decision, receipt.requirement_id, receipt.coverage_kind
    );
    Ok(())
}

fn run_ci_run_receipt(input: CiRunReceiptCommandInput) -> Outcome<()> {
    let junit_text = std::fs::read_to_string(&input.junit).map_err(molten::error::MoltenError::from)?;
    let counts = parse_junit_counts(&junit_text)?;
    let receipt = molten::testing_hardening::build_ci_test_run_receipt(&molten::testing_hardening::CiTestRunInput {
        source_ref: molten::preserves_rail::content_ref_from_bytes(input.source_marker.as_bytes()),
        profile_id: input.profile_id,
        command_surface: input.command_surface,
        nextest_config_ref: raw_file_ref(&input.nextest_config)?,
        cargo_metadata_ref: raw_file_ref(&input.cargo_metadata)?,
        binaries_metadata_ref: raw_file_ref(&input.binaries_metadata)?,
        junit_ref: raw_file_ref(&input.junit)?,
        counts,
        decision: input.decision,
        diagnostics: Vec::new(),
        caveats: input.caveats,
    })?;
    write_optional_preserves(input.out.as_ref(), &receipt.value)?;
    eprintln!("ci-test-run receipt={} decision={}", receipt.receipt_ref, receipt.decision);
    Ok(())
}

fn run_nextest_profile_matrix(input: NextestProfileMatrixCommandInput) -> Outcome<()> {
    let config_text = std::fs::read_to_string(&input.nextest_config).map_err(molten::error::MoltenError::from)?;
    let config = parse_nextest_config(&config_text)?;
    let profiles = nextest_profiles_from_config(&config)?;
    let matrix = molten::testing_hardening::build_nextest_profile_matrix(
        &molten::testing_hardening::NextestProfileMatrixInput { profiles },
    )?;
    write_optional_preserves(input.out.as_ref(), &matrix.value)?;
    write_optional_text(input.summary_out.as_ref(), &render_nextest_profile_matrix_summary(&matrix))?;
    eprintln!("nextest-profile-matrix ref={} decision={}", matrix.matrix_ref, matrix.decision);
    if matrix.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nextest profile matrix denied: {}",
            matrix.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
        )))
    }
}

fn parse_nextest_config(text: &str) -> Outcome<toml::Value> {
    toml::from_str::<toml::Value>(text)
        .map_err(|error| molten::error::MoltenError::invalid_harness(format!("invalid nextest config TOML: {error}")))
}

fn nextest_profiles_from_config(config: &toml::Value) -> Outcome<Vec<molten::testing_hardening::SemanticProfileInput>> {
    let mut profiles = molten::testing_hardening::reviewed_nextest_profile_rows();
    for profile in &mut profiles {
        profile.filter_expression =
            nextest_profile_string(config, &profile.profile_id, NEXTEST_DEFAULT_FILTER_FIELD)?.unwrap_or_default();
        profile.retry_policy = nextest_retry_policy(config, &profile.profile_id)?;
        profile.expected_junit_path =
            nextest_profile_nested_string(config, &profile.profile_id, NEXTEST_JUNIT_TABLE, NEXTEST_JUNIT_PATH_FIELD)?
                .unwrap_or_default();
    }
    Ok(profiles)
}

fn nextest_retry_policy(config: &toml::Value, profile_id: &str) -> Outcome<String> {
    let retries = nextest_profile_integer(config, profile_id, NEXTEST_RETRIES_FIELD)?.unwrap_or(NEXTEST_ZERO_RETRIES);
    let flaky_result = nextest_profile_string(config, profile_id, NEXTEST_FLAKY_RESULT_FIELD)?.unwrap_or_default();
    if retries == NEXTEST_ZERO_RETRIES {
        return Ok("zero-retry".to_string());
    }
    if flaky_result == NEXTEST_FLAKY_PASS {
        Ok("retry-pass".to_string())
    } else {
        Ok("retry-diagnostic".to_string())
    }
}

fn nextest_profile_string(config: &toml::Value, profile_id: &str, field: &str) -> Outcome<Option<String>> {
    nextest_profile_field(config, profile_id, field)?.map_or(Ok(None), |value| {
        value.as_str().map(|text| Some(text.to_string())).ok_or_else(|| {
            molten::error::MoltenError::invalid_harness(format!("profile {profile_id} field {field} must be a string"))
        })
    })
}

fn nextest_profile_nested_string(
    config: &toml::Value,
    profile_id: &str,
    table_field: &str,
    field: &str,
) -> Outcome<Option<String>> {
    nextest_profile_nested_field(config, profile_id, table_field, field)?.map_or(Ok(None), |value| {
        value.as_str().map(|text| Some(text.to_string())).ok_or_else(|| {
            molten::error::MoltenError::invalid_harness(format!(
                "profile {profile_id} field {table_field}.{field} must be a string"
            ))
        })
    })
}

fn nextest_profile_integer(config: &toml::Value, profile_id: &str, field: &str) -> Outcome<Option<i64>> {
    nextest_profile_field(config, profile_id, field)?.map_or(Ok(None), |value| {
        value.as_integer().map(Some).ok_or_else(|| {
            molten::error::MoltenError::invalid_harness(format!(
                "profile {profile_id} field {field} must be an integer"
            ))
        })
    })
}

fn nextest_profile_field<'a>(
    config: &'a toml::Value,
    profile_id: &str,
    field: &str,
) -> Outcome<Option<&'a toml::Value>> {
    let mut current_profile = profile_id.to_string();
    for _depth in 0..MAX_NEXTEST_PROFILE_INHERITANCE_DEPTH {
        let Some(table) = nextest_profile_table(config, &current_profile) else {
            return Ok(None);
        };
        if let Some(value) = table.get(field) {
            return Ok(Some(value));
        }
        let Some(parent) = table.get(NEXTEST_INHERITS_FIELD).and_then(|value| value.as_str()) else {
            return Ok(None);
        };
        current_profile = parent.to_string();
    }
    Err(molten::error::MoltenError::invalid_harness(format!(
        "profile {profile_id} inheritance exceeds bound"
    )))
}

fn nextest_profile_nested_field<'a>(
    config: &'a toml::Value,
    profile_id: &str,
    table_field: &str,
    field: &str,
) -> Outcome<Option<&'a toml::Value>> {
    let mut current_profile = profile_id.to_string();
    for _depth in 0..MAX_NEXTEST_PROFILE_INHERITANCE_DEPTH {
        let Some(table) = nextest_profile_table(config, &current_profile) else {
            return Ok(None);
        };
        if let Some(value) =
            table.get(table_field).and_then(|nested| nested.as_table()).and_then(|nested| nested.get(field))
        {
            return Ok(Some(value));
        }
        let Some(parent) = table.get(NEXTEST_INHERITS_FIELD).and_then(|value| value.as_str()) else {
            return Ok(None);
        };
        current_profile = parent.to_string();
    }
    Err(molten::error::MoltenError::invalid_harness(format!(
        "profile {profile_id} inheritance exceeds bound"
    )))
}

fn nextest_profile_table<'a>(
    config: &'a toml::Value,
    profile_id: &str,
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    config
        .get(NEXTEST_PROFILE_TABLE)
        .and_then(|profiles| profiles.as_table())
        .and_then(|profiles| profiles.get(profile_id))
        .and_then(|profile| profile.as_table())
}

fn render_nextest_profile_matrix_summary(matrix: &molten::testing_hardening::NextestProfileMatrix) -> String {
    let diagnostics = if matrix.diagnostics.is_empty() {
        "none".to_string()
    } else {
        matrix.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
    };
    format!(
        "nextest-profile-matrix ref={} decision={} diagnostics={}\n",
        matrix.matrix_ref, matrix.decision, diagnostics
    )
}

fn run_config_lint(input: ConfigLintCommandInput) -> Outcome<()> {
    let files = read_config_lint_files(&input.root)?;
    let source_pins = read_source_pin_records(&input.root)?;
    let report = molten::project_config_portability::build_config_portability_report(
        &molten::project_config_portability::ConfigPortabilityInput { files, source_pins },
    )?;
    write_optional_preserves(input.out.as_ref(), &report.value)?;
    write_optional_text(input.summary_out.as_ref(), &render_config_lint_summary(&report))?;
    eprintln!("config-portability report={} decision={}", report.report_ref, report.decision);
    if report.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "config portability denied: {}",
            report.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
        )))
    }
}

fn read_config_lint_files(
    root: &std::path::Path,
) -> Outcome<Vec<molten::project_config_portability::ConfigFileRecord>> {
    let mut records = Vec::with_capacity(CONFIG_LINT_FILES.len());
    for (relative_path, release_scoped) in CONFIG_LINT_FILES {
        let path = root.join(relative_path);
        let contents = std::fs::read_to_string(&path).map_err(molten::error::MoltenError::from)?;
        records.push(molten::project_config_portability::ConfigFileRecord {
            path: (*relative_path).to_string(),
            contents,
            release_scoped: *release_scoped,
        });
    }
    Ok(records)
}
