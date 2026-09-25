
fn validate_kind(kind: &str) -> Result<()> {
    validate_non_empty(kind, "artifact kind")?;
    if kind.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "artifact kind {kind} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_pointer_kind(kind: &str) -> Result<()> {
    if matches!(kind, "name" | "alias" | "tag" | "channel") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported artifact pointer kind {kind}; expected name, alias, tag, or channel"
        )))
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_ARTIFACT_REF_LIST, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn extend_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: Vec<T>,
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    for item in incoming {
        values.push_item(item);
    }
    Ok(())
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("artifact registry redb index error: {error}"))
}
