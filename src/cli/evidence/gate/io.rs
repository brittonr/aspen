type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

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

pub(super) fn write_optional_artifact_failure(
    path: Option<&FilePath>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    artifact_value: &preserves::IOValue,
) -> Outcome<()> {
    let artifact_ref = molten::preserves_rail::canonical_hash(artifact_value)?;
    write_optional_failure(
        path,
        phase,
        error,
        Some(vec![
            molten::preserves_rail::record("artifact-ref", vec![molten::preserves_rail::string(&artifact_ref)]),
            molten::preserves_rail::record("artifact", vec![artifact_value.clone()]),
        ]),
    )
}

pub(super) fn write_optional_failure(
    path: Option<&FilePath>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Outcome<()> {
    let failure = molten::harness::failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

pub(super) fn emit_gate_receipt(path: Option<&FilePath>, receipt: &preserves::IOValue) -> Outcome<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("gate receipt {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("gate receipt {receipt_ref}");
    }
    Ok(())
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
