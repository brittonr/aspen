fn ensure_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_ITEMS, label)
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn decision_for(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    }
}

fn placeholder_ref() -> Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(b"missing"))
}

fn hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_u64(label: &'static str, value: u64) -> IoValue {
    record(label, vec![IoValue::new(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![sequence(values)])
}
