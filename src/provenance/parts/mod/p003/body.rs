
fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a string")))
}

fn record_ref(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let value = record_string(value, tag)?;
    validate_ref(&value, tag)?;
    Ok(value)
}

fn record_ref_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_PROVENANCE_REFS, tag)?;
    items.iter().map(|item| required_ref(item, tag)).collect()
}

fn record_string_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_PROVENANCE_REFS, tag)?;
    items
        .iter()
        .map(|item| {
            item.as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} item must be a string")))
        })
        .collect()
}

fn required_ref(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let value = value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} ref must be a string")))?;
    validate_ref(&value, tag)?;
    Ok(value)
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn validate_refs(refs: &[String], context: &str) -> Result<()> {
    ensure_ref_bound(refs.len(), MAX_PROVENANCE_REFS, context)?;
    for reference in refs {
        validate_ref(reference, context)?;
    }
    Ok(())
}

fn ensure_ref_bound(len: usize, max: usize, context: &str) -> Result<()> {
    if len <= max {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("too many {context}: {len} > {max}")))
    }
}

fn validate_ref(value: &str, context: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("invalid {context}: expected canonical content ref: {error}"))
    })
}

fn synthetic_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("provenance-synthetic-ref-v1", vec![string(kind), string(label)]))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/provenance/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/provenance/parts/mod/tests/m000/p001/body.rs"));
}
