type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

const COVERAGE_FIELDS: usize = 5;
const EXEMPTION_FIELDS: usize = 3;
const JUNIT_TESTS_ATTRIBUTE: &str = "tests";
const JUNIT_FAILURES_ATTRIBUTE: &str = "failures";
const JUNIT_ERRORS_ATTRIBUTE: &str = "errors";
const JUNIT_SKIPPED_ATTRIBUTE: &str = "skipped";
const JUNIT_QUOTE: char = '"';
const NEXTEST_PROFILE_TABLE: &str = "profile";
const NEXTEST_INHERITS_FIELD: &str = "inherits";
const NEXTEST_DEFAULT_FILTER_FIELD: &str = "default-filter";
const NEXTEST_RETRIES_FIELD: &str = "retries";
const NEXTEST_FLAKY_RESULT_FIELD: &str = "flaky-result";
const NEXTEST_JUNIT_TABLE: &str = "junit";
const NEXTEST_JUNIT_PATH_FIELD: &str = "path";
const NEXTEST_ZERO_RETRIES: i64 = 0;
const MAX_NEXTEST_PROFILE_INHERITANCE_DEPTH: usize = 16;
const NEXTEST_FLAKY_PASS: &str = "pass";
const DIAGNOSTIC_JOIN_SEPARATOR: &str = "; ";
const CONFIG_LINT_FILES: &[(&str, bool)] = &[
    (".pre-commit-config.yaml", true),
    ("flake.nix", true),
    ("rust-toolchain.toml", true),
    ("README.md", true),
    ("docs/proof-workflow.md", true),
];
const CARGO_SOURCE_PREFIX: &str = "git+ssh://git@github.com/OnixResearch/";
const NIX_SOURCE_PREFIX: &str = "ssh://git@github.com/OnixResearch/";
const SOURCE_REVISION_SEPARATOR: char = '#';
const GIT_SUFFIX: &str = ".git";
const TOML_QUOTE: char = '"';
const EFFECTIVE_CONFIG_FIELD_PARTS: usize = 5;
const EFFECTIVE_CONFIG_FIELD_SEPARATOR: char = '|';
const EFFECTIVE_CONFIG_CAVEAT_SEPARATOR: char = ',';
const EFFECTIVE_CONFIG_NONE_REF: &str = "none";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum TraceabilityCommand {
    Scan {
        #[arg(long, default_value = ".")]
        root: FilePath,
        #[arg(long)]
        changed_only: bool,
        #[arg(long = "coverage")]
        coverage: Vec<String>,
        #[arg(long = "exemption")]
        exemptions: Vec<String>,
        #[arg(long = "receipt")]
        receipts: Vec<FilePath>,
        #[arg(long = "require-receipt-backed")]
        require_receipt_backed: bool,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
        #[arg(long = "readback-out")]
        readback_out: Option<FilePath>,
    },
    VerificationRun {
        #[arg(long)]
        requirement: String,
        #[arg(long = "coverage-kind")]
        coverage_kind: String,
        #[arg(long)]
        target: String,
        #[arg(long = "argv")]
        argv: Vec<String>,
        #[arg(long = "profile-ref")]
        profile_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long = "exit-status")]
        exit_status: i64,
        #[arg(long = "stdout-ref")]
        stdout_ref: String,
        #[arg(long = "stderr-ref")]
        stderr_ref: String,
        #[arg(long = "artifact-ref")]
        artifact_refs: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    CiRunReceipt {
        #[arg(long = "source-marker")]
        source_marker: String,
        #[arg(long = "profile-id")]
        profile_id: String,
        #[arg(long = "command-surface")]
        command_surface: String,
        #[arg(long = "nextest-config")]
        nextest_config: FilePath,
        #[arg(long = "cargo-metadata")]
        cargo_metadata: FilePath,
        #[arg(long = "binaries-metadata")]
        binaries_metadata: FilePath,
        #[arg(long)]
        junit: FilePath,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    NextestProfileMatrix {
        #[arg(long = "nextest-config")]
        nextest_config: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
    ConfigLint {
        #[arg(long, default_value = ".")]
        root: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
    EffectiveConfig {
        #[arg(long = "profile-ref")]
        profile_refs: Vec<String>,
        #[arg(long = "field")]
        fields: Vec<String>,
        #[arg(long = "release-mode")]
        release_mode: bool,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
}

pub(crate) fn run_traceability_command(command: TraceabilityCommand) -> Outcome<()> {
    match command {
        TraceabilityCommand::Scan {
            root,
            changed_only,
            coverage,
            exemptions,
            receipts,
            require_receipt_backed,
            out,
            summary_out,
            readback_out,
        } => run_scan(ScanInput {
            root,
            changed_only,
            coverage,
            exemptions,
            receipts,
            require_receipt_backed,
            out,
            summary_out,
            readback_out,
        }),
        TraceabilityCommand::VerificationRun {
            requirement,
            coverage_kind,
            target,
            argv,
            profile_ref,
            toolchain_refs,
            exit_status,
            stdout_ref,
            stderr_ref,
            artifact_refs,
            out,
        } => run_verification_run(VerificationRunCommandInput {
            requirement,
            coverage_kind,
            target,
            argv,
            profile_ref,
            toolchain_refs,
            exit_status,
            stdout_ref,
            stderr_ref,
            artifact_refs,
            out,
        }),
        TraceabilityCommand::CiRunReceipt {
            source_marker,
            profile_id,
            command_surface,
            nextest_config,
            cargo_metadata,
            binaries_metadata,
            junit,
            decision,
            caveats,
            out,
        } => run_ci_run_receipt(CiRunReceiptCommandInput {
            source_marker,
            profile_id,
            command_surface,
            nextest_config,
            cargo_metadata,
            binaries_metadata,
            junit,
            decision,
            caveats,
            out,
        }),
        TraceabilityCommand::NextestProfileMatrix {
            nextest_config,
            out,
            summary_out,
        } => run_nextest_profile_matrix(NextestProfileMatrixCommandInput {
            nextest_config,
            out,
            summary_out,
        }),
        TraceabilityCommand::ConfigLint { root, out, summary_out } => {
            run_config_lint(ConfigLintCommandInput { root, out, summary_out })
        }
        TraceabilityCommand::EffectiveConfig {
            profile_refs,
            fields,
            release_mode,
            out,
            summary_out,
        } => run_effective_config(EffectiveConfigCommandInput {
            profile_refs,
            fields,
            release_mode,
            out,
            summary_out,
        }),
    }
}

struct ScanInput {
    root: FilePath,
    changed_only: bool,
    coverage: Vec<String>,
    exemptions: Vec<String>,
    receipts: Vec<FilePath>,
    require_receipt_backed: bool,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
    readback_out: Option<FilePath>,
}

struct VerificationRunCommandInput {
    requirement: String,
    coverage_kind: String,
    target: String,
    argv: Vec<String>,
    profile_ref: String,
    toolchain_refs: Vec<String>,
    exit_status: i64,
    stdout_ref: String,
    stderr_ref: String,
    artifact_refs: Vec<String>,
    out: Option<FilePath>,
}

struct CiRunReceiptCommandInput {
    source_marker: String,
    profile_id: String,
    command_surface: String,
    nextest_config: FilePath,
    cargo_metadata: FilePath,
    binaries_metadata: FilePath,
    junit: FilePath,
    decision: String,
    caveats: Vec<String>,
    out: Option<FilePath>,
}

struct NextestProfileMatrixCommandInput {
    nextest_config: FilePath,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

struct ConfigLintCommandInput {
    root: FilePath,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

struct EffectiveConfigCommandInput {
    profile_refs: Vec<String>,
    fields: Vec<String>,
    release_mode: bool,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

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

fn read_source_pin_records(
    root: &std::path::Path,
) -> Outcome<Vec<molten::project_config_portability::SourcePinRecord>> {
    let cargo_lock = std::fs::read_to_string(root.join("Cargo.lock")).map_err(molten::error::MoltenError::from)?;
    let flake = std::fs::read_to_string(root.join("flake.nix")).map_err(molten::error::MoltenError::from)?;
    let cargo_revisions = cargo_private_revisions(&cargo_lock);
    let nix_revisions = nix_private_revisions(&flake);
    let mut dependencies = std::collections::BTreeSet::new();
    dependencies.extend(cargo_revisions.keys().cloned());
    dependencies.extend(nix_revisions.keys().cloned());
    Ok(dependencies
        .into_iter()
        .map(|dependency| molten::project_config_portability::SourcePinRecord {
            cargo_revision: cargo_revisions.get(&dependency).cloned().unwrap_or_else(|| "missing".to_string()),
            nix_revision: nix_revisions.get(&dependency).cloned().unwrap_or_else(|| "missing".to_string()),
            dependency,
        })
        .collect())
}

fn cargo_private_revisions(lock_text: &str) -> std::collections::BTreeMap<String, String> {
    let mut revisions = std::collections::BTreeMap::new();
    for line in lock_text.lines() {
        let Some(source_start) = line.find(CARGO_SOURCE_PREFIX) else {
            continue;
        };
        let source = &line[source_start + CARGO_SOURCE_PREFIX.len()..];
        if let Some((dependency, revision)) = parse_source_dependency_revision(source) {
            revisions.entry(dependency).or_insert(revision);
        }
    }
    revisions
}

fn nix_private_revisions(flake_text: &str) -> std::collections::BTreeMap<String, String> {
    let mut revisions = std::collections::BTreeMap::new();
    for line in flake_text.lines() {
        let Some(source_start) = line.find(NIX_SOURCE_PREFIX) else {
            continue;
        };
        let source = &line[source_start + NIX_SOURCE_PREFIX.len()..];
        if let Some((dependency, revision)) = parse_source_dependency_revision(source) {
            revisions.entry(dependency).or_insert(revision);
        }
    }
    revisions
}

fn parse_source_dependency_revision(source: &str) -> Option<(String, String)> {
    let (dependency_part, revision_part) = source.split_once(SOURCE_REVISION_SEPARATOR)?;
    let dependency = dependency_part.split_once(GIT_SUFFIX).map(|(name, _suffix)| name).unwrap_or(dependency_part);
    if dependency.is_empty() {
        return None;
    }
    let revision = revision_part.split(TOML_QUOTE).next().unwrap_or_default().trim().to_string();
    if revision.is_empty() {
        return None;
    }
    Some((dependency.to_string(), revision))
}

fn render_config_lint_summary(report: &molten::project_config_portability::ConfigPortabilityReport) -> String {
    let diagnostics = if report.diagnostics.is_empty() {
        "none".to_string()
    } else {
        report.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
    };
    format!(
        "config-portability report={} decision={} compared={} diagnostics={}\n",
        report.report_ref,
        report.decision,
        report.compared_source_pins.join(","),
        diagnostics
    )
}

fn run_effective_config(input: EffectiveConfigCommandInput) -> Outcome<()> {
    let sources = parse_effective_config_fields(input.fields)?;
    let readback = molten::project_effective_config::build_effective_config_readback(
        &molten::project_effective_config::EffectiveConfigInput {
            profile_refs: input.profile_refs,
            sources,
            release_mode: input.release_mode,
            diagnostics: Vec::new(),
        },
    )?;
    write_optional_preserves(input.out.as_ref(), &readback.value)?;
    write_optional_text(
        input.summary_out.as_ref(),
        &molten::project_effective_config::explain_effective_config(&readback)?,
    )?;
    eprintln!("effective-config ref={} decision={}", readback.fingerprint_ref, readback.decision);
    if readback.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "effective config denied: {}",
            readback.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
        )))
    }
}

fn parse_effective_config_fields(
    fields: Vec<String>,
) -> Outcome<Vec<molten::project_effective_config::ConfigSourceInput>> {
    let mut output = Vec::with_capacity(fields.len());
    for field in fields {
        let parts = field.split(EFFECTIVE_CONFIG_FIELD_SEPARATOR).map(str::to_string).collect::<Vec<_>>();
        if parts.len() != EFFECTIVE_CONFIG_FIELD_PARTS {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "effective config field must have {EFFECTIVE_CONFIG_FIELD_PARTS} pipe-delimited parts"
            )));
        }
        if parts.iter().take(EFFECTIVE_CONFIG_FIELD_PARTS - 1).any(|part| part.trim().is_empty()) {
            return Err(molten::error::MoltenError::invalid_harness("effective config field parts must not be empty"));
        }
        let caveats = if parts[4].trim().is_empty() {
            Vec::new()
        } else {
            parts[4]
                .split(EFFECTIVE_CONFIG_CAVEAT_SEPARATOR)
                .map(str::trim)
                .filter(|caveat| !caveat.is_empty())
                .map(str::to_string)
                .collect()
        };
        output.push(molten::project_effective_config::ConfigSourceInput {
            field: parts[0].clone(),
            value: parts[1].clone(),
            source_class: parts[2].clone(),
            source_ref: if parts[3] == EFFECTIVE_CONFIG_NONE_REF {
                None
            } else {
                Some(parts[3].clone())
            },
            admitted_override: parts[2] == "cli-override",
            caveats,
        });
    }
    Ok(output)
}

fn collect_spec_sources(root: &std::path::Path) -> Outcome<Vec<molten::requirement_traceability::SpecSource>> {
    let mut sources = Vec::new();
    collect_specs_under(&root.join("cairn/specs"), false, &mut sources)?;
    collect_specs_under(&root.join("cairn/changes"), true, &mut sources)?;
    Ok(sources)
}

fn raw_file_ref(path: &std::path::Path) -> Outcome<String> {
    let bytes = std::fs::read(path).map_err(molten::error::MoltenError::from)?;
    Ok(molten::preserves_rail::content_ref_from_bytes(&bytes))
}

fn parse_junit_counts(text: &str) -> Outcome<molten::testing_hardening::CiTestCounts> {
    let total = junit_attribute(text, JUNIT_TESTS_ATTRIBUTE)?;
    let failures = junit_optional_attribute(text, JUNIT_FAILURES_ATTRIBUTE)?;
    let errors = junit_optional_attribute(text, JUNIT_ERRORS_ATTRIBUTE)?;
    let skipped = junit_optional_attribute(text, JUNIT_SKIPPED_ATTRIBUTE)?;
    let failed = failures
        .checked_add(errors)
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("JUnit failure/error count overflow"))?;
    Ok(molten::testing_hardening::CiTestCounts {
        total,
        passed: junit_passed_count(total, failed, skipped)?,
        failed,
        skipped,
    })
}

fn junit_passed_count(total: u64, failed: u64, skipped: u64) -> Outcome<u64> {
    total
        .checked_sub(failed)
        .and_then(|count| count.checked_sub(skipped))
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("JUnit passed count underflow"))
}

fn junit_optional_attribute(text: &str, name: &str) -> Outcome<u64> {
    match junit_attribute_value(text, name)? {
        Some(value) => parse_junit_attribute_value(value, name),
        None => Ok(0),
    }
}

fn junit_attribute(text: &str, name: &str) -> Outcome<u64> {
    let Some(value) = junit_attribute_value(text, name)? else {
        return Err(molten::error::MoltenError::invalid_harness(format!("JUnit missing {name} attribute")));
    };
    parse_junit_attribute_value(value, name)
}

fn junit_attribute_value<'a>(text: &'a str, name: &str) -> Outcome<Option<&'a str>> {
    let prefix = format!("{name}=\" ");
    let compact_prefix = prefix.trim_end();
    let Some(start_index) = text.find(compact_prefix).map(|index| index + compact_prefix.len()) else {
        return Ok(None);
    };
    let rest = &text[start_index..];
    let Some(end_index) = rest.find(JUNIT_QUOTE) else {
        return Err(molten::error::MoltenError::invalid_harness(format!("JUnit unterminated {name} attribute")));
    };
    Ok(Some(&rest[..end_index]))
}

fn parse_junit_attribute_value(value: &str, name: &str) -> Outcome<u64> {
    value
        .parse::<u64>()
        .map_err(|error| molten::error::MoltenError::invalid_harness(format!("JUnit invalid {name} count: {error}")))
}

fn collect_specs_under(
    path: &std::path::Path,
    changed: bool,
    sources: &mut Vec<molten::requirement_traceability::SpecSource>,
) -> Outcome<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.file_name().is_some_and(|name| name == std::ffi::OsStr::new("spec.md")) {
            let markdown = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
            sources.push(molten::requirement_traceability::SpecSource {
                source: path.display().to_string(),
                markdown,
                changed,
                default_kind: "evidence".to_string(),
            });
        }
        return Ok(());
    }
    let mut entries = std::fs::read_dir(path)
        .map_err(molten::error::MoltenError::from)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(molten::error::MoltenError::from)?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        collect_specs_under(&entry.path(), changed, sources)?;
    }
    Ok(())
}

fn parse_coverage_inputs(
    root: &std::path::Path,
    coverage_items: Vec<String>,
    exemption_items: Vec<String>,
) -> Outcome<Vec<molten::requirement_traceability::CoverageInput>> {
    let mut coverage = std::collections::BTreeMap::<String, molten::requirement_traceability::CoverageInput>::new();
    for item in coverage_items {
        let fields = split_fields(&item, COVERAGE_FIELDS, "coverage")?;
        let evidence = evidence_from_fields(root, &fields)?;
        let entry =
            coverage
                .entry(fields[0].clone())
                .or_insert_with(|| molten::requirement_traceability::CoverageInput {
                    requirement_id: fields[0].clone(),
                    positive: Vec::new(),
                    negative: Vec::new(),
                    exemption: None,
                });
        match fields[1].as_str() {
            "positive" => entry.positive.push(evidence),
            "negative" => entry.negative.push(evidence),
            other => {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "coverage kind {other} must be positive or negative"
                )));
            }
        }
    }
    for item in exemption_items {
        let fields = split_fields(&item, EXEMPTION_FIELDS, "exemption")?;
        let entry =
            coverage
                .entry(fields[0].clone())
                .or_insert_with(|| molten::requirement_traceability::CoverageInput {
                    requirement_id: fields[0].clone(),
                    positive: Vec::new(),
                    negative: Vec::new(),
                    exemption: None,
                });
        entry.exemption = Some(molten::requirement_traceability::CoverageExemption {
            class: fields[1].clone(),
            evidence: fields[2].clone(),
        });
    }
    Ok(coverage.into_values().collect())
}

fn parse_receipt_inputs(
    root: &std::path::Path,
    receipt_paths: Vec<FilePath>,
) -> Outcome<Vec<molten::requirement_traceability::CoverageInput>> {
    let mut sources = Vec::with_capacity(receipt_paths.len());
    for path in receipt_paths {
        let text = std::fs::read_to_string(&path).map_err(molten::error::MoltenError::from)?;
        let value = molten::preserves_rail::parse_text(&text)?;
        let receipt = molten::requirement_traceability::parse_verification_run_receipt(&value)?;
        let target_exists = root.join(&receipt.target).exists();
        sources.push(molten::requirement_traceability::ReceiptCoverageSource { value, target_exists });
    }
    molten::requirement_traceability::coverage_from_verification_receipts(&sources)
}

fn evidence_from_fields(
    root: &std::path::Path,
    fields: &[String],
) -> Outcome<molten::requirement_traceability::VerificationEvidence> {
    let target = fields[2].clone();
    let artifact_ref = fields[4].clone();
    let target_exists = root.join(&target).exists();
    let artifact_present = molten::preserves_rail::validate_content_ref(&artifact_ref).is_ok();
    Ok(molten::requirement_traceability::VerificationEvidence {
        target,
        command: fields[3].clone(),
        artifact_refs: vec![artifact_ref.clone()],
        artifact_ref,
        target_exists,
        artifact_present,
        source: "compatibility".to_string(),
        receipt_ref: None,
        expected_decision: "compatibility".to_string(),
    })
}

fn split_fields(item: &str, expected: usize, label: &str) -> Outcome<Vec<String>> {
    let fields = item.split('|').map(str::to_string).collect::<Vec<_>>();
    if fields.len() != expected {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "{label} entry must have {expected} pipe-delimited fields"
        )));
    }
    if fields.iter().any(|field| field.trim().is_empty()) {
        return Err(molten::error::MoltenError::invalid_harness(format!("{label} fields must not be empty")));
    }
    Ok(fields)
}

fn write_optional_preserves(path: Option<&FilePath>, value: &preserves::IOValue) -> Outcome<()> {
    let text = molten::preserves_rail::to_text(value)?;
    write_optional_text(path, &text)
}

fn write_optional_text(path: Option<&FilePath>, text: &str) -> Outcome<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
        }
        std::fs::write(path, text).map_err(molten::error::MoltenError::from)?;
    } else {
        println!("{text}");
    }
    Ok(())
}
