//! Field readers over `PackedValue` records, mirroring the rail's `IOValue` parsers.

use alloc::borrow::ToOwned;

use crate::bounded::Count;

type String = alloc::string::String;
type Vec<T> = alloc::vec::Vec<T>;
type ViewError = super::ViewError;
type ContentRef = crate::identity::ContentRef;
type PackedValue = crate::model::PackedValue;

/// The fields of `<label ...>` with exactly `arity` fields.
// r[impl molten.preserves_rail_toolkit.parser_builders]
// r[impl molten.preserves_rail_toolkit.negative_shapes]
pub fn simple_record_fields<'a>(
    value: &'a PackedValue,
    label: &str,
    arity: u64,
) -> Result<&'a [PackedValue], ViewError> {
    arity.to_usize("record arity")?;
    value.symbol_record_fields(label, arity).ok_or_else(|| ViewError::RecordShape {
        label: label.to_owned(),
        arity,
    })
}

pub fn required_string_field(value: &PackedValue, field: &str) -> Result<String, ViewError> {
    required_str(value, field).map(str::to_owned)
}

pub fn required_content_ref(value: &PackedValue, field: &str) -> Result<ContentRef, ViewError> {
    parse_ref(required_str(value, field)?, field)
}

pub fn required_content_ref_string(value: &PackedValue, field: &str) -> Result<String, ViewError> {
    Ok(required_content_ref(value, field)?.into_string())
}

/// Decodes the single canonical `Option<ContentRef>` encoding: `<none>` or `<some "blake3:...">`.
/// A bare string is a second encoding of `Some` and is rejected.
// r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
pub fn optional_content_ref(value: &PackedValue, field: &str) -> Result<Option<ContentRef>, ViewError> {
    crate::identity::decode_optional_ref(value).map_err(|error| match error {
        crate::identity::OptionalRefError::Shape => ViewError::OptionalRefShape {
            field: field.to_owned(),
        },
        crate::identity::OptionalRefError::NotString => ViewError::ExpectedString {
            field: field.to_owned(),
        },
        crate::identity::OptionalRefError::InvalidRef(error) => ViewError::ContentRef {
            field: field.to_owned(),
            value: some_text(value).to_owned(),
            error,
        },
    })
}

pub fn optional_content_ref_string(value: &PackedValue, field: &str) -> Result<Option<String>, ViewError> {
    Ok(optional_content_ref(value, field)?.map(ContentRef::into_string))
}

pub fn required_sequence_field<'a>(value: &'a PackedValue, field: &str) -> Result<&'a [PackedValue], ViewError> {
    match value {
        PackedValue::Sequence(items) => Ok(items),
        _ => Err(ViewError::ExpectedSequence {
            field: field.to_owned(),
        }),
    }
}

/// The string in `<record_name "...">`.
pub fn record_string_field(value: &PackedValue, record_name: &str, field: &str) -> Result<String, ViewError> {
    required_string_field(single_field(value, record_name)?, field)
}

/// The content reference in `<record_name "blake3:...">`.
pub fn record_content_ref(value: &PackedValue, record_name: &str, field: &str) -> Result<ContentRef, ViewError> {
    required_content_ref(single_field(value, record_name)?, field)
}

pub fn record_content_ref_string(value: &PackedValue, record_name: &str, field: &str) -> Result<String, ViewError> {
    Ok(record_content_ref(value, record_name, field)?.into_string())
}

/// The references in `<record_name ["blake3:..." ...]>`, at most `maximum` of them.
pub fn record_content_ref_sequence(
    value: &PackedValue,
    record_name: &str,
    field: &str,
    maximum: u64,
) -> Result<Vec<ContentRef>, ViewError> {
    let maximum = maximum.to_usize(field)?;
    let values = required_sequence_field(single_field(value, record_name)?, field)?;
    crate::bounded::ensure_count_at_most(values.len(), maximum, field)?;
    values.iter().map(|item| required_content_ref(item, field)).collect()
}

pub fn record_content_ref_strings(
    value: &PackedValue,
    record_name: &str,
    field: &str,
    maximum: u64,
) -> Result<Vec<String>, ViewError> {
    let refs = record_content_ref_sequence(value, record_name, field, maximum)?;
    Ok(refs.into_iter().map(ContentRef::into_string).collect())
}

fn required_str<'a>(value: &'a PackedValue, field: &str) -> Result<&'a str, ViewError> {
    value.as_str().ok_or_else(|| ViewError::ExpectedString {
        field: field.to_owned(),
    })
}

fn parse_ref(text: &str, field: &str) -> Result<ContentRef, ViewError> {
    ContentRef::parse(text).map_err(|error| ViewError::ContentRef {
        field: field.to_owned(),
        value: text.to_owned(),
        error,
    })
}

fn single_field<'a>(value: &'a PackedValue, record_name: &str) -> Result<&'a PackedValue, ViewError> {
    let [field] = simple_record_fields(value, record_name, 1)? else {
        return Err(ViewError::RecordShape {
            label: record_name.to_owned(),
            arity: 1,
        });
    };
    Ok(field)
}

/// The string of a `<some "...">` whose reference was rejected.
fn some_text(value: &PackedValue) -> &str {
    value
        .symbol_record_fields(crate::identity::SOME_LABEL, 1)
        .and_then(<[_]>::first)
        .and_then(PackedValue::as_str)
        .unwrap_or_default()
}
