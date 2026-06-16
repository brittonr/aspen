use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::evidence::signed_receipt_summary;
use molten::harness::failure_summary;
use molten::harness::failure_value;
use molten::harness::gate_receipt_summary;
use molten::harness::replay_report_value;
use molten::harness::report_failure_value;
use molten::harness::report_summary;
use molten::harness::repro_bundle_summary;
use molten::harness::repro_verify_receipt_summary;
use molten::harness::validate_report_value;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::secrets;

use crate::cli_remote;

#[derive(Debug, Subcommand)]
pub(crate) enum ReportCommand {
    Show {
        report: PathBuf,
    },
    Validate {
        report: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
}

pub(crate) fn run_report_command(command: ReportCommand) -> Result<()> {
    match command {
        ReportCommand::Show { report } => {
            let report_value = read_preserves_file(&report)?;
            println!("{}", report_show_summary(&report_value)?);
            Ok(())
        }
        ReportCommand::Validate { report, failure_out } => {
            let report_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "validate")?;
            let validation = match validate_report_value(&report_value) {
                Ok(validation) => validation,
                Err(error) => {
                    write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            let replay = match replay_report_value(&report_value) {
                Ok(replay) => replay,
                Err(error) => {
                    write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            println!(
                "report validate ok report={} suite={} observations={} final_state={} replay_actual={}",
                validation.report_ref,
                validation.suite_ref,
                validation.observations,
                validation.final_state_hash,
                replay.actual_report_ref
            );
            Ok(())
        }
    }
}

fn report_show_summary(report_value: &preserves::IOValue) -> Result<String> {
    let report_error = match report_summary(report_value) {
        Ok(summary) => return Ok(summary),
        Err(error) => error,
    };
    if let Ok(summary) = failure_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = repro_bundle_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = gate_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = repro_verify_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = signed_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = secrets::fixture_report_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = secrets::secrets_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = cli_remote::remote_dataspace_gate_summary(report_value) {
        return Ok(summary);
    }
    Err(report_error)
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
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

fn write_optional_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn write_optional_report_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    report_value: &preserves::IOValue,
) -> Result<()> {
    let failure = report_failure_value(phase, error, report_value)?;
    emit_failure(path, &failure)
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
