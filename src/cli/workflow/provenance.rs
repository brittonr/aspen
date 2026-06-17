use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::provenance;

#[path = "provenance/input.rs"]
mod input;

const PROVENANCE_CLI_EVIDENCE_LIMIT: usize = 64;
const _: () = assert!(PROVENANCE_CLI_EVIDENCE_LIMIT <= 100_000);

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum ProvenanceCommand {
    BuildRecord {
        #[arg(long)]
        expected_artifact_ref: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long = "build-param")]
        build_params: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "nix-derivation-ref")]
        nix_derivation_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    VerifyBuild {
        build_record: PathBuf,
        #[arg(long)]
        actual_artifact_ref: String,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Record {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        trust_state: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "review-ref")]
        review_refs: Vec<String>,
        #[arg(long = "test-ref")]
        test_refs: Vec<String>,
        #[arg(long = "source-gate-ref")]
        source_gate_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "build-record-ref")]
        build_record_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Fixture {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Evaluate {
        #[arg(long)]
        operation: String,
        #[arg(long, default_value = "node-control")]
        profile: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long = "provenance")]
        provenance_paths: Vec<PathBuf>,
        #[arg(long = "build-verification")]
        build_verification_paths: Vec<PathBuf>,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_provenance_command(command: ProvenanceCommand) -> Result<()> {
    match command {
        ProvenanceCommand::BuildRecord {
            expected_artifact_ref,
            source_refs,
            dependency_closure_ref,
            toolchain_refs,
            build_params,
            builder_ref,
            nix_derivation_refs,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let build_params = input::parse_build_params(&build_params)?;
            let value = provenance::provenance_build_record_value(&provenance::ProvenanceBuildRecordInput {
                expected_artifact_ref: &expected_artifact_ref,
                source_refs: &source_refs,
                dependency_closure_ref: &dependency_closure_ref,
                toolchain_refs: &toolchain_refs,
                build_params: &build_params,
                builder_ref: &builder_ref,
                nix_derivation_refs: &nix_derivation_refs,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance build record ref={reference} expected_artifact={expected_artifact_ref}"),
            );
            Ok(())
        }
        ProvenanceCommand::VerifyBuild {
            build_record,
            actual_artifact_ref,
            prior_diagnostics,
            receipt_out,
        } => {
            let build_record_value = read_preserves_file(&build_record)?;
            let verification = provenance::verify_provenance_build(&provenance::ProvenanceBuildVerificationInput {
                build_record_value: &build_record_value,
                actual_artifact_ref: &actual_artifact_ref,
                prior_diagnostics: &prior_diagnostics,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &verification.receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "provenance build verification decision={} expected={} actual={} receipt={} record={}",
                    verification.decision,
                    verification.expected_artifact_ref,
                    verification.actual_artifact_ref,
                    verification.receipt_ref,
                    verification.build_record_ref
                ),
            );
            Ok(())
        }
        ProvenanceCommand::Record {
            artifact_ref,
            trust_state,
            source_refs,
            dependency_closure_ref,
            toolchain_refs,
            builder_ref,
            review_refs,
            test_refs,
            source_gate_refs,
            policy_refs,
            build_record_refs,
            out,
        } => {
            let value = provenance::provenance_record_value(&provenance::ProvenanceRecordInput {
                artifact_ref: &artifact_ref,
                trust_state: &trust_state,
                source_refs: &source_refs,
                dependency_closure_ref: &dependency_closure_ref,
                toolchain_refs: &toolchain_refs,
                builder_ref: &builder_ref,
                review_refs: &review_refs,
                test_refs: &test_refs,
                source_gate_refs: &source_gate_refs,
                policy_refs: &policy_refs,
                build_record_refs: &build_record_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance record ref={reference} artifact={artifact_ref} trust_state={trust_state}"),
            );
            Ok(())
        }
        ProvenanceCommand::Fixture { artifact_ref, out } => {
            let value = provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance fixture ref={reference} artifact={artifact_ref} trust_state=reviewed"),
            );
            Ok(())
        }
        ProvenanceCommand::Evaluate {
            operation,
            profile,
            artifact_ref,
            provenance_paths,
            build_verification_paths,
            prior_diagnostics,
            receipt_out,
        } => {
            let mut provenance_values = input::BoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance evidence");
            for path in provenance_paths {
                provenance_values.push(read_preserves_file(&path)?)?;
            }
            let provenance_values = provenance_values.into_vec();
            let mut build_verification_values =
                input::BoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance build verification evidence");
            for path in build_verification_paths {
                build_verification_values.push(read_preserves_file(&path)?)?;
            }
            let build_verification_values = build_verification_values.into_vec();
            let evaluation = provenance::evaluate_provenance(&provenance::ProvenanceEvaluationInput {
                operation: &operation,
                profile: &profile,
                artifact_ref: &artifact_ref,
                provenance_values: &provenance_values,
                build_verification_values: &build_verification_values,
                prior_diagnostics: &prior_diagnostics,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "provenance decision={} operation={} artifact={} receipt={} matched={}",
                    evaluation.decision,
                    operation,
                    artifact_ref,
                    evaluation.receipt_ref,
                    evaluation.matched_record_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        ProvenanceCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", provenance::provenance_summary(&value)?);
            Ok(())
        }
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_optional_preserves(out: Option<&PathBuf>, value: &preserves::IOValue) -> Result<bool> {
    if let Some(path) = out {
        write_file(path, &to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", to_text(value)?);
        Ok(false)
    }
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
