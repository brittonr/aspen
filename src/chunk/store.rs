//! Content-addressed chunk store: fixed-size chunking, manifests, receipts, Redb index, pins, GC,
//! lineage, and Iroh/peer exchange.
//!
//! The public surface is re-exported here by name and reached through both the hidden
//! `molten::blocks` path and the `chunk_store` facade alias. `src/lib.rs` mounts this file with
//! `#[path]`, so each submodule names its file under `store/` explicitly. Submodules own one
//! responsibility each and reference siblings by qualified `crate::blocks::` paths.
//!
//! Former shard items stay public at their old `molten::blocks::` paths:
//!
//! ```
//! let put: fn(&std::path::Path, &str, &[u8], u64) -> molten::error::Result<molten::blocks::ChunkPut> =
//!     molten::blocks::put_bytes;
//! let parse: fn(&preserves::IOValue, Option<&str>) -> molten::error::Result<molten::blocks::ChunkManifest> =
//!     molten::blocks::parse_manifest_value;
//! let _ = (put, parse, molten::blocks::evaluate_chunk_availability);
//! assert_eq!(molten::blocks::FIXED_V1_CHUNKER, molten::chunk_store::FIXED_V1_CHUNKER);
//! assert_eq!(molten::blocks::DEFAULT_FIXED_V1_CHUNK_SIZE, 64 * 1024);
//! let _: Option<molten::blocks::ChunkTraversalSyncPlan> = None;
//! let _: Option<molten::blocks::RemoteRangeReadbackReceipt> = None;
//! ```

#[path = "store/exchange/mod.rs"]
mod exchange;
#[path = "store/history/mod.rs"]
mod history;
#[path = "store/integrity.rs"]
mod integrity;
#[path = "store/layout.rs"]
mod layout;
#[path = "store/limits.rs"]
mod limits;
#[path = "store/local/mod.rs"]
mod local;
#[path = "store/model.rs"]
mod model;
#[path = "store/outboard.rs"]
mod outboard;
#[path = "store/planner.rs"]
mod planner;
#[path = "store/registry/mod.rs"]
mod registry;
#[path = "store/retire/mod.rs"]
mod retire;
#[cfg(test)]
#[path = "store/tests/mod.rs"]
mod tests;
#[path = "store/wire/mod.rs"]
mod wire;

pub use exchange::announce::publish_iroh_blobs;
pub use exchange::announce::publish_iroh_blobs_with_roots;
pub use exchange::landing::fetch_iroh_blobs;
pub use exchange::landing::fetch_iroh_blobs_with_roots;
pub use exchange::mirror::sync_missing_chunks;
pub use exchange::mirror::sync_missing_chunks_with_roots;
pub use history::assemble::build_chunk_lineage;
pub use history::assemble::build_chunk_lineage_with_root;
pub use history::audit::parse_chunk_lineage_value;
pub use local::census::evaluate_chunk_availability;
pub use local::census::list_chunk_refs;
pub use local::census::list_chunk_refs_with_root;
pub use local::census::list_manifest_refs;
pub use local::census::list_manifest_refs_with_root;
pub use local::census::list_receipt_refs;
pub use local::census::list_receipt_refs_with_root;
pub use local::census::read_receipt;
pub use local::census::read_receipt_with_root;
pub use local::hold::chunk_is_pinned;
pub use local::hold::chunk_is_pinned_with_root;
pub use local::hold::manifest_is_pinned;
pub use local::hold::manifest_is_pinned_with_root;
pub use local::hold::pin_chunk;
pub use local::hold::pin_chunk_with_root;
pub use local::hold::pin_manifest;
pub use local::hold::pin_manifest_with_root;
pub use local::hold::unpin_chunk;
pub use local::hold::unpin_chunk_with_root;
pub use local::hold::unpin_manifest;
pub use local::hold::unpin_manifest_with_root;
pub use local::ingest::put_bytes;
pub use local::ingest::put_bytes_with_metadata;
pub use local::ingest::put_bytes_with_root;
pub use local::ingest::put_bytes_with_transforms;
pub use local::readout::missing_chunks;
pub use local::readout::missing_chunks_with_root;
pub use local::readout::read_manifest;
pub use local::readout::read_manifest_with_root;
pub use local::readout::read_object;
pub use local::readout::read_object_with_root;
pub use local::readout::verify_manifest;
pub use local::readout::verify_manifest_with_root;
pub use local::slice::range_read;
pub use local::slice::range_read_with_root;
pub use model::CapabilityChunkRoot;
pub use model::ChunkAvailabilityDecision;
pub use model::ChunkAvailabilityInput;
pub use model::ChunkGc;
pub use model::ChunkGcInput;
pub use model::ChunkIndexRebuild;
pub use model::ChunkIndexStatus;
pub use model::ChunkIrohFetch;
pub use model::ChunkIrohPublish;
pub use model::ChunkLineage;
pub use model::ChunkManifest;
pub use model::ChunkPin;
pub use model::ChunkPut;
pub use model::ChunkRangeRead;
pub use model::ChunkRead;
pub use model::ChunkReceipt;
pub use model::ChunkReceiptCheck;
pub use model::ChunkRef;
pub use model::ChunkSync;
pub use model::ChunkTransforms;
pub use model::ChunkVerify;
pub use model::DEFAULT_FIXED_V1_CHUNK_SIZE;
pub use model::FIXED_V1_CHUNKER;
pub use model::PutBytesWithMetadataInput;
pub use model::PutBytesWithTransformsInput;
pub use model::open_capability_chunk_root;
pub use outboard::RemoteByteSourceHint;
pub use outboard::RemoteByteSourceHintInput;
pub use outboard::RemoteRangeReadbackInput;
pub use outboard::RemoteRangeReadbackReceipt;
pub use outboard::remote_byte_source_hint;
pub use outboard::verify_remote_range_readback;
pub use planner::ChunkFetchEffect;
pub use planner::ChunkSyncResponseInput;
pub use planner::ChunkSyncResponseReceipt;
pub use planner::ChunkTraversalSyncInput;
pub use planner::ChunkTraversalSyncPlan;
pub use planner::plan_chunk_traversal_sync;
pub use planner::validate_chunk_sync_response;
pub use registry::survey::index_status;
pub use registry::survey::index_status_with_root;
pub use registry::survey::rebuild_index;
pub use registry::survey::rebuild_index_with_root;
pub use retire::sweep::gc;
pub use retire::sweep::gc_with_root;
pub use wire::decode::parse_chunk_ref_value;
pub use wire::decode::parse_manifest_value;
pub use wire::outcome::parse_receipt_value;

/// Reads one chunk file and verifies its domain-separated content ref and recorded length.
pub(crate) fn read_verified_chunk(
    root: &CapabilityChunkRoot,
    chunk: &ChunkRef,
    chunk_size: usize,
) -> crate::error::Result<Vec<u8>> {
    let path = layout::chunk_path(&chunk.chunk_ref)?;
    let bytes = root.root().read(&path)?;
    let actual_ref = hash_chunk(&bytes, chunk_size);
    if actual_ref != chunk.chunk_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk hash mismatch: got {actual_ref}, expected {}",
            chunk.chunk_ref
        )));
    }
    if bytes.len() as u64 != chunk.length {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk length mismatch: got {}, expected {}",
            bytes.len(),
            chunk.length
        )));
    }
    Ok(bytes)
}

/// The `fixed_v1` chunk content ref: BLAKE3 over a domain tag, the chunk-size domain, and the chunk
/// bytes.
pub(crate) fn hash_chunk(bytes: &[u8], chunk_size: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten.chunk-store.chunk.fixed_v1\0");
    hasher.update(integrity::chunk_domain(chunk_size).as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize())
}
