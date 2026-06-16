use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::harness::failure_value;
use molten::harness::replay_report_value;
use molten::harness::report_failure_value;
use molten::harness::run_suite_value;
use molten::harness::suite_failure_value;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

pub(crate) fn run_harness_suite_command(suite: PathBuf, report_out: Option<PathBuf>) -> Result<()> {
    let suite_text = match fs::read_to_string(&suite).map_err(MoltenError::from) {
        Ok(suite_text) => suite_text,
        Err(error) => {
            write_optional_failure(report_out.as_ref(), "preflight", &error, None)?;
            return Err(error);
        }
    };
    let suite_value = match parse_text(&suite_text) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_failure(report_out.as_ref(), "preflight", &error, None)?;
            return Err(error);
        }
    };
    let run = match run_suite_value(&suite_value) {
        Ok(run) => run,
        Err(error) => {
            let phase = run_failure_phase(&error);
            write_optional_suite_failure(report_out.as_ref(), phase, &error, &suite_value)?;
            return Err(error);
        }
    };
    let report_text = to_text(&run.report_value)?;
    if let Some(path) = report_out {
        write_file(&path, &report_text)?;
        println!("report {} written to {}", run.report_ref, path.display());
    } else {
        println!("{report_text}");
        eprintln!("report {}", run.report_ref);
    }
    Ok(())
}

pub(crate) fn run_harness_replay_command(report: PathBuf, failure_out: Option<PathBuf>) -> Result<()> {
    let report_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "replay")?;
    let replay = match replay_report_value(&report_value) {
        Ok(replay) => replay,
        Err(error) => {
            write_optional_report_failure(failure_out.as_ref(), "replay", &error, &report_value)?;
            return Err(error);
        }
    };
    println!(
        "replay ok expected={} actual={} final_state={}",
        replay.expected_report_ref, replay.actual_report_ref, replay.final_state_hash
    );
    Ok(())
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

fn run_failure_phase(error: &MoltenError) -> &'static str {
    match error {
        MoltenError::InvalidHarness(_) => "preflight",
        MoltenError::Io(_) | MoltenError::Preserves(_) | MoltenError::HarnessDivergence(_) => "execute",
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

fn write_optional_suite_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    suite_value: &preserves::IOValue,
) -> Result<()> {
    let failure = suite_failure_value(phase, error, suite_value)?;
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
