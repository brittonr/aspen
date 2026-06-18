use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::error::Result;

pub(crate) fn emit_analysis(value: &preserves::IOValue, out: Option<&PathBuf>) -> Result<()> {
    let text = molten::preserves_rail::to_text(value)?;
    write_optional_output(out, &text)
}

pub(crate) fn emit_job_analysis(value: &preserves::IOValue, out: Option<&PathBuf>) -> Result<()> {
    emit_analysis(value, out)
}

pub(crate) fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    molten::preserves_rail::parse_text(&fs::read_to_string(path)?)
}

pub(crate) fn read_preserves_files(paths: &[PathBuf]) -> Result<Vec<preserves::IOValue>> {
    let mut values = super::core::Items::new(super::JOB_CLI_EVIDENCE_LIMIT, "Preserves input files");
    for path in paths {
        values.push(read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}

pub(crate) fn values_canonical_refs(values: &[preserves::IOValue]) -> Result<Vec<String>> {
    let mut refs = super::core::Items::new(super::JOB_CLI_EVIDENCE_LIMIT, "Preserves input refs");
    for value in values {
        refs.push(molten::preserves_rail::canonical_hash(value)?)?;
    }
    Ok(refs.into_vec())
}

pub(crate) fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
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

pub(crate) fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index:02}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

pub(crate) fn write_optional_output(out: Option<&PathBuf>, contents: &str) -> Result<()> {
    if let Some(path) = out {
        write_file(path, contents)?;
    } else {
        println!("{contents}");
    }
    Ok(())
}

pub(crate) fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn content_arg(value: &str, label: &str) -> Result<molten::job_dag::JobContentRef> {
    let parts = value.split('@').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "job {label} must be formatted as <content-ref>@<size>@<format>[@<schema-ref>]"
        )));
    }
    let size = parts[1].parse::<u64>().map_err(|error| {
        molten::error::MoltenError::invalid_harness(format!("job {label} size is invalid: {error}"))
    })?;
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

pub(crate) fn synthetic_ref(kind: &str, label: &str) -> Result<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("job-cli-ref", vec![
        molten::preserves_rail::string(kind),
        molten::preserves_rail::string(label),
    ]))
}
