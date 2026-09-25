
fn write_immutable_bytes(
    root: &CapabilityChunkRoot,
    path: &StorePath,
    bytes: &[u8],
    expected_ref: &str,
    parser: fn(&[u8]) -> Result<IoValue>,
) -> Result<()> {
    if root.root().try_exists(path)? {
        let existing = root.root().read(path)?;
        let existing_value = parser(&existing)?;
        let existing_ref = canonical_hash(&existing_value)?;
        if existing_ref != expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "immutable content path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        root.root().write(path, bytes)?;
    }
    Ok(())
}
