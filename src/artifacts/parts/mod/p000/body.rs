use redb::ReadableDatabase;
use redb::ReadableTable;

type LocalStorePath = crate::local_store::LocalStorePath;
type Path = std::path::Path;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

type TableDef<'a, K, V> = redb::TableDefinition<'a, K, V>;
type RailValue = preserves::Value<IoValue>;

pub const INLINE_PAYLOAD_LIMIT: usize = 4096;

const MAX_ARTIFACT_REF_LIST: usize = 4096;
const MAX_ARTIFACT_RECORDS: usize = 100_000;
const MAX_ARTIFACT_POINTERS: usize = 100_000;
const MAX_ARTIFACT_RECEIPTS: usize = 100_000;
const MAX_ARTIFACT_DIAGNOSTICS: usize = 256;
const MAX_ARTIFACT_CHECKS: usize = 64;
const RELEASE_SNAPSHOT_RECORD_ARITY: usize = 16;
pub const ARTIFACT_IDENTITY_HASH_ALGORITHM: &str = "blake3";
pub const RELEASE_SNAPSHOT_ARTIFACT_KIND: &str = "release-snapshot";
const RAW_SOURCE_CANONICALIZER: &str = "raw-source-text";
const RENDERED_LOG_CANONICALIZER: &str = "rendered-log";
const PRESERVES_VALUE_CANONICALIZER: &str = "preserves-canonical-value-v1";
const SUPPORTED_ARTIFACT_KINDS: &[&str] = &[
    "artifact",
    "authority-context",
    "doc",
    "job-dag",
    "module",
    "nickel",
    "node-control-artifact",
    "octet-evidence",
    "octet-gate-receipt",
    "operator-artifact",
    "plugin-executor",
    "preserves-schema",
    "receipt",
    RELEASE_SNAPSHOT_ARTIFACT_KIND,
    "schema",
    "schema-identity",
    "stage",
    "steel",
    "transcript",
    "transcript-example",
    "transcript-run-receipt",
    "trellis",
    "upgrade-receipt",
    "wasm",
];

const _: () = assert!(INLINE_PAYLOAD_LIMIT <= 1_048_576);
const _: () = assert!(MAX_ARTIFACT_REF_LIST <= 100_000);
const _: () = assert!(MAX_ARTIFACT_RECORDS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_POINTERS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_RECEIPTS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_ARTIFACT_CHECKS <= 1_000);
const _: () = assert!(RELEASE_SNAPSHOT_RECORD_ARITY <= MAX_ARTIFACT_CHECKS);

const INDEX_FILE: &str = "artifact-registry.redb";
const INDEX_ARTIFACTS: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_artifacts_v1");
const INDEX_PAYLOADS: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_payloads_v1");
const INDEX_NAMES: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_names_v1");
const INDEX_DEPS: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_dependencies_v1");
const INDEX_REVERSE: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_reverse_dependencies_v1");
const INDEX_KIND: TableDef<&str, &str> = TableDef::new("artifact_registry_kind_v1");
const INDEX_SCHEMA: TableDef<&str, &str> = TableDef::new("artifact_registry_schema_v1");
const INDEX_EFFECT: TableDef<&str, &str> = TableDef::new("artifact_registry_effect_v1");
const INDEX_POLICY: TableDef<&str, &str> = TableDef::new("artifact_registry_policy_v1");
const INDEX_EVIDENCE: TableDef<&str, &str> = TableDef::new("artifact_registry_evidence_v1");
const INDEX_RECEIPTS: TableDef<&str, &[u8]> = TableDef::new("artifact_registry_receipts_v1");

pub type CapabilityArtifactRoot = crate::local_store::ArtifactStoreRoot;

pub fn open_capability_artifact_root(root: &Path) -> Result<CapabilityArtifactRoot> {
    crate::local_store::ArtifactStoreRoot::open(root)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactPayloadRef {
    Inline { value_ref: String, length: u64 },
    ContentRef { manifest_ref: String, length: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstallInput {
    pub kind: String,
    pub payload: IoValue,
    pub schema_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub effect_manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub installer_ref: String,
    pub capability_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactValueInput<'a> {
    pub kind: &'a str,
    pub payload: &'a ArtifactPayloadRef,
    pub schema_refs: &'a [String],
    pub dependency_refs: &'a [String],
    pub effect_manifest_ref: Option<&'a str>,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct SetNamePointerInput<'a> {
    pub pointer_kind: &'a str,
    pub name: &'a str,
    pub artifact_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_ref: String,
    pub kind: String,
    pub domain: String,
    pub payload: ArtifactPayloadRef,
    pub schema_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub effect_manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstall {
    pub artifact_ref: String,
    pub decision: String,
    pub artifact: ArtifactRecord,
    pub identity_receipt_ref: String,
    pub identity_receipt_value: IoValue,
    pub missing_dependencies: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNamePointer {
    pub pointer_ref: String,
    pub pointer_kind: String,
    pub name: String,
    pub artifact_ref: String,
    pub previous_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameViewInput {
    pub view_kind: String,
    pub name: String,
    pub scope: String,
    pub target_kind: String,
    pub target_ref: String,
    pub issuer_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub tombstone_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameView {
    pub view_ref: String,
    pub view_kind: String,
    pub name: String,
    pub scope: String,
    pub target_kind: String,
    pub target_ref: String,
    pub issuer_ref: String,
    pub previous_view_ref: Option<String>,
    pub tombstone_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameViewUpdate {
    pub view: ArtifactNameView,
    pub pointer: ArtifactNamePointer,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameResolutionInput {
    pub view_kind: String,
    pub name: String,
    pub scope: Option<String>,
    pub candidate_views: Vec<ArtifactNameView>,
    pub stale_view_refs: Vec<String>,
    pub normative_use: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameResolution {
    pub resolution_ref: String,
    pub decision: String,
    pub resolved_ref: Option<String>,
    pub candidate_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameUseInput {
    pub operation: String,
    pub name: Option<String>,
    pub exact_artifact_ref: Option<String>,
    pub resolution_receipt_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub capability_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNameUseReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub subject_ref: String,
    pub name: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactIdentityInput<'a> {
    pub kind: &'a str,
    pub identity_domain: &'a str,
    pub canonical_payload_ref: &'a str,
    pub canonicalizer: &'a str,
    pub artifact_ref: Option<&'a str>,
    pub schema_refs: &'a [String],
    pub dependency_summary_refs: &'a [String],
    pub effect_manifest_ref: Option<&'a str>,
    pub policy_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub hash_algorithm: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIdentityReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub artifact_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactDependencyEdge {
    pub edge_ref: String,
    pub source_ref: String,
    pub target_ref: String,
    pub target_kind: String,
    pub relation: String,
    pub required: bool,
    pub scope: String,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpactQueryInput {
    pub subject_ref: String,
    pub relation_filters: Vec<String>,
    pub include_transitive: bool,
    pub hidden_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpactQueryReceipt {
    pub query_ref: String,
    pub decision: String,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub redacted_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotDraftInput {
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotValueInput {
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub dependency_closure_digest: String,
    pub dependency_index_ref: String,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_subject_ref: String,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshot {
    pub snapshot_ref: String,
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub dependency_closure_digest: String,
    pub dependency_index_ref: String,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_subject_ref: String,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotInstallInput {
    pub snapshot: ReleaseSnapshotDraftInput,
    pub installer_ref: String,
    pub capability_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotInstall {
    pub artifact_ref: String,
    pub snapshot: ReleaseSnapshot,
    pub install: ArtifactInstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotVerifyInput {
    pub snapshot_ref: String,
    pub required_caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub snapshot_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelUpdateInput {
    pub channel: String,
    pub snapshot_ref: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelUpdate {
    pub pointer: ArtifactNamePointer,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelAdmissionInput {
    pub channel_pointer_ref: String,
    pub release_evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelAdmissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactClosure {
    pub roots: Vec<String>,
    pub closure_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub closure_hash: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpact {
    pub seeds: Vec<String>,
    pub impacted_refs: Vec<String>,
    pub impact_hash: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIndexRebuild {
    pub artifacts: usize,
    pub names: usize,
    pub receipt_value: IoValue,
}

pub fn install_artifact(root: &Path, input: &ArtifactInstallInput) -> Result<ArtifactInstall> {
    let root = open_capability_artifact_root(root)?;
    install_artifact_with_root(&root, input)
}

pub fn install_artifact_with_root(
    root: &CapabilityArtifactRoot,
    input: &ArtifactInstallInput,
) -> Result<ArtifactInstall> {
    validate_install_input(input)?;
    ensure_dirs(root)?;
    let payload = prepare_install_payload(root, &input.payload)?;
    let artifact = build_install_artifact(input, &payload.payload_ref)?;
    let identity_receipt = artifact_identity_receipt(&identity_input_from_artifact(&artifact))?;
    if identity_receipt.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "artifact identity denied: {}",
            identity_receipt.diagnostics.join("; ")
        )));
    }
    let missing_dependencies = missing_dependencies(root, &input.dependency_refs)?;
    let decision = install_decision(&missing_dependencies);
    let refs = install_refs(input, &artifact, &identity_receipt.receipt_ref, payload.chunk_receipt_ref.as_ref())?;
    let diagnostics = install_diagnostics(&missing_dependencies)?;
    let receipt_value = install_receipt_value(&artifact, decision, &refs, &diagnostics, &missing_dependencies)?;
    commit_install(root, &artifact, &payload.payload_bytes, &receipt_value, missing_dependencies.is_empty())?;
    Ok(ArtifactInstall {
        artifact_ref: artifact.artifact_ref.clone(),
        decision: decision.to_string(),
        artifact,
        identity_receipt_ref: identity_receipt.receipt_ref,
        identity_receipt_value: identity_receipt.value,
        missing_dependencies,
        receipt_value,
    })
}

struct InstallPayload {
    payload_bytes: Vec<u8>,
    payload_ref: ArtifactPayloadRef,
    chunk_receipt_ref: Option<String>,
}

fn prepare_install_payload(root: &CapabilityArtifactRoot, payload: &IoValue) -> Result<InstallPayload> {
    let payload_bytes = canonical_bytes(payload)?;
    let payload_value_ref = canonical_hash(payload)?;
    let (payload_ref, chunk_receipt_ref) = if payload_bytes.len() <= INLINE_PAYLOAD_LIMIT {
        (
            ArtifactPayloadRef::Inline {
                value_ref: payload_value_ref,
                length: payload_bytes.len() as u64,
            },
            None,
        )
    } else {
        let put = put_payload_bytes(root, &payload_bytes)?;
        (
            ArtifactPayloadRef::ContentRef {
                manifest_ref: put.manifest_ref,
                length: payload_bytes.len() as u64,
            },
            Some(canonical_hash(&put.receipt_value)?),
        )
    };
    Ok(InstallPayload {
        payload_bytes,
        payload_ref,
        chunk_receipt_ref,
    })
}

fn build_install_artifact(input: &ArtifactInstallInput, payload_ref: &ArtifactPayloadRef) -> Result<ArtifactRecord> {
    let value = artifact_value(ArtifactValueInput {
        kind: &input.kind,
        payload: payload_ref,
        schema_refs: &input.schema_refs,
        dependency_refs: &input.dependency_refs,
        effect_manifest_ref: input.effect_manifest_ref.as_deref(),
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
    })?;
    parse_artifact_value(&value)
}

fn install_decision(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "deny"
    }
}

fn install_refs(
    input: &ArtifactInstallInput,
    artifact: &ArtifactRecord,
    identity_receipt_ref: &str,
    chunk_receipt_ref: Option<&String>,
) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    push_bounded(&mut refs, identity_receipt_ref.to_string(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    push_bounded(&mut refs, input.installer_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.dependency_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.schema_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        push_bounded(&mut refs, effect_manifest_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    if let Some(chunk_receipt_ref) = chunk_receipt_ref {
        push_bounded(&mut refs, chunk_receipt_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    Ok(refs)
}

fn install_diagnostics(missing_dependencies: &[String]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for dependency in missing_dependencies {
        push_bounded(
            &mut diagnostics,
            format!("missing dependency {dependency}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact install diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn install_receipt_value(
    artifact: &ArtifactRecord,
    decision: &str,
    refs: &[String],
    diagnostics: &[String],
    missing_dependencies: &[String],
) -> Result<IoValue> {
    artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "install",
        decision,
        subject_ref: &artifact.artifact_ref,
        name: None,
        refs,
        diagnostics,
        checks: &[
            ("domain-separated-identity", "pass"),
            ("canonical-payload-ref", "pass"),
            ("dependency-closure", dependency_check(missing_dependencies)),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
            ("names-are-metadata", "pass"),
        ],
    })
}

fn dependency_check(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "fail"
    }
}
