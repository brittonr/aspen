
fn pinned_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let pins = root.join("pins");
    if !pins.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in std::fs::read_dir(pins).map_err(crate::error::MoltenError::from)? {
        let entry = entry.map_err(crate::error::MoltenError::from)?;
        if entry.file_type().map_err(crate::error::MoltenError::from)?.is_file() {
            let reference = std::fs::read_to_string(entry.path()).map_err(crate::error::MoltenError::from)?;
            crate::preserves_rail::validate_content_ref(&reference).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!(
                    "ledger pin file contains invalid content ref {reference}: {error}"
                ))
            })?;
            push_bounded(&mut refs, reference, MAX_SCAN_ENTRIES, "ledger pinned refs")?;
        }
    }
    Ok(refs)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ledger/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ledger/parts/mod/tests/m000/p001/body.rs"));
}
