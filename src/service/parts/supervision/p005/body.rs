
fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, label: &str) -> Result<()> {
    if let Some(reference) = reference {
        require_ref(reference, label)
    } else {
        Ok(())
    }
}

fn validate_service_id(value: &str, label: &str) -> Result<()> {
    if value.starts_with("svc:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected svc: service id for {label}, got {value}")))
    }
}

fn required_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = required_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
    })
}

fn required_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn synthetic_ref(label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("service-supervision-fixture-ref", vec![
        crate::preserves_rail::string(label),
    ]))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/service/parts/supervision/tests/m000/p000/body.rs"));
}
