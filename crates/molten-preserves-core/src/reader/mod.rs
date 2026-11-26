//! The strict, bounded canonical packed decoder.
//!
//! `decode_canonical` accepts exactly the bytes `crate::writer::encode_canonical` produces for some
//! value: valid tags, minimal varint lengths, minimal integers, binary64 floats, UTF-8 strings and
//! symbols, labelled records, set elements and dictionary keys in strictly increasing canonical
//! byte order, no annotations, and no trailing bytes. Every length is checked against the remaining
//! input before any allocation, and the explicit limits bound input bytes, nesting depth, total
//! nodes, and the children of one compound. Decoding keeps an explicit frame stack instead of
//! recursing.

pub mod failure;
mod input;
mod machine;

/// Nesting ceiling for input that is already resident in memory.
pub const IN_MEMORY_MAX_DEPTH: u64 = 1024;

/// Explicit ceilings for one decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeLimits {
    /// Maximum input length in bytes.
    pub max_bytes: u64,
    /// Maximum number of simultaneously open compounds, `Embedded` prefixes, and annotations.
    pub max_depth: u64,
    /// Maximum number of values (including annotation prefixes) in the whole input.
    pub max_nodes: u64,
    /// Maximum children delivered to one compound; a dictionary entry counts its key and its value.
    pub max_container_items: u64,
}

impl DecodeLimits {
    /// Limits for input already resident in memory: the slice itself bounds bytes, nodes, and
    /// widths, so only nesting depth is capped.
    pub const IN_MEMORY: Self = Self {
        max_bytes: u64::MAX,
        max_depth: IN_MEMORY_MAX_DEPTH,
        max_nodes: u64::MAX,
        max_container_items: u64::MAX,
    };
}

/// What the decoder does with an annotation (`0x85`) prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationPolicy {
    /// Annotations are not canonical; reject them.
    Reject,
    /// Decode and drop annotations, like `preserves` readers that do not keep annotations. The rest
    /// of the input must still be canonical.
    Skip,
}

/// Decodes exactly one canonical packed value.
pub fn decode_canonical(
    bytes: &[u8],
    limits: &DecodeLimits,
) -> Result<crate::model::PackedValue, failure::DecodeError> {
    machine::decode(bytes, limits, AnnotationPolicy::Reject)
}

/// Decodes one packed value whose only non-canonical content is annotations, which are dropped.
pub fn decode_skipping_annotations(
    bytes: &[u8],
    limits: &DecodeLimits,
) -> Result<crate::model::PackedValue, failure::DecodeError> {
    machine::decode(bytes, limits, AnnotationPolicy::Skip)
}
