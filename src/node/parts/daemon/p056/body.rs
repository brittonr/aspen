
fn record_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<u64> {
    record_string(value, tag)?.parse::<u64>().map_err(|error| {
        MoltenError::invalid_harness(format!("{tag} must contain an unsigned integer string: {error}"))
    })
}

fn validate_ingress_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_ingress_ref(reference, label)?;
    }
    Ok(())
}
