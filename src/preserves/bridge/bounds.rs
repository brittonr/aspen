//! Allocation-free packed-value shape admission before recursive decoding.
//!
//! The preflight itself lives in `molten_preserves_core::bounds`; this shell maps its denial to
//! the canonical `Failure`.

/// Maximum packed nesting admitted by bounded external-data readers.
pub const MAX_DEPTH: usize = molten_preserves_core::bounds::MAX_DEPTH;

/// Token ceiling shared by emitted external receipts and bounded ledger reads.
pub const MAX_VALUE_TOKENS: u64 = molten_preserves_core::bounds::MAX_VALUE_TOKENS;

/// Check one complete packed value without allocating its decoded tree.
/// Callers separately bound input bytes and enforce canonical encoding.
pub fn preflight(bytes: &[u8], maximum_tokens: u64) -> crate::error::Result<()> {
    molten_preserves_core::bounds::preflight(bytes, maximum_tokens)
        .map_err(|denied| crate::error::Failure::invalid_harness(denied.to_string()))
}
