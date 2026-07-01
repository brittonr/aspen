type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type IoValue = preserves::IOValue;
type MoltenError = molten::error::MoltenError;
type Result<T> = molten::error::Result<T>;

fn failure_value(phase: &str, error: &MoltenError, diagnostics: Vec<IoValue>) -> IoValue {
    molten::harness::failure_value(phase, error, diagnostics)
}

fn replay_report_value(report_value: &IoValue) -> Result<molten::harness::ReplayOutcome> {
    molten::harness::replay_report_value(report_value)
}

fn report_failure_value(phase: &str, error: &MoltenError, report_value: &IoValue) -> Result<IoValue> {
    molten::harness::report_failure_value(phase, error, report_value)
}

fn run_suite_value(value: &IoValue) -> Result<molten::harness::HarnessRun> {
    molten::harness::run_suite_value(value)
}

fn suite_failure_value(phase: &str, error: &MoltenError, suite_value: &IoValue) -> Result<IoValue> {
    molten::harness::suite_failure_value(phase, error, suite_value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    molten::preserves_rail::canonical_hash(value)
}

fn parse_text(text: &str) -> Result<IoValue> {
    molten::preserves_rail::parse_text(text)
}

fn to_text(value: &IoValue) -> Result<String> {
    molten::preserves_rail::to_text(value)
}

pub(crate) fn run_harness_suite_command(suite: PathBuf, report_out: Option<PathBuf>) -> Result<()> {
    let suite_text = match std::fs::read_to_string(&suite).map_err(MoltenError::from) {
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
) -> Result<IoValue> {
    let text = match std::fs::read_to_string(path).map_err(MoltenError::from) {
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
    diagnostics: Option<Vec<IoValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn write_optional_suite_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    suite_value: &IoValue,
) -> Result<()> {
    let failure = suite_failure_value(phase, error, suite_value)?;
    emit_failure(path, &failure)
}

fn write_optional_report_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    report_value: &IoValue,
) -> Result<()> {
    let failure = report_failure_value(phase, error, report_value)?;
    emit_failure(path, &failure)
}

fn emit_failure(path: Option<&PathBuf>, failure: &IoValue) -> Result<()> {
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
        std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(MoltenError::from)
}
