use crate::bounded::VecSink;

type OrderedSet<T> = std::collections::BTreeSet<T>;
type CompoundClass = preserves::CompoundClass;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;
type ValueClass = preserves::ValueClass;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
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

pub const CLASS_EPHEMERAL_CACHE: &str = "ephemeral-cache";
pub const CLASS_DEBUG_TRACE: &str = "debug-trace";
pub const CLASS_REPLAY_SNAPSHOT: &str = "replay-snapshot";
pub const CLASS_AUDIT_RECEIPT: &str = "audit-receipt";
pub const CLASS_DURABLE_VALUE: &str = "durable-value";
pub const CLASS_PUBLIC_ARTIFACT: &str = "public-artifact";
pub const CLASS_PRIVATE_SECRET_REF: &str = "private-secret-ref";
pub const CLASS_UPGRADE_ROLLBACK: &str = "upgrade-rollback";
pub const CLASS_LEGAL_HOLD: &str = "legal-hold";

pub const SOURCE_ACTIVE_SESSION: &str = "active-session";
pub const SOURCE_ARTIFACT: &str = "artifact";
pub const SOURCE_BLOB: &str = "blob";
pub const SOURCE_RECEIPT: &str = "receipt";
pub const SOURCE_SNAPSHOT: &str = "snapshot";
pub const SOURCE_TRANSCRIPT: &str = "transcript";
pub const SOURCE_DOC: &str = "doc";
pub const SOURCE_POLICY: &str = "policy";
pub const SOURCE_UPGRADE: &str = "upgrade";
pub const SOURCE_STORAGE_REF: &str = "storage-ref";
pub const SOURCE_REMOTE_CACHE: &str = "remote-cache";
pub const SOURCE_EVALUATION_CACHE: &str = "evaluation-cache";
pub const SOURCE_OPERATOR_HOLD: &str = "operator-hold";
pub const SOURCE_LEGAL_HOLD: &str = "legal-hold";
pub const SOURCE_SECRET_REDACTION: &str = "secret-redaction";

pub const ACTION_PIN: &str = "pin";
pub const ACTION_UNPIN: &str = "unpin";
pub const ACTION_RETAIN: &str = "retain";
pub const ACTION_ELIGIBILITY: &str = "eligibility";
pub const ACTION_DELETE: &str = "delete";
pub const ACTION_TOMBSTONE: &str = "tombstone";
pub const ACTION_REDACT: &str = "redact";
pub const ACTION_COMPACT: &str = "compact";

pub const ADMISSION_KIND_POLICY: &str = "policy";
pub const ADMISSION_KIND_AUTHORITY: &str = "authority";
pub const ADMISSION_KIND_SUPPORTING_EVIDENCE: &str = "supporting-evidence";
pub const ADMISSION_KIND_REFERENCE_INDEX: &str = "reference-index";
pub const ADMISSION_KIND_REMOTE_GC: &str = "remote-gc";

const STORE_DIR: &str = "retention";
const PIN_DIR: &str = "pins";
const ADMISSION_DIR: &str = "admissions";
const REMOTE_CLEARANCE_DIR: &str = "remote-clearances";
const REMOTE_CLEARANCE_REQUEST_DIR: &str = "remote-clearance-requests";
const REMOTE_CLEARANCE_RESPONSE_DIR: &str = "remote-clearance-responses";
const REMOTE_CLEARANCE_IMPORT_DIR: &str = "remote-clearance-imports";
const REMOTE_CLEARANCE_LIVE_WORKFLOW_DIR: &str = "remote-clearance-live-workflows";
const GC_PLAN_DIR: &str = "gc-plans";
const GC_APPLY_DIR: &str = "gc-applies";
const GC_EXECUTE_DIR: &str = "gc-executes";
const GC_AUDIT_DIR: &str = "gc-audits";
const RECEIPT_DIR: &str = "receipts";
const TOMBSTONE_DIR: &str = "tombstones";
const BUNDLE_PROFILE_FILE: &str = "bundle-profile.preserves";
const BUNDLE_REDACTED_DIR: &str = "redacted";
const MAX_RETENTION_REFS: usize = 4096;
const MAX_RETENTION_DIAGNOSTICS: usize = 128;
const MAX_RETENTION_TEXT_LEN: usize = 1024;
const RETENTION_GC_LIFECYCLE_DIAGNOSTIC_CAPACITY: usize = 16;
const APPLY_DIAGNOSTICS: &str = "retention GC apply diagnostics";
const MAX_REF_FILE_NAME: usize = 128;
const _: () = assert!(MAX_RETENTION_REFS <= 100_000);
const _: () = assert!(MAX_RETENTION_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_RETENTION_TEXT_LEN <= 4096);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassProfileInput {
    pub class_name: String,
    pub minimum_age_seconds: u64,
    pub maximum_age_seconds: Option<u64>,
    pub deletion_authority_ref: String,
    pub policy_refs: Vec<String>,
    pub has_secret_redaction_hook: bool,
    pub has_remote_gc_plan: bool,
    pub can_compact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassProfile {
    pub profile_ref: String,
    pub class_name: String,
    pub minimum_age_seconds: u64,
    pub maximum_age_seconds: Option<u64>,
    pub deletion_authority_ref: String,
    pub policy_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinInput {
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub source: String,
    pub reason: String,
    pub owner_ref: String,
    pub expiry_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub has_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub pin_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub source: String,
    pub reason: String,
    pub owner_ref: String,
    pub expiry_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceIndexInput {
    pub object_ref: String,
    pub object_kind: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceIndex {
    pub index_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_complete: bool,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct UnpinObjectInput<'a> {
    pub root: &'a Path,
    pub pin_ref: &'a str,
    pub requester_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub has_authority: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ReferenceIndexForObjectInput<'a> {
    pub root: &'a Path,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retained_refs: &'a [String],
    pub remote_refs: &'a [String],
    pub is_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationInput<'a> {
    pub root: &'a Path,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub requester_ref: &'a str,
    pub is_reference_index_complete: bool,
    pub retained_refs: &'a [String],
    pub remote_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub has_delete_authority: bool,
    pub has_remote_gc_clearance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DestructiveEvidence {
    pub requester_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub remote_peer_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub reference_index_refs: Vec<String>,
    pub remote_gc_refs: Vec<String>,
    pub remote_clearance_refs: Vec<String>,
    pub is_reference_index_complete: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GcPlanInput<'a> {
    pub root: &'a Path,
    pub subsystem: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub evidence: &'a DestructiveEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGate {
    pub name: String,
    pub decision: String,
    pub required_refs: Vec<String>,
    pub admitted_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcPlan {
    pub plan_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub requester_ref: Option<String>,
    pub index_ref: String,
    pub evidence: DestructiveEvidence,
    pub gates: Vec<PlanGate>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct GcApplyFromPlanInput<'a> {
    pub root: &'a Path,
    pub plan_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcApply {
    pub apply_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub requester_ref: Option<String>,
    pub plan_ref: String,
    pub recomputed_plan_ref: String,
    pub retention_receipt_ref: Option<String>,
    pub tombstone_ref: Option<String>,
    pub admission_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}
