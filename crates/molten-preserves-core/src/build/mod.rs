//! `PackedValue` constructors that mirror the `molten` rail's `IOValue` builders.
//!
//! A value built here encodes to the same canonical bytes, and so has the same `blake3:` identity,
//! as the same logical value built with the rail's `symbol`, `string`, `u64_value`, `bool_value`,
//! `sequence`, `record`, `optional_ref_value`, `refs_sequence` and `checks_value`. The rail keeps
//! its own `IOValue` builders, because building a `PackedValue` and converting it would add a full
//! tree copy to every shell value; a bounded differential property in the rail's tests holds the
//! two byte-identical.

pub use crate::identity::optional_ref_value;

pub const CHECKS_LABEL: &str = "checks";
pub const CHECK_LABEL: &str = "check";

pub fn symbol(name: &str) -> crate::model::PackedValue {
    crate::model::PackedValue::symbol(name)
}

pub fn string(value: impl AsRef<str>) -> crate::model::PackedValue {
    crate::model::PackedValue::string(value.as_ref())
}

pub fn u64_value(value: u64) -> crate::model::PackedValue {
    crate::model::PackedValue::from(value)
}

pub fn bool_value(value: bool) -> crate::model::PackedValue {
    crate::model::PackedValue::Boolean(value)
}

pub fn sequence(values: alloc::vec::Vec<crate::model::PackedValue>) -> crate::model::PackedValue {
    crate::model::PackedValue::Sequence(values)
}

/// A record labelled by the symbol `label`.
// r[impl molten.preserves_rail_toolkit.parser_builders]
pub fn record(label: &str, fields: alloc::vec::Vec<crate::model::PackedValue>) -> crate::model::PackedValue {
    crate::model::PackedValue::symbol_record(label, fields)
}

/// A sequence of reference strings, written as given.
pub fn refs_sequence(refs: &[impl AsRef<str>]) -> crate::model::PackedValue {
    sequence(refs.iter().map(string).collect())
}

/// `<checks [<check name status> ...]>`.
// r[impl molten.preserves_rail_toolkit.check_sets]
pub fn checks_value(checks: &[(&str, &str)]) -> crate::model::PackedValue {
    let entries = checks.iter().map(|(name, status)| record(CHECK_LABEL, alloc::vec![string(name), string(status)]));
    record(CHECKS_LABEL, alloc::vec![sequence(entries.collect())])
}

/// The `blake3:<hex>` identity of `value`: BLAKE3 over its canonical packed bytes, the string the
/// rail's `canonical_hash` returns.
// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
pub fn canonical_hash(value: &crate::model::PackedValue) -> alloc::string::String {
    crate::identity::ContentRef::of_value(value).into_string()
}

/// The `blake3:<hex>` identity of already-canonical bytes.
pub fn content_ref_from_bytes(bytes: &[u8]) -> alloc::string::String {
    crate::identity::ContentRef::of_bytes(bytes).into_string()
}

#[cfg(test)]
mod tests;
