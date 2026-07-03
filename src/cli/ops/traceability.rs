type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

const COVERAGE_FIELDS: usize = 5;
const EXEMPTION_FIELDS: usize = 3;

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

fn collect_spec_sources(root: &std::path::Path) -> Outcome<Vec<molten::requirement_traceability::SpecSource>> {
    let mut sources = Vec::new();
    collect_specs_under(&root.join("cairn/specs"), false, &mut sources)?;
    collect_specs_under(&root.join("cairn/changes"), true, &mut sources)?;
    Ok(sources)
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
