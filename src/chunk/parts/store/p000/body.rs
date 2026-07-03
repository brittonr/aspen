type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::ReadableTableMetadata;

type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type CompoundClass = preserves::CompoundClass;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type ValueClass = preserves::ValueClass;
type Database = redb::Database;
type TableDefinition<K, V> = redb::TableDefinition<'static, K, V>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const CHUNK_LINEAGE_SCHEMA: &str = crate::preserves_rail::CHUNK_LINEAGE_SCHEMA;
const CHUNK_MANIFEST_SCHEMA: &str = crate::preserves_rail::CHUNK_MANIFEST_SCHEMA;
const CHUNK_REF_SCHEMA: &str = crate::preserves_rail::CHUNK_REF_SCHEMA;
const CHUNK_STORE_RECEIPT_SCHEMA: &str = crate::preserves_rail::CHUNK_STORE_RECEIPT_SCHEMA;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub(super) fn read_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn remove_file(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    crate::preserves_rail::content_ref_from_blake3_hash(hash)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn content_ref_from_hex(hex: &str) -> Result<String> {
    crate::preserves_rail::content_ref_from_hex(hex)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const FIXED_V1_CHUNKER: &str = "fixed_v1";
pub const DEFAULT_FIXED_V1_CHUNK_SIZE: u64 = 64 * 1024;

const MAX_CHUNK_STORE_CHUNKS: usize = 65_536;
const MAX_CHUNK_STORE_REFS: usize = 100_000;
const MAX_CHUNK_STORE_RECEIPTS: usize = 100_000;
const MAX_CHUNK_STORE_CHECKS: usize = 64;
const MAX_CHUNK_STORE_CONTEXT_REFS: usize = 100_000;
const MAX_CHUNK_STORE_OBJECT_BYTES: usize = 1_073_741_824;
const MAX_CHUNK_STORE_MANIFESTS: usize = 100_000;
const CHUNK_AVAILABILITY_DIAGNOSTIC_CAPACITY: usize = 8;

const _: () = assert!(DEFAULT_FIXED_V1_CHUNK_SIZE > 0);
const _: () = assert!(MAX_CHUNK_STORE_CHUNKS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_REFS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_RECEIPTS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_CHECKS <= 1_000);
const _: () = assert!(MAX_CHUNK_STORE_CONTEXT_REFS <= 1_000_000);
const _: () = assert!(MAX_CHUNK_STORE_OBJECT_BYTES <= 1_073_741_824);
const _: () = assert!(MAX_CHUNK_STORE_MANIFESTS <= 1_000_000);

const INDEX_FILE: &str = "chunk-index.redb";
const CHUNK_INDEX_SCHEMA: &str = "molten.chunk-store.index.chunk.v1";
const CHUNK_TRANSFORMS_SCHEMA: &str = "molten.chunk-store.transforms.v1";
const CHUNK_IROH_TICKET_SCHEMA: &str = "molten.chunk-store.iroh-ticket.v1";
const PARTIAL_FETCH_SCHEMA: &str = "molten.chunk-store.index.partial-fetch.v1";

const INDEX_MANIFESTS: TableDefinition<&str, &[u8]> = TableDefinition::new("chunk_store_manifests_v1");
const INDEX_CHUNKS: TableDefinition<&str, &[u8]> = TableDefinition::new("chunk_store_chunks_v1");
const INDEX_AVAILABILITY: TableDefinition<&str, &str> = TableDefinition::new("chunk_store_availability_v1");
const INDEX_PINS: TableDefinition<&str, &str> = TableDefinition::new("chunk_store_pins_v1");
const INDEX_PARTIAL_FETCHES: TableDefinition<&str, &[u8]> = TableDefinition::new("chunk_store_partial_fetches_v1");
const INDEX_RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("chunk_store_receipts_v1");

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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStorePut {
    pub manifest_ref: String,
    pub object_kind: String,
    pub total_len: u64,
    pub chunk_refs: Vec<String>,
    pub dedup_hits: usize,
    pub manifest_value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreVerify {
    pub manifest_ref: String,
    pub total_len: u64,
    pub chunk_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreRead {
    pub manifest_ref: String,
    pub bytes: Vec<u8>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreRangeRead {
    pub manifest_ref: String,
    pub offset: u64,
    pub length: u64,
    pub bytes: Vec<u8>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreSync {
    pub manifest_ref: String,
    pub missing_before: Vec<String>,
    pub fetched_chunks: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreIrohPublish {
    pub ticket: String,
    pub manifest_ref: String,
    pub manifest_blob_ref: String,
    pub chunk_blob_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreIrohFetch {
    pub ticket: String,
    pub manifest_ref: String,
    pub manifest_blob_ref: String,
    pub missing_before: Vec<String>,
    pub fetched_chunks: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IrohChunkBlob {
    chunk_ref: String,
    blob_ref: String,
    length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IrohChunkTicket {
    manifest_ref: String,
    manifest_blob_ref: String,
    chunks: Vec<IrohChunkBlob>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreGc {
    pub dry_run: bool,
    pub decision: String,
    pub removed_manifests: Vec<String>,
    pub removed_chunks: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkStoreGcInput<'a> {
    pub dry_run: bool,
    pub retention_evidence: &'a crate::retention::DestructiveEvidence,
    pub apply_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStorePin {
    pub kind: String,
    pub reference: String,
    pub pinned: bool,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreIndexStatus {
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
pub struct ChunkStoreIndexRebuild {
    pub status: ChunkStoreIndexStatus,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreReceiptCheck {
    pub name: String,
    pub status: String,
}
