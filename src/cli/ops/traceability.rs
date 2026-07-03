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
            out,
            summary_out,
        } => run_scan(ScanInput {
            root,
            changed_only,
            coverage,
            exemptions,
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
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

fn run_scan(input: ScanInput) -> Outcome<()> {
    let sources = collect_spec_sources(&input.root)?;
    let mut requirements = molten::requirement_traceability::requirements_from_sources(&sources)?;
    if input.changed_only {
        requirements.retain(|requirement| requirement.changed);
    }
    let coverage = parse_coverage_inputs(&input.root, input.coverage, input.exemptions)?;
    let manifest = molten::requirement_traceability::build_traceability_manifest(
        &molten::requirement_traceability::TraceabilityInput { requirements, coverage },
    )?;
    write_optional_preserves(input.out.as_ref(), &manifest.value)?;
    let summary = molten::requirement_traceability::render_summary(&manifest.summary)?;
    write_optional_text(input.summary_out.as_ref(), &summary)?;
    println!(
        "traceability ref={} decision={} covered={} missing-negative={} stale={}",
        manifest.manifest_ref,
        manifest.decision,
        manifest.summary.covered.len(),
        manifest.summary.missing_negative.len(),
        manifest.summary.stale_reference.len()
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
        artifact_ref,
        target_exists,
        artifact_present,
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
