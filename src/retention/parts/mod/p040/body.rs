
pub fn read_receipt_with_root(root: &CapabilityRetentionRoot, receipt_ref: &str) -> Result<Receipt> {
    require_ref(receipt_ref, "retention receipt ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(RECEIPT_DIR, receipt_ref)?)?;
    parse_receipt(&value)
}

pub fn read_tombstone(root: &Path, tombstone_ref: &str) -> Result<Tombstone> {
    let root = open_capability_retention_root(root)?;
    read_tombstone_with_root(&root, tombstone_ref)
}

pub fn read_tombstone_with_root(root: &CapabilityRetentionRoot, tombstone_ref: &str) -> Result<Tombstone> {
    require_ref(tombstone_ref, "retention tombstone ref")?;
    let value = read_store_value_with_root(root, &capability_ref_path(TOMBSTONE_DIR, tombstone_ref)?)?;
    let tombstone = parse_tombstone(&value)?;
    if tombstone.tombstone_ref != tombstone_ref {
        return Err(MoltenError::invalid_harness("stored retention tombstone ref mismatch"));
    }
    Ok(tombstone)
}
