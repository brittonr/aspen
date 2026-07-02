
fn finding_counts(status: Option<&StatusArtifact>, lint_counts: &OrderedMap<String, u64>) -> FindingCounts {
    let mut counts = FindingCounts::default();
    if let Some(status) = status {
        counts.total = status.total_findings;
        counts.warnings = status.warning_findings;
        counts.errors = status.error_findings;
        counts.autofixable = status.autofixable_findings;
    }
    counts.critical = CRITICAL_LINTS.iter().map(|name| lint_counts.get(*name).copied().unwrap_or(0)).sum();
    counts.uncovered = if counts.total == 0 { 0 } else { counts.total };
    counts
}

fn counts_value(counts: &FindingCounts) -> IoValue {
    record("counts", vec![
        record("findings", vec![u64_value(counts.total)]),
        record("warnings", vec![u64_value(counts.warnings)]),
        record("errors", vec![u64_value(counts.errors)]),
        record("autofixable", vec![u64_value(counts.autofixable)]),
        record("critical", vec![u64_value(counts.critical)]),
        record("uncovered", vec![u64_value(counts.uncovered)]),
    ])
}

fn checks_value(checks: &[Check]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|check| record("check", vec![string(check.name), string(check.status)])).collect(),
    )])
}

fn optional_ref(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn insert_bounded<K: Ord, V>(
    values: &mut OrderedMap<K, V>,
    key: K,
    value: V,
    maximum: usize,
    label: &str,
) -> Result<()> {
    if !values.contains_key(&key) {
        let total = values
            .len()
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
        ensure_count_at_most(total, maximum, label)?;
    }
    values.insert(key, value);
    Ok(())
}

fn push_finding_bounded(values: &mut impl crate::bounded::VecSink<FindingEntry>, value: FindingEntry) -> bool {
    if values.item_count() >= MAX_OCTET_FINDING_ENTRIES {
        return false;
    }
    values.push_item(value);
    true
}

fn push_token_bounded(
    values: &mut impl crate::bounded::VecSink<String>,
    value: String,
) -> std::result::Result<(), String> {
    if values.item_count() >= MAX_OCTET_COMMAND_TOKENS {
        return Err("octet command scope argument count exceeds command token bound".to_string());
    }
    values.push_item(value);
    Ok(())
}

fn push_check(checks: &mut impl crate::bounded::VecSink<Check>, name: &'static str, pass: bool) {
    checks.push_item(Check {
        name,
        status: if pass { "pass" } else { "fail" },
    });
}

fn push_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) {
    if diagnostics.item_count() < MAX_DIAGNOSTICS {
        diagnostics.push_item(diagnostic);
    }
}

fn bytes_ref(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}

fn b3_ref_from_bytes(bytes: &[u8]) -> Result<String> {
    let reference = content_ref_from_bytes(bytes);
    Ok(format!("b3:{}", content_ref_hex(&reference)?))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/octet/parts/gate/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/octet/parts/gate/tests/m000/p001/body.rs"));
}
