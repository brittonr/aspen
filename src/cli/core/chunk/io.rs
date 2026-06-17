pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
