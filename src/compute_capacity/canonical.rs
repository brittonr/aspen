//! Versioned canonical Preserves projections of the pure compute-capacity core.
//! A reference identifies evidence, not permission to reserve or activate capacity.

type Result<T, E = crate::error::Failure> = std::result::Result<T, E>;
type Field = preserves::Value<preserves::IOValue>;

const OFFER_RECORD: &str = "compute-capacity-offer-v2";
const ALLOCATION_RECORD: &str = "compute-capacity-allocation-v2";
const READBACK_RECORD: &str = "compute-capacity-readback-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub value: preserves::IOValue,
    pub reference: String,
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn materialize(value: preserves::IOValue) -> Result<Record> {
    let reference = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Record { value, reference })
}

fn fields<'a>(value: &'a Field, label: &str, arity: u64) -> Result<std::borrow::Cow<'a, preserves::Record<Field>>> {
    crate::preserves_rail::simple_record_fields(value.into(), label, arity)
}

fn text(value: &Field, name: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(value, name)
}

fn reference(value: &Field, name: &str) -> Result<String> {
    crate::preserves_rail::required_content_ref_string(value, name)
}

fn number(value: &Field, name: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| invalid(format!("{name} must be u64")))?
        .map_err(|_| invalid(format!("{name} exceeds u64")))
}

fn boolean(value: &Field, name: &str) -> Result<bool> {
    value.as_boolean().ok_or_else(|| invalid(format!("{name} must be boolean")))
}

fn optional<T>(value: &Field, parse: impl FnOnce(&Field) -> Result<T>) -> Result<Option<T>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let fields = fields(value, "some", 1)?;
        parse(&fields[0]).map(Some)
    }
}

fn optional_value<T>(value: Option<T>, encode: impl FnOnce(T) -> preserves::IOValue) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |value| crate::preserves_rail::record("some", vec![encode(value)]),
    )
}

fn sequence(value: &Field, name: &str, parse: impl Fn(&Field) -> Result<String>) -> Result<Vec<String>> {
    let values = crate::preserves_rail::required_sequence_field(value, name)?;
    if values.len() > molten_core::compute_capacity::MAX_ITEMS {
        return Err(invalid(format!("{name} exceeds capacity bound")));
    }
    values.iter().map(parse).collect()
}

fn records<T>(value: &Field, name: &str, parse: impl Fn(&Field) -> Result<T>) -> Result<Vec<T>> {
    let values = crate::preserves_rail::required_sequence_field(value, name)?;
    if values.len() > molten_core::compute_capacity::MAX_ITEMS {
        return Err(invalid(format!("{name} exceeds capacity bound")));
    }
    values.iter().map(parse).collect()
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn token(value: &str, name: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > molten_core::compute_capacity::MAX_TEXT
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(invalid(format!("{name} is not a bounded nonempty token")));
    }
    Ok(())
}

fn ref_token(value: &str, name: &str) -> Result<()> {
    crate::preserves_rail::ContentRef::parse(value)
        .map_err(|_| invalid(format!("{name} is not a canonical content reference")))?;
    Ok(())
}

fn bounded(values: &[String], name: &str, refs: bool) -> Result<()> {
    if values.len() > molten_core::compute_capacity::MAX_ITEMS {
        return Err(invalid(format!("{name} exceeds capacity bound")));
    }
    for value in values {
        if refs {
            ref_token(value, name)?
        } else {
            token(value, name)?
        }
    }
    Ok(())
}

include!("canonical/enums.rs");
include!("canonical/offers.rs");
include!("canonical/allocations.rs");
include!("canonical/readbacks.rs");

#[cfg(test)]
mod tests;
