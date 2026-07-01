type CliError = molten::error::MoltenError;
type FsPath = std::path::Path;
type FsPathBuf = std::path::PathBuf;
type IoValue = preserves::IOValue;

pub(crate) fn emit_analysis(value: &IoValue, out: Option<&FsPathBuf>) -> Result<(), CliError> {
    let text = molten::preserves_rail::to_text(value)?;
    write_optional_output(out, &text)
}

pub(crate) fn emit_job_analysis(value: &IoValue, out: Option<&FsPathBuf>) -> Result<(), CliError> {
    emit_analysis(value, out)
}

pub(crate) fn read_preserves_file(path: &FsPath) -> Result<IoValue, CliError> {
    molten::preserves_rail::parse_text(&std::fs::read_to_string(path)?)
}

pub(crate) fn read_preserves_files(paths: &[FsPathBuf]) -> Result<Vec<IoValue>, CliError> {
    let mut values = super::core::Items::new(super::JOB_CLI_EVIDENCE_LIMIT, "Preserves input files");
    for path in paths {
        values.push(read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}

pub(crate) fn values_canonical_refs(values: &[IoValue]) -> Result<Vec<String>, CliError> {
    let mut refs = super::core::Items::new(super::JOB_CLI_EVIDENCE_LIMIT, "Preserves input refs");
    for value in values {
        refs.push(molten::preserves_rail::canonical_hash(value)?)?;
    }
    Ok(refs.into_vec())
}

pub(crate) fn emit_named_receipt(path: Option<&FsPathBuf>, label: &str, receipt: &IoValue) -> Result<(), CliError> {
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

pub(crate) fn write_indexed_values(out: &FsPath, prefix: &str, values: &[IoValue]) -> Result<(), CliError> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index:02}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

pub(crate) fn write_optional_output(out: Option<&FsPathBuf>, contents: &str) -> Result<(), CliError> {
    if let Some(path) = out {
        write_file(path, contents)?;
    } else {
        println!("{contents}");
    }
    Ok(())
}

pub(crate) fn write_file(path: &FsPath, contents: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn content_arg(value: &str, label: &str) -> Result<molten::job_dag::JobContentRef, CliError> {
    let parts = value.split('@').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(CliError::invalid_harness(format!(
            "job {label} must be formatted as <content-ref>@<size>@<format>[@<schema-ref>]"
        )));
    }
    let size = parts[1]
        .parse::<u64>()
        .map_err(|error| CliError::invalid_harness(format!("job {label} size is invalid: {error}")))?;
    let schema_ref = if parts.len() == 4 {
        Some(parts[3].to_string())
    } else {
        None
    };
    Ok(molten::job_dag::JobContentRef {
        content_ref: parts[0].to_string(),
        size,
        format: parts[2].to_string(),
        schema_ref,
    })
}

pub(crate) fn synthetic_ref(kind: &str, label: &str) -> Result<String, CliError> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("job-cli-ref", vec![
        molten::preserves_rail::string(kind),
        molten::preserves_rail::string(label),
    ]))
}
