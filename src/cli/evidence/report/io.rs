type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn read_preserves_file_with_failure(
    path: &std::path::Path,
    failure_out: Option<&FilePath>,
    phase: &'static str,
) -> Outcome<preserves::IOValue> {
    let text = match std::fs::read_to_string(path).map_err(molten::error::MoltenError::from) {
        Ok(text) => text,
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            return Err(error);
        }
    };
    match molten::preserves_rail::parse_text(&text) {
        Ok(value) => Ok(value),
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            Err(error)
        }
    }
}

fn write_optional_failure(
    path: Option<&FilePath>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Outcome<()> {
    let failure = molten::harness::failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

pub(super) fn write_optional_report_failure(
    path: Option<&FilePath>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    report_value: &preserves::IOValue,
) -> Outcome<()> {
    let failure = molten::harness::report_failure_value(phase, error, report_value)?;
    emit_failure(path, &failure)
}

fn emit_failure(path: Option<&FilePath>, failure: &preserves::IOValue) -> Outcome<()> {
    let failure_text = molten::preserves_rail::to_text(failure)?;
    let failure_ref = molten::preserves_rail::canonical_hash(failure)?;
    if let Some(path) = path {
        write_file(path, &failure_text)?;
        eprintln!("failure {failure_ref} written to {}", path.display());
    } else {
        println!("{failure_text}");
        eprintln!("failure {failure_ref}");
    }
    Ok(())
}

fn write_file(path: &std::path::Path, contents: &str) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
