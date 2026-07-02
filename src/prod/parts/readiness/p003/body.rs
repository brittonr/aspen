
fn is_pass(decision: &str) -> bool {
    decision == "pass"
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" | "degraded" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported production readiness decision {other}; expected pass, deny, degraded, unavailable, or skipped"
        ))),
    }
}

fn validate_source_gate_status(status: &str) -> Result<()> {
    match status {
        SOURCE_REMEDIATED_ZERO_STATUS | CONFIGURATION_CLEAN_CAVEAT_STATUS | "stale" | "missing" | "failed" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported production source gate status {other}; expected source-remediated-zero, configuration-clean-caveat, stale, missing, or failed"
        ))),
    }
}

fn validate_allowed_text(label: &str, value: &str, allowed: &[&str]) -> Result<()> {
    validate_text_field(label, value)?;
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported production {label} {value}; expected one of {}",
            allowed.join(", ")
        )))
    }
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("production readiness {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_text_slice(label: &str, values: &[String]) -> Result<()> {
    string_values(label, values).map(|_| ())
}

fn validate_diagnostics(values: &[String]) -> Result<()> {
    string_values("diagnostic", values).map(|_| ())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_PROD_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} ref count {} exceeds bound {MAX_PROD_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid production readiness {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn require_pass_refs(label: &str, refs: &[String], decision: &str) -> Result<()> {
    validate_ref_slice(label, refs)?;
    if is_pass(decision) && refs.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label} ref"
        )))
    } else {
        Ok(())
    }
}

fn require_non_empty_refs(label: &str, refs: &[String]) -> Result<()> {
    validate_ref_slice(label, refs)?;
    if refs.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "production readiness receipt requires at least one {label} ref"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_texts(label: &str, values: &[String], decision: &str) -> Result<()> {
    validate_text_slice(label, values)?;
    if is_pass(decision) && values.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label}"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_coverage(label: &str, groups: &[&[String]], decision: &str) -> Result<()> {
    if is_pass(decision) && groups.iter().all(|group| group.is_empty()) {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires {label} coverage"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_metric_bound(label: &str, actual: u64, maximum: u64, decision: &str) -> Result<()> {
    if is_pass(decision) && actual > maximum {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness {label} {actual} exceeds bound {maximum}"
        )))
    } else {
        Ok(())
    }
}

fn ref_values(label: &str, refs: &[String]) -> Result<Vec<IoValue>> {
    validate_ref_slice(label, refs)?;
    Ok(refs.iter().map(string).collect())
}

fn string_values(label: &str, values: &[String]) -> Result<Vec<IoValue>> {
    if values.len() > MAX_PROD_TEXTS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} count {} exceeds bound {MAX_PROD_TEXTS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/prod/parts/readiness/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/prod/parts/readiness/tests/m000/p001/body.rs"));
}
