
fn validate_no_advance_transition(state_ref: &str, transition: ReceiptTransitionInput<'_>) -> Result<()> {
    if transition.after_state_ref.is_some() || transition.preserved_state_ref != Some(state_ref) {
        return Err(MoltenError::invalid_harness("no-advance transition must bind preserved-state as receipt state"));
    }
    Ok(())
}

fn validate_read_consistency_mode(value: &str) -> Result<()> {
    match value {
        READ_CONSISTENCY_LINEARIZABLE | READ_CONSISTENCY_LOCAL_STALE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported coordination read consistency mode {value}"))),
    }
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_ref(value: &str, label: &str) -> Result<()> {
    validate_non_empty(value, label)?;
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_COORDINATION_REFS, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, maximum, label)
}

fn vec_len_u64<T>(values: &[T]) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination vector length overflow"))
}

fn set_len_u64<T>(values: &OrderedSet<T>) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination set length overflow"))
}

fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p003/body.rs"));
}
