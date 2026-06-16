use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::harness::failure_value;
use molten::harness::gate_check_value;
use molten::harness::gate_receipt_value;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;

#[derive(Debug, Subcommand)]
pub(crate) enum GateCommand {
    Check {
        artifact: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_gate_command(command: GateCommand) -> Result<()> {
    match command {
        GateCommand::Check {
            artifact,
            failure_out,
            receipt_out,
        } => {
            let artifact_value = read_preserves_file_with_failure(&artifact, failure_out.as_ref(), "validate")?;
            let check = match gate_check_value(&artifact_value) {
                Ok(check) => check,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "validate", &error, &artifact_value)?;
                    return Err(error);
                }
            };
            let receipt = gate_receipt_value(&check);
            if let Err(error) = emit_gate_receipt(receipt_out.as_ref(), &receipt) {
                write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
                return Err(error);
            }
            Ok(())
        }
    }
}

fn read_preserves_file_with_failure(
    path: &Path,
    failure_out: Option<&PathBuf>,
    phase: &'static str,
) -> Result<preserves::IOValue> {
    let text = match fs::read_to_string(path).map_err(MoltenError::from) {
        Ok(text) => text,
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            return Err(error);
        }
    };
    match parse_text(&text) {
        Ok(value) => Ok(value),
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            Err(error)
        }
    }
}

fn write_optional_artifact_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    artifact_value: &preserves::IOValue,
) -> Result<()> {
    let artifact_ref = canonical_hash(artifact_value)?;
    write_optional_failure(
        path,
        phase,
        error,
        Some(vec![
            record("artifact-ref", vec![string(&artifact_ref)]),
            record("artifact", vec![artifact_value.clone()]),
        ]),
    )
}

fn write_optional_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn emit_gate_receipt(path: Option<&PathBuf>, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("gate receipt {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("gate receipt {receipt_ref}");
    }
    Ok(())
}

fn emit_failure(path: Option<&PathBuf>, failure: &preserves::IOValue) -> Result<()> {
    let failure_text = to_text(failure)?;
    let failure_ref = canonical_hash(failure)?;
    if let Some(path) = path {
        write_file(path, &failure_text)?;
        eprintln!("failure {failure_ref} written to {}", path.display());
    } else {
        println!("{failure_text}");
        eprintln!("failure {failure_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
