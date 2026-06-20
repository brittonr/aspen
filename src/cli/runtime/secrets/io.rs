pub(super) fn write_indexed_values(
    out: &std::path::Path,
    prefix: &str,
    values: &[preserves::IOValue],
) -> molten::error::Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
