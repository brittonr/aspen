
fn denied_trace_only(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "trace-only",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["production trace-only cache value cannot be returned as semantic output".to_string()],
        checks: &[("trace-only-not-semantic", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache trace-only denial: {}",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_stale(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "stale-deny",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["policy-current refs do not match current request refs".to_string()],
        checks: &[("policy-current-revalidation", "fail"), ("stale-deny", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache stale policy-current entry denied: {}",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_invalid_hit(
    root: &Path,
    key_ref: &str,
    value_ref: &str,
    refs: &[String],
    diagnostics: &[String],
) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "invalid-hit-deny",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics,
        checks: &[("cache-hit-validity", "fail"), ("stale-deny", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache hit denied by validity checks: {}",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn hit_receipt(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<IoValue> {
    store_and_return_receipt(root, &ReceiptValueInput {
        operation: "hit",
        decision: "pass",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &[],
        checks: &[("cache-hit", "pass"), ("output-integrity", "pass")],
    })
}
