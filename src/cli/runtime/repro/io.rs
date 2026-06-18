pub(crate) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(crate) fn read_preserves_file_with_failure(
    path: &std::path::Path,
    failure_out: Option<&std::path::PathBuf>,
    phase: &'static str,
) -> molten::error::Result<preserves::IOValue> {
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

pub(crate) fn write_optional_failure(
    path: Option<&std::path::PathBuf>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> molten::error::Result<()> {
    let failure = molten::harness::failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

pub(crate) fn write_optional_artifact_failure(
    path: Option<&std::path::PathBuf>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    artifact_value: &preserves::IOValue,
) -> molten::error::Result<()> {
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

pub(crate) fn emit_verify_receipt(
    path: Option<&std::path::PathBuf>,
    receipt: &preserves::IOValue,
) -> molten::error::Result<()> {
    emit_named_receipt(path, "repro verify receipt", receipt)
}

pub(crate) fn emit_named_receipt(
    path: Option<&std::path::PathBuf>,
    label: &str,
    receipt: &preserves::IOValue,
) -> molten::error::Result<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn emit_failure(path: Option<&std::path::PathBuf>, failure: &preserves::IOValue) -> molten::error::Result<()> {
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

pub(crate) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
