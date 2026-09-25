
fn required_string(value: &Value<preserves::IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(actual: usize, label: &str) -> Result<()> {
    if actual <= MAX_RUNTIME_ITEMS {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {MAX_RUNTIME_ITEMS}")))
    }
}
