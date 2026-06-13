use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::octet_gate;
use molten::octet_remediation;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

#[derive(Debug, Subcommand)]
pub(crate) enum OctetCommand {
    Gate {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long, default_value = "strict-ci")]
        profile: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SourceGate {
        #[command(subcommand)]
        command: OctetSourceGateCommand,
    },
    Baseline {
        #[command(subcommand)]
        command: OctetBaselineCommand,
    },
    Review {
        #[command(subcommand)]
        command: OctetReviewCommand,
    },
    Artifacts {
        #[command(subcommand)]
        command: OctetArtifactsCommand,
    },
    Remediation {
        #[command(subcommand)]
        command: OctetRemediationCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OctetRemediationCommand {
    Plan {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long = "lib-artifacts")]
        lib_artifacts: Option<PathBuf>,
        #[arg(long = "focused-object-corpus")]
        focused_object_corpus: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OctetSourceGateCommand {
    Validate {
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        gate_receipt: PathBuf,
        #[arg(long = "source-scope")]
        source_scope: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OctetArtifactsCommand {
    Import {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OctetReviewCommand {
    Write {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long = "finding-key")]
        finding_keys: Vec<String>,
        #[arg(long, default_value = "manual review")]
        rationale: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OctetBaselineCommand {
    Write {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "manual")]
        created_at: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long)]
        target_next: Option<u64>,
    },
    Check {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        as_of: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long = "review")]
        reviews: Vec<PathBuf>,
    },
}

pub(crate) fn run_octet_command(command: OctetCommand) -> Result<()> {
    match command {
        OctetCommand::Gate {
            artifacts,
            profile,
            receipt_out,
        } => {
            let evaluation = octet_gate::evaluate_octet_gate(&octet_gate::OctetGateInput {
                artifacts_dir: artifacts.clone(),
                profile,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet gate receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet gate denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet gate pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
        OctetCommand::SourceGate { command } => run_octet_source_gate_command(command),
        OctetCommand::Baseline { command } => run_octet_baseline_command(command),
        OctetCommand::Review { command } => run_octet_review_command(command),
        OctetCommand::Artifacts { command } => run_octet_artifacts_command(command),
        OctetCommand::Remediation { command } => run_octet_remediation_command(command),
    }
}

fn run_octet_remediation_command(command: OctetRemediationCommand) -> Result<()> {
    match command {
        OctetRemediationCommand::Plan {
            artifacts,
            lib_artifacts,
            focused_object_corpus,
            receipt_out,
        } => {
            let plan =
                octet_remediation::build_octet_remediation_plan(&octet_remediation::OctetRemediationPlanInput {
                    artifacts_dir: artifacts,
                    lib_artifacts_dir: lib_artifacts,
                    focused_object_corpus,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "octet remediation plan", &plan.value)?;
            println!("octet remediation plan receipt={}", plan.plan_ref);
            Ok(())
        }
    }
}

fn run_octet_source_gate_command(command: OctetSourceGateCommand) -> Result<()> {
    match command {
        OctetSourceGateCommand::Validate {
            consumer,
            subject,
            gate_receipt,
            source_scope,
            receipt_out,
        } => {
            let gate_receipt_value = read_preserves_file(&gate_receipt)?;
            let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
                consumer,
                subject_ref: subject,
                gate_receipt_value: Some(gate_receipt_value),
                source_scope,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet source gate validation", &validation.value)?;
            if validation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet source gate validation denied receipt={}",
                    validation.validation_ref
                )));
            }
            println!("octet source gate validation pass receipt={}", validation.validation_ref);
            Ok(())
        }
    }
}

fn run_octet_artifacts_command(command: OctetArtifactsCommand) -> Result<()> {
    match command {
        OctetArtifactsCommand::Import {
            artifacts,
            ledger,
            receipt_out,
        } => {
            let imported = octet_gate::import_octet_artifacts_to_ledger(&octet_gate::OctetArtifactLedgerInput {
                artifacts_dir: artifacts,
                ledger_root: ledger.clone(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet artifact ledger receipt", &imported.receipt_value)?;
            println!(
                "octet artifacts import decision={} receipt={} imported={} ledger={}",
                imported.decision,
                imported.receipt_ref,
                imported.imported_refs.len(),
                ledger.display()
            );
            Ok(())
        }
    }
}

fn run_octet_review_command(command: OctetReviewCommand) -> Result<()> {
    match command {
        OctetReviewCommand::Write {
            out,
            profile,
            expires_at,
            finding_keys,
            rationale,
        } => {
            let review = octet_gate::build_octet_review_manifest(&octet_gate::OctetReviewManifestInput {
                profile,
                expires_at,
                finding_keys,
                rationale,
            })?;
            write_file(&out, &to_text(&review.review_value)?)?;
            println!("octet review manifest {} written to {}", review.review_ref, out.display());
            Ok(())
        }
    }
}

fn run_octet_baseline_command(command: OctetBaselineCommand) -> Result<()> {
    match command {
        OctetBaselineCommand::Write {
            artifacts,
            out,
            created_at,
            expires_at,
            target_next,
        } => {
            let baseline = octet_gate::build_octet_warning_baseline(&octet_gate::OctetWarningBaselineInput {
                artifacts_dir: artifacts,
                created_at,
                expires_at,
                target_next,
            })?;
            write_file(&out, &to_text(&baseline.baseline_value)?)?;
            println!(
                "octet warning baseline {} written to {} findings={} critical={}",
                baseline.baseline_ref,
                out.display(),
                baseline.finding_count,
                baseline.critical_count
            );
            Ok(())
        }
        OctetBaselineCommand::Check {
            artifacts,
            baseline,
            profile,
            as_of,
            receipt_out,
            reviews,
        } => {
            let baseline_value = read_preserves_file(&baseline)?;
            let review_values = reviews.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let evaluation = octet_gate::check_octet_warning_baseline(&octet_gate::OctetBaselineCheckInput {
                artifacts_dir: artifacts.clone(),
                baseline_value,
                profile,
                as_of,
                review_values,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet baseline receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet baseline denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet baseline pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
