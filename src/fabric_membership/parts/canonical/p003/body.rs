
fn resource_value(resources: ResourceAmount) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-resource-amount-v1", vec![
        field("cpu-millis", crate::preserves_rail::u64_value(resources.cpu_millis)),
        field("memory-bytes", crate::preserves_rail::u64_value(resources.memory_bytes)),
        field("storage-bytes", crate::preserves_rail::u64_value(resources.storage_bytes)),
    ])
}

fn validate_evidence_ref(label: &str, value: &str) -> crate::error::Result<()> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let is_valid = value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    });
    if is_valid {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("{label} ref is malformed")))
    }
}

fn checked_increment(value: u64) -> crate::error::Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("membership readback count overflow"))
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::sequence(Vec::new()),
        |value| crate::preserves_rail::sequence(vec![crate::preserves_rail::string(value)]),
    )
}

fn sorted_strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    strings_value(values)
}

fn checks(values: &[&str]) -> preserves::IOValue {
    strings_value(values.iter().copied())
}
