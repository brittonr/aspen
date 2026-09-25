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
