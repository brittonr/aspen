//! Count and byte bounds for chunk-store collections, and bounded collection helpers.

pub(super) const MAX_CHUNK_STORE_CHUNKS: usize = 65_536;
pub(super) const MAX_CHUNK_STORE_REFS: usize = 100_000;
pub(super) const MAX_CHUNK_STORE_RECEIPTS: usize = 100_000;
pub(super) const MAX_CHUNK_STORE_CHECKS: usize = 64;
pub(super) const MAX_CHUNK_STORE_CONTEXT_REFS: usize = 100_000;
pub(super) const MAX_CHUNK_STORE_OBJECT_BYTES: usize = 1_073_741_824;
pub(super) const MAX_CHUNK_STORE_MANIFESTS: usize = 100_000;

const _: () = assert!(MAX_CHUNK_STORE_CHUNKS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_REFS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_RECEIPTS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_CHECKS <= 1_000);
const _: () = assert!(MAX_CHUNK_STORE_CONTEXT_REFS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_OBJECT_BYTES <= 1_073_741_824);
const _: () = assert!(MAX_CHUNK_STORE_MANIFESTS <= 1_000_000);

pub(super) fn extend_bytes_bounded(
    bytes: &mut impl crate::bounded::VecSink<u8>,
    incoming: &[u8],
    maximum: usize,
    label: &str,
) -> crate::error::Result<()> {
    let final_count = crate::bounded::checked_count_sum(bytes.item_count(), incoming.len(), maximum, label)?;
    bytes.reserve_items(final_count.saturating_sub(bytes.item_count()));
    bytes.extend_cloned_items(incoming);
    Ok(())
}

pub(super) fn insert_set_bounded<T: Ord>(
    values: &mut std::collections::BTreeSet<T>,
    value: T,
    maximum: usize,
    label: &str,
) -> crate::error::Result<bool> {
    if !values.contains(&value) {
        crate::bounded::checked_count_sum(values.len(), 1, maximum, label)?;
    }
    Ok(values.insert(value))
}

fn chunk_count_for_len(byte_len: usize, chunk_size: usize, label: &str) -> crate::error::Result<usize> {
    if chunk_size == 0 {
        return Err(crate::error::Failure::invalid_harness(format!("{label} chunk size must be non-zero")));
    }
    let count = byte_len.div_ceil(chunk_size);
    crate::bounded::ensure_count_at_most(count, MAX_CHUNK_STORE_CHUNKS, label)?;
    Ok(count)
}

pub(super) fn chunk_size_to_usize(chunk_size: u64, label: &str) -> crate::error::Result<usize> {
    usize::try_from(chunk_size).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("{label} is unsupported on this platform: {error}"))
    })
}

pub(super) fn validate_put_bounds(
    byte_len: usize,
    chunk_size: usize,
    policy_ref_count: usize,
) -> crate::error::Result<()> {
    crate::bounded::ensure_count_at_most(byte_len, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    chunk_count_for_len(byte_len, chunk_size, "chunk store object chunks")?;
    crate::bounded::ensure_count_at_most(policy_ref_count, MAX_CHUNK_STORE_REFS, "chunk store policy refs")?;
    Ok(())
}
