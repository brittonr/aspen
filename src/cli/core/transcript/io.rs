pub(super) fn read_transcript_input(
    path: &std::path::Path,
) -> molten::error::Result<molten::transcripts::TranscriptArtifact> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    if let Ok(value) = molten::preserves_rail::parse_text(&text)
        && let Ok(transcript) = molten::transcripts::parse_transcript_artifact(&value)
    {
        return Ok(transcript);
    }
    molten::transcripts::parse_markdown(&text, &molten::transcripts::TranscriptParseInput::default())
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn emit_named_receipt(
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

pub(super) fn write_optional_failure(
    path: Option<&std::path::PathBuf>,
    phase: &'static str,
    error: &molten::error::MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> molten::error::Result<()> {
    let failure = molten::harness::failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
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

pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
