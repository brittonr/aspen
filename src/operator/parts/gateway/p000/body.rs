type ChunkManifest = crate::chunk_store::ChunkManifest;
type ChunkTransforms = crate::chunk_store::ChunkTransforms;
type IoValue = preserves::IOValue;
type Map<K, V> = std::collections::BTreeMap<K, V>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Set<T> = std::collections::BTreeSet<T>;

fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    crate::preserves_rail::content_ref_from_blake3_hash(hash)
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

pub const READ_SCHEMA: &str = "molten.operator.gateway-read-receipt.v1";
pub const RANGE_SCHEMA: &str = "molten.operator.gateway-range-receipt.v1";
pub const INDEX_SCHEMA: &str = "molten.operator.gateway-index-receipt.v1";

const MAX_REFS: usize = 256;
const MAX_DIAGNOSTICS: usize = 64;
const MAX_MEMBERS: usize = 512;
const MAX_MEMBER_NAME_BYTES: usize = 256;
const MAX_MIME_BYTES: usize = 128;
const MIN_CHUNK_SIZE: usize = 1;
const MAX_CHUNK_BYTES: usize = 1_073_741_824;
const RANGE_START: u64 = 0;
const EMPTY_RANGE_LENGTH: u64 = 0;
const EVIDENCE_ONLY_CAVEAT: &str = "gateway receipts are readback evidence only and do not grant authority, policy admission, provenance trust, source-gate acceptance, retention clearance, execution permission, or mutation rights";
const PUBLIC_PROFILE: &str = "public";
const DIAGNOSTIC_PROFILE: &str = "diagnostic";
const INTERNAL_PROFILE: &str = "internal";

const _: () = assert!(MAX_REFS > 0);
const _: () = assert!(MAX_DIAGNOSTICS > 0);
const _: () = assert!(MAX_MEMBERS > 0);
const _: () = assert!(MAX_MEMBER_NAME_BYTES > 0);
const _: () = assert!(MAX_MIME_BYTES > 0);
const _: () = assert!(MIN_CHUNK_SIZE > 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visibility {
    pub profile: String,
    pub visibility_policy_refs: Vec<String>,
    pub retention_refs: Vec<String>,
    pub reveal_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub hidden_refs: Vec<String>,
    pub allow_sensitive_names: bool,
}

#[derive(Debug, Clone)]
pub struct ReadInput<'a> {
    pub object_ref: String,
    pub member: Option<String>,
    pub requested_range: Option<Range>,
    pub requester_ref: String,
    pub manifest: Option<&'a ChunkManifest>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadDecision {
    pub decision: String,
    pub object_ref: String,
    pub member: Option<String>,
    pub normalized_range: Option<Range>,
    pub required_chunk_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone)]
pub struct RangeVerificationInput<'a> {
    pub read: ReadInput<'a>,
    pub chunk_bytes: Map<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeVerification {
    pub decision: String,
    pub manifest_ref: String,
    pub normalized_range: Range,
    pub chunk_refs: Vec<String>,
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub object_ref: String,
    pub size: u64,
    pub mime_hint: Option<String>,
    pub sensitive: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexInput {
    pub bundle_ref: String,
    pub requester_ref: String,
    pub visibility: Visibility,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub name: String,
    pub object_ref: Option<String>,
    pub size: Option<u64>,
    pub mime_hint: Option<String>,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexDecision {
    pub decision: String,
    pub bundle_ref: String,
    pub entries: Vec<IndexEntry>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

trait DiagnosticSink {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()>;
}

impl DiagnosticSink for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()> {
        let next = self
            .len()
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("gateway diagnostic count overflow"))?;
        validate_count(next, MAX_DIAGNOSTICS, "gateway diagnostic")?;
        self.push(diagnostic);
        Ok(())
    }
}

pub fn decide_readback(input: &ReadInput<'_>) -> Result<ReadDecision> {
    let mut diagnostics = Vec::new();
    collect_ref_diagnostics(std::slice::from_ref(&input.object_ref), "object", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.requester_ref), "requester", &mut diagnostics)?;
    collect_visibility_diagnostics(&input.visibility, &mut diagnostics)?;
    if input.visibility.hidden_refs.iter().any(|reference| reference == &input.object_ref) {
        push_diagnostic(&mut diagnostics, "requested object is hidden by gateway visibility policy")?;
    }
    let is_protected = input
        .manifest
        .map(|manifest| {
            manifest.transforms.confidentiality != "public" || manifest.transforms.protected_commitment_ref.is_some()
        })
        .unwrap_or(false);
    if is_protected && input.visibility.reveal_refs.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "protected object denied before rendering names, refs, MIME hints, sizes, or bytes",
        )?;
    }
    let normalized_range = normalize_range(input.manifest, input.requested_range, &mut diagnostics)?;
    let required_chunk_refs = input
        .manifest
        .zip(normalized_range)
        .map(|(manifest, range)| required_chunks_for_range(manifest, range, &mut diagnostics))
        .transpose()?
        .unwrap_or_default();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let receipt_value = read_receipt_value(ReadReceiptInput {
        decision: &decision,
        object_ref: &input.object_ref,
        member: input.member.as_deref(),
        requester_ref: &input.requester_ref,
        normalized_range,
        required_chunk_refs: &required_chunk_refs,
        visibility: &input.visibility,
        diagnostics: &diagnostics,
    })?;
    Ok(ReadDecision {
        decision,
        object_ref: input.object_ref.clone(),
        member: input.member.clone(),
        normalized_range,
        required_chunk_refs,
        diagnostics,
        receipt_value,
    })
}

pub fn verify_range(input: &RangeVerificationInput<'_>) -> Result<RangeVerification> {
    let read = decide_readback(&input.read)?;
    let manifest = input
        .read
        .manifest
        .ok_or_else(|| MoltenError::invalid_harness("gateway range verification requires a chunk manifest"))?;
    let range = read.normalized_range.unwrap_or(Range {
        offset: RANGE_START,
        length: EMPTY_RANGE_LENGTH,
    });
    let mut diagnostics = read.diagnostics.clone();
    if read.decision != "pass" {
        let receipt_value = range_receipt_value(RangeReceiptInput {
            decision: "deny",
            manifest_ref: &manifest.manifest_ref,
            normalized_range: range,
            chunk_refs: &read.required_chunk_refs,
            diagnostics: &diagnostics,
        })?;
        return Ok(RangeVerification {
            decision: "deny".to_string(),
            manifest_ref: manifest.manifest_ref.clone(),
            normalized_range: range,
            chunk_refs: read.required_chunk_refs,
            bytes: Vec::new(),
            diagnostics,
            receipt_value,
        });
    }
    if manifest.transforms != ChunkTransforms::public_plaintext() {
        push_diagnostic(&mut diagnostics, "unsupported transform denies before response bytes")?;
    }
    let chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway chunk size unsupported: {error}")))?;
    if chunk_size < MIN_CHUNK_SIZE {
        push_diagnostic(&mut diagnostics, "manifest chunk size must be non-zero")?;
    }
    let verified = if diagnostics.is_empty() {
        reconstruct_verified_range(manifest, range, &input.chunk_bytes, chunk_size, &mut diagnostics)?
    } else {
        Vec::new()
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let bytes = if decision == "pass" { verified } else { Vec::new() };
    let receipt_value = range_receipt_value(RangeReceiptInput {
        decision: &decision,
        manifest_ref: &manifest.manifest_ref,
        normalized_range: range,
        chunk_refs: &read.required_chunk_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(RangeVerification {
        decision,
        manifest_ref: manifest.manifest_ref.clone(),
        normalized_range: range,
        chunk_refs: read.required_chunk_refs,
        bytes,
        diagnostics,
        receipt_value,
    })
}
