
fn validate_adapter_operation(operation: &str) -> Result<()> {
    if matches!(operation, "start" | "verify" | "deny" | "shutdown") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node adapter operation {operation}")))
    }
}

fn validate_control_operation(operation: &str) -> Result<()> {
    if matches!(operation, "status" | "install" | "run" | "gate" | "shutdown") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node control operation {operation}")))
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node runtime decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(actual: usize, maximum: usize, field: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{field} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, field: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{field} count overflow")))?;
    ensure_count_at_most(total, maximum, field)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn status(ok: bool) -> &'static str {
    if ok { "pass" } else { "fail" }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/runtime/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/runtime/tests/m000/p001/body.rs"));
}
