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
    pub retention_evidence: &'a crate::retention::DestructiveRetentionEvidence,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub manifest_ref: Option<String>,
    pub chunk_refs: Vec<String>,
    pub checks: Vec<ChunkStoreReceiptCheck>,
    pub details: Vec<IoValue>,
    pub value: IoValue,
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
    pub value: IoValue,
}

pub struct PutBytesWithMetadataInput<'a> {
    pub root: &'a Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a IoValue,
    pub policy_refs: &'a [String],
}

pub struct PutBytesWithTransformsInput<'a> {
    pub root: &'a Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a IoValue,
    pub policy_refs: &'a [String],
    pub transforms: &'a ChunkTransforms,
}

pub fn put_bytes(root: &Path, object_kind: &str, bytes: &[u8], chunk_size: u64) -> Result<ChunkStorePut> {
    let metadata = record("chunk-metadata-v1", vec![
        record("object-kind", vec![string(object_kind)]),
        record("policy", vec![string("public-local-fixture")]),
    ]);
    put_bytes_with_metadata(&PutBytesWithMetadataInput {
        root,
        object_kind,
        bytes,
        chunk_size,
        metadata: &metadata,
        policy_refs: &[],
    })
}

pub fn put_bytes_with_metadata(input: &PutBytesWithMetadataInput<'_>) -> Result<ChunkStorePut> {
    put_bytes_with_transforms(&PutBytesWithTransformsInput {
        root: input.root,
        object_kind: input.object_kind,
        bytes: input.bytes,
        chunk_size: input.chunk_size,
        metadata: input.metadata,
        policy_refs: input.policy_refs,
        transforms: &ChunkTransforms::public_plaintext(),
    })
}

struct Ready {
    chunk_size: usize,
    metadata_ref: String,
}

struct Written {
    values: Vec<IoValue>,
    refs: Vec<ChunkRef>,
    dedup_refs: Vec<String>,
    dedup_hits: usize,
}

struct FinalizeInput<'a> {
    root: &'a Path,
    object_kind: &'a str,
    total_len: u64,
    chunk_size_input: u64,
    chunk_size: usize,
    transforms: &'a ChunkTransforms,
    metadata_ref: &'a str,
    policy_refs: &'a [String],
    written: Written,
}

pub fn put_bytes_with_transforms(input: &PutBytesWithTransformsInput<'_>) -> Result<ChunkStorePut> {
    let ready = prepare_put(input)?;
    let written = write_put_parts(input.root, input.bytes, ready.chunk_size, input.transforms)?;
    finish_put(FinalizeInput {
        root: input.root,
        object_kind: input.object_kind,
        total_len: input.bytes.len() as u64,
        chunk_size_input: input.chunk_size,
        chunk_size: ready.chunk_size,
        transforms: input.transforms,
        metadata_ref: &ready.metadata_ref,
        policy_refs: input.policy_refs,
        written,
    })
}

fn prepare_put(input: &PutBytesWithTransformsInput<'_>) -> Result<Ready> {
    ensure_dirs(input.root)?;
    if input.object_kind.is_empty() {
        let receipt_value =
            denial_receipt_value("manifest-create", None, &[], "chunk store object kind must not be empty", vec![
                ("object-kind", "fail"),
                ("denial-receipt", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("chunk store object kind must not be empty"));
    }
    if input.chunk_size == 0 {
        let receipt_value =
            denial_receipt_value("manifest-create", None, &[], "fixed_v1 chunk size must be non-zero", vec![
                ("chunk-size", "fail"),
                ("denial-receipt", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("fixed_v1 chunk size must be non-zero"));
    }
    let chunk_size = match chunk_size_to_usize(input.chunk_size, "fixed_v1 chunk size") {
        Ok(chunk_size) => chunk_size,
        Err(error) => {
            let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
                ("chunk-size", "fail"),
                ("denial-receipt", "pass"),
            ]);
            store_receipt(input.root, &receipt_value)?;
            return Err(error);
        }
    };
    if let Err(error) = validate_put_transforms(input.transforms) {
        let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
            ("confidentiality-policy", "fail"),
            ("transform-ordering", "fail"),
            ("deny-plaintext-hash-leakage", "pass"),
        ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(error);
    }
    if let Err(error) = validate_put_bounds(input.bytes.len(), chunk_size, input.policy_refs.len()) {
        let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
            ("resource-bounds", "fail"),
            ("denial-receipt", "pass"),
        ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(error);
    }
    let metadata_ref = canonical_hash(input.metadata)?;
    write_immutable_bytes(
        &metadata_path(input.root, &metadata_ref)?,
        &canonical_bytes(input.metadata)?,
        &metadata_ref,
        parse_canonical_bytes,
    )?;
    Ok(Ready {
        chunk_size,
        metadata_ref,
    })
}

fn ensure_part(root: &Path, chunk: &[u8], chunk_size: usize) -> Result<(String, bool)> {
    let chunk_ref = hash_chunk(chunk, chunk_size);
    let path = chunk_path(root, &chunk_ref)?;
    if path.exists() {
        if let Err(error) = verify_raw_chunk_file(&path, &chunk_ref, chunk.len() as u64, chunk_size) {
            let receipt_value =
                denial_receipt_value("dedup-hit", None, std::slice::from_ref(&chunk_ref), error.to_string(), vec![
                    ("existing-chunk-hash-binding", "fail"),
                    ("deny-corrupt-dedup-source", "pass"),
                ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
        Ok((chunk_ref, true))
    } else {
        fs::write(&path, chunk).map_err(MoltenError::from)?;
        Ok((chunk_ref, false))
    }
}

fn write_put_parts(root: &Path, bytes: &[u8], chunk_size: usize, transforms: &ChunkTransforms) -> Result<Written> {
    let mut values = Vec::new();
    let mut refs = Vec::new();
    let mut dedup_refs = Vec::new();
    let mut dedup_hits = 0usize;
    for chunk in bytes.chunks(chunk_size) {
        let (chunk_ref, was_present) = ensure_part(root, chunk, chunk_size)?;
        if was_present {
            dedup_hits += 1;
            push_bounded(&mut dedup_refs, chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store dedup chunks")?;
        }
        push_bounded(
            &mut refs,
            ChunkRef {
                chunk_ref: chunk_ref.clone(),
                length: chunk.len() as u64,
                domain: chunk_domain(chunk_size),
                chunker: FIXED_V1_CHUNKER.to_string(),
                transforms: transforms.clone(),
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store chunk refs",
        )?;
        push_bounded(
            &mut values,
            chunk_ref_value(&chunk_ref, chunk.len() as u64, chunk_size, transforms),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store chunk values",
        )?;
    }
    Ok(Written {
        values,
        refs,
        dedup_refs,
        dedup_hits,
    })
}

fn note_reuse(root: &Path, manifest_ref: &str, refs: &[String]) -> Result<()> {
    if refs.is_empty() {
        return Ok(());
    }
    let dedup_receipt = self::receipt_value(ChunkStoreReceiptValueInput {
        operation: "dedup-hit",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: refs,
        checks: vec![
            ("existing-chunk-hash-binding", "pass"),
            ("dedup-no-rewrite", "pass"),
            ("receipt-index-update", "pass"),
        ],
        details: vec![record("dedup-hits", vec![u64_value(refs.len() as u64)])],
    });
    store_receipt(root, &dedup_receipt)
}

fn finish_put(input: FinalizeInput<'_>) -> Result<ChunkStorePut> {
    let Written {
        values,
        refs,
        dedup_refs,
        dedup_hits,
    } = input.written;
    let root_ref = chunk_root_ref(&refs)?;
    let manifest_value = manifest_value(&ChunkManifestValueInput {
        object_kind: input.object_kind,
        total_len: input.total_len,
        chunk_size: input.chunk_size_input,
        transforms: input.transforms,
        metadata_ref: input.metadata_ref,
        policy_refs: input.policy_refs,
        chunks: &values,
        root_ref: &root_ref,
        evidence_refs: &[],
    });
    let manifest_ref = canonical_hash(&manifest_value)?;
    write_immutable_bytes(
        &manifest_path(input.root, &manifest_ref)?,
        &canonical_bytes(&manifest_value)?,
        &manifest_ref,
        parse_canonical_bytes,
    )?;
    let receipt_chunk_refs = refs.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "manifest-create",
        decision: "pass",
        manifest_ref: Some(&manifest_ref),
        chunk_refs: &receipt_chunk_refs,
        checks: vec![
            ("fixed-v1-chunking", "pass"),
            ("chunk-hash-binding", "pass"),
            ("manifest-root-binding", "pass"),
            ("immutable-chunk-store", "pass"),
            ("confidentiality-policy-admission", "pass"),
            ("transform-ordering", "pass"),
            ("protected-commitment-binding", "pass"),
            ("redb-index-update", "pass"),
        ],
        details: vec![
            record("object-kind", vec![string(input.object_kind)]),
            record("total-len", vec![u64_value(input.total_len)]),
            record("chunk-size", vec![u64_value(input.chunk_size as u64)]),
            record("dedup-hits", vec![u64_value(dedup_hits as u64)]),
        ],
    });
    index_put(input.root, &manifest_value, &refs, &receipt_value)?;
    note_reuse(input.root, &manifest_ref, &dedup_refs)?;
    Ok(ChunkStorePut {
        manifest_ref,
        object_kind: input.object_kind.to_string(),
        total_len: input.total_len,
        chunk_refs: refs.into_iter().map(|chunk| chunk.chunk_ref).collect(),
        dedup_hits,
        manifest_value,
        receipt_value,
    })
}

pub fn read_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkManifest> {
    let bytes = fs::read(manifest_path(root, manifest_ref)?).map_err(MoltenError::from)?;
    let value = parse_canonical_bytes(&bytes)?;
    parse_manifest_value(&value, Some(manifest_ref))
}

pub fn parse_manifest_value(value: &IoValue, expected_manifest_ref: Option<&str>) -> Result<ChunkManifest> {
    let fields = simple_record_any(value, "chunk-manifest-v1")?;
    let arity = record_arity(&fields);
    if arity != 10 && arity != 11 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <chunk-manifest-v1 ...> with arity 10 or 11, got {arity}"
        )));
    }
    require_schema(&fields[0], CHUNK_MANIFEST_SCHEMA, "chunk manifest")?;
    let object_kind = record_string(&fields[1], "object-kind")?;
    let total_len = record_u64(&fields[2], "total-len")?;
    let chunker = record_string(&fields[3], "chunker")?;
    let chunk_size = record_u64(&fields[4], "chunk-size")?;
    let (transforms, metadata_index) = if arity == 11 {
        (parse_transforms_field(&fields[5])?, 6)
    } else {
        (ChunkTransforms::public_plaintext(), 5)
    };
    let metadata_ref = record_string(&fields[metadata_index], "metadata-ref")?;
    let policy_refs = record_string_sequence(&fields[metadata_index + 1], "policy-refs")?;
    let chunk_values = record_sequence(&fields[metadata_index + 2], "chunks")?;
    let root_ref = record_string(&fields[metadata_index + 3], "root-ref")?;
    let evidence_refs = record_string_sequence(&fields[metadata_index + 4], "evidence-refs")?;
    ensure_count_at_most(policy_refs.len(), MAX_CHUNK_STORE_REFS, "chunk manifest policy refs")?;
    ensure_count_at_most(chunk_values.len(), MAX_CHUNK_STORE_CHUNKS, "chunk manifest chunks")?;
    ensure_count_at_most(evidence_refs.len(), MAX_CHUNK_STORE_REFS, "chunk manifest evidence refs")?;
    let manifest_ref = canonical_hash(value)?;
    if let Some(expected) = expected_manifest_ref
        && manifest_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest hash mismatch: got {manifest_ref}, expected {expected}"
        )));
    }
    if chunker != FIXED_V1_CHUNKER {
        return Err(MoltenError::invalid_harness(format!("unsupported chunker {chunker}")));
    }
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness("chunk manifest chunk-size must be non-zero"));
    }
    validate_transform_shape(&transforms)?;
    let chunks = refs_from_values(&chunk_values, chunk_size, &transforms)?;
    validate_fixed_chunk_lengths(total_len, chunk_size, &chunks)?;
    ensure_distinct_commitments(&chunks)?;
    let recomputed_root = chunk_root_ref(&chunks)?;
    if recomputed_root != root_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest root mismatch: got {root_ref}, expected {recomputed_root}"
        )));
    }
    Ok(ChunkManifest {
        manifest_ref,
        object_kind,
        total_len,
        chunker,
        chunk_size,
        transforms,
        metadata_ref,
        policy_refs,
        chunks,
        root_ref,
        evidence_refs,
        value: value.clone(),
    })
}

fn refs_from_values(values: &[IoValue], chunk_size: u64, transforms: &ChunkTransforms) -> Result<Vec<ChunkRef>> {
    let mut chunks = Vec::new();
    for value in values {
        let chunk = parse_chunk_ref_value(value, chunk_size)?;
        if chunk.transforms != *transforms {
            return Err(MoltenError::invalid_harness(format!(
                "chunk transform mismatch for {}: manifest transforms differ from chunk ref transforms",
                chunk.chunk_ref
            )));
        }
        push_bounded(&mut chunks, chunk, MAX_CHUNK_STORE_CHUNKS, "chunk manifest chunks")?;
    }
    Ok(chunks)
}

fn ensure_distinct_commitments(chunks: &[ChunkRef]) -> Result<()> {
    for chunk in chunks {
        if chunk.transforms.protected_commitment_ref.as_deref() == Some(chunk.chunk_ref.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "protected commitment ref for chunk {} must differ from the plaintext chunk ref",
                chunk.chunk_ref
            )));
        }
    }
    Ok(())
}

pub fn parse_chunk_ref_value(value: &IoValue, expected_chunk_size: u64) -> Result<ChunkRef> {
    let fields = simple_record_any(value, "chunk-ref-v1")?;
    let arity = record_arity(&fields);
    if arity != 7 && arity != 8 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <chunk-ref-v1 ...> with arity 7 or 8, got {arity}"
        )));
    }
    require_schema(&fields[0], CHUNK_REF_SCHEMA, "chunk ref")?;
    let chunk_ref = record_string(&fields[1], "hash")?;
    let length = record_u64(&fields[2], "length")?;
    let domain = record_string(&fields[3], "domain")?;
    let chunker = record_string(&fields[4], "chunker")?;
    let (transforms, location_index) = if arity == 8 {
        (parse_transforms_field(&fields[5])?, 6)
    } else {
        (ChunkTransforms::public_plaintext(), 5)
    };
    let _location_hints = record_sequence(&fields[location_index], "location-hints")?;
    let _evidence_refs = record_string_sequence(&fields[location_index + 1], "evidence-refs")?;
    if chunker != FIXED_V1_CHUNKER {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk ref chunker {chunker}")));
    }
    let expected_chunk_size_usize = chunk_size_to_usize(expected_chunk_size, "expected chunk size")?;
    let expected_domain = chunk_domain(expected_chunk_size_usize);
    if domain != expected_domain {
        return Err(MoltenError::invalid_harness(format!(
            "chunk ref domain mismatch: got {domain}, expected {expected_domain}"
        )));
    }
    if length == 0 || length > expected_chunk_size {
        return Err(MoltenError::invalid_harness(format!(
            "chunk ref length {length} outside fixed_v1 bounds 1..={expected_chunk_size}"
        )));
    }
    validate_transform_shape(&transforms)?;
    Ok(ChunkRef {
        chunk_ref,
        length,
        domain,
        chunker,
        transforms,
    })
}

pub fn verify_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStoreVerify> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("chunk-verify", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("chunk-verify", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    if let Err(error) = verify_manifest_chunks(root, &manifest) {
        let receipt_value =
            denial_receipt_value("chunk-verify", Some(&manifest.manifest_ref), &chunk_refs, error.to_string(), vec![
                ("chunk-hash-length", "fail"),
                ("deny-corrupt-or-missing-chunk", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(error);
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "chunk-verify",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![
            ("manifest-ref-binding", "pass"),
            ("chunk-hash-length", "pass"),
            ("chunk-order-root", "pass"),
            ("reconstructed-total-length", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![record("total-len", vec![u64_value(manifest.total_len)])],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreVerify {
        manifest_ref: manifest.manifest_ref,
        total_len: manifest.total_len,
        chunk_refs,
        receipt_value,
    })
}

pub fn read_object(root: &Path, manifest_ref: &str) -> Result<ChunkStoreRead> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("fetch", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value = denial_receipt_value("fetch", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
            ("transform-mode", "fail"),
            ("deny-unsupported-transform", "pass"),
        ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let bytes = match reconstruct_object(root, &manifest) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("fetch", Some(&manifest.manifest_ref), &chunk_refs, error.to_string(), vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-or-missing-chunk", "pass"),
                ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "fetch",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![
            ("manifest-ref-binding", "pass"),
            ("streaming-chunk-verification", "pass"),
            ("reconstructed-total-length", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![record("total-len", vec![u64_value(bytes.len() as u64)])],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreRead {
        manifest_ref: manifest.manifest_ref,
        bytes,
        receipt_value,
    })
}

struct SpanInput<'a> {
    root: &'a Path,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    offset: u64,
    length: u64,
}

struct SpanWindow {
    chunk_size: usize,
    offset: usize,
    end: usize,
    first: usize,
    last_exclusive: usize,
    expected_len: usize,
}

struct SpanData {
    bytes: Vec<u8>,
    refs: Vec<String>,
}

fn span_window(manifest: &ChunkManifest, offset: u64, length: u64) -> Result<SpanWindow> {
    let chunk_size = usize::try_from(manifest.chunk_size).map_err(|error| {
        MoltenError::invalid_harness(format!("manifest chunk size is unsupported on this platform: {error}"))
    })?;
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness("manifest chunk size must be greater than zero"));
    }
    let offset_usize = usize::try_from(offset).map_err(|error| {
        MoltenError::invalid_harness(format!("range offset is unsupported on this platform: {error}"))
    })?;
    let length_usize = usize::try_from(length).map_err(|error| {
        MoltenError::invalid_harness(format!("range length is unsupported on this platform: {error}"))
    })?;
    ensure_count_at_most(length_usize, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store range bytes")?;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| MoltenError::invalid_harness("range offset and length overflow"))?;
    let end_usize = usize::try_from(end)
        .map_err(|error| MoltenError::invalid_harness(format!("range end is unsupported on this platform: {error}")))?;
    let first = offset_usize
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("range chunk size must be non-zero"))?;
    Ok(SpanWindow {
        chunk_size,
        offset: offset_usize,
        end: end_usize,
        first,
        last_exclusive: end_usize.div_ceil(chunk_size),
        expected_len: length_usize,
    })
}

fn read_span(input: SpanInput<'_>) -> Result<SpanData> {
    let mut bytes = Vec::new();
    let mut refs = Vec::new();
    let mut expected_len = 0usize;
    if input.length > 0 {
        let window = span_window(input.manifest, input.offset, input.length)?;
        expected_len = window.expected_len;
        for index in window.first..window.last_exclusive {
            let chunk =
                input.manifest.chunks.get(index).ok_or_else(|| {
                    MoltenError::invalid_harness(format!("range maps to missing chunk index {index}"))
                })?;
            let chunk_bytes = match read_verified_chunk(input.root, chunk, window.chunk_size) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let receipt_value = denial_receipt_value(
                        "range-read",
                        Some(&input.manifest.manifest_ref),
                        input.refs,
                        error.to_string(),
                        vec![
                            ("range-chunk-verification", "fail"),
                            ("deny-corrupt-or-missing-chunk", "pass"),
                        ],
                    );
                    store_receipt(input.root, &receipt_value)?;
                    return Err(error);
                }
            };
            let chunk_start = index * window.chunk_size;
            let wanted_start = window.offset.saturating_sub(chunk_start);
            let wanted_end = window.end.saturating_sub(chunk_start).min(chunk_bytes.len());
            extend_bytes_bounded(
                &mut bytes,
                &chunk_bytes[wanted_start..wanted_end],
                MAX_CHUNK_STORE_OBJECT_BYTES,
                "chunk store range bytes",
            )?;
            push_bounded(
                &mut refs,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store range touched chunks",
            )?;
        }
    }
    if bytes.len() != expected_len {
        let message = format!("range reconstruction length mismatch: got {}, expected {}", bytes.len(), input.length);
        let receipt_value =
            denial_receipt_value("range-read", Some(&input.manifest.manifest_ref), input.refs, &message, vec![
                ("range-chunk-verification", "fail"),
                ("deny-range-reconstruction-mismatch", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(SpanData { bytes, refs })
}

pub fn range_read(root: &Path, manifest_ref: &str, offset: u64, length: u64) -> Result<ChunkStoreRangeRead> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("range-read", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if offset > manifest.total_len || offset.saturating_add(length) > manifest.total_len {
        let message =
            format!("range {offset}..{} outside object length {}", offset.saturating_add(length), manifest.total_len);
        let receipt_value =
            denial_receipt_value("range-read", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("range-bounds", "fail"),
                ("deny-out-of-bounds-range", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("range-read", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let span = read_span(SpanInput {
        root,
        manifest: &manifest,
        refs: &chunk_refs,
        offset,
        length,
    })?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "range-read",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &span.refs,
        checks: vec![
            ("range-bounds", "pass"),
            ("chunk-order-root", "pass"),
            ("range-chunk-verification", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![
            record("offset", vec![u64_value(offset)]),
            record("length", vec![u64_value(length)]),
        ],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreRangeRead {
        manifest_ref: manifest.manifest_ref,
        offset,
        length,
        bytes: span.bytes,
        receipt_value,
    })
}

pub fn missing_chunks(root: &Path, manifest_ref: &str) -> Result<Vec<String>> {
    let manifest = read_manifest(root, manifest_ref)?;
    let mut missing = Vec::new();
    let mut available = Vec::new();
    for chunk in &manifest.chunks {
        if chunk_path(root, &chunk.chunk_ref)?.exists() {
            push_bounded(
                &mut available,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store available chunks",
            )?;
        } else {
            push_bounded(&mut missing, chunk.chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store missing chunks")?;
        }
    }
    index_set_manifest_chunk_availability(root, &manifest, &available, &missing, None)?;
    Ok(missing)
}

pub fn sync_missing_chunks(source_root: &Path, dest_root: &Path, manifest_ref: &str) -> Result<ChunkStoreSync> {
    ensure_dirs(dest_root)?;
    let manifest = read_manifest(source_root, manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("remote-sync", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let manifest_bytes = fs::read(manifest_path(source_root, manifest_ref)?).map_err(MoltenError::from)?;
    write_immutable_bytes(
        &manifest_path(dest_root, manifest_ref)?,
        &manifest_bytes,
        manifest_ref,
        parse_canonical_bytes,
    )?;

    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan = scan_refs(dest_root, &manifest, chunk_size)?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "in-progress", &scan.missing_before, &[])?;
    index_set_manifest_chunk_availability(dest_root, &manifest, &scan.already_available, &scan.missing_before, None)?;

    let fetched_chunks = copy_refs(CopyInput {
        source_root,
        dest_root,
        manifest: &manifest,
        chunk_size,
        missing_before: &scan.missing_before,
    })?;
    verify_manifest(dest_root, manifest_ref)?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "remote-sync",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &fetched_chunks,
        checks: vec![
            ("manifest-identity-preserved", "pass"),
            ("missing-chunk-calculation", "pass"),
            ("redb-partial-fetch-state", "pass"),
            ("resumable-fetch", "pass"),
            ("streaming-chunk-verification", "pass"),
        ],
        details: vec![
            record("missing-before", vec![sequence(scan.missing_before.iter().map(string).collect())]),
            record("fetched", vec![sequence(fetched_chunks.iter().map(string).collect())]),
        ],
    });
    let available_after = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(dest_root, &manifest, &available_after, &[], Some(&receipt_value))?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "complete", &scan.missing_before, &fetched_chunks)?;
    Ok(ChunkStoreSync {
        manifest_ref: manifest.manifest_ref,
        missing_before: scan.missing_before,
        fetched_chunks,
        receipt_value,
    })
}

struct Scan {
    missing_before: Vec<String>,
    already_available: Vec<String>,
}

fn scan_refs(dest_root: &Path, manifest: &ChunkManifest, chunk_size: usize) -> Result<Scan> {
    let mut missing_before = Vec::new();
    let mut already_available = Vec::new();
    for chunk in &manifest.chunks {
        let dest_chunk_path = chunk_path(dest_root, &chunk.chunk_ref)?;
        if dest_chunk_path.exists() {
            read_verified_chunk(dest_root, chunk, chunk_size)?;
            push_bounded(
                &mut already_available,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store already available chunks",
            )?;
        } else {
            push_bounded(
                &mut missing_before,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store missing-before chunks",
            )?;
        }
    }
    Ok(Scan {
        missing_before,
        already_available,
    })
}

struct CopyInput<'a> {
    source_root: &'a Path,
    dest_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_size: usize,
    missing_before: &'a [String],
}

fn copy_refs(input: CopyInput<'_>) -> Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for chunk_ref in input.missing_before {
        let chunk = input
            .manifest
            .chunks
            .iter()
            .find(|candidate| &candidate.chunk_ref == chunk_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("manifest missing expected chunk {chunk_ref}")))?;
        let bytes = read_verified_chunk(input.source_root, chunk, input.chunk_size)?;
        fs::write(chunk_path(input.dest_root, &chunk.chunk_ref)?, bytes).map_err(MoltenError::from)?;
        push_bounded(
            &mut fetched_chunks,
            chunk.chunk_ref.clone(),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store fetched chunks",
        )?;
        index_set_partial_fetch(
            input.dest_root,
            &input.manifest.manifest_ref,
            "in-progress",
            input.missing_before,
            &fetched_chunks,
        )?;
    }
    Ok(fetched_chunks)
}

struct HeadInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_refs: &'a [String],
}

fn write_head(input: HeadInput<'_>) -> Result<String> {
    let manifest_bytes =
        fs::read(manifest_path(input.store_root, &input.manifest.manifest_ref)?).map_err(MoltenError::from)?;
    let manifest_blob_ref = hash_blob_bytes(&manifest_bytes);
    if manifest_blob_ref != input.manifest.manifest_ref {
        let message = format!(
            "Iroh manifest blob ref {manifest_blob_ref} does not preserve manifest identity {}",
            input.manifest.manifest_ref
        );
        let receipt_value =
            denial_receipt_value("iroh-publish", Some(&input.manifest.manifest_ref), input.chunk_refs, &message, vec![
                ("manifest-identity-preserved", "fail"),
                ("transport-does-not-grant-trust", "pass"),
            ]);
        store_receipt(input.store_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    write_immutable_blob(&iroh_blob_path(input.iroh_root, &manifest_blob_ref)?, &manifest_bytes, &manifest_blob_ref)?;
    Ok(manifest_blob_ref)
}

struct PartsInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_refs: &'a [String],
    chunk_size: usize,
}

fn write_parts(input: PartsInput<'_>) -> Result<Vec<IrohChunkBlob>> {
    let mut chunk_blobs = Vec::new();
    for chunk in &input.manifest.chunks {
        let bytes = match read_verified_chunk(input.store_root, chunk, input.chunk_size) {
            Ok(bytes) => bytes,
            Err(error) => {
                let receipt_value = denial_receipt_value(
                    "iroh-publish",
                    Some(&input.manifest.manifest_ref),
                    input.chunk_refs,
                    error.to_string(),
                    vec![
                        ("streaming-chunk-verification", "fail"),
                        ("deny-corrupt-or-missing-chunk", "pass"),
                    ],
                );
                store_receipt(input.store_root, &receipt_value)?;
                return Err(error);
            }
        };
        let blob_ref = hash_blob_bytes(&bytes);
        write_immutable_blob(&iroh_blob_path(input.iroh_root, &blob_ref)?, &bytes, &blob_ref)?;
        push_bounded(
            &mut chunk_blobs,
            IrohChunkBlob {
                chunk_ref: chunk.chunk_ref.clone(),
                blob_ref,
                length: chunk.length,
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh chunk blobs",
        )?;
    }
    Ok(chunk_blobs)
}

struct FinishInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    node: &'a str,
    manifest: ChunkManifest,
    manifest_blob_ref: String,
    chunk_blobs: Vec<IrohChunkBlob>,
    chunk_refs: &'a [String],
}

fn finish_pass(input: FinishInput<'_>) -> Result<ChunkStoreIrohPublish> {
    let ticket_value = iroh_ticket_value(&input.manifest.manifest_ref, &input.manifest_blob_ref, &input.chunk_blobs);
    let ticket_ref = canonical_hash(&ticket_value)?;
    write_immutable_bytes(
        &iroh_ticket_path(input.iroh_root, &input.manifest.manifest_ref)?,
        &canonical_bytes(&ticket_value)?,
        &ticket_ref,
        parse_canonical_bytes,
    )?;
    let ticket = format!("iroh-local-chunk:{}", input.manifest.manifest_ref);
    let chunk_blob_refs = input.chunk_blobs.iter().map(|chunk| chunk.blob_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "iroh-publish",
        decision: "pass",
        manifest_ref: Some(&input.manifest.manifest_ref),
        chunk_refs: input.chunk_refs,
        checks: vec![
            ("manifest-identity-preserved", "pass"),
            ("chunk-blob-verification", "pass"),
            ("iroh-location-hints-only", "pass"),
            ("transport-does-not-grant-trust", "pass"),
        ],
        details: vec![
            record("node", vec![string(input.node)]),
            record("ticket", vec![string(&ticket)]),
            record("ticket-ref", vec![string(&ticket_ref)]),
            record("manifest-blob-ref", vec![string(&input.manifest_blob_ref)]),
            record("chunk-blob-refs", vec![sequence(chunk_blob_refs.iter().map(string).collect())]),
        ],
    });
    store_receipt(input.store_root, &receipt_value)?;
    Ok(ChunkStoreIrohPublish {
        ticket,
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.manifest_blob_ref,
        chunk_blob_refs,
        receipt_value,
    })
}

fn manifest_refs(manifest: &ChunkManifest) -> Vec<String> {
    manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect()
}

fn claim_manifest(dest_root: &Path, ticket: &str, expected_manifest_ref: Option<&str>) -> Result<String> {
    let advertised_manifest_ref = match ticket.strip_prefix("iroh-local-chunk:") {
        Some(manifest_ref) => manifest_ref,
        None => {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                None,
                &[],
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
                vec![("ticket-shape", "fail"), ("deny-unsupported-ticket", "pass")],
            );
            store_receipt(dest_root, &receipt_value)?;
            return Err(MoltenError::invalid_harness(
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
            ));
        }
    };
    if let Some(expected) = expected_manifest_ref
        && expected != advertised_manifest_ref
    {
        let message = format!("Iroh chunk ticket advertises manifest {advertised_manifest_ref}, expected {expected}");
        let receipt_value = denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], &message, vec![
            ("ticket-manifest-binding", "fail"),
            ("deny-wrong-manifest", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(advertised_manifest_ref.to_string())
}

fn loaded_ticket(iroh_root: &Path, dest_root: &Path, advertised_manifest_ref: &str) -> Result<IrohChunkTicket> {
    let ticket_value = match read_iroh_ticket(iroh_root, advertised_manifest_ref) {
        Ok(ticket_value) => ticket_value,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], error.to_string(), vec![
                    ("ticket-availability", "fail"),
                    ("deny-missing-or-invalid-ticket", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    let parsed_ticket = match parse_iroh_ticket_value(&ticket_value) {
        Ok(parsed_ticket) => parsed_ticket,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], error.to_string(), vec![
                    ("ticket-shape", "fail"),
                    ("deny-missing-or-invalid-ticket", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if parsed_ticket.manifest_ref != advertised_manifest_ref {
        let message = format!(
            "Iroh ticket manifest {} does not match advertised manifest {advertised_manifest_ref}",
            parsed_ticket.manifest_ref
        );
        let receipt_value = denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], &message, vec![
            ("ticket-manifest-binding", "fail"),
            ("deny-wrong-manifest", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(parsed_ticket)
}

fn received_manifest(iroh_root: &Path, dest_root: &Path, parsed_ticket: &IrohChunkTicket) -> Result<ChunkManifest> {
    let manifest_bytes = match read_iroh_blob(iroh_root, &parsed_ticket.manifest_blob_ref) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], error.to_string(), vec![
                    ("manifest-blob-availability", "fail"),
                    ("deny-missing-manifest-blob", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if hash_blob_bytes(&manifest_bytes) != parsed_ticket.manifest_blob_ref {
        let message = format!("Iroh manifest blob {} failed blob hash verification", parsed_ticket.manifest_blob_ref);
        let receipt_value = denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], &message, vec![
            ("manifest-blob-verification", "fail"),
            ("deny-corrupt-manifest-blob", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let manifest_value = parse_canonical_bytes(&manifest_bytes)?;
    let manifest_ref = canonical_hash(&manifest_value)?;
    if manifest_ref != parsed_ticket.manifest_ref {
        let message = format!("Iroh manifest blob hashes to {manifest_ref}, expected {}", parsed_ticket.manifest_ref);
        let receipt_value = denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], &message, vec![
            ("manifest-identity-preserved", "fail"),
            ("transport-does-not-grant-trust", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    write_immutable_bytes(
        &manifest_path(dest_root, &manifest_ref)?,
        &manifest_bytes,
        &manifest_ref,
        parse_canonical_bytes,
    )?;
    parse_manifest_value(&manifest_value, Some(&manifest_ref))
}

fn ticket_parts(parsed_ticket: &IrohChunkTicket) -> OrderedMap<String, IrohChunkBlob> {
    parsed_ticket.chunks.iter().map(|chunk| (chunk.chunk_ref.clone(), chunk.clone())).collect()
}

fn require_part(
    parts: &OrderedMap<String, IrohChunkBlob>,
    dest_root: &Path,
    manifest: &ChunkManifest,
    refs: &[String],
    part_ref: &str,
) -> Result<()> {
    if parts.contains_key(part_ref) {
        return Ok(());
    }
    let message = format!("Iroh ticket lacks blob mapping for chunk {part_ref}");
    let receipt_value = denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, &message, vec![
        ("ticket-chunk-map", "fail"),
        ("deny-incomplete-ticket", "pass"),
    ]);
    store_receipt(dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

fn available_ref(
    dest_root: &Path,
    manifest: &ChunkManifest,
    refs: &[String],
    part: &ChunkRef,
    part_size: usize,
) -> Result<Option<String>> {
    let dest_chunk_path = chunk_path(dest_root, &part.chunk_ref)?;
    if !dest_chunk_path.exists() {
        return Ok(None);
    }
    match read_verified_chunk(dest_root, part, part_size) {
        Ok(_) => Ok(Some(part.chunk_ref.clone())),
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, error.to_string(), vec![
                    ("existing-chunk-verification", "fail"),
                    ("deny-corrupt-dedup-source", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            Err(error)
        }
    }
}

fn plan_incoming(
    dest_root: &Path,
    manifest: &ChunkManifest,
    refs: &[String],
    parts: &OrderedMap<String, IrohChunkBlob>,
    part_size: usize,
) -> Result<Scan> {
    let mut missing_before = Vec::new();
    let mut already_available = Vec::new();
    for part in &manifest.chunks {
        require_part(parts, dest_root, manifest, refs, &part.chunk_ref)?;
        if let Some(part_ref) = available_ref(dest_root, manifest, refs, part, part_size)? {
            push_bounded(
                &mut already_available,
                part_ref,
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store already available chunks",
            )?;
        } else {
            push_bounded(
                &mut missing_before,
                part.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store missing-before chunks",
            )?;
        }
    }
    Ok(Scan {
        missing_before,
        already_available,
    })
}

fn part_for<'a>(manifest: &'a ChunkManifest, part_ref: &str) -> Result<&'a ChunkRef> {
    manifest
        .chunks
        .iter()
        .find(|candidate| candidate.chunk_ref == part_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("manifest missing expected chunk {part_ref}")))
}

fn blob_for<'a>(parts: &'a OrderedMap<String, IrohChunkBlob>, part_ref: &str) -> Result<&'a IrohChunkBlob> {
    parts
        .get(part_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("Iroh ticket lacks blob mapping for chunk {part_ref}")))
}

fn require_blob_len(
    dest_root: &Path,
    manifest: &ChunkManifest,
    refs: &[String],
    part: &ChunkRef,
    blob: &IrohChunkBlob,
) -> Result<()> {
    if blob.length == part.length {
        return Ok(());
    }
    let message = format!(
        "Iroh ticket length {} for chunk {} does not match manifest length {}",
        blob.length, part.chunk_ref, part.length
    );
    let receipt_value = denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, &message, vec![
        ("ticket-chunk-map", "fail"),
        ("deny-incomplete-ticket", "pass"),
    ]);
    store_receipt(dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

struct BlobInput<'a> {
    iroh_root: &'a Path,
    dest_root: &'a Path,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    blob: &'a IrohChunkBlob,
}

fn blob_bytes(input: BlobInput<'_>) -> Result<Vec<u8>> {
    let bytes: Vec<u8> = match read_iroh_blob(input.iroh_root, &input.blob.blob_ref) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![("chunk-blob-availability", "fail"), ("deny-missing-chunk-blob", "pass")],
            );
            store_receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if hash_blob_bytes(&bytes) == input.blob.blob_ref {
        return Ok(bytes);
    }
    let message = format!("Iroh chunk blob {} failed blob hash verification", input.blob.blob_ref);
    let receipt_value =
        denial_receipt_value("iroh-fetch", Some(&input.manifest.manifest_ref), input.refs, &message, vec![
            ("chunk-blob-verification", "fail"),
            ("deny-corrupt-chunk-blob", "pass"),
        ]);
    store_receipt(input.dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

struct IncomingInput<'a> {
    iroh_root: &'a Path,
    dest_root: &'a Path,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    parts: &'a OrderedMap<String, IrohChunkBlob>,
    missing_before: &'a [String],
    part_size: usize,
}

fn copy_incoming(input: IncomingInput<'_>) -> Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for part_ref in input.missing_before {
        let part = part_for(input.manifest, part_ref)?;
        let blob = blob_for(input.parts, part_ref)?;
        require_blob_len(input.dest_root, input.manifest, input.refs, part, blob)?;
        let bytes = blob_bytes(BlobInput {
            iroh_root: input.iroh_root,
            dest_root: input.dest_root,
            manifest: input.manifest,
            refs: input.refs,
            blob,
        })?;
        if let Err(error) = verify_raw_chunk_bytes(&bytes, &part.chunk_ref, part.length, input.part_size) {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-chunk-blob", "pass"),
                ],
            );
            store_receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
        fs::write(chunk_path(input.dest_root, &part.chunk_ref)?, bytes).map_err(MoltenError::from)?;
        push_bounded(
            &mut fetched_chunks,
            part.chunk_ref.clone(),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store fetched chunks",
        )?;
        index_set_partial_fetch(
            input.dest_root,
            &input.manifest.manifest_ref,
            "in-progress",
            input.missing_before,
            &fetched_chunks,
        )?;
    }
    Ok(fetched_chunks)
}

struct FinishIncoming<'a> {
    dest_root: &'a Path,
    ticket_text: &'a str,
    peer: &'a str,
    manifest: ChunkManifest,
    parsed_ticket: IrohChunkTicket,
    missing_before: Vec<String>,
    fetched_chunks: Vec<String>,
}

fn finish_incoming(input: FinishIncoming<'_>) -> Result<ChunkStoreIrohFetch> {
    verify_manifest(input.dest_root, &input.manifest.manifest_ref)?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "iroh-fetch",
        decision: "pass",
        manifest_ref: Some(&input.manifest.manifest_ref),
        chunk_refs: &input.fetched_chunks,
        checks: vec![
            ("ticket-manifest-binding", "pass"),
            ("manifest-identity-preserved", "pass"),
            ("missing-chunk-calculation", "pass"),
            ("resumable-fetch", "pass"),
            ("streaming-chunk-verification", "pass"),
            ("transport-does-not-grant-trust", "pass"),
        ],
        details: vec![
            record("peer", vec![string(input.peer)]),
            record("ticket", vec![string(input.ticket_text)]),
            record("manifest-blob-ref", vec![string(&input.parsed_ticket.manifest_blob_ref)]),
            record("missing-before", vec![sequence(input.missing_before.iter().map(string).collect())]),
            record("fetched", vec![sequence(input.fetched_chunks.iter().map(string).collect())]),
        ],
    });
    let available_after = manifest_refs(&input.manifest);
    index_set_manifest_chunk_availability(
        input.dest_root,
        &input.manifest,
        &available_after,
        &[],
        Some(&receipt_value),
    )?;
    index_set_partial_fetch(
        input.dest_root,
        &input.manifest.manifest_ref,
        "complete",
        &input.missing_before,
        &input.fetched_chunks,
    )?;
    Ok(ChunkStoreIrohFetch {
        ticket: input.ticket_text.to_string(),
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.parsed_ticket.manifest_blob_ref,
        missing_before: input.missing_before,
        fetched_chunks: input.fetched_chunks,
        receipt_value,
    })
}

pub fn publish_iroh_blobs(
    store_root: &Path,
    iroh_root: &Path,
    manifest_ref: &str,
    node: &str,
) -> Result<ChunkStoreIrohPublish> {
    ensure_dirs(store_root)?;
    ensure_iroh_dirs(iroh_root)?;
    let manifest = match read_manifest(store_root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("iroh-publish", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(store_root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("iroh-publish", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(store_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }

    let manifest_blob_ref = write_head(HeadInput {
        store_root,
        iroh_root,
        manifest: &manifest,
        chunk_refs: &chunk_refs,
    })?;

    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let chunk_blobs = write_parts(PartsInput {
        store_root,
        iroh_root,
        manifest: &manifest,
        chunk_refs: &chunk_refs,
        chunk_size,
    })?;

    finish_pass(FinishInput {
        store_root,
        iroh_root,
        node,
        manifest,
        manifest_blob_ref,
        chunk_blobs,
        chunk_refs: &chunk_refs,
    })
}

pub fn fetch_iroh_blobs(
    iroh_root: &Path,
    dest_root: &Path,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
    peer: &str,
) -> Result<ChunkStoreIrohFetch> {
    ensure_dirs(dest_root)?;
    ensure_iroh_dirs(iroh_root)?;
    let advertised_manifest_ref = claim_manifest(dest_root, ticket, expected_manifest_ref)?;
    let parsed_ticket = loaded_ticket(iroh_root, dest_root, &advertised_manifest_ref)?;
    let manifest = received_manifest(iroh_root, dest_root, &parsed_ticket)?;
    let chunk_refs = manifest_refs(&manifest);
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }

    let ticket_chunks = ticket_parts(&parsed_ticket);
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan = plan_incoming(dest_root, &manifest, &chunk_refs, &ticket_chunks, chunk_size)?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "in-progress", &scan.missing_before, &[])?;
    index_set_manifest_chunk_availability(dest_root, &manifest, &scan.already_available, &scan.missing_before, None)?;

    let fetched_chunks = copy_incoming(IncomingInput {
        iroh_root,
        dest_root,
        manifest: &manifest,
        refs: &chunk_refs,
        parts: &ticket_chunks,
        missing_before: &scan.missing_before,
        part_size: chunk_size,
    })?;

    finish_incoming(FinishIncoming {
        dest_root,
        ticket_text: ticket,
        peer,
        manifest,
        parsed_ticket,
        missing_before: scan.missing_before,
        fetched_chunks,
    })
}

pub fn pin_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let manifest = read_manifest(root, manifest_ref)?;
    fs::write(manifest_pin_path(root, manifest_ref)?, manifest_ref).map_err(MoltenError::from)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "pin",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![("manifest-exists", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("manifest")])],
    });
    index_set_pin(root, "manifest", manifest_ref, true, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let pin_path = manifest_pin_path(root, manifest_ref)?;
    if pin_path.exists() {
        fs::remove_file(pin_path).map_err(MoltenError::from)?;
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "unpin",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: &[],
        checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("manifest")])],
    });
    index_set_pin(root, "manifest", manifest_ref, false, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn pin_chunk(root: &Path, chunk_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let path = chunk_path(root, chunk_ref)?;
    if !path.exists() {
        let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
            operation: "pin",
            decision: "deny",
            manifest_ref: None,
            chunk_refs: &[chunk_ref.to_string()],
            checks: vec![("chunk-exists", "fail"), ("deny-missing-chunk-pin", "pass")],
            details: vec![record("pin-kind", vec![string("chunk")])],
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(format!("cannot pin missing chunk {chunk_ref}")));
    }
    fs::write(chunk_pin_path(root, chunk_ref)?, chunk_ref).map_err(MoltenError::from)?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "pin",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &[chunk_ref.to_string()],
        checks: vec![("chunk-exists", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("chunk")])],
    });
    index_set_pin(root, "chunk", chunk_ref, true, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_chunk(root: &Path, chunk_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let pin_path = chunk_pin_path(root, chunk_ref)?;
    if pin_path.exists() {
        fs::remove_file(pin_path).map_err(MoltenError::from)?;
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "unpin",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &[chunk_ref.to_string()],
        checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("chunk")])],
    });
    index_set_pin(root, "chunk", chunk_ref, false, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn manifest_is_pinned(root: &Path, manifest_ref: &str) -> Result<bool> {
    validate_content_ref(manifest_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("chunk manifest pin ref is invalid: {error}")))?;
    Ok(manifest_pin_path(root, manifest_ref)?.exists())
}

pub fn chunk_is_pinned(root: &Path, chunk_ref: &str) -> Result<bool> {
    validate_content_ref(chunk_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("chunk pin ref is invalid: {error}")))?;
    Ok(chunk_pin_path(root, chunk_ref)?.exists())
}

fn pass_or_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

struct ApplyRefMatchInput<'a> {
    root: &'a Path,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = crate::retention::read_retention_gc_apply(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

struct GcTargets {
    manifests: Vec<String>,
    chunks: Vec<String>,
}

fn gc_targets(root: &Path, pinned_manifests: Vec<String>, mut reachable_chunks: Vec<String>) -> Result<GcTargets> {
    let mut manifests = Vec::new();
    for manifest_ref in list_manifest_refs(root)? {
        if pinned_manifests.iter().any(|pinned| pinned == &manifest_ref) {
            let manifest = read_manifest(root, &manifest_ref)?;
            for chunk in manifest.chunks {
                if !reachable_chunks.iter().any(|reachable| reachable == &chunk.chunk_ref) {
                    push_bounded(
                        &mut reachable_chunks,
                        chunk.chunk_ref,
                        MAX_CHUNK_STORE_CHUNKS,
                        "chunk store reachable chunks",
                    )?;
                }
            }
        } else {
            push_bounded(
                &mut manifests,
                manifest_ref.clone(),
                MAX_CHUNK_STORE_MANIFESTS,
                "chunk store removed manifests",
            )?;
        }
    }
    let mut chunks = Vec::new();
    for chunk_ref in list_chunk_refs(root)? {
        if reachable_chunks.iter().any(|reachable| reachable == &chunk_ref) {
            continue;
        }
        push_bounded(&mut chunks, chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store removed chunks")?;
    }
    Ok(GcTargets { manifests, chunks })
}

struct GcEnv<'a> {
    root: &'a Path,
    is_dry_run: bool,
    evidence: &'a crate::retention::DestructiveRetentionEvidence,
    apply_refs: &'a [String],
    action: &'a str,
    requester_ref: &'a str,
}

#[derive(Clone, Copy)]
struct GcObject<'a> {
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

#[derive(Default)]
struct GcNotes {
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipts: Vec<String>,
    execution_gates: Vec<String>,
    denials: Vec<String>,
}

impl GcNotes {
    fn consider(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> Result<()> {
        let admission = crate::retention::admit_destructive_retention_evidence(
            crate::retention::DestructiveRetentionAdmissionInput {
                root: env.root,
                evidence: env.evidence,
                object_ref: object.object_ref,
                object_kind: object.object_kind,
                retention_class: object.retention_class,
                action: env.action,
            },
        )?;
        self.note_admission(&admission)?;
        let evaluation = crate::retention::evaluate(crate::retention::EvaluationInput {
            root: env.root,
            object_ref: object.object_ref,
            object_kind: object.object_kind,
            retention_class: object.retention_class,
            action: env.action,
            requester_ref: env.requester_ref,
            is_reference_index_complete: env.evidence.is_reference_index_complete,
            retained_refs: &env.evidence.retained_refs,
            remote_refs: &env.evidence.remote_refs,
            policy_refs: &env.evidence.policy_refs,
            evidence_refs: &env.evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })?;
        push_bounded(
            &mut self.receipts,
            evaluation.receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention receipt refs",
        )?;
        let is_execution_denied = if env.is_dry_run {
            false
        } else {
            self.note_execution(env, object)?
        };
        if admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_execution_denied {
            push_bounded(
                &mut self.denials,
                object.object_ref.to_string(),
                MAX_CHUNK_STORE_REFS,
                "chunk store retention denials",
            )?;
        }
        Ok(())
    }

    fn note_admission(&mut self, admission: &crate::retention::DestructiveRetentionAdmission) -> Result<()> {
        for diagnostic in &admission.diagnostics {
            push_bounded(
                &mut self.admission_diagnostics,
                diagnostic.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission diagnostics",
            )?;
        }
        for reference in &admission.admitted_refs {
            push_bounded(
                &mut self.admission_refs,
                reference.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission refs",
            )?;
        }
        Ok(())
    }

    fn note_execution(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> Result<bool> {
        let apply_ref = matching_apply_ref(ApplyRefMatchInput {
            root: env.root,
            apply_refs: env.apply_refs,
            subsystem: "chunk-gc",
            action: env.action,
            object_ref: object.object_ref,
            object_kind: object.object_kind,
            retention_class: object.retention_class,
        });
        let execution_gate =
            crate::retention::store_retention_gc_execution_gate(crate::retention::RetentionGcExecutionGateInput {
                root: env.root,
                subsystem: "chunk-gc",
                action: env.action,
                object_ref: object.object_ref,
                object_kind: object.object_kind,
                retention_class: object.retention_class,
                apply_ref,
            })?;
        push_bounded(
            &mut self.execution_gates,
            execution_gate.execution_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention execution gate refs",
        )?;
        if execution_gate.decision == "pass" {
            return Ok(false);
        }
        for diagnostic in &execution_gate.diagnostics {
            push_bounded(
                &mut self.execution_diagnostics,
                diagnostic.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention execution diagnostics",
            )?;
        }
        Ok(true)
    }
}

#[derive(Clone, Copy)]
struct GcReceiptInput<'a> {
    is_dry_run: bool,
    decision: &'a str,
    removed_manifests: &'a [String],
    removed_chunks: &'a [String],
    notes: &'a GcNotes,
    evidence_summary: &'a IoValue,
}

fn gc_receipt_value(input: GcReceiptInput<'_>) -> IoValue {
    receipt_value(ChunkStoreReceiptValueInput {
        operation: "gc",
        decision: input.decision,
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("deny-incomplete-reachability-proof", "pass"),
            ("chunk-tombstone-eligibility", if input.decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            (
                "retention-execution-gate",
                pass_or_fail(input.is_dry_run || input.notes.execution_diagnostics.is_empty()),
            ),
            ("retention-authority-evidence", pass_or_fail(input.notes.admission_diagnostics.is_empty())),
            ("redb-index-update", if input.decision == "pass" { "pass" } else { "fail" }),
        ],
        details: vec![
            record("mode", vec![string(if input.is_dry_run { "dry-run" } else { "apply" })]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("denied", vec![sequence(input.notes.denials.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
            record("retention-diagnostics", vec![sequence(
                input.notes.admission_diagnostics.iter().map(string).collect(),
            )]),
            record("retention-execution-diagnostics", vec![sequence(
                input.notes.execution_diagnostics.iter().map(string).collect(),
            )]),
        ],
    })
}

fn gc_tombstone_value(input: GcReceiptInput<'_>) -> Option<IoValue> {
    if input.is_dry_run
        || input.decision != "pass"
        || (input.removed_manifests.is_empty() && input.removed_chunks.is_empty())
    {
        return None;
    }
    Some(self::receipt_value(ChunkStoreReceiptValueInput {
        operation: "tombstone",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("tombstone-eligibility", "pass"),
            ("gc-mode-binding", "pass"),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", "pass"),
            ("retention-authority-evidence", "pass"),
        ],
        details: vec![
            record("mode", vec![string("apply")]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
        ],
    }))
}

struct GcFinishInput<'a> {
    root: &'a Path,
    is_dry_run: bool,
    targets: GcTargets,
    notes: GcNotes,
    evidence_summary: IoValue,
}

fn finish_gc(input: GcFinishInput<'_>) -> Result<ChunkStoreGc> {
    let decision = if input.notes.denials.is_empty() { "pass" } else { "deny" };
    let mut removed_manifests = Vec::new();
    let mut removed_chunks = Vec::new();
    if decision == "pass" {
        removed_manifests = input.targets.manifests;
        removed_chunks = input.targets.chunks;
        if !input.is_dry_run {
            for manifest_ref in &removed_manifests {
                fs::remove_file(manifest_path(input.root, manifest_ref)?).map_err(MoltenError::from)?;
            }
            for chunk_ref in &removed_chunks {
                fs::remove_file(chunk_path(input.root, chunk_ref)?).map_err(MoltenError::from)?;
            }
        }
    }
    let receipt_input = GcReceiptInput {
        is_dry_run: input.is_dry_run,
        decision,
        removed_manifests: &removed_manifests,
        removed_chunks: &removed_chunks,
        notes: &input.notes,
        evidence_summary: &input.evidence_summary,
    };
    let receipt_value = gc_receipt_value(receipt_input);
    let tombstone_receipt = gc_tombstone_value(receipt_input);
    index_apply_gc(&IndexApplyGcInput {
        root: input.root,
        dry_run: input.is_dry_run,
        removed_manifests: &removed_manifests,
        removed_chunks: &removed_chunks,
        receipt_value: &receipt_value,
        tombstone_receipt: tombstone_receipt.as_ref(),
    })?;
    Ok(ChunkStoreGc {
        dry_run: input.is_dry_run,
        decision: decision.to_string(),
        removed_manifests,
        removed_chunks,
        retention_receipt_refs: input.notes.receipts,
        execution_gate_refs: input.notes.execution_gates,
        receipt_value,
    })
}

pub fn gc(root: &Path, input: ChunkStoreGcInput<'_>) -> Result<ChunkStoreGc> {
    ensure_dirs(root)?;
    let targets = gc_targets(
        root,
        pinned_refs(&root.join("pins").join("manifests"))?,
        pinned_refs(&root.join("pins").join("chunks"))?,
    )?;
    let action = if input.dry_run {
        crate::retention::ACTION_ELIGIBILITY
    } else {
        crate::retention::ACTION_DELETE
    };
    let requester_ref = crate::retention::destructive_retention_requester_ref(
        input.retention_evidence,
        "chunk-store-gc-missing-requester",
    )?;
    let evidence_summary = crate::retention::destructive_retention_evidence_value(input.retention_evidence)?;
    let env = GcEnv {
        root,
        is_dry_run: input.dry_run,
        evidence: input.retention_evidence,
        apply_refs: input.apply_refs,
        action,
        requester_ref: &requester_ref,
    };
    let mut notes = GcNotes::default();
    for manifest_ref in &targets.manifests {
        notes.consider(&env, GcObject {
            object_ref: manifest_ref,
            object_kind: "chunk-manifest",
            retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT,
        })?;
    }
    for chunk_ref in &targets.chunks {
        notes.consider(&env, GcObject {
            object_ref: chunk_ref,
            object_kind: "chunk",
            retention_class: crate::retention::CLASS_DURABLE_VALUE,
        })?;
    }
    finish_gc(GcFinishInput {
        root,
        is_dry_run: input.dry_run,
        targets,
        notes,
        evidence_summary,
    })
}

pub fn list_manifest_refs(root: &Path) -> Result<Vec<String>> {
    refs_from_dir(&root.join("manifests"))
}

pub fn list_chunk_refs(root: &Path) -> Result<Vec<String>> {
    refs_from_dir(&root.join("chunks"))
}

pub fn list_receipt_refs(root: &Path) -> Result<Vec<String>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        push_bounded(&mut refs, key.value().to_string(), MAX_CHUNK_STORE_RECEIPTS, "chunk store receipt refs")?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ChunkStoreReceipt> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown chunk store receipt {receipt_ref}")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_receipt_value(&value, Some(receipt_ref))
}

pub fn build_chunk_lineage(root: &Path, manifest_ref: &str) -> Result<ChunkLineage> {
    let manifest = read_manifest(root, manifest_ref)?;
    let mut receipts = list_receipt_refs(root)?
        .into_iter()
        .map(|receipt_ref| read_receipt(root, &receipt_ref))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|receipt| receipt.decision == "pass" && receipt.manifest_ref.as_deref() == Some(manifest_ref))
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        lineage_operation_rank(&left.operation)
            .cmp(&lineage_operation_rank(&right.operation))
            .then_with(|| left.receipt_ref.cmp(&right.receipt_ref))
    });
    if receipts.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "no pass chunk-store receipts available for lineage manifest {manifest_ref}"
        )));
    }

    let chain = crate::evidence_chain::ChainScope::new(
        "chunk-lineage",
        manifest.manifest_ref.clone(),
        manifest.root_ref.clone(),
    );
    let producer = lineage_producer()?;
    let series = link_series(&manifest, &receipts, &chain, &producer)?;
    let evidence = pass_evidence(&chain, &manifest, &series.refs, &series.receipt_refs)?;
    let value = lineage_value(&LineageValueInput {
        manifest_ref: &manifest.manifest_ref,
        root_ref: &manifest.root_ref,
        link_values: &series.values,
        receipt_values: &series.receipt_values,
        verify_receipt_value: &evidence.verify_value,
        predicate_values: &evidence.predicate_values,
    });
    let lineage_ref = canonical_hash(&value)?;
    Ok(ChunkLineage {
        lineage_ref,
        manifest_ref: manifest.manifest_ref,
        root_ref: manifest.root_ref,
        link_refs: series.refs,
        receipt_refs: series.receipt_refs,
        verify_receipt_ref: evidence.verify_ref,
        predicate_receipt_refs: evidence.predicate_refs,
        value,
    })
}

struct LinkSeries {
    refs: Vec<String>,
    values: Vec<IoValue>,
    receipt_refs: Vec<String>,
    receipt_values: Vec<IoValue>,
}

fn link_series(
    manifest: &ChunkManifest,
    receipts: &[ChunkStoreReceipt],
    chain: &crate::evidence_chain::ChainScope,
    producer: &crate::evidence_chain::ChainProducer,
) -> Result<LinkSeries> {
    ensure_count_at_most(receipts.len(), MAX_CHUNK_STORE_RECEIPTS, "chunk lineage receipts")?;
    let mut links = Vec::with_capacity(receipts.len());
    let mut values = Vec::with_capacity(receipts.len());
    let mut receipt_refs = Vec::with_capacity(receipts.len());
    let mut receipt_values = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let payload = crate::evidence_chain::ChainPayload::new(
            "chunk-store-receipt",
            receipt.receipt_ref.clone(),
            CHUNK_STORE_RECEIPT_SCHEMA,
        );
        let trellis_input_ref = canonical_hash(&record("chunk-lineage-input", vec![
            string(&manifest.manifest_ref),
            string(&manifest.root_ref),
            string(&receipt.receipt_ref),
            string(&receipt.operation),
        ]))?;
        let input = if let Some(previous) = links.last() {
            crate::evidence_chain::ChainLinkInput::append(
                previous,
                payload,
                lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        } else {
            crate::evidence_chain::ChainLinkInput::genesis(
                chain.clone(),
                payload,
                lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        };
        let link_value = crate::evidence_chain::chain_link_value(&input);
        let link = crate::evidence_chain::parse_chain_link(&link_value)?;
        push_bounded(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        push_bounded(
            &mut receipt_values,
            receipt.value.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt values",
        )?;
        push_bounded(&mut values, link_value, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage link values")?;
        push_bounded(&mut links, link, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage links")?;
    }
    Ok(LinkSeries {
        refs: links.iter().map(|link| link.link_ref.clone()).collect(),
        values,
        receipt_refs,
        receipt_values,
    })
}

struct PassEvidence {
    predicate_values: Vec<IoValue>,
    predicate_refs: Vec<String>,
    verify_value: IoValue,
    verify_ref: String,
}

struct Ends {
    head_ref: String,
    anchor_ref: String,
}

fn pass_evidence(
    chain: &crate::evidence_chain::ChainScope,
    manifest: &ChunkManifest,
    link_refs: &[String],
    receipt_refs: &[String],
) -> Result<PassEvidence> {
    let ends = chain_ends(link_refs)?;
    let predicate_values = predicate_set(PredicateInput {
        manifest,
        link_refs,
        receipt_refs,
        ends: &ends,
    });
    let predicate_refs = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect::<Vec<_>>();
    let verify_value = verify_value(VerifyInput {
        chain,
        link_refs,
        receipt_refs,
        ends: &ends,
        predicate_refs: &predicate_refs,
    });
    let verify_ref = canonical_hash(&verify_value)?;
    Ok(PassEvidence {
        predicate_values,
        predicate_refs,
        verify_value,
        verify_ref,
    })
}

fn chain_ends(link_refs: &[String]) -> Result<Ends> {
    let head_ref = link_refs
        .last()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage requires at least one chain link"))?;
    let anchor_ref = link_refs
        .first()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage requires at least one chain link"))?;
    Ok(Ends { head_ref, anchor_ref })
}

struct PredicateInput<'a> {
    manifest: &'a ChunkManifest,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
}

fn predicate_set(input: PredicateInput<'_>) -> Vec<IoValue> {
    let context_refs = vec![
        input.manifest.manifest_ref.clone(),
        input.manifest.root_ref.clone(),
        input.manifest.metadata_ref.clone(),
    ];
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_subject_refs = vec![input.ends.anchor_ref.clone(), input.ends.head_ref.clone()];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    let checkpoint_checks = vec![
        crate::evidence_chain::ChainCheck::pass("checkpoint-range-coverage"),
        crate::evidence_chain::ChainCheck::pass("verified-range"),
    ];
    vec![
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &segment_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            decision: "pass",
            subject_refs: std::slice::from_ref(&input.ends.head_ref),
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &fork_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: "pass",
            subject_refs: &anchor_subject_refs,
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &anchor_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &checkpoint_checks,
        }),
    ]
}

struct VerifyInput<'a> {
    chain: &'a crate::evidence_chain::ChainScope,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
    predicate_refs: &'a [String],
}

fn verify_value(input: VerifyInput<'_>) -> IoValue {
    let verify_diagnostics = Vec::new();
    let verify_receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain: input.chain,
        anchor_ref: Some(&input.ends.anchor_ref),
        expected_head: Some(&input.ends.head_ref),
        discovered_heads: std::slice::from_ref(&input.ends.head_ref),
        verified_links: input.link_refs,
        payload_refs: input.receipt_refs,
        diagnostics: &verify_diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt: verify_receipt,
            predicate_receipt_refs: input.predicate_refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

pub fn parse_chunk_lineage_value(value: &IoValue) -> Result<ChunkLineage> {
    let fields = simple_record(value, "chunk-lineage-v1", 8)?;
    require_schema(&fields[0], CHUNK_LINEAGE_SCHEMA, "chunk lineage")?;
    let manifest_ref = record_string(&fields[1], "manifest")?;
    let root_ref = record_string(&fields[2], "root")?;
    filename_for_ref(&manifest_ref)?;
    filename_for_ref(&root_ref)?;
    let link_values = record_sequence(&fields[3], "links")?;
    let receipt_values = record_sequence(&fields[4], "receipts")?;
    let verify_receipt_value = lineage_record_value(&fields[5], "verify-receipt")?;
    let predicate_values = record_sequence(&fields[6], "predicates")?;
    let checks = parse_lineage_checks(&fields[7])?;
    require_lineage_check(&checks, "manifest-root-binding")?;
    require_lineage_check(&checks, "receipt-payload-binding")?;
    require_lineage_check(&checks, "lineage-no-global-head")?;
    require_lineage_check(&checks, "lineage-continuity")?;
    require_lineage_check(&checks, "lineage-predicate-receipts")?;
    if link_values.is_empty() || link_values.len() != receipt_values.len() {
        return Err(MoltenError::invalid_harness(
            "chunk lineage must contain matching non-empty link and receipt sequences",
        ));
    }

    let receipts = receipt_values
        .iter()
        .map(|receipt_value| parse_receipt_value(receipt_value, None))
        .collect::<Result<Vec<_>>>()?;
    let entries = parsed_entries(EntryInput {
        manifest_ref: &manifest_ref,
        root_ref: &root_ref,
        link_values: &link_values,
        receipts: &receipts,
    })?;

    let predicates = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?;
    let predicate_receipt_refs = predicates.iter().map(|predicate| predicate.receipt_ref.clone()).collect::<Vec<_>>();
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    let range_predicate =
        require_chunk_lineage_predicate(&predicates, crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE)?;
    if range_predicate.subject_refs.as_slice() != entries.link_refs.as_slice()
        || range_predicate.input_refs.as_slice() != entries.receipt_refs.as_slice()
    {
        return Err(MoltenError::invalid_harness(
            "chunk lineage range predicate does not bind lineage links and receipts",
        ));
    }
    validate_chunk_lineage_verify_receipt(
        &verify_receipt_value,
        &entries.first_chain,
        &entries.link_refs,
        &entries.receipt_refs,
        &predicate_receipt_refs,
    )?;
    let verify_receipt_ref = canonical_hash(&verify_receipt_value)?;
    Ok(ChunkLineage {
        lineage_ref: canonical_hash(value)?,
        manifest_ref,
        root_ref,
        link_refs: entries.link_refs,
        receipt_refs: entries.receipt_refs,
        verify_receipt_ref,
        predicate_receipt_refs,
        value: value.clone(),
    })
}

struct EntryInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    link_values: &'a [IoValue],
    receipts: &'a [ChunkStoreReceipt],
}

struct ParsedEntries {
    first_chain: crate::evidence_chain::ChainScope,
    link_refs: Vec<String>,
    receipt_refs: Vec<String>,
}

fn parsed_entries(input: EntryInput<'_>) -> Result<ParsedEntries> {
    let mut first_chain = None;
    let mut link_refs = Vec::with_capacity(input.link_values.len());
    let mut receipt_refs = Vec::with_capacity(input.receipts.len());
    for (position, (link_value, receipt)) in input.link_values.iter().zip(input.receipts.iter()).enumerate() {
        let previous_ref = if position == 0 {
            None
        } else {
            link_refs.get(position - 1).map(String::as_str)
        };
        let entry = checked_entry(LinkInput {
            manifest_ref: input.manifest_ref,
            root_ref: input.root_ref,
            position,
            previous_ref,
            value: link_value,
            receipt,
        })?;
        if position == 0 {
            first_chain = Some(entry.chain);
        }
        push_bounded(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        push_bounded(&mut link_refs, entry.link_ref, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage link refs")?;
    }
    let first_chain = first_chain.ok_or_else(|| MoltenError::invalid_harness("chunk lineage missing first link"))?;
    Ok(ParsedEntries {
        first_chain,
        link_refs,
        receipt_refs,
    })
}

struct LinkInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    position: usize,
    previous_ref: Option<&'a str>,
    value: &'a IoValue,
    receipt: &'a ChunkStoreReceipt,
}

struct CheckedEntry {
    chain: crate::evidence_chain::ChainScope,
    link_ref: String,
}

fn checked_entry(input: LinkInput<'_>) -> Result<CheckedEntry> {
    if input.receipt.manifest_ref.as_deref() != Some(input.manifest_ref) || input.receipt.decision != "pass" {
        return Err(MoltenError::invalid_harness(
            "chunk lineage receipt does not bind the lineage manifest as pass evidence",
        ));
    }
    let link = crate::evidence_chain::parse_chain_link(input.value)?;
    if link.chain.scope != "chunk-lineage" || link.chain.id != input.manifest_ref || link.chain.epoch != input.root_ref
    {
        return Err(MoltenError::invalid_harness("chunk lineage link scope must be per manifest/root, not global"));
    }
    if link.sequence != input.position as u64 {
        return Err(MoltenError::invalid_harness("chunk lineage link sequence is not contiguous"));
    }
    if input.position == 0 {
        if link.previous_link_ref.is_some() {
            return Err(MoltenError::invalid_harness("chunk lineage genesis link must not name a previous link"));
        }
    } else if link.previous_link_ref.as_deref() != input.previous_ref {
        return Err(MoltenError::invalid_harness("chunk lineage link does not bind previous lineage receipt"));
    }
    if link.payload.artifact_ref != input.receipt.receipt_ref || link.payload.schema != CHUNK_STORE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(
            "chunk lineage link payload does not bind embedded chunk-store receipt",
        ));
    }
    require_lineage_context(&link.context_refs, "manifest", input.manifest_ref)?;
    require_lineage_context(&link.context_refs, "chunk-root", input.root_ref)?;
    for chunk_ref in &input.receipt.chunk_refs {
        require_lineage_context(&link.context_refs, "chunk", chunk_ref)?;
    }
    Ok(CheckedEntry {
        chain: link.chain,
        link_ref: link.link_ref,
    })
}

fn lineage_operation_rank(operation: &str) -> (u8, &str) {
    let rank = match operation {
        "manifest-create" => 0,
        "verify" => 1,
        "fetch" => 2,
        "iroh-publish" => 3,
        "iroh-fetch" => 4,
        "range-read" => 5,
        "dedup-hit" => 6,
        "pin" => 7,
        "unpin" => 8,
        "gc" => 9,
        _ => 100,
    };
    (rank, operation)
}

fn lineage_producer() -> Result<crate::evidence_chain::ChainProducer> {
    Ok(crate::evidence_chain::ChainProducer::new(
        "molten-chunk-lineage",
        canonical_hash(&record("chunk-lineage-producer-key", vec![string("molten")]))?,
    ))
}

fn lineage_context_refs(
    manifest: &ChunkManifest,
    receipt: &ChunkStoreReceipt,
) -> Result<Vec<crate::evidence_chain::ChainContextRef>> {
    let mut refs = vec![
        crate::evidence_chain::ChainContextRef::new("manifest", manifest.manifest_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("chunk-root", manifest.root_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("metadata", manifest.metadata_ref.clone()),
        crate::evidence_chain::ChainContextRef::new(
            "operation",
            canonical_hash(&record("chunk-lineage-operation", vec![string(&receipt.operation)]))?,
        ),
    ];
    for chunk in &manifest.chunks {
        push_bounded(
            &mut refs,
            crate::evidence_chain::ChainContextRef::new("chunk", chunk.chunk_ref.clone()),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage context refs",
        )?;
    }
    for detail in &receipt.details {
        collect_detail_context_refs(DetailContextRefsInput {
            value: detail,
            refs: &mut refs,
        })?;
    }
    Ok(refs)
}

struct DetailContextRefsInput<'a> {
    value: &'a IoValue,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs(input: DetailContextRefsInput<'_>) -> Result<()> {
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, input.value.clone(), MAX_CHUNK_STORE_CONTEXT_REFS, "chunk lineage detail scan values")?;
    let refs = input.refs;
    while let Some(current) = pending.pop() {
        if let Some(text) = current.as_string() {
            collect_detail_context_refs_push_text(DetailTextInput {
                text: text.into_owned(),
                refs,
            })?;
            continue;
        }
        collect_detail_context_refs_push_children(DetailChildInput {
            value: &current,
            pending: &mut pending,
        })?;
    }
    Ok(())
}

struct DetailTextInput<'a> {
    text: String,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs_push_text(input: DetailTextInput<'_>) -> Result<()> {
    if validate_content_ref(&input.text).is_ok() {
        push_bounded(
            input.refs,
            crate::evidence_chain::ChainContextRef::new("detail-ref", input.text),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    } else if input.text.starts_with("iroh-local-chunk:") {
        push_bounded(
            input.refs,
            crate::evidence_chain::ChainContextRef::new(
                "ticket",
                canonical_hash(&record("iroh-ticket", vec![string(input.text)]))?,
            ),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    }
    Ok(())
}

struct DetailChildInput<'a> {
    value: &'a IoValue,
    pending: &'a mut Vec<IoValue>,
}

fn collect_detail_context_refs_push_children(input: DetailChildInput<'_>) -> Result<()> {
    if let Some(sequence) = input.value.collect_sequence() {
        for item in sequence.iter().rev() {
            collect_detail_context_refs_push_child(DetailPushInput {
                values: input.pending,
                value: value_to_iovalue(item),
            })?;
        }
        return Ok(());
    }
    match input.value.value_class() {
        ValueClass::Atomic(_) | ValueClass::Embedded => {}
        ValueClass::Compound(CompoundClass::Record)
        | ValueClass::Compound(CompoundClass::Sequence)
        | ValueClass::Compound(CompoundClass::Set) => {
            let mut children = Vec::new();
            for child in input.value.iter() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&child),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
        ValueClass::Compound(CompoundClass::Dictionary) => {
            let mut children = Vec::new();
            for (key, value) in input.value.entries() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&key),
                })?;
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&value),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
    }
    Ok(())
}

struct DetailPushInput<'a> {
    values: &'a mut Vec<IoValue>,
    value: IoValue,
}

fn collect_detail_context_refs_push_child(input: DetailPushInput<'_>) -> Result<()> {
    push_bounded(input.values, input.value, MAX_CHUNK_STORE_CONTEXT_REFS, "chunk lineage detail scan values")
}

struct LineageValueInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    link_values: &'a [IoValue],
    receipt_values: &'a [IoValue],
    verify_receipt_value: &'a IoValue,
    predicate_values: &'a [IoValue],
}

fn lineage_value(input: &LineageValueInput<'_>) -> IoValue {
    record("chunk-lineage-v1", vec![
        string(CHUNK_LINEAGE_SCHEMA),
        record("manifest", vec![string(input.manifest_ref)]),
        record("root", vec![string(input.root_ref)]),
        record("links", vec![sequence(input.link_values.to_vec())]),
        record("receipts", vec![sequence(input.receipt_values.to_vec())]),
        record("verify-receipt", vec![input.verify_receipt_value.clone()]),
        record("predicates", vec![sequence(input.predicate_values.to_vec())]),
        record("checks", vec![sequence(
            [
                "manifest-root-binding",
                "receipt-payload-binding",
                "lineage-no-global-head",
                "lineage-continuity",
                "lineage-predicate-receipts",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn parse_lineage_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let checks = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <checks ...> field"))?;
    let check_values = checks[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected sequence for lineage checks"))?;
    let mut parsed = Vec::new();
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "lineage check name")?;
        let status = required_string(&check[1], "lineage check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("lineage check {name} status is {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CHUNK_STORE_CHECKS, "chunk lineage checks")?;
    }
    Ok(parsed)
}

fn require_lineage_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chunk lineage missing {expected} check")))
    }
}

fn lineage_record_value(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    Ok(value_to_iovalue(&record[0]))
}

fn require_lineage_context(
    context_refs: &[crate::evidence_chain::ChainContextRef],
    label: &str,
    expected: &str,
) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label && context.artifact_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chunk lineage link missing {label} context ref {expected}")))
    }
}

fn require_chunk_lineage_predicate<'a>(
    predicates: &'a [crate::evidence_chain::ChainPredicateReceipt],
    expected_kind: &str,
) -> Result<&'a crate::evidence_chain::ChainPredicateReceipt> {
    predicates
        .iter()
        .find(|predicate| predicate.predicate == expected_kind && predicate.decision == "pass")
        .ok_or_else(|| {
            MoltenError::invalid_harness(format!("chunk lineage missing passing {expected_kind} predicate receipt"))
        })
}

fn validate_chunk_lineage_verify_receipt(
    value: &IoValue,
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    receipt_refs: &[String],
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "chunk lineage verify schema")?;
    if schema != crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk lineage verify schema {schema}")));
    }
    let decision = record_string(&receipt[1], "decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chunk lineage verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = parse_lineage_chain_scope(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt chain scope mismatch"));
    }
    let anchor_ref = record_optional_ref(&receipt[3], "anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage verify receipt missing anchor"))?;
    let expected_head = record_optional_ref(&receipt[4], "expected-head")?
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage verify receipt missing expected head"))?;
    if Some(&anchor_ref) != link_refs.first() || Some(&expected_head) != link_refs.last() {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt does not bind lineage anchor/head"));
    }
    if record_string_sequence(&receipt[5], "discovered-heads")? != vec![expected_head] {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt discovered head mismatch"));
    }
    if record_string_sequence(&receipt[6], "verified-links")? != link_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt links mismatch"));
    }
    if record_string_sequence(&receipt[7], "payloads")? != receipt_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt payload refs mismatch"));
    }
    if record_string_sequence(&receipt[8], "predicates")? != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt predicate refs mismatch"));
    }
    Ok(())
}

fn parse_lineage_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain scope field"))?;
    Ok(crate::evidence_chain::ChainScope::new(
        record_string(&chain[0], "scope")?,
        record_string(&chain[1], "id")?,
        record_string(&chain[2], "epoch")?,
    ))
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_string(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {label}")))
    }
}

pub fn index_status(root: &Path) -> Result<ChunkStoreIndexStatus> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let manifests = read_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?.len().map_err(index_error)?;
    let chunks = read_txn.open_table(INDEX_CHUNKS).map_err(index_error)?.len().map_err(index_error)?;
    let availability = read_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
    let mut available_chunks = 0;
    let mut missing_chunks = 0;
    for item in availability.iter().map_err(index_error)? {
        let (_key, value) = item.map_err(index_error)?;
        match value.value() {
            "available" => available_chunks += 1,
            "missing" => missing_chunks += 1,
            _ => {}
        }
    }
    let pins = read_txn.open_table(INDEX_PINS).map_err(index_error)?;
    let mut manifest_pins = 0;
    let mut chunk_pins = 0;
    for item in pins.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        if key.value().starts_with("manifest:") {
            manifest_pins += 1;
        } else if key.value().starts_with("chunk:") {
            chunk_pins += 1;
        }
    }
    let partial_fetches =
        read_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?.len().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?.len().map_err(index_error)?;
    Ok(ChunkStoreIndexStatus {
        manifests,
        chunks,
        available_chunks,
        missing_chunks,
        manifest_pins,
        chunk_pins,
        partial_fetches,
        receipts,
    })
}

pub fn rebuild_index(root: &Path) -> Result<ChunkStoreIndexRebuild> {
    ensure_dirs(root)?;
    let inputs = scan_inputs(root)?;
    let receipt_value = write_inputs(root, &inputs)?;
    Ok(ChunkStoreIndexRebuild {
        status: index_status(root)?,
        receipt_value,
    })
}

struct IndexInputs {
    manifest_entries: Vec<(String, Vec<u8>, ChunkManifest)>,
    chunk_entries: OrderedMap<String, (ChunkRef, String)>,
    pinned_manifests: Vec<String>,
    pinned_chunks: Vec<String>,
}

fn scan_inputs(root: &Path) -> Result<IndexInputs> {
    let mut manifest_entries = Vec::new();
    let mut chunk_entries: OrderedMap<String, (ChunkRef, String)> = OrderedMap::new();
    for manifest_ref in list_manifest_refs(root)? {
        let bytes = fs::read(manifest_path(root, &manifest_ref)?).map_err(MoltenError::from)?;
        let value = parse_canonical_bytes(&bytes)?;
        let manifest = parse_manifest_value(&value, Some(&manifest_ref))?;
        scan_manifest_chunks(root, &manifest, &mut chunk_entries)?;
        push_bounded(
            &mut manifest_entries,
            (manifest_ref, bytes, manifest),
            MAX_CHUNK_STORE_MANIFESTS,
            "chunk store index manifest entries",
        )?;
    }
    Ok(IndexInputs {
        manifest_entries,
        chunk_entries,
        pinned_manifests: pinned_refs(&root.join("pins").join("manifests"))?,
        pinned_chunks: pinned_refs(&root.join("pins").join("chunks"))?,
    })
}

fn scan_manifest_chunks(
    root: &Path,
    manifest: &ChunkManifest,
    chunk_entries: &mut OrderedMap<String, (ChunkRef, String)>,
) -> Result<()> {
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let available = if chunk_path(root, &chunk.chunk_ref)?.exists() {
            read_verified_chunk(root, chunk, chunk_size)?;
            "available"
        } else {
            "missing"
        };
        chunk_entries
            .entry(chunk.chunk_ref.clone())
            .and_modify(|(_existing, status)| {
                if available == "available" {
                    *status = "available".to_string();
                }
            })
            .or_insert_with(|| (chunk.clone(), available.to_string()));
    }
    Ok(())
}

fn write_inputs(root: &Path, inputs: &IndexInputs) -> Result<IoValue> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_index_tables_in_tx(&write_txn)?;
    write_manifest_entries(&write_txn, &inputs.manifest_entries)?;
    write_entries(&write_txn, &inputs.chunk_entries)?;
    write_pin_entries(&write_txn, &inputs.pinned_manifests, &inputs.pinned_chunks)?;
    let receipt_value = rebuild_receipt(inputs);
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    drop(db);
    Ok(receipt_value)
}

fn write_manifest_entries(
    write_txn: &redb::WriteTransaction,
    manifest_entries: &[(String, Vec<u8>, ChunkManifest)],
) -> Result<()> {
    let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
    for (manifest_ref, bytes, _manifest) in manifest_entries {
        manifests.insert(manifest_ref.as_str(), bytes.as_slice()).map_err(index_error)?;
    }
    Ok(())
}

fn write_entries(write_txn: &redb::WriteTransaction, entries: &OrderedMap<String, (ChunkRef, String)>) -> Result<()> {
    let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
    let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
    for (chunk_ref, (chunk, status)) in entries {
        let value = canonical_bytes(&chunk_index_value(chunk))?;
        chunks.insert(chunk_ref.as_str(), value.as_slice()).map_err(index_error)?;
        availability.insert(chunk_ref.as_str(), status.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn write_pin_entries(
    write_txn: &redb::WriteTransaction,
    pinned_manifests: &[String],
    pinned_chunks: &[String],
) -> Result<()> {
    let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
    for manifest_ref in pinned_manifests {
        pins.insert(pin_key("manifest", manifest_ref).as_str(), "manifest").map_err(index_error)?;
    }
    for chunk_ref in pinned_chunks {
        pins.insert(pin_key("chunk", chunk_ref).as_str(), "chunk").map_err(index_error)?;
    }
    Ok(())
}

fn rebuild_receipt(inputs: &IndexInputs) -> IoValue {
    let chunk_refs = inputs.chunk_entries.keys().cloned().collect::<Vec<_>>();
    receipt_value(ChunkStoreReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &chunk_refs,
        checks: vec![
            ("redb-index-manifests", "pass"),
            ("redb-index-chunks", "pass"),
            ("redb-index-availability", "pass"),
            ("redb-index-pins", "pass"),
        ],
        details: vec![
            record("manifests", vec![u64_value(inputs.manifest_entries.len() as u64)]),
            record("chunks", vec![u64_value(inputs.chunk_entries.len() as u64)]),
            record("manifest-pins", vec![u64_value(inputs.pinned_manifests.len() as u64)]),
            record("chunk-pins", vec![u64_value(inputs.pinned_chunks.len() as u64)]),
        ],
    })
}

struct ChunkManifestValueInput<'a> {
    object_kind: &'a str,
    total_len: u64,
    chunk_size: u64,
    transforms: &'a ChunkTransforms,
    metadata_ref: &'a str,
    policy_refs: &'a [String],
    chunks: &'a [IoValue],
    root_ref: &'a str,
    evidence_refs: &'a [String],
}

fn manifest_value(input: &ChunkManifestValueInput<'_>) -> IoValue {
    record("chunk-manifest-v1", vec![
        string(CHUNK_MANIFEST_SCHEMA),
        record("object-kind", vec![string(input.object_kind)]),
        record("total-len", vec![u64_value(input.total_len)]),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("chunk-size", vec![u64_value(input.chunk_size)]),
        record("transforms", vec![transforms_value(input.transforms)]),
        record("metadata-ref", vec![string(input.metadata_ref)]),
        record("policy-refs", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("chunks", vec![sequence(input.chunks.to_vec())]),
        record("root-ref", vec![string(input.root_ref)]),
        record("evidence-refs", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
    ])
}

fn chunk_ref_value(chunk_ref: &str, length: u64, chunk_size: usize, transforms: &ChunkTransforms) -> IoValue {
    record("chunk-ref-v1", vec![
        string(CHUNK_REF_SCHEMA),
        record("hash", vec![string(chunk_ref)]),
        record("length", vec![u64_value(length)]),
        record("domain", vec![string(chunk_domain(chunk_size))]),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("transforms", vec![transforms_value(transforms)]),
        record("location-hints", vec![sequence(vec![record("local-content-key", vec![string(
            filename_for_ref(chunk_ref).unwrap_or_else(|_| "unsupported".to_string()),
        )])])]),
        record("evidence-refs", vec![sequence(Vec::new())]),
    ])
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

fn transforms_value(transforms: &ChunkTransforms) -> IoValue {
    record("transforms-v1", vec![
        string(CHUNK_TRANSFORMS_SCHEMA),
        record("compression", vec![string(&transforms.compression)]),
        record("encryption", vec![string(&transforms.encryption)]),
        record("ordering", vec![string(&transforms.ordering)]),
        record("confidentiality", vec![string(&transforms.confidentiality)]),
        record("protected-commitment", vec![
            transforms.protected_commitment_ref.as_ref().map(string).unwrap_or_else(|| record("none", vec![])),
        ]),
    ])
}

fn parse_transforms_field(value: &Value<IoValue>) -> Result<ChunkTransforms> {
    let fields = value
        .collect_simple_record("transforms", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transforms ...> field"))?;
    let transform_record = fields[0]
        .collect_simple_record("transforms-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transforms-v1 ...> value"))?;
    require_schema(&transform_record[0], CHUNK_TRANSFORMS_SCHEMA, "chunk transforms")?;
    let compression = record_string(&transform_record[1], "compression")?;
    let encryption = record_string(&transform_record[2], "encryption")?;
    let ordering = record_string(&transform_record[3], "ordering")?;
    let confidentiality = record_string(&transform_record[4], "confidentiality")?;
    let protected_commitment_ref = record_optional_string(&transform_record[5], "protected-commitment")?;
    Ok(ChunkTransforms {
        compression,
        encryption,
        ordering,
        confidentiality,
        protected_commitment_ref,
    })
}

fn validate_transform_shape(transforms: &ChunkTransforms) -> Result<()> {
    if transforms.compression != "none" && transforms.compression != "zstd-placeholder" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk compression mode {}",
            transforms.compression
        )));
    }
    if transforms.encryption != "none" && transforms.encryption != "protected-commitment" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk encryption mode {}",
            transforms.encryption
        )));
    }
    if transforms.confidentiality != "public" && transforms.confidentiality != "confidential" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk confidentiality mode {}",
            transforms.confidentiality
        )));
    }
    let expected_ordering = match (transforms.compression.as_str(), transforms.encryption.as_str()) {
        ("none", "none") => "identity",
        ("zstd-placeholder", "none") => "compress",
        ("none", "protected-commitment") => "encrypt",
        ("zstd-placeholder", "protected-commitment") => "compress-then-encrypt",
        _ => {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported chunk transform pair compression={} encryption={}",
                transforms.compression, transforms.encryption
            )));
        }
    };
    if transforms.ordering != expected_ordering {
        return Err(MoltenError::invalid_harness(format!(
            "chunk transform ordering {} does not match expected {expected_ordering}",
            transforms.ordering
        )));
    }
    if transforms.confidentiality == "confidential" {
        let Some(commitment_ref) = &transforms.protected_commitment_ref else {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require a protected commitment ref",
            ));
        };
        if commitment_ref.trim().is_empty() {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require a non-empty protected commitment ref",
            ));
        }
        if transforms.encryption != "protected-commitment" {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require protected-commitment encryption",
            ));
        }
    }
    Ok(())
}

fn validate_put_transforms(transforms: &ChunkTransforms) -> Result<()> {
    validate_transform_shape(transforms)?;
    if transforms.confidentiality == "confidential" {
        return Err(MoltenError::invalid_harness(
            "confidential chunk-store writes require a protected encryption implementation before chunk refs may be emitted",
        ));
    }
    if transforms != &ChunkTransforms::public_plaintext() {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk-store transform for writes: compression={} encryption={} ordering={}",
            transforms.compression, transforms.encryption, transforms.ordering
        )));
    }
    Ok(())
}

fn unsupported_transform_message(manifest: &ChunkManifest) -> Option<String> {
    let supported = ChunkTransforms::public_plaintext();
    if manifest.transforms != supported {
        return Some(format!(
            "unsupported chunk-store transform for {}: compression={} encryption={} ordering={} confidentiality={}",
            manifest.manifest_ref,
            manifest.transforms.compression,
            manifest.transforms.encryption,
            manifest.transforms.ordering,
            manifest.transforms.confidentiality
        ));
    }
    for chunk in &manifest.chunks {
        if chunk.transforms != supported {
            return Some(format!(
                "unsupported chunk-store transform for chunk {}: compression={} encryption={} ordering={} confidentiality={}",
                chunk.chunk_ref,
                chunk.transforms.compression,
                chunk.transforms.encryption,
                chunk.transforms.ordering,
                chunk.transforms.confidentiality
            ));
        }
    }
    None
}

fn chunk_root_ref(chunks: &[ChunkRef]) -> Result<String> {
    canonical_hash(&record("chunk-root-v1", vec![
        string(crate::preserves_rail::CHUNK_ROOT_SCHEMA),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("chunks", vec![sequence(
            chunks
                .iter()
                .map(|chunk| record("chunk", vec![string(&chunk.chunk_ref), u64_value(chunk.length)]))
                .collect(),
        )]),
    ]))
}

struct ChunkStoreReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    manifest_ref: Option<&'a str>,
    chunk_refs: &'a [String],
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

fn receipt_value(input: ChunkStoreReceiptValueInput<'_>) -> IoValue {
    record("chunk-store-receipt-v1", vec![
        string(CHUNK_STORE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![input.manifest_ref.map(string).unwrap_or_else(|| record("none", vec![]))]),
        record("chunks", vec![sequence(input.chunk_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .into_iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
        record("details", vec![sequence(input.details)]),
    ])
}

fn denial_receipt_value(
    operation: &str,
    manifest_ref: Option<&str>,
    chunk_refs: &[String],
    reason: impl Into<String>,
    checks: Vec<(&str, &str)>,
) -> IoValue {
    let reason = reason.into();
    receipt_value(ChunkStoreReceiptValueInput {
        operation,
        decision: "deny",
        manifest_ref,
        chunk_refs,
        checks,
        details: vec![record("reason", vec![string(&reason)])],
    })
}

pub fn parse_receipt_value(value: &IoValue, expected_receipt_ref: Option<&str>) -> Result<ChunkStoreReceipt> {
    let fields = simple_record(value, "chunk-store-receipt-v1", 7)?;
    require_schema(&fields[0], CHUNK_STORE_RECEIPT_SCHEMA, "chunk store receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    let decision = record_string(&fields[2], "decision")?;
    let manifest_ref = record_optional_string(&fields[3], "manifest")?;
    let chunk_refs = record_string_sequence(&fields[4], "chunks")?;
    let check_values = record_sequence(&fields[5], "checks")?;
    let details = record_sequence(&fields[6], "details")?;
    if operation.is_empty() {
        return Err(MoltenError::invalid_harness("chunk store receipt operation must not be empty"));
    }
    if decision != "pass" && decision != "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "chunk store receipt decision must be pass or deny, got {decision}"
        )));
    }
    if let Some(manifest_ref) = &manifest_ref {
        filename_for_ref(manifest_ref)?;
    }
    for chunk_ref in &chunk_refs {
        filename_for_ref(chunk_ref)?;
    }
    let mut checks = Vec::new();
    for check_value in &check_values {
        let check = simple_record(check_value, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if name.is_empty() {
            return Err(MoltenError::invalid_harness("chunk store receipt check name must not be empty"));
        }
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "chunk store receipt check status must be pass or fail, got {status}"
            )));
        }
        push_bounded(
            &mut checks,
            ChunkStoreReceiptCheck { name, status },
            MAX_CHUNK_STORE_CHECKS,
            "chunk store receipt checks",
        )?;
    }
    let receipt_ref = canonical_hash(value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "chunk store receipt hash mismatch: got {receipt_ref}, expected {expected}"
        )));
    }
    Ok(ChunkStoreReceipt {
        receipt_ref,
        operation,
        decision,
        manifest_ref,
        chunk_refs,
        checks,
        details,
        value: value.clone(),
    })
}

fn chunk_index_value(chunk: &ChunkRef) -> IoValue {
    record("chunk-index-v1", vec![
        string(CHUNK_INDEX_SCHEMA),
        record("chunk-ref", vec![string(&chunk.chunk_ref)]),
        record("length", vec![u64_value(chunk.length)]),
        record("domain", vec![string(&chunk.domain)]),
        record("chunker", vec![string(&chunk.chunker)]),
        record("transforms", vec![transforms_value(&chunk.transforms)]),
    ])
}

fn partial_fetch_value(manifest_ref: &str, status: &str, missing_before: &[String], fetched: &[String]) -> IoValue {
    record("partial-fetch-v1", vec![
        string(PARTIAL_FETCH_SCHEMA),
        record("manifest", vec![string(manifest_ref)]),
        record("status", vec![string(status)]),
        record("missing-before", vec![sequence(missing_before.iter().map(string).collect())]),
        record("fetched", vec![sequence(fetched.iter().map(string).collect())]),
    ])
}

fn iroh_ticket_value(manifest_ref: &str, manifest_blob_ref: &str, chunks: &[IrohChunkBlob]) -> IoValue {
    record("chunk-store-iroh-ticket-v1", vec![
        string(CHUNK_IROH_TICKET_SCHEMA),
        record("adapter", vec![string("iroh-local")]),
        record("manifest-ref", vec![string(manifest_ref)]),
        record("manifest-blob-ref", vec![string(manifest_blob_ref)]),
        record("chunks", vec![sequence(
            chunks
                .iter()
                .map(|chunk| {
                    record("chunk-blob", vec![
                        string(&chunk.chunk_ref),
                        string(&chunk.blob_ref),
                        u64_value(chunk.length),
                    ])
                })
                .collect(),
        )]),
    ])
}

fn parse_iroh_ticket_value(value: &IoValue) -> Result<IrohChunkTicket> {
    let fields = simple_record(value, "chunk-store-iroh-ticket-v1", 5)?;
    require_schema(&fields[0], CHUNK_IROH_TICKET_SCHEMA, "chunk-store Iroh ticket")?;
    let adapter = record_string(&fields[1], "adapter")?;
    if adapter != "iroh-local" {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk-store Iroh adapter {adapter}")));
    }
    let manifest_ref = record_string(&fields[2], "manifest-ref")?;
    filename_for_ref(&manifest_ref)?;
    let manifest_blob_ref = record_string(&fields[3], "manifest-blob-ref")?;
    filename_for_ref(&manifest_blob_ref)?;
    let chunk_values = record_sequence(&fields[4], "chunks")?;
    let mut chunks = Vec::new();
    let mut seen = OrderedSet::new();
    for chunk_value in &chunk_values {
        let chunk_blob = simple_record(chunk_value, "chunk-blob", 3)?;
        let chunk_ref = required_string(&chunk_blob[0], "chunk ref")?;
        let blob_ref = required_string(&chunk_blob[1], "blob ref")?;
        let length = required_u64(&chunk_blob[2], "chunk blob length")?;
        filename_for_ref(&chunk_ref)?;
        filename_for_ref(&blob_ref)?;
        if length == 0 {
            return Err(MoltenError::invalid_harness(format!(
                "Iroh chunk ticket maps {chunk_ref} to zero-length blob"
            )));
        }
        if !insert_set_bounded(
            &mut seen,
            chunk_ref.clone(),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunk set",
        )? {
            return Err(MoltenError::invalid_harness(format!(
                "Iroh chunk ticket has duplicate chunk mapping for {chunk_ref}"
            )));
        }
        push_bounded(
            &mut chunks,
            IrohChunkBlob {
                chunk_ref,
                blob_ref,
                length,
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunks",
        )?;
    }
    Ok(IrohChunkTicket {
        manifest_ref,
        manifest_blob_ref,
        chunks,
    })
}

fn read_iroh_ticket(root: &Path, manifest_ref: &str) -> Result<IoValue> {
    let bytes = fs::read(iroh_ticket_path(root, manifest_ref)?).map_err(MoltenError::from)?;
    parse_canonical_bytes(&bytes)
}

fn read_iroh_blob(root: &Path, blob_ref: &str) -> Result<Vec<u8>> {
    let bytes = fs::read(iroh_blob_path(root, blob_ref)?).map_err(MoltenError::from)?;
    let actual_ref = hash_blob_bytes(&bytes);
    if actual_ref != blob_ref {
        return Err(MoltenError::invalid_harness(format!("Iroh blob {blob_ref} hashes to {actual_ref}")));
    }
    Ok(bytes)
}

fn index_put(root: &Path, manifest_value: &IoValue, chunks: &[ChunkRef], receipt_value: &IoValue) -> Result<()> {
    let manifest = parse_manifest_value(manifest_value, None)?;
    let chunk_refs = chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, &manifest, &chunk_refs, &[], Some(receipt_value))
}

fn index_manifest_available(root: &Path, manifest: &ChunkManifest, receipt_value: &IoValue) -> Result<()> {
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, manifest, &chunk_refs, &[], Some(receipt_value))
}

fn index_set_manifest_chunk_availability(
    root: &Path,
    manifest: &ChunkManifest,
    available: &[String],
    missing: &[String],
    receipt_value: Option<&IoValue>,
) -> Result<()> {
    let available = available.iter().cloned().collect::<OrderedSet<_>>();
    let missing = missing.iter().cloned().collect::<OrderedSet<_>>();
    let manifest_bytes = canonical_bytes(&manifest.value)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        manifests.insert(manifest.manifest_ref.as_str(), manifest_bytes.as_slice()).map_err(index_error)?;
    }
    {
        let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        for chunk in &manifest.chunks {
            let chunk_value = canonical_bytes(&chunk_index_value(chunk))?;
            chunks.insert(chunk.chunk_ref.as_str(), chunk_value.as_slice()).map_err(index_error)?;
            let status = if missing.contains(&chunk.chunk_ref) {
                "missing"
            } else if available.contains(&chunk.chunk_ref) || chunk_path(root, &chunk.chunk_ref)?.exists() {
                "available"
            } else {
                "missing"
            };
            availability.insert(chunk.chunk_ref.as_str(), status).map_err(index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        store_receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(index_error)
}

fn index_set_partial_fetch(
    root: &Path,
    manifest_ref: &str,
    status: &str,
    missing_before: &[String],
    fetched: &[String],
) -> Result<()> {
    let value = canonical_bytes(&partial_fetch_value(manifest_ref, status, missing_before, fetched))?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut partial_fetches = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        partial_fetches.insert(manifest_ref, value.as_slice()).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)
}

fn index_set_pin(
    root: &Path,
    kind: &str,
    reference: &str,
    pinned: bool,
    receipt_value: Option<&IoValue>,
) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        let key = pin_key(kind, reference);
        if pinned {
            pins.insert(key.as_str(), kind).map_err(index_error)?;
        } else {
            pins.remove(key.as_str()).map_err(index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        store_receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(index_error)
}

struct IndexApplyGcInput<'a> {
    root: &'a Path,
    dry_run: bool,
    removed_manifests: &'a [String],
    removed_chunks: &'a [String],
    receipt_value: &'a IoValue,
    tombstone_receipt: Option<&'a IoValue>,
}

fn index_apply_gc(input: &IndexApplyGcInput<'_>) -> Result<()> {
    let db = ensure_index_tables(input.root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if !input.dry_run {
        {
            let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
            let mut partial_fetches = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
            for manifest_ref in input.removed_manifests {
                manifests.remove(manifest_ref.as_str()).map_err(index_error)?;
                partial_fetches.remove(manifest_ref.as_str()).map_err(index_error)?;
            }
        }
        {
            let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
            let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
            let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
            for chunk_ref in input.removed_chunks {
                chunks.remove(chunk_ref.as_str()).map_err(index_error)?;
                availability.remove(chunk_ref.as_str()).map_err(index_error)?;
                pins.remove(pin_key("chunk", chunk_ref).as_str()).map_err(index_error)?;
            }
        }
    }
    store_receipt_in_tx(&write_txn, input.receipt_value)?;
    if let Some(tombstone_receipt) = input.tombstone_receipt {
        store_receipt_in_tx(&write_txn, tombstone_receipt)?;
    }
    write_txn.commit().map_err(index_error)
}

fn store_receipt(root: &Path, receipt_value: &IoValue) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &IoValue) -> Result<()> {
    let parsed = parse_receipt_value(receipt_value, None)?;
    let receipt_bytes = canonical_bytes(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts.insert(parsed.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(index_error)?;
    Ok(())
}

fn ensure_index_tables(root: &Path) -> Result<Database> {
    fs::create_dir_all(root).map_err(MoltenError::from)?;
    let db = Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn clear_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    {
        let mut table = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    Ok(())
}

fn table_keys(table: &redb::Table<'_, &str, &[u8]>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn pin_key(kind: &str, reference: &str) -> String {
    format!("{kind}:{reference}")
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("chunk store redb index error: {error}"))
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_bytes_bounded(
    bytes: &mut impl crate::bounded::VecSink<u8>,
    incoming: &[u8],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(bytes.item_count(), incoming.len(), maximum, label)?;
    bytes.reserve_items(final_count.saturating_sub(bytes.item_count()));
    bytes.extend_cloned_items(incoming);
    Ok(())
}

fn insert_set_bounded<T: Ord>(values: &mut OrderedSet<T>, value: T, maximum: usize, label: &str) -> Result<bool> {
    if !values.contains(&value) {
        checked_count_sum(values.len(), 1, maximum, label)?;
    }
    Ok(values.insert(value))
}

fn chunk_count_for_len(byte_len: usize, chunk_size: usize, label: &str) -> Result<usize> {
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness(format!("{label} chunk size must be non-zero")));
    }
    let count = byte_len.div_ceil(chunk_size);
    ensure_count_at_most(count, MAX_CHUNK_STORE_CHUNKS, label)?;
    Ok(count)
}

fn chunk_size_to_usize(chunk_size: u64, label: &str) -> Result<usize> {
    usize::try_from(chunk_size)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} is unsupported on this platform: {error}")))
}

fn validate_put_bounds(byte_len: usize, chunk_size: usize, policy_ref_count: usize) -> Result<()> {
    ensure_count_at_most(byte_len, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    chunk_count_for_len(byte_len, chunk_size, "chunk store object chunks")?;
    ensure_count_at_most(policy_ref_count, MAX_CHUNK_STORE_REFS, "chunk store policy refs")?;
    Ok(())
}

fn reconstruct_object(root: &Path, manifest: &ChunkManifest) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut bytes)?;
    Ok(bytes)
}

fn verify_manifest_chunks(root: &Path, manifest: &ChunkManifest) -> Result<()> {
    let mut sink = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut sink)
}

fn verify_manifest_chunks_into(
    root: &Path,
    manifest: &ChunkManifest,
    bytes: &mut impl crate::bounded::VecSink<u8>,
) -> Result<()> {
    let total_len = usize::try_from(manifest.total_len).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk manifest total length is unsupported on this platform: {error}"))
    })?;
    ensure_count_at_most(total_len, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let chunk_bytes = read_verified_chunk(root, chunk, chunk_size)?;
        extend_bytes_bounded(&mut *bytes, &chunk_bytes, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    }
    let actual_len = u64::try_from(bytes.item_count()).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk manifest actual length is unsupported on this platform: {error}"))
    })?;
    if actual_len != manifest.total_len {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest total length mismatch: got {}, expected {}",
            actual_len, manifest.total_len
        )));
    }
    Ok(())
}

fn read_verified_chunk(root: &Path, chunk: &ChunkRef, chunk_size: usize) -> Result<Vec<u8>> {
    let path = chunk_path(root, &chunk.chunk_ref)?;
    let bytes = fs::read(&path).map_err(MoltenError::from)?;
    let actual_ref = hash_chunk(&bytes, chunk_size);
    if actual_ref != chunk.chunk_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk hash mismatch: got {actual_ref}, expected {}",
            chunk.chunk_ref
        )));
    }
    if bytes.len() as u64 != chunk.length {
        return Err(MoltenError::invalid_harness(format!(
            "chunk length mismatch: got {}, expected {}",
            bytes.len(),
            chunk.length
        )));
    }
    Ok(bytes)
}

fn verify_raw_chunk_file(path: &Path, chunk_ref: &str, length: u64, chunk_size: usize) -> Result<()> {
    let bytes = fs::read(path).map_err(MoltenError::from)?;
    verify_raw_chunk_bytes(&bytes, chunk_ref, length, chunk_size).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk store content path for {chunk_ref} is invalid: {error}"))
    })
}

fn verify_raw_chunk_bytes(bytes: &[u8], chunk_ref: &str, length: u64, chunk_size: usize) -> Result<()> {
    let actual_ref = hash_chunk(bytes, chunk_size);
    if actual_ref != chunk_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk bytes hash mismatch: got {actual_ref}, expected {chunk_ref}"
        )));
    }
    if bytes.len() as u64 != length {
        return Err(MoltenError::invalid_harness(format!("chunk bytes length {}, expected {length}", bytes.len())));
    }
    Ok(())
}

fn validate_fixed_chunk_lengths(total_len: u64, chunk_size: u64, chunks: &[ChunkRef]) -> Result<()> {
    let reconstructed = chunks.iter().map(|chunk| chunk.length).sum::<u64>();
    if reconstructed != total_len {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest total length mismatch: refs sum to {reconstructed}, expected {total_len}"
        )));
    }
    if total_len == 0 {
        if !chunks.is_empty() {
            return Err(MoltenError::invalid_harness("empty object manifest must not contain chunks"));
        }
        return Ok(());
    }
    for (index, chunk) in chunks.iter().enumerate() {
        let is_last = index + 1 == chunks.len();
        if !is_last && chunk.length != chunk_size {
            return Err(MoltenError::invalid_harness(format!(
                "non-final fixed_v1 chunk {index} has length {}, expected {chunk_size}",
                chunk.length
            )));
        }
        if is_last && (chunk.length == 0 || chunk.length > chunk_size) {
            return Err(MoltenError::invalid_harness(format!(
                "final fixed_v1 chunk {index} has invalid length {} for chunk size {chunk_size}",
                chunk.length
            )));
        }
    }
    Ok(())
}

fn hash_chunk(bytes: &[u8], chunk_size: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten.chunk-store.chunk.fixed_v1\0");
    hasher.update(chunk_domain(chunk_size).as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    content_ref_from_blake3_hash(hasher.finalize())
}

fn chunk_domain(chunk_size: usize) -> String {
    format!("molten.chunk-store.chunk.fixed_v1:{chunk_size}")
}

fn hash_blob_bytes(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}

fn ensure_iroh_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("blobs")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("tickets")).map_err(MoltenError::from)
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("chunks")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("manifests")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("metadata")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("pins").join("manifests")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("pins").join("chunks")).map_err(MoltenError::from)
}

fn chunk_path(root: &Path, chunk_ref: &str) -> Result<PathBuf> {
    Ok(root.join("chunks").join(filename_for_ref(chunk_ref)?))
}

fn iroh_blob_path(root: &Path, blob_ref: &str) -> Result<PathBuf> {
    Ok(root.join("blobs").join(filename_for_ref(blob_ref)?))
}

fn iroh_ticket_path(root: &Path, manifest_ref: &str) -> Result<PathBuf> {
    Ok(root.join("tickets").join(filename_for_ref(manifest_ref)?))
}

fn manifest_path(root: &Path, manifest_ref: &str) -> Result<PathBuf> {
    Ok(root.join("manifests").join(filename_for_ref(manifest_ref)?))
}

fn metadata_path(root: &Path, metadata_ref: &str) -> Result<PathBuf> {
    Ok(root.join("metadata").join(filename_for_ref(metadata_ref)?))
}

fn manifest_pin_path(root: &Path, manifest_ref: &str) -> Result<PathBuf> {
    Ok(root.join("pins").join("manifests").join(filename_for_ref(manifest_ref)?))
}

fn chunk_pin_path(root: &Path, chunk_ref: &str) -> Result<PathBuf> {
    Ok(root.join("pins").join("chunks").join(filename_for_ref(chunk_ref)?))
}

fn filename_for_ref(reference: &str) -> Result<String> {
    let hex = content_ref_hex(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported chunk-store ref {reference}: {error}")))?;
    Ok(format!("blake3_{hex}.bin"))
}

fn ref_from_filename(filename: &str) -> Option<String> {
    let hex = filename.strip_prefix("blake3_").and_then(|value| value.strip_suffix(".bin"))?;
    content_ref_from_hex(hex).ok()
}

fn refs_from_dir(dir: &Path) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in fs::read_dir(dir).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if !entry.file_type().map_err(MoltenError::from)?.is_file() {
            continue;
        }
        if let Some(reference) = ref_from_filename(&entry.file_name().to_string_lossy()) {
            push_bounded(&mut refs, reference, MAX_CHUNK_STORE_REFS, "chunk store refs from directory")?;
        }
    }
    refs.sort();
    Ok(refs)
}

fn pinned_refs(dir: &Path) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in fs::read_dir(dir).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if entry.file_type().map_err(MoltenError::from)?.is_file() {
            push_bounded(
                &mut refs,
                fs::read_to_string(entry.path()).map_err(MoltenError::from)?,
                MAX_CHUNK_STORE_REFS,
                "chunk store pinned refs",
            )?;
        }
    }
    refs.sort();
    Ok(refs)
}

fn write_immutable_bytes(
    path: &Path,
    bytes: &[u8],
    expected_ref: &str,
    parser: fn(&[u8]) -> Result<IoValue>,
) -> Result<()> {
    if path.exists() {
        let existing = fs::read(path).map_err(MoltenError::from)?;
        let existing_value = parser(&existing)?;
        let existing_ref = canonical_hash(&existing_value)?;
        if existing_ref != expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "immutable content path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        fs::write(path, bytes).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn write_immutable_blob(path: &Path, bytes: &[u8], expected_ref: &str) -> Result<()> {
    if path.exists() {
        let existing = fs::read(path).map_err(MoltenError::from)?;
        let existing_ref = hash_blob_bytes(&existing);
        if existing_ref != expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "immutable blob path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        fs::write(path, bytes).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_record_any<'a>(value: &'a IoValue, label: &str) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, None)
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> record")))
}

fn record_arity(record: &Record<Value<IoValue>>) -> usize {
    record._vec().len().saturating_sub(1)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_u64(&record[0], label)
}

fn record_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(sequence.iter().map(value_to_iovalue).collect())
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_sequence(value, label)?.iter().map(|value| required_string(value, label)).collect()
}

fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    if let Some(value) = record[0].as_string() {
        Ok(Some(value.into_owned()))
    } else if record[0].collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        Err(MoltenError::invalid_harness(format!("expected string or <none> for {label}")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} schema {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn fixed_v1_chunking_has_stable_manifest_identity() {
        let root = temp_dir("chunk-stable");
        let bytes = b"abcdefghij0123456789";
        let first = put_bytes(&root, "artifact", bytes, 4).expect("put first");
        let second = put_bytes(&root, "artifact", bytes, 4).expect("put second");
        assert_eq!(first.manifest_ref, second.manifest_ref);
        assert_eq!(second.dedup_hits, first.chunk_refs.len());
        let different_chunk_size = put_bytes(&root, "artifact", bytes, 5).expect("put different size");
        assert_ne!(first.manifest_ref, different_chunk_size.manifest_ref);
        let different_bytes = put_bytes(&root, "artifact", b"abcdefghij012345678X", 4).expect("put different bytes");
        assert_ne!(first.manifest_ref, different_bytes.manifest_ref);
    }

    #[hegel::test(test_cases = 32)]
    fn hegel_chunk_store_determinism_range_resumable_and_no_dangling(tc: hegel::TestCase) {
        let bytes = tc.draw(hegel::generators::binary().max_size(96));
        let chunk_size = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(16));
        let root = temp_dir("chunk-hegel-root");
        let duplicate_root = temp_dir("chunk-hegel-duplicate");
        let sync_dest = temp_dir("chunk-hegel-sync-dest");

        let first = put_bytes(&root, "artifact", &bytes, chunk_size).expect("put first");
        let duplicate = put_bytes(&duplicate_root, "artifact", &bytes, chunk_size).expect("put duplicate");
        assert_eq!(first.manifest_ref, duplicate.manifest_ref);
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read full").bytes, bytes);

        let offset = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(bytes.len()));
        let max_len = bytes.len().saturating_sub(offset);
        let length = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max_len));
        let range = range_read(&root, &first.manifest_ref, offset as u64, length as u64).expect("range read");
        assert_eq!(range.bytes, bytes[offset..offset + length]);

        let sync = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("sync missing");
        assert_eq!(sync.missing_before.len(), first.chunk_refs.len());
        assert_eq!(read_object(&sync_dest, &first.manifest_ref).expect("read synced").bytes, bytes);
        let repeat = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("repeat sync");
        assert!(repeat.missing_before.is_empty());
        assert!(repeat.fetched_chunks.is_empty());
        assert!(missing_chunks(&sync_dest, &first.manifest_ref).expect("missing after sync").is_empty());

        pin_manifest(&root, &first.manifest_ref).expect("pin manifest");
        let retention_evidence = retention_evidence(&root, "hegel-pinned");
        gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc pinned root");
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read after gc").bytes, bytes);
        for chunk_ref in &first.chunk_refs {
            assert!(chunk_path(&root, chunk_ref).expect("chunk path").exists());
        }
    }

    #[test]
    fn chunks_deduplicate_across_objects_and_verify_ranges() {
        let root = temp_dir("chunk-dedup");
        let first = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put first");
        let second = put_bytes(&root, "snapshot", b"aaaabbbbdddd", 4).expect("put second");
        assert_eq!(second.dedup_hits, 2);
        assert_eq!(list_chunk_refs(&root).expect("list chunks").len(), 4);
        let read = read_object(&root, &first.manifest_ref).expect("read object");
        assert_eq!(read.bytes, b"aaaabbbbcccc");
        let range = range_read(&root, &first.manifest_ref, 2, 8).expect("range read");
        assert_eq!(range.bytes, b"aabbbbcc");
        verify_manifest(&root, &first.manifest_ref).expect("verify first");
        verify_manifest(&root, &second.manifest_ref).expect("verify second");
    }

    #[test]
    fn sync_fetches_only_missing_chunks_and_preserves_manifest_identity() {
        let source = temp_dir("chunk-sync-source");
        let dest = temp_dir("chunk-sync-dest");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let _dest_seed = put_bytes(&dest, "artifact", b"aaaabbbb", 4).expect("seed destination");
        let sync = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync missing chunks");
        assert_eq!(sync.manifest_ref, source_put.manifest_ref);
        assert_eq!(sync.fetched_chunks, vec![source_put.chunk_refs[2].clone()]);
        assert_eq!(read_object(&dest, &source_put.manifest_ref).expect("read synced").bytes, b"aaaabbbbcccc");
        let repeat = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("repeat sync");
        assert!(repeat.fetched_chunks.is_empty());
        assert!(repeat.missing_before.is_empty());
    }

    #[test]
    fn iroh_adapter_publishes_and_fetches_missing_verified_chunks() {
        let source = temp_dir("chunk-iroh-source");
        let dest = temp_dir("chunk-iroh-dest");
        let iroh = temp_dir("chunk-iroh-blobs");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let published = publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
        assert_eq!(published.manifest_ref, source_put.manifest_ref);
        assert_eq!(published.manifest_blob_ref, source_put.manifest_ref);
        assert_eq!(published.chunk_blob_refs.len(), source_put.chunk_refs.len());
        let _dest_seed = put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed destination");
        let fetched = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("fetch");
        assert_eq!(fetched.manifest_ref, source_put.manifest_ref);
        assert_eq!(fetched.missing_before.len(), 2);
        assert_eq!(fetched.fetched_chunks, source_put.chunk_refs[1..].to_vec());
        assert_eq!(read_object(&dest, &source_put.manifest_ref).expect("read fetched").bytes, b"aaaabbbbcccc");
        let repeat = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("repeat fetch");
        assert!(repeat.missing_before.is_empty());
        assert!(repeat.fetched_chunks.is_empty());
        let receipts = list_receipt_refs(&dest)
            .expect("list receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&dest, receipt_ref).expect("read receipt"))
            .collect::<Vec<_>>();
        let has_pass_fetch_receipt = receipts
            .iter()
            .filter(|receipt| receipt.operation == "iroh-fetch")
            .any(|receipt| receipt.decision == "pass");
        assert!(has_pass_fetch_receipt);
        let wrong = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some("blake3:deadbeef"), "peer:test")
            .expect_err("wrong expected manifest fails");
        assert!(wrong.to_string().contains("expected blake3:deadbeef"));
    }

    #[test]
    fn lineage_chains_bind_manifest_publication_fetch_and_scope() {
        let source = temp_dir("chunk-lineage-source");
        let dest = temp_dir("chunk-lineage-dest");
        let iroh = temp_dir("chunk-lineage-iroh");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let published = publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
        let source_lineage = build_chunk_lineage(&source, &source_put.manifest_ref).expect("source lineage");
        assert_eq!(source_lineage.manifest_ref, source_put.manifest_ref);
        assert!(source_lineage.receipt_refs.len() >= 2);
        parse_chunk_lineage_value(&source_lineage.value).expect("parse source lineage");
        let source_text = to_text(&source_lineage.value).expect("render source lineage");
        assert!(source_text.contains("chunk-lineage"));
        assert!(source_text.contains("iroh-publish"));
        assert!(source_text.contains("lineage-no-global-head"));

        let fetched = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("fetch");
        let dest_lineage = build_chunk_lineage(&dest, &fetched.manifest_ref).expect("dest lineage");
        assert_eq!(dest_lineage.manifest_ref, source_put.manifest_ref);
        parse_chunk_lineage_value(&dest_lineage.value).expect("parse dest lineage");
        assert!(to_text(&dest_lineage.value).expect("render dest lineage").contains("iroh-fetch"));

        let manifest = read_manifest(&source, &source_put.manifest_ref).expect("read manifest");
        let wrong_root = canonical_hash(&record("wrong-root", vec![string("lineage")])).expect("wrong root ref");
        let tampered_root =
            parse_text(&source_text.replacen(&manifest.root_ref, &wrong_root, 1)).expect("parse tampered root lineage");
        let error = parse_chunk_lineage_value(&tampered_root).expect_err("tampered root fails");
        assert!(["root", "scope"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

        let tampered_ticket = parse_text(&source_text.replacen("iroh-local-chunk", "iroh-tampered-chunk", 1))
            .expect("parse tampered ticket lineage");
        let error = parse_chunk_lineage_value(&tampered_ticket).expect_err("tampered ticket fails");
        assert!(["payload", "receipt"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

        let other_put = put_bytes(&source, "artifact", b"different", 4).expect("put other");
        let other_lineage = build_chunk_lineage(&source, &other_put.manifest_ref).expect("other lineage");
        assert_ne!(source_lineage.manifest_ref, other_lineage.manifest_ref);
        assert_ne!(source_lineage.link_refs.last(), other_lineage.link_refs.last());
    }

    #[test]
    fn verification_rejects_corrupted_missing_or_tampered_chunks() {
        let root = temp_dir("chunk-corrupt");
        let put = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::write(chunk_path(&root, &manifest.chunks[1].chunk_ref).expect("chunk path"), b"zzzz").expect("corrupt");
        let error = verify_manifest(&root, &put.manifest_ref).expect_err("corruption fails");
        assert!(error.to_string().contains("chunk hash mismatch"));

        fs::remove_dir_all(root.join("chunks")).expect("remove chunks");
        fs::create_dir_all(root.join("chunks")).expect("recreate chunks");
        let put = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put after corruption");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::remove_file(chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path")).expect("remove chunk");
        let missing = missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
        assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
        let error = read_object(&root, &put.manifest_ref).expect_err("missing chunk fails");
        assert!(["No such file", "io error"].iter().any(|needle| error.to_string().contains(needle)));
    }

    #[test]
    fn gc_preserves_pinned_manifest_chunks_and_removes_unpinned_content() {
        let root = temp_dir("chunk-gc");
        let pinned = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put pinned");
        let unpinned = put_bytes(&root, "artifact", b"dddd", 4).expect("put unpinned");
        pin_manifest(&root, &pinned.manifest_ref).expect("pin manifest");
        let retention_evidence = retention_evidence(&root, "gc-remove");
        let apply_refs = gc_apply_refs(
            &root,
            std::slice::from_ref(&unpinned.manifest_ref),
            &unpinned.chunk_refs,
            &retention_evidence,
        );
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc");
        assert!(gc.removed_manifests.contains(&unpinned.manifest_ref));
        assert!(gc.removed_chunks.contains(&unpinned.chunk_refs[0]));
        read_object(&root, &pinned.manifest_ref).expect("pinned object remains readable");
        assert!(read_manifest(&root, &unpinned.manifest_ref).is_err());
    }

    #[test]
    fn chunk_gc_requires_retention_pass_before_removal() {
        let root = temp_dir("chunk-retention-gc");
        let put = put_bytes(&root, "artifact", b"retained", 4).expect("put retained");
        let owner_ref = canonical_hash(&record("chunk-test-ref", vec![string("owner")])).expect("owner ref");
        let policy_refs = vec![canonical_hash(&record("chunk-test-ref", vec![string("policy")])).expect("policy ref")];
        let evidence_refs =
            vec![canonical_hash(&record("chunk-test-ref", vec![string("evidence")])).expect("evidence ref")];
        crate::retention::pin_object(&root, crate::retention::PinInput {
            object_ref: put.manifest_ref.clone(),
            object_kind: "chunk-manifest".to_string(),
            retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT.to_string(),
            source: crate::retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold".to_string(),
            owner_ref,
            expiry_ref: None,
            policy_refs,
            evidence_refs,
            has_authority: true,
        })
        .expect("retention pin");
        let retention_evidence = retention_evidence(&root, "retention-pin");
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_manifests.is_empty());
        assert!(gc.removed_chunks.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        read_object(&root, &put.manifest_ref).expect("retained object remains readable");
    }

    #[test]
    fn chunk_gc_denies_incomplete_reference_index_and_remote_uncertainty() {
        let root = temp_dir("chunk-retention-incomplete-remote");
        let put = put_bytes(&root, "artifact", b"remote", 3).expect("put remote-retained");
        let mut retention_evidence = retention_evidence(&root, "incomplete-remote");
        retention_evidence.remote_refs = vec![chunk_test_ref("remote", "incomplete-remote")];
        retention_evidence.is_reference_index_complete = false;
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc denied");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_manifests.is_empty());
        assert!(gc.removed_chunks.is_empty());
        read_object(&root, &put.manifest_ref).expect("remote-uncertain object remains readable");
    }

    #[test]
    fn redb_index_tracks_rebuild_pins_missing_chunks_and_partial_fetches() {
        let root = temp_dir("chunk-index");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
        let status = index_status(&root).expect("index status after put");
        assert_eq!(status.manifests, 1);
        assert_eq!(status.chunks, 2);
        assert_eq!(status.available_chunks, 2);
        assert_eq!(status.missing_chunks, 0);
        assert_eq!(status.receipts, 1);

        pin_manifest(&root, &put.manifest_ref).expect("pin manifest");
        let status = index_status(&root).expect("index status after pin");
        assert_eq!(status.manifest_pins, 1);
        let rebuild = rebuild_index(&root).expect("rebuild index");
        assert_eq!(rebuild.status.manifests, 1);
        assert_eq!(rebuild.status.chunks, 2);
        assert_eq!(rebuild.status.manifest_pins, 1);
        assert_eq!(rebuild.status.receipts, 3);

        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::remove_file(chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path")).expect("remove chunk");
        let missing = missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
        assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
        let status = index_status(&root).expect("index status after missing scan");
        assert_eq!(status.available_chunks, 1);
        assert_eq!(status.missing_chunks, 1);

        let source = temp_dir("chunk-index-source");
        let dest = temp_dir("chunk-index-dest");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed dest");
        let sync = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync");
        assert_eq!(sync.missing_before.len(), 2);
        assert_eq!(sync.fetched_chunks.len(), 2);
        let status = index_status(&dest).expect("dest index status");
        assert_eq!(status.partial_fetches, 1);
        assert_eq!(status.missing_chunks, 0);
        assert_eq!(status.available_chunks, 3);
    }

    #[test]
    fn receipt_index_covers_pass_denial_dedup_and_tombstone_evidence() {
        let root = temp_dir("chunk-receipts");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
        put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("dedup put");
        verify_manifest(&root, &put.manifest_ref).expect("verify");
        read_object(&root, &put.manifest_ref).expect("fetch");
        range_read(&root, &put.manifest_ref, 1, 5).expect("range");
        pin_manifest(&root, &put.manifest_ref).expect("pin");
        unpin_manifest(&root, &put.manifest_ref).expect("unpin");
        let retention_evidence = retention_evidence(&root, "receipt-index");
        let apply_refs =
            gc_apply_refs(&root, std::slice::from_ref(&put.manifest_ref), &put.chunk_refs, &retention_evidence);
        gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc");

        let before_rebuild = list_receipt_refs(&root).expect("list receipts");
        let receipts = before_rebuild
            .iter()
            .map(|receipt_ref| read_receipt(&root, receipt_ref).expect("read receipt"))
            .collect::<Vec<_>>();
        for receipt in &receipts {
            assert_eq!(canonical_hash(&receipt.value).expect("receipt ref"), receipt.receipt_ref);
            parse_receipt_value(&receipt.value, Some(&receipt.receipt_ref)).expect("validate receipt");
        }
        let operations = receipts.iter().map(|receipt| receipt.operation.as_str()).collect::<OrderedSet<_>>();
        for expected in [
            "manifest-create",
            "dedup-hit",
            "chunk-verify",
            "fetch",
            "range-read",
            "pin",
            "unpin",
            "gc",
            "tombstone",
        ] {
            assert!(operations.contains(expected), "missing receipt operation {expected}");
        }

        rebuild_index(&root).expect("rebuild preserves receipt table");
        let after_rebuild = list_receipt_refs(&root).expect("list receipts after rebuild");
        for receipt_ref in before_rebuild {
            assert!(after_rebuild.contains(&receipt_ref), "receipt {receipt_ref} survived rebuild");
        }

        let denial_root = temp_dir("chunk-denial-receipts");
        let denial_put = put_bytes(&denial_root, "artifact", b"aaaabbbb", 4).expect("put denial fixture");
        let denial_manifest = read_manifest(&denial_root, &denial_put.manifest_ref).expect("read denial manifest");
        fs::write(chunk_path(&denial_root, &denial_manifest.chunks[0].chunk_ref).expect("chunk path"), b"zzzz")
            .expect("corrupt chunk");
        verify_manifest(&denial_root, &denial_put.manifest_ref).expect_err("corrupt verify denied");
        range_read(&denial_root, &denial_put.manifest_ref, 99, 1).expect_err("range denied");
        let missing_chunk_ref =
            canonical_hash(&record("chunk-test-ref", vec![string("missing-pin")])).expect("missing pin ref");
        pin_chunk(&denial_root, &missing_chunk_ref).expect_err("missing chunk pin denied");
        let denials = list_receipt_refs(&denial_root)
            .expect("list denial receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&denial_root, receipt_ref).expect("read denial receipt"))
            .filter(|receipt| receipt.decision == "deny")
            .collect::<Vec<_>>();
        assert!(denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
        assert!(denials.iter().any(|receipt| receipt.operation == "range-read"));
        assert!(denials.iter().any(|receipt| receipt.operation == "pin"));
    }

    #[test]
    fn confidentiality_and_transform_modes_fail_closed_until_supported() {
        assert_confidential_write_denials();
        let (root, transformed_manifest_ref) = write_unsupported_manifest();
        assert_unsupported_transform_denials(&root, &transformed_manifest_ref);
    }

    fn assert_confidential_write_denials() {
        let confidential_root = temp_dir("chunk-confidential-deny");
        let metadata = record("chunk-metadata-v1", vec![record("object-kind", vec![string("artifact")])]);
        let mut confidential_without_commitment = ChunkTransforms::public_plaintext();
        confidential_without_commitment.confidentiality = "confidential".to_string();
        let error = put_bytes_with_transforms(&PutBytesWithTransformsInput {
            root: &confidential_root,
            object_kind: "artifact",
            bytes: b"secret bytes",
            chunk_size: 4,
            metadata: &metadata,
            policy_refs: &[],
            transforms: &confidential_without_commitment,
        })
        .expect_err("confidential write without commitment is denied");
        assert!(error.to_string().contains("protected commitment"));
        let denial_receipts = list_receipt_refs(&confidential_root)
            .expect("list confidential receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&confidential_root, receipt_ref).expect("read receipt"))
            .filter(|receipt| receipt.decision == "deny" && receipt.operation == "manifest-create")
            .collect::<Vec<_>>();
        assert_eq!(denial_receipts.len(), 1);
        let protected_shape = ChunkTransforms::confidential_protected("blake3:protected-commitment-fixture");
        let protected_error = put_bytes_with_transforms(&PutBytesWithTransformsInput {
            root: &confidential_root,
            object_kind: "artifact",
            bytes: b"secret bytes",
            chunk_size: 4,
            metadata: &metadata,
            policy_refs: &[],
            transforms: &protected_shape,
        })
        .expect_err("protected confidential writes are denied until encryption exists");
        assert!(protected_error.to_string().contains("protected encryption implementation"));
    }

    fn write_unsupported_manifest() -> (std::path::PathBuf, String) {
        let root = temp_dir("chunk-transform-unsupported");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put public");
        let public_manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        let unsupported = ChunkTransforms {
            compression: "zstd-placeholder".to_string(),
            encryption: "none".to_string(),
            ordering: "compress".to_string(),
            confidentiality: "public".to_string(),
            protected_commitment_ref: None,
        };
        let transformed_chunks = public_manifest
            .chunks
            .iter()
            .map(|chunk| ChunkRef {
                chunk_ref: chunk.chunk_ref.clone(),
                length: chunk.length,
                domain: chunk.domain.clone(),
                chunker: chunk.chunker.clone(),
                transforms: unsupported.clone(),
            })
            .collect::<Vec<_>>();
        let transformed_chunk_values = transformed_chunks
            .iter()
            .map(|chunk| {
                chunk_ref_value(
                    &chunk.chunk_ref,
                    chunk.length,
                    usize::try_from(public_manifest.chunk_size).expect("test chunk size fits usize"),
                    &unsupported,
                )
            })
            .collect::<Vec<_>>();
        let transformed_root_ref = chunk_root_ref(&transformed_chunks).expect("chunk root");
        let transformed_manifest_value = manifest_value(&ChunkManifestValueInput {
            object_kind: &public_manifest.object_kind,
            total_len: public_manifest.total_len,
            chunk_size: public_manifest.chunk_size,
            transforms: &unsupported,
            metadata_ref: &public_manifest.metadata_ref,
            policy_refs: &public_manifest.policy_refs,
            chunks: &transformed_chunk_values,
            root_ref: &transformed_root_ref,
            evidence_refs: &public_manifest.evidence_refs,
        });
        let transformed_manifest_ref = canonical_hash(&transformed_manifest_value).expect("manifest ref");
        fs::write(
            manifest_path(&root, &transformed_manifest_ref).expect("manifest path"),
            canonical_bytes(&transformed_manifest_value).expect("manifest bytes"),
        )
        .expect("write transformed manifest");
        let parsed = read_manifest(&root, &transformed_manifest_ref).expect("parse transformed manifest");
        assert_eq!(parsed.transforms, unsupported);
        (root, transformed_manifest_ref)
    }

    fn assert_unsupported_transform_denials(root: &std::path::Path, transformed_manifest_ref: &str) {
        assert!(
            verify_manifest(root, transformed_manifest_ref)
                .expect_err("verify rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        assert!(
            read_object(root, transformed_manifest_ref)
                .expect_err("read rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        assert!(
            range_read(root, transformed_manifest_ref, 0, 1)
                .expect_err("range rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        let transform_denials = list_receipt_refs(root)
            .expect("list transform receipts")
            .iter()
            .map(|receipt_ref| read_receipt(root, receipt_ref).expect("read transform receipt"))
            .filter(|receipt| receipt.decision == "deny")
            .collect::<Vec<_>>();
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "fetch"));
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "range-read"));
    }

    #[test]
    fn manifest_text_roundtrip_keeps_identity() {
        let root = temp_dir("chunk-roundtrip");
        let put = put_bytes(&root, "artifact", b"abcdef", 3).expect("put");
        let rendered = to_text(&put.manifest_value).expect("render manifest");
        let reparsed = crate::preserves_rail::parse_text(&rendered).expect("parse manifest");
        let parsed = parse_manifest_value(&reparsed, Some(&put.manifest_ref)).expect("parse manifest value");
        assert_eq!(parsed.chunks.len(), 2);
    }

    fn gc_apply_refs(
        root: &std::path::Path,
        manifest_refs: &[String],
        chunk_refs: &[String],
        evidence: &crate::retention::DestructiveRetentionEvidence,
    ) -> Vec<String> {
        let mut apply_refs = Vec::with_capacity(manifest_refs.len() + chunk_refs.len());
        for manifest_ref in manifest_refs {
            let plan = crate::retention::store_retention_gc_plan(crate::retention::RetentionGcPlanInput {
                root,
                subsystem: "chunk-gc",
                object_ref: manifest_ref,
                object_kind: "chunk-manifest",
                retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT,
                action: crate::retention::ACTION_DELETE,
                evidence,
            })
            .expect("store manifest GC plan");
            apply_refs.push(
                crate::retention::apply_retention_gc_plan(crate::retention::RetentionGcApplyFromPlanInput {
                    root,
                    plan_ref: &plan.plan_ref,
                })
                .expect("apply manifest GC plan")
                .apply_ref,
            );
        }
        for chunk_ref in chunk_refs {
            let plan = crate::retention::store_retention_gc_plan(crate::retention::RetentionGcPlanInput {
                root,
                subsystem: "chunk-gc",
                object_ref: chunk_ref,
                object_kind: "chunk",
                retention_class: crate::retention::CLASS_DURABLE_VALUE,
                action: crate::retention::ACTION_DELETE,
                evidence,
            })
            .expect("store chunk GC plan");
            apply_refs.push(
                crate::retention::apply_retention_gc_plan(crate::retention::RetentionGcApplyFromPlanInput {
                    root,
                    plan_ref: &plan.plan_ref,
                })
                .expect("apply chunk GC plan")
                .apply_ref,
            );
        }
        apply_refs
    }

    fn retention_evidence(root: &std::path::Path, label: &str) -> crate::retention::DestructiveRetentionEvidence {
        let requester_ref = chunk_test_ref("requester", label);
        let mut policy_refs = Vec::new();
        let mut authority_refs = Vec::new();
        let mut evidence_refs = Vec::new();
        let mut reference_index_refs = Vec::new();
        for manifest_ref in list_manifest_refs(root).expect("list manifests for retention evidence") {
            push_admissions(
                root,
                label,
                &requester_ref,
                &manifest_ref,
                "chunk-manifest",
                crate::retention::CLASS_PUBLIC_ARTIFACT,
                &mut policy_refs,
                &mut authority_refs,
                &mut evidence_refs,
                &mut reference_index_refs,
            );
        }
        for chunk_ref in list_chunk_refs(root).expect("list chunks for retention evidence") {
            push_admissions(
                root,
                label,
                &requester_ref,
                &chunk_ref,
                "chunk",
                crate::retention::CLASS_DURABLE_VALUE,
                &mut policy_refs,
                &mut authority_refs,
                &mut evidence_refs,
                &mut reference_index_refs,
            );
        }
        crate::retention::DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref),
            policy_refs,
            authority_refs,
            evidence_refs,
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs,
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn push_admissions(
        root: &std::path::Path,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        policy_refs: &mut Vec<String>,
        authority_refs: &mut Vec<String>,
        evidence_refs: &mut Vec<String>,
        reference_index_refs: &mut Vec<String>,
    ) {
        policy_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_POLICY,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        authority_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_AUTHORITY,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        evidence_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        reference_index_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
    }

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        remote_refs: &[String],
        is_reference_index_complete: bool,
    ) -> String {
        crate::retention::store_retention_evidence_admission(root, &crate::retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            bound_refs: &[chunk_test_ref(kind, label)],
            retained_refs: &[],
            remote_refs,
            is_reference_index_complete,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retention admission")
        .admission_ref
    }

    fn chunk_test_ref(kind: &str, label: &str) -> String {
        canonical_hash(&record("chunk-test-ref", vec![string(kind), string(label)])).expect("chunk test ref")
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
