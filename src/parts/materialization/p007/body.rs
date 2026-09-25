
fn remove_tree_if_present(dir: &cap_std::fs::Dir, path: &std::path::Path) -> crate::error::Result<()> {
    match entry_kind(dir, path)? {
        None => Ok(()),
        Some(MaterializationMemberKind::Directory) => dir.remove_dir_all(path).map_err(crate::error::MoltenError::from),
        Some(_) => Err(invalid("materialization stage path is not a real directory")),
    }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/parts/materialization/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/parts/materialization/tests/m000/p001/body.rs"));
}
