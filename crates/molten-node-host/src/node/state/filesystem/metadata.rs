//! Regular-file checks before and after acquiring a leaf handle.
pub(super) fn existing_leaf(
    dir: &cap_std::fs::Dir,
    leaf: &std::ffi::OsStr,
    path: &std::path::Path,
    operation: &str,
) -> crate::error::Result<()> {
    match dir.symlink_metadata(leaf) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(crate::node_state::invalid(format!(
                    "node state {operation} leaf {} must be a regular file",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    }
    Ok(())
}

pub(super) fn opened_file(
    file: &cap_std::fs::File,
    path: &std::path::Path,
    operation: &str,
) -> crate::error::Result<cap_std::fs::Metadata> {
    let metadata = file.metadata().map_err(crate::error::MoltenError::from)?;
    if !metadata.is_file() {
        return Err(crate::node_state::invalid(format!(
            "node state {operation} leaf {} changed away from a regular file",
            path.display()
        )));
    }
    Ok(metadata)
}
