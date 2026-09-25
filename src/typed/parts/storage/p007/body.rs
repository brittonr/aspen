
struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    storage_ref: Option<&'a str>,
    namespace: Option<&'a str>,
    key: Option<&'a str>,
    schema_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    effect: &'a EffectEvidence,
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

struct DenialReceiptValueInput<'a> {
    operation: &'a str,
    storage_ref: Option<&'a str>,
    namespace: Option<&'a str>,
    key: Option<&'a str>,
    schema_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    reason: String,
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

struct PayloadParts {
    value: IoValue,
    details: Vec<IoValue>,
}

struct PersistInput<'a> {
    root: &'a Path,
    storage_key: &'a str,
    storage_ref: &'a str,
    typed_ref_value: &'a IoValue,
    value_ref: &'a str,
    value_bytes: &'a [u8],
    receipt_value: &'a IoValue,
}

struct EntryInput<'a> {
    input: &'a PutInput,
    schema_ref: &'a str,
    value_ref: &'a str,
    payload: &'a IoValue,
    revision: u64,
    effect_handle_ref: &'a str,
}
