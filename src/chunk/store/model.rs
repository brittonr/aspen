//! Public chunk-store DTOs, chunker constants, and the capability root type.

pub const FIXED_V1_CHUNKER: &str = "fixed_v1";
pub const DEFAULT_FIXED_V1_CHUNK_SIZE: u64 = 64 * 1024;

const _: () = assert!(DEFAULT_FIXED_V1_CHUNK_SIZE > 0);

pub type CapabilityChunkRoot = crate::local_store::ChunkStoreRoot;

pub fn open_capability_chunk_root(root: &std::path::Path) -> crate::error::Result<CapabilityChunkRoot> {
    crate::local_store::ChunkStoreRoot::open(root)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTransforms {
    pub compression: String,
    pub encryption: String,
    pub ordering: String,
    pub confidentiality: String,
    pub protected_commitment_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRef {
    pub chunk_ref: String,
    pub length: u64,
    pub domain: String,
    pub chunker: String,
    pub transforms: ChunkTransforms,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkManifest {
    pub manifest_ref: String,
    pub object_kind: String,
    pub total_len: u64,
    pub chunker: String,
    pub chunk_size: u64,
    pub transforms: ChunkTransforms,
    pub metadata_ref: String,
    pub policy_refs: Vec<String>,
    pub chunks: Vec<ChunkRef>,
    pub root_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPut {
    pub manifest_ref: String,
    pub object_kind: String,
    pub total_len: u64,
    pub chunk_refs: Vec<String>,
    pub dedup_hits: usize,
    pub manifest_value: preserves::IOValue,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkVerify {
    pub manifest_ref: String,
    pub total_len: u64,
    pub chunk_refs: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRead {
    pub manifest_ref: String,
    pub bytes: Vec<u8>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRangeRead {
    pub manifest_ref: String,
    pub offset: u64,
    pub length: u64,
    pub bytes: Vec<u8>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSync {
    pub manifest_ref: String,
    pub missing_before: Vec<String>,
    pub fetched_chunks: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkIrohPublish {
    pub ticket: String,
    pub manifest_ref: String,
    pub manifest_blob_ref: String,
    pub chunk_blob_refs: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkIrohFetch {
    pub ticket: String,
    pub manifest_ref: String,
    pub manifest_blob_ref: String,
    pub missing_before: Vec<String>,
    pub fetched_chunks: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkGc {
    pub dry_run: bool,
    pub decision: String,
    pub removed_manifests: Vec<String>,
    pub removed_chunks: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkGcInput<'a> {
    pub dry_run: bool,
    pub retention_evidence: &'a crate::retention::DestructiveEvidence,
    pub apply_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPin {
    pub kind: String,
    pub reference: String,
    pub pinned: bool,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkIndexStatus {
    pub manifests: u64,
    pub chunks: u64,
    pub available_chunks: u64,
    pub missing_chunks: u64,
    pub manifest_pins: u64,
    pub chunk_pins: u64,
    pub partial_fetches: u64,
    pub receipts: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkAvailabilityInput<'a> {
    pub manifest: &'a ChunkManifest,
    pub available_chunk_refs: &'a [String],
    pub missing_chunk_refs: &'a [String],
    pub indexed_available_refs: &'a [String],
    pub indexed_missing_refs: &'a [String],
    pub partial_fetch_missing_refs: &'a [String],
    pub partial_fetch_fetched_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkAvailabilityDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkIndexRebuild {
    pub status: ChunkIndexStatus,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReceiptCheck {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub manifest_ref: Option<String>,
    pub chunk_refs: Vec<String>,
    pub checks: Vec<ChunkReceiptCheck>,
    pub details: Vec<preserves::IOValue>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkLineage {
    pub lineage_ref: String,
    pub manifest_ref: String,
    pub root_ref: String,
    pub link_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub verify_receipt_ref: String,
    pub predicate_receipt_refs: Vec<String>,
    pub value: preserves::IOValue,
}

pub struct PutBytesWithMetadataInput<'a> {
    pub root: &'a std::path::Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a preserves::IOValue,
    pub policy_refs: &'a [String],
}

pub struct PutBytesWithTransformsInput<'a> {
    pub root: &'a std::path::Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a preserves::IOValue,
    pub policy_refs: &'a [String],
    pub transforms: &'a ChunkTransforms,
}

impl ChunkTransforms {
    pub fn public_plaintext() -> Self {
        Self {
            compression: "none".to_string(),
            encryption: "none".to_string(),
            ordering: "identity".to_string(),
            confidentiality: "public".to_string(),
            protected_commitment_ref: None,
        }
    }

    pub fn confidential_protected(commitment_ref: impl Into<String>) -> Self {
        Self {
            compression: "zstd-placeholder".to_string(),
            encryption: "protected-commitment".to_string(),
            ordering: "compress-then-encrypt".to_string(),
            confidentiality: "confidential".to_string(),
            protected_commitment_ref: Some(commitment_ref.into()),
        }
    }
}
