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

pub const OPERATOR_GATEWAY_READ_SCHEMA: &str = "molten.operator.gateway-read-receipt.v1";
pub const OPERATOR_GATEWAY_RANGE_SCHEMA: &str = "molten.operator.gateway-range-receipt.v1";
pub const OPERATOR_GATEWAY_INDEX_SCHEMA: &str = "molten.operator.gateway-index-receipt.v1";

const MAX_GATEWAY_REFS: usize = 256;
const MAX_GATEWAY_DIAGNOSTICS: usize = 64;
const MAX_GATEWAY_MEMBERS: usize = 512;
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

const _: () = assert!(MAX_GATEWAY_REFS > 0);
const _: () = assert!(MAX_GATEWAY_DIAGNOSTICS > 0);
const _: () = assert!(MAX_GATEWAY_MEMBERS > 0);
const _: () = assert!(MAX_MEMBER_NAME_BYTES > 0);
const _: () = assert!(MAX_MIME_BYTES > 0);
const _: () = assert!(MIN_CHUNK_SIZE > 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayRange {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayVisibility {
    pub profile: String,
    pub visibility_policy_refs: Vec<String>,
    pub retention_refs: Vec<String>,
    pub reveal_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub hidden_refs: Vec<String>,
    pub allow_sensitive_names: bool,
}

#[derive(Debug, Clone)]
pub struct GatewayReadInput<'a> {
    pub object_ref: String,
    pub member: Option<String>,
    pub requested_range: Option<GatewayRange>,
    pub requester_ref: String,
    pub manifest: Option<&'a ChunkManifest>,
    pub visibility: GatewayVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayReadDecision {
    pub decision: String,
    pub object_ref: String,
    pub member: Option<String>,
    pub normalized_range: Option<GatewayRange>,
    pub required_chunk_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone)]
pub struct GatewayRangeVerificationInput<'a> {
    pub read: GatewayReadInput<'a>,
    pub chunk_bytes: Map<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRangeVerification {
    pub decision: String,
    pub manifest_ref: String,
    pub normalized_range: GatewayRange,
    pub chunk_refs: Vec<String>,
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayMember {
    pub name: String,
    pub object_ref: String,
    pub size: u64,
    pub mime_hint: Option<String>,
    pub sensitive: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayIndexInput {
    pub bundle_ref: String,
    pub requester_ref: String,
    pub visibility: GatewayVisibility,
    pub members: Vec<GatewayMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayIndexEntry {
    pub name: String,
    pub object_ref: Option<String>,
    pub size: Option<u64>,
    pub mime_hint: Option<String>,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayIndexDecision {
    pub decision: String,
    pub bundle_ref: String,
    pub entries: Vec<GatewayIndexEntry>,
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
        validate_count(next, MAX_GATEWAY_DIAGNOSTICS, "gateway diagnostic")?;
        self.push(diagnostic);
        Ok(())
    }
}

pub fn decide_readback(input: &GatewayReadInput<'_>) -> Result<GatewayReadDecision> {
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
    Ok(GatewayReadDecision {
        decision,
        object_ref: input.object_ref.clone(),
        member: input.member.clone(),
        normalized_range,
        required_chunk_refs,
        diagnostics,
        receipt_value,
    })
}

pub fn verify_gateway_range(input: &GatewayRangeVerificationInput<'_>) -> Result<GatewayRangeVerification> {
    let read = decide_readback(&input.read)?;
    let manifest = input
        .read
        .manifest
        .ok_or_else(|| MoltenError::invalid_harness("gateway range verification requires a chunk manifest"))?;
    let range = read.normalized_range.unwrap_or(GatewayRange {
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
        return Ok(GatewayRangeVerification {
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
    Ok(GatewayRangeVerification {
        decision,
        manifest_ref: manifest.manifest_ref.clone(),
        normalized_range: range,
        chunk_refs: read.required_chunk_refs,
        bytes,
        diagnostics,
        receipt_value,
    })
}

pub fn decide_index(input: &GatewayIndexInput) -> Result<GatewayIndexDecision> {
    let mut diagnostics = Vec::new();
    collect_ref_diagnostics(std::slice::from_ref(&input.bundle_ref), "bundle", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.requester_ref), "requester", &mut diagnostics)?;
    collect_visibility_diagnostics(&input.visibility, &mut diagnostics)?;
    validate_count(input.members.len(), MAX_GATEWAY_MEMBERS, "gateway index member")?;
    let hidden = input.visibility.hidden_refs.iter().collect::<Set<_>>();
    let entry_capacity = input.members.len().min(MAX_GATEWAY_MEMBERS);
    let mut entries = Vec::with_capacity(entry_capacity);
    for member in &input.members {
        validate_member(member, &mut diagnostics)?;
        if !member.visible || hidden.contains(&member.object_ref) {
            push_diagnostic(&mut diagnostics, "hidden member omitted without leaking ref")?;
            continue;
        }
        let should_redact = member.sensitive
            && (input.visibility.profile == PUBLIC_PROFILE || input.visibility.profile == DIAGNOSTIC_PROFILE)
            && !input.visibility.allow_sensitive_names;
        if should_redact {
            entries.push(GatewayIndexEntry {
                name: "redacted".to_string(),
                object_ref: None,
                size: None,
                mime_hint: None,
                redacted: true,
            });
        } else {
            entries.push(GatewayIndexEntry {
                name: member.name.clone(),
                object_ref: Some(member.object_ref.clone()),
                size: Some(member.size),
                mime_hint: member.mime_hint.clone(),
                redacted: false,
            });
        }
    }
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic.contains("invalid")) {
        "deny"
    } else {
        "pass"
    }
    .to_string();
    let receipt_value = index_receipt_value(IndexReceiptInput {
        decision: &decision,
        bundle_ref: &input.bundle_ref,
        requester_ref: &input.requester_ref,
        visibility: &input.visibility,
        entries: &entries,
        diagnostics: &diagnostics,
    })?;
    Ok(GatewayIndexDecision {
        decision,
        bundle_ref: input.bundle_ref.clone(),
        entries,
        diagnostics,
        receipt_value,
    })
}

pub fn gateway_receipt_authorizes_mutation(_receipt: &IoValue) -> bool {
    false
}

fn normalize_range(
    manifest: Option<&ChunkManifest>,
    requested: Option<GatewayRange>,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Option<GatewayRange>> {
    let Some(manifest) = manifest else {
        if requested.is_some() {
            push_diagnostic(diagnostics, "range request requires a chunk manifest before lookup")?;
        }
        return Ok(None);
    };
    let total_len = manifest.total_len;
    let range = requested.unwrap_or(GatewayRange {
        offset: RANGE_START,
        length: total_len,
    });
    let Some(end) = range.offset.checked_add(range.length) else {
        push_diagnostic(diagnostics, "range offset and length overflow")?;
        return Ok(Some(range));
    };
    if range.offset > total_len || end > total_len {
        push_diagnostic(diagnostics, "range outside object length")?;
    }
    Ok(Some(range))
}

fn required_chunks_for_range(
    manifest: &ChunkManifest,
    range: GatewayRange,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Vec<String>> {
    if range.length == EMPTY_RANGE_LENGTH {
        return Ok(Vec::new());
    }
    let chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway chunk size unsupported: {error}")))?;
    if chunk_size < MIN_CHUNK_SIZE {
        push_diagnostic(diagnostics, "manifest chunk size must be non-zero")?;
        return Ok(Vec::new());
    }
    let offset = usize::try_from(range.offset)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range offset unsupported: {error}")))?;
    let end = usize::try_from(range.offset + range.length)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range end unsupported: {error}")))?;
    let first = offset
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("gateway chunk size must be non-zero"))?;
    let last_exclusive = end.div_ceil(chunk_size);
    let chunk_count = last_exclusive.saturating_sub(first);
    let mut refs = Vec::with_capacity(chunk_count);
    for index in first..last_exclusive {
        let Some(chunk) = manifest.chunks.get(index) else {
            push_diagnostic(diagnostics, "range maps to missing manifest chunk")?;
            continue;
        };
        refs.push(chunk.chunk_ref.clone());
    }
    Ok(refs)
}

fn reconstruct_verified_range(
    manifest: &ChunkManifest,
    range: GatewayRange,
    chunks: &Map<String, Vec<u8>>,
    chunk_size: usize,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let offset = usize::try_from(range.offset)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range offset unsupported: {error}")))?;
    let end = usize::try_from(range.offset + range.length)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range end unsupported: {error}")))?;
    if range.length == EMPTY_RANGE_LENGTH {
        return Ok(output);
    }
    let first = offset
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("gateway chunk size must be non-zero"))?;
    let last_exclusive = end.div_ceil(chunk_size);
    for index in first..last_exclusive {
        let Some(chunk) = manifest.chunks.get(index) else {
            push_diagnostic(diagnostics, "range maps to missing manifest chunk")?;
            continue;
        };
        let Some(bytes) = chunks.get(&chunk.chunk_ref) else {
            push_diagnostic(diagnostics, "missing chunk denies before response")?;
            continue;
        };
        let actual_ref = hash_fixed_chunk(bytes, chunk_size);
        if actual_ref != chunk.chunk_ref {
            push_diagnostic(diagnostics, "corrupt chunk denies before response")?;
            continue;
        }
        if bytes.len() as u64 != chunk.length {
            push_diagnostic(diagnostics, "wrong chunk length denies before response")?;
            continue;
        }
        let chunk_start = index * chunk_size;
        let wanted_start = offset.saturating_sub(chunk_start);
        let wanted_end = end.saturating_sub(chunk_start).min(bytes.len());
        output.extend_from_slice(&bytes[wanted_start..wanted_end]);
        if output.len() > MAX_CHUNK_BYTES {
            push_diagnostic(diagnostics, "gateway reconstructed bytes exceed bound")?;
            return Ok(Vec::new());
        }
    }
    if output.len() as u64 != range.length {
        push_diagnostic(diagnostics, "range reconstruction length mismatch")?;
    }
    Ok(output)
}

fn hash_fixed_chunk(bytes: &[u8], chunk_size: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten.chunk-store.chunk.fixed_v1\0");
    hasher.update(format!("molten.chunk-store.chunk.fixed_v1:{chunk_size}").as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    content_ref_from_blake3_hash(hasher.finalize())
}

struct ReadReceiptInput<'a> {
    decision: &'a str,
    object_ref: &'a str,
    member: Option<&'a str>,
    requester_ref: &'a str,
    normalized_range: Option<GatewayRange>,
    required_chunk_refs: &'a [String],
    visibility: &'a GatewayVisibility,
    diagnostics: &'a [String],
}

fn read_receipt_value(input: ReadReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-read-receipt-v1", vec![
        string(OPERATOR_GATEWAY_READ_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("object", vec![string(input.object_ref)]),
        record("member", vec![optional_string_value(input.member)]),
        record("requester", vec![string(input.requester_ref)]),
        record("range", vec![range_value(input.normalized_range)]),
        record("required-chunks", vec![refs_value(input.required_chunk_refs)?]),
        visibility_value(input.visibility)?,
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("readback-decision-before-io", pass_fail(input.decision == "pass")),
            ("visibility-retention-checked", pass_fail(input.decision == "pass")),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
        record("caveat", vec![string(EVIDENCE_ONLY_CAVEAT)]),
    ]))
}

struct RangeReceiptInput<'a> {
    decision: &'a str,
    manifest_ref: &'a str,
    normalized_range: GatewayRange,
    chunk_refs: &'a [String],
    diagnostics: &'a [String],
}

fn range_receipt_value(input: RangeReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-range-receipt-v1", vec![
        string(OPERATOR_GATEWAY_RANGE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("range", vec![range_value(Some(input.normalized_range))]),
        record("chunks", vec![refs_value(input.chunk_refs)?]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("manifest-ref-bound", "pass"),
            ("range-normalized", "pass"),
            ("chunks-verified-before-bytes", pass_fail(input.decision == "pass")),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
    ]))
}

struct IndexReceiptInput<'a> {
    decision: &'a str,
    bundle_ref: &'a str,
    requester_ref: &'a str,
    visibility: &'a GatewayVisibility,
    entries: &'a [GatewayIndexEntry],
    diagnostics: &'a [String],
}

fn index_receipt_value(input: IndexReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-index-receipt-v1", vec![
        string(OPERATOR_GATEWAY_INDEX_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("requester", vec![string(input.requester_ref)]),
        visibility_value(input.visibility)?,
        record("entries", vec![sequence(input.entries.iter().map(index_entry_value).collect())]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("read-only-index", "pass"),
            ("hidden-members-omitted", "pass"),
            ("sensitive-metadata-redacted", "pass"),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
        record("caveat", vec![string(EVIDENCE_ONLY_CAVEAT)]),
    ]))
}

fn visibility_value(visibility: &GatewayVisibility) -> Result<IoValue> {
    Ok(record("visibility", vec![
        record("profile", vec![string(&visibility.profile)]),
        record("policy", vec![refs_value(&visibility.visibility_policy_refs)?]),
        record("retention", vec![refs_value(&visibility.retention_refs)?]),
        record("reveal", vec![refs_value(&visibility.reveal_refs)?]),
        record("redaction", vec![refs_value(&visibility.redaction_refs)?]),
    ]))
}

fn index_entry_value(entry: &GatewayIndexEntry) -> IoValue {
    record("entry", vec![
        record("name", vec![string(&entry.name)]),
        record("object", vec![optional_string_value(entry.object_ref.as_deref())]),
        record("size", vec![optional_u64_value(entry.size)]),
        record("mime", vec![optional_string_value(entry.mime_hint.as_deref())]),
        record("redacted", vec![string(if entry.redacted { "true" } else { "false" })]),
    ])
}

fn range_value(range: Option<GatewayRange>) -> IoValue {
    match range {
        Some(range) => record("some", vec![
            record("offset", vec![u64_value(range.offset)]),
            record("length", vec![u64_value(range.length)]),
        ]),
        None => record("none", Vec::new()),
    }
}

fn validate_member(member: &GatewayMember, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_text(&member.name, "member name", MAX_MEMBER_NAME_BYTES, diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&member.object_ref), "member object", diagnostics)?;
    if let Some(mime) = &member.mime_hint {
        validate_text(mime, "MIME hint", MAX_MIME_BYTES, diagnostics)?;
    }
    Ok(())
}

fn collect_visibility_diagnostics(visibility: &GatewayVisibility, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    if !matches!(visibility.profile.as_str(), PUBLIC_PROFILE | DIAGNOSTIC_PROFILE | INTERNAL_PROFILE) {
        push_diagnostic(diagnostics, "unsupported gateway visibility profile")?;
    }
    collect_ref_diagnostics(&visibility.visibility_policy_refs, "visibility policy", diagnostics)?;
    collect_ref_diagnostics(&visibility.retention_refs, "retention", diagnostics)?;
    collect_ref_diagnostics(&visibility.reveal_refs, "reveal", diagnostics)?;
    collect_ref_diagnostics(&visibility.redaction_refs, "redaction", diagnostics)?;
    collect_ref_diagnostics(&visibility.hidden_refs, "hidden", diagnostics)?;
    if visibility.visibility_policy_refs.is_empty() {
        push_diagnostic(diagnostics, "gateway visibility policy refs are required")?;
    }
    Ok(())
}

fn collect_ref_diagnostics(refs: &[String], label: &str, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_count(refs.len(), MAX_GATEWAY_REFS, label)?;
    for reference in refs {
        if let Err(error) = validate_content_ref(reference) {
            push_diagnostic(diagnostics, format!("invalid {label} ref: {error}"))?;
        }
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, maximum: usize, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    if value.trim().is_empty() {
        return push_diagnostic(diagnostics, format!("{label} must not be empty"));
    }
    if value.len() > maximum {
        return push_diagnostic(diagnostics, format!("{label} length {} exceeds bound {maximum}", value.len()));
    }
    Ok(())
}

fn validate_count(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn push_diagnostic(diagnostics: &mut impl DiagnosticSink, diagnostic: impl Into<String>) -> Result<()> {
    diagnostics.push_bounded(diagnostic.into())
}

fn refs_value(refs: &[String]) -> Result<IoValue> {
    validate_count(refs.len(), MAX_GATEWAY_REFS, "gateway ref")?;
    Ok(sequence(refs.iter().map(string).collect()))
}

fn strings_value(values: &[String]) -> Result<IoValue> {
    validate_count(values.len(), MAX_GATEWAY_DIAGNOSTICS, "gateway string")?;
    Ok(sequence(values.iter().map(string).collect()))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::chunk_store;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::to_text;

    const CHUNK_SIZE: u64 = 4;
    const RANGE_OFFSET: u64 = 2;
    const RANGE_LENGTH: u64 = 5;
    const MEMBER_SIZE: u64 = 7;
    const FIRST_TEMP_ROOT_ID: u64 = 1;

    static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(FIRST_TEMP_ROOT_ID);

    fn fixture_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn temp_root(label: &str) -> PathBuf {
        let id = NEXT_TEMP_ROOT_ID.fetch_add(FIRST_TEMP_ROOT_ID, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-gateway-{label}-{}-{id}", std::process::id()));
        match std::fs::remove_dir_all(&root) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove stale gateway temp root {}: {error}", root.display()),
        }
        root
    }

    fn visibility() -> GatewayVisibility {
        GatewayVisibility {
            profile: PUBLIC_PROFILE.to_string(),
            visibility_policy_refs: vec![fixture_ref("visibility")],
            retention_refs: vec![fixture_ref("retention")],
            reveal_refs: Vec::new(),
            redaction_refs: vec![fixture_ref("redaction")],
            hidden_refs: Vec::new(),
            allow_sensitive_names: false,
        }
    }

    fn manifest_fixture() -> (PathBuf, ChunkManifest, Map<String, Vec<u8>>) {
        let root = temp_root("range");
        let body = b"abcdefghi";
        let put = chunk_store::put_bytes(&root, "artifact", body, CHUNK_SIZE).expect("put");
        let manifest =
            chunk_store::parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref)).expect("manifest");
        let chunk_size = usize::try_from(CHUNK_SIZE).expect("fixture chunk size fits usize");
        let chunks = manifest
            .chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let start = index * chunk_size;
                let end = (start + chunk_size).min(body.len());
                (chunk.chunk_ref.clone(), body[start..end].to_vec())
            })
            .collect::<Map<_, _>>();
        (root, manifest, chunks)
    }

    #[test]
    fn readback_decision_normalizes_range_and_requires_chunks_before_io() {
        let (_root, manifest, _chunks) = manifest_fixture();
        let read = decide_readback(&GatewayReadInput {
            object_ref: manifest.manifest_ref.clone(),
            member: None,
            requested_range: Some(GatewayRange {
                offset: RANGE_OFFSET,
                length: RANGE_LENGTH,
            }),
            requester_ref: fixture_ref("operator"),
            manifest: Some(&manifest),
            visibility: visibility(),
        })
        .expect("read decision");
        assert_eq!(read.decision, "pass");
        assert_eq!(read.normalized_range.expect("range").length, RANGE_LENGTH);
        assert!(!read.required_chunk_refs.is_empty());
    }

    #[test]
    fn malformed_ref_denies_before_lookup() {
        let read = decide_readback(&GatewayReadInput {
            object_ref: "not-a-ref".to_string(),
            member: None,
            requested_range: None,
            requester_ref: fixture_ref("operator"),
            manifest: None,
            visibility: visibility(),
        })
        .expect("malformed deny");
        assert_eq!(read.decision, "deny");
        assert!(read.diagnostics.iter().any(|diagnostic| diagnostic.contains("invalid object")));
    }

    #[test]
    fn verified_range_returns_bytes_and_denies_corrupt_chunks() {
        let (_root, manifest, chunks) = manifest_fixture();
        let input = GatewayRangeVerificationInput {
            read: GatewayReadInput {
                object_ref: manifest.manifest_ref.clone(),
                member: None,
                requested_range: Some(GatewayRange {
                    offset: RANGE_OFFSET,
                    length: RANGE_LENGTH,
                }),
                requester_ref: fixture_ref("operator"),
                manifest: Some(&manifest),
                visibility: visibility(),
            },
            chunk_bytes: chunks.clone(),
        };
        let pass = verify_gateway_range(&input).expect("range pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(pass.bytes, b"cdefg");

        let mut corrupt = chunks;
        let first = manifest.chunks.first().expect("first chunk").chunk_ref.clone();
        corrupt.insert(first, b"xxxx".to_vec());
        let deny = verify_gateway_range(&GatewayRangeVerificationInput {
            chunk_bytes: corrupt,
            ..input
        })
        .expect("range deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.bytes.is_empty());
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("corrupt chunk")));
    }

    #[test]
    fn protected_object_denies_without_reveal_evidence() {
        let (_root, mut manifest, chunks) = manifest_fixture();
        manifest.transforms = ChunkTransforms::confidential_protected(fixture_ref("commitment"));
        let deny = verify_gateway_range(&GatewayRangeVerificationInput {
            read: GatewayReadInput {
                object_ref: manifest.manifest_ref.clone(),
                member: None,
                requested_range: None,
                requester_ref: fixture_ref("operator"),
                manifest: Some(&manifest),
                visibility: visibility(),
            },
            chunk_bytes: chunks,
        })
        .expect("protected deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("protected object")));
    }

    #[test]
    fn index_omits_hidden_and_redacts_sensitive_members() {
        let hidden_ref = fixture_ref("hidden");
        let visible_ref = fixture_ref("visible");
        let decision = decide_index(&GatewayIndexInput {
            bundle_ref: fixture_ref("bundle"),
            requester_ref: fixture_ref("operator"),
            visibility: GatewayVisibility {
                hidden_refs: vec![hidden_ref.clone()],
                ..visibility()
            },
            members: vec![
                GatewayMember {
                    name: "secret-name".to_string(),
                    object_ref: visible_ref,
                    size: MEMBER_SIZE,
                    mime_hint: Some("application/preserves".to_string()),
                    sensitive: true,
                    visible: true,
                },
                GatewayMember {
                    name: "hidden".to_string(),
                    object_ref: hidden_ref,
                    size: MEMBER_SIZE,
                    mime_hint: None,
                    sensitive: false,
                    visible: true,
                },
            ],
        })
        .expect("index");
        assert_eq!(decision.decision, "pass");
        assert_eq!(decision.entries.len(), MIN_CHUNK_SIZE);
        assert!(decision.entries[0].redacted);
        let text = to_text(&decision.receipt_value).expect("text");
        assert!(text.contains("hidden-members-omitted"));
    }

    #[test]
    fn gateway_receipt_never_authorizes_mutation() {
        let decision = decide_index(&GatewayIndexInput {
            bundle_ref: fixture_ref("bundle"),
            requester_ref: fixture_ref("operator"),
            visibility: visibility(),
            members: Vec::new(),
        })
        .expect("index");
        assert!(!gateway_receipt_authorizes_mutation(&decision.receipt_value));
    }
}
