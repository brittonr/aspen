
pub const RKYV_DERIVED_ARCHIVE_MANIFEST_SCHEMA: &str = "molten.local-eval-cache.rkyv-derived-archive-manifest.v1";
pub const RKYV_DERIVED_ARCHIVE_ADMISSION_SCHEMA: &str = "molten.local-eval-cache.rkyv-derived-archive-admission.v1";

const RKYV_CURRENT_PROFILE: &str = "rkyv-derived-cache-v1";
const RKYV_IDENTITY_DERIVED_SIDECAR: &str = "derived-sidecar";
pub const RKYV_PURPOSE_REPLAY_INDEX: &str = "replay-index";
const RKYV_RETENTION_EPHEMERAL_CACHE: &str = "ephemeral-cache";
const RKYV_RETENTION_REPLAY_SNAPSHOT: &str = "replay-snapshot";
const RKYV_DECISION_ADMIT: &str = "admit";
const RKYV_DECISION_REBUILD: &str = "rebuild";
const RKYV_DECISION_DENY: &str = "deny";
const RKYV_SOURCE_DIGEST_LIMIT: usize = 64;
const RKYV_DIAGNOSTIC_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RkyvSourceDigest {
    pub source_ref: String,
    pub blake3_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RkyvDerivedArchiveManifestInput {
    pub cache_purpose: String,
    pub artifact_kind: String,
    pub profile_version: String,
    pub producer_tool_ref: String,
    pub producer_version: String,
    pub source_digests: Vec<RkyvSourceDigest>,
    pub archive_byte_digest: String,
    pub validation_required: bool,
    pub validation_receipt_ref: Option<String>,
    pub rebuild_capability: Option<String>,
    pub retention_class: String,
    pub identity_claim: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RkyvDerivedArchiveManifest {
    pub manifest_ref: String,
    pub cache_purpose: String,
    pub artifact_kind: String,
    pub profile_version: String,
    pub producer_tool_ref: String,
    pub producer_version: String,
    pub source_digests: Vec<RkyvSourceDigest>,
    pub archive_byte_digest: String,
    pub validation_required: bool,
    pub validation_receipt_ref: Option<String>,
    pub rebuild_capability: Option<String>,
    pub retention_class: String,
    pub identity_claim: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RkyvArchiveAdmissionInput<'a> {
    pub manifest: &'a RkyvDerivedArchiveManifest,
    pub current_sources: &'a [RkyvSourceDigest],
    pub observed_archive_digest: &'a str,
    pub observed_validation_receipt_ref: Option<&'a str>,
    pub validation_passed: bool,
    pub caller_allows_rebuild: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RkyvArchiveAdmission {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub source_refs: Vec<String>,
    pub value: IoValue,
}

pub fn rkyv_source_digest(source_ref: &str, canonical_source: &IoValue) -> Result<RkyvSourceDigest> {
    validate_ref(source_ref, "rkyv source ref")?;
    Ok(RkyvSourceDigest {
        source_ref: source_ref.to_string(),
        blake3_digest: canonical_hash(canonical_source)?,
    })
}

pub fn rkyv_derived_archive_manifest(
    input: &RkyvDerivedArchiveManifestInput,
) -> Result<RkyvDerivedArchiveManifest> {
    validate_rkyv_manifest_input(input)?;
    let value = rkyv_derived_archive_manifest_value(input)?;
    Ok(RkyvDerivedArchiveManifest {
        manifest_ref: canonical_hash(&value)?,
        cache_purpose: input.cache_purpose.clone(),
        artifact_kind: input.artifact_kind.clone(),
        profile_version: input.profile_version.clone(),
        producer_tool_ref: input.producer_tool_ref.clone(),
        producer_version: input.producer_version.clone(),
        source_digests: input.source_digests.clone(),
        archive_byte_digest: input.archive_byte_digest.clone(),
        validation_required: input.validation_required,
        validation_receipt_ref: input.validation_receipt_ref.clone(),
        rebuild_capability: input.rebuild_capability.clone(),
        retention_class: input.retention_class.clone(),
        identity_claim: input.identity_claim.clone(),
        value,
    })
}

pub fn admit_rkyv_derived_archive(input: RkyvArchiveAdmissionInput<'_>) -> Result<RkyvArchiveAdmission> {
    validate_rkyv_sources(input.current_sources, "current rkyv source")?;
    validate_ref(input.observed_archive_digest, "observed rkyv archive digest")?;
    if let Some(receipt_ref) = input.observed_validation_receipt_ref {
        validate_ref(receipt_ref, "observed rkyv validation receipt")?;
    }
    let mut diagnostics = Vec::new();
    collect_rkyv_admission_diagnostics(input, &mut diagnostics)?;
    let decision = rkyv_admission_decision(input, &diagnostics);
    let source_refs = input
        .manifest
        .source_digests
        .iter()
        .map(|source| source.source_ref.clone())
        .collect::<Vec<_>>();
    let value = rkyv_archive_admission_value(&decision, input, &source_refs, &diagnostics)?;
    Ok(RkyvArchiveAdmission {
        decision,
        diagnostics,
        manifest_ref: input.manifest.manifest_ref.clone(),
        source_refs,
        value,
    })
}
