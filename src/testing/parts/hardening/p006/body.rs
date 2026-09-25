
fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_bound(values.len(), "string values")?;
    for value in values {
        validate_text("string value", value)?;
    }
    Ok(values.iter().map(string).collect())
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/testing/parts/hardening/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/testing/parts/hardening/tests/m000/p001/body.rs"));
}
