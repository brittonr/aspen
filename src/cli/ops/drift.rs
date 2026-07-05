type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum DriftCommand {
    Compare {
        #[arg(long)]
        workflow: String,
        #[arg(long)]
        left: FilePath,
        #[arg(long)]
        right: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
    },
    CompareSummary {
        #[arg(long)]
        workflow: String,
        #[arg(long = "left-ref")]
        left_refs: Vec<String>,
        #[arg(long = "right-ref")]
        right_refs: Vec<String>,
        #[arg(long = "left-field")]
        left_fields: Vec<String>,
        #[arg(long = "right-field")]
        right_fields: Vec<String>,
        #[arg(long = "variance")]
        variances: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Rerun {
        #[arg(long)]
        workflow: String,
        #[arg(long = "left-root")]
        left_root: FilePath,
        #[arg(long = "right-root")]
        right_root: FilePath,
        #[arg(long)]
        command: String,
        #[arg(long = "arg")]
        args: Vec<String>,
        #[arg(long)]
        artifact: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
    },
}

pub(crate) fn run_drift_command(command: DriftCommand) -> Outcome<()> {
    match command {
        DriftCommand::Compare {
            workflow,
            left,
            right,
            out,
        } => run_compare(workflow, left, right, out),
        DriftCommand::CompareSummary {
            workflow,
            left_refs,
            right_refs,
            left_fields,
            right_fields,
            variances,
            out,
        } => run_compare_summary(CompareSummaryInput {
            workflow,
            left_refs,
            right_refs,
            left_fields,
            right_fields,
            variances,
            out,
        }),
        DriftCommand::Rerun {
            workflow,
            left_root,
            right_root,
            command,
            args,
            artifact,
            out,
        } => run_rerun(RerunInput {
            workflow,
            left_root,
            right_root,
            command,
            args,
            artifact,
            out,
        }),
    }
}

struct RerunInput {
    workflow: String,
    left_root: FilePath,
    right_root: FilePath,
    command: String,
    args: Vec<String>,
    artifact: FilePath,
    out: Option<FilePath>,
}

struct CompareSummaryInput {
    workflow: String,
    left_refs: Vec<String>,
    right_refs: Vec<String>,
    left_fields: Vec<String>,
    right_fields: Vec<String>,
    variances: Vec<String>,
    out: Option<FilePath>,
}

fn run_compare(workflow: String, left: FilePath, right: FilePath, out: Option<FilePath>) -> Outcome<()> {
    let left_value = read_preserves_file(&left)?;
    let right_value = read_preserves_file(&right)?;
    let comparison = molten::deterministic_drift::compare(&molten::deterministic_drift::ComparisonInput {
        left: molten::deterministic_drift::artifact_summary(&workflow, &left_value)?,
        right: molten::deterministic_drift::artifact_summary(&workflow, &right_value)?,
        allowed_variances: Vec::new(),
    })?;
    finish_comparison(comparison, out)
}

fn run_compare_summary(input: CompareSummaryInput) -> Outcome<()> {
    let comparison = molten::deterministic_drift::compare(&molten::deterministic_drift::ComparisonInput {
        left: molten::deterministic_drift::EvidenceSummary {
            workflow: input.workflow.clone(),
            fields: parse_fields(input.left_refs, input.left_fields)?,
        },
        right: molten::deterministic_drift::EvidenceSummary {
            workflow: input.workflow,
            fields: parse_fields(input.right_refs, input.right_fields)?,
        },
        allowed_variances: parse_variances(input.variances)?,
    })?;
    finish_comparison(comparison, input.out)
}

fn run_rerun(input: RerunInput) -> Outcome<()> {
    ensure_fresh_root(&input.left_root)?;
    ensure_fresh_root(&input.right_root)?;
    run_workflow(&input.command, &input.args, &input.left_root, "left")?;
    run_workflow(&input.command, &input.args, &input.right_root, "right")?;
    let left_artifact = input.left_root.join(&input.artifact);
    let right_artifact = input.right_root.join(&input.artifact);
    run_compare(input.workflow, left_artifact, right_artifact, input.out)
}

fn finish_comparison(comparison: molten::deterministic_drift::Comparison, out: Option<FilePath>) -> Outcome<()> {
    write_optional_preserves(out.as_ref(), &comparison.value)?;
    println!(
        "deterministic-drift ref={} decision={} diagnostics={}",
        comparison.receipt_ref,
        comparison.decision,
        comparison.diagnostics.len()
    );
    if comparison.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "deterministic drift denied: {}",
            comparison
                .diagnostics
                .iter()
                .map(|diagnostic| format!("{}:{}", diagnostic.path, diagnostic.kind))
                .collect::<Vec<_>>()
                .join(",")
        )))
    }
}

fn ensure_fresh_root(root: &std::path::Path) -> Outcome<()> {
    if root.exists() {
        let mut entries = std::fs::read_dir(root).map_err(molten::error::MoltenError::from)?;
        if entries.next().is_some() {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "drift rerun root {} must be empty",
                root.display()
            )));
        }
    }
    std::fs::create_dir_all(root).map_err(molten::error::MoltenError::from)
}

fn run_workflow(command: &str, args: &[String], root: &std::path::Path, label: &str) -> Outcome<()> {
    let root_text = root.display().to_string();
    let resolved_args = args.iter().map(|arg| arg.replace("{root}", &root_text)).collect::<Vec<_>>();
    let output = std::process::Command::new(command)
        .args(&resolved_args)
        .output()
        .map_err(molten::error::MoltenError::from)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "drift rerun {label} command failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

fn parse_fields(refs: Vec<String>, fields: Vec<String>) -> Outcome<Vec<molten::deterministic_drift::EvidenceField>> {
    let mut output = Vec::with_capacity(refs.len() + fields.len());
    for pair in refs {
        let (path, value) = split_pair(&pair, "left/right ref")?;
        output.push(molten::deterministic_drift::EvidenceField {
            path,
            value,
            is_ref: true,
        });
    }
    for pair in fields {
        let (path, value) = split_pair(&pair, "left/right field")?;
        output.push(molten::deterministic_drift::EvidenceField {
            path,
            value,
            is_ref: false,
        });
    }
    Ok(output)
}

fn parse_variances(items: Vec<String>) -> Outcome<Vec<molten::deterministic_drift::AllowedVariance>> {
    let mut output = Vec::with_capacity(items.len());
    for item in items {
        let (path, reason) = split_pair(&item, "variance")?;
        output.push(molten::deterministic_drift::AllowedVariance { path, reason });
    }
    Ok(output)
}

fn split_pair(pair: &str, label: &str) -> Outcome<(String, String)> {
    let Some((key, value)) = pair.split_once('=') else {
        return Err(molten::error::MoltenError::invalid_harness(format!("{label} must use path=value syntax")));
    };
    if key.trim().is_empty() || value.trim().is_empty() {
        return Err(molten::error::MoltenError::invalid_harness(format!("{label} path and value must not be empty")));
    }
    Ok((key.to_string(), value.to_string()))
}

fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

fn write_optional_preserves(path: Option<&FilePath>, value: &preserves::IOValue) -> Outcome<()> {
    let text = molten::preserves_rail::to_text(value)?;
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
