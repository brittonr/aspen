
pub fn strict_canonical_decode_with_ref(
    bytes: &[u8],
    expected_ref: &str,
    boundary: &str,
) -> Result<StrictCanonicalDecode> {
    let expected = ContentRef::parse(expected_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{boundary} expected content ref is invalid: {error}"))
    })?;
    let decoded = strict_canonical_decode(bytes)?;
    if decoded.value_ref != expected {
        return Err(MoltenError::invalid_harness(format!(
            "{boundary} strict canonical decode ref mismatch: expected {}, got {}",
            expected, decoded.value_ref
        )));
    }
    Ok(decoded)
}

pub fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    Ok(strict_canonical_decode(bytes)?.value)
}

pub fn canonical_hash(value: &IoValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(content_ref_from_bytes(&bytes))
}

pub fn content_ref_from_bytes(bytes: &[u8]) -> String {
    molten_preserves_core::identity::ContentRef::of_bytes(bytes).into_string()
}

pub fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    molten_preserves_core::identity::ContentRef::from_hash(hash).into_string()
}

pub fn canonical_content_ref(value: &IoValue) -> Result<ContentRef> {
    ContentRef::parse(&canonical_hash(value)?)
}

// The typed ids, closed vocabularies and check records are the kernel's; their `parse` denials are
// `ViewError`s that convert into the same `Failure::InvalidHarness` messages as before.
pub use molten_preserves_core::view::CheckStatus;
pub use molten_preserves_core::view::Decision;
pub use molten_preserves_core::view::OperationId;
pub use molten_preserves_core::view::ParsedCheck;
pub use molten_preserves_core::view::ProfileId;
pub use molten_preserves_core::view::ReplayClass;
pub use molten_preserves_core::view::SchemaId;
pub use molten_preserves_core::view::StableId;

type ViewError = molten_preserves_core::view::ViewError;

pub fn validate_stable_id(value: &str, label: &str) -> Result<()> {
    Ok(molten_preserves_core::view::validate_stable_id(value, label)?)
}

pub fn symbol(name: &'static str) -> IoValue {
    IoValue::symbol(name)
}

pub fn string(value: impl AsRef<str>) -> IoValue {
    IoValue::new(value.as_ref().to_owned())
}

pub fn u64_value(value: u64) -> IoValue {
    IoValue::new(value)
}

pub fn bool_value(value: bool) -> IoValue {
    IoValue::new(value)
}

pub fn sequence(values: Vec<IoValue>) -> IoValue {
    IoValue::new(values)
}

pub fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    IoValue::record(symbol(label), fields)
}

pub fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    IoValue::from(value.clone())
}

// The `IOValue` parsers below keep their own traversal (see `molten_preserves_core::view` for why)
// and take every denial from the kernel's `ViewError`, so both share one set of messages; the
// `reader_parity` property holds their accept/deny decisions and results identical.
// r[impl molten.preserves_rail_toolkit.parser_builders]
// r[impl molten.preserves_rail_toolkit.negative_shapes]
pub fn simple_record_fields<'a>(
    value: &'a IoValue,
    label: &str,
    arity: u64,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    let expected_arity = crate::bounded::usize_from_u64(arity, "record arity")?;
    value.collect_simple_record(label, Some(expected_arity)).ok_or_else(|| {
        MoltenError::from(ViewError::RecordShape {
            label: label.to_owned(),
            arity,
        })
    })
}

pub fn required_string_field(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::from(ViewError::ExpectedString { field: field.to_owned() }))
}

pub fn required_content_ref(value: &Value<IoValue>, field: &str) -> Result<ContentRef> {
    let reference = required_string_field(value, field)?;
    match molten_preserves_core::identity::validate_ref(&reference) {
        Ok(()) => Ok(ContentRef(reference)),
        Err(error) => Err(MoltenError::from(ViewError::ContentRef {
            field: field.to_owned(),
            value: reference,
            error,
        })),
    }
}

pub fn required_content_ref_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    Ok(required_content_ref(value, field)?.into_string())
}

/// Decodes the single canonical `Option<ContentRef>` encoding: `<none>` or `<some "blake3:...">`. A bare string is a
/// second encoding of `Some` and is rejected so one logical value has one canonical identity.
// r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
pub fn optional_content_ref(value: &Value<IoValue>, field: &str) -> Result<Option<ContentRef>> {
    identity_bridge::decode_optional_ref(&value_to_iovalue(value)).map_err(|issue| {
        let field = field.to_owned();
        match issue {
            identity_bridge::OptionalRefIssue::Shape => MoltenError::from(ViewError::OptionalRefShape { field }),
            identity_bridge::OptionalRefIssue::NotString => MoltenError::from(ViewError::ExpectedString { field }),
            identity_bridge::OptionalRefIssue::InvalidRef { value, error } => {
                MoltenError::from(ViewError::ContentRef { field, value, error })
            }
            identity_bridge::OptionalRefIssue::Conversion(error) => error,
        }
    })
}
