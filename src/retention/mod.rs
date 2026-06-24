use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::CompoundClass;
use preserves::IOValue;
use preserves::Value;
use preserves::ValueClass;

use crate::bounded::VecSink;
use crate::error::MoltenError;
use crate::error::Result;
use crate::node_daemon;
use crate::node_runtime;
use crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA;
use crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_SCHEMA;
use crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA;
use crate::preserves_rail::RETENTION_CANDIDATE_EXPLAIN_SCHEMA;
use crate::preserves_rail::RETENTION_CLASS_SCHEMA;
use crate::preserves_rail::RETENTION_EVIDENCE_ADMISSION_SCHEMA;
use crate::preserves_rail::RETENTION_GC_APPLY_SCHEMA;
use crate::preserves_rail::RETENTION_GC_AUDIT_SCHEMA;
use crate::preserves_rail::RETENTION_GC_EXECUTE_SCHEMA;
use crate::preserves_rail::RETENTION_GC_PLAN_SCHEMA;
use crate::preserves_rail::RETENTION_PIN_SCHEMA;
use crate::preserves_rail::RETENTION_RECEIPT_SCHEMA;
use crate::preserves_rail::RETENTION_REFERENCE_INDEX_SCHEMA;
use crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA;
use crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA;
use crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA;
use crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA;
use crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_SCHEMA;
use crate::preserves_rail::RETENTION_TOMBSTONE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

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
const MAX_REF_FILE_NAME: usize = 128;
const _: () = assert!(MAX_RETENTION_REFS <= 100_000);
const _: () = assert!(MAX_RETENTION_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_RETENTION_TEXT_LEN <= 4096);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionClassProfileInput {
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
pub struct RetentionClassProfile {
    pub profile_ref: String,
    pub class_name: String,
    pub minimum_age_seconds: u64,
    pub maximum_age_seconds: Option<u64>,
    pub deletion_authority_ref: String,
    pub policy_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPinInput {
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
pub struct RetentionPin {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionReferenceIndexInput {
    pub object_ref: String,
    pub object_kind: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionReferenceIndex {
    pub index_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_complete: bool,
    pub value: IOValue,
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
pub struct RetentionEvaluationInput<'a> {
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
pub struct DestructiveRetentionEvidence {
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
pub struct RetentionGcPlanInput<'a> {
    pub root: &'a Path,
    pub subsystem: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub evidence: &'a DestructiveRetentionEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPlanGate {
    pub name: String,
    pub decision: String,
    pub required_refs: Vec<String>,
    pub admitted_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionGcPlan {
    pub plan_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub requester_ref: Option<String>,
    pub index_ref: String,
    pub evidence: DestructiveRetentionEvidence,
    pub gates: Vec<RetentionPlanGate>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionGcApplyFromPlanInput<'a> {
    pub root: &'a Path,
    pub plan_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionGcApply {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionGcExecutionGateInput<'a> {
    pub root: &'a Path,
    pub subsystem: &'a str,
    pub action: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub apply_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionGcExecutionGate {
    pub execution_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub apply_ref: Option<String>,
    pub plan_ref: Option<String>,
    pub recomputed_plan_ref: Option<String>,
    pub retention_receipt_ref: Option<String>,
    pub tombstone_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionGcAuditInput<'a> {
    pub root: &'a Path,
    pub execution_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionGcAudit {
    pub audit_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub plan_ref: Option<String>,
    pub plan_decision: String,
    pub apply_ref: Option<String>,
    pub apply_decision: String,
    pub execution_ref: String,
    pub execution_decision: String,
    pub retention_receipt_ref: Option<String>,
    pub retention_receipt_decision: String,
    pub tombstone_ref: Option<String>,
    pub tombstone_status: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionCandidateExplainInput<'a> {
    pub root: &'a Path,
    pub object_ref: &'a str,
    pub object_kind: Option<&'a str>,
    pub retention_class: Option<&'a str>,
    pub action: Option<&'a str>,
    pub subsystem: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCandidateExplain {
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub pin_refs: Vec<String>,
    pub admission_refs: Vec<String>,
    pub remote_clearance_refs: Vec<String>,
    pub remote_clearance_import_refs: Vec<String>,
    pub gc_plan_refs: Vec<String>,
    pub gc_apply_refs: Vec<String>,
    pub gc_execution_refs: Vec<String>,
    pub gc_audit_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionCandidateBundleExportProfile {
    Internal,
    Public,
    Diagnostic,
}

impl RetentionCandidateBundleExportProfile {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "internal" => Ok(Self::Internal),
            "public" => Ok(Self::Public),
            "diagnostic" => Ok(Self::Diagnostic),
            _ => Err(MoltenError::invalid_harness(format!(
                "unsupported retention bundle export profile {value}; expected internal, public, or diagnostic"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Public => "public",
            Self::Diagnostic => "diagnostic",
        }
    }

    fn loss_classification(self) -> &'static str {
        match self {
            Self::Internal => "local-full-fidelity",
            Self::Public => "deny-sensitive",
            Self::Diagnostic => "diagnostic-redacted-view",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionCandidateBundleExportInput<'a> {
    pub root: &'a Path,
    pub explain_value: &'a IOValue,
    pub out: &'a Path,
    pub profile: RetentionCandidateBundleExportProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCandidateBundle {
    pub bundle_ref: String,
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub gc_plan_refs: Vec<String>,
    pub gc_apply_refs: Vec<String>,
    pub gc_execution_refs: Vec<String>,
    pub gc_audit_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCandidateBundleProfile {
    pub profile_ref: String,
    pub decision: String,
    pub profile: String,
    pub loss_classification: String,
    pub bundle_ref: String,
    pub marker_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionCandidateBundleVerifyInput<'a> {
    pub bundle_dir: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCandidateBundleVerify {
    pub verify_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub artifact_refs: Vec<String>,
    pub file_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionEvidenceAdmissionInput<'a> {
    pub kind: &'a str,
    pub decision: &'a str,
    pub requester_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub bound_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub remote_refs: &'a [String],
    pub is_reference_index_complete: bool,
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionEvidenceAdmission {
    pub admission_ref: String,
    pub kind: String,
    pub decision: String,
    pub requester_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub bound_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_reference_index_complete: bool,
    pub is_current: bool,
    pub revoked_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceInput<'a> {
    pub decision: &'a str,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearance {
    pub clearance_ref: String,
    pub decision: String,
    pub requester_ref: String,
    pub peer_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub remote_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub evidence_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub is_current: bool,
    pub revoked_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceRequestInput<'a> {
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceRequest {
    pub request_ref: String,
    pub requester_ref: String,
    pub peer_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub remote_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceResponseInput<'a> {
    pub root: &'a Path,
    pub request_value: &'a IOValue,
    pub evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceResponse {
    pub response_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub request: RetentionRemoteGcClearanceRequest,
    pub clearance_ref: String,
    pub clearance: RetentionRemoteGcClearance,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceImportInput<'a> {
    pub root: &'a Path,
    pub request_value: &'a IOValue,
    pub response_value: &'a IOValue,
    pub expected_peer_ref: Option<&'a str>,
    pub expected_remote_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceImportValueInput<'a> {
    pub decision: &'a str,
    pub request_ref: &'a str,
    pub response_ref: &'a str,
    pub clearance_ref: Option<&'a str>,
    pub peer_ref: &'a str,
    pub remote_ref: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceImport {
    pub import_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub clearance_ref: Option<String>,
    pub peer_ref: String,
    pub remote_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceLiveLoopbackInput<'a> {
    pub root: &'a Path,
    pub requester_node_root: &'a Path,
    pub peer_node_root: &'a Path,
    pub requester_node_id: &'a str,
    pub peer_node_id: &'a str,
    pub topic: &'a str,
    pub request_sequence: u64,
    pub response_sequence: u64,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub retention_evidence_refs: &'a [String],
    pub response_evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub response_diagnostics: &'a [String],
    pub request_peer_bootstrap_refs: &'a [String],
    pub request_authority_refs: &'a [String],
    pub request_policy_refs: &'a [String],
    pub request_resource_refs: &'a [String],
    pub request_transport_evidence_refs: &'a [String],
    pub response_peer_bootstrap_refs: &'a [String],
    pub response_authority_refs: &'a [String],
    pub response_policy_refs: &'a [String],
    pub response_resource_refs: &'a [String],
    pub response_transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceLiveRequestSendInput<'a> {
    pub root: &'a Path,
    pub requester_node_root: Option<&'a Path>,
    pub peer_ticket_value: &'a IOValue,
    pub requester_node_id: &'a str,
    pub peer_node_id: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub retention_evidence_refs: &'a [String],
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceLiveResponseSendInput<'a> {
    pub root: &'a Path,
    pub peer_node_root: Option<&'a Path>,
    pub requester_ticket_value: &'a IOValue,
    pub request_value: &'a IOValue,
    pub peer_node_id: &'a str,
    pub requester_node_id: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
    pub response_evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub response_diagnostics: &'a [String],
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceLiveImportWorkflowInput<'a> {
    pub root: &'a Path,
    pub request_value: &'a IOValue,
    pub response_value: &'a IOValue,
    pub request_control_value: &'a IOValue,
    pub request_send_receipt_value: &'a IOValue,
    pub request_receive_receipt_value: &'a IOValue,
    pub request_ingress_ref: &'a str,
    pub response_control_value: &'a IOValue,
    pub response_send_receipt_value: &'a IOValue,
    pub response_receive_receipt_value: &'a IOValue,
    pub response_ingress_ref: &'a str,
    pub expected_peer_ref: Option<&'a str>,
    pub expected_remote_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionRemoteGcClearanceLiveWorkflowValueInput<'a> {
    pub request_value: &'a IOValue,
    pub response_value: &'a IOValue,
    pub import_value: &'a IOValue,
    pub request_control_ref: &'a str,
    pub request_publish_ref: &'a str,
    pub request_receive_ref: &'a str,
    pub request_ingress_ref: &'a str,
    pub response_control_ref: &'a str,
    pub response_publish_ref: &'a str,
    pub response_receive_ref: &'a str,
    pub response_ingress_ref: &'a str,
    pub transport_diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceLiveWorkflow {
    pub workflow_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub import_ref: String,
    pub clearance_ref: Option<String>,
    pub peer_ref: String,
    pub remote_ref: String,
    pub request_live_refs: Vec<String>,
    pub response_live_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceLiveLoopback {
    pub request: RetentionRemoteGcClearanceRequest,
    pub response: RetentionRemoteGcClearanceResponse,
    pub import: RetentionRemoteGcClearanceImport,
    pub workflow: RetentionRemoteGcClearanceLiveWorkflow,
    pub request_publish_receipt_value: IOValue,
    pub request_receive_receipt_value: IOValue,
    pub response_publish_receipt_value: IOValue,
    pub response_receive_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceLiveRequestSend {
    pub request: RetentionRemoteGcClearanceRequest,
    pub control_ref: String,
    pub control_value: IOValue,
    pub send: node_daemon::NodeControlLiveSend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceLiveResponseSend {
    pub response: RetentionRemoteGcClearanceResponse,
    pub control_ref: String,
    pub control_value: IOValue,
    pub send: node_daemon::NodeControlLiveSend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRemoteGcClearanceLiveImportWorkflow {
    pub import: RetentionRemoteGcClearanceImport,
    pub workflow: RetentionRemoteGcClearanceLiveWorkflow,
    pub request_send_receipt_ref: String,
    pub response_send_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestructiveRetentionAdmission {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub admitted_refs: Vec<String>,
    pub has_delete_authority: bool,
    pub has_remote_gc_clearance: bool,
}

pub struct DestructiveRetentionAdmissionInput<'a> {
    pub root: &'a Path,
    pub evidence: &'a DestructiveRetentionEvidence,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub requester_ref: String,
    pub index_ref: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub tombstone_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionTombstone {
    pub tombstone_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub receipt_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinOperation {
    pub pin: RetentionPin,
    pub receipt: RetentionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionEvaluation {
    pub receipt: RetentionReceipt,
    pub index: RetentionReferenceIndex,
    pub tombstone: Option<RetentionTombstone>,
}

pub fn retention_class_profile_value(input: &RetentionClassProfileInput) -> Result<IOValue> {
    validate_class_profile_input(input)?;
    let diagnostics = class_profile_diagnostics(input)?;
    Ok(record("retention-class-v1", vec![
        string(RETENTION_CLASS_SCHEMA),
        record("class", vec![string(&input.class_name)]),
        record("minimum-age-seconds", vec![u64_value(input.minimum_age_seconds)]),
        record("maximum-age-seconds", vec![optional_u64_value(input.maximum_age_seconds)]),
        record("deletion-authority", vec![string(&input.deletion_authority_ref)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("capabilities", vec![sequence(vec![
            record("secret-redaction-hook", vec![string(pass_or_deny(input.has_secret_redaction_hook))]),
            record("remote-gc-plan", vec![string(pass_or_deny(input.has_remote_gc_plan))]),
            record("compaction", vec![string(pass_or_deny(input.can_compact))]),
        ])]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("class-known", "pass"),
            ("policy-bound", "pass"),
            ("mutable-name-not-gc-proof", "pass"),
        ]),
    ]))
}

pub fn parse_retention_class_profile(value: &IOValue) -> Result<RetentionClassProfile> {
    let fields = value
        .collect_simple_record("retention-class-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-class-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_CLASS_SCHEMA, "retention class schema")?;
    let class_name = record_string(&fields[1], "class")?;
    let minimum_age_seconds = record_u64(&fields[2], "minimum-age-seconds")?;
    let maximum_age_seconds = record_optional_u64(&fields[3], "maximum-age-seconds")?;
    let deletion_authority_ref = record_ref(&fields[4], "deletion-authority")?;
    let policy_refs = record_ref_sequence(&fields[5], "policy")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    validate_retention_class(&class_name)?;
    require_check(&parse_checks(&fields[8])?, "mutable-name-not-gc-proof", "retention class profile")?;
    Ok(RetentionClassProfile {
        profile_ref: canonical_hash(value)?,
        class_name,
        minimum_age_seconds,
        maximum_age_seconds,
        deletion_authority_ref,
        policy_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn retention_pin_value(input: &RetentionPinInput) -> Result<IOValue> {
    validate_pin_input(input)?;
    let authority_status = if input.has_authority { "pass" } else { "deny" };
    Ok(record("retention-pin-v1", vec![
        string(RETENTION_PIN_SCHEMA),
        object_value(&input.object_ref, &input.object_kind),
        record("class", vec![string(&input.retention_class)]),
        record("source", vec![string(&input.source)]),
        record("reason", vec![string(&input.reason)]),
        record("owner", vec![string(&input.owner_ref)]),
        record("expiry", vec![optional_ref_value(input.expiry_ref.as_deref())]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        checks_value(&[
            ("object-ref-bound", "pass"),
            ("pin-source-bound", "pass"),
            ("authority-bound", authority_status),
            ("mutable-name-not-gc-proof", "pass"),
        ]),
    ]))
}

pub fn parse_retention_pin(value: &IOValue) -> Result<RetentionPin> {
    let fields = value
        .collect_simple_record("retention-pin-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-pin-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_PIN_SCHEMA, "retention pin schema")?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let retention_class = record_string(&fields[2], "class")?;
    let source = record_string(&fields[3], "source")?;
    let reason = record_string(&fields[4], "reason")?;
    let owner_ref = record_ref(&fields[5], "owner")?;
    let expiry_ref = record_optional_ref(&fields[6], "expiry")?;
    let policy_refs = record_ref_sequence(&fields[7], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[8], "evidence")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "object-ref-bound", "retention pin")?;
    require_check(&checks, "pin-source-bound", "retention pin")?;
    validate_retention_class(&retention_class)?;
    validate_pin_source(&source)?;
    Ok(RetentionPin {
        pin_ref: canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        source,
        reason,
        owner_ref,
        expiry_ref,
        policy_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn pin_object(root: &Path, input: RetentionPinInput) -> Result<PinOperation> {
    ensure_store(root)?;
    let pin_value = retention_pin_value(&input)?;
    let pin = parse_retention_pin(&pin_value)?;
    write_store_value(&pin_path(root, &pin.pin_ref)?, &pin.value)?;
    let index = reference_index_for_object(ReferenceIndexForObjectInput {
        root,
        object_ref: &pin.object_ref,
        object_kind: &pin.object_kind,
        retained_refs: &[],
        remote_refs: &[],
        is_complete: true,
    })?;
    let diagnostics = if input.has_authority {
        Vec::new()
    } else {
        vec!["pin-authority-missing".to_string()]
    };
    let decision = if input.has_authority { "pass" } else { "deny" };
    let receipt = retention_receipt(RetentionReceiptBuildInput {
        decision,
        action: ACTION_PIN,
        object_ref: &pin.object_ref,
        object_kind: &pin.object_kind,
        retention_class: &pin.retention_class,
        requester_ref: &pin.owner_ref,
        index_ref: &index.index_ref,
        pin_refs: std::slice::from_ref(&pin.pin_ref),
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &pin.policy_refs,
        evidence_refs: &pin.evidence_refs,
        tombstone_ref: None,
        diagnostics: &diagnostics,
    })?;
    write_store_value(&receipt_path(root, &receipt.receipt_ref)?, &receipt.value)?;
    Ok(PinOperation { pin, receipt })
}

pub fn unpin_object(input: UnpinObjectInput<'_>) -> Result<RetentionReceipt> {
    ensure_store(input.root)?;
    require_ref(input.pin_ref, "pin ref")?;
    require_ref(input.requester_ref, "requester ref")?;
    validate_refs(input.policy_refs, "unpin policy ref")?;
    validate_refs(input.evidence_refs, "unpin evidence ref")?;
    let pin_file = pin_path(input.root, input.pin_ref)?;
    let pin_result = read_store_value(&pin_file).and_then(|value| parse_retention_pin(&value));
    let (decision, object_ref, object_kind, retention_class, diagnostics) = match pin_result {
        Ok(pin) if input.has_authority => {
            fs::remove_file(&pin_file).map_err(MoltenError::from)?;
            ("pass", pin.object_ref, pin.object_kind, pin.retention_class, Vec::new())
        }
        Ok(pin) => ("deny", pin.object_ref, pin.object_kind, pin.retention_class, vec![
            "unpin-authority-missing".to_string(),
        ]),
        Err(_) => ("deny", input.pin_ref.to_string(), "unknown".to_string(), CLASS_AUDIT_RECEIPT.to_string(), vec![
            "pin-ref-not-found".to_string(),
        ]),
    };
    let index = reference_index_for_object(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retained_refs: &[],
        remote_refs: &[],
        is_complete: true,
    })?;
    let receipt = retention_receipt(RetentionReceiptBuildInput {
        decision,
        action: ACTION_UNPIN,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retention_class: &retention_class,
        requester_ref: input.requester_ref,
        index_ref: &index.index_ref,
        pin_refs: &[input.pin_ref.to_string()],
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        tombstone_ref: None,
        diagnostics: &diagnostics,
    })?;
    write_store_value(&receipt_path(input.root, &receipt.receipt_ref)?, &receipt.value)?;
    Ok(receipt)
}

pub fn reference_index_value(input: &RetentionReferenceIndexInput) -> Result<IOValue> {
    validate_reference_index_input(input)?;
    Ok(record("retention-reference-index-v1", vec![
        string(RETENTION_REFERENCE_INDEX_SCHEMA),
        object_value(&input.object_ref, &input.object_kind),
        record("pins", vec![strings_sequence(&input.pin_refs)]),
        record("retained", vec![strings_sequence(&input.retained_refs)]),
        record("tombstones", vec![strings_sequence(&input.tombstone_refs)]),
        record("remote", vec![strings_sequence(&input.remote_refs)]),
        record("proof", vec![string(if input.is_complete { "complete" } else { "incomplete" })]),
        checks_value(&[
            ("active-pins-indexed", "pass"),
            ("receipt-dependencies-indexed", "pass"),
            ("mutable-name-not-gc-proof", "pass"),
            ("remote-cache-considered", pass_or_deny(input.is_complete)),
        ]),
    ]))
}

pub fn parse_reference_index(value: &IOValue) -> Result<RetentionReferenceIndex> {
    let fields = value
        .collect_simple_record("retention-reference-index-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-reference-index-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_REFERENCE_INDEX_SCHEMA, "retention reference index schema")?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let pin_refs = record_ref_sequence(&fields[2], "pins")?;
    let retained_refs = record_ref_sequence(&fields[3], "retained")?;
    let tombstone_refs = record_ref_sequence(&fields[4], "tombstones")?;
    let remote_refs = record_ref_sequence(&fields[5], "remote")?;
    let proof = record_string(&fields[6], "proof")?;
    require_check(&parse_checks(&fields[7])?, "mutable-name-not-gc-proof", "retention reference index")?;
    Ok(RetentionReferenceIndex {
        index_ref: canonical_hash(value)?,
        object_ref,
        object_kind,
        pin_refs,
        retained_refs,
        tombstone_refs,
        remote_refs,
        is_complete: proof == "complete",
        value: value.clone(),
    })
}

pub fn reference_index_for_object(input: ReferenceIndexForObjectInput<'_>) -> Result<RetentionReferenceIndex> {
    ensure_store(input.root)?;
    let pins = pins_for_object(input.root, input.object_ref)?;
    let mut pin_refs = Vec::with_capacity(pins.len());
    for pin in &pins {
        push_bounded(&mut pin_refs, pin.pin_ref.clone(), MAX_RETENTION_REFS, "retention index pin refs")?;
    }
    let tombstone_refs = tombstone_refs_for_object(input.root, input.object_ref)?;
    let value = reference_index_value(&RetentionReferenceIndexInput {
        object_ref: input.object_ref.to_string(),
        object_kind: input.object_kind.to_string(),
        pin_refs,
        retained_refs: input.retained_refs.to_vec(),
        tombstone_refs,
        remote_refs: input.remote_refs.to_vec(),
        is_complete: input.is_complete,
    })?;
    parse_reference_index(&value)
}

pub fn evaluate_retention(input: RetentionEvaluationInput<'_>) -> Result<RetentionEvaluation> {
    ensure_store(input.root)?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention object ref")?;
    validate_name(input.object_kind, "retention object kind")?;
    require_ref(input.requester_ref, "retention requester ref")?;
    validate_refs(input.policy_refs, "retention policy ref")?;
    validate_refs(input.evidence_refs, "retention evidence ref")?;
    validate_refs(input.retained_refs, "retention retained ref")?;
    validate_refs(input.remote_refs, "retention remote ref")?;
    let index = reference_index_for_object(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        is_complete: input.is_reference_index_complete,
    })?;
    let diagnostics = retention_diagnostics(&input, &index)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut tombstone = None;
    let mut tombstone_ref = None;
    if decision == "pass" && is_destructive_action(input.action) {
        let created = retention_tombstone(RetentionTombstoneBuildInput {
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            receipt_ref: "pending",
            policy_refs: input.policy_refs,
            evidence_refs: input.evidence_refs,
        })?;
        tombstone_ref = Some(created.tombstone_ref.clone());
        tombstone = Some(created);
    }
    let receipt = retention_receipt(RetentionReceiptBuildInput {
        decision,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        requester_ref: input.requester_ref,
        index_ref: &index.index_ref,
        pin_refs: &index.pin_refs,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        tombstone_ref: tombstone_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    write_store_value(&receipt_path(input.root, &receipt.receipt_ref)?, &receipt.value)?;
    if let Some(created) = tombstone {
        write_store_value(&tombstone_path(input.root, &created.tombstone_ref)?, &created.value)?;
        return Ok(RetentionEvaluation {
            receipt,
            index,
            tombstone: Some(created),
        });
    }
    Ok(RetentionEvaluation {
        receipt,
        index,
        tombstone: None,
    })
}

pub fn retention_evidence_admission_value(input: &RetentionEvidenceAdmissionInput<'_>) -> Result<IOValue> {
    validate_evidence_admission_input(input)?;
    Ok(record("retention-evidence-admission-v1", vec![
        string(RETENTION_EVIDENCE_ADMISSION_SCHEMA),
        record("kind", vec![string(input.kind)]),
        record("decision", vec![string(input.decision)]),
        record("requester", vec![string(input.requester_ref)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("action", vec![string(input.action)]),
        record("bound", vec![strings_sequence(input.bound_refs)]),
        record("retained", vec![strings_sequence(input.retained_refs)]),
        record("remote", vec![strings_sequence(input.remote_refs)]),
        record("reference-index-complete", vec![string(pass_or_deny(input.is_reference_index_complete))]),
        record("current", vec![string(pass_or_deny(input.is_current))]),
        record("revoked", vec![strings_sequence(input.revoked_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("scope-bound", "pass"),
            ("typed-admission", "pass"),
            ("non-authority-evidence-separated", "pass"),
        ]),
    ]))
}

pub fn parse_retention_evidence_admission(value: &IOValue) -> Result<RetentionEvidenceAdmission> {
    let fields = value
        .collect_simple_record("retention-evidence-admission-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-evidence-admission-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_EVIDENCE_ADMISSION_SCHEMA, "retention evidence admission schema")?;
    let kind = record_string(&fields[1], "kind")?;
    validate_admission_kind(&kind)?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    let requester_ref = record_ref(&fields[3], "requester")?;
    let (object_ref, object_kind) = parse_object_value(&fields[4])?;
    let retention_class = record_string(&fields[5], "class")?;
    validate_retention_class(&retention_class)?;
    let action = record_string(&fields[6], "action")?;
    validate_action(&action)?;
    let bound_refs = record_ref_sequence(&fields[7], "bound")?;
    let retained_refs = record_ref_sequence(&fields[8], "retained")?;
    let remote_refs = record_ref_sequence(&fields[9], "remote")?;
    let is_reference_index_complete = record_pass_bool(&fields[10], "reference-index-complete")?;
    let is_current = record_pass_bool(&fields[11], "current")?;
    let revoked_refs = record_ref_sequence(&fields[12], "revoked")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    require_check(&parse_checks(&fields[14])?, "typed-admission", "retention evidence admission")?;
    Ok(RetentionEvidenceAdmission {
        admission_ref: canonical_hash(value)?,
        kind,
        decision,
        requester_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        bound_refs,
        retained_refs,
        remote_refs,
        is_reference_index_complete,
        is_current,
        revoked_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_retention_evidence_admission(
    root: &Path,
    input: &RetentionEvidenceAdmissionInput<'_>,
) -> Result<RetentionEvidenceAdmission> {
    ensure_store(root)?;
    let value = retention_evidence_admission_value(input)?;
    let admission = parse_retention_evidence_admission(&value)?;
    write_store_value(&admission_path(root, &admission.admission_ref)?, &admission.value)?;
    Ok(admission)
}

pub fn retention_remote_gc_clearance_value(input: &RetentionRemoteGcClearanceInput<'_>) -> Result<IOValue> {
    validate_remote_gc_clearance_input(input)?;
    Ok(record("retention-remote-gc-clearance-v1", vec![
        string(RETENTION_REMOTE_GC_CLEARANCE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("requester", vec![string(input.requester_ref)]),
        record("peer", vec![string(input.peer_ref)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("action", vec![string(input.action)]),
        record("remote", vec![string(input.remote_ref)]),
        record("policy", vec![string(input.policy_ref)]),
        record("authority", vec![string(input.authority_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        record("retained", vec![strings_sequence(input.retained_refs)]),
        record("current", vec![string(pass_or_deny(input.is_current))]),
        record("revoked", vec![strings_sequence(input.revoked_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("peer-bound", "pass"),
            ("scope-bound", "pass"),
            ("remote-ref-bound", "pass"),
            ("non-authority-evidence-separated", "pass"),
        ]),
    ]))
}

pub fn parse_retention_remote_gc_clearance(value: &IOValue) -> Result<RetentionRemoteGcClearance> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-v1", Some(16))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_REMOTE_GC_CLEARANCE_SCHEMA, "retention remote GC clearance schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let requester_ref = record_ref(&fields[2], "requester")?;
    let peer_ref = record_ref(&fields[3], "peer")?;
    let (object_ref, object_kind) = parse_object_value(&fields[4])?;
    let retention_class = record_string(&fields[5], "class")?;
    validate_retention_class(&retention_class)?;
    let action = record_string(&fields[6], "action")?;
    validate_action(&action)?;
    let remote_ref = record_ref(&fields[7], "remote")?;
    let policy_ref = record_ref(&fields[8], "policy")?;
    let authority_ref = record_ref(&fields[9], "authority")?;
    let evidence_refs = record_ref_sequence(&fields[10], "evidence")?;
    let retained_refs = record_ref_sequence(&fields[11], "retained")?;
    let is_current = record_pass_bool(&fields[12], "current")?;
    let revoked_refs = record_ref_sequence(&fields[13], "revoked")?;
    let diagnostics = record_string_sequence(&fields[14], "diagnostics")?;
    require_check(&parse_checks(&fields[15])?, "peer-bound", "retention remote GC clearance")?;
    Ok(RetentionRemoteGcClearance {
        clearance_ref: canonical_hash(value)?,
        decision,
        requester_ref,
        peer_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        remote_ref,
        policy_ref,
        authority_ref,
        evidence_refs,
        retained_refs,
        is_current,
        revoked_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_retention_remote_gc_clearance(
    root: &Path,
    input: &RetentionRemoteGcClearanceInput<'_>,
) -> Result<RetentionRemoteGcClearance> {
    ensure_store(root)?;
    let value = retention_remote_gc_clearance_value(input)?;
    let clearance = parse_retention_remote_gc_clearance(&value)?;
    write_store_value(&remote_clearance_path(root, &clearance.clearance_ref)?, &clearance.value)?;
    Ok(clearance)
}

pub fn retention_remote_gc_clearance_request_value(
    input: &RetentionRemoteGcClearanceRequestInput<'_>,
) -> Result<IOValue> {
    validate_remote_gc_clearance_request_input(input)?;
    Ok(record("retention-remote-gc-clearance-request-v1", vec![
        string(RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA),
        record("requester", vec![string(input.requester_ref)]),
        record("peer", vec![string(input.peer_ref)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("action", vec![string(input.action)]),
        record("remote", vec![string(input.remote_ref)]),
        record("policy", vec![string(input.policy_ref)]),
        record("authority", vec![string(input.authority_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[("request-scope-bound", "pass"), ("peer-bound", "pass")]),
    ]))
}

pub fn parse_retention_remote_gc_clearance_request(value: &IOValue) -> Result<RetentionRemoteGcClearanceRequest> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-request-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-request-v1 ...>"))?;
    require_schema(
        &fields[0],
        RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA,
        "retention remote clearance request schema",
    )?;
    require_check(&parse_checks(&fields[10])?, "request-scope-bound", "retention remote clearance request")?;
    let (object_ref, object_kind) = parse_object_value(&fields[3])?;
    let request = RetentionRemoteGcClearanceRequest {
        request_ref: canonical_hash(value)?,
        requester_ref: record_ref(&fields[1], "requester")?,
        peer_ref: record_ref(&fields[2], "peer")?,
        object_ref,
        object_kind,
        retention_class: record_string(&fields[4], "class")?,
        action: record_string(&fields[5], "action")?,
        remote_ref: record_ref(&fields[6], "remote")?,
        policy_ref: record_ref(&fields[7], "policy")?,
        authority_ref: record_ref(&fields[8], "authority")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    };
    validate_remote_gc_clearance_request(&request)?;
    Ok(request)
}

pub fn store_retention_remote_gc_clearance_request(
    root: &Path,
    input: &RetentionRemoteGcClearanceRequestInput<'_>,
) -> Result<RetentionRemoteGcClearanceRequest> {
    ensure_store(root)?;
    let value = retention_remote_gc_clearance_request_value(input)?;
    let request = parse_retention_remote_gc_clearance_request(&value)?;
    write_store_value(&remote_clearance_request_path(root, &request.request_ref)?, &request.value)?;
    Ok(request)
}

pub fn store_retention_remote_gc_clearance_response(
    input: RetentionRemoteGcClearanceResponseInput<'_>,
) -> Result<RetentionRemoteGcClearanceResponse> {
    ensure_store(input.root)?;
    let request = parse_retention_remote_gc_clearance_request(input.request_value)?;
    let diagnostics = remote_clearance_response_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut clearance_evidence_refs = request.evidence_refs.clone();
    for reference in input.evidence_refs {
        push_bounded(
            &mut clearance_evidence_refs,
            reference.clone(),
            MAX_RETENTION_REFS,
            "retention remote clearance response evidence refs",
        )?;
    }
    let clearance_value = retention_remote_gc_clearance_value(&RetentionRemoteGcClearanceInput {
        decision,
        requester_ref: &request.requester_ref,
        peer_ref: &request.peer_ref,
        object_ref: &request.object_ref,
        object_kind: &request.object_kind,
        retention_class: &request.retention_class,
        action: &request.action,
        remote_ref: &request.remote_ref,
        policy_ref: &request.policy_ref,
        authority_ref: &request.authority_ref,
        evidence_refs: &clearance_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: &diagnostics,
    })?;
    let clearance = parse_retention_remote_gc_clearance(&clearance_value)?;
    let value = retention_remote_gc_clearance_response_value(&request, &clearance, decision, &diagnostics)?;
    let response = parse_retention_remote_gc_clearance_response(&value)?;
    write_store_value(&remote_clearance_response_path(input.root, &response.response_ref)?, &response.value)?;
    Ok(response)
}

pub fn retention_remote_gc_clearance_response_value(
    request: &RetentionRemoteGcClearanceRequest,
    clearance: &RetentionRemoteGcClearance,
    decision: &str,
    diagnostics: &[String],
) -> Result<IOValue> {
    validate_decision(decision)?;
    ensure_count_at_most(
        diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance response diagnostics",
    )?;
    validate_remote_gc_clearance_workflow_scope(request, clearance)?;
    Ok(record("retention-remote-gc-clearance-response-v1", vec![
        string(RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA),
        record("request", vec![string(&request.request_ref), request.value.clone()]),
        record("decision", vec![string(decision)]),
        record("clearance", vec![string(&clearance.clearance_ref), clearance.value.clone()]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("request-ref-verified", "pass"),
            ("clearance-ref-verified", "pass"),
            ("clearance-scope-bound", pass_or_deny(decision == clearance.decision)),
        ]),
    ]))
}

pub fn parse_retention_remote_gc_clearance_response(value: &IOValue) -> Result<RetentionRemoteGcClearanceResponse> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-response-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-response-v1 ...>"))?;
    require_schema(
        &fields[0],
        RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA,
        "retention remote clearance response schema",
    )?;
    let request = parse_embedded_remote_clearance_request(&fields[1])?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    let clearance = parse_embedded_remote_clearance(&fields[3])?;
    let diagnostics = record_string_sequence(&fields[4], "diagnostics")?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "request-ref-verified", "retention remote clearance response")?;
    require_check(&checks, "clearance-ref-verified", "retention remote clearance response")?;
    if decision != clearance.decision {
        return Err(MoltenError::invalid_harness("remote clearance response decision does not match clearance"));
    }
    validate_remote_gc_clearance_workflow_scope(&request, &clearance)?;
    Ok(RetentionRemoteGcClearanceResponse {
        response_ref: canonical_hash(value)?,
        decision,
        request_ref: request.request_ref.clone(),
        request,
        clearance_ref: clearance.clearance_ref.clone(),
        clearance,
        diagnostics,
        value: value.clone(),
    })
}

pub fn import_retention_remote_gc_clearance_response(
    input: RetentionRemoteGcClearanceImportInput<'_>,
) -> Result<RetentionRemoteGcClearanceImport> {
    ensure_store(input.root)?;
    if let Some(peer_ref) = input.expected_peer_ref {
        require_ref(peer_ref, "retention remote clearance import expected peer ref")?;
    }
    if let Some(remote_ref) = input.expected_remote_ref {
        require_ref(remote_ref, "retention remote clearance import expected remote ref")?;
    }
    let request = parse_retention_remote_gc_clearance_request(input.request_value)?;
    let response = match parse_retention_remote_gc_clearance_response(input.response_value) {
        Ok(response) => response,
        Err(error) => {
            let diagnostics = vec![format!("remote-clearance-tampered-response:{error}")];
            let response_ref = canonical_hash(input.response_value)?;
            let value = retention_remote_gc_clearance_import_value(&RetentionRemoteGcClearanceImportValueInput {
                decision: "deny",
                request_ref: &request.request_ref,
                response_ref: &response_ref,
                clearance_ref: None,
                peer_ref: &request.peer_ref,
                remote_ref: &request.remote_ref,
                diagnostics: &diagnostics,
            })?;
            let import = parse_retention_remote_gc_clearance_import(&value)?;
            write_store_value(&remote_clearance_import_path(input.root, &import.import_ref)?, &import.value)?;
            return Ok(import);
        }
    };
    let mut diagnostics = Vec::new();
    push_remote_clearance_import_diagnostics(&mut diagnostics, &request, &response, input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let clearance_ref = if decision == "pass" {
        write_store_value(
            &remote_clearance_path(input.root, &response.clearance.clearance_ref)?,
            &response.clearance.value,
        )?;
        Some(response.clearance.clearance_ref.clone())
    } else {
        None
    };
    let value = retention_remote_gc_clearance_import_value(&RetentionRemoteGcClearanceImportValueInput {
        decision,
        request_ref: &request.request_ref,
        response_ref: &response.response_ref,
        clearance_ref: clearance_ref.as_deref(),
        peer_ref: &request.peer_ref,
        remote_ref: &request.remote_ref,
        diagnostics: &diagnostics,
    })?;
    let import = parse_retention_remote_gc_clearance_import(&value)?;
    write_store_value(&remote_clearance_import_path(input.root, &import.import_ref)?, &import.value)?;
    Ok(import)
}

pub fn retention_remote_gc_clearance_import_value(
    input: &RetentionRemoteGcClearanceImportValueInput<'_>,
) -> Result<IOValue> {
    validate_decision(input.decision)?;
    require_ref(input.request_ref, "retention remote clearance import request ref")?;
    require_ref(input.response_ref, "retention remote clearance import response ref")?;
    if let Some(reference) = input.clearance_ref {
        require_ref(reference, "retention remote clearance import clearance ref")?;
    }
    require_ref(input.peer_ref, "retention remote clearance import peer ref")?;
    require_ref(input.remote_ref, "retention remote clearance import remote ref")?;
    ensure_count_at_most(
        input.diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance import diagnostics",
    )?;
    Ok(record("retention-remote-gc-clearance-import-v1", vec![
        string(RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("request", vec![string(input.request_ref)]),
        record("response", vec![string(input.response_ref)]),
        record("clearance", vec![optional_ref_value(input.clearance_ref)]),
        record("peer", vec![string(input.peer_ref)]),
        record("remote", vec![string(input.remote_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("evidence-only", "pass"),
            ("local-clearance-stored", pass_or_deny(input.clearance_ref.is_some())),
        ]),
    ]))
}

pub fn parse_retention_remote_gc_clearance_import(value: &IOValue) -> Result<RetentionRemoteGcClearanceImport> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-import-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-import-v1 ...>"))?;
    require_schema(
        &fields[0],
        RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA,
        "retention remote clearance import schema",
    )?;
    require_check(&parse_checks(&fields[8])?, "evidence-only", "retention remote clearance import")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let request_ref = record_ref(&fields[2], "request")?;
    let response_ref = record_ref(&fields[3], "response")?;
    let clearance_ref = record_optional_ref(&fields[4], "clearance")?;
    let peer_ref = record_ref(&fields[5], "peer")?;
    let remote_ref = record_ref(&fields[6], "remote")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    Ok(RetentionRemoteGcClearanceImport {
        import_ref: canonical_hash(value)?,
        decision,
        request_ref,
        response_ref,
        clearance_ref,
        peer_ref,
        remote_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub async fn run_retention_remote_gc_clearance_live_loopback(
    input: RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
) -> Result<RetentionRemoteGcClearanceLiveLoopback> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_loopback_input(&input)?;
    let request = store_retention_remote_gc_clearance_request(input.root, &RetentionRemoteGcClearanceRequestInput {
        requester_ref: input.requester_ref,
        peer_ref: input.peer_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        remote_ref: input.remote_ref,
        policy_ref: input.policy_ref,
        authority_ref: input.authority_ref,
        evidence_refs: input.retention_evidence_refs,
    })?;
    let request_control_evidence = request_evidence(&input, &request.request_ref)?;
    let (request_control_ref, request_control_value) =
        request_control(&input, &request.request_ref, &request_control_evidence)?;
    let request_live = request_leg(&input, &request_control_value, &request_control_evidence).await?;

    let response = store_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceResponseInput {
        root: input.root,
        request_value: &request.value,
        evidence_refs: input.response_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: input.response_diagnostics,
    })?;
    let response_control_evidence = response_evidence(&input, &request.request_ref, &response.response_ref)?;
    let (response_control_ref, response_control_value) =
        response_control(&input, &request.request_ref, &response.response_ref, &response_control_evidence)?;
    let response_live = response_leg(&input, &response_control_value, &response_control_evidence).await?;

    let import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
        root: input.root,
        request_value: &request.value,
        response_value: &response.value,
        expected_peer_ref: Some(input.peer_ref),
        expected_remote_ref: Some(input.remote_ref),
    })?;
    let transport_diagnostics = transport_notes(&request_live, &response_live)?;
    let workflow_value = loopback_value(&LoopbackValueInput {
        request_value: &request.value,
        response_value: &response.value,
        import_value: &import.value,
        request_control_ref: &request_control_ref,
        response_control_ref: &response_control_ref,
        request_live: &request_live,
        response_live: &response_live,
        transport_diagnostics: &transport_diagnostics,
    })?;
    let workflow = store_retention_remote_gc_clearance_live_workflow(input.root, &workflow_value)?;
    Ok(RetentionRemoteGcClearanceLiveLoopback {
        request,
        response,
        import,
        workflow,
        request_publish_receipt_value: request_live.publish_receipt_value,
        request_receive_receipt_value: request_live.receive_receipt_value,
        response_publish_receipt_value: response_live.publish_receipt_value,
        response_receive_receipt_value: response_live.receive_receipt_value,
    })
}

pub async fn send_retention_remote_gc_clearance_live_request(
    input: RetentionRemoteGcClearanceLiveRequestSendInput<'_>,
) -> Result<RetentionRemoteGcClearanceLiveRequestSend> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_request_send_input(&input)?;
    let request = store_retention_remote_gc_clearance_request(input.root, &RetentionRemoteGcClearanceRequestInput {
        requester_ref: input.requester_ref,
        peer_ref: input.peer_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        remote_ref: input.remote_ref,
        policy_ref: input.policy_ref,
        authority_ref: input.authority_ref,
        evidence_refs: input.retention_evidence_refs,
    })?;
    let control_evidence = refs_with_extra(
        input.transport_evidence_refs,
        std::slice::from_ref(&request.request_ref),
        "retention live request transport evidence ref",
    )?;
    let (control_ref, control_value) = remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: &request.request_ref,
        payload_ref: None,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
    })?;
    let send = node_daemon::send_node_control_live_ingress(&node_daemon::NodeControlLiveSendInput {
        state_root: input.requester_node_root,
        request_value: &control_value,
        receiver_ticket_value: input.peer_ticket_value,
        from_peer: input.requester_node_id,
        sequence: input.sequence,
        expected_operation_ref: None,
        expected_receiver_node: Some(input.peer_node_id),
        expected_topic: Some(input.topic),
        expected_endpoint: None,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
        join_timeout_ms: input.join_timeout_ms,
    })
    .await?;
    Ok(RetentionRemoteGcClearanceLiveRequestSend {
        request,
        control_ref,
        control_value,
        send,
    })
}

pub async fn send_retention_remote_gc_clearance_live_response(
    input: RetentionRemoteGcClearanceLiveResponseSendInput<'_>,
) -> Result<RetentionRemoteGcClearanceLiveResponseSend> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_response_send_input(&input)?;
    let request = parse_retention_remote_gc_clearance_request(input.request_value)?;
    let response = store_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceResponseInput {
        root: input.root,
        request_value: input.request_value,
        evidence_refs: input.response_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: input.response_diagnostics,
    })?;
    let control_evidence = refs_with_extra(
        input.transport_evidence_refs,
        &[request.request_ref.clone(), response.response_ref.clone()],
        "retention live response transport evidence ref",
    )?;
    let (control_ref, control_value) = remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: &response.response_ref,
        payload_ref: Some(&request.request_ref),
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
    })?;
    let send = node_daemon::send_node_control_live_ingress(&node_daemon::NodeControlLiveSendInput {
        state_root: input.peer_node_root,
        request_value: &control_value,
        receiver_ticket_value: input.requester_ticket_value,
        from_peer: input.peer_node_id,
        sequence: input.sequence,
        expected_operation_ref: None,
        expected_receiver_node: Some(input.requester_node_id),
        expected_topic: Some(input.topic),
        expected_endpoint: None,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
        join_timeout_ms: input.join_timeout_ms,
    })
    .await?;
    Ok(RetentionRemoteGcClearanceLiveResponseSend {
        response,
        control_ref,
        control_value,
        send,
    })
}

pub fn import_retention_remote_gc_clearance_live_workflow(
    input: RetentionRemoteGcClearanceLiveImportWorkflowInput<'_>,
) -> Result<RetentionRemoteGcClearanceLiveImportWorkflow> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_import_workflow_input(&input)?;
    let request = parse_retention_remote_gc_clearance_request(input.request_value)?;
    let response_ref = canonical_hash(input.response_value)?;
    let import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
        root: input.root,
        request_value: input.request_value,
        response_value: input.response_value,
        expected_peer_ref: input.expected_peer_ref,
        expected_remote_ref: input.expected_remote_ref,
    })?;
    let request_control = node_runtime::parse_node_control_request(input.request_control_value)?;
    let response_control = node_runtime::parse_node_control_request(input.response_control_value)?;
    let request_control_ref = canonical_hash(input.request_control_value)?;
    let response_control_ref = canonical_hash(input.response_control_value)?;
    let request_send = node_daemon::parse_node_control_live_send_receipt(input.request_send_receipt_value)?;
    let response_send = node_daemon::parse_node_control_live_send_receipt(input.response_send_receipt_value)?;
    let request_receive = parse_node_live_transport_receipt(input.request_receive_receipt_value)?;
    let response_receive = parse_node_live_transport_receipt(input.response_receive_receipt_value)?;
    let diagnostics = live_import_diagnostics(LiveImportDiagnosticsInput {
        request: &request,
        response_ref: &response_ref,
        request_control: &request_control,
        response_control: &response_control,
        request_send: &request_send,
        response_send: &response_send,
        request_receive: &request_receive,
        response_receive: &response_receive,
        request_ingress_ref: input.request_ingress_ref,
        response_ingress_ref: input.response_ingress_ref,
    })?;
    let request_publish_ref = live_send_publish_ref(&request_send);
    let response_publish_ref = live_send_publish_ref(&response_send);
    let request_receive_ref = request_receive.receipt_ref.clone();
    let response_receive_ref = response_receive.receipt_ref.clone();
    let workflow_value =
        retention_remote_gc_clearance_live_workflow_value(&RetentionRemoteGcClearanceLiveWorkflowValueInput {
            request_value: input.request_value,
            response_value: input.response_value,
            import_value: &import.value,
            request_control_ref: &request_control_ref,
            request_publish_ref: &request_publish_ref,
            request_receive_ref: &request_receive_ref,
            request_ingress_ref: input.request_ingress_ref,
            response_control_ref: &response_control_ref,
            response_publish_ref: &response_publish_ref,
            response_receive_ref: &response_receive_ref,
            response_ingress_ref: input.response_ingress_ref,
            transport_diagnostics: &diagnostics,
        })?;
    let workflow = store_retention_remote_gc_clearance_live_workflow(input.root, &workflow_value)?;
    Ok(RetentionRemoteGcClearanceLiveImportWorkflow {
        import,
        workflow,
        request_send_receipt_ref: request_send.receipt_ref,
        response_send_receipt_ref: response_send.receipt_ref,
    })
}

struct LiveImportDiagnosticsInput<'a> {
    request: &'a RetentionRemoteGcClearanceRequest,
    response_ref: &'a str,
    request_control: &'a node_runtime::NodeControlRequest,
    response_control: &'a node_runtime::NodeControlRequest,
    request_send: &'a node_daemon::NodeControlLiveSendReceipt,
    response_send: &'a node_daemon::NodeControlLiveSendReceipt,
    request_receive: &'a NodeLiveTransportReceipt,
    response_receive: &'a NodeLiveTransportReceipt,
    request_ingress_ref: &'a str,
    response_ingress_ref: &'a str,
}

fn live_import_diagnostics(input: LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = live_import_request_diagnostics(&input)?;
    extend_bounded(
        &mut diagnostics,
        live_import_response_diagnostics(&input)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn live_import_request_diagnostics(input: &LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_control_diagnostics("request-control", input.request_control, &input.request.request_ref, None),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_send_diagnostics("request-send", input.request_send),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics_from("request-receive", input.request_receive)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_receive_binding_diagnostics(
            "request-receive",
            input.request_send,
            input.request_receive,
            input.request_ingress_ref,
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn live_import_response_diagnostics(input: &LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_control_diagnostics(
            "response-control",
            input.response_control,
            input.response_ref,
            Some(&input.request.request_ref),
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_send_diagnostics("response-send", input.response_send),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics_from("response-receive", input.response_receive)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_receive_binding_diagnostics(
            "response-receive",
            input.response_send,
            input.response_receive,
            input.response_ingress_ref,
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

pub fn retention_remote_gc_clearance_live_workflow_value(
    input: &RetentionRemoteGcClearanceLiveWorkflowValueInput<'_>,
) -> Result<IOValue> {
    validate_remote_gc_clearance_live_workflow_value_input(input)?;
    let parts = flow_parts(input)?;
    let refs = flow_refs(input);
    let decision = if parts.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("retention-remote-gc-clearance-live-workflow-v1", vec![
        string(RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA),
        record("decision", vec![string(decision)]),
        record("request", vec![string(&parts.request.request_ref), input.request_value.clone()]),
        record("response", vec![string(&parts.response_ref), input.response_value.clone()]),
        record("import", vec![string(&parts.import.import_ref), input.import_value.clone()]),
        record("request-live", vec![strings_sequence(&refs.request)]),
        record("response-live", vec![strings_sequence(&refs.response)]),
        record("scope", vec![
            record("requester", vec![string(&parts.request.requester_ref)]),
            record("peer", vec![string(&parts.request.peer_ref)]),
            record("remote", vec![string(&parts.request.remote_ref)]),
            object_value(&parts.request.object_ref, &parts.request.object_kind),
            record("class", vec![string(&parts.request.retention_class)]),
            record("action", vec![string(&parts.request.action)]),
        ]),
        record("diagnostics", vec![sequence(parts.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            (
                "request-response-bound",
                pass_or_deny(
                    parts.response.as_ref().is_some_and(|value| value.request_ref == parts.request.request_ref),
                ),
            ),
            ("live-transport-bound", pass_or_deny(input.transport_diagnostics.is_empty())),
            ("import-gate", pass_or_deny(parts.import.decision == "pass")),
            ("transport-is-not-authority", "pass"),
            ("live-receipt-is-not-clearance", "pass"),
            ("authority-policy-still-required", "pass"),
            ("remote-gc-still-required", "pass"),
        ]),
    ]))
}

struct FlowParts {
    request: RetentionRemoteGcClearanceRequest,
    response_ref: String,
    response: Option<RetentionRemoteGcClearanceResponse>,
    import: RetentionRemoteGcClearanceImport,
    diagnostics: Vec<String>,
}

struct FlowRefs {
    request: Vec<String>,
    response: Vec<String>,
}

struct FlowDiagnosticsInput<'a> {
    request: &'a RetentionRemoteGcClearanceRequest,
    response: Option<&'a RetentionRemoteGcClearanceResponse>,
    response_ref: &'a str,
    import: &'a RetentionRemoteGcClearanceImport,
    parse_diagnostic: Option<String>,
    transport_diagnostics: &'a [String],
}

fn flow_parts(input: &RetentionRemoteGcClearanceLiveWorkflowValueInput<'_>) -> Result<FlowParts> {
    let request = parse_retention_remote_gc_clearance_request(input.request_value)?;
    let response_ref = canonical_hash(input.response_value)?;
    let (response, parse_diagnostic) = match parse_retention_remote_gc_clearance_response(input.response_value) {
        Ok(response) => (Some(response), None),
        Err(error) => (None, Some(format!("remote-clearance-live-tampered-response:{error}"))),
    };
    let import = parse_retention_remote_gc_clearance_import(input.import_value)?;
    let diagnostics = flow_diagnostics(FlowDiagnosticsInput {
        request: &request,
        response: response.as_ref(),
        response_ref: &response_ref,
        import: &import,
        parse_diagnostic,
        transport_diagnostics: input.transport_diagnostics,
    })?;
    Ok(FlowParts {
        request,
        response_ref,
        response,
        import,
        diagnostics,
    })
}

fn flow_diagnostics(input: FlowDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        input.transport_diagnostics.to_vec(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    if let Some(diagnostic) = input.parse_diagnostic {
        push_bounded(&mut diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention live workflow diagnostics")?;
    }
    extend_bounded(
        &mut diagnostics,
        response_notes(input.request, input.response)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        import_notes(input.request, input.response_ref, input.import)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        input.import.diagnostics.clone(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn response_notes(
    request: &RetentionRemoteGcClearanceRequest,
    response: Option<&RetentionRemoteGcClearanceResponse>,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    if let Some(response) = response {
        if response.request_ref != request.request_ref {
            push_bounded(
                &mut notes,
                "remote-clearance-live-wrong-request".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention live workflow diagnostics",
            )?;
        }
        if response.decision != "pass" {
            push_bounded(
                &mut notes,
                "remote-clearance-live-response-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention live workflow diagnostics",
            )?;
        }
    }
    Ok(notes)
}

fn import_notes(
    request: &RetentionRemoteGcClearanceRequest,
    response_ref: &str,
    import: &RetentionRemoteGcClearanceImport,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    if import.request_ref != request.request_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-request".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.response_ref != response_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-response".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.peer_ref != request.peer_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-peer".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.remote_ref != request.remote_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-remote".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.decision != "pass" {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-deny".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    Ok(notes)
}

fn flow_refs(input: &RetentionRemoteGcClearanceLiveWorkflowValueInput<'_>) -> FlowRefs {
    FlowRefs {
        request: vec![
            input.request_control_ref.to_string(),
            input.request_publish_ref.to_string(),
            input.request_receive_ref.to_string(),
            input.request_ingress_ref.to_string(),
        ],
        response: vec![
            input.response_control_ref.to_string(),
            input.response_publish_ref.to_string(),
            input.response_receive_ref.to_string(),
            input.response_ingress_ref.to_string(),
        ],
    }
}

pub fn parse_retention_remote_gc_clearance_live_workflow(
    value: &IOValue,
) -> Result<RetentionRemoteGcClearanceLiveWorkflow> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-live-workflow-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-live-workflow-v1 ...>"))?;
    require_schema(
        &fields[0],
        RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA,
        "retention remote clearance live workflow schema",
    )?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "transport-is-not-authority", "retention remote clearance live workflow")?;
    require_check(&checks, "live-receipt-is-not-clearance", "retention remote clearance live workflow")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let request = parse_embedded_remote_clearance_request(&fields[2])?;
    let (response_ref, response_value) = parse_embedded_value(&fields[3], "response")?;
    let import = parse_embedded_remote_clearance_import(&fields[4])?;
    let request_live_refs = record_ref_sequence(&fields[5], "request-live")?;
    let response_live_refs = record_ref_sequence(&fields[6], "response-live")?;
    let diagnostics = record_string_sequence(&fields[8], "diagnostics")?;
    if let Ok(response) = parse_retention_remote_gc_clearance_response(&response_value)
        && decision == "pass"
        && response.request_ref != request.request_ref
    {
        return Err(MoltenError::invalid_harness(
            "retention remote clearance live workflow pass response request mismatch",
        ));
    }
    if decision == "pass" && (import.request_ref != request.request_ref || import.response_ref != response_ref) {
        return Err(MoltenError::invalid_harness(
            "retention remote clearance live workflow pass import binding mismatch",
        ));
    }
    Ok(RetentionRemoteGcClearanceLiveWorkflow {
        workflow_ref: canonical_hash(value)?,
        decision,
        request_ref: request.request_ref,
        response_ref,
        import_ref: import.import_ref,
        clearance_ref: import.clearance_ref,
        peer_ref: request.peer_ref,
        remote_ref: request.remote_ref,
        request_live_refs,
        response_live_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_retention_remote_gc_clearance_live_workflow(
    root: &Path,
    value: &IOValue,
) -> Result<RetentionRemoteGcClearanceLiveWorkflow> {
    ensure_store(root)?;
    let workflow = parse_retention_remote_gc_clearance_live_workflow(value)?;
    write_store_value(&remote_clearance_live_workflow_path(root, &workflow.workflow_ref)?, &workflow.value)?;
    Ok(workflow)
}

struct AdmissionScope<'a> {
    requester_ref: Option<&'a str>,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct AdmissionRefsInput<'a> {
    root: &'a Path,
    refs: &'a [String],
    expected_kind: &'a str,
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
}

struct AdmissionRefsResult {
    diagnostics: Vec<String>,
    admitted_refs: Vec<String>,
    remote_refs: Vec<String>,
}

struct RemoteClearanceRefsInput<'a> {
    root: &'a Path,
    refs: &'a [String],
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
    required_peer_refs: &'a [String],
    policy_refs: &'a [String],
    authority_refs: &'a [String],
}

struct RemoteClearanceRefsResult {
    diagnostics: Vec<String>,
    admitted_refs: Vec<String>,
    remote_refs: Vec<String>,
    peer_refs: Vec<String>,
}

pub struct RetentionGcPlanValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: Option<&'a str>,
    index: &'a RetentionReferenceIndex,
    evidence_value: &'a IOValue,
    gates: &'a [RetentionPlanGate],
    diagnostics: &'a [String],
}

struct RetentionGcApplyValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: Option<&'a str>,
    plan_ref: &'a str,
    recomputed_plan_ref: &'a str,
    retention_receipt_ref: Option<&'a str>,
    tombstone_ref: Option<&'a str>,
    admission_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RetentionGcExecutionGateValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    apply_ref: Option<&'a str>,
    plan_ref: Option<&'a str>,
    recomputed_plan_ref: Option<&'a str>,
    retention_receipt_ref: Option<&'a str>,
    tombstone_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

struct RetentionGcAuditValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    plan_ref: Option<&'a str>,
    plan_decision: &'a str,
    apply_ref: Option<&'a str>,
    apply_decision: &'a str,
    execution_ref: &'a str,
    execution_decision: &'a str,
    retention_receipt_ref: Option<&'a str>,
    retention_receipt_decision: &'a str,
    tombstone_ref: Option<&'a str>,
    tombstone_status: &'a str,
    diagnostics: &'a [String],
}

struct RetentionCandidateExplainValueInput<'a> {
    object_ref: &'a str,
    object_kind: Option<&'a str>,
    retention_class: Option<&'a str>,
    action: Option<&'a str>,
    subsystem: Option<&'a str>,
    pin_refs: &'a [String],
    admission_refs: &'a [String],
    remote_clearance_refs: &'a [String],
    remote_clearance_import_refs: &'a [String],
    gc_plan_refs: &'a [String],
    gc_apply_refs: &'a [String],
    gc_execution_refs: &'a [String],
    gc_audit_refs: &'a [String],
    retention_receipt_refs: &'a [String],
    tombstone_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RetentionCandidateFilter<'a> {
    object_ref: &'a str,
    object_kind: Option<&'a str>,
    retention_class: Option<&'a str>,
    action: Option<&'a str>,
    subsystem: Option<&'a str>,
}

struct RetentionCandidateBundleValueInput<'a> {
    explain: &'a RetentionCandidateExplain,
    artifact_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RetentionCandidateBundleProfileValueInput<'a> {
    profile: RetentionCandidateBundleExportProfile,
    decision: &'a str,
    bundle_ref: &'a str,
    marker_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RetentionBundleArtifactGroupInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    dir_name: &'a str,
    refs: &'a [String],
    read: fn(&Path, &str) -> Result<IOValue>,
}

struct RetentionCandidateBundleVerifyValueInput<'a> {
    bundle: &'a RetentionCandidateBundle,
    decision: &'a str,
    file_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RetentionBundleVerifyGroupInput<'a> {
    bundle_dir: &'a Path,
    dir_name: &'a str,
    refs: &'a [String],
    parse: fn(&IOValue) -> Result<()>,
}

struct Group<'a> {
    dir_name: &'a str,
    refs: &'a [String],
    parse: fn(&IOValue) -> Result<()>,
}

struct RetentionBundleArtifactGroupScanInput<'a> {
    group_dir: &'a Path,
    dir_name: &'a str,
    expected_refs: &'a BTreeSet<String>,
}

struct RetentionAuditScope<'a> {
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

struct GcAuditScope<'a> {
    subsystem: &'a str,
    retention: RetentionAuditScope<'a>,
}

struct GateAdmissions {
    policy: AdmissionRefsResult,
    authority: AdmissionRefsResult,
    supporting: AdmissionRefsResult,
    reference_index: AdmissionRefsResult,
    remote_gc: AdmissionRefsResult,
}

struct RetentionGateInputs<'a> {
    input: &'a RetentionGcPlanInput<'a>,
    policy: AdmissionRefsResult,
    authority: AdmissionRefsResult,
    supporting: AdmissionRefsResult,
    reference_index: AdmissionRefsResult,
    remote_gc: AdmissionRefsResult,
    remote_clearance: RemoteClearanceRefsResult,
    has_delete_authority: bool,
    has_remote_gc_clearance: bool,
}

struct PlanGateBuildInput<'a> {
    name: &'a str,
    is_required: bool,
    required_refs: &'a [String],
    admitted_refs: &'a [String],
    diagnostics: Vec<String>,
}

struct LocalRetentionGateInput<'a> {
    input: &'a RetentionGcPlanInput<'a>,
    index: &'a RetentionReferenceIndex,
    has_delete_authority: bool,
    has_remote_gc_clearance: bool,
}

struct MissingDiagnosticInput<'a> {
    diagnostics: &'a [String],
    is_missing: bool,
    missing_diagnostic: &'a str,
}

struct RemoteClearanceGateInput<'a> {
    diagnostics: &'a [String],
    has_missing_refs: bool,
    has_missing_peers: bool,
}

struct LiveControlRequestInput<'a> {
    target_ref: &'a str,
    payload_ref: Option<&'a str>,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn remote_clearance_live_control_request_value(input: &LiveControlRequestInput<'_>) -> Result<(String, IOValue)> {
    let value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
        operation: "gate",
        target_ref: Some(input.target_ref),
        payload_ref: input.payload_ref,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let reference = canonical_hash(&value)?;
    Ok((reference, value))
}

struct LoopbackValueInput<'a> {
    request_value: &'a IOValue,
    response_value: &'a IOValue,
    import_value: &'a IOValue,
    request_control_ref: &'a str,
    response_control_ref: &'a str,
    request_live: &'a node_daemon::NodeControlLiveLoopback,
    response_live: &'a node_daemon::NodeControlLiveLoopback,
    transport_diagnostics: &'a [String],
}

fn request_evidence(input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>, request_ref: &str) -> Result<Vec<String>> {
    let extra_refs = [request_ref.to_string()];
    refs_with_extra(input.request_transport_evidence_refs, &extra_refs, "retention live request transport evidence ref")
}

fn response_evidence(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    response_ref: &str,
) -> Result<Vec<String>> {
    let extra_refs = [request_ref.to_string(), response_ref.to_string()];
    refs_with_extra(
        input.response_transport_evidence_refs,
        &extra_refs,
        "retention live response transport evidence ref",
    )
}

fn request_control(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    evidence_refs: &[String],
) -> Result<(String, IOValue)> {
    remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: request_ref,
        payload_ref: None,
        authority_refs: input.request_authority_refs,
        policy_refs: input.request_policy_refs,
        resource_refs: input.request_resource_refs,
        evidence_refs,
    })
}

fn response_control(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    response_ref: &str,
    evidence_refs: &[String],
) -> Result<(String, IOValue)> {
    remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: response_ref,
        payload_ref: Some(request_ref),
        authority_refs: input.response_authority_refs,
        policy_refs: input.response_policy_refs,
        resource_refs: input.response_resource_refs,
        evidence_refs,
    })
}

async fn request_leg(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
    control_value: &IOValue,
    evidence_refs: &[String],
) -> Result<node_daemon::NodeControlLiveLoopback> {
    node_daemon::node_control_live_iroh_loopback(&node_daemon::NodeControlLiveLoopbackInput {
        state_root: input.peer_node_root,
        request_value: control_value,
        from_peer: input.requester_node_id,
        to_node: input.peer_node_id,
        topic: input.topic,
        sequence: input.request_sequence,
        peer_bootstrap_refs: input.request_peer_bootstrap_refs,
        authority_refs: input.request_authority_refs,
        policy_refs: input.request_policy_refs,
        resource_refs: input.request_resource_refs,
        evidence_refs,
    })
    .await
}

async fn response_leg(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
    control_value: &IOValue,
    evidence_refs: &[String],
) -> Result<node_daemon::NodeControlLiveLoopback> {
    node_daemon::node_control_live_iroh_loopback(&node_daemon::NodeControlLiveLoopbackInput {
        state_root: input.requester_node_root,
        request_value: control_value,
        from_peer: input.peer_node_id,
        to_node: input.requester_node_id,
        topic: input.topic,
        sequence: input.response_sequence,
        peer_bootstrap_refs: input.response_peer_bootstrap_refs,
        authority_refs: input.response_authority_refs,
        policy_refs: input.response_policy_refs,
        resource_refs: input.response_resource_refs,
        evidence_refs,
    })
    .await
}

fn transport_notes(
    request_live: &node_daemon::NodeControlLiveLoopback,
    response_live: &node_daemon::NodeControlLiveLoopback,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("request-publish", &request_live.publish_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("request-receive", &request_live.receive_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("response-publish", &response_live.publish_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("response-receive", &response_live.receive_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn loopback_value(input: &LoopbackValueInput<'_>) -> Result<IOValue> {
    retention_remote_gc_clearance_live_workflow_value(&RetentionRemoteGcClearanceLiveWorkflowValueInput {
        request_value: input.request_value,
        response_value: input.response_value,
        import_value: input.import_value,
        request_control_ref: input.request_control_ref,
        request_publish_ref: &input.request_live.publish_receipt_ref,
        request_receive_ref: &input.request_live.receive_receipt_ref,
        request_ingress_ref: &input.request_live.ingress_receipt_ref,
        response_control_ref: input.response_control_ref,
        response_publish_ref: &input.response_live.publish_receipt_ref,
        response_receive_ref: &input.response_live.receive_receipt_ref,
        response_ingress_ref: &input.response_live.ingress_receipt_ref,
        transport_diagnostics: input.transport_diagnostics,
    })
}

fn gate_scope<'a>(input: &'a RetentionGcPlanInput<'a>) -> AdmissionScope<'a> {
    AdmissionScope {
        requester_ref: input.evidence.requester_ref.as_deref(),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
    }
}

fn gate_admissions(input: &RetentionGcPlanInput<'_>, scope: &AdmissionScope<'_>) -> Result<GateAdmissions> {
    let policy = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.policy_refs,
        expected_kind: ADMISSION_KIND_POLICY,
        scope,
        required_remote_refs: &[],
    })?;
    let authority = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.authority_refs,
        expected_kind: ADMISSION_KIND_AUTHORITY,
        scope,
        required_remote_refs: &[],
    })?;
    let supporting = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.evidence_refs,
        expected_kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
        scope,
        required_remote_refs: &[],
    })?;
    let reference_index = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.reference_index_refs,
        expected_kind: ADMISSION_KIND_REFERENCE_INDEX,
        scope,
        required_remote_refs: &[],
    })?;
    let remote_gc = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.remote_gc_refs,
        expected_kind: ADMISSION_KIND_REMOTE_GC,
        scope,
        required_remote_refs: &input.evidence.remote_refs,
    })?;
    Ok(GateAdmissions {
        policy,
        authority,
        supporting,
        reference_index,
        remote_gc,
    })
}

fn gate_remote_clearance(
    input: &RetentionGcPlanInput<'_>,
    scope: &AdmissionScope<'_>,
) -> Result<RemoteClearanceRefsResult> {
    admit_remote_clearance_refs(RemoteClearanceRefsInput {
        root: input.root,
        refs: &input.evidence.remote_clearance_refs,
        scope,
        required_remote_refs: &input.evidence.remote_refs,
        required_peer_refs: &input.evidence.remote_peer_refs,
        policy_refs: &input.evidence.policy_refs,
        authority_refs: &input.evidence.authority_refs,
    })
}

fn has_all_refs(required: &[String], admitted: &[String]) -> bool {
    required.is_empty() || required.iter().all(|reference| admitted.iter().any(|candidate| candidate == reference))
}

fn is_clearance_complete(
    input: &RetentionGcPlanInput<'_>,
    remote_gc: &AdmissionRefsResult,
    remote_clearance: &RemoteClearanceRefsResult,
) -> bool {
    let has_local_plan = has_all_refs(&input.evidence.remote_refs, &remote_gc.remote_refs);
    let has_remote_refs = has_all_refs(&input.evidence.remote_refs, &remote_clearance.remote_refs);
    let has_remote_peers = has_all_refs(&input.evidence.remote_peer_refs, &remote_clearance.peer_refs);
    has_local_plan && has_remote_refs && has_remote_peers
}

fn retention_gate_inputs<'a>(input: &'a RetentionGcPlanInput<'a>) -> Result<RetentionGateInputs<'a>> {
    let scope = gate_scope(input);
    let admissions = gate_admissions(input, &scope)?;
    let remote_clearance = gate_remote_clearance(input, &scope)?;
    let has_remote_gc_clearance = is_clearance_complete(input, &admissions.remote_gc, &remote_clearance);
    let has_delete_authority = is_destructive_action(input.action)
        && !admissions.authority.admitted_refs.is_empty()
        && !admissions.policy.admitted_refs.is_empty()
        && !admissions.supporting.admitted_refs.is_empty()
        && (!input.evidence.is_reference_index_complete || !admissions.reference_index.admitted_refs.is_empty())
        && has_remote_gc_clearance;
    Ok(RetentionGateInputs {
        input,
        policy: admissions.policy,
        authority: admissions.authority,
        supporting: admissions.supporting,
        reference_index: admissions.reference_index,
        remote_gc: admissions.remote_gc,
        remote_clearance,
        has_delete_authority,
        has_remote_gc_clearance,
    })
}

fn retention_plan_gates(
    input: &RetentionGateInputs<'_>,
    index: &RetentionReferenceIndex,
) -> Result<Vec<RetentionPlanGate>> {
    let mut gates = Vec::new();
    push_access_gates(&mut gates, input)?;
    push_index_gates(&mut gates, input, index)?;
    push_external_gates(&mut gates, input)?;
    Ok(gates)
}

fn push_access_gates(gates: &mut impl VecSink<RetentionPlanGate>, input: &RetentionGateInputs<'_>) -> Result<()> {
    push_bounded(
        gates,
        requester_gate(input.input.evidence.requester_ref.as_deref())?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "policy",
            is_required: true,
            required_refs: &input.input.evidence.policy_refs,
            admitted_refs: &input.policy.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.policy.diagnostics,
                is_missing: input.input.evidence.policy_refs.is_empty(),
                missing_diagnostic: "retention-policy-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "authority",
            is_required: is_destructive_action(input.input.action),
            required_refs: &input.input.evidence.authority_refs,
            admitted_refs: &input.authority.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.authority.diagnostics,
                is_missing: is_destructive_action(input.input.action) && input.input.evidence.authority_refs.is_empty(),
                missing_diagnostic: "delete-authority-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "supporting-evidence",
            is_required: is_destructive_action(input.input.action),
            required_refs: &input.input.evidence.evidence_refs,
            admitted_refs: &input.supporting.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.supporting.diagnostics,
                is_missing: is_destructive_action(input.input.action) && input.input.evidence.evidence_refs.is_empty(),
                missing_diagnostic: "retention-evidence-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn push_index_gates(
    gates: &mut impl VecSink<RetentionPlanGate>,
    input: &RetentionGateInputs<'_>,
    index: &RetentionReferenceIndex,
) -> Result<()> {
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "reference-index",
            is_required: input.input.evidence.is_reference_index_complete,
            required_refs: &input.input.evidence.reference_index_refs,
            admitted_refs: &input.reference_index.admitted_refs,
            diagnostics: reference_index_gate_diagnostics(input)?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        local_retention_gate(LocalRetentionGateInput {
            input: input.input,
            index,
            has_delete_authority: input.has_delete_authority,
            has_remote_gc_clearance: input.has_remote_gc_clearance,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn push_external_gates(gates: &mut impl VecSink<RetentionPlanGate>, input: &RetentionGateInputs<'_>) -> Result<()> {
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "remote-gc",
            is_required: is_destructive_action(input.input.action) && !input.input.evidence.remote_refs.is_empty(),
            required_refs: &input.input.evidence.remote_gc_refs,
            admitted_refs: &input.remote_gc.admitted_refs,
            diagnostics: diagnostics_with_missing(MissingDiagnosticInput {
                diagnostics: &input.remote_gc.diagnostics,
                is_missing: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_refs.is_empty()
                    && input.input.evidence.remote_gc_refs.is_empty(),
                missing_diagnostic: "remote-gc-evidence-missing",
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "remote-clearance",
            is_required: is_destructive_action(input.input.action)
                && (!input.input.evidence.remote_refs.is_empty() || !input.input.evidence.remote_peer_refs.is_empty()),
            required_refs: &input.input.evidence.remote_clearance_refs,
            admitted_refs: &input.remote_clearance.admitted_refs,
            diagnostics: remote_clearance_gate_diagnostics(RemoteClearanceGateInput {
                diagnostics: &input.remote_clearance.diagnostics,
                has_missing_refs: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_refs.is_empty()
                    && input.input.evidence.remote_clearance_refs.is_empty(),
                has_missing_peers: is_destructive_action(input.input.action)
                    && !input.input.evidence.remote_peer_refs.is_empty()
                    && input.input.evidence.remote_clearance_refs.is_empty(),
            })?,
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    let empty_refs = Vec::new();
    push_bounded(
        gates,
        retention_plan_gate(PlanGateBuildInput {
            name: "evidence-only-boundary",
            is_required: false,
            required_refs: &empty_refs,
            admitted_refs: &empty_refs,
            diagnostics: Vec::new(),
        })?,
        MAX_RETENTION_REFS,
        "retention GC plan gates",
    )?;
    Ok(())
}

fn requester_gate(requester_ref: Option<&str>) -> Result<RetentionPlanGate> {
    let required_refs = requester_ref.map(|reference| vec![reference.to_string()]).unwrap_or_default();
    let diagnostics = if requester_ref.is_some() {
        Vec::new()
    } else {
        vec!["retention-requester-missing".to_string()]
    };
    retention_plan_gate(PlanGateBuildInput {
        name: "requester",
        is_required: true,
        required_refs: &required_refs,
        admitted_refs: &required_refs,
        diagnostics,
    })
}

fn local_retention_gate(input: LocalRetentionGateInput<'_>) -> Result<RetentionPlanGate> {
    let requester_ref = match input.input.evidence.requester_ref.as_ref() {
        Some(reference) => reference.clone(),
        None => synthetic_ref("retention-gc-plan-missing-requester")?,
    };
    let local_input = RetentionEvaluationInput {
        root: input.input.root,
        object_ref: input.input.object_ref,
        object_kind: input.input.object_kind,
        retention_class: input.input.retention_class,
        action: input.input.action,
        requester_ref: &requester_ref,
        is_reference_index_complete: input.input.evidence.is_reference_index_complete,
        retained_refs: &input.input.evidence.retained_refs,
        remote_refs: &input.input.evidence.remote_refs,
        policy_refs: &input.input.evidence.policy_refs,
        evidence_refs: &input.input.evidence.evidence_refs,
        has_delete_authority: input.has_delete_authority,
        has_remote_gc_clearance: input.has_remote_gc_clearance,
    };
    let diagnostics = retention_diagnostics(&local_input, input.index)?;
    let required_refs = vec![input.index.index_ref.clone()];
    let admitted_refs = if diagnostics.is_empty() {
        required_refs.clone()
    } else {
        Vec::new()
    };
    retention_plan_gate(PlanGateBuildInput {
        name: "local-retention",
        is_required: true,
        required_refs: &required_refs,
        admitted_refs: &admitted_refs,
        diagnostics,
    })
}

fn diagnostics_with_missing(input: MissingDiagnosticInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics.to_vec();
    if input.is_missing {
        push_bounded(
            &mut diagnostics,
            input.missing_diagnostic.to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan gate diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn reference_index_gate_diagnostics(input: &RetentionGateInputs<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.reference_index.diagnostics.clone();
    if !input.input.evidence.is_reference_index_complete {
        push_bounded(
            &mut diagnostics,
            "incomplete-reference-proof".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan reference-index diagnostics",
        )?;
    }
    if input.input.evidence.is_reference_index_complete && input.input.evidence.reference_index_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "reference-index-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan reference-index diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn remote_clearance_gate_diagnostics(input: RemoteClearanceGateInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics.to_vec();
    if input.has_missing_refs || input.has_missing_peers {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan remote-clearance diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn retention_plan_gate(input: PlanGateBuildInput<'_>) -> Result<RetentionPlanGate> {
    validate_name(input.name, "retention GC plan gate name")?;
    validate_refs(input.required_refs, "retention GC plan required ref")?;
    validate_refs(input.admitted_refs, "retention GC plan admitted ref")?;
    let is_pass = input.diagnostics.is_empty() && (!input.is_required || !input.admitted_refs.is_empty());
    Ok(RetentionPlanGate {
        name: input.name.to_string(),
        decision: pass_or_deny(is_pass).to_string(),
        required_refs: input.required_refs.to_vec(),
        admitted_refs: input.admitted_refs.to_vec(),
        diagnostics: input.diagnostics,
    })
}

fn retention_plan_gate_value(input: &RetentionPlanGate) -> Result<IOValue> {
    validate_name(&input.name, "retention GC plan gate name")?;
    validate_decision(&input.decision)?;
    validate_refs(&input.required_refs, "retention GC plan gate required ref")?;
    validate_refs(&input.admitted_refs, "retention GC plan gate admitted ref")?;
    Ok(record("gate", vec![
        record("name", vec![string(&input.name)]),
        record("decision", vec![string(&input.decision)]),
        record("required", vec![strings_sequence(&input.required_refs)]),
        record("admitted", vec![strings_sequence(&input.admitted_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
    ]))
}

fn parse_retention_plan_gates(value: &Value<IOValue>) -> Result<Vec<RetentionPlanGate>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("gates", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gates"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gate sequence"))?;
    let mut gates = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let gate_value = value_to_iovalue(entry);
        push_bounded(
            &mut gates,
            parse_retention_plan_gate(&gate_value)?,
            MAX_RETENTION_REFS,
            "retention GC plan gates",
        )?;
    }
    Ok(gates)
}

fn parse_retention_plan_gate(value: &IOValue) -> Result<RetentionPlanGate> {
    let fields = value
        .collect_simple_record("gate", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gate"))?;
    let name = record_string(&fields[0], "name")?;
    validate_name(&name, "retention GC plan gate name")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let required_refs = record_ref_sequence(&fields[2], "required")?;
    let admitted_refs = record_ref_sequence(&fields[3], "admitted")?;
    let diagnostics = record_string_sequence(&fields[4], "diagnostics")?;
    Ok(RetentionPlanGate {
        name,
        decision,
        required_refs,
        admitted_refs,
        diagnostics,
    })
}

fn parse_embedded_reference_index(value: &Value<IOValue>) -> Result<(String, RetentionReferenceIndex)> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("index", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded retention index"))?;
    let index_ref = required_string(&fields[0], "embedded retention index ref")?;
    require_ref(&index_ref, "embedded retention index ref")?;
    let index_value = value_to_iovalue(&fields[1]);
    let index = parse_reference_index(&index_value)?;
    if index.index_ref != index_ref {
        return Err(MoltenError::invalid_harness("embedded retention index ref mismatch"));
    }
    Ok((index_ref, index))
}

fn parse_embedded_destructive_retention_evidence_summary(value: &Value<IOValue>) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("retention-evidence", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded retention evidence summary"))?;
    parse_destructive_retention_evidence_summary(&value_to_iovalue(&fields[0]))
}

fn parse_destructive_retention_evidence_summary(value: &IOValue) -> Result<IOValue> {
    parse_destructive_retention_evidence_summary_to_evidence(value)?;
    Ok(value.clone())
}

fn parse_destructive_retention_evidence_summary_to_evidence(value: &IOValue) -> Result<DestructiveRetentionEvidence> {
    let fields = value
        .collect_simple_record("retention-evidence-summary-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-evidence-summary-v1 ...>"))?;
    let requester_fields = fields[0]
        .collect_simple_record("requester", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention evidence requester"))?;
    let requester_value = value_to_iovalue(&requester_fields[0]);
    let requester_ref = if requester_value.collect_simple_record("none", Some(0)).is_some() {
        None
    } else {
        let requester_ref = required_string(&requester_fields[0], "retention evidence requester")?;
        require_ref(&requester_ref, "retention evidence requester")?;
        Some(requester_ref)
    };
    let evidence = DestructiveRetentionEvidence {
        requester_ref,
        policy_refs: record_ref_sequence(&fields[1], "policy")?,
        authority_refs: record_ref_sequence(&fields[2], "authority")?,
        evidence_refs: record_ref_sequence(&fields[3], "evidence")?,
        retained_refs: record_ref_sequence(&fields[4], "retained")?,
        remote_peer_refs: record_ref_sequence(&fields[5], "remote-peer")?,
        remote_refs: record_ref_sequence(&fields[6], "remote")?,
        reference_index_refs: record_ref_sequence(&fields[7], "reference-index")?,
        remote_gc_refs: record_ref_sequence(&fields[8], "remote-gc")?,
        remote_clearance_refs: record_ref_sequence(&fields[9], "remote-clearance")?,
        is_reference_index_complete: record_pass_bool(&fields[10], "reference-index-complete")?,
    };
    parse_checks(&fields[11])?;
    validate_destructive_retention_evidence(&evidence)?;
    Ok(evidence)
}

struct AdmissionCheck {
    is_admitted: bool,
    scope_mismatches: usize,
}

fn push_admission_diagnostic<S>(diagnostics: &mut S, diagnostic: String) -> Result<()>
where S: VecSink<String> {
    push_bounded(diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")
}

fn check_admission_basics<S>(
    input: &AdmissionRefsInput<'_>,
    reference: &str,
    admission: &RetentionEvidenceAdmission,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if admission.admission_ref != reference {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-ref-mismatch:{}", input.expected_kind, reference),
        )?;
    }
    if admission.kind != input.expected_kind {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-kind-mismatch:{}", input.expected_kind, reference),
        )?;
    }
    if admission.decision != "pass" {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-not-pass:{}", input.expected_kind, reference))?;
    }
    if !admission.is_current {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-stale:{}", input.expected_kind, reference))?;
    }
    if !admission.revoked_refs.is_empty() {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-revoked:{}", input.expected_kind, reference))?;
    }
    if admission.bound_refs.is_empty() {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-empty-bound-refs:{}", input.expected_kind, reference),
        )?;
    }
    Ok(is_admitted)
}

fn admission_scope_mismatch_count(scope: &AdmissionScope<'_>, admission: &RetentionEvidenceAdmission) -> usize {
    let mut count = 0usize;
    if scope.requester_ref != Some(admission.requester_ref.as_str()) {
        count += 1;
    }
    if admission.object_ref != scope.object_ref || admission.object_kind != scope.object_kind {
        count += 1;
    }
    if admission.retention_class != scope.retention_class {
        count += 1;
    }
    if admission.action != scope.action {
        count += 1;
    }
    count
}

fn check_admission_required_refs<S>(
    input: &AdmissionRefsInput<'_>,
    reference: &str,
    admission: &RetentionEvidenceAdmission,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if input.expected_kind == ADMISSION_KIND_REFERENCE_INDEX && !admission.is_reference_index_complete {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("reference-index-admission-incomplete:{}", reference))?;
    }
    if input.expected_kind == ADMISSION_KIND_REMOTE_GC {
        for required in input.required_remote_refs {
            if !admission.remote_refs.iter().any(|remote| remote == required) {
                is_admitted = false;
                push_admission_diagnostic(
                    diagnostics,
                    format!("remote-gc-admission-missing-remote:{}:{}", reference, required),
                )?;
            }
        }
    }
    Ok(is_admitted)
}

fn check_admission_ref<S>(
    input: &AdmissionRefsInput<'_>,
    reference: &str,
    admission: &RetentionEvidenceAdmission,
    diagnostics: &mut S,
) -> Result<AdmissionCheck>
where
    S: VecSink<String>,
{
    let mut is_admitted = check_admission_basics(input, reference, admission, diagnostics)?;
    let scope_mismatches = admission_scope_mismatch_count(input.scope, admission);
    if scope_mismatches > 0 {
        is_admitted = false;
    }
    if !check_admission_required_refs(input, reference, admission, diagnostics)? {
        is_admitted = false;
    }
    Ok(AdmissionCheck {
        is_admitted,
        scope_mismatches,
    })
}

fn admit_evidence_refs(input: AdmissionRefsInput<'_>) -> Result<AdmissionRefsResult> {
    let mut diagnostics = Vec::new();
    let mut admitted_refs = Vec::new();
    let mut remote_refs = Vec::new();
    let mut scope_mismatches = 0usize;
    for reference in input.refs {
        let admission = match read_retention_evidence_admission(input.root, reference) {
            Ok(admission) => admission,
            Err(error) => {
                push_admission_diagnostic(
                    &mut diagnostics,
                    format!("{}-admission-unreadable:{}:{}", input.expected_kind, reference, error),
                )?;
                continue;
            }
        };
        let check = check_admission_ref(&input, reference, &admission, &mut diagnostics)?;
        scope_mismatches += check.scope_mismatches;
        if check.is_admitted {
            push_bounded(&mut admitted_refs, admission.admission_ref, MAX_RETENTION_REFS, "retention admitted refs")?;
            for remote_ref in admission.remote_refs {
                push_bounded(&mut remote_refs, remote_ref, MAX_RETENTION_REFS, "retention admitted remote refs")?;
            }
        }
    }
    if !input.refs.is_empty() && admitted_refs.is_empty() && scope_mismatches > 0 {
        push_admission_diagnostic(&mut diagnostics, format!("{}-admission-scope-mismatch", input.expected_kind))?;
    }
    Ok(AdmissionRefsResult {
        diagnostics,
        admitted_refs,
        remote_refs,
    })
}

struct Check {
    is_admitted: bool,
    scope_mismatches: usize,
}

fn push_clear_note<S>(diagnostics: &mut S, message: String) -> Result<()>
where S: VecSink<String> {
    push_bounded(diagnostics, message, MAX_RETENTION_DIAGNOSTICS, "retention remote clearance diagnostics")
}

fn check_state<S>(reference: &str, clearance: &RetentionRemoteGcClearance, diagnostics: &mut S) -> Result<bool>
where S: VecSink<String> {
    let mut is_admitted = true;
    if clearance.clearance_ref != *reference {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-ref-mismatch:{}", reference))?;
    }
    if clearance.decision != "pass" {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-not-pass:{}", reference))?;
    }
    if !clearance.is_current {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-stale:{}", reference))?;
    }
    if !clearance.revoked_refs.is_empty() {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-revoked:{}", reference))?;
    }
    if !clearance.retained_refs.is_empty() {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-retained:{}", clearance.remote_ref))?;
    }
    Ok(is_admitted)
}

fn check_scope(input: &RemoteClearanceRefsInput<'_>, clearance: &RetentionRemoteGcClearance) -> Check {
    let mut scope_mismatches = 0usize;
    if input.scope.requester_ref != Some(clearance.requester_ref.as_str()) {
        scope_mismatches += 1;
    }
    if clearance.object_ref != input.scope.object_ref || clearance.object_kind != input.scope.object_kind {
        scope_mismatches += 1;
    }
    if clearance.retention_class != input.scope.retention_class {
        scope_mismatches += 1;
    }
    if clearance.action != input.scope.action {
        scope_mismatches += 1;
    }
    Check {
        is_admitted: scope_mismatches == 0,
        scope_mismatches,
    }
}

fn check_bindings<S>(
    input: &RemoteClearanceRefsInput<'_>,
    clearance: &RetentionRemoteGcClearance,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if !input.policy_refs.iter().any(|policy_ref| policy_ref == &clearance.policy_ref) {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-policy-mismatch:{}", clearance.remote_ref))?;
    }
    if !input.authority_refs.iter().any(|authority_ref| authority_ref == &clearance.authority_ref) {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-authority-mismatch:{}", clearance.remote_ref))?;
    }
    Ok(is_admitted)
}

fn check_clear_ref<S>(
    input: &RemoteClearanceRefsInput<'_>,
    reference: &str,
    clearance: &RetentionRemoteGcClearance,
    diagnostics: &mut S,
) -> Result<Check>
where
    S: VecSink<String>,
{
    let is_state_admitted = check_state(reference, clearance, diagnostics)?;
    let scope = check_scope(input, clearance);
    let is_binding_admitted = check_bindings(input, clearance, diagnostics)?;
    Ok(Check {
        is_admitted: is_state_admitted && scope.is_admitted && is_binding_admitted,
        scope_mismatches: scope.scope_mismatches,
    })
}

fn collect_clear_refs(
    admitted_refs: &mut impl VecSink<String>,
    remote_refs: &mut impl VecSink<String>,
    peer_refs: &mut impl VecSink<String>,
    clearance: RetentionRemoteGcClearance,
) -> Result<()> {
    push_bounded(admitted_refs, clearance.clearance_ref, MAX_RETENTION_REFS, "retention remote clearance refs")?;
    push_bounded(remote_refs, clearance.remote_ref, MAX_RETENTION_REFS, "retention remote clearance remote refs")?;
    push_bounded(peer_refs, clearance.peer_ref, MAX_RETENTION_REFS, "retention remote clearance peer refs")
}

fn push_missing_clear_refs<S>(
    input: &RemoteClearanceRefsInput<'_>,
    remote_refs: &[String],
    peer_refs: &[String],
    diagnostics: &mut S,
) -> Result<()>
where
    S: VecSink<String>,
{
    for required in input.required_remote_refs {
        if !remote_refs.iter().any(|remote| remote == required) {
            push_clear_note(diagnostics, format!("remote-clearance-missing-remote:{}", required))?;
        }
    }
    for required in input.required_peer_refs {
        if !peer_refs.iter().any(|peer| peer == required) {
            push_clear_note(diagnostics, format!("remote-clearance-missing-peer:{}", required))?;
        }
    }
    Ok(())
}

fn admit_remote_clearance_refs(input: RemoteClearanceRefsInput<'_>) -> Result<RemoteClearanceRefsResult> {
    let mut diagnostics = Vec::new();
    let mut admitted_refs = Vec::new();
    let mut remote_refs = Vec::new();
    let mut peer_refs = Vec::new();
    let mut scope_mismatches = 0usize;
    for reference in input.refs {
        let clearance = match read_retention_remote_gc_clearance(input.root, reference) {
            Ok(clearance) => clearance,
            Err(error) => {
                push_clear_note(&mut diagnostics, format!("remote-clearance-unreadable:{}:{}", reference, error))?;
                continue;
            }
        };
        let check = check_clear_ref(&input, reference, &clearance, &mut diagnostics)?;
        scope_mismatches += check.scope_mismatches;
        if check.is_admitted {
            collect_clear_refs(&mut admitted_refs, &mut remote_refs, &mut peer_refs, clearance)?;
        }
    }
    if !input.refs.is_empty() && admitted_refs.is_empty() && scope_mismatches > 0 {
        push_clear_note(&mut diagnostics, "remote-clearance-scope-mismatch".to_string())?;
    }
    push_missing_clear_refs(&input, &remote_refs, &peer_refs, &mut diagnostics)?;
    Ok(RemoteClearanceRefsResult {
        diagnostics,
        admitted_refs,
        remote_refs,
        peer_refs,
    })
}

fn read_retention_evidence_admission(root: &Path, admission_ref: &str) -> Result<RetentionEvidenceAdmission> {
    require_ref(admission_ref, "retention evidence admission ref")?;
    let value = read_store_value(&admission_path(root, admission_ref)?)?;
    parse_retention_evidence_admission(&value)
}

fn read_retention_remote_gc_clearance(root: &Path, clearance_ref: &str) -> Result<RetentionRemoteGcClearance> {
    require_ref(clearance_ref, "retention remote GC clearance ref")?;
    let value = read_store_value(&remote_clearance_path(root, clearance_ref)?)?;
    parse_retention_remote_gc_clearance(&value)
}

pub fn admit_destructive_retention_evidence(
    input: DestructiveRetentionAdmissionInput<'_>,
) -> Result<DestructiveRetentionAdmission> {
    ensure_store(input.root)?;
    validate_destructive_retention_evidence(input.evidence)?;
    require_ref(input.object_ref, "retention admission object ref")?;
    validate_name(input.object_kind, "retention admission object kind")?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    let mut diagnostics = destructive_retention_evidence_diagnostics(input.evidence, input.action)?;
    let mut admitted_refs = Vec::new();
    let scope = AdmissionScope {
        requester_ref: input.evidence.requester_ref.as_deref(),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
    };
    let policy = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.policy_refs,
        expected_kind: ADMISSION_KIND_POLICY,
        scope: &scope,
        required_remote_refs: &[],
    })?;
    let authority = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.authority_refs,
        expected_kind: ADMISSION_KIND_AUTHORITY,
        scope: &scope,
        required_remote_refs: &[],
    })?;
    let supporting = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.evidence_refs,
        expected_kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
        scope: &scope,
        required_remote_refs: &[],
    })?;
    let reference_index = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.reference_index_refs,
        expected_kind: ADMISSION_KIND_REFERENCE_INDEX,
        scope: &scope,
        required_remote_refs: &[],
    })?;
    let remote_gc = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.remote_gc_refs,
        expected_kind: ADMISSION_KIND_REMOTE_GC,
        scope: &scope,
        required_remote_refs: &input.evidence.remote_refs,
    })?;
    let remote_clearance = admit_remote_clearance_refs(RemoteClearanceRefsInput {
        root: input.root,
        refs: &input.evidence.remote_clearance_refs,
        scope: &scope,
        required_remote_refs: &input.evidence.remote_refs,
        required_peer_refs: &input.evidence.remote_peer_refs,
        policy_refs: &input.evidence.policy_refs,
        authority_refs: &input.evidence.authority_refs,
    })?;
    let has_policy_admission = !policy.admitted_refs.is_empty();
    let has_authority_admission = !authority.admitted_refs.is_empty();
    let has_supporting_admission = !supporting.admitted_refs.is_empty();
    let has_reference_index_admission = !reference_index.admitted_refs.is_empty();
    for diagnostic in policy
        .diagnostics
        .into_iter()
        .chain(authority.diagnostics)
        .chain(supporting.diagnostics)
        .chain(reference_index.diagnostics)
        .chain(remote_gc.diagnostics)
        .chain(remote_clearance.diagnostics)
    {
        push_bounded(&mut diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")?;
    }
    for reference in policy
        .admitted_refs
        .into_iter()
        .chain(authority.admitted_refs)
        .chain(supporting.admitted_refs)
        .chain(reference_index.admitted_refs)
        .chain(remote_gc.admitted_refs.clone())
        .chain(remote_clearance.admitted_refs.clone())
    {
        push_bounded(&mut admitted_refs, reference, MAX_RETENTION_REFS, "retention admitted refs")?;
    }
    let has_local_remote_gc_plan = input.evidence.remote_refs.is_empty()
        || input
            .evidence
            .remote_refs
            .iter()
            .all(|reference| remote_gc.remote_refs.iter().any(|remote| remote == reference));
    let has_remote_ref_clearance = input.evidence.remote_refs.is_empty()
        || input
            .evidence
            .remote_refs
            .iter()
            .all(|reference| remote_clearance.remote_refs.iter().any(|remote| remote == reference));
    let has_remote_peer_clearance = input.evidence.remote_peer_refs.is_empty()
        || input
            .evidence
            .remote_peer_refs
            .iter()
            .all(|peer| remote_clearance.peer_refs.iter().any(|cleared_peer| cleared_peer == peer));
    let has_remote_refs_clearance = has_local_remote_gc_plan && has_remote_ref_clearance && has_remote_peer_clearance;
    let has_delete_authority = is_destructive_action(input.action)
        && has_authority_admission
        && has_policy_admission
        && has_supporting_admission
        && (!input.evidence.is_reference_index_complete || has_reference_index_admission)
        && has_remote_refs_clearance;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(DestructiveRetentionAdmission {
        decision: decision.to_string(),
        diagnostics,
        admitted_refs,
        has_delete_authority,
        has_remote_gc_clearance: has_remote_refs_clearance,
    })
}

pub fn destructive_retention_requester_ref(
    input: &DestructiveRetentionEvidence,
    fallback_label: &str,
) -> Result<String> {
    validate_destructive_retention_evidence(input)?;
    if let Some(requester_ref) = input.requester_ref.as_ref() {
        Ok(requester_ref.clone())
    } else {
        synthetic_ref(fallback_label)
    }
}

pub fn destructive_retention_has_authority(input: &DestructiveRetentionEvidence) -> bool {
    input.requester_ref.is_some() && !input.authority_refs.is_empty()
}

pub fn validate_destructive_retention_evidence(input: &DestructiveRetentionEvidence) -> Result<()> {
    if let Some(requester_ref) = input.requester_ref.as_ref() {
        require_ref(requester_ref, "retention requester ref")?;
    }
    validate_refs(&input.policy_refs, "retention policy ref")?;
    validate_refs(&input.authority_refs, "retention authority ref")?;
    validate_refs(&input.evidence_refs, "retention evidence ref")?;
    validate_refs(&input.retained_refs, "retention retained ref")?;
    validate_refs(&input.remote_peer_refs, "retention remote peer ref")?;
    validate_refs(&input.remote_refs, "retention remote ref")?;
    validate_refs(&input.reference_index_refs, "retention reference-index ref")?;
    validate_refs(&input.remote_gc_refs, "retention remote-gc ref")?;
    validate_refs(&input.remote_clearance_refs, "retention remote clearance ref")
}

fn validate_retention_gc_plan_input(input: &RetentionGcPlanInput<'_>) -> Result<()> {
    validate_name(input.subsystem, "retention GC plan subsystem")?;
    require_ref(input.object_ref, "retention GC plan object ref")?;
    validate_name(input.object_kind, "retention GC plan object kind")?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    validate_destructive_retention_evidence(input.evidence)
}

struct MissingNote<'a> {
    emit: bool,
    message: &'a str,
}

fn push_missing_notes<S>(diagnostics: &mut S, notes: &[MissingNote<'_>]) -> Result<()>
where S: VecSink<String> {
    for note in notes {
        if note.emit {
            push_bounded(
                diagnostics,
                note.message.to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention destructive evidence diagnostics",
            )?;
        }
    }
    Ok(())
}

pub fn destructive_retention_evidence_diagnostics(
    input: &DestructiveRetentionEvidence,
    action: &str,
) -> Result<Vec<String>> {
    validate_destructive_retention_evidence(input)?;
    validate_action(action)?;
    let is_destructive = is_destructive_action(action);
    let mut diagnostics = Vec::new();
    let notes = [
        MissingNote {
            emit: input.requester_ref.is_none(),
            message: "retention-requester-missing",
        },
        MissingNote {
            emit: input.policy_refs.is_empty(),
            message: "retention-policy-missing",
        },
        MissingNote {
            emit: is_destructive && input.authority_refs.is_empty(),
            message: "delete-authority-missing",
        },
        MissingNote {
            emit: is_destructive && input.evidence_refs.is_empty(),
            message: "retention-evidence-missing",
        },
        MissingNote {
            emit: !input.is_reference_index_complete,
            message: "incomplete-reference-proof",
        },
        MissingNote {
            emit: is_destructive && input.is_reference_index_complete && input.reference_index_refs.is_empty(),
            message: "reference-index-evidence-missing",
        },
        MissingNote {
            emit: !input.retained_refs.is_empty(),
            message: "retained-dependencies-present",
        },
        MissingNote {
            emit: is_destructive && !input.remote_refs.is_empty() && input.remote_gc_refs.is_empty(),
            message: "remote-gc-evidence-missing",
        },
        MissingNote {
            emit: is_destructive
                && (!input.remote_refs.is_empty() || !input.remote_peer_refs.is_empty())
                && input.remote_clearance_refs.is_empty(),
            message: "remote-clearance-evidence-missing",
        },
    ];
    push_missing_notes(&mut diagnostics, &notes)?;
    Ok(diagnostics)
}

pub fn destructive_retention_evidence_value(input: &DestructiveRetentionEvidence) -> Result<IOValue> {
    validate_destructive_retention_evidence(input)?;
    let requester_value = input.requester_ref.as_deref().map(string).unwrap_or_else(|| record("none", Vec::new()));
    Ok(record("retention-evidence-summary-v1", vec![
        record("requester", vec![requester_value]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("authority", vec![strings_sequence(&input.authority_refs)]),
        record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        record("retained", vec![strings_sequence(&input.retained_refs)]),
        record("remote-peer", vec![strings_sequence(&input.remote_peer_refs)]),
        record("remote", vec![strings_sequence(&input.remote_refs)]),
        record("reference-index", vec![strings_sequence(&input.reference_index_refs)]),
        record("remote-gc", vec![strings_sequence(&input.remote_gc_refs)]),
        record("remote-clearance", vec![strings_sequence(&input.remote_clearance_refs)]),
        record("reference-index-complete", vec![string(pass_or_deny(input.is_reference_index_complete))]),
        checks_value(&[
            ("requester-bound", pass_or_deny(input.requester_ref.is_some())),
            ("policy-bound", pass_or_deny(!input.policy_refs.is_empty())),
            ("authority-bound", pass_or_deny(!input.authority_refs.is_empty())),
            ("evidence-bound", pass_or_deny(!input.evidence_refs.is_empty())),
            ("reference-index-bound", pass_or_deny(!input.reference_index_refs.is_empty())),
            ("remote-gc-bound", pass_or_deny(input.remote_refs.is_empty() || !input.remote_gc_refs.is_empty())),
            (
                "remote-clearance-bound",
                pass_or_deny(
                    (input.remote_refs.is_empty() && input.remote_peer_refs.is_empty())
                        || !input.remote_clearance_refs.is_empty(),
                ),
            ),
        ]),
    ]))
}

pub fn store_retention_gc_plan(input: RetentionGcPlanInput<'_>) -> Result<RetentionGcPlan> {
    ensure_store(input.root)?;
    validate_retention_gc_plan_input(&input)?;
    let index = reference_index_for_object(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.evidence.retained_refs.as_slice(),
        remote_refs: input.evidence.remote_refs.as_slice(),
        is_complete: input.evidence.is_reference_index_complete,
    })?;
    let gate_inputs = retention_gate_inputs(&input)?;
    let gates = retention_plan_gates(&gate_inputs, &index)?;
    let mut diagnostics = Vec::new();
    for gate in &gates {
        extend_bounded(
            &mut diagnostics,
            gate.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC plan diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if gates.iter().all(|gate| gate.decision == "pass") && diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    let evidence_value = destructive_retention_evidence_value(input.evidence)?;
    let value = retention_gc_plan_value(&RetentionGcPlanValueInput {
        decision,
        subsystem: input.subsystem,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        requester_ref: input.evidence.requester_ref.as_deref(),
        index: &index,
        evidence_value: &evidence_value,
        gates: &gates,
        diagnostics: &diagnostics,
    })?;
    let plan = parse_retention_gc_plan(&value)?;
    write_store_value(&gc_plan_path(input.root, &plan.plan_ref)?, &plan.value)?;
    Ok(plan)
}

pub fn retention_gc_plan_value(input: &RetentionGcPlanValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC plan subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC plan object ref")?;
    validate_name(input.object_kind, "retention GC plan object kind")?;
    validate_retention_class(input.retention_class)?;
    if let Some(requester_ref) = input.requester_ref {
        require_ref(requester_ref, "retention GC plan requester ref")?;
    }
    parse_destructive_retention_evidence_summary(input.evidence_value)?;
    let gate_values = input.gates.iter().map(retention_plan_gate_value).collect::<Result<Vec<_>>>()?;
    Ok(record("retention-gc-plan-v1", vec![
        string(RETENTION_GC_PLAN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("mode", vec![string("dry-run")]),
        record("subsystem", vec![string(input.subsystem)]),
        record("action", vec![string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("requester", vec![optional_ref_value(input.requester_ref)]),
        record("index", vec![string(&input.index.index_ref), input.index.value.clone()]),
        record("retention-evidence", vec![input.evidence_value.clone()]),
        record("gates", vec![sequence(gate_values)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("dry-run-only", "pass"),
            ("no-retention-receipt-written", "pass"),
            ("no-tombstone-written", "pass"),
            ("plan-is-not-authority", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_gc_plan(value: &IOValue) -> Result<RetentionGcPlan> {
    let fields = value
        .collect_simple_record("retention-gc-plan-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-plan-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_GC_PLAN_SCHEMA, "retention GC plan schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "dry-run" {
        return Err(MoltenError::invalid_harness("retention GC plan mode must be dry-run"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC plan subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_retention_class(&retention_class)?;
    let requester_ref = record_optional_ref(&fields[7], "requester")?;
    let (index_ref, index) = parse_embedded_reference_index(&fields[8])?;
    if index.object_ref != object_ref || index.object_kind != object_kind {
        return Err(MoltenError::invalid_harness("retention GC plan index scope mismatch"));
    }
    let evidence_value = parse_embedded_destructive_retention_evidence_summary(&fields[9])?;
    let evidence = parse_destructive_retention_evidence_summary_to_evidence(&evidence_value)?;
    if requester_ref != evidence.requester_ref {
        return Err(MoltenError::invalid_harness("retention GC plan requester evidence mismatch"));
    }
    let gates = parse_retention_plan_gates(&fields[10])?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "dry-run-only", "retention GC plan")?;
    require_check(&checks, "plan-is-not-authority", "retention GC plan")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC plan")?;
    let evidence_ref = canonical_hash(&evidence_value)?;
    require_ref(&evidence_ref, "retention GC plan evidence summary ref")?;
    Ok(RetentionGcPlan {
        plan_ref: canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        index_ref,
        evidence,
        gates,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_retention_gc_plan(root: &Path, plan_ref: &str) -> Result<RetentionGcPlan> {
    require_ref(plan_ref, "retention GC plan ref")?;
    let value = read_store_value(&gc_plan_path(root, plan_ref)?)?;
    let plan = parse_retention_gc_plan(&value)?;
    if plan.plan_ref != plan_ref {
        return Err(MoltenError::invalid_harness("stored retention GC plan ref mismatch"));
    }
    Ok(plan)
}

pub fn apply_retention_gc_plan(input: RetentionGcApplyFromPlanInput<'_>) -> Result<RetentionGcApply> {
    ensure_store(input.root)?;
    let original = read_retention_gc_plan(input.root, input.plan_ref)?;
    let recomputed = store_retention_gc_plan(RetentionGcPlanInput {
        root: input.root,
        subsystem: &original.subsystem,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        action: &original.action,
        evidence: &original.evidence,
    })?;
    let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
        root: input.root,
        evidence: &original.evidence,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        action: &original.action,
    })?;
    let mut diagnostics = Vec::new();
    if original.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-apply-plan-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            original.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
    }
    if recomputed.plan_ref != original.plan_ref {
        push_bounded(
            &mut diagnostics,
            "retention-gc-apply-plan-drift".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
    }
    if recomputed.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-apply-recomputed-plan-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            recomputed.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
    }
    if admission.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-apply-admission-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            admission.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC apply diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let mut retention_receipt_ref = None;
    let mut tombstone_ref = None;
    if diagnostics.is_empty() {
        let requester_ref =
            destructive_retention_requester_ref(&original.evidence, "retention-gc-apply-missing-requester")?;
        let evaluation = evaluate_retention(RetentionEvaluationInput {
            root: input.root,
            object_ref: &original.object_ref,
            object_kind: &original.object_kind,
            retention_class: &original.retention_class,
            action: &original.action,
            requester_ref: &requester_ref,
            is_reference_index_complete: original.evidence.is_reference_index_complete,
            retained_refs: &original.evidence.retained_refs,
            remote_refs: &original.evidence.remote_refs,
            policy_refs: &original.evidence.policy_refs,
            evidence_refs: &original.evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })?;
        retention_receipt_ref = Some(evaluation.receipt.receipt_ref.clone());
        tombstone_ref = evaluation.tombstone.as_ref().map(|created| created.tombstone_ref.clone());
        if evaluation.receipt.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                "retention-gc-apply-retention-receipt-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC apply diagnostics",
            )?;
            extend_bounded(
                &mut diagnostics,
                evaluation.receipt.diagnostics.iter().cloned(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC apply diagnostics",
            )?;
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut admission_refs = admission.admitted_refs;
    admission_refs.sort();
    admission_refs.dedup();
    let value = retention_gc_apply_value(&RetentionGcApplyValueInput {
        decision,
        subsystem: &original.subsystem,
        action: &original.action,
        object_ref: &original.object_ref,
        object_kind: &original.object_kind,
        retention_class: &original.retention_class,
        requester_ref: original.requester_ref.as_deref(),
        plan_ref: &original.plan_ref,
        recomputed_plan_ref: &recomputed.plan_ref,
        retention_receipt_ref: retention_receipt_ref.as_deref(),
        tombstone_ref: tombstone_ref.as_deref(),
        admission_refs: &admission_refs,
        diagnostics: &diagnostics,
    })?;
    let apply = parse_retention_gc_apply(&value)?;
    write_store_value(&gc_apply_path(input.root, &apply.apply_ref)?, &apply.value)?;
    Ok(apply)
}

fn retention_gc_apply_value(input: &RetentionGcApplyValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC apply subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC apply object ref")?;
    validate_name(input.object_kind, "retention GC apply object kind")?;
    validate_retention_class(input.retention_class)?;
    if let Some(requester_ref) = input.requester_ref {
        require_ref(requester_ref, "retention GC apply requester ref")?;
    }
    require_ref(input.plan_ref, "retention GC apply plan ref")?;
    require_ref(input.recomputed_plan_ref, "retention GC apply recomputed plan ref")?;
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC apply receipt ref")?;
    }
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC apply tombstone ref")?;
    }
    validate_refs(input.admission_refs, "retention GC apply admission ref")?;
    let is_plan_unchanged = input.plan_ref == input.recomputed_plan_ref;
    let is_plan_passed = input.decision == "pass";
    let is_tombstone_bound =
        !is_destructive_action(input.action) || input.decision != "pass" || input.tombstone_ref.is_some();
    Ok(record("retention-gc-apply-v1", vec![
        string(RETENTION_GC_APPLY_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("mode", vec![string("apply")]),
        record("subsystem", vec![string(input.subsystem)]),
        record("action", vec![string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("requester", vec![optional_ref_value(input.requester_ref)]),
        record("plan", vec![string(input.plan_ref)]),
        record("recomputed-plan", vec![string(input.recomputed_plan_ref)]),
        record("retention-receipt", vec![optional_ref_value(input.retention_receipt_ref)]),
        record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        record("admission", vec![strings_sequence(input.admission_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("plan-ref-bound", "pass"),
            ("plan-recomputed-before-mutation", "pass"),
            ("plan-unchanged", pass_or_deny(is_plan_unchanged)),
            ("plan-decision-pass", pass_or_deny(is_plan_passed)),
            ("normal-admission-run", "pass"),
            (
                "retention-receipt-bound",
                pass_or_deny(input.decision != "pass" || input.retention_receipt_ref.is_some()),
            ),
            ("tombstone-bound", pass_or_deny(is_tombstone_bound)),
            (
                "deny-before-mutation",
                pass_or_deny(input.decision == "pass" || input.retention_receipt_ref.is_none()),
            ),
            ("plan-is-not-authority", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_gc_apply(value: &IOValue) -> Result<RetentionGcApply> {
    let fields = value
        .collect_simple_record("retention-gc-apply-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-apply-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_GC_APPLY_SCHEMA, "retention GC apply schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "apply" {
        return Err(MoltenError::invalid_harness("retention GC apply mode must be apply"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC apply subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_retention_class(&retention_class)?;
    let requester_ref = record_optional_ref(&fields[7], "requester")?;
    let plan_ref = record_ref(&fields[8], "plan")?;
    let recomputed_plan_ref = record_ref(&fields[9], "recomputed-plan")?;
    let retention_receipt_ref = record_optional_ref(&fields[10], "retention-receipt")?;
    let tombstone_ref = record_optional_ref(&fields[11], "tombstone")?;
    let admission_refs = record_ref_sequence(&fields[12], "admission")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "plan-ref-bound", "retention GC apply")?;
    require_check(&checks, "plan-recomputed-before-mutation", "retention GC apply")?;
    require_check(&checks, "normal-admission-run", "retention GC apply")?;
    require_check(&checks, "plan-is-not-authority", "retention GC apply")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC apply")?;
    Ok(RetentionGcApply {
        apply_ref: canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        plan_ref,
        recomputed_plan_ref,
        retention_receipt_ref,
        tombstone_ref,
        admission_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_retention_gc_apply(root: &Path, apply_ref: &str) -> Result<RetentionGcApply> {
    require_ref(apply_ref, "retention GC apply ref")?;
    let value = read_store_value(&gc_apply_path(root, apply_ref)?)?;
    let apply = parse_retention_gc_apply(&value)?;
    if apply.apply_ref != apply_ref {
        return Err(MoltenError::invalid_harness("stored retention GC apply ref mismatch"));
    }
    Ok(apply)
}

pub fn read_retention_gc_execution_gate(root: &Path, execution_ref: &str) -> Result<RetentionGcExecutionGate> {
    require_ref(execution_ref, "retention GC execution ref")?;
    let value = read_store_value(&gc_execute_path(root, execution_ref)?)?;
    let gate = parse_retention_gc_execution_gate(&value)?;
    if gate.execution_ref != execution_ref {
        return Err(MoltenError::invalid_harness("stored retention GC execution ref mismatch"));
    }
    Ok(gate)
}

pub fn store_retention_gc_execution_gate(input: RetentionGcExecutionGateInput<'_>) -> Result<RetentionGcExecutionGate> {
    ensure_store(input.root)?;
    validate_name(input.subsystem, "retention GC execution subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC execution object ref")?;
    validate_name(input.object_kind, "retention GC execution object kind")?;
    validate_retention_class(input.retention_class)?;
    let mut diagnostics = Vec::new();
    let mut plan_ref = None;
    let mut recomputed_plan_ref = None;
    let mut retention_receipt_ref = None;
    let mut tombstone_ref = None;
    if let Some(apply_ref) = input.apply_ref {
        require_ref(apply_ref, "retention GC execution apply ref")?;
        match read_retention_gc_apply(input.root, apply_ref) {
            Ok(apply) => {
                plan_ref = Some(apply.plan_ref.clone());
                recomputed_plan_ref = Some(apply.recomputed_plan_ref.clone());
                retention_receipt_ref = apply.retention_receipt_ref.clone();
                tombstone_ref = apply.tombstone_ref.clone();
                extend_bounded(
                    &mut diagnostics,
                    execution_gate_apply_diagnostics(&input, &apply)?,
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
                if let Some(receipt_ref) = apply.retention_receipt_ref.as_ref() {
                    extend_bounded(
                        &mut diagnostics,
                        execution_gate_receipt_diagnostics(input.root, &input, receipt_ref)?,
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention GC execution diagnostics",
                    )?;
                } else {
                    push_bounded(
                        &mut diagnostics,
                        "retention-gc-execute-retention-receipt-missing".to_string(),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention GC execution diagnostics",
                    )?;
                }
                extend_bounded(
                    &mut diagnostics,
                    execution_gate_tombstone_binding_diagnostics(input.root, &input, &apply)?,
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            Err(error) => push_bounded(
                &mut diagnostics,
                format!("retention-gc-execute-apply-unreadable:{error}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC execution diagnostics",
            )?,
        }
    } else {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = retention_gc_execution_gate_value(&RetentionGcExecutionGateValueInput {
        decision,
        subsystem: input.subsystem,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        apply_ref: input.apply_ref,
        plan_ref: plan_ref.as_deref(),
        recomputed_plan_ref: recomputed_plan_ref.as_deref(),
        retention_receipt_ref: retention_receipt_ref.as_deref(),
        tombstone_ref: tombstone_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let gate = parse_retention_gc_execution_gate(&value)?;
    write_store_value(&gc_execute_path(input.root, &gate.execution_ref)?, &gate.value)?;
    Ok(gate)
}

fn execution_gate_apply_diagnostics(
    input: &RetentionGcExecutionGateInput<'_>,
    apply: &RetentionGcApply,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if apply.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            apply.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    if apply.plan_ref != apply.recomputed_plan_ref {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-plan-drift".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    if apply.subsystem != input.subsystem
        || apply.action != input.action
        || apply.object_ref != input.object_ref
        || apply.object_kind != input.object_kind
        || apply.retention_class != input.retention_class
    {
        push_bounded(
            &mut diagnostics,
            "retention-gc-execute-apply-scope-mismatch".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn execution_gate_receipt_diagnostics(
    root: &Path,
    input: &RetentionGcExecutionGateInput<'_>,
    receipt_ref: &str,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    match read_retention_receipt(root, receipt_ref) {
        Ok(receipt) => {
            if receipt.decision != "pass" {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-not-pass".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if receipt.object_ref != input.object_ref
                || receipt.object_kind != input.object_kind
                || receipt.retention_class != input.retention_class
                || receipt.action != input.action
            {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-scope-mismatch".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if receipt.tombstone_ref.is_none() && is_destructive_action(input.action) {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-retention-receipt-tombstone-missing".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
        }
        Err(error) => push_bounded(
            &mut diagnostics,
            format!("retention-gc-execute-retention-receipt-unreadable:{error}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?,
    }
    Ok(diagnostics)
}

fn execution_gate_tombstone_binding_diagnostics(
    root: &Path,
    input: &RetentionGcExecutionGateInput<'_>,
    apply: &RetentionGcApply,
) -> Result<Vec<String>> {
    let Some(tombstone_ref) = apply.tombstone_ref.as_ref() else {
        let mut diagnostics = Vec::new();
        if is_destructive_action(input.action) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-execute-tombstone-missing".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC execution diagnostics",
            )?;
        }
        return Ok(diagnostics);
    };
    execution_gate_tombstone_diagnostics(root, input, tombstone_ref, apply.retention_receipt_ref.as_deref())
}

fn execution_gate_tombstone_diagnostics(
    root: &Path,
    input: &RetentionGcExecutionGateInput<'_>,
    tombstone_ref: &str,
    receipt_ref: Option<&str>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    match read_retention_tombstone(root, tombstone_ref) {
        Ok(tombstone) => {
            if tombstone.object_ref != input.object_ref
                || tombstone.object_kind != input.object_kind
                || tombstone.retention_class != input.retention_class
                || tombstone.action != input.action
            {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-execute-tombstone-scope-mismatch".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC execution diagnostics",
                )?;
            }
            if let Some(expected_receipt_ref) = receipt_ref {
                let pending_receipt_ref = synthetic_ref("pending-retention-receipt")?;
                if tombstone.receipt_ref != expected_receipt_ref && tombstone.receipt_ref != pending_receipt_ref {
                    push_bounded(
                        &mut diagnostics,
                        "retention-gc-execute-tombstone-receipt-mismatch".to_string(),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention GC execution diagnostics",
                    )?;
                }
            }
        }
        Err(error) => push_bounded(
            &mut diagnostics,
            format!("retention-gc-execute-tombstone-unreadable:{error}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC execution diagnostics",
        )?,
    }
    Ok(diagnostics)
}

fn retention_gc_execution_gate_value(input: &RetentionGcExecutionGateValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC execution subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC execution object ref")?;
    validate_name(input.object_kind, "retention GC execution object kind")?;
    validate_retention_class(input.retention_class)?;
    if let Some(apply_ref) = input.apply_ref {
        require_ref(apply_ref, "retention GC execution apply ref")?;
    }
    if let Some(plan_ref) = input.plan_ref {
        require_ref(plan_ref, "retention GC execution plan ref")?;
    }
    if let Some(recomputed_plan_ref) = input.recomputed_plan_ref {
        require_ref(recomputed_plan_ref, "retention GC execution recomputed plan ref")?;
    }
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC execution receipt ref")?;
    }
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC execution tombstone ref")?;
    }
    Ok(record("retention-gc-execute-v1", vec![
        string(RETENTION_GC_EXECUTE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("mode", vec![string("execute-gate")]),
        record("subsystem", vec![string(input.subsystem)]),
        record("action", vec![string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("apply", vec![optional_ref_value(input.apply_ref)]),
        record("plan", vec![optional_ref_value(input.plan_ref)]),
        record("recomputed-plan", vec![optional_ref_value(input.recomputed_plan_ref)]),
        record("retention-receipt", vec![optional_ref_value(input.retention_receipt_ref)]),
        record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("apply-ref-required", pass_or_deny(input.apply_ref.is_some())),
            ("apply-decision-pass", pass_or_deny(input.decision == "pass")),
            (
                "apply-plan-unchanged",
                pass_or_deny(input.plan_ref.is_some() && input.plan_ref == input.recomputed_plan_ref),
            ),
            ("retention-receipt-bound", pass_or_deny(input.retention_receipt_ref.is_some())),
            (
                "tombstone-bound",
                pass_or_deny(!is_destructive_action(input.action) || input.tombstone_ref.is_some()),
            ),
            ("execute-gate-is-not-authority", "pass"),
            ("normal-admission-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_gc_execution_gate(value: &IOValue) -> Result<RetentionGcExecutionGate> {
    let fields = value
        .collect_simple_record("retention-gc-execute-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-execute-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_GC_EXECUTE_SCHEMA, "retention GC execution schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "execute-gate" {
        return Err(MoltenError::invalid_harness("retention GC execution mode must be execute-gate"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC execution subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_retention_class(&retention_class)?;
    let apply_ref = record_optional_ref(&fields[7], "apply")?;
    let plan_ref = record_optional_ref(&fields[8], "plan")?;
    let recomputed_plan_ref = record_optional_ref(&fields[9], "recomputed-plan")?;
    let retention_receipt_ref = record_optional_ref(&fields[10], "retention-receipt")?;
    let tombstone_ref = record_optional_ref(&fields[11], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "execute-gate-is-not-authority", "retention GC execution")?;
    require_check(&checks, "normal-admission-still-required", "retention GC execution")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC execution")?;
    Ok(RetentionGcExecutionGate {
        execution_ref: canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        apply_ref,
        plan_ref,
        recomputed_plan_ref,
        retention_receipt_ref,
        tombstone_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn audit_retention_gc_execution(input: RetentionGcAuditInput<'_>) -> Result<RetentionGcAudit> {
    ensure_store(input.root)?;
    let execution = read_retention_gc_execution_gate(input.root, input.execution_ref)?;
    let execution_scope = gc_audit_scope(
        &execution.subsystem,
        &execution.action,
        &execution.object_ref,
        &execution.object_kind,
        &execution.retention_class,
    );
    let mut diagnostics = Vec::new();
    if execution.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            "retention-gc-audit-execution-not-pass".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
        extend_bounded(
            &mut diagnostics,
            execution.diagnostics.iter().cloned(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    }

    let mut apply_decision = "missing".to_string();
    let mut plan_ref = execution.plan_ref.clone();
    let mut plan_decision = "missing".to_string();
    if let Some(apply_ref) = execution.apply_ref.as_ref() {
        let apply = read_retention_gc_apply(input.root, apply_ref)?;
        apply_decision.clone_from(&apply.decision);
        if apply.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-apply-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if !same_gc_scope(
            &execution_scope,
            &gc_audit_scope(
                &apply.subsystem,
                &apply.action,
                &apply.object_ref,
                &apply.object_kind,
                &apply.retention_class,
            ),
        ) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-apply-scope-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if execution.plan_ref.as_deref().is_some_and(|reference| reference != apply.plan_ref) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-execution-apply-plan-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if execution
            .retention_receipt_ref
            .as_deref()
            .is_some_and(|reference| apply.retention_receipt_ref.as_deref() != Some(reference))
        {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-execution-apply-receipt-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if execution
            .tombstone_ref
            .as_deref()
            .is_some_and(|reference| apply.tombstone_ref.as_deref() != Some(reference))
        {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-execution-apply-tombstone-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        plan_ref.get_or_insert(apply.plan_ref.clone());
    } else {
        push_bounded(
            &mut diagnostics,
            "retention-gc-audit-apply-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    }

    if let Some(reference) = plan_ref.as_ref() {
        let plan = read_retention_gc_plan(input.root, reference)?;
        plan_decision.clone_from(&plan.decision);
        if plan.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-plan-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if !same_gc_scope(
            &execution_scope,
            &gc_audit_scope(&plan.subsystem, &plan.action, &plan.object_ref, &plan.object_kind, &plan.retention_class),
        ) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-plan-scope-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
    } else {
        push_bounded(
            &mut diagnostics,
            "retention-gc-audit-plan-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    }

    let mut retention_receipt_decision = "missing".to_string();
    if let Some(receipt_ref) = execution.retention_receipt_ref.as_ref() {
        let receipt = read_retention_receipt(input.root, receipt_ref)?;
        retention_receipt_decision.clone_from(&receipt.decision);
        if receipt.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-retention-receipt-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if !same_retention_scope(
            &execution_scope.retention,
            &retention_audit_scope(
                &receipt.action,
                &receipt.object_ref,
                &receipt.object_kind,
                &receipt.retention_class,
            ),
        ) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-retention-receipt-scope-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
    } else {
        push_bounded(
            &mut diagnostics,
            "retention-gc-audit-retention-receipt-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    }

    let mut tombstone_status = "missing".to_string();
    if let Some(tombstone_ref) = execution.tombstone_ref.as_ref() {
        let tombstone = read_retention_tombstone(input.root, tombstone_ref)?;
        tombstone_status = "present".to_string();
        if !same_retention_scope(
            &execution_scope.retention,
            &retention_audit_scope(
                &tombstone.action,
                &tombstone.object_ref,
                &tombstone.object_kind,
                &tombstone.retention_class,
            ),
        ) {
            push_bounded(
                &mut diagnostics,
                "retention-gc-audit-tombstone-scope-mismatch".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention GC audit diagnostics",
            )?;
        }
        if let Some(receipt_ref) = execution.retention_receipt_ref.as_ref() {
            let pending_receipt_ref = synthetic_ref("pending-retention-receipt")?;
            if tombstone.receipt_ref != *receipt_ref && tombstone.receipt_ref != pending_receipt_ref {
                push_bounded(
                    &mut diagnostics,
                    "retention-gc-audit-tombstone-receipt-mismatch".to_string(),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention GC audit diagnostics",
                )?;
            }
        }
    } else if is_destructive_action(&execution.action) {
        push_bounded(
            &mut diagnostics,
            "retention-gc-audit-tombstone-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention GC audit diagnostics",
        )?;
    } else {
        tombstone_status = "not-required".to_string();
    }

    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = retention_gc_audit_value(&RetentionGcAuditValueInput {
        decision,
        subsystem: &execution.subsystem,
        action: &execution.action,
        object_ref: &execution.object_ref,
        object_kind: &execution.object_kind,
        retention_class: &execution.retention_class,
        plan_ref: plan_ref.as_deref(),
        plan_decision: &plan_decision,
        apply_ref: execution.apply_ref.as_deref(),
        apply_decision: &apply_decision,
        execution_ref: &execution.execution_ref,
        execution_decision: &execution.decision,
        retention_receipt_ref: execution.retention_receipt_ref.as_deref(),
        retention_receipt_decision: &retention_receipt_decision,
        tombstone_ref: execution.tombstone_ref.as_deref(),
        tombstone_status: &tombstone_status,
        diagnostics: &diagnostics,
    })?;
    let audit = parse_retention_gc_audit(&value)?;
    write_store_value(&gc_audit_path(input.root, &audit.audit_ref)?, &audit.value)?;
    Ok(audit)
}

fn gc_audit_scope<'a>(
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> GcAuditScope<'a> {
    GcAuditScope {
        subsystem,
        retention: retention_audit_scope(action, object_ref, object_kind, retention_class),
    }
}

fn retention_audit_scope<'a>(
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> RetentionAuditScope<'a> {
    RetentionAuditScope {
        action,
        object_ref,
        object_kind,
        retention_class,
    }
}

fn same_gc_scope(left: &GcAuditScope<'_>, right: &GcAuditScope<'_>) -> bool {
    left.subsystem == right.subsystem && same_retention_scope(&left.retention, &right.retention)
}

fn same_retention_scope(left: &RetentionAuditScope<'_>, right: &RetentionAuditScope<'_>) -> bool {
    left.action == right.action
        && left.object_ref == right.object_ref
        && left.object_kind == right.object_kind
        && left.retention_class == right.retention_class
}

fn retention_gc_audit_value(input: &RetentionGcAuditValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_name(input.subsystem, "retention GC audit subsystem")?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention GC audit object ref")?;
    validate_name(input.object_kind, "retention GC audit object kind")?;
    validate_retention_class(input.retention_class)?;
    if let Some(plan_ref) = input.plan_ref {
        require_ref(plan_ref, "retention GC audit plan ref")?;
    }
    validate_audit_step_status(input.plan_decision, "retention GC audit plan decision")?;
    if let Some(apply_ref) = input.apply_ref {
        require_ref(apply_ref, "retention GC audit apply ref")?;
    }
    validate_audit_step_status(input.apply_decision, "retention GC audit apply decision")?;
    require_ref(input.execution_ref, "retention GC audit execution ref")?;
    validate_decision(input.execution_decision)?;
    if let Some(receipt_ref) = input.retention_receipt_ref {
        require_ref(receipt_ref, "retention GC audit receipt ref")?;
    }
    validate_audit_step_status(input.retention_receipt_decision, "retention GC audit receipt decision")?;
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention GC audit tombstone ref")?;
    }
    validate_audit_step_status(input.tombstone_status, "retention GC audit tombstone status")?;
    Ok(record("retention-gc-audit-v1", vec![
        string(RETENTION_GC_AUDIT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("mode", vec![string("audit")]),
        record("subsystem", vec![string(input.subsystem)]),
        record("action", vec![string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("plan", vec![optional_ref_value(input.plan_ref), string(input.plan_decision)]),
        record("apply", vec![optional_ref_value(input.apply_ref), string(input.apply_decision)]),
        record("execution", vec![string(input.execution_ref), string(input.execution_decision)]),
        record("retention-receipt", vec![
            optional_ref_value(input.retention_receipt_ref),
            string(input.retention_receipt_decision),
        ]),
        record("tombstone", vec![optional_ref_value(input.tombstone_ref), string(input.tombstone_status)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("audit-is-not-authority", "pass"),
            ("plan-link-bound", pass_or_deny(input.plan_ref.is_some())),
            ("apply-link-bound", pass_or_deny(input.apply_ref.is_some())),
            ("execution-link-bound", "pass"),
            ("retention-receipt-link-bound", pass_or_deny(input.retention_receipt_ref.is_some())),
            (
                "tombstone-link-bound",
                pass_or_deny(!is_destructive_action(input.action) || input.tombstone_ref.is_some()),
            ),
            ("normal-admission-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

fn validate_audit_step_status(status: &str, label: &str) -> Result<()> {
    match status {
        "pass" | "deny" | "missing" | "present" | "not-required" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported {label}: {other}"))),
    }
}

pub fn parse_retention_gc_audit(value: &IOValue) -> Result<RetentionGcAudit> {
    let fields = value
        .collect_simple_record("retention-gc-audit-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-gc-audit-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_GC_AUDIT_SCHEMA, "retention GC audit schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let mode = record_string(&fields[2], "mode")?;
    if mode != "audit" {
        return Err(MoltenError::invalid_harness("retention GC audit mode must be audit"));
    }
    let subsystem = record_string(&fields[3], "subsystem")?;
    validate_name(&subsystem, "retention GC audit subsystem")?;
    let action = record_string(&fields[4], "action")?;
    validate_action(&action)?;
    let (object_ref, object_kind) = parse_object_value(&fields[5])?;
    let retention_class = record_string(&fields[6], "class")?;
    validate_retention_class(&retention_class)?;
    let (plan_ref, plan_decision) = record_optional_ref_with_status(&fields[7], "plan")?;
    let (apply_ref, apply_decision) = record_optional_ref_with_status(&fields[8], "apply")?;
    let execution_fields = fields[9]
        .collect_simple_record("execution", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC audit execution record"))?;
    let execution_ref = required_string(&execution_fields[0], "retention GC audit execution ref")?;
    require_ref(&execution_ref, "retention GC audit execution ref")?;
    let execution_decision = required_string(&execution_fields[1], "retention GC audit execution decision")?;
    validate_decision(&execution_decision)?;
    let (retention_receipt_ref, retention_receipt_decision) =
        record_optional_ref_with_status(&fields[10], "retention-receipt")?;
    let (tombstone_ref, tombstone_status) = record_optional_ref_with_status(&fields[11], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "audit-is-not-authority", "retention GC audit")?;
    require_check(&checks, "normal-admission-still-required", "retention GC audit")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention GC audit")?;
    Ok(RetentionGcAudit {
        audit_ref: canonical_hash(value)?,
        decision,
        subsystem,
        action,
        object_ref,
        object_kind,
        retention_class,
        plan_ref,
        plan_decision,
        apply_ref,
        apply_decision,
        execution_ref,
        execution_decision,
        retention_receipt_ref,
        retention_receipt_decision,
        tombstone_ref,
        tombstone_status,
        diagnostics,
        value: value.clone(),
    })
}

pub fn read_retention_gc_audit(root: &Path, audit_ref: &str) -> Result<RetentionGcAudit> {
    require_ref(audit_ref, "retention GC audit ref")?;
    let value = read_store_value(&gc_audit_path(root, audit_ref)?)?;
    let audit = parse_retention_gc_audit(&value)?;
    if audit.audit_ref != audit_ref {
        return Err(MoltenError::invalid_harness("stored retention GC audit ref mismatch"));
    }
    Ok(audit)
}

pub fn explain_retention_candidate(input: RetentionCandidateExplainInput<'_>) -> Result<RetentionCandidateExplain> {
    validate_retention_candidate_explain_input(&input)?;
    let filter = RetentionCandidateFilter {
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    };
    let pin_refs = collect_matching_retention_refs(
        &pins_dir(input.root),
        parse_retention_pin,
        |pin| filter.matches_object(&pin.object_ref, &pin.object_kind, &pin.retention_class),
        |pin| pin.pin_ref.clone(),
        "retention candidate pins",
    )?;
    let admission_refs = collect_matching_retention_refs(
        &admissions_dir(input.root),
        parse_retention_evidence_admission,
        |admission| {
            filter.matches_retention(
                &admission.object_ref,
                &admission.object_kind,
                &admission.retention_class,
                &admission.action,
            )
        },
        |admission| admission.admission_ref.clone(),
        "retention candidate admissions",
    )?;
    let remote_clearance_refs = collect_matching_retention_refs(
        &remote_clearances_dir(input.root),
        parse_retention_remote_gc_clearance,
        |clearance| {
            filter.matches_retention(
                &clearance.object_ref,
                &clearance.object_kind,
                &clearance.retention_class,
                &clearance.action,
            )
        },
        |clearance| clearance.clearance_ref.clone(),
        "retention candidate remote clearances",
    )?;
    let remote_clearance_import_refs = collect_matching_retention_refs(
        &remote_clearance_imports_dir(input.root),
        parse_retention_remote_gc_clearance_import,
        |import| import.clearance_ref.as_ref().is_some_and(|reference| remote_clearance_refs.contains(reference)),
        |import| import.import_ref.clone(),
        "retention candidate remote clearance imports",
    )?;
    let gc_plan_refs = collect_matching_retention_refs(
        &gc_plans_dir(input.root),
        parse_retention_gc_plan,
        |plan| {
            filter.matches_gc(&plan.subsystem, &plan.object_ref, &plan.object_kind, &plan.retention_class, &plan.action)
        },
        |plan| plan.plan_ref.clone(),
        "retention candidate GC plans",
    )?;
    let gc_apply_refs = collect_matching_retention_refs(
        &gc_applies_dir(input.root),
        parse_retention_gc_apply,
        |apply| {
            filter.matches_gc(
                &apply.subsystem,
                &apply.object_ref,
                &apply.object_kind,
                &apply.retention_class,
                &apply.action,
            )
        },
        |apply| apply.apply_ref.clone(),
        "retention candidate GC applies",
    )?;
    let gc_execution_refs = collect_matching_retention_refs(
        &gc_executes_dir(input.root),
        parse_retention_gc_execution_gate,
        |execute| {
            filter.matches_gc(
                &execute.subsystem,
                &execute.object_ref,
                &execute.object_kind,
                &execute.retention_class,
                &execute.action,
            )
        },
        |execute| execute.execution_ref.clone(),
        "retention candidate GC executions",
    )?;
    let gc_audit_refs = collect_matching_retention_refs(
        &gc_audits_dir(input.root),
        parse_retention_gc_audit,
        |audit| {
            filter.matches_gc(
                &audit.subsystem,
                &audit.object_ref,
                &audit.object_kind,
                &audit.retention_class,
                &audit.action,
            )
        },
        |audit| audit.audit_ref.clone(),
        "retention candidate GC audits",
    )?;
    let retention_receipt_refs = collect_matching_retention_refs(
        &receipts_dir(input.root),
        parse_retention_receipt,
        |receipt| {
            filter.matches_retention(
                &receipt.object_ref,
                &receipt.object_kind,
                &receipt.retention_class,
                &receipt.action,
            )
        },
        |receipt| receipt.receipt_ref.clone(),
        "retention candidate receipts",
    )?;
    let tombstone_refs = collect_matching_retention_refs(
        &tombstones_dir(input.root),
        parse_tombstone,
        |tombstone| {
            filter.matches_retention(
                &tombstone.object_ref,
                &tombstone.object_kind,
                &tombstone.retention_class,
                &tombstone.action,
            )
        },
        |tombstone| tombstone.tombstone_ref.clone(),
        "retention candidate tombstones",
    )?;
    let diagnostics = retention_candidate_explain_diagnostics(&RetentionCandidateExplainValueInput {
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
        pin_refs: &pin_refs,
        admission_refs: &admission_refs,
        remote_clearance_refs: &remote_clearance_refs,
        remote_clearance_import_refs: &remote_clearance_import_refs,
        gc_plan_refs: &gc_plan_refs,
        gc_apply_refs: &gc_apply_refs,
        gc_execution_refs: &gc_execution_refs,
        gc_audit_refs: &gc_audit_refs,
        retention_receipt_refs: &retention_receipt_refs,
        tombstone_refs: &tombstone_refs,
        diagnostics: &[],
    })?;
    let value = retention_candidate_explain_value(&RetentionCandidateExplainValueInput {
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
        pin_refs: &pin_refs,
        admission_refs: &admission_refs,
        remote_clearance_refs: &remote_clearance_refs,
        remote_clearance_import_refs: &remote_clearance_import_refs,
        gc_plan_refs: &gc_plan_refs,
        gc_apply_refs: &gc_apply_refs,
        gc_execution_refs: &gc_execution_refs,
        gc_audit_refs: &gc_audit_refs,
        retention_receipt_refs: &retention_receipt_refs,
        tombstone_refs: &tombstone_refs,
        diagnostics: &diagnostics,
    })?;
    parse_retention_candidate_explain(&value)
}

fn retention_candidate_explain_diagnostics(input: &RetentionCandidateExplainValueInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.pin_refs.is_empty()
        && input.admission_refs.is_empty()
        && input.remote_clearance_refs.is_empty()
        && input.remote_clearance_import_refs.is_empty()
        && input.gc_plan_refs.is_empty()
        && input.gc_apply_refs.is_empty()
        && input.gc_execution_refs.is_empty()
        && input.gc_audit_refs.is_empty()
        && input.retention_receipt_refs.is_empty()
        && input.tombstone_refs.is_empty()
    {
        push_bounded(
            &mut diagnostics,
            "retention-candidate-no-known-evidence".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention candidate explain diagnostics",
        )?;
    }
    if !input.pin_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "active-pins-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention candidate explain diagnostics",
        )?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    Ok(diagnostics)
}

fn retention_candidate_explain_value(input: &RetentionCandidateExplainValueInput<'_>) -> Result<IOValue> {
    validate_retention_candidate_explain_value_input(input)?;
    Ok(record("retention-candidate-explain-v1", vec![
        string(RETENTION_CANDIDATE_EXPLAIN_SCHEMA),
        record("object", vec![string(input.object_ref), optional_string_value(input.object_kind)]),
        record("filters", vec![
            record("class", vec![optional_string_value(input.retention_class)]),
            record("action", vec![optional_string_value(input.action)]),
            record("subsystem", vec![optional_string_value(input.subsystem)]),
        ]),
        record("pins", vec![strings_sequence(input.pin_refs)]),
        record("admissions", vec![strings_sequence(input.admission_refs)]),
        record("remote-clearances", vec![strings_sequence(input.remote_clearance_refs)]),
        record("remote-clearance-imports", vec![strings_sequence(input.remote_clearance_import_refs)]),
        record("gc-plans", vec![strings_sequence(input.gc_plan_refs)]),
        record("gc-applies", vec![strings_sequence(input.gc_apply_refs)]),
        record("gc-executes", vec![strings_sequence(input.gc_execution_refs)]),
        record("gc-audits", vec![strings_sequence(input.gc_audit_refs)]),
        record("retention-receipts", vec![strings_sequence(input.retention_receipt_refs)]),
        record("tombstones", vec![strings_sequence(input.tombstone_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("read-only-explain", "pass"),
            ("catalog-discovery-only", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_candidate_explain(value: &IOValue) -> Result<RetentionCandidateExplain> {
    let fields = value
        .collect_simple_record("retention-candidate-explain-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-explain-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_CANDIDATE_EXPLAIN_SCHEMA, "retention candidate explain schema")?;
    let object_fields = fields[1]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate object"))?;
    let object_ref = required_string(&object_fields[0], "retention candidate object ref")?;
    require_ref(&object_ref, "retention candidate object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention candidate object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention candidate object kind")?;
    }
    let filter_fields = fields[2]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate filters"))?;
    let retention_class = record_optional_string(&filter_fields[0], "class")?;
    if let Some(retention_class) = retention_class.as_deref() {
        validate_retention_class(retention_class)?;
    }
    let action = record_optional_string(&filter_fields[1], "action")?;
    if let Some(action) = action.as_deref() {
        validate_action(action)?;
    }
    let subsystem = record_optional_string(&filter_fields[2], "subsystem")?;
    if let Some(subsystem) = subsystem.as_deref() {
        validate_name(subsystem, "retention candidate subsystem")?;
    }
    let pin_refs = record_ref_sequence(&fields[3], "pins")?;
    let admission_refs = record_ref_sequence(&fields[4], "admissions")?;
    let remote_clearance_refs = record_ref_sequence(&fields[5], "remote-clearances")?;
    let remote_clearance_import_refs = record_ref_sequence(&fields[6], "remote-clearance-imports")?;
    let gc_plan_refs = record_ref_sequence(&fields[7], "gc-plans")?;
    let gc_apply_refs = record_ref_sequence(&fields[8], "gc-applies")?;
    let gc_execution_refs = record_ref_sequence(&fields[9], "gc-executes")?;
    let gc_audit_refs = record_ref_sequence(&fields[10], "gc-audits")?;
    let retention_receipt_refs = record_ref_sequence(&fields[11], "retention-receipts")?;
    let tombstone_refs = record_ref_sequence(&fields[12], "tombstones")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "read-only-explain", "retention candidate explain")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate explain")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate explain")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate explain")?;
    Ok(RetentionCandidateExplain {
        explain_ref: canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        pin_refs,
        admission_refs,
        remote_clearance_refs,
        remote_clearance_import_refs,
        gc_plan_refs,
        gc_apply_refs,
        gc_execution_refs,
        gc_audit_refs,
        retention_receipt_refs,
        tombstone_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn export_retention_candidate_bundle(
    input: RetentionCandidateBundleExportInput<'_>,
) -> Result<RetentionCandidateBundle> {
    let explain = parse_retention_candidate_explain(input.explain_value)?;
    fs::create_dir_all(input.out).map_err(MoltenError::from)?;
    let artifact_dir = input.out.join("artifacts");
    fs::create_dir_all(&artifact_dir).map_err(MoltenError::from)?;
    write_store_value(&input.out.join("explain.preserves"), &explain.value)?;
    let mut artifact_refs = Vec::new();
    let mut diagnostics = Vec::new();
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "gc-plans",
            refs: &explain.gc_plan_refs,
            read: read_retention_gc_plan_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "gc-applies",
            refs: &explain.gc_apply_refs,
            read: read_retention_gc_apply_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "gc-executes",
            refs: &explain.gc_execution_refs,
            read: read_retention_gc_execution_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "gc-audits",
            refs: &explain.gc_audit_refs,
            read: read_retention_gc_audit_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "receipts",
            refs: &explain.retention_receipt_refs,
            read: read_retention_receipt_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    export_retention_bundle_artifact_group(
        RetentionBundleArtifactGroupInput {
            root: input.root,
            bundle_dir: &artifact_dir,
            dir_name: "tombstones",
            refs: &explain.tombstone_refs,
            read: read_retention_tombstone_value,
        },
        &mut artifact_refs,
        &mut diagnostics,
    )?;
    artifact_refs.sort();
    artifact_refs.dedup();
    diagnostics.sort();
    diagnostics.dedup();
    let value = retention_candidate_bundle_value(&RetentionCandidateBundleValueInput {
        explain: &explain,
        artifact_refs: &artifact_refs,
        diagnostics: &diagnostics,
    })?;
    write_store_value(&input.out.join("bundle.preserves"), &value)?;
    let bundle = parse_retention_candidate_bundle(&value)?;
    let profile = profile_retention_candidate_bundle(input.out, input.profile, &bundle)?;
    write_store_value(&input.out.join(BUNDLE_PROFILE_FILE), &profile.value)?;
    if input.profile == RetentionCandidateBundleExportProfile::Diagnostic {
        write_retention_candidate_bundle_redacted_view(input.out, &bundle)?;
    }
    Ok(bundle)
}

fn export_retention_bundle_artifact_group(
    input: RetentionBundleArtifactGroupInput<'_>,
    artifact_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let group_dir = input.bundle_dir.join(input.dir_name);
    fs::create_dir_all(&group_dir).map_err(MoltenError::from)?;
    for reference in input.refs {
        match (input.read)(input.root, reference) {
            Ok(value) => {
                write_store_value(&group_dir.join(format!("{}.preserves", ref_file_name(reference)?)), &value)?;
                push_bounded(artifact_refs, reference.clone(), MAX_RETENTION_REFS, "retention bundle artifact refs")?;
            }
            Err(_) => push_bounded(
                diagnostics,
                format!("retention-bundle-missing-artifact:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle diagnostics",
            )?,
        }
    }
    Ok(())
}

fn profile_retention_candidate_bundle(
    bundle_dir: &Path,
    profile: RetentionCandidateBundleExportProfile,
    bundle: &RetentionCandidateBundle,
) -> Result<RetentionCandidateBundleProfile> {
    let mut marker_refs = Vec::new();
    let mut diagnostics = Vec::new();
    if profile != RetentionCandidateBundleExportProfile::Internal {
        collect_retention_bundle_sensitive_markers(&bundle.value, "/bundle", &bundle.bundle_ref, &mut marker_refs)?;
        let explain_value = read_store_value(&bundle_dir.join("explain.preserves"))?;
        collect_retention_bundle_sensitive_markers(&explain_value, "/explain", &bundle.bundle_ref, &mut marker_refs)?;
        collect_retention_bundle_artifact_sensitive_markers(bundle_dir, &bundle.bundle_ref, &mut marker_refs)?;
        marker_refs.sort();
        marker_refs.dedup();
    }
    match profile {
        RetentionCandidateBundleExportProfile::Internal => {}
        RetentionCandidateBundleExportProfile::Public => {
            if !marker_refs.is_empty() {
                push_bounded(
                    &mut diagnostics,
                    format!("retention-bundle-public-sensitive-markers:{}", marker_refs.len()),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention bundle profile diagnostics",
                )?;
            }
        }
        RetentionCandidateBundleExportProfile::Diagnostic => {
            push_bounded(
                &mut diagnostics,
                format!("retention-bundle-diagnostic-redacted-markers:{}", marker_refs.len()),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle profile diagnostics",
            )?;
        }
    }
    let decision = if profile == RetentionCandidateBundleExportProfile::Public && !marker_refs.is_empty() {
        "deny"
    } else {
        "pass"
    };
    let value = retention_candidate_bundle_profile_value(&RetentionCandidateBundleProfileValueInput {
        profile,
        decision,
        bundle_ref: &bundle.bundle_ref,
        marker_refs: &marker_refs,
        diagnostics: &diagnostics,
    })?;
    parse_retention_candidate_bundle_profile(&value)
}

fn collect_retention_bundle_artifact_sensitive_markers(
    bundle_dir: &Path,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<()> {
    let artifact_dir = bundle_dir.join("artifacts");
    if !artifact_dir.exists() {
        return Ok(());
    }
    for dir_name in retention_bundle_artifact_dirs() {
        let group_dir = artifact_dir.join(dir_name);
        if !group_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&group_dir).map_err(MoltenError::from)? {
            let entry = entry.map_err(MoltenError::from)?;
            if !entry.file_type().map_err(MoltenError::from)?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("preserves") {
                continue;
            }
            let value = read_store_value(&path)?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            collect_retention_bundle_sensitive_markers(
                &value,
                &format!("/artifacts/{dir_name}/{file_name}"),
                bundle_ref,
                marker_refs,
            )?;
        }
    }
    Ok(())
}

fn collect_retention_bundle_sensitive_markers(
    value: &IOValue,
    path: &str,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<()> {
    let mut stack = Vec::new();
    push_bounded(
        &mut stack,
        (value.clone(), path.to_string()),
        MAX_RETENTION_REFS,
        "retention bundle marker scan stack",
    )?;
    while let Some((current, current_path)) = stack.pop() {
        if let Some(label) = record_label_string(&current)
            && is_sensitive_retention_bundle_token(&label)
        {
            push_bounded(
                marker_refs,
                retention_bundle_marker_ref(bundle_ref, &current_path, &label)?,
                MAX_RETENTION_REFS,
                "retention bundle profile markers",
            )?;
        }
        if let Some(text) = current.as_string()
            && is_sensitive_retention_bundle_token(&text)
        {
            push_bounded(
                marker_refs,
                retention_bundle_marker_ref(bundle_ref, &current_path, &text)?,
                MAX_RETENTION_REFS,
                "retention bundle profile markers",
            )?;
        }
        if matches!(
            current.value_class(),
            ValueClass::Compound(CompoundClass::Record) | ValueClass::Compound(CompoundClass::Sequence)
        ) {
            let mut children = Vec::new();
            for (index, child) in current.iter().enumerate() {
                push_bounded(
                    &mut children,
                    (index, value_to_iovalue(&child)),
                    MAX_RETENTION_REFS,
                    "retention bundle marker scan children",
                )?;
            }
            for (index, child) in children.into_iter().rev() {
                push_bounded(
                    &mut stack,
                    (child, format!("{current_path}/{index}")),
                    MAX_RETENTION_REFS,
                    "retention bundle marker scan stack",
                )?;
            }
        }
    }
    Ok(())
}

fn write_retention_candidate_bundle_redacted_view(bundle_dir: &Path, bundle: &RetentionCandidateBundle) -> Result<()> {
    let redacted_dir = bundle_dir.join(BUNDLE_REDACTED_DIR);
    let mut ignored_markers = Vec::new();
    let bundle_value = read_store_value(&bundle_dir.join("bundle.preserves"))?;
    let redacted_bundle =
        redacted_retention_bundle_value(&bundle_value, "/bundle", &bundle.bundle_ref, &mut ignored_markers)?;
    write_store_value(&redacted_dir.join("bundle.preserves"), &redacted_bundle)?;
    let explain_value = read_store_value(&bundle_dir.join("explain.preserves"))?;
    let redacted_explain =
        redacted_retention_bundle_value(&explain_value, "/explain", &bundle.bundle_ref, &mut ignored_markers)?;
    write_store_value(&redacted_dir.join("explain.preserves"), &redacted_explain)?;
    let artifact_dir = bundle_dir.join("artifacts");
    for dir_name in retention_bundle_artifact_dirs() {
        let group_dir = artifact_dir.join(dir_name);
        if !group_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&group_dir).map_err(MoltenError::from)? {
            let entry = entry.map_err(MoltenError::from)?;
            if !entry.file_type().map_err(MoltenError::from)?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("preserves") {
                continue;
            }
            let value = read_store_value(&path)?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let redacted = redacted_retention_bundle_value(
                &value,
                &format!("/artifacts/{dir_name}/{file_name}"),
                &bundle.bundle_ref,
                &mut ignored_markers,
            )?;
            write_store_value(&redacted_dir.join("artifacts").join(dir_name).join(file_name), &redacted)?;
        }
    }
    Ok(())
}

fn redacted_retention_bundle_value(
    value: &IOValue,
    path: &str,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<IOValue> {
    enum RedactionFrame {
        Visit { value: IOValue, path: String },
        BuildRecord { label: IOValue, child_count: usize },
        BuildSequence { child_count: usize },
    }

    let mut frames = Vec::new();
    push_bounded(
        &mut frames,
        RedactionFrame::Visit {
            value: value.clone(),
            path: path.to_string(),
        },
        MAX_RETENTION_REFS,
        "retention bundle redaction stack",
    )?;
    let mut results = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            RedactionFrame::Visit {
                value: current,
                path: current_path,
            } => {
                if let Some(label) = record_label_string(&current)
                    && is_sensitive_retention_bundle_token(&label)
                {
                    let marker_ref = retention_bundle_marker_ref(bundle_ref, &current_path, &label)?;
                    push_bounded(
                        marker_refs,
                        marker_ref.clone(),
                        MAX_RETENTION_REFS,
                        "retention bundle profile markers",
                    )?;
                    push_bounded(
                        &mut results,
                        record("retention-bundle-redaction-marker", vec![string(&marker_ref)]),
                        MAX_RETENTION_REFS,
                        "retention bundle redacted values",
                    )?;
                    continue;
                }
                if let Some(text) = current.as_string()
                    && is_sensitive_retention_bundle_token(&text)
                {
                    let marker_ref = retention_bundle_marker_ref(bundle_ref, &current_path, &text)?;
                    push_bounded(
                        marker_refs,
                        marker_ref.clone(),
                        MAX_RETENTION_REFS,
                        "retention bundle profile markers",
                    )?;
                    push_bounded(
                        &mut results,
                        record("retention-bundle-redaction-marker", vec![string(&marker_ref)]),
                        MAX_RETENTION_REFS,
                        "retention bundle redacted values",
                    )?;
                    continue;
                }
                match current.value_class() {
                    ValueClass::Atomic(_) | ValueClass::Embedded => {
                        push_bounded(&mut results, current, MAX_RETENTION_REFS, "retention bundle redacted values")?
                    }
                    ValueClass::Compound(CompoundClass::Record) => {
                        let label = value_to_iovalue(&current.label());
                        let mut children = Vec::new();
                        for (index, child) in current.iter().enumerate() {
                            push_bounded(
                                &mut children,
                                (index, value_to_iovalue(&child)),
                                MAX_RETENTION_REFS,
                                "retention bundle redaction children",
                            )?;
                        }
                        let child_count = children.len();
                        push_bounded(
                            &mut frames,
                            RedactionFrame::BuildRecord { label, child_count },
                            MAX_RETENTION_REFS,
                            "retention bundle redaction stack",
                        )?;
                        for (index, child) in children.into_iter().rev() {
                            push_bounded(
                                &mut frames,
                                RedactionFrame::Visit {
                                    value: child,
                                    path: format!("{current_path}/{index}"),
                                },
                                MAX_RETENTION_REFS,
                                "retention bundle redaction stack",
                            )?;
                        }
                    }
                    ValueClass::Compound(CompoundClass::Sequence) => {
                        let mut children = Vec::new();
                        for (index, child) in current.iter().enumerate() {
                            push_bounded(
                                &mut children,
                                (index, value_to_iovalue(&child)),
                                MAX_RETENTION_REFS,
                                "retention bundle redaction children",
                            )?;
                        }
                        let child_count = children.len();
                        push_bounded(
                            &mut frames,
                            RedactionFrame::BuildSequence { child_count },
                            MAX_RETENTION_REFS,
                            "retention bundle redaction stack",
                        )?;
                        for (index, child) in children.into_iter().rev() {
                            push_bounded(
                                &mut frames,
                                RedactionFrame::Visit {
                                    value: child,
                                    path: format!("{current_path}/{index}"),
                                },
                                MAX_RETENTION_REFS,
                                "retention bundle redaction stack",
                            )?;
                        }
                    }
                    ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                        push_bounded(&mut results, current, MAX_RETENTION_REFS, "retention bundle redacted values")?;
                    }
                }
            }
            RedactionFrame::BuildRecord { label, child_count } => {
                let start = results
                    .len()
                    .checked_sub(child_count)
                    .ok_or_else(|| MoltenError::invalid_harness("retention bundle redaction record stack underflow"))?;
                let fields = results.split_off(start);
                push_bounded(
                    &mut results,
                    IOValue::record(label, fields),
                    MAX_RETENTION_REFS,
                    "retention bundle redacted values",
                )?;
            }
            RedactionFrame::BuildSequence { child_count } => {
                let start = results.len().checked_sub(child_count).ok_or_else(|| {
                    MoltenError::invalid_harness("retention bundle redaction sequence stack underflow")
                })?;
                let values = results.split_off(start);
                push_bounded(&mut results, sequence(values), MAX_RETENTION_REFS, "retention bundle redacted values")?;
            }
        }
    }
    if results.len() != 1 {
        return Err(MoltenError::invalid_harness("retention bundle redaction result stack mismatch"));
    }
    results
        .pop()
        .ok_or_else(|| MoltenError::invalid_harness("retention bundle redaction produced no result"))
}

fn retention_bundle_marker_ref(bundle_ref: &str, path: &str, token: &str) -> Result<String> {
    canonical_hash(&record("retention-bundle-sensitive-marker", vec![string(bundle_ref), string(path), string(token)]))
}

fn is_sensitive_retention_bundle_token(value: &str) -> bool {
    matches!(
        value,
        "secret"
            | "confidential"
            | "credential"
            | "private"
            | "encrypted-ref"
            | "secret-ref-v1"
            | "encrypted-ref-v1"
            | CLASS_PRIVATE_SECRET_REF
    )
}

fn record_label_string(value: &IOValue) -> Option<String> {
    if !value.is_record() {
        return None;
    }
    value.label().as_symbol().map(Cow::into_owned)
}

fn retention_candidate_bundle_profile_value(input: &RetentionCandidateBundleProfileValueInput<'_>) -> Result<IOValue> {
    validate_retention_candidate_bundle_profile_value_input(input)?;
    Ok(record("retention-candidate-bundle-profile-v1", vec![
        string(RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA),
        record("profile", vec![string(input.profile.as_str())]),
        record("loss-classification", vec![string(input.profile.loss_classification())]),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("markers", vec![strings_sequence(input.marker_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("profile-is-not-authority", "pass"),
            ("read-only-profile", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_candidate_bundle_profile(value: &IOValue) -> Result<RetentionCandidateBundleProfile> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-profile-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-profile-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA, "retention candidate bundle profile schema")?;
    let profile = record_string(&fields[1], "profile")?;
    let parsed_profile = RetentionCandidateBundleExportProfile::parse(&profile)?;
    let loss_classification = record_string(&fields[2], "loss-classification")?;
    if loss_classification != parsed_profile.loss_classification() {
        return Err(MoltenError::invalid_harness("retention bundle profile loss classification mismatch"));
    }
    let decision = record_string(&fields[3], "decision")?;
    validate_decision(&decision)?;
    let bundle_ref = record_ref(&fields[4], "bundle")?;
    let marker_refs = record_ref_sequence(&fields[5], "markers")?;
    let diagnostics = record_string_sequence(&fields[6], "diagnostics")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "profile-is-not-authority", "retention candidate bundle profile")?;
    require_check(&checks, "read-only-profile", "retention candidate bundle profile")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle profile")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle profile")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle profile")?;
    Ok(RetentionCandidateBundleProfile {
        profile_ref: canonical_hash(value)?,
        decision,
        profile,
        loss_classification,
        bundle_ref,
        marker_refs,
        diagnostics,
        value: value.clone(),
    })
}

fn validate_retention_candidate_bundle_profile_value_input(
    input: &RetentionCandidateBundleProfileValueInput<'_>,
) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(input.bundle_ref, "retention bundle profile bundle ref")?;
    validate_refs(input.marker_refs, "retention bundle profile marker ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle profile diagnostics")
}

fn retention_candidate_bundle_value(input: &RetentionCandidateBundleValueInput<'_>) -> Result<IOValue> {
    validate_retention_candidate_bundle_value_input(input)?;
    Ok(record("retention-candidate-bundle-v1", vec![
        string(RETENTION_CANDIDATE_BUNDLE_SCHEMA),
        record("explain", vec![string(&input.explain.explain_ref)]),
        record("object", vec![
            string(&input.explain.object_ref),
            optional_string_value(input.explain.object_kind.as_deref()),
        ]),
        record("filters", vec![
            record("class", vec![optional_string_value(input.explain.retention_class.as_deref())]),
            record("action", vec![optional_string_value(input.explain.action.as_deref())]),
            record("subsystem", vec![optional_string_value(input.explain.subsystem.as_deref())]),
        ]),
        record("gc-plans", vec![strings_sequence(&input.explain.gc_plan_refs)]),
        record("gc-applies", vec![strings_sequence(&input.explain.gc_apply_refs)]),
        record("gc-executes", vec![strings_sequence(&input.explain.gc_execution_refs)]),
        record("gc-audits", vec![strings_sequence(&input.explain.gc_audit_refs)]),
        record("retention-receipts", vec![strings_sequence(&input.explain.retention_receipt_refs)]),
        record("tombstones", vec![strings_sequence(&input.explain.tombstone_refs)]),
        record("artifacts", vec![strings_sequence(input.artifact_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("bundle-is-not-authority", "pass"),
            ("read-only-export", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_candidate_bundle(value: &IOValue) -> Result<RetentionCandidateBundle> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_CANDIDATE_BUNDLE_SCHEMA, "retention candidate bundle schema")?;
    let explain_ref = record_ref(&fields[1], "explain")?;
    let object_fields = fields[2]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention candidate bundle object"))?;
    let object_ref = required_string(&object_fields[0], "retention bundle object ref")?;
    require_ref(&object_ref, "retention bundle object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention bundle object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention bundle object kind")?;
    }
    let filter_fields = fields[3]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle filters"))?;
    let retention_class = record_optional_string(&filter_fields[0], "class")?;
    if let Some(retention_class) = retention_class.as_deref() {
        validate_retention_class(retention_class)?;
    }
    let action = record_optional_string(&filter_fields[1], "action")?;
    if let Some(action) = action.as_deref() {
        validate_action(action)?;
    }
    let subsystem = record_optional_string(&filter_fields[2], "subsystem")?;
    if let Some(subsystem) = subsystem.as_deref() {
        validate_name(subsystem, "retention bundle subsystem")?;
    }
    let gc_plan_refs = record_ref_sequence(&fields[4], "gc-plans")?;
    let gc_apply_refs = record_ref_sequence(&fields[5], "gc-applies")?;
    let gc_execution_refs = record_ref_sequence(&fields[6], "gc-executes")?;
    let gc_audit_refs = record_ref_sequence(&fields[7], "gc-audits")?;
    let retention_receipt_refs = record_ref_sequence(&fields[8], "retention-receipts")?;
    let tombstone_refs = record_ref_sequence(&fields[9], "tombstones")?;
    let artifact_refs = record_ref_sequence(&fields[10], "artifacts")?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "bundle-is-not-authority", "retention candidate bundle")?;
    require_check(&checks, "read-only-export", "retention candidate bundle")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle")?;
    Ok(RetentionCandidateBundle {
        bundle_ref: canonical_hash(value)?,
        explain_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        gc_plan_refs,
        gc_apply_refs,
        gc_execution_refs,
        gc_audit_refs,
        retention_receipt_refs,
        tombstone_refs,
        artifact_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn verify_retention_candidate_bundle(
    input: RetentionCandidateBundleVerifyInput<'_>,
) -> Result<RetentionCandidateBundleVerify> {
    let bundle_value = read_store_value(&input.bundle_dir.join("bundle.preserves"))?;
    let bundle = parse_retention_candidate_bundle(&bundle_value)?;
    let explain_value = read_store_value(&input.bundle_dir.join("explain.preserves"))?;
    let explain = parse_retention_candidate_explain(&explain_value)?;
    let mut diagnostics = Vec::new();
    push_retention_bundle_scope_diagnostics(&bundle, &explain, &mut diagnostics)?;
    let expected_refs = retention_candidate_bundle_expected_refs(&bundle)?;
    let expected_ref_set = push_expected_ref_notes(&bundle, &expected_refs, &mut diagnostics)?;
    let mut file_refs = Vec::new();
    scan_retention_bundle_artifact_files(
        &input.bundle_dir.join("artifacts"),
        &expected_ref_set,
        &mut file_refs,
        &mut diagnostics,
    )?;
    verify_artifact_groups(input.bundle_dir, &bundle, &mut diagnostics)?;
    file_refs.sort();
    diagnostics.sort();
    diagnostics.dedup();
    push_file_ref_notes(&bundle, &file_refs, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = retention_candidate_bundle_verify_value(&RetentionCandidateBundleVerifyValueInput {
        bundle: &bundle,
        decision,
        file_refs: &file_refs,
        diagnostics: &diagnostics,
    })?;
    parse_retention_candidate_bundle_verify(&value)
}

fn push_expected_ref_notes(
    bundle: &RetentionCandidateBundle,
    expected_refs: &[String],
    diagnostics: &mut impl VecSink<String>,
) -> Result<BTreeSet<String>> {
    push_duplicate_ref_diagnostics(&bundle.artifact_refs, "retention-bundle-duplicate-manifest-ref", diagnostics)?;
    push_duplicate_ref_diagnostics(expected_refs, "retention-bundle-duplicate-expected-ref", diagnostics)?;
    let manifest_refs = ref_set(&bundle.artifact_refs);
    let expected_ref_set = ref_set(expected_refs);
    for reference in expected_refs {
        if !manifest_refs.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-manifest-missing-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    for reference in &bundle.artifact_refs {
        if !expected_ref_set.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-manifest-unreferenced-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(expected_ref_set)
}

fn verify_artifact_groups(
    bundle_dir: &Path,
    bundle: &RetentionCandidateBundle,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let groups = [
        Group {
            dir_name: "gc-plans",
            refs: &bundle.gc_plan_refs,
            parse: parse_retention_gc_plan_kind,
        },
        Group {
            dir_name: "gc-applies",
            refs: &bundle.gc_apply_refs,
            parse: parse_retention_gc_apply_kind,
        },
        Group {
            dir_name: "gc-executes",
            refs: &bundle.gc_execution_refs,
            parse: parse_retention_gc_execution_kind,
        },
        Group {
            dir_name: "gc-audits",
            refs: &bundle.gc_audit_refs,
            parse: parse_retention_gc_audit_kind,
        },
        Group {
            dir_name: "receipts",
            refs: &bundle.retention_receipt_refs,
            parse: parse_retention_receipt_kind,
        },
        Group {
            dir_name: "tombstones",
            refs: &bundle.tombstone_refs,
            parse: parse_retention_tombstone_kind,
        },
    ];
    for group in groups {
        verify_retention_bundle_artifact_group(
            RetentionBundleVerifyGroupInput {
                bundle_dir,
                dir_name: group.dir_name,
                refs: group.refs,
                parse: group.parse,
            },
            diagnostics,
        )?;
    }
    Ok(())
}

fn push_file_ref_notes(
    bundle: &RetentionCandidateBundle,
    file_refs: &[String],
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let file_ref_set = ref_set(file_refs);
    let manifest_refs = ref_set(&bundle.artifact_refs);
    for reference in &bundle.artifact_refs {
        if !file_ref_set.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-listed-ref-missing-file:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    for reference in file_refs {
        if !manifest_refs.contains(reference) {
            push_bounded(
                diagnostics,
                format!("retention-bundle-unlisted-file-ref:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(())
}

struct MismatchNote {
    is_same: bool,
    note: &'static str,
}

impl MismatchNote {
    fn new(is_same: bool, note: &'static str) -> Self {
        Self { is_same, note }
    }
}

fn push_mismatch_notes(checks: &[MismatchNote], diagnostics: &mut impl VecSink<String>) -> Result<()> {
    for check in checks {
        if check.is_same {
            continue;
        }
        push_bounded(
            diagnostics,
            check.note.to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
    }
    Ok(())
}

fn push_retention_bundle_scope_diagnostics(
    bundle: &RetentionCandidateBundle,
    explain: &RetentionCandidateExplain,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let checks = [
        MismatchNote::new(bundle.explain_ref == explain.explain_ref, "retention-bundle-explain-ref-mismatch"),
        MismatchNote::new(bundle.object_ref == explain.object_ref, "retention-bundle-object-mismatch"),
        MismatchNote::new(bundle.object_kind == explain.object_kind, "retention-bundle-kind-mismatch"),
        MismatchNote::new(bundle.retention_class == explain.retention_class, "retention-bundle-class-mismatch"),
        MismatchNote::new(bundle.action == explain.action, "retention-bundle-action-mismatch"),
        MismatchNote::new(bundle.subsystem == explain.subsystem, "retention-bundle-subsystem-mismatch"),
        MismatchNote::new(bundle.gc_plan_refs == explain.gc_plan_refs, "retention-bundle-plan-refs-mismatch"),
        MismatchNote::new(bundle.gc_apply_refs == explain.gc_apply_refs, "retention-bundle-apply-refs-mismatch"),
        MismatchNote::new(
            bundle.gc_execution_refs == explain.gc_execution_refs,
            "retention-bundle-execute-refs-mismatch",
        ),
        MismatchNote::new(bundle.gc_audit_refs == explain.gc_audit_refs, "retention-bundle-audit-refs-mismatch"),
        MismatchNote::new(
            bundle.retention_receipt_refs == explain.retention_receipt_refs,
            "retention-bundle-receipt-refs-mismatch",
        ),
        MismatchNote::new(bundle.tombstone_refs == explain.tombstone_refs, "retention-bundle-tombstone-refs-mismatch"),
    ];
    push_mismatch_notes(&checks, diagnostics)
}

fn retention_candidate_bundle_expected_refs(bundle: &RetentionCandidateBundle) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_ref_slice(&mut refs, &bundle.gc_plan_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_apply_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_execution_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_audit_refs)?;
    push_ref_slice(&mut refs, &bundle.retention_receipt_refs)?;
    push_ref_slice(&mut refs, &bundle.tombstone_refs)?;
    Ok(refs)
}

fn push_ref_slice(values: &mut impl VecSink<String>, refs: &[String]) -> Result<()> {
    for reference in refs {
        push_bounded(values, reference.clone(), MAX_RETENTION_REFS, "retention bundle expected refs")?;
    }
    Ok(())
}

fn push_duplicate_ref_diagnostics(refs: &[String], prefix: &str, diagnostics: &mut impl VecSink<String>) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for reference in refs {
        if !seen.insert(reference.clone()) {
            duplicates.insert(reference.clone());
        }
    }
    for reference in duplicates {
        push_bounded(
            diagnostics,
            format!("{prefix}:{reference}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
    }
    Ok(())
}

fn ref_set(refs: &[String]) -> BTreeSet<String> {
    refs.iter().cloned().collect()
}

fn scan_retention_bundle_artifact_files(
    artifact_dir: &Path,
    expected_refs: &BTreeSet<String>,
    file_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    if !artifact_dir.exists() {
        push_bounded(
            diagnostics,
            "retention-bundle-artifacts-dir-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention bundle verify diagnostics",
        )?;
        return Ok(());
    }
    let mut seen_files = BTreeSet::new();
    for entry in fs::read_dir(artifact_dir).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        let file_type = entry.file_type().map_err(MoltenError::from)?;
        if !file_type.is_dir() {
            push_bounded(
                diagnostics,
                "retention-bundle-unexpected-artifact-root-entry".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        if !retention_bundle_artifact_dirs().contains(&dir_name.as_str()) {
            push_bounded(
                diagnostics,
                "retention-bundle-unexpected-artifact-dir".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        scan_retention_bundle_artifact_group_files(
            RetentionBundleArtifactGroupScanInput {
                group_dir: &entry.path(),
                dir_name: &dir_name,
                expected_refs,
            },
            file_refs,
            diagnostics,
            &mut seen_files,
        )?;
    }
    Ok(())
}

fn scan_retention_bundle_artifact_group_files(
    input: RetentionBundleArtifactGroupScanInput<'_>,
    file_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
    seen_files: &mut BTreeSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(input.group_dir).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        let file_type = entry.file_type().map_err(MoltenError::from)?;
        if !file_type.is_file()
            || entry.path().extension().and_then(|extension| extension.to_str()) != Some("preserves")
        {
            push_bounded(
                diagnostics,
                format!("retention-bundle-unexpected-artifact-entry:{}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        match read_store_value(&entry.path()) {
            Ok(value) => {
                let actual_ref = canonical_hash(&value)?;
                if !seen_files.insert(actual_ref.clone()) {
                    push_bounded(
                        diagnostics,
                        format!("retention-bundle-duplicate-file-ref:{actual_ref}"),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention bundle verify diagnostics",
                    )?;
                }
                if !input.expected_refs.contains(&actual_ref) {
                    push_bounded(
                        diagnostics,
                        format!("retention-bundle-unreferenced-file:{}:{actual_ref}", input.dir_name),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention bundle verify diagnostics",
                    )?;
                }
                push_bounded(file_refs, actual_ref, MAX_RETENTION_REFS, "retention bundle file refs")?;
            }
            Err(_) => push_bounded(
                diagnostics,
                format!("retention-bundle-unreadable-file:{}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?,
        }
    }
    Ok(())
}

fn verify_retention_bundle_artifact_group(
    input: RetentionBundleVerifyGroupInput<'_>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    for reference in input.refs {
        let path = input
            .bundle_dir
            .join("artifacts")
            .join(input.dir_name)
            .join(format!("{}.preserves", ref_file_name(reference)?));
        if !path.exists() {
            push_bounded(
                diagnostics,
                format!("retention-bundle-missing-file:{}:{reference}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        let value = match read_store_value(&path) {
            Ok(value) => value,
            Err(_) => {
                push_bounded(
                    diagnostics,
                    format!("retention-bundle-unreadable-file:{}", input.dir_name),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention bundle verify diagnostics",
                )?;
                continue;
            }
        };
        let actual_ref = canonical_hash(&value)?;
        if &actual_ref != reference {
            push_bounded(
                diagnostics,
                format!("retention-bundle-tampered-file:{}:{reference}:{actual_ref}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
            continue;
        }
        if (input.parse)(&value).is_err() {
            push_bounded(
                diagnostics,
                format!("retention-bundle-kind-mismatch:{}:{reference}", input.dir_name),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle verify diagnostics",
            )?;
        }
    }
    Ok(())
}

fn retention_bundle_artifact_dirs() -> &'static [&'static str] {
    &[
        "gc-plans",
        "gc-applies",
        "gc-executes",
        "gc-audits",
        "receipts",
        "tombstones",
    ]
}

fn retention_candidate_bundle_verify_value(input: &RetentionCandidateBundleVerifyValueInput<'_>) -> Result<IOValue> {
    validate_retention_candidate_bundle_verify_value_input(input)?;
    Ok(record("retention-candidate-bundle-verify-v1", vec![
        string(RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(&input.bundle.bundle_ref)]),
        record("explain", vec![string(&input.bundle.explain_ref)]),
        record("object", vec![
            string(&input.bundle.object_ref),
            optional_string_value(input.bundle.object_kind.as_deref()),
        ]),
        record("filters", vec![
            record("class", vec![optional_string_value(input.bundle.retention_class.as_deref())]),
            record("action", vec![optional_string_value(input.bundle.action.as_deref())]),
            record("subsystem", vec![optional_string_value(input.bundle.subsystem.as_deref())]),
        ]),
        record("artifacts", vec![strings_sequence(&input.bundle.artifact_refs)]),
        record("files", vec![strings_sequence(input.file_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("verify-is-not-authority", "pass"),
            ("read-only-verify", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_retention_candidate_bundle_verify(value: &IOValue) -> Result<RetentionCandidateBundleVerify> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-verify-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-verify-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA, "retention candidate bundle verify schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let bundle_ref = record_ref(&fields[2], "bundle")?;
    let explain_ref = record_ref(&fields[3], "explain")?;
    let object_fields = fields[4]
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle verify object"))?;
    let object_ref = required_string(&object_fields[0], "retention bundle verify object ref")?;
    require_ref(&object_ref, "retention bundle verify object ref")?;
    let object_kind = optional_record_string(&object_fields[1], "retention bundle verify object kind")?;
    if let Some(object_kind) = object_kind.as_deref() {
        validate_name(object_kind, "retention bundle verify object kind")?;
    }
    let filter_fields = fields[5]
        .collect_simple_record("filters", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention bundle verify filters"))?;
    let retention_class = record_optional_string(&filter_fields[0], "class")?;
    if let Some(retention_class) = retention_class.as_deref() {
        validate_retention_class(retention_class)?;
    }
    let action = record_optional_string(&filter_fields[1], "action")?;
    if let Some(action) = action.as_deref() {
        validate_action(action)?;
    }
    let subsystem = record_optional_string(&filter_fields[2], "subsystem")?;
    if let Some(subsystem) = subsystem.as_deref() {
        validate_name(subsystem, "retention bundle verify subsystem")?;
    }
    let artifact_refs = record_ref_sequence(&fields[6], "artifacts")?;
    let file_refs = record_ref_sequence(&fields[7], "files")?;
    let diagnostics = record_string_sequence(&fields[8], "diagnostics")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "verify-is-not-authority", "retention candidate bundle verify")?;
    require_check(&checks, "read-only-verify", "retention candidate bundle verify")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle verify")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle verify")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle verify")?;
    Ok(RetentionCandidateBundleVerify {
        verify_ref: canonical_hash(value)?,
        decision,
        bundle_ref,
        explain_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        artifact_refs,
        file_refs,
        diagnostics,
        value: value.clone(),
    })
}

fn validate_retention_candidate_bundle_verify_value_input(
    input: &RetentionCandidateBundleVerifyValueInput<'_>,
) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(&input.bundle.bundle_ref, "retention bundle verify bundle ref")?;
    require_ref(&input.bundle.explain_ref, "retention bundle verify explain ref")?;
    validate_refs(&input.bundle.artifact_refs, "retention bundle verify artifact ref")?;
    validate_refs(input.file_refs, "retention bundle verify file ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle verify diagnostics")
}

fn parse_retention_gc_plan_kind(value: &IOValue) -> Result<()> {
    parse_retention_gc_plan(value).map(|_| ())
}

fn parse_retention_gc_apply_kind(value: &IOValue) -> Result<()> {
    parse_retention_gc_apply(value).map(|_| ())
}

fn parse_retention_gc_execution_kind(value: &IOValue) -> Result<()> {
    parse_retention_gc_execution_gate(value).map(|_| ())
}

fn parse_retention_gc_audit_kind(value: &IOValue) -> Result<()> {
    parse_retention_gc_audit(value).map(|_| ())
}

fn parse_retention_receipt_kind(value: &IOValue) -> Result<()> {
    parse_retention_receipt(value).map(|_| ())
}

fn parse_retention_tombstone_kind(value: &IOValue) -> Result<()> {
    parse_tombstone(value).map(|_| ())
}

fn validate_retention_candidate_bundle_value_input(input: &RetentionCandidateBundleValueInput<'_>) -> Result<()> {
    require_ref(&input.explain.explain_ref, "retention bundle explain ref")?;
    validate_retention_candidate_explain_value_input(&RetentionCandidateExplainValueInput {
        object_ref: &input.explain.object_ref,
        object_kind: input.explain.object_kind.as_deref(),
        retention_class: input.explain.retention_class.as_deref(),
        action: input.explain.action.as_deref(),
        subsystem: input.explain.subsystem.as_deref(),
        pin_refs: &input.explain.pin_refs,
        admission_refs: &input.explain.admission_refs,
        remote_clearance_refs: &input.explain.remote_clearance_refs,
        remote_clearance_import_refs: &input.explain.remote_clearance_import_refs,
        gc_plan_refs: &input.explain.gc_plan_refs,
        gc_apply_refs: &input.explain.gc_apply_refs,
        gc_execution_refs: &input.explain.gc_execution_refs,
        gc_audit_refs: &input.explain.gc_audit_refs,
        retention_receipt_refs: &input.explain.retention_receipt_refs,
        tombstone_refs: &input.explain.tombstone_refs,
        diagnostics: &input.explain.diagnostics,
    })?;
    validate_refs(input.artifact_refs, "retention bundle artifact ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle diagnostics")
}

fn read_retention_gc_plan_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_gc_plan(root, reference)?.value)
}

fn read_retention_gc_apply_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_gc_apply(root, reference)?.value)
}

fn read_retention_gc_execution_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_gc_execution_gate(root, reference)?.value)
}

fn read_retention_gc_audit_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_gc_audit(root, reference)?.value)
}

fn read_retention_receipt_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_receipt(root, reference)?.value)
}

fn read_retention_tombstone_value(root: &Path, reference: &str) -> Result<IOValue> {
    Ok(read_retention_tombstone(root, reference)?.value)
}

fn validate_retention_candidate_explain_input(input: &RetentionCandidateExplainInput<'_>) -> Result<()> {
    require_ref(input.object_ref, "retention candidate object ref")?;
    if let Some(object_kind) = input.object_kind {
        validate_name(object_kind, "retention candidate object kind")?;
    }
    if let Some(retention_class) = input.retention_class {
        validate_retention_class(retention_class)?;
    }
    if let Some(action) = input.action {
        validate_action(action)?;
    }
    if let Some(subsystem) = input.subsystem {
        validate_name(subsystem, "retention candidate subsystem")?;
    }
    Ok(())
}

fn validate_retention_candidate_explain_value_input(input: &RetentionCandidateExplainValueInput<'_>) -> Result<()> {
    validate_retention_candidate_explain_input(&RetentionCandidateExplainInput {
        root: Path::new("."),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    })?;
    validate_refs(input.pin_refs, "retention candidate pin ref")?;
    validate_refs(input.admission_refs, "retention candidate admission ref")?;
    validate_refs(input.remote_clearance_refs, "retention candidate remote clearance ref")?;
    validate_refs(input.remote_clearance_import_refs, "retention candidate remote clearance import ref")?;
    validate_refs(input.gc_plan_refs, "retention candidate GC plan ref")?;
    validate_refs(input.gc_apply_refs, "retention candidate GC apply ref")?;
    validate_refs(input.gc_execution_refs, "retention candidate GC execution ref")?;
    validate_refs(input.gc_audit_refs, "retention candidate GC audit ref")?;
    validate_refs(input.retention_receipt_refs, "retention candidate receipt ref")?;
    validate_refs(input.tombstone_refs, "retention candidate tombstone ref")?;
    validate_diagnostics(input.diagnostics, "retention candidate explain diagnostics")
}

impl RetentionCandidateFilter<'_> {
    fn matches_object(&self, object_ref: &str, object_kind: &str, retention_class: &str) -> bool {
        object_ref == self.object_ref
            && self.object_kind.is_none_or(|expected| expected == object_kind)
            && self.retention_class.is_none_or(|expected| expected == retention_class)
    }

    fn matches_retention(&self, object_ref: &str, object_kind: &str, retention_class: &str, action: &str) -> bool {
        self.matches_object(object_ref, object_kind, retention_class)
            && self.action.is_none_or(|expected| expected == action)
    }

    fn matches_gc(
        &self,
        subsystem: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> bool {
        self.matches_retention(object_ref, object_kind, retention_class, action)
            && self.subsystem.is_none_or(|expected| expected == subsystem)
    }
}

fn collect_matching_retention_refs<T, Parse, Matches, Reference>(
    dir: &Path,
    parse: Parse,
    matches: Matches,
    reference: Reference,
    label: &str,
) -> Result<Vec<String>>
where
    Parse: Fn(&IOValue) -> Result<T>,
    Matches: Fn(&T) -> bool,
    Reference: Fn(&T) -> String,
{
    let mut refs = Vec::new();
    if !dir.exists() {
        return Ok(refs);
    }
    let mut paths = Vec::new();
    for entry_result in fs::read_dir(dir).map_err(MoltenError::from)? {
        let entry = entry_result.map_err(MoltenError::from)?;
        if entry.file_type().map_err(MoltenError::from)?.is_file() {
            push_bounded(&mut paths, entry.path(), MAX_RETENTION_REFS, label)?;
        }
    }
    paths.sort();
    for path in paths {
        let value = read_store_value(&path)?;
        let parsed = parse(&value)?;
        if matches(&parsed) {
            push_bounded(&mut refs, reference(&parsed), MAX_RETENTION_REFS, label)?;
        }
    }
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |text| record("some", vec![string(text)]))
}

fn record_optional_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    optional_record_string(&fields[0], label)
}

fn optional_record_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let inner = value_to_iovalue(value);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = inner
            .collect_simple_record("some", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional string for {label}")))?;
        Ok(Some(required_string(&some[0], label)?))
    }
}

fn record_optional_ref_with_status(value: &Value<IOValue>, label: &str) -> Result<(Option<String>, String)> {
    let fields = value
        .collect_simple_record(label, Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected retention GC audit {label} record")))?;
    let inner = value_to_iovalue(&fields[0]);
    let reference = if inner.collect_simple_record("none", Some(0)).is_some() {
        None
    } else {
        let some = inner
            .collect_simple_record("some", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
        let reference = required_string(&some[0], label)?;
        require_ref(&reference, label)?;
        Some(reference)
    };
    let status = required_string(&fields[1], label)?;
    validate_audit_step_status(&status, label)?;
    Ok((reference, status))
}

pub fn parse_retention_receipt(value: &IOValue) -> Result<RetentionReceipt> {
    let fields = value
        .collect_simple_record("retention-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-receipt-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_RECEIPT_SCHEMA, "retention receipt schema")?;
    let decision = record_string(&fields[1], "decision")?;
    let action = record_string(&fields[2], "action")?;
    let (object_ref, object_kind) = parse_object_value(&fields[3])?;
    let retention_class = record_string(&fields[4], "class")?;
    let requester_ref = record_ref(&fields[5], "requester")?;
    let index_ref = record_ref(&fields[6], "index")?;
    let pin_refs = record_ref_sequence(&fields[7], "pins")?;
    let retained_refs = record_ref_sequence(&fields[8], "retained")?;
    let remote_refs = record_ref_sequence(&fields[9], "remote")?;
    let tombstone_ref = record_optional_ref(&fields[10], "tombstone")?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "reference-index-bound", "retention receipt")?;
    validate_action(&action)?;
    validate_retention_class(&retention_class)?;
    Ok(RetentionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        action,
        object_ref,
        object_kind,
        retention_class,
        requester_ref,
        index_ref,
        pin_refs,
        retained_refs,
        remote_refs,
        tombstone_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn parse_tombstone(value: &IOValue) -> Result<RetentionTombstone> {
    let fields = value
        .collect_simple_record("retention-tombstone-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-tombstone-v1 ...>"))?;
    require_schema(&fields[0], RETENTION_TOMBSTONE_SCHEMA, "retention tombstone schema")?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let retention_class = record_string(&fields[2], "class")?;
    let action = record_string(&fields[3], "action")?;
    let receipt_ref = record_ref(&fields[4], "receipt")?;
    let policy_refs = record_ref_sequence(&fields[5], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[6], "evidence")?;
    require_check(&parse_checks(&fields[8])?, "audit-visible-tombstone", "retention tombstone")?;
    validate_retention_class(&retention_class)?;
    validate_action(&action)?;
    Ok(RetentionTombstone {
        tombstone_ref: canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        action,
        receipt_ref,
        policy_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn read_retention_receipt(root: &Path, receipt_ref: &str) -> Result<RetentionReceipt> {
    require_ref(receipt_ref, "retention receipt ref")?;
    let value = read_store_value(&receipt_path(root, receipt_ref)?)?;
    parse_retention_receipt(&value)
}

pub fn read_retention_tombstone(root: &Path, tombstone_ref: &str) -> Result<RetentionTombstone> {
    require_ref(tombstone_ref, "retention tombstone ref")?;
    let value = read_store_value(&tombstone_path(root, tombstone_ref)?)?;
    let tombstone = parse_tombstone(&value)?;
    if tombstone.tombstone_ref != tombstone_ref {
        return Err(MoltenError::invalid_harness("stored retention tombstone ref mismatch"));
    }
    Ok(tombstone)
}

pub fn retention_summary(value: &IOValue) -> Result<String> {
    if let Some(text) = base(value) {
        return Ok(text);
    }
    if let Some(text) = admission(value) {
        return Ok(text);
    }
    if let Some(text) = peer(value) {
        return Ok(text);
    }
    if let Some(text) = live(value) {
        return Ok(text);
    }
    if let Some(text) = gate(value) {
        return Ok(text);
    }
    if let Some(text) = audit(value) {
        return Ok(text);
    }
    if let Some(text) = review(value) {
        return Ok(text);
    }
    if let Some(text) = profile(value) {
        return Ok(text);
    }
    if let Some(text) = stored(value) {
        return Ok(text);
    }
    Err(MoltenError::invalid_harness("unsupported retention artifact"))
}

fn base(value: &IOValue) -> Option<String> {
    if let Ok(profile) = parse_retention_class_profile(value) {
        return Some(format!(
            "retention class ref={} class={} min={} max={} policies={} diagnostics={}",
            profile.profile_ref,
            profile.class_name,
            profile.minimum_age_seconds,
            profile.maximum_age_seconds.map_or_else(|| "none".to_string(), |value| value.to_string()),
            profile.policy_refs.len(),
            profile.diagnostics.join(",")
        ));
    }
    if let Ok(pin) = parse_retention_pin(value) {
        return Some(format!(
            "retention pin ref={} object={} kind={} class={} source={} owner={}",
            pin.pin_ref, pin.object_ref, pin.object_kind, pin.retention_class, pin.source, pin.owner_ref
        ));
    }
    if let Ok(index) = parse_reference_index(value) {
        return Some(format!(
            "retention index ref={} object={} kind={} pins={} retained={} remote={} complete={}",
            index.index_ref,
            index.object_ref,
            index.object_kind,
            index.pin_refs.len(),
            index.retained_refs.len(),
            index.remote_refs.len(),
            index.is_complete
        ));
    }
    None
}

fn admission(value: &IOValue) -> Option<String> {
    if let Ok(admission) = parse_retention_evidence_admission(value) {
        return Some(format!(
            "retention admission ref={} kind={} decision={} object={} class={} action={} current={} revoked={} diagnostics={}",
            admission.admission_ref,
            admission.kind,
            admission.decision,
            admission.object_ref,
            admission.retention_class,
            admission.action,
            admission.is_current,
            admission.revoked_refs.len(),
            admission.diagnostics.join(",")
        ));
    }
    None
}

fn peer(value: &IOValue) -> Option<String> {
    if let Ok(request) = parse_retention_remote_gc_clearance_request(value) {
        return Some(format!(
            "retention remote clearance request ref={} requester={} peer={} remote={} object={} class={} action={} evidence={}",
            request.request_ref,
            request.requester_ref,
            request.peer_ref,
            request.remote_ref,
            request.object_ref,
            request.retention_class,
            request.action,
            request.evidence_refs.len()
        ));
    }
    if let Ok(response) = parse_retention_remote_gc_clearance_response(value) {
        return Some(format!(
            "retention remote clearance response ref={} decision={} request={} clearance={} peer={} remote={} diagnostics={}",
            response.response_ref,
            response.decision,
            response.request_ref,
            response.clearance_ref,
            response.request.peer_ref,
            response.request.remote_ref,
            response.diagnostics.join(",")
        ));
    }
    if let Ok(import) = parse_retention_remote_gc_clearance_import(value) {
        return Some(format!(
            "retention remote clearance import ref={} decision={} request={} response={} clearance={} peer={} remote={} diagnostics={}",
            import.import_ref,
            import.decision,
            import.request_ref,
            import.response_ref,
            import.clearance_ref.as_deref().unwrap_or("none"),
            import.peer_ref,
            import.remote_ref,
            import.diagnostics.join(",")
        ));
    }
    None
}

fn live(value: &IOValue) -> Option<String> {
    if let Ok(workflow) = parse_retention_remote_gc_clearance_live_workflow(value) {
        return Some(format!(
            "retention remote clearance live workflow ref={} decision={} request={} response={} import={} clearance={} peer={} remote={} diagnostics={}",
            workflow.workflow_ref,
            workflow.decision,
            workflow.request_ref,
            workflow.response_ref,
            workflow.import_ref,
            workflow.clearance_ref.as_deref().unwrap_or("none"),
            workflow.peer_ref,
            workflow.remote_ref,
            workflow.diagnostics.join(",")
        ));
    }
    if let Ok(clearance) = parse_retention_remote_gc_clearance(value) {
        return Some(format!(
            "retention remote clearance ref={} decision={} peer={} remote={} object={} class={} action={} current={} retained={} revoked={} diagnostics={}",
            clearance.clearance_ref,
            clearance.decision,
            clearance.peer_ref,
            clearance.remote_ref,
            clearance.object_ref,
            clearance.retention_class,
            clearance.action,
            clearance.is_current,
            clearance.retained_refs.len(),
            clearance.revoked_refs.len(),
            clearance.diagnostics.join(",")
        ));
    }
    None
}

fn gate(value: &IOValue) -> Option<String> {
    if let Ok(plan) = parse_retention_gc_plan(value) {
        return Some(format!(
            "retention gc plan ref={} decision={} subsystem={} action={} object={} class={} requester={} index={} gates={} diagnostics={}",
            plan.plan_ref,
            plan.decision,
            plan.subsystem,
            plan.action,
            plan.object_ref,
            plan.retention_class,
            plan.requester_ref.as_deref().unwrap_or("none"),
            plan.index_ref,
            plan.gates.len(),
            plan.diagnostics.join(",")
        ));
    }
    if let Ok(apply) = parse_retention_gc_apply(value) {
        return Some(format!(
            "retention gc apply ref={} decision={} subsystem={} action={} object={} class={} plan={} recomputed={} receipt={} tombstone={} diagnostics={}",
            apply.apply_ref,
            apply.decision,
            apply.subsystem,
            apply.action,
            apply.object_ref,
            apply.retention_class,
            apply.plan_ref,
            apply.recomputed_plan_ref,
            apply.retention_receipt_ref.as_deref().unwrap_or("none"),
            apply.tombstone_ref.as_deref().unwrap_or("none"),
            apply.diagnostics.join(",")
        ));
    }
    None
}

fn audit(value: &IOValue) -> Option<String> {
    if let Ok(execute) = parse_retention_gc_execution_gate(value) {
        return Some(format!(
            "retention gc execute ref={} decision={} subsystem={} action={} object={} class={} apply={} plan={} receipt={} tombstone={} diagnostics={}",
            execute.execution_ref,
            execute.decision,
            execute.subsystem,
            execute.action,
            execute.object_ref,
            execute.retention_class,
            execute.apply_ref.as_deref().unwrap_or("none"),
            execute.plan_ref.as_deref().unwrap_or("none"),
            execute.retention_receipt_ref.as_deref().unwrap_or("none"),
            execute.tombstone_ref.as_deref().unwrap_or("none"),
            execute.diagnostics.join(",")
        ));
    }
    if let Ok(audit) = parse_retention_gc_audit(value) {
        return Some(format!(
            "retention gc audit ref={} decision={} subsystem={} action={} object={} class={} plan={} apply={} execution={} receipt={} tombstone={} diagnostics={}",
            audit.audit_ref,
            audit.decision,
            audit.subsystem,
            audit.action,
            audit.object_ref,
            audit.retention_class,
            audit.plan_ref.as_deref().unwrap_or("none"),
            audit.apply_ref.as_deref().unwrap_or("none"),
            audit.execution_ref,
            audit.retention_receipt_ref.as_deref().unwrap_or("none"),
            audit.tombstone_ref.as_deref().unwrap_or("none"),
            audit.diagnostics.join(",")
        ));
    }
    None
}

fn review(value: &IOValue) -> Option<String> {
    if let Ok(explain) = parse_retention_candidate_explain(value) {
        return Some(format!(
            "retention candidate explain ref={} object={} kind={} class={} action={} subsystem={} pins={} admissions={} clearances={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
            explain.explain_ref,
            explain.object_ref,
            explain.object_kind.as_deref().unwrap_or("any"),
            explain.retention_class.as_deref().unwrap_or("any"),
            explain.action.as_deref().unwrap_or("any"),
            explain.subsystem.as_deref().unwrap_or("any"),
            explain.pin_refs.len(),
            explain.admission_refs.len(),
            explain.remote_clearance_refs.len(),
            explain.gc_plan_refs.len(),
            explain.gc_apply_refs.len(),
            explain.gc_execution_refs.len(),
            explain.gc_audit_refs.len(),
            explain.retention_receipt_refs.len(),
            explain.tombstone_refs.len(),
            explain.diagnostics.join(",")
        ));
    }
    if let Ok(bundle) = parse_retention_candidate_bundle(value) {
        return Some(format!(
            "retention candidate bundle ref={} explain={} object={} kind={} class={} action={} subsystem={} artifacts={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
            bundle.bundle_ref,
            bundle.explain_ref,
            bundle.object_ref,
            bundle.object_kind.as_deref().unwrap_or("any"),
            bundle.retention_class.as_deref().unwrap_or("any"),
            bundle.action.as_deref().unwrap_or("any"),
            bundle.subsystem.as_deref().unwrap_or("any"),
            bundle.artifact_refs.len(),
            bundle.gc_plan_refs.len(),
            bundle.gc_apply_refs.len(),
            bundle.gc_execution_refs.len(),
            bundle.gc_audit_refs.len(),
            bundle.retention_receipt_refs.len(),
            bundle.tombstone_refs.len(),
            bundle.diagnostics.join(",")
        ));
    }
    None
}

fn profile(value: &IOValue) -> Option<String> {
    if let Ok(profile) = parse_retention_candidate_bundle_profile(value) {
        return Some(format!(
            "retention candidate bundle profile ref={} decision={} profile={} loss={} bundle={} markers={} diagnostics={}",
            profile.profile_ref,
            profile.decision,
            profile.profile,
            profile.loss_classification,
            profile.bundle_ref,
            profile.marker_refs.len(),
            profile.diagnostics.join(",")
        ));
    }
    if let Ok(verify) = parse_retention_candidate_bundle_verify(value) {
        return Some(format!(
            "retention candidate bundle verify ref={} decision={} bundle={} explain={} object={} kind={} class={} action={} subsystem={} artifacts={} files={} diagnostics={}",
            verify.verify_ref,
            verify.decision,
            verify.bundle_ref,
            verify.explain_ref,
            verify.object_ref,
            verify.object_kind.as_deref().unwrap_or("any"),
            verify.retention_class.as_deref().unwrap_or("any"),
            verify.action.as_deref().unwrap_or("any"),
            verify.subsystem.as_deref().unwrap_or("any"),
            verify.artifact_refs.len(),
            verify.file_refs.len(),
            verify.diagnostics.join(",")
        ));
    }
    None
}

fn stored(value: &IOValue) -> Option<String> {
    if let Ok(receipt) = parse_retention_receipt(value) {
        return Some(format!(
            "retention receipt ref={} decision={} action={} object={} class={} pins={} tombstone={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.action,
            receipt.object_ref,
            receipt.retention_class,
            receipt.pin_refs.len(),
            receipt.tombstone_ref.as_deref().unwrap_or("none"),
            receipt.diagnostics.join(",")
        ));
    }
    if let Ok(tombstone) = parse_tombstone(value) {
        return Some(format!(
            "retention tombstone ref={} object={} class={} action={} receipt={}",
            tombstone.tombstone_ref,
            tombstone.object_ref,
            tombstone.retention_class,
            tombstone.action,
            tombstone.receipt_ref
        ));
    }
    None
}

pub fn run_fixture(out: &Path) -> Result<Vec<(String, IOValue)>> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    let root = out.join("state");
    ensure_store(&root)?;
    let seed = seed_refs()?;
    let class = class_value(&seed)?;
    let pin = pin_step(&root, &seed)?;
    let deny = eval_step(&root, &seed, ACTION_DELETE)?;
    let unpin = unpin_object(UnpinObjectInput {
        root: &root,
        pin_ref: &pin.pin.pin_ref,
        requester_ref: &seed.owner_ref,
        policy_refs: &seed.policy_refs,
        evidence_refs: &seed.evidence_refs,
        has_authority: true,
    })?;
    let delete = eval_step(&root, &seed, ACTION_TOMBSTONE)?;
    let artifacts = output_values(OutputValues {
        class,
        pin,
        deny,
        unpin,
        delete,
    })?;
    for (name, value) in &artifacts {
        write_store_value(&out.join(name), value)?;
    }
    Ok(artifacts)
}

struct SeedRefs {
    object_ref: String,
    owner_ref: String,
    policy_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

fn seed_refs() -> Result<SeedRefs> {
    Ok(SeedRefs {
        object_ref: synthetic_ref("retention-object")?,
        owner_ref: synthetic_ref("owner")?,
        policy_refs: vec![synthetic_ref("policy")?],
        evidence_refs: vec![synthetic_ref("evidence")?],
    })
}

fn class_value(seed: &SeedRefs) -> Result<IOValue> {
    retention_class_profile_value(&RetentionClassProfileInput {
        class_name: CLASS_PRIVATE_SECRET_REF.to_string(),
        minimum_age_seconds: 0,
        maximum_age_seconds: Some(86_400),
        deletion_authority_ref: synthetic_ref("authority")?,
        policy_refs: seed.policy_refs.clone(),
        has_secret_redaction_hook: true,
        has_remote_gc_plan: true,
        can_compact: true,
    })
}

fn pin_step(root: &Path, seed: &SeedRefs) -> Result<PinOperation> {
    pin_object(root, RetentionPinInput {
        object_ref: seed.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: CLASS_PRIVATE_SECRET_REF.to_string(),
        source: SOURCE_SECRET_REDACTION.to_string(),
        reason: "private repro reveal pending".to_string(),
        owner_ref: seed.owner_ref.clone(),
        expiry_ref: None,
        policy_refs: seed.policy_refs.clone(),
        evidence_refs: seed.evidence_refs.clone(),
        has_authority: true,
    })
}

fn eval_step(root: &Path, seed: &SeedRefs, action: &str) -> Result<RetentionEvaluation> {
    evaluate_retention(RetentionEvaluationInput {
        root,
        object_ref: &seed.object_ref,
        object_kind: "encrypted-ref",
        retention_class: CLASS_PRIVATE_SECRET_REF,
        action,
        requester_ref: &seed.owner_ref,
        is_reference_index_complete: true,
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &seed.policy_refs,
        evidence_refs: &seed.evidence_refs,
        has_delete_authority: true,
        has_remote_gc_clearance: true,
    })
}

struct OutputValues {
    class: IOValue,
    pin: PinOperation,
    deny: RetentionEvaluation,
    unpin: RetentionReceipt,
    delete: RetentionEvaluation,
}

fn output_values(parts: OutputValues) -> Result<Vec<(String, IOValue)>> {
    let OutputValues {
        class,
        pin,
        deny,
        unpin,
        delete,
    } = parts;
    let PinOperation {
        pin,
        receipt: pin_receipt,
    } = pin;
    let RetentionEvaluation {
        receipt: deny_receipt, ..
    } = deny;
    let RetentionEvaluation {
        receipt: delete_receipt,
        tombstone,
        ..
    } = delete;
    let mut artifacts = Vec::new();
    push_named(&mut artifacts, "retention-class.preserves", class)?;
    push_named(&mut artifacts, "pin.preserves", pin.value)?;
    push_named(&mut artifacts, "pin-receipt.preserves", pin_receipt.value)?;
    push_named(&mut artifacts, "delete-denied.preserves", deny_receipt.value)?;
    push_named(&mut artifacts, "unpin-receipt.preserves", unpin.value)?;
    push_named(&mut artifacts, "tombstone-receipt.preserves", delete_receipt.value)?;
    if let Some(tombstone) = tombstone {
        push_named(&mut artifacts, "tombstone.preserves", tombstone.value)?;
    }
    Ok(artifacts)
}

struct RetentionReceiptBuildInput<'a> {
    decision: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: &'a str,
    index_ref: &'a str,
    pin_refs: &'a [String],
    retained_refs: &'a [String],
    remote_refs: &'a [String],
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    tombstone_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

fn retention_receipt(input: RetentionReceiptBuildInput<'_>) -> Result<RetentionReceipt> {
    validate_receipt_build_input(&input)?;
    let value = record("retention-receipt-v1", vec![
        string(RETENTION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("action", vec![string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("requester", vec![string(input.requester_ref)]),
        record("index", vec![string(input.index_ref)]),
        record("pins", vec![strings_sequence(input.pin_refs)]),
        record("retained", vec![strings_sequence(input.retained_refs)]),
        record("remote", vec![strings_sequence(input.remote_refs)]),
        record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("policy", vec![strings_sequence(input.policy_refs)]),
        checks_value(&[
            ("reference-index-bound", "pass"),
            ("policy-bound", pass_or_deny(!input.policy_refs.is_empty())),
            ("authority-bound", pass_or_deny(input.decision == "pass" || input.action == ACTION_ELIGIBILITY)),
            ("mutable-name-not-gc-proof", "pass"),
            ("remote-cache-considered", "pass"),
        ]),
    ]);
    parse_retention_receipt(&value)
}

struct RetentionTombstoneBuildInput<'a> {
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
    receipt_ref: &'a str,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn retention_tombstone(input: RetentionTombstoneBuildInput<'_>) -> Result<RetentionTombstone> {
    let receipt_ref = if input.receipt_ref == "pending" {
        synthetic_ref("pending-retention-receipt")?
    } else {
        input.receipt_ref.to_string()
    };
    let value = record("retention-tombstone-v1", vec![
        string(RETENTION_TOMBSTONE_SCHEMA),
        object_value(input.object_ref, input.object_kind),
        record("class", vec![string(input.retention_class)]),
        record("action", vec![string(input.action)]),
        record("receipt", vec![string(&receipt_ref)]),
        record("policy", vec![strings_sequence(input.policy_refs)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        record("public-metadata", vec![sequence(vec![
            record("object-kind", vec![string(input.object_kind)]),
            record("class", vec![string(input.retention_class)]),
            record("content", vec![string("redacted-or-deleted")]),
        ])]),
        checks_value(&[
            ("audit-visible-tombstone", "pass"),
            ("secret-content-not-leaked", "pass"),
            ("deletion-not-hidden", "pass"),
        ]),
    ]);
    parse_tombstone(&value)
}

fn retention_diagnostics(input: &RetentionEvaluationInput<'_>, index: &RetentionReferenceIndex) -> Result<Vec<String>> {
    let is_destructive = is_destructive_action(input.action);
    let mut diagnostics = Vec::new();
    push_notes(&mut diagnostics, [
        (!input.is_reference_index_complete, "incomplete-reference-proof"),
        (!index.pin_refs.is_empty(), "active-pins-present"),
        (!input.retained_refs.is_empty(), "retained-dependencies-present"),
        (input.policy_refs.is_empty(), "retention-policy-missing"),
        (is_destructive && input.evidence_refs.is_empty(), "retention-evidence-missing"),
        (is_destructive && !input.has_delete_authority, "delete-authority-missing"),
        (
            is_destructive && !input.remote_refs.is_empty() && !input.has_remote_gc_clearance,
            "remote-cache-refs-present",
        ),
        (input.retention_class == CLASS_LEGAL_HOLD && is_destructive, "legal-hold-class-not-deletable"),
        (
            input.retention_class == CLASS_PRIVATE_SECRET_REF && input.action == ACTION_COMPACT,
            "private-secret-ref-compaction-denied",
        ),
    ])?;
    Ok(diagnostics)
}

fn push_notes<S, I>(values: &mut S, entries: I) -> Result<()>
where
    S: VecSink<String>,
    I: IntoIterator<Item = (bool, &'static str)>,
{
    for (is_active, note) in entries {
        if is_active {
            push_bounded(values, note.to_string(), MAX_RETENTION_DIAGNOSTICS, "retention diagnostics")?;
        }
    }
    Ok(())
}

fn class_profile_diagnostics(input: &RetentionClassProfileInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.class_name == CLASS_PRIVATE_SECRET_REF && !input.has_secret_redaction_hook {
        push_bounded(
            &mut diagnostics,
            "private-secret-redaction-hook-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if !input.has_remote_gc_plan {
        push_bounded(
            &mut diagnostics,
            "remote-gc-plan-not-declared".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn is_destructive_action(action: &str) -> bool {
    matches!(action, ACTION_DELETE | ACTION_TOMBSTONE | ACTION_REDACT | ACTION_COMPACT)
}

fn validate_class_profile_input(input: &RetentionClassProfileInput) -> Result<()> {
    validate_retention_class(&input.class_name)?;
    require_ref(&input.deletion_authority_ref, "retention deletion authority ref")?;
    validate_refs(&input.policy_refs, "retention class policy ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("retention class profile requires policy refs"));
    }
    if input.maximum_age_seconds.is_some_and(|maximum| maximum < input.minimum_age_seconds) {
        return Err(MoltenError::invalid_harness("retention maximum age cannot be below minimum age"));
    }
    Ok(())
}

fn validate_pin_input(input: &RetentionPinInput) -> Result<()> {
    require_ref(&input.object_ref, "retention pin object ref")?;
    validate_name(&input.object_kind, "retention pin object kind")?;
    validate_retention_class(&input.retention_class)?;
    validate_pin_source(&input.source)?;
    validate_name(&input.reason, "retention pin reason")?;
    require_ref(&input.owner_ref, "retention pin owner ref")?;
    if let Some(expiry) = input.expiry_ref.as_deref() {
        require_ref(expiry, "retention pin expiry ref")?;
    }
    validate_refs(&input.policy_refs, "retention pin policy ref")?;
    validate_refs(&input.evidence_refs, "retention pin evidence ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("retention pin requires policy refs"));
    }
    Ok(())
}

fn validate_reference_index_input(input: &RetentionReferenceIndexInput) -> Result<()> {
    require_ref(&input.object_ref, "retention index object ref")?;
    validate_name(&input.object_kind, "retention index object kind")?;
    validate_refs(&input.pin_refs, "retention index pin ref")?;
    validate_refs(&input.retained_refs, "retention index retained ref")?;
    validate_refs(&input.tombstone_refs, "retention index tombstone ref")?;
    validate_refs(&input.remote_refs, "retention index remote ref")?;
    Ok(())
}

fn validate_receipt_build_input(input: &RetentionReceiptBuildInput<'_>) -> Result<()> {
    if input.decision != "pass" && input.decision != "deny" {
        return Err(MoltenError::invalid_harness("retention receipt decision must be pass or deny"));
    }
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention receipt object ref")?;
    validate_name(input.object_kind, "retention receipt object kind")?;
    validate_retention_class(input.retention_class)?;
    require_ref(input.requester_ref, "retention receipt requester ref")?;
    require_ref(input.index_ref, "retention receipt index ref")?;
    validate_refs(input.pin_refs, "retention receipt pin ref")?;
    validate_refs(input.retained_refs, "retention receipt retained ref")?;
    validate_refs(input.remote_refs, "retention receipt remote ref")?;
    validate_refs(input.policy_refs, "retention receipt policy ref")?;
    validate_refs(input.evidence_refs, "retention receipt evidence ref")?;
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention receipt tombstone ref")?;
    }
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention receipt diagnostics")
}

fn validate_retention_class(value: &str) -> Result<()> {
    if RETENTION_CLASSES.iter().any(|class| class == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention class {value}")))
    }
}

fn validate_pin_source(value: &str) -> Result<()> {
    if PIN_SOURCES.iter().any(|source| source == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention pin source {value}")))
    }
}

fn validate_action(value: &str) -> Result<()> {
    if RETENTION_ACTIONS.iter().any(|action| action == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention action {value}")))
    }
}

fn validate_admission_kind(value: &str) -> Result<()> {
    if ADMISSION_KINDS.iter().any(|kind| kind == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention admission kind {value}")))
    }
}

fn validate_decision(value: &str) -> Result<()> {
    if matches!(value, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention admission decision {value}")))
    }
}

fn validate_evidence_admission_input(input: &RetentionEvidenceAdmissionInput<'_>) -> Result<()> {
    validate_admission_kind(input.kind)?;
    validate_decision(input.decision)?;
    require_ref(input.requester_ref, "retention admission requester ref")?;
    require_ref(input.object_ref, "retention admission object ref")?;
    validate_name(input.object_kind, "retention admission object kind")?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    validate_refs(input.bound_refs, "retention admission bound ref")?;
    validate_refs(input.retained_refs, "retention admission retained ref")?;
    validate_refs(input.remote_refs, "retention admission remote ref")?;
    validate_refs(input.revoked_refs, "retention admission revoked ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")
}

fn validate_remote_gc_clearance_input(input: &RetentionRemoteGcClearanceInput<'_>) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(input.requester_ref, "retention remote clearance requester ref")?;
    require_ref(input.peer_ref, "retention remote clearance peer ref")?;
    require_ref(input.object_ref, "retention remote clearance object ref")?;
    validate_name(input.object_kind, "retention remote clearance object kind")?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.remote_ref, "retention remote clearance remote ref")?;
    require_ref(input.policy_ref, "retention remote clearance policy ref")?;
    require_ref(input.authority_ref, "retention remote clearance authority ref")?;
    validate_refs(input.evidence_refs, "retention remote clearance evidence ref")?;
    validate_refs(input.retained_refs, "retention remote clearance retained ref")?;
    validate_refs(input.revoked_refs, "retention remote clearance revoked ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention remote clearance diagnostics")
}

fn validate_remote_gc_clearance_request_input(input: &RetentionRemoteGcClearanceRequestInput<'_>) -> Result<()> {
    require_ref(input.requester_ref, "retention remote clearance request requester ref")?;
    require_ref(input.peer_ref, "retention remote clearance request peer ref")?;
    require_ref(input.object_ref, "retention remote clearance request object ref")?;
    validate_name(input.object_kind, "retention remote clearance request object kind")?;
    validate_retention_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.remote_ref, "retention remote clearance request remote ref")?;
    require_ref(input.policy_ref, "retention remote clearance request policy ref")?;
    require_ref(input.authority_ref, "retention remote clearance request authority ref")?;
    validate_refs(input.evidence_refs, "retention remote clearance request evidence ref")
}

fn validate_remote_gc_clearance_request(request: &RetentionRemoteGcClearanceRequest) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RetentionRemoteGcClearanceRequestInput {
        requester_ref: &request.requester_ref,
        peer_ref: &request.peer_ref,
        object_ref: &request.object_ref,
        object_kind: &request.object_kind,
        retention_class: &request.retention_class,
        action: &request.action,
        remote_ref: &request.remote_ref,
        policy_ref: &request.policy_ref,
        authority_ref: &request.authority_ref,
        evidence_refs: &request.evidence_refs,
    })
}

fn validate_remote_gc_clearance_live_loopback_input(
    input: &RetentionRemoteGcClearanceLiveLoopbackInput<'_>,
) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RetentionRemoteGcClearanceRequestInput {
        requester_ref: input.requester_ref,
        peer_ref: input.peer_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        remote_ref: input.remote_ref,
        policy_ref: input.policy_ref,
        authority_ref: input.authority_ref,
        evidence_refs: input.retention_evidence_refs,
    })?;
    validate_refs(input.response_evidence_refs, "retention live response evidence ref")?;
    validate_refs(input.retained_refs, "retention live retained ref")?;
    validate_refs(input.revoked_refs, "retention live revoked ref")?;
    ensure_count_at_most(
        input.response_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live response diagnostics",
    )?;
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.request_peer_bootstrap_refs, "retention live request peer bootstrap ref")?;
    validate_refs(input.request_authority_refs, "retention live request authority ref")?;
    validate_refs(input.request_policy_refs, "retention live request policy ref")?;
    validate_refs(input.request_resource_refs, "retention live request resource ref")?;
    validate_refs(input.request_transport_evidence_refs, "retention live request evidence ref")?;
    validate_refs(input.response_peer_bootstrap_refs, "retention live response peer bootstrap ref")?;
    validate_refs(input.response_authority_refs, "retention live response authority ref")?;
    validate_refs(input.response_policy_refs, "retention live response policy ref")?;
    validate_refs(input.response_resource_refs, "retention live response resource ref")?;
    validate_refs(input.response_transport_evidence_refs, "retention live response evidence ref")?;
    Ok(())
}

fn validate_remote_gc_clearance_live_request_send_input(
    input: &RetentionRemoteGcClearanceLiveRequestSendInput<'_>,
) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RetentionRemoteGcClearanceRequestInput {
        requester_ref: input.requester_ref,
        peer_ref: input.peer_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        remote_ref: input.remote_ref,
        policy_ref: input.policy_ref,
        authority_ref: input.authority_ref,
        evidence_refs: input.retention_evidence_refs,
    })?;
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.peer_bootstrap_refs, "retention live peer bootstrap ref")?;
    validate_refs(input.authority_refs, "retention live authority ref")?;
    validate_refs(input.policy_refs, "retention live policy ref")?;
    validate_refs(input.resource_refs, "retention live resource ref")?;
    validate_refs(input.transport_evidence_refs, "retention live transport evidence ref")
}

fn validate_remote_gc_clearance_live_response_send_input(
    input: &RetentionRemoteGcClearanceLiveResponseSendInput<'_>,
) -> Result<()> {
    parse_retention_remote_gc_clearance_request(input.request_value)?;
    validate_refs(input.response_evidence_refs, "retention live response evidence ref")?;
    validate_refs(input.retained_refs, "retention live retained ref")?;
    validate_refs(input.revoked_refs, "retention live revoked ref")?;
    ensure_count_at_most(
        input.response_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live response diagnostics",
    )?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.peer_bootstrap_refs, "retention live response peer bootstrap ref")?;
    validate_refs(input.authority_refs, "retention live response authority ref")?;
    validate_refs(input.policy_refs, "retention live response policy ref")?;
    validate_refs(input.resource_refs, "retention live response resource ref")?;
    validate_refs(input.transport_evidence_refs, "retention live response transport evidence ref")
}

fn validate_remote_gc_clearance_live_import_workflow_input(
    input: &RetentionRemoteGcClearanceLiveImportWorkflowInput<'_>,
) -> Result<()> {
    require_ref(input.request_ingress_ref, "retention live request ingress ref")?;
    require_ref(input.response_ingress_ref, "retention live response ingress ref")?;
    if let Some(peer_ref) = input.expected_peer_ref {
        require_ref(peer_ref, "retention live expected peer ref")?;
    }
    if let Some(remote_ref) = input.expected_remote_ref {
        require_ref(remote_ref, "retention live expected remote ref")?;
    }
    Ok(())
}

fn validate_remote_gc_clearance_live_workflow_value_input(
    input: &RetentionRemoteGcClearanceLiveWorkflowValueInput<'_>,
) -> Result<()> {
    require_ref(input.request_control_ref, "retention live request control ref")?;
    require_ref(input.request_publish_ref, "retention live request publish ref")?;
    require_ref(input.request_receive_ref, "retention live request receive ref")?;
    require_ref(input.request_ingress_ref, "retention live request ingress ref")?;
    require_ref(input.response_control_ref, "retention live response control ref")?;
    require_ref(input.response_publish_ref, "retention live response publish ref")?;
    require_ref(input.response_receive_ref, "retention live response receive ref")?;
    require_ref(input.response_ingress_ref, "retention live response ingress ref")?;
    ensure_count_at_most(
        input.transport_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live transport diagnostics",
    )
}

fn remote_clearance_response_diagnostics(input: RetentionRemoteGcClearanceResponseInput<'_>) -> Result<Vec<String>> {
    validate_refs(input.evidence_refs, "retention remote clearance response evidence ref")?;
    validate_refs(input.retained_refs, "retention remote clearance response retained ref")?;
    validate_refs(input.revoked_refs, "retention remote clearance response revoked ref")?;
    ensure_count_at_most(
        input.diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance response diagnostics",
    )?;
    let mut diagnostics = input.diagnostics.to_vec();
    if !input.is_current {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-stale".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    if !input.revoked_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-revoked".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    if !input.retained_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-retained".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn validate_remote_gc_clearance_workflow_scope(
    request: &RetentionRemoteGcClearanceRequest,
    clearance: &RetentionRemoteGcClearance,
) -> Result<()> {
    if clearance.requester_ref != request.requester_ref
        || clearance.peer_ref != request.peer_ref
        || clearance.object_ref != request.object_ref
        || clearance.object_kind != request.object_kind
        || clearance.retention_class != request.retention_class
        || clearance.action != request.action
        || clearance.remote_ref != request.remote_ref
        || clearance.policy_ref != request.policy_ref
        || clearance.authority_ref != request.authority_ref
    {
        return Err(MoltenError::invalid_harness("remote clearance workflow scope mismatch"));
    }
    Ok(())
}

fn parse_embedded_remote_clearance_request(value: &Value<IOValue>) -> Result<RetentionRemoteGcClearanceRequest> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("request", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance request"))?;
    let request_ref = required_string(&fields[0], "remote clearance request ref")?;
    require_ref(&request_ref, "remote clearance request ref")?;
    let request_value = value_to_iovalue(&fields[1]);
    let request = parse_retention_remote_gc_clearance_request(&request_value)?;
    if request.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance request ref mismatch"));
    }
    Ok(request)
}

fn parse_embedded_remote_clearance(value: &Value<IOValue>) -> Result<RetentionRemoteGcClearance> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("clearance", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance"))?;
    let clearance_ref = required_string(&fields[0], "remote clearance ref")?;
    require_ref(&clearance_ref, "remote clearance ref")?;
    let clearance_value = value_to_iovalue(&fields[1]);
    let clearance = parse_retention_remote_gc_clearance(&clearance_value)?;
    if clearance.clearance_ref != clearance_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance ref mismatch"));
    }
    Ok(clearance)
}

fn parse_embedded_remote_clearance_import(value: &Value<IOValue>) -> Result<RetentionRemoteGcClearanceImport> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("import", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance import"))?;
    let import_ref = required_string(&fields[0], "remote clearance import ref")?;
    require_ref(&import_ref, "remote clearance import ref")?;
    let import_value = value_to_iovalue(&fields[1]);
    let import = parse_retention_remote_gc_clearance_import(&import_value)?;
    if import.import_ref != import_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance import ref mismatch"));
    }
    Ok(import)
}

fn parse_embedded_value(value: &Value<IOValue>, label: &str) -> Result<(String, IOValue)> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected embedded {label}")))?;
    let value_ref = required_string(&fields[0], label)?;
    require_ref(&value_ref, label)?;
    let embedded = value_to_iovalue(&fields[1]);
    if canonical_hash(&embedded)? != value_ref {
        return Err(MoltenError::invalid_harness(format!("embedded {label} ref mismatch")));
    }
    Ok((value_ref, embedded))
}

fn push_import_diagnostic<S>(diagnostics: &mut S, diagnostic: &str) -> Result<()>
where S: VecSink<String> {
    push_bounded(
        diagnostics,
        diagnostic.to_string(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance import diagnostics",
    )
}

fn push_remote_clearance_import_diagnostics<S>(
    diagnostics: &mut S,
    request: &RetentionRemoteGcClearanceRequest,
    response: &RetentionRemoteGcClearanceResponse,
    input: RetentionRemoteGcClearanceImportInput<'_>,
) -> Result<()>
where
    S: VecSink<String>,
{
    if response.request_ref != request.request_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-request")?;
    }
    if response.decision != "pass" {
        push_import_diagnostic(diagnostics, "remote-clearance-response-not-pass")?;
    }
    let clearance = &response.clearance;
    if clearance.decision != "pass" {
        push_import_diagnostic(diagnostics, "remote-clearance-not-pass")?;
    }
    if !clearance.is_current {
        push_import_diagnostic(diagnostics, "remote-clearance-stale")?;
    }
    if !clearance.revoked_refs.is_empty() {
        push_import_diagnostic(diagnostics, "remote-clearance-revoked")?;
    }
    if !clearance.retained_refs.is_empty() {
        push_import_diagnostic(diagnostics, "remote-clearance-retained")?;
    }
    if clearance.peer_ref != request.peer_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-peer")?;
    }
    if clearance.remote_ref != request.remote_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-remote")?;
    }
    if let Some(expected_peer_ref) = input.expected_peer_ref
        && expected_peer_ref != request.peer_ref
    {
        push_import_diagnostic(diagnostics, "remote-clearance-expected-peer-mismatch")?;
    }
    if let Some(expected_remote_ref) = input.expected_remote_ref
        && expected_remote_ref != request.remote_ref
    {
        push_import_diagnostic(diagnostics, "remote-clearance-expected-remote-mismatch")?;
    }
    for diagnostic in &response.diagnostics {
        push_import_diagnostic(diagnostics, diagnostic)?;
    }
    Ok(())
}

fn ensure_store(root: &Path) -> Result<()> {
    fs::create_dir_all(pins_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(admissions_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(remote_clearances_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(remote_clearance_requests_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(remote_clearance_responses_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(remote_clearance_imports_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(remote_clearance_live_workflows_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(gc_plans_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(gc_applies_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(gc_executes_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(gc_audits_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(receipts_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(tombstones_dir(root)).map_err(MoltenError::from)
}

fn pins_for_object(root: &Path, object_ref: &str) -> Result<Vec<RetentionPin>> {
    let mut pins = Vec::new();
    let dir = pins_dir(root);
    if !dir.exists() {
        return Ok(pins);
    }
    for entry_result in fs::read_dir(dir).map_err(MoltenError::from)? {
        let entry = entry_result.map_err(MoltenError::from)?;
        if !entry.file_type().map_err(MoltenError::from)?.is_file() {
            continue;
        }
        let value = read_store_value(&entry.path())?;
        let pin = parse_retention_pin(&value)?;
        if pin.object_ref == object_ref {
            push_bounded(&mut pins, pin, MAX_RETENTION_REFS, "retention pins")?;
        }
    }
    pins.sort_by(|left, right| left.pin_ref.cmp(&right.pin_ref));
    Ok(pins)
}

fn tombstone_refs_for_object(root: &Path, object_ref: &str) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    let dir = tombstones_dir(root);
    if !dir.exists() {
        return Ok(refs);
    }
    for entry_result in fs::read_dir(dir).map_err(MoltenError::from)? {
        let entry = entry_result.map_err(MoltenError::from)?;
        if !entry.file_type().map_err(MoltenError::from)?.is_file() {
            continue;
        }
        let value = read_store_value(&entry.path())?;
        let tombstone = parse_tombstone(&value)?;
        if tombstone.object_ref == object_ref {
            push_bounded(&mut refs, tombstone.tombstone_ref, MAX_RETENTION_REFS, "retention tombstone refs")?;
        }
    }
    refs.sort();
    Ok(refs)
}

fn store_dir(root: &Path) -> PathBuf {
    root.join(STORE_DIR)
}

fn pins_dir(root: &Path) -> PathBuf {
    store_dir(root).join(PIN_DIR)
}

fn admissions_dir(root: &Path) -> PathBuf {
    store_dir(root).join(ADMISSION_DIR)
}

fn remote_clearances_dir(root: &Path) -> PathBuf {
    store_dir(root).join(REMOTE_CLEARANCE_DIR)
}

fn remote_clearance_requests_dir(root: &Path) -> PathBuf {
    store_dir(root).join(REMOTE_CLEARANCE_REQUEST_DIR)
}

fn remote_clearance_responses_dir(root: &Path) -> PathBuf {
    store_dir(root).join(REMOTE_CLEARANCE_RESPONSE_DIR)
}

fn remote_clearance_imports_dir(root: &Path) -> PathBuf {
    store_dir(root).join(REMOTE_CLEARANCE_IMPORT_DIR)
}

fn remote_clearance_live_workflows_dir(root: &Path) -> PathBuf {
    store_dir(root).join(REMOTE_CLEARANCE_LIVE_WORKFLOW_DIR)
}

fn gc_plans_dir(root: &Path) -> PathBuf {
    store_dir(root).join(GC_PLAN_DIR)
}

fn gc_applies_dir(root: &Path) -> PathBuf {
    store_dir(root).join(GC_APPLY_DIR)
}

fn gc_executes_dir(root: &Path) -> PathBuf {
    store_dir(root).join(GC_EXECUTE_DIR)
}

fn gc_audits_dir(root: &Path) -> PathBuf {
    store_dir(root).join(GC_AUDIT_DIR)
}

fn receipts_dir(root: &Path) -> PathBuf {
    store_dir(root).join(RECEIPT_DIR)
}

fn tombstones_dir(root: &Path) -> PathBuf {
    store_dir(root).join(TOMBSTONE_DIR)
}

fn pin_path(root: &Path, pin_ref: &str) -> Result<PathBuf> {
    Ok(pins_dir(root).join(format!("{}.preserves", ref_file_name(pin_ref)?)))
}

fn admission_path(root: &Path, admission_ref: &str) -> Result<PathBuf> {
    Ok(admissions_dir(root).join(format!("{}.preserves", ref_file_name(admission_ref)?)))
}

fn remote_clearance_path(root: &Path, clearance_ref: &str) -> Result<PathBuf> {
    Ok(remote_clearances_dir(root).join(format!("{}.preserves", ref_file_name(clearance_ref)?)))
}

fn remote_clearance_request_path(root: &Path, request_ref: &str) -> Result<PathBuf> {
    Ok(remote_clearance_requests_dir(root).join(format!("{}.preserves", ref_file_name(request_ref)?)))
}

fn remote_clearance_response_path(root: &Path, response_ref: &str) -> Result<PathBuf> {
    Ok(remote_clearance_responses_dir(root).join(format!("{}.preserves", ref_file_name(response_ref)?)))
}

fn remote_clearance_import_path(root: &Path, import_ref: &str) -> Result<PathBuf> {
    Ok(remote_clearance_imports_dir(root).join(format!("{}.preserves", ref_file_name(import_ref)?)))
}

fn remote_clearance_live_workflow_path(root: &Path, workflow_ref: &str) -> Result<PathBuf> {
    Ok(remote_clearance_live_workflows_dir(root).join(format!("{}.preserves", ref_file_name(workflow_ref)?)))
}

fn gc_plan_path(root: &Path, plan_ref: &str) -> Result<PathBuf> {
    Ok(gc_plans_dir(root).join(format!("{}.preserves", ref_file_name(plan_ref)?)))
}

fn gc_apply_path(root: &Path, apply_ref: &str) -> Result<PathBuf> {
    Ok(gc_applies_dir(root).join(format!("{}.preserves", ref_file_name(apply_ref)?)))
}

fn gc_execute_path(root: &Path, execution_ref: &str) -> Result<PathBuf> {
    Ok(gc_executes_dir(root).join(format!("{}.preserves", ref_file_name(execution_ref)?)))
}

fn gc_audit_path(root: &Path, audit_ref: &str) -> Result<PathBuf> {
    Ok(gc_audits_dir(root).join(format!("{}.preserves", ref_file_name(audit_ref)?)))
}

fn receipt_path(root: &Path, receipt_ref: &str) -> Result<PathBuf> {
    Ok(receipts_dir(root).join(format!("{}.preserves", ref_file_name(receipt_ref)?)))
}

fn tombstone_path(root: &Path, tombstone_ref: &str) -> Result<PathBuf> {
    Ok(tombstones_dir(root).join(format!("{}.preserves", ref_file_name(tombstone_ref)?)))
}

fn ref_file_name(reference: &str) -> Result<String> {
    require_ref(reference, "retention file ref")?;
    let name = reference.replace(':', "_");
    ensure_count_at_most(name.len(), MAX_REF_FILE_NAME, "retention ref file name")?;
    Ok(name)
}

fn write_store_value(path: &Path, value: &IOValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, to_text(value)?).map_err(MoltenError::from)
}

fn read_store_value(path: &Path) -> Result<IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn object_value(object_ref: &str, object_kind: &str) -> IOValue {
    record("object", vec![string(object_ref), string(object_kind)])
}

fn parse_object_value(value: &Value<IOValue>) -> Result<(String, String)> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected object record"))?;
    let object_ref = required_string(&fields[0], "object ref")?;
    require_ref(&object_ref, "object ref")?;
    let object_kind = required_string(&fields[1], "object kind")?;
    validate_name(&object_kind, "object kind")?;
    Ok((object_ref, object_kind))
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |number| record("some", vec![u64_value(number)]))
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    let mut checks = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let check_value = value_to_iovalue(entry);
        let check_fields = check_value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
        push_bounded(
            &mut checks,
            (required_string(&check_fields[0], "check name")?, required_string(&check_fields[1], "check status")?),
            MAX_RETENTION_REFS,
            "retention checks",
        )?;
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing pass check {name}")))
    }
}

fn live_send_publish_ref(send: &node_daemon::NodeControlLiveSendReceipt) -> String {
    send.transport_receipt_ref.clone().unwrap_or_else(|| send.receipt_ref.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeLiveTransportReceipt {
    receipt_ref: String,
    operation: String,
    decision: String,
    node_id: String,
    envelope_ref: String,
    ingress_receipt_ref: Option<String>,
    diagnostics: Vec<String>,
}

fn parse_node_live_transport_receipt(value: &IOValue) -> Result<NodeLiveTransportReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA, "node control live transport receipt")?;
    require_check(&parse_checks(&fields[10])?, "transport-is-not-authority", "node control live transport")?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    Ok(NodeLiveTransportReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision,
        node_id: record_string(&fields[5], "node")?,
        envelope_ref: record_ref(&fields[7], "envelope")?,
        ingress_receipt_ref: record_optional_ref(&fields[8], "ingress-receipt")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
    })
}

fn node_live_control_diagnostics(
    phase: &str,
    control: &node_runtime::NodeControlRequest,
    expected_target_ref: &str,
    expected_payload_ref: Option<&str>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if control.operation != "gate" {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-operation:{}", control.operation));
    }
    if control.target_ref.as_deref() != Some(expected_target_ref) {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-target"));
    }
    if control.payload_ref.as_deref() != expected_payload_ref {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-payload"));
    }
    diagnostics
}

fn node_live_send_diagnostics(phase: &str, send: &node_daemon::NodeControlLiveSendReceipt) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(send.diagnostics.len().saturating_add(2));
    for diagnostic in &send.diagnostics {
        diagnostics.push(format!("remote-clearance-live-{phase}:{diagnostic}"));
    }
    if send.decision != "pass" {
        diagnostics.push(format!("remote-clearance-live-{phase}-send-deny:{}", send.decision));
    }
    if send.transport_receipt_ref.is_none() {
        diagnostics.push(format!("remote-clearance-live-{phase}-missing-transport-receipt"));
    }
    diagnostics
}

fn node_live_transport_diagnostics(phase: &str, value: &IOValue) -> Result<Vec<String>> {
    let receipt = parse_node_live_transport_receipt(value)?;
    node_live_transport_diagnostics_from(phase, &receipt)
}

fn node_live_transport_diagnostics_from(phase: &str, receipt: &NodeLiveTransportReceipt) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in &receipt.diagnostics {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}:{diagnostic}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    if receipt.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}-transport-deny:{}:{}", receipt.operation, receipt.decision),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn node_live_receive_binding_diagnostics(
    phase: &str,
    send: &node_daemon::NodeControlLiveSendReceipt,
    receive: &NodeLiveTransportReceipt,
    expected_ingress_ref: &str,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if receive.operation != "receive" {
        diagnostics.push(format!("remote-clearance-live-{phase}-not-receive:{}", receive.operation));
    }
    if receive.envelope_ref != send.envelope_ref {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-envelope"));
    }
    if receive.node_id != send.to_node {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-node"));
    }
    if receive.ingress_receipt_ref.as_deref() != Some(expected_ingress_ref) {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-ingress"));
    }
    diagnostics
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let inner = value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[0], label)?;
    require_ref(&reference, label)?;
    Ok(Some(reference))
}

fn record_optional_u64(value: &Value<IOValue>, label: &str) -> Result<Option<u64>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let inner = value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional u64 for {label}")))?;
    let number = some[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))?;
    Ok(Some(number))
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let refs = record_string_sequence(value, label)?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        push_bounded(&mut values, required_string(entry, label)?, MAX_RETENTION_REFS, "retention string sequence")?;
    }
    Ok(values)
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_pass_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    match record_string(value, label)?.as_str() {
        "pass" => Ok(true),
        "deny" => Ok(false),
        other => Err(MoltenError::invalid_harness(format!("expected pass or deny for {label}, got {other}"))),
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {label} {expected}, got {actual}")))
    }
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RETENTION_REFS, label)?;
    for value in values {
        require_ref(value, label)?;
    }
    Ok(())
}

fn validate_diagnostics(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RETENTION_DIAGNOSTICS, label)?;
    for value in values {
        validate_name(value, label)?;
    }
    Ok(())
}

fn require_ref(value: &str, label: &str) -> Result<()> {
    validate_name(value, label)?;
    validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} must be a canonical content ref: {error}")))
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} cannot be empty")));
    }
    ensure_count_at_most(value.len(), MAX_RETENTION_TEXT_LEN, label)
}

fn ensure_count_at_most(count: usize, limit: usize, label: &str) -> Result<()> {
    if count > limit {
        Err(MoltenError::invalid_harness(format!("{label} exceeds limit {limit}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T, S>(values: &mut S, value: T, limit: usize, label: &str) -> Result<()>
where S: VecSink<T> {
    ensure_count_at_most(values.item_count() + 1, limit, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_bounded<T, S, I>(values: &mut S, items: I, limit: usize, label: &str) -> Result<()>
where
    S: VecSink<T>,
    I: IntoIterator<Item = T>,
{
    for item in items {
        push_bounded(values, item, limit, label)?;
    }
    Ok(())
}

fn refs_with_extra(base_refs: &[String], extra_refs: &[String], label: &str) -> Result<Vec<String>> {
    validate_refs(base_refs, label)?;
    validate_refs(extra_refs, label)?;
    let mut refs = base_refs.to_vec();
    extend_bounded(&mut refs, extra_refs.iter().cloned(), MAX_RETENTION_REFS, label)?;
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn push_named<S>(values: &mut S, name: &str, value: IOValue) -> Result<()>
where S: VecSink<(String, IOValue)> {
    push_bounded(values, (name.to_string(), value), MAX_RETENTION_REFS, "retention fixture artifacts")
}

fn pass_or_deny(value: bool) -> &'static str {
    if value { "pass" } else { "deny" }
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("retention-synthetic-ref", vec![string(label)]))
}

const RETENTION_CLASSES: &[&str] = &[
    CLASS_EPHEMERAL_CACHE,
    CLASS_DEBUG_TRACE,
    CLASS_REPLAY_SNAPSHOT,
    CLASS_AUDIT_RECEIPT,
    CLASS_DURABLE_VALUE,
    CLASS_PUBLIC_ARTIFACT,
    CLASS_PRIVATE_SECRET_REF,
    CLASS_UPGRADE_ROLLBACK,
    CLASS_LEGAL_HOLD,
];

const PIN_SOURCES: &[&str] = &[
    SOURCE_ACTIVE_SESSION,
    SOURCE_ARTIFACT,
    SOURCE_BLOB,
    SOURCE_RECEIPT,
    SOURCE_SNAPSHOT,
    SOURCE_TRANSCRIPT,
    SOURCE_DOC,
    SOURCE_POLICY,
    SOURCE_UPGRADE,
    SOURCE_STORAGE_REF,
    SOURCE_REMOTE_CACHE,
    SOURCE_EVALUATION_CACHE,
    SOURCE_OPERATOR_HOLD,
    SOURCE_LEGAL_HOLD,
    SOURCE_SECRET_REDACTION,
];

const RETENTION_ACTIONS: &[&str] = &[
    ACTION_PIN,
    ACTION_UNPIN,
    ACTION_RETAIN,
    ACTION_ELIGIBILITY,
    ACTION_DELETE,
    ACTION_TOMBSTONE,
    ACTION_REDACT,
    ACTION_COMPACT,
];

const ADMISSION_KINDS: &[&str] = &[
    ADMISSION_KIND_POLICY,
    ADMISSION_KIND_AUTHORITY,
    ADMISSION_KIND_SUPPORTING_EVIDENCE,
    ADMISSION_KIND_REFERENCE_INDEX,
    ADMISSION_KIND_REMOTE_GC,
];

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::Ipv4Addr;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use n0_future::StreamExt;

    use super::*;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::record;
    use crate::preserves_rail::string;

    #[test]
    fn pinned_objects_are_not_delete_eligible_until_unpinned() {
        let root = temp_dir("retention-pinned");
        let object_ref = fake_ref("object");
        let owner_ref = fake_ref("owner");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let pin = pin_object(&root, RetentionPinInput {
            object_ref: object_ref.clone(),
            object_kind: "artifact".to_string(),
            retention_class: CLASS_PUBLIC_ARTIFACT.to_string(),
            source: SOURCE_ARTIFACT.to_string(),
            reason: "installed artifact".to_string(),
            owner_ref: owner_ref.clone(),
            expiry_ref: None,
            policy_refs: policy_refs.clone(),
            evidence_refs: evidence_refs.clone(),
            has_authority: true,
        })
        .expect("pin");
        let denied = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            requester_ref: &owner_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("deny delete");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
        let unpin = unpin_object(UnpinObjectInput {
            root: &root,
            pin_ref: &pin.pin.pin_ref,
            requester_ref: &owner_ref,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_authority: true,
        })
        .expect("unpin");
        assert_eq!(unpin.decision, "pass");
        let allowed = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_TOMBSTONE,
            requester_ref: &owner_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("allow tombstone");
        assert_eq!(allowed.receipt.decision, "pass");
        assert!(allowed.tombstone.is_some());
    }

    #[test]
    fn incomplete_reference_proof_denies_gc() {
        let root = temp_dir("retention-incomplete");
        let object_ref = fake_ref("object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let receipt = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_AUDIT_RECEIPT,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: false,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("incomplete deny")
        .receipt;
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "incomplete-reference-proof"));
    }

    #[test]
    fn retained_dependencies_and_legal_holds_deny_deletion() {
        let root = temp_dir("retention-retained");
        let object_ref = fake_ref("object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let retained_refs = vec![fake_ref("receipt-dependency")];
        let retained = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_AUDIT_RECEIPT,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &retained_refs,
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("retained deny");
        assert_eq!(retained.receipt.decision, "deny");
        let legal = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_LEGAL_HOLD,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("legal deny");
        assert_eq!(legal.receipt.decision, "deny");
    }

    #[test]
    fn tombstone_summary_preserves_audit_without_secret_content() {
        let root = temp_dir("retention-tombstone");
        let object_ref = fake_ref("secret-object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("redaction")];
        let evaluation = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "encrypted-ref",
            retention_class: CLASS_PRIVATE_SECRET_REF,
            action: ACTION_REDACT,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("redact");
        let tombstone = evaluation.tombstone.expect("tombstone");
        let text = to_text(&tombstone.value).expect("text");
        assert!(text.contains("redacted-or-deleted"));
        assert!(!text.contains("plaintext"));
        let summary = retention_summary(&tombstone.value).expect("summary");
        assert!(summary.contains("retention tombstone"));
    }

    #[test]
    fn hegel_like_no_dangling_retained_ref_and_deny_on_incomplete_proof() {
        for count in 0..8_u64 {
            let root = temp_dir("retention-hegel-like");
            let object_ref = fake_ref(&format!("object-{count}"));
            let requester_ref = fake_ref("requester");
            let policy_refs = vec![fake_ref("policy")];
            let evidence_refs = vec![fake_ref("evidence")];
            let retained_refs =
                (0..count).map(|index| fake_ref(&format!("retained-{count}-{index}"))).collect::<Vec<_>>();
            let evaluation = evaluate_retention(RetentionEvaluationInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: "audit-receipt",
                retention_class: CLASS_AUDIT_RECEIPT,
                action: ACTION_DELETE,
                requester_ref: &requester_ref,
                is_reference_index_complete: count % 2 == 0,
                retained_refs: &retained_refs,
                remote_refs: &[],
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_delete_authority: true,
                has_remote_gc_clearance: true,
            })
            .expect("evaluate");
            if count == 0 {
                assert_eq!(evaluation.receipt.decision, "pass");
            } else {
                assert_eq!(evaluation.receipt.decision, "deny");
            }
        }
    }

    #[test]
    fn destructive_admission_rejects_forged_and_mismatched_refs() {
        let root = temp_dir("retention-admission-forged");
        let requester_ref = fake_ref("requester");
        let object_ref = fake_ref("object");
        let wrong_object_ref = fake_ref("wrong-object");
        let wrong_policy = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_POLICY,
            label: "wrong-policy",
            requester_ref: &requester_ref,
            object_ref: &wrong_object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let evidence = DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref),
            policy_refs: vec![wrong_policy],
            authority_refs: vec![fake_ref("forged-authority")],
            evidence_refs: vec![fake_ref("forged-evidence")],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![fake_ref("forged-index")],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        };
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root: &root,
            evidence: &evidence,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
        })
        .expect("admission denial");
        assert_eq!(admission.decision, "deny");
        assert!(!admission.has_delete_authority);
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("scope-mismatch")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("unreadable")));
    }

    #[test]
    fn destructive_admission_rejects_stale_and_revoked_refs() {
        let root = temp_dir("retention-admission-stale");
        let requester_ref = fake_ref("requester");
        let object_ref = fake_ref("object");
        let stale_authority =
            scoped_ref(&root, ADMISSION_KIND_AUTHORITY, "stale-authority", &requester_ref, &object_ref, false, &[
                fake_ref("revocation"),
            ]);
        let policy = scoped_ref(&root, ADMISSION_KIND_POLICY, "policy", &requester_ref, &object_ref, true, &[]);
        let support =
            scoped_ref(&root, ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", &requester_ref, &object_ref, true, &[]);
        let index = scoped_ref(&root, ADMISSION_KIND_REFERENCE_INDEX, "index", &requester_ref, &object_ref, true, &[]);
        let evidence = DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref),
            policy_refs: vec![policy],
            authority_refs: vec![stale_authority],
            evidence_refs: vec![support],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![index],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        };
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root: &root,
            evidence: &evidence,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
        })
        .expect("admission denial");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("revoked")));
    }

    #[test]
    fn destructive_admission_accepts_matching_remote_gc_refs() {
        let root = temp_dir("retention-admission-remote");
        let fixture = store_passing_plan_fixture(&root, "admission-remote");
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root: &root,
            evidence: &fixture.evidence,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("admission pass");
        assert_eq!(admission.decision, "pass");
        assert!(admission.has_delete_authority);
        assert!(admission.has_remote_gc_clearance);
        let evaluation = evaluate_retention(RetentionEvaluationInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            requester_ref: &fixture.requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &fixture.evidence.remote_refs,
            policy_refs: &fixture.evidence.policy_refs,
            evidence_refs: &fixture.evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })
        .expect("evaluate remote clearance");
        assert_eq!(evaluation.receipt.decision, "pass");
    }

    #[test]
    fn gc_plan_lists_gates_and_avoids_receipts_or_tombstones() {
        let root = temp_dir("retention-gc-plan-pass");
        let fixture = store_passing_plan_fixture(&root, "plan-pass");
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "chunk-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        assert_eq!(plan.decision, "pass");
        assert_eq!(store_file_count(&receipts_dir(&root)), 0);
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
        assert_eq!(store_file_count(&gc_plans_dir(&root)), 1);
        let gate_names = plan.gates.iter().map(|gate| gate.name.as_str()).collect::<Vec<_>>();
        assert!(gate_names.contains(&"policy"));
        assert!(gate_names.contains(&"authority"));
        assert!(gate_names.contains(&"reference-index"));
        assert!(gate_names.contains(&"remote-gc"));
        assert!(gate_names.contains(&"remote-clearance"));
        let parsed = parse_retention_gc_plan(&plan.value).expect("parse plan");
        assert_eq!(parsed.plan_ref, plan.plan_ref);
    }

    #[test]
    fn gc_plan_rejects_requester_evidence_mismatch() {
        let root = temp_dir("retention-gc-plan-requester-mismatch");
        let fixture = store_passing_plan_fixture(&root, "plan-requester-mismatch");
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store requester-bound plan");
        let mut mismatched_evidence = fixture.evidence.clone();
        mismatched_evidence.requester_ref = Some(fake_ref("wrong-plan-requester"));
        let evidence_value =
            destructive_retention_evidence_value(&mismatched_evidence).expect("mismatched evidence value");
        let index = reference_index_for_object(ReferenceIndexForObjectInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retained_refs: &fixture.evidence.retained_refs,
            remote_refs: &fixture.evidence.remote_refs,
            is_complete: fixture.evidence.is_reference_index_complete,
        })
        .expect("reference index");
        let tampered = retention_gc_plan_value(&RetentionGcPlanValueInput {
            decision: &plan.decision,
            subsystem: &plan.subsystem,
            action: &plan.action,
            object_ref: &plan.object_ref,
            object_kind: &plan.object_kind,
            retention_class: &plan.retention_class,
            requester_ref: plan.requester_ref.as_deref(),
            index: &index,
            evidence_value: &evidence_value,
            gates: &plan.gates,
            diagnostics: &plan.diagnostics,
        })
        .expect("tampered plan value");
        let error = parse_retention_gc_plan(&tampered).expect_err("requester mismatch must fail closed");
        assert!(format!("{error}").contains("requester evidence mismatch"));
    }

    #[test]
    fn gc_plan_denies_missing_clearance_and_is_not_clearance() {
        let root = temp_dir("retention-gc-plan-deny");
        let fixture = store_passing_plan_fixture(&root, "plan-deny");
        let mut evidence = fixture.evidence.clone();
        evidence.remote_clearance_refs.clear();
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &evidence,
        })
        .expect("store denied plan");
        assert_eq!(plan.decision, "deny");
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-evidence-missing"));
        let mut plan_as_clearance = evidence;
        plan_as_clearance.remote_clearance_refs = vec![plan.plan_ref];
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root: &root,
            evidence: &plan_as_clearance,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("plan ref is not clearance");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("remote-clearance-unreadable")));
    }

    #[test]
    fn gc_apply_from_plan_writes_apply_receipt_and_tombstone() {
        let root = temp_dir("retention-gc-apply-pass");
        let fixture = store_passing_plan_fixture(&root, "apply-pass");
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store apply plan");
        let apply = apply_retention_gc_plan(RetentionGcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        assert_eq!(apply.decision, "pass");
        assert_eq!(apply.plan_ref, plan.plan_ref);
        assert_eq!(apply.recomputed_plan_ref, plan.plan_ref);
        assert!(apply.retention_receipt_ref.is_some());
        assert!(apply.tombstone_ref.is_some());
        assert_eq!(store_file_count(&gc_applies_dir(&root)), 1);
        assert_eq!(store_file_count(&tombstones_dir(&root)), 1);
        let parsed = parse_retention_gc_apply(&apply.value).expect("parse apply");
        assert_eq!(parsed.apply_ref, apply.apply_ref);
        read_retention_receipt(&root, parsed.retention_receipt_ref.as_deref().expect("receipt ref"))
            .expect("read retention receipt");
    }

    #[test]
    fn gc_audit_binds_plan_apply_execution_receipt_and_tombstone() {
        let root = temp_dir("retention-gc-audit-pass");
        let fixture = store_passing_plan_fixture(&root, "audit-pass");
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store audit plan");
        let apply = apply_retention_gc_plan(RetentionGcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply audit plan");
        let execution = store_retention_gc_execution_gate(RetentionGcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store execution gate");
        assert_eq!(execution.decision, "pass");
        let audit = audit_retention_gc_execution(RetentionGcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit execution");
        assert_eq!(audit.decision, "pass");
        assert_eq!(audit.plan_ref.as_deref(), Some(plan.plan_ref.as_str()));
        assert_eq!(audit.apply_ref.as_deref(), Some(apply.apply_ref.as_str()));
        assert_eq!(audit.execution_ref, execution.execution_ref);
        assert_eq!(audit.retention_receipt_ref, apply.retention_receipt_ref);
        assert_eq!(audit.tombstone_ref, apply.tombstone_ref);
        assert_eq!(store_file_count(&gc_audits_dir(&root)), 1);
        assert!(retention_summary(&audit.value).expect("audit summary").contains("retention gc audit"));
    }

    #[test]
    fn candidate_explain_lists_known_retention_gc_evidence() {
        let root = temp_dir("retention-candidate-explain");
        let fixture = store_passing_plan_fixture(&root, "explain-pass");
        let flow = passing_flow(&root, &fixture, "ledger-gc");
        let explain = explain_retention_candidate(RetentionCandidateExplainInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: Some("chunk"),
            retention_class: Some(CLASS_DURABLE_VALUE),
            action: Some(ACTION_DELETE),
            subsystem: Some("ledger-gc"),
        })
        .expect("explain retention candidate");
        assert_eq!(explain.pin_refs.len(), 0);
        assert_eq!(explain.admission_refs.len(), 5);
        assert_eq!(explain.remote_clearance_refs.len(), 1);
        assert_eq!(explain.gc_plan_refs, vec![flow.plan.plan_ref.clone()]);
        assert_eq!(explain.gc_apply_refs, vec![flow.apply.apply_ref.clone()]);
        assert_eq!(explain.gc_execution_refs, vec![flow.execution.execution_ref.clone()]);
        assert_eq!(explain.gc_audit_refs, vec![flow.audit.audit_ref.clone()]);
        assert_eq!(explain.retention_receipt_refs.len(), 1);
        assert_eq!(explain.tombstone_refs.len(), 1);
        assert!(explain.diagnostics.is_empty());
        assert!(retention_summary(&explain.value).expect("explain summary").contains("retention candidate explain"));
        let bundle_dir = root.join("bundle");
        let bundle = export_retention_candidate_bundle(RetentionCandidateBundleExportInput {
            root: &root,
            explain_value: &explain.value,
            out: &bundle_dir,
            profile: RetentionCandidateBundleExportProfile::Internal,
        })
        .expect("export retention candidate bundle");
        assert_eq!(bundle.explain_ref, explain.explain_ref);
        assert_eq!(bundle.artifact_refs.len(), 6);
        assert!(bundle.diagnostics.is_empty());
        assert!(bundle_dir.join("bundle.preserves").exists());
        assert!(bundle_dir.join("explain.preserves").exists());
        assert!(bundle_dir.join("artifacts/gc-plans").exists());
        assert!(retention_summary(&bundle.value).expect("bundle summary").contains("retention candidate bundle"));
        let verify = verify_retention_candidate_bundle(RetentionCandidateBundleVerifyInput {
            bundle_dir: &bundle_dir,
        })
        .expect("verify intact retention candidate bundle");
        assert_eq!(verify.decision, "pass");
        assert_eq!(verify.bundle_ref, bundle.bundle_ref);
        assert_eq!(verify.explain_ref, explain.explain_ref);
        assert_eq!(verify.artifact_refs.len(), 6);
        assert_eq!(verify.file_refs.len(), 6);
        assert!(verify.diagnostics.is_empty());
        assert!(
            retention_summary(&verify.value)
                .expect("bundle verify summary")
                .contains("retention candidate bundle verify")
        );
        let tampered_path = bundle_dir
            .join("artifacts/gc-plans")
            .join(format!("{}.preserves", ref_file_name(&flow.plan.plan_ref).expect("plan file name")));
        write_store_value(&tampered_path, &record("tampered", vec![string("plan")])).expect("tamper bundle plan");
        let tampered = verify_retention_candidate_bundle(RetentionCandidateBundleVerifyInput {
            bundle_dir: &bundle_dir,
        })
        .expect("verify tampered retention candidate bundle");
        assert_eq!(tampered.decision, "deny");
        assert!(
            tampered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
        );
    }

    #[test]
    fn candidate_bundle_reports_missing_local_artifacts() {
        let root = temp_dir("retention-bundle-missing");
        let missing_ref = fake_ref("missing-plan");
        let object_ref = fake_ref("bundle-object");
        let explain_value = retention_candidate_explain_value(&RetentionCandidateExplainValueInput {
            object_ref: &object_ref,
            object_kind: Some("encrypted-ref"),
            retention_class: Some("private-secret-ref"),
            action: Some("delete"),
            subsystem: Some("ledger-gc"),
            pin_refs: &[],
            admission_refs: &[],
            remote_clearance_refs: &[],
            remote_clearance_import_refs: &[],
            gc_plan_refs: std::slice::from_ref(&missing_ref),
            gc_apply_refs: &[],
            gc_execution_refs: &[],
            gc_audit_refs: &[],
            retention_receipt_refs: &[],
            tombstone_refs: &[],
            diagnostics: &[],
        })
        .expect("explain value");
        let bundle = export_retention_candidate_bundle(RetentionCandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &root.join("bundle"),
            profile: RetentionCandidateBundleExportProfile::Internal,
        })
        .expect("bundle with missing artifact diagnostic");
        assert!(bundle.artifact_refs.is_empty());
        assert_eq!(bundle.diagnostics, vec![format!("retention-bundle-missing-artifact:{missing_ref}")]);
        let verify = verify_retention_candidate_bundle(RetentionCandidateBundleVerifyInput {
            bundle_dir: &root.join("bundle"),
        })
        .expect("verify missing artifact bundle");
        assert_eq!(verify.decision, "deny");
        assert!(
            verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-missing-file:gc-plans"))
        );
    }

    #[test]
    fn candidate_bundle_profiles_deny_or_redact_sensitive_handoff() {
        let root = temp_dir("retention-bundle-profile");
        let object_ref = fake_ref("bundle-profile-object");
        let plan_ref = fake_ref("bundle-profile-plan");
        let explain_value = sensitive_explain_value(&object_ref, &plan_ref);
        let public_dir = root.join("public");
        let public_bundle = export_retention_candidate_bundle(RetentionCandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &public_dir,
            profile: RetentionCandidateBundleExportProfile::Public,
        })
        .expect("public profile bundle export");
        let public_profile = parse_retention_candidate_bundle_profile(
            &read_store_value(&public_dir.join(BUNDLE_PROFILE_FILE)).expect("read public bundle profile"),
        )
        .expect("parse public profile");
        assert_eq!(public_profile.bundle_ref, public_bundle.bundle_ref);
        assert_eq!(public_profile.profile, "public");
        assert_eq!(public_profile.decision, "deny");
        assert!(!public_profile.marker_refs.is_empty());
        assert!(
            public_profile
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-public-sensitive-markers"))
        );

        let diagnostic_dir = root.join("diagnostic");
        let diagnostic_bundle = export_retention_candidate_bundle(RetentionCandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &diagnostic_dir,
            profile: RetentionCandidateBundleExportProfile::Diagnostic,
        })
        .expect("diagnostic profile bundle export");
        let diagnostic_profile = parse_retention_candidate_bundle_profile(
            &read_store_value(&diagnostic_dir.join(BUNDLE_PROFILE_FILE)).expect("read diagnostic bundle profile"),
        )
        .expect("parse diagnostic profile");
        assert_eq!(diagnostic_profile.bundle_ref, diagnostic_bundle.bundle_ref);
        assert_eq!(diagnostic_profile.profile, "diagnostic");
        assert_eq!(diagnostic_profile.decision, "pass");
        assert!(!diagnostic_profile.marker_refs.is_empty());
        let redacted_explain = fs::read_to_string(diagnostic_dir.join(BUNDLE_REDACTED_DIR).join("explain.preserves"))
            .expect("read redacted explain");
        assert!(!redacted_explain.contains(CLASS_PRIVATE_SECRET_REF));
        assert!(!redacted_explain.contains("encrypted-ref"));
        let verify = verify_retention_candidate_bundle(RetentionCandidateBundleVerifyInput {
            bundle_dir: &diagnostic_dir,
        })
        .expect("verify diagnostic source bundle");
        assert_eq!(verify.decision, "deny");
    }

    #[test]
    fn gc_audit_denies_missing_chain_links_without_authority() {
        let root = temp_dir("retention-gc-audit-deny");
        let object_ref = fake_ref("audit-missing-object");
        let execution = store_retention_gc_execution_gate(RetentionGcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: None,
        })
        .expect("store denied execution gate");
        let audit = audit_retention_gc_execution(RetentionGcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit missing links");
        assert_eq!(audit.decision, "deny");
        assert!(audit.plan_ref.is_none());
        assert!(audit.apply_ref.is_none());
        assert!(audit.retention_receipt_ref.is_none());
        assert!(audit.tombstone_ref.is_none());
        assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-apply-missing"));
        assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-plan-missing"));
    }

    #[test]
    fn gc_apply_from_plan_denies_drift_before_tombstone() {
        let root = temp_dir("retention-gc-apply-drift");
        let fixture = store_passing_plan_fixture(&root, "apply-drift");
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "chunk-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store drift plan");
        let pin = pin_object(&root, RetentionPinInput {
            object_ref: fixture.object_ref.clone(),
            object_kind: "chunk".to_string(),
            retention_class: CLASS_DURABLE_VALUE.to_string(),
            source: SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold after plan".to_string(),
            owner_ref: fixture.requester_ref.clone(),
            expiry_ref: None,
            policy_refs: fixture.evidence.policy_refs.clone(),
            evidence_refs: fixture.evidence.evidence_refs.clone(),
            has_authority: true,
        })
        .expect("pin after plan");
        assert_eq!(pin.receipt.decision, "pass");
        let receipt_count = store_file_count(&receipts_dir(&root));
        let apply = apply_retention_gc_plan(RetentionGcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply drift plan");
        assert_eq!(apply.decision, "deny");
        assert!(apply.retention_receipt_ref.is_none());
        assert!(apply.tombstone_ref.is_none());
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-drift"));
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
        assert_eq!(store_file_count(&receipts_dir(&root)), receipt_count);
    }

    #[test]
    fn gc_apply_from_denied_plan_writes_only_apply_receipt() {
        let root = temp_dir("retention-gc-apply-denied-plan");
        let fixture = store_passing_plan_fixture(&root, "apply-denied-plan");
        let mut evidence = fixture.evidence;
        evidence.remote_clearance_refs = Vec::new();
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &evidence,
        })
        .expect("store denied plan");
        assert_eq!(plan.decision, "deny");
        let apply = apply_retention_gc_plan(RetentionGcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply denied plan");
        assert_eq!(apply.decision, "deny");
        assert!(apply.retention_receipt_ref.is_none());
        assert!(apply.tombstone_ref.is_none());
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-not-pass"));
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
    }

    #[test]
    fn destructive_admission_rejects_unreconciled_remote_clearance() {
        let root = temp_dir("retention-admission-remote-deny");
        let case = deny_case(&root);
        let refs = denial_refs(&root, &case);

        let mut partial = case.base();
        partial.remote_clearance_refs = vec![refs.partial];
        assert_denial(&root, &case, &partial, "partial remote denial", &["missing-remote", "missing-peer"]);

        let wrong_peer = case.scoped(refs.wrong_peer);
        assert_denial(&root, &case, &wrong_peer, "wrong peer denial", &["missing-peer"]);

        let stale = case.scoped(refs.stale);
        assert_denial(&root, &case, &stale, "stale remote denial", &["stale", "revoked"]);

        let retained = case.scoped(refs.retained);
        assert_denial(&root, &case, &retained, "retained remote denial", &["retained"]);

        let forged = case.scoped(fake_ref("forged-clearance"));
        assert_denial(&root, &case, &forged, "forged remote denial", &["unreadable"]);
    }

    #[test]
    fn remote_clearance_workflow_imports_peer_clearance_and_denies_wrong_request() {
        let root = temp_dir("retention-remote-clearance-workflow");
        let case = live_case(&root, "workflow");
        let pair = pair_with_label(&root, &case, "workflow-peer-evidence");

        let import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
            root: &root,
            request_value: &pair.request_value,
            response_value: &pair.response_value,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("import clearance");
        assert_eq!(import.decision, "pass");
        let clearance_ref = import.clearance_ref.clone().expect("clearance imported");
        assert_case_pass(&root, &case, clearance_ref);

        let wrong_request =
            store_retention_remote_gc_clearance_request(&root, &RetentionRemoteGcClearanceRequestInput {
                requester_ref: &case.requester,
                peer_ref: &case.peer,
                object_ref: &fake_ref("workflow-wrong-object"),
                object_kind: "chunk",
                retention_class: CLASS_DURABLE_VALUE,
                action: ACTION_DELETE,
                remote_ref: &case.remote,
                policy_ref: &fake_ref("workflow-wrong-policy"),
                authority_ref: &fake_ref("workflow-wrong-authority"),
                evidence_refs: &[],
            })
            .expect("store wrong request");
        let wrong_import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
            root: &root,
            request_value: &wrong_request.value,
            response_value: &pair.response_value,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("deny wrong request import");
        assert_eq!(wrong_import.decision, "deny");
        assert!(wrong_import.clearance_ref.is_none());
        assert!(wrong_import.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-wrong-request"));

        let tampered_response = record("not-a-remote-clearance-response", vec![string("tampered")]);
        let tampered_import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
            root: &root,
            request_value: &pair.request_value,
            response_value: &tampered_response,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("deny tampered response import");
        assert_eq!(tampered_import.decision, "deny");
        assert!(tampered_import.clearance_ref.is_none());
        assert!(
            tampered_import
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("remote-clearance-tampered-response"))
        );
    }

    #[test]
    fn remote_clearance_live_loopback_imports_peer_clearance_for_destructive_admission() {
        let root = temp_dir("retention-remote-clearance-live-loopback");
        let requester_node_root = temp_dir("retention-live-requester-node");
        let peer_node_root = temp_dir("retention-live-peer-node");
        let requester_node_id = "retention-live-requester";
        let peer_node_id = "retention-live-peer";
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &requester_node_root,
            node_id: requester_node_id,
        })
        .expect("init requester node");
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &peer_node_root,
            node_id: peer_node_id,
        })
        .expect("init peer node");
        let request_live = live_direction_refs(&peer_node_root, requester_node_id, "request");
        let response_live = live_direction_refs(&requester_node_root, peer_node_id, "response");
        let case = live_case(&root, "live");
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        let live = runtime
            .block_on(run_retention_remote_gc_clearance_live_loopback(RetentionRemoteGcClearanceLiveLoopbackInput {
                root: &root,
                requester_node_root: &requester_node_root,
                peer_node_root: &peer_node_root,
                requester_node_id,
                peer_node_id,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                request_sequence: 1,
                response_sequence: 1,
                requester_ref: &case.requester,
                peer_ref: &case.peer,
                object_ref: &case.object,
                object_kind: "chunk",
                retention_class: CLASS_DURABLE_VALUE,
                action: ACTION_DELETE,
                remote_ref: &case.remote,
                policy_ref: &case.policy,
                authority_ref: &case.authority,
                retention_evidence_refs: std::slice::from_ref(&case.support),
                response_evidence_refs: &[fake_ref("live-peer-evidence")],
                retained_refs: &[],
                is_current: true,
                revoked_refs: &[],
                response_diagnostics: &[],
                request_peer_bootstrap_refs: &request_live.peer_bootstrap_refs,
                request_authority_refs: &request_live.authority_refs,
                request_policy_refs: &request_live.policy_refs,
                request_resource_refs: &request_live.resource_refs,
                request_transport_evidence_refs: &request_live.evidence_refs,
                response_peer_bootstrap_refs: &response_live.peer_bootstrap_refs,
                response_authority_refs: &response_live.authority_refs,
                response_policy_refs: &response_live.policy_refs,
                response_resource_refs: &response_live.resource_refs,
                response_transport_evidence_refs: &response_live.evidence_refs,
            }))
            .expect("live loopback");
        assert_eq!(live.workflow.decision, "pass");
        assert_eq!(live.import.decision, "pass");
        let clearance_ref = live.import.clearance_ref.clone().expect("live clearance imported");
        assert_case_pass(&root, &case, clearance_ref);
    }

    #[test]
    fn remote_clearance_live_multihost_request_and_response_send_write_artifacts_on_denied_transport() {
        let case = no_endpoint_case();
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

        let request = case_request(&runtime, &case);
        assert_eq!(request.request.peer_ref, case.peer);
        assert_send_denial(&request.send.send_receipt_value, Some("ticket has no endpoint addresses"));

        let response = case_response(&runtime, &case, &request);
        assert_eq!(response.response.request_ref, request.request.request_ref);
        assert_send_denial(&response.send.send_receipt_value, None);
    }

    #[test]
    fn remote_clearance_live_multihost_import_workflow_binds_explicit_send_receive_evidence() {
        let root = temp_dir("retention-remote-clearance-live-multihost");
        let material = fixture_material(&root);
        let clearance_ref = assert_import_pass(&root, &material);
        let wrong_request_receive = fake_live_transport_receipt(
            "publish",
            "wrong-peer-node",
            "wrong-request-envelope",
            &fake_ref("wrong-request-ingress"),
        );
        assert_wrong_receive(&root, &material, &wrong_request_receive);
        assert_case_pass(&root, &material.case, clearance_ref);
    }

    #[test]
    fn remote_clearance_live_multihost_two_node_happy_path_uses_real_receive_evidence() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let mut live = two_node_live().await;
            let refs = two_node_refs(&live.roots.requester_store);
            let request = send_two_node_request(&mut live, &refs).await;
            let response = send_two_node_response(&mut live, &request).await;
            let imported = import_two_node_workflow(&live.roots.requester_store, &refs, &request, &response);
            assert_two_node_import(&imported, &request, &response);
            assert_two_node_admission(&live.roots.requester_store, refs, imported);
            live.shutdown().await;
        });
    }

    #[test]
    fn remote_clearance_live_workflow_denies_retained_wrong_peer_and_tampered_response() {
        let root = temp_dir("retention-remote-clearance-live-deny");
        let requester_ref = fake_ref("live-deny-requester");
        let peer_ref = fake_ref("live-deny-peer");
        let object_ref = fake_ref("live-deny-object");
        let remote_ref = fake_ref("live-deny-remote");
        let policy = fake_ref("live-deny-policy");
        let authority = fake_ref("live-deny-authority");
        let request = store_retention_remote_gc_clearance_request(&root, &RetentionRemoteGcClearanceRequestInput {
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &remote_ref,
            policy_ref: &policy,
            authority_ref: &authority,
            evidence_refs: &[],
        })
        .expect("store live deny request");
        let live_refs = fake_live_refs("retained");

        assert_retained(&root, &request, &remote_ref, &live_refs);
        assert_tampered(&root, &request, &peer_ref, &remote_ref, &live_refs);
    }

    fn assert_retained(
        root: &Path,
        request: &RetentionRemoteGcClearanceRequest,
        remote_ref: &str,
        live_refs: &[String],
    ) {
        let retained_ref = fake_ref("live-deny-retained");
        let response = store_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceResponseInput {
            root,
            request_value: &request.value,
            evidence_refs: &[],
            retained_refs: std::slice::from_ref(&retained_ref),
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retained response");
        let wrong_peer_import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
            root,
            request_value: &request.value,
            response_value: &response.value,
            expected_peer_ref: Some(&fake_ref("wrong-live-peer")),
            expected_remote_ref: Some(remote_ref),
        })
        .expect("wrong peer import");
        let retained_workflow =
            retention_remote_gc_clearance_live_workflow_value(&RetentionRemoteGcClearanceLiveWorkflowValueInput {
                request_value: &request.value,
                response_value: &response.value,
                import_value: &wrong_peer_import.value,
                request_control_ref: &live_refs[0],
                request_publish_ref: &live_refs[1],
                request_receive_ref: &live_refs[2],
                request_ingress_ref: &live_refs[3],
                response_control_ref: &live_refs[4],
                response_publish_ref: &live_refs[5],
                response_receive_ref: &live_refs[6],
                response_ingress_ref: &live_refs[7],
                transport_diagnostics: &[],
            })
            .expect("retained live workflow value");
        let retained =
            parse_retention_remote_gc_clearance_live_workflow(&retained_workflow).expect("parse retained live");
        assert_eq!(retained.decision, "deny");
        assert!(retained.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-retained"));
        assert!(
            retained
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "remote-clearance-expected-peer-mismatch")
        );
    }

    fn assert_tampered(
        root: &Path,
        request: &RetentionRemoteGcClearanceRequest,
        peer_ref: &str,
        remote_ref: &str,
        live_refs: &[String],
    ) {
        let tampered_response = record("not-a-remote-clearance-response", vec![string("tampered")]);
        let tampered_import = import_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceImportInput {
            root,
            request_value: &request.value,
            response_value: &tampered_response,
            expected_peer_ref: Some(peer_ref),
            expected_remote_ref: Some(remote_ref),
        })
        .expect("tampered import");
        let tampered_workflow =
            retention_remote_gc_clearance_live_workflow_value(&RetentionRemoteGcClearanceLiveWorkflowValueInput {
                request_value: &request.value,
                response_value: &tampered_response,
                import_value: &tampered_import.value,
                request_control_ref: &live_refs[0],
                request_publish_ref: &live_refs[1],
                request_receive_ref: &live_refs[2],
                request_ingress_ref: &live_refs[3],
                response_control_ref: &live_refs[4],
                response_publish_ref: &live_refs[5],
                response_receive_ref: &live_refs[6],
                response_ingress_ref: &live_refs[7],
                transport_diagnostics: &[],
            })
            .expect("tampered live workflow value");
        let tampered =
            parse_retention_remote_gc_clearance_live_workflow(&tampered_workflow).expect("parse tampered live");
        assert_eq!(tampered.decision, "deny");
        assert!(
            tampered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("remote-clearance-live-tampered-response"))
        );
    }

    struct LiveNodeHarness {
        ticket: crate::node_daemon::NodeControlLiveTicket,
        topic: iroh_gossip::api::GossipTopic,
        router: iroh::protocol::Router,
    }

    struct LiveDirectionEvidenceInput<'a> {
        sender_root: &'a Path,
        receiver_root: &'a Path,
        receiver_ticket: &'a crate::node_daemon::NodeControlLiveTicket,
        sender_node_id: &'a str,
        receiver_node_id: &'a str,
        topic: &'a str,
        policy_refs: &'a [String],
    }

    struct LiveDirectionEvidence {
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }

    struct TwoNodeRoots {
        requester_store: PathBuf,
        peer_store: PathBuf,
        requester_node: PathBuf,
        peer_node: PathBuf,
    }

    struct TwoNodeLive {
        roots: TwoNodeRoots,
        topic: &'static str,
        peer_live: LiveNodeHarness,
        requester_live: LiveNodeHarness,
        control_policy_refs: Vec<String>,
        control_resource_refs: Vec<String>,
        request_evidence: LiveDirectionEvidence,
        response_evidence: LiveDirectionEvidence,
    }

    struct TwoNodeRefs {
        requester_ref: String,
        peer_ref: String,
        object_ref: String,
        remote_ref: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        remote_gc: String,
    }

    struct TwoNodeAdmissionInput<'a> {
        root: &'a Path,
        kind: &'a str,
        label: &'a str,
        requester_ref: &'a str,
        object_ref: &'a str,
        remote_refs: &'a [String],
    }

    struct SentRequest {
        send: RetentionRemoteGcClearanceLiveRequestSend,
        receive: crate::node_daemon::NodeControlLiveIngressReceive,
    }

    struct SentResponse {
        send: RetentionRemoteGcClearanceLiveResponseSend,
        receive: crate::node_daemon::NodeControlLiveIngressReceive,
    }

    impl TwoNodeLive {
        async fn shutdown(self) {
            self.peer_live.router.shutdown().await.expect("peer router shutdown");
            self.requester_live.router.shutdown().await.expect("requester router shutdown");
        }
    }

    fn two_node_roots() -> TwoNodeRoots {
        let roots = TwoNodeRoots {
            requester_store: temp_dir("retention-remote-clearance-live-two-node-requester-store"),
            peer_store: temp_dir("retention-remote-clearance-live-two-node-peer-store"),
            requester_node: temp_dir("retention-remote-clearance-live-two-node-requester-node"),
            peer_node: temp_dir("retention-remote-clearance-live-two-node-peer-node"),
        };
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &roots.requester_node,
            node_id: "requester-node",
        })
        .expect("init requester node");
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &roots.peer_node,
            node_id: "peer-node",
        })
        .expect("init peer node");
        crate::node_daemon::run_local_node(&crate::node_daemon::NodeDaemonRunInput {
            state_root: &roots.requester_node,
        })
        .expect("run requester node");
        crate::node_daemon::run_local_node(&crate::node_daemon::NodeDaemonRunInput {
            state_root: &roots.peer_node,
        })
        .expect("run peer node");
        roots
    }

    async fn two_node_live() -> TwoNodeLive {
        let roots = two_node_roots();
        let topic = crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC;
        let peer_live = start_bound_live_node(&roots.peer_node, topic).await;
        let requester_live = start_bound_live_node(&roots.requester_node, topic).await;
        let control_policy_refs = vec![fake_ref("two-node-control-policy")];
        let control_resource_refs = vec![fake_ref("two-node-control-resource")];
        let request_evidence = install_live_direction_evidence(&LiveDirectionEvidenceInput {
            sender_root: &roots.requester_node,
            receiver_root: &roots.peer_node,
            receiver_ticket: &peer_live.ticket,
            sender_node_id: "requester-node",
            receiver_node_id: "peer-node",
            topic,
            policy_refs: &control_policy_refs,
        });
        let response_evidence = install_live_direction_evidence(&LiveDirectionEvidenceInput {
            sender_root: &roots.peer_node,
            receiver_root: &roots.requester_node,
            receiver_ticket: &requester_live.ticket,
            sender_node_id: "peer-node",
            receiver_node_id: "requester-node",
            topic,
            policy_refs: &control_policy_refs,
        });
        TwoNodeLive {
            roots,
            topic,
            peer_live,
            requester_live,
            control_policy_refs,
            control_resource_refs,
            request_evidence,
            response_evidence,
        }
    }

    fn two_node_refs(root: &Path) -> TwoNodeRefs {
        let requester_ref = fake_ref("two-node-requester");
        let peer_ref = fake_ref("two-node-peer");
        let object_ref = fake_ref("two-node-object");
        let remote_ref = fake_ref("two-node-remote");
        let policy = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_POLICY,
            label: "two-node-policy",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let authority = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_AUTHORITY,
            label: "two-node-authority",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let support = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label: "two-node-support",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let index = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_REFERENCE_INDEX,
            label: "two-node-index",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let remote_gc = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_REMOTE_GC,
            label: "two-node-remote-gc",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: std::slice::from_ref(&remote_ref),
        });
        TwoNodeRefs {
            requester_ref,
            peer_ref,
            object_ref,
            remote_ref,
            policy,
            authority,
            support,
            index,
            remote_gc,
        }
    }

    fn store_two_node_admission(input: TwoNodeAdmissionInput<'_>) -> String {
        store_test_admission(TestAdmissionInput {
            root: input.root,
            kind: input.kind,
            label: input.label,
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: input.remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        })
    }

    async fn send_two_node_request(live: &mut TwoNodeLive, refs: &TwoNodeRefs) -> SentRequest {
        let send = send_retention_remote_gc_clearance_live_request(RetentionRemoteGcClearanceLiveRequestSendInput {
            root: &live.roots.requester_store,
            requester_node_root: Some(&live.roots.requester_node),
            peer_ticket_value: &live.peer_live.ticket.value,
            requester_node_id: "requester-node",
            peer_node_id: "peer-node",
            topic: live.topic,
            sequence: 1,
            max_attempts: crate::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            join_timeout_ms: 10_000,
            requester_ref: &refs.requester_ref,
            peer_ref: &refs.peer_ref,
            object_ref: &refs.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &refs.remote_ref,
            policy_ref: &refs.policy,
            authority_ref: &refs.authority,
            retention_evidence_refs: std::slice::from_ref(&refs.support),
            peer_bootstrap_refs: &live.request_evidence.peer_bootstrap_refs,
            authority_refs: &live.request_evidence.authority_refs,
            policy_refs: &live.control_policy_refs,
            resource_refs: &live.control_resource_refs,
            transport_evidence_refs: &[],
        })
        .await
        .expect("two-node request send");
        let receipt = crate::node_daemon::parse_node_control_live_send_receipt(&send.send.send_receipt_value)
            .expect("request send receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(send.send.transport_receipt_ref.is_some());
        let receive =
            receive_one_live_ingress(&live.roots.peer_node, live.topic, "peer-node", &mut live.peer_live.topic).await;
        assert!(receive.has_enqueued);
        assert_eq!(receive.envelope_ref, send.send.envelope_ref);
        SentRequest { send, receive }
    }

    async fn send_two_node_response(live: &mut TwoNodeLive, request: &SentRequest) -> SentResponse {
        let peer_response_evidence = vec![fake_ref("two-node-peer-reference-index")];
        let send = send_retention_remote_gc_clearance_live_response(RetentionRemoteGcClearanceLiveResponseSendInput {
            root: &live.roots.peer_store,
            peer_node_root: Some(&live.roots.peer_node),
            requester_ticket_value: &live.requester_live.ticket.value,
            request_value: &request.send.request.value,
            peer_node_id: "peer-node",
            requester_node_id: "requester-node",
            topic: live.topic,
            sequence: 1,
            max_attempts: crate::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            join_timeout_ms: 10_000,
            response_evidence_refs: &peer_response_evidence,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            response_diagnostics: &[],
            peer_bootstrap_refs: &live.response_evidence.peer_bootstrap_refs,
            authority_refs: &live.response_evidence.authority_refs,
            policy_refs: &live.control_policy_refs,
            resource_refs: &live.control_resource_refs,
            transport_evidence_refs: &[],
        })
        .await
        .expect("two-node response send");
        let receipt = crate::node_daemon::parse_node_control_live_send_receipt(&send.send.send_receipt_value)
            .expect("response send receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(send.send.transport_receipt_ref.is_some());
        let receive = receive_one_live_ingress(
            &live.roots.requester_node,
            live.topic,
            "requester-node",
            &mut live.requester_live.topic,
        )
        .await;
        assert!(receive.has_enqueued);
        assert_eq!(receive.envelope_ref, send.send.envelope_ref);
        SentResponse { send, receive }
    }

    fn import_two_node_workflow(
        root: &Path,
        refs: &TwoNodeRefs,
        request: &SentRequest,
        response: &SentResponse,
    ) -> RetentionRemoteGcClearanceLiveImportWorkflow {
        import_retention_remote_gc_clearance_live_workflow(RetentionRemoteGcClearanceLiveImportWorkflowInput {
            root,
            request_value: &request.send.request.value,
            response_value: &response.send.response.value,
            request_control_value: &request.send.control_value,
            request_send_receipt_value: &request.send.send.send_receipt_value,
            request_receive_receipt_value: &request.receive.transport_receipt_value,
            request_ingress_ref: &request.receive.ingress_receipt_ref,
            response_control_value: &response.send.control_value,
            response_send_receipt_value: &response.send.send.send_receipt_value,
            response_receive_receipt_value: &response.receive.transport_receipt_value,
            response_ingress_ref: &response.receive.ingress_receipt_ref,
            expected_peer_ref: Some(&refs.peer_ref),
            expected_remote_ref: Some(&refs.remote_ref),
        })
        .expect("two-node import workflow")
    }

    fn assert_two_node_import(
        imported: &RetentionRemoteGcClearanceLiveImportWorkflow,
        request: &SentRequest,
        response: &SentResponse,
    ) {
        assert_eq!(imported.import.decision, "pass");
        assert_eq!(imported.workflow.decision, "pass");
        assert!(imported.workflow.diagnostics.is_empty());
        assert_eq!(
            imported.workflow.request_live_refs[1],
            request.send.send.transport_receipt_ref.clone().expect("request publish receipt")
        );
        assert_eq!(imported.workflow.request_live_refs[2], request.receive.transport_receipt_ref);
        assert_eq!(
            imported.workflow.response_live_refs[1],
            response.send.send.transport_receipt_ref.clone().expect("response publish receipt")
        );
        assert_eq!(imported.workflow.response_live_refs[2], response.receive.transport_receipt_ref);
    }

    fn assert_two_node_admission(
        root: &Path,
        refs: TwoNodeRefs,
        imported: RetentionRemoteGcClearanceLiveImportWorkflow,
    ) {
        let clearance_ref = imported.import.clearance_ref.expect("clearance stored");
        let TwoNodeRefs {
            requester_ref,
            peer_ref,
            object_ref,
            remote_ref,
            policy,
            authority,
            support,
            index,
            remote_gc,
        } = refs;
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root,
            evidence: &DestructiveRetentionEvidence {
                requester_ref: Some(requester_ref),
                policy_refs: vec![policy],
                authority_refs: vec![authority],
                evidence_refs: vec![support],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![peer_ref],
                remote_refs: vec![remote_ref],
                reference_index_refs: vec![index],
                remote_gc_refs: vec![remote_gc],
                remote_clearance_refs: vec![clearance_ref],
                is_reference_index_complete: true,
            },
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("two-node destructive admission");
        assert_eq!(admission.decision, "pass");
    }

    async fn start_bound_live_node(state_root: &Path, topic: &str) -> LiveNodeHarness {
        let identity_text = fs::read_to_string(state_root.join("identity.preserves")).expect("node identity file");
        let identity_value = parse_text(&identity_text).expect("parse node identity file");
        let identity = crate::node_identity::parse_node_identity(&identity_value).expect("parse node identity");
        let seed = blake3::hash(
            format!("molten.node-control.live.endpoint.v1:{}:{}", identity.node_id, identity.endpoint_id).as_bytes(),
        );
        let lookup = iroh::address_lookup::memory::MemoryLookup::new();
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .address_lookup(lookup.clone())
            .alpns(vec![iroh_gossip::ALPN.to_vec()])
            .clear_ip_transports()
            .bind_addr((Ipv4Addr::LOCALHOST, 0))
            .expect("live endpoint bind addr")
            .secret_key(iroh::SecretKey::from_bytes(seed.as_bytes()))
            .bind()
            .await
            .expect("live endpoint bind");
        let endpoint_addr = endpoint.addr();
        let live_endpoint_id = format!("iroh:{}", endpoint.id());
        let address_refs = endpoint_addr.addrs.iter().map(ToString::to_string).collect::<Vec<_>>();
        let ticket_value =
            crate::node_daemon::node_control_live_ticket_value(&crate::node_daemon::NodeControlLiveTicketInput {
                node_id: &identity.node_id,
                node_identity_ref: &identity.identity_ref,
                logical_endpoint_id: &identity.endpoint_id,
                live_endpoint_id: &live_endpoint_id,
                topic,
                address_refs: &address_refs,
                policy_refs: &identity.policy_refs,
                evidence_refs: &identity.receipt_refs,
            })
            .expect("bound live ticket value");
        let ticket = crate::node_daemon::parse_node_control_live_ticket(&ticket_value).expect("bound live ticket");
        lookup.add_endpoint_info(endpoint_addr);
        let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
        let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
        let topic = gossip.subscribe(local_live_topic_id(topic), Vec::new()).await.expect("subscribe live topic");
        LiveNodeHarness { ticket, topic, router }
    }

    fn install_live_direction_evidence(input: &LiveDirectionEvidenceInput<'_>) -> LiveDirectionEvidence {
        let admission =
            crate::node_daemon::admit_node_control_live_peer(&crate::node_daemon::NodeControlLivePeerAdmitInput {
                state_root: input.receiver_root,
                ticket_value: &input.receiver_ticket.value,
                peer_id: input.sender_node_id,
                sequence: 1,
                expires_at: None,
                policy_refs: input.policy_refs,
                evidence_refs: &[],
            })
            .expect("live peer admission");
        let import = crate::node_daemon::import_node_control_live_ticket(
            &crate::node_daemon::NodeControlLiveTicketImportInput {
                state_root: input.sender_root,
                ticket_value: &input.receiver_ticket.value,
                peer_admission_value: Some(&admission.value),
                expected_node: Some(input.receiver_node_id),
                expected_topic: Some(input.topic),
                expected_endpoint: Some(&input.receiver_ticket.live_endpoint_id),
                expected_peer: Some(input.sender_node_id),
                as_of_sequence: 1,
            },
        )
        .expect("sender imports live ticket admission");
        assert_eq!(import.decision, "pass");
        let operations = vec!["gate".to_string()];
        let grant_value = crate::node_daemon::node_control_authority_grant_value(
            &crate::node_daemon::NodeControlAuthorityGrantInput {
                peer_id: input.sender_node_id,
                node_id: input.receiver_node_id,
                operations: &operations,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                policy_refs: input.policy_refs,
                revocation_refs: &[],
                evidence_refs: &[],
            },
        )
        .expect("live authority grant value");
        let sender_grant = crate::node_daemon::import_node_control_authority_grant(input.sender_root, &grant_value)
            .expect("sender imports authority grant");
        let receiver_grant = crate::node_daemon::import_node_control_authority_grant(input.receiver_root, &grant_value)
            .expect("receiver imports authority grant");
        assert_eq!(sender_grant.grant_ref, receiver_grant.grant_ref);
        LiveDirectionEvidence {
            peer_bootstrap_refs: vec![admission.admission_ref],
            authority_refs: vec![sender_grant.grant_ref],
        }
    }

    async fn receive_one_live_ingress(
        state_root: &Path,
        topic: &str,
        receiver_node: &str,
        receiver: &mut iroh_gossip::api::GossipTopic,
    ) -> crate::node_daemon::NodeControlLiveIngressReceive {
        for _ in 0..16 {
            let event = tokio::time::timeout(Duration::from_millis(1_000), receiver.next())
                .await
                .expect("live receive event timeout")
                .expect("live receive stream ended")
                .expect("live receive event");
            if let Some(received) =
                crate::node_daemon::receive_node_control_live_ingress_event(state_root, &event, topic, receiver_node)
                    .expect("receive live ingress")
            {
                return received;
            }
        }
        panic!("live receiver did not observe ingress envelope");
    }

    fn local_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
        let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
        iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
    }

    struct TestRemoteClearanceInput<'a> {
        root: &'a std::path::Path,
        label: &'a str,
        requester_ref: &'a str,
        peer_ref: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
        remote_ref: &'a str,
        policy_ref: &'a str,
        authority_ref: &'a str,
        is_current: bool,
        revoked_refs: &'a [String],
        retained_refs: &'a [String],
    }

    struct TestAdmissionInput<'a> {
        root: &'a std::path::Path,
        kind: &'a str,
        label: &'a str,
        requester_ref: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
        remote_refs: &'a [String],
        is_reference_index_complete: bool,
        is_current: bool,
        revoked_refs: &'a [String],
    }

    struct LiveDirectionRefs {
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        evidence_refs: Vec<String>,
    }

    fn live_direction_refs(root: &std::path::Path, peer_id: &str, label: &str) -> LiveDirectionRefs {
        let policy_refs = vec![fake_ref(&format!("{label}-node-policy"))];
        let resource_refs = vec![fake_ref(&format!("{label}-node-resource"))];
        let evidence_refs = vec![fake_ref(&format!("{label}-node-evidence"))];
        let ticket = crate::node_daemon::export_node_control_live_ticket(
            &crate::node_daemon::NodeControlLiveTicketExportInput {
                state_root: root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            },
        )
        .expect("export live ticket");
        let admission =
            crate::node_daemon::admit_node_control_live_peer(&crate::node_daemon::NodeControlLivePeerAdmitInput {
                state_root: root,
                ticket_value: &ticket.value,
                peer_id,
                sequence: 1,
                expires_at: None,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })
            .expect("admit live peer");
        assert_eq!(admission.decision, "pass");
        let operations = vec!["gate".to_string()];
        let revocation_refs = Vec::new();
        let authority_value = crate::node_daemon::node_control_authority_grant_value(
            &crate::node_daemon::NodeControlAuthorityGrantInput {
                peer_id,
                node_id: &ticket.node_id,
                operations: &operations,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                policy_refs: &policy_refs,
                revocation_refs: &revocation_refs,
                evidence_refs: &evidence_refs,
            },
        )
        .expect("authority grant value");
        let authority = crate::node_daemon::import_node_control_authority_grant(root, &authority_value)
            .expect("import authority grant");
        LiveDirectionRefs {
            peer_bootstrap_refs: vec![admission.admission_ref],
            authority_refs: vec![authority.grant_ref],
            policy_refs,
            resource_refs,
            evidence_refs,
        }
    }

    fn sensitive_explain_value(object_ref: &str, plan_ref: &str) -> IOValue {
        let plan_refs = vec![plan_ref.to_string()];
        retention_candidate_explain_value(&RetentionCandidateExplainValueInput {
            object_ref,
            object_kind: Some("encrypted-ref"),
            retention_class: Some(CLASS_PRIVATE_SECRET_REF),
            action: Some(ACTION_DELETE),
            subsystem: Some("ledger-gc"),
            pin_refs: &[],
            admission_refs: &[],
            remote_clearance_refs: &[],
            remote_clearance_import_refs: &[],
            gc_plan_refs: &plan_refs,
            gc_apply_refs: &[],
            gc_execution_refs: &[],
            gc_audit_refs: &[],
            retention_receipt_refs: &[],
            tombstone_refs: &[],
            diagnostics: &[],
        })
        .expect("sensitive explain value")
    }

    fn fake_live_refs(label: &str) -> Vec<String> {
        (0..8).map(|index| fake_ref(&format!("{label}-live-ref-{index}"))).collect()
    }

    fn fake_live_transport_receipt(operation: &str, node_id: &str, envelope_label: &str, ingress_ref: &str) -> IOValue {
        record("node-control-live-transport-receipt-v1", vec![
            string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
            record("operation", vec![string(operation)]),
            record("decision", vec![string("pass")]),
            record("transport", vec![string("iroh-gossip")]),
            record("topic", vec![string(crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]),
            record("node", vec![string(node_id)]),
            record("delivered-from", vec![optional_ref_value(Some(&fake_ref(&format!("{envelope_label}-peer"))))]),
            record("envelope", vec![string(fake_ref(envelope_label))]),
            record("ingress-receipt", vec![optional_ref_value(Some(ingress_ref))]),
            record("diagnostics", vec![sequence(Vec::new())]),
            checks_value(&[
                ("canonical-envelope-ref", "pass"),
                ("live-iroh-gossip", "pass"),
                ("peer-bootstrap-before-enqueue", "pass"),
                ("transport-is-not-authority", "pass"),
                ("durable-inbox-boundary", "pass"),
            ]),
        ])
    }

    fn fake_live_send_receipt(
        from_peer: &str,
        to_node: &str,
        envelope_label: &str,
        transport_ref: &str,
        ticket_label: &str,
    ) -> IOValue {
        record("node-control-live-send-receipt-v1", vec![
            string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
            record("decision", vec![string("pass")]),
            record("transport", vec![string("iroh-gossip")]),
            record("topic", vec![string(crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]),
            record("from-peer", vec![string(from_peer)]),
            record("to-node", vec![string(to_node)]),
            record("receiver-ticket", vec![string(fake_ref(ticket_label))]),
            record("receiver-endpoint", vec![string(to_node)]),
            record("receiver-addresses", vec![sequence(vec![string(fake_ref(&format!("{ticket_label}-address")))])]),
            record("envelope", vec![string(fake_ref(envelope_label))]),
            record("transport-receipt", vec![optional_ref_value(Some(transport_ref))]),
            record("diagnostics", vec![sequence(Vec::new())]),
            checks_value(&[
                ("receiver-ticket-bound", "pass"),
                ("receiver-address-bound", "pass"),
                ("receiver-address-supported", "pass"),
                ("receiver-ticket-expected", "pass"),
                ("operation-id-bound", "pass"),
                ("sender-state-root-evidence", "pass"),
                ("join-or-publish-succeeded", "pass"),
                ("canonical-envelope-ref", "pass"),
                ("live-iroh-gossip", "pass"),
                ("transport-is-not-authority", "pass"),
                ("durable-inbox-boundary", "pass"),
            ]),
        ])
    }

    fn store_test_remote_clearance(input: TestRemoteClearanceInput<'_>) -> String {
        store_retention_remote_gc_clearance(input.root, &RetentionRemoteGcClearanceInput {
            decision: "pass",
            requester_ref: input.requester_ref,
            peer_ref: input.peer_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            remote_ref: input.remote_ref,
            policy_ref: input.policy_ref,
            authority_ref: input.authority_ref,
            evidence_refs: &[fake_ref(input.label)],
            retained_refs: input.retained_refs,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            diagnostics: &[],
        })
        .expect("store test remote clearance")
        .clearance_ref
    }

    fn scoped_ref(
        root: &Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        is_current: bool,
        revoked_refs: &[String],
    ) -> String {
        store_test_admission(TestAdmissionInput {
            root,
            kind,
            label,
            requester_ref,
            object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current,
            revoked_refs,
        })
    }

    fn store_test_admission(input: TestAdmissionInput<'_>) -> String {
        store_retention_evidence_admission(input.root, &RetentionEvidenceAdmissionInput {
            kind: input.kind,
            decision: "pass",
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            bound_refs: &[fake_ref(input.label)],
            retained_refs: &[],
            remote_refs: input.remote_refs,
            is_reference_index_complete: input.is_reference_index_complete,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            diagnostics: &[],
        })
        .expect("store test admission")
        .admission_ref
    }

    struct TestPlanFixture {
        requester_ref: String,
        object_ref: String,
        evidence: DestructiveRetentionEvidence,
    }

    struct Flow {
        plan: RetentionGcPlan,
        apply: RetentionGcApply,
        execution: RetentionGcExecutionGate,
        audit: RetentionGcAudit,
    }

    fn passing_flow(root: &Path, fixture: &TestPlanFixture, subsystem: &str) -> Flow {
        let plan = store_retention_gc_plan(RetentionGcPlanInput {
            root,
            subsystem,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        let apply = apply_retention_gc_plan(RetentionGcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        let execution = store_retention_gc_execution_gate(RetentionGcExecutionGateInput {
            root,
            subsystem,
            action: ACTION_DELETE,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store execution");
        let audit = audit_retention_gc_execution(RetentionGcAuditInput {
            root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit execution");
        Flow {
            plan,
            apply,
            execution,
            audit,
        }
    }

    struct SeedInput<'a> {
        root: &'a Path,
        kind: &'a str,
        label: String,
        requester_ref: &'a str,
        object_ref: &'a str,
        remote_refs: &'a [String],
    }

    fn seed_ref(input: SeedInput<'_>) -> String {
        store_test_admission(TestAdmissionInput {
            root: input.root,
            kind: input.kind,
            label: &input.label,
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: input.remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        })
    }

    fn seed_set(
        root: &Path,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        remote_refs: &[String],
    ) -> [String; 5] {
        let empty_refs: &[String] = &[];
        [
            (ADMISSION_KIND_POLICY, "policy", empty_refs),
            (ADMISSION_KIND_AUTHORITY, "authority", empty_refs),
            (ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", empty_refs),
            (ADMISSION_KIND_REFERENCE_INDEX, "index", empty_refs),
            (ADMISSION_KIND_REMOTE_GC, "remote-gc", remote_refs),
        ]
        .map(|(kind, suffix, remote_refs)| {
            seed_ref(SeedInput {
                root,
                kind,
                label: format!("{label}-{suffix}"),
                requester_ref,
                object_ref,
                remote_refs,
            })
        })
    }

    struct DenyCase {
        requester: String,
        object: String,
        remotes: Vec<String>,
        peers: Vec<String>,
        wrong_peer: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        gc: String,
    }

    impl DenyCase {
        fn base(&self) -> DestructiveRetentionEvidence {
            DestructiveRetentionEvidence {
                requester_ref: Some(self.requester.clone()),
                policy_refs: vec![self.policy.clone()],
                authority_refs: vec![self.authority.clone()],
                evidence_refs: vec![self.support.clone()],
                retained_refs: Vec::new(),
                remote_peer_refs: self.peers.clone(),
                remote_refs: self.remotes.clone(),
                reference_index_refs: vec![self.index.clone()],
                remote_gc_refs: vec![self.gc.clone()],
                remote_clearance_refs: Vec::new(),
                is_reference_index_complete: true,
            }
        }

        fn scoped(&self, stored_ref: String) -> DestructiveRetentionEvidence {
            let mut evidence = self.base();
            evidence.remote_refs = vec![self.remotes[0].clone()];
            evidence.remote_peer_refs = vec![self.peers[0].clone()];
            evidence.remote_clearance_refs = vec![stored_ref];
            evidence
        }
    }

    struct DenyRefs {
        partial: String,
        wrong_peer: String,
        stale: String,
        retained: String,
    }

    struct ClearInput<'a> {
        root: &'a Path,
        case: &'a DenyCase,
        label: &'a str,
        peer: &'a str,
        remote: &'a str,
        is_current: bool,
        revoked_refs: &'a [String],
        retained_refs: &'a [String],
    }

    fn deny_case(root: &Path) -> DenyCase {
        let requester = fake_ref("requester-deny");
        let object = fake_ref("object-deny");
        let remotes = vec![fake_ref("remote-a"), fake_ref("remote-b")];
        let peers = vec![fake_ref("peer-a"), fake_ref("peer-b")];
        let empty_refs: &[String] = &[];
        let [policy, authority, support, index, gc] = [
            (ADMISSION_KIND_POLICY, "policy-deny", empty_refs),
            (ADMISSION_KIND_AUTHORITY, "authority-deny", empty_refs),
            (ADMISSION_KIND_SUPPORTING_EVIDENCE, "support-deny", empty_refs),
            (ADMISSION_KIND_REFERENCE_INDEX, "index-deny", empty_refs),
            (ADMISSION_KIND_REMOTE_GC, "remote-gc-deny", remotes.as_slice()),
        ]
        .map(|(kind, label, remote_refs)| {
            seed_ref(SeedInput {
                root,
                kind,
                label: label.to_string(),
                requester_ref: &requester,
                object_ref: &object,
                remote_refs,
            })
        });
        DenyCase {
            requester,
            object,
            remotes,
            peers,
            wrong_peer: fake_ref("peer-wrong"),
            policy,
            authority,
            support,
            index,
            gc,
        }
    }

    fn clear_ref(input: ClearInput<'_>) -> String {
        store_test_remote_clearance(TestRemoteClearanceInput {
            root: input.root,
            label: input.label,
            requester_ref: &input.case.requester,
            peer_ref: input.peer,
            object_ref: &input.case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: input.remote,
            policy_ref: &input.case.policy,
            authority_ref: &input.case.authority,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            retained_refs: input.retained_refs,
        })
    }

    fn denial_refs(root: &Path, case: &DenyCase) -> DenyRefs {
        let empty_refs: &[String] = &[];
        let revoked_refs = vec![fake_ref("remote-revocation")];
        let retained_refs = vec![fake_ref("remote-retained-object")];
        DenyRefs {
            partial: clear_ref(ClearInput {
                root,
                case,
                label: "clearance-a",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: empty_refs,
            }),
            wrong_peer: clear_ref(ClearInput {
                root,
                case,
                label: "wrong-peer-clearance",
                peer: &case.wrong_peer,
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: empty_refs,
            }),
            stale: clear_ref(ClearInput {
                root,
                case,
                label: "stale-clearance",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: false,
                revoked_refs: &revoked_refs,
                retained_refs: empty_refs,
            }),
            retained: clear_ref(ClearInput {
                root,
                case,
                label: "retained-clearance",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: &retained_refs,
            }),
        }
    }

    fn assert_denial(
        root: &Path,
        case: &DenyCase,
        evidence: &DestructiveRetentionEvidence,
        reason: &str,
        expected: &[&str],
    ) {
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root,
            evidence,
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect(reason);
        assert_eq!(admission.decision, "deny");
        for needle in expected {
            assert!(
                admission.diagnostics.iter().any(|diagnostic| diagnostic.contains(needle)),
                "missing diagnostic {needle} in {:?}",
                admission.diagnostics
            );
        }
    }

    struct LiveCase {
        requester: String,
        peer: String,
        object: String,
        remote: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        gc: String,
    }

    fn live_case(root: &Path, label: &str) -> LiveCase {
        let requester = fake_ref(&format!("{label}-requester"));
        let peer = fake_ref(&format!("{label}-peer"));
        let object = fake_ref(&format!("{label}-object"));
        let remote = fake_ref(&format!("{label}-remote"));
        let seeds = seed_set(root, label, &requester, &object, std::slice::from_ref(&remote));
        let [policy, authority, support, index, gc] = seeds;
        LiveCase {
            requester,
            peer,
            object,
            remote,
            policy,
            authority,
            support,
            index,
            gc,
        }
    }

    fn assert_case_pass(root: &Path, case: &LiveCase, clearance: String) {
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root,
            evidence: &DestructiveRetentionEvidence {
                requester_ref: Some(case.requester.clone()),
                policy_refs: vec![case.policy.clone()],
                authority_refs: vec![case.authority.clone()],
                evidence_refs: vec![case.support.clone()],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![case.peer.clone()],
                remote_refs: vec![case.remote.clone()],
                reference_index_refs: vec![case.index.clone()],
                remote_gc_refs: vec![case.gc.clone()],
                remote_clearance_refs: vec![clearance],
                is_reference_index_complete: true,
            },
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("admit live clearance");
        assert_eq!(admission.decision, "pass");
        assert!(admission.has_remote_gc_clearance);
    }

    struct Pair {
        request_value: IOValue,
        response_value: IOValue,
        request_ref: String,
        response_ref: String,
    }

    struct Traffic {
        request_ingress: String,
        response_ingress: String,
        request_receive: IOValue,
        response_receive: IOValue,
        request_send: IOValue,
        response_send: IOValue,
    }

    struct Material {
        case: LiveCase,
        request_value: IOValue,
        response_value: IOValue,
        request_control: IOValue,
        response_control: IOValue,
        traffic: Traffic,
    }

    fn request_pair(root: &Path, case: &LiveCase) -> Pair {
        pair_with_label(root, case, "multihost-peer-evidence")
    }

    fn pair_with_label(root: &Path, case: &LiveCase, label: &str) -> Pair {
        let request = store_retention_remote_gc_clearance_request(root, &RetentionRemoteGcClearanceRequestInput {
            requester_ref: &case.requester,
            peer_ref: &case.peer,
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &case.remote,
            policy_ref: &case.policy,
            authority_ref: &case.authority,
            evidence_refs: std::slice::from_ref(&case.support),
        })
        .expect("request");
        let response = store_retention_remote_gc_clearance_response(RetentionRemoteGcClearanceResponseInput {
            root,
            request_value: &request.value,
            evidence_refs: &[fake_ref(label)],
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("response");
        Pair {
            request_value: request.value,
            response_value: response.value,
            request_ref: request.request_ref,
            response_ref: response.response_ref,
        }
    }

    fn control_values(pair: &Pair) -> (IOValue, IOValue) {
        let request_control = remote_clearance_live_control_request_value(&LiveControlRequestInput {
            target_ref: &pair.request_ref,
            payload_ref: None,
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: std::slice::from_ref(&pair.request_ref),
        })
        .expect("request control")
        .1;
        let response_control = remote_clearance_live_control_request_value(&LiveControlRequestInput {
            target_ref: &pair.response_ref,
            payload_ref: Some(&pair.request_ref),
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[pair.request_ref.clone(), pair.response_ref.clone()],
        })
        .expect("response control")
        .1;
        (request_control, response_control)
    }

    fn traffic_values() -> Traffic {
        let request_ingress = fake_ref("multihost-request-ingress");
        let response_ingress = fake_ref("multihost-response-ingress");
        let request_publish = fake_ref("multihost-request-publish");
        let response_publish = fake_ref("multihost-response-publish");
        let request_receive = fake_live_transport_receipt("receive", "peer-node", "request-envelope", &request_ingress);
        let response_receive =
            fake_live_transport_receipt("receive", "requester-node", "response-envelope", &response_ingress);
        let request_send = fake_live_send_receipt(
            "requester-node",
            "peer-node",
            "request-envelope",
            &request_publish,
            "request-ticket",
        );
        let response_send = fake_live_send_receipt(
            "peer-node",
            "requester-node",
            "response-envelope",
            &response_publish,
            "response-ticket",
        );
        Traffic {
            request_ingress,
            response_ingress,
            request_receive,
            response_receive,
            request_send,
            response_send,
        }
    }

    struct TicketPair {
        requester_root: PathBuf,
        peer_root: PathBuf,
        peer_ticket: crate::node_daemon::NodeControlLiveTicket,
        requester_ticket: crate::node_daemon::NodeControlLiveTicket,
    }

    struct NoEndpointCase {
        root: PathBuf,
        nodes: TicketPair,
        requester: String,
        peer: String,
        object: String,
        remote: String,
        policy: String,
        authority: String,
        evidence: String,
    }

    fn ticket_pair() -> TicketPair {
        let requester_root = temp_dir("retention-remote-clearance-live-multihost-requester");
        let peer_root = temp_dir("retention-remote-clearance-live-multihost-peer");
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &requester_root,
            node_id: "requester-node",
        })
        .expect("init requester node");
        crate::node_daemon::init_local_node(&crate::node_daemon::NodeDaemonInitInput {
            state_root: &peer_root,
            node_id: "peer-node",
        })
        .expect("init peer node");
        let policy = vec![fake_ref("multihost-ticket-policy")];
        let evidence = vec![fake_ref("multihost-ticket-evidence")];
        let peer_ticket = crate::node_daemon::export_node_control_live_ticket(
            &crate::node_daemon::NodeControlLiveTicketExportInput {
                state_root: &peer_root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy,
                evidence_refs: &evidence,
            },
        )
        .expect("peer ticket");
        let requester_ticket = crate::node_daemon::export_node_control_live_ticket(
            &crate::node_daemon::NodeControlLiveTicketExportInput {
                state_root: &requester_root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy,
                evidence_refs: &evidence,
            },
        )
        .expect("requester ticket");
        TicketPair {
            requester_root,
            peer_root,
            peer_ticket,
            requester_ticket,
        }
    }

    fn no_endpoint_case() -> NoEndpointCase {
        NoEndpointCase {
            root: temp_dir("retention-remote-clearance-live-multihost-send"),
            nodes: ticket_pair(),
            requester: fake_ref("multihost-send-requester"),
            peer: fake_ref("multihost-send-peer"),
            object: fake_ref("multihost-send-object"),
            remote: fake_ref("multihost-send-remote"),
            policy: fake_ref("multihost-send-policy"),
            authority: fake_ref("multihost-send-authority"),
            evidence: fake_ref("multihost-send-evidence"),
        }
    }

    fn case_request(
        runtime: &tokio::runtime::Runtime,
        case: &NoEndpointCase,
    ) -> RetentionRemoteGcClearanceLiveRequestSend {
        runtime
            .block_on(send_retention_remote_gc_clearance_live_request(RetentionRemoteGcClearanceLiveRequestSendInput {
                root: &case.root,
                requester_node_root: Some(&case.nodes.requester_root),
                peer_ticket_value: &case.nodes.peer_ticket.value,
                requester_node_id: "requester-node",
                peer_node_id: "peer-node",
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                sequence: 1,
                max_attempts: 1,
                join_timeout_ms: 1,
                requester_ref: &case.requester,
                peer_ref: &case.peer,
                object_ref: &case.object,
                object_kind: "chunk",
                retention_class: CLASS_DURABLE_VALUE,
                action: ACTION_DELETE,
                remote_ref: &case.remote,
                policy_ref: &case.policy,
                authority_ref: &case.authority,
                retention_evidence_refs: std::slice::from_ref(&case.evidence),
                peer_bootstrap_refs: &[],
                authority_refs: &[],
                policy_refs: &[],
                resource_refs: &[],
                transport_evidence_refs: &[],
            }))
            .expect("request send")
    }

    fn case_response(
        runtime: &tokio::runtime::Runtime,
        case: &NoEndpointCase,
        request: &RetentionRemoteGcClearanceLiveRequestSend,
    ) -> RetentionRemoteGcClearanceLiveResponseSend {
        runtime
            .block_on(send_retention_remote_gc_clearance_live_response(
                RetentionRemoteGcClearanceLiveResponseSendInput {
                    root: &case.root,
                    peer_node_root: Some(&case.nodes.peer_root),
                    requester_ticket_value: &case.nodes.requester_ticket.value,
                    request_value: &request.request.value,
                    peer_node_id: "peer-node",
                    requester_node_id: "requester-node",
                    topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                    sequence: 1,
                    max_attempts: 1,
                    join_timeout_ms: 1,
                    response_evidence_refs: std::slice::from_ref(&case.evidence),
                    retained_refs: &[],
                    is_current: true,
                    revoked_refs: &[],
                    response_diagnostics: &[],
                    peer_bootstrap_refs: &[],
                    authority_refs: &[],
                    policy_refs: &[],
                    resource_refs: &[],
                    transport_evidence_refs: &[],
                },
            ))
            .expect("response send")
    }

    fn assert_send_denial(value: &IOValue, expected: Option<&str>) {
        let receipt = crate::node_daemon::parse_node_control_live_send_receipt(value).expect("send receipt");
        assert_eq!(receipt.decision, "deny");
        if let Some(needle) = expected {
            assert!(receipt.diagnostics.iter().any(|value| value.contains(needle)));
        }
    }

    fn fixture_material(root: &Path) -> Material {
        let case = live_case(root, "multihost");
        let pair = request_pair(root, &case);
        let (request_control, response_control) = control_values(&pair);
        Material {
            case,
            request_value: pair.request_value,
            response_value: pair.response_value,
            request_control,
            response_control,
            traffic: traffic_values(),
        }
    }

    fn import_with(
        root: &Path,
        material: &Material,
        request_receive: &IOValue,
    ) -> RetentionRemoteGcClearanceLiveImportWorkflow {
        import_retention_remote_gc_clearance_live_workflow(RetentionRemoteGcClearanceLiveImportWorkflowInput {
            root,
            request_value: &material.request_value,
            response_value: &material.response_value,
            request_control_value: &material.request_control,
            request_send_receipt_value: &material.traffic.request_send,
            request_receive_receipt_value: request_receive,
            request_ingress_ref: &material.traffic.request_ingress,
            response_control_value: &material.response_control,
            response_send_receipt_value: &material.traffic.response_send,
            response_receive_receipt_value: &material.traffic.response_receive,
            response_ingress_ref: &material.traffic.response_ingress,
            expected_peer_ref: Some(&material.case.peer),
            expected_remote_ref: Some(&material.case.remote),
        })
        .expect("workflow import")
    }

    fn assert_import_pass(root: &Path, material: &Material) -> String {
        let imported = import_with(root, material, &material.traffic.request_receive);
        assert_eq!(imported.import.decision, "pass");
        assert_eq!(imported.workflow.decision, "pass");
        assert_eq!(imported.workflow.request_live_refs.len(), 4);
        imported.import.clearance_ref.clone().expect("clearance stored")
    }

    fn assert_wrong_receive(root: &Path, material: &Material, wrong_request_receive: &IOValue) {
        let workflow = import_with(root, material, wrong_request_receive);
        assert_eq!(workflow.import.decision, "pass");
        assert_eq!(workflow.workflow.decision, "deny");
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value.contains("remote-clearance-live-request-receive-not-receive"))
        );
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value == "remote-clearance-live-request-receive-wrong-envelope")
        );
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value == "remote-clearance-live-request-receive-wrong-ingress")
        );
    }

    fn store_passing_plan_fixture(root: &std::path::Path, label: &str) -> TestPlanFixture {
        let requester_ref = fake_ref(&format!("{label}-requester"));
        let object_ref = fake_ref(&format!("{label}-object"));
        let peer_ref = fake_ref(&format!("{label}-peer"));
        let remote_ref = fake_ref(&format!("{label}-remote"));
        let remote_refs = std::slice::from_ref(&remote_ref);
        let [policy, authority, support, index, remote_gc] =
            seed_set(root, label, &requester_ref, &object_ref, remote_refs);
        let remote_clearance = store_test_remote_clearance(TestRemoteClearanceInput {
            root,
            label: &format!("{label}-clearance"),
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &remote_ref,
            policy_ref: &policy,
            authority_ref: &authority,
            is_current: true,
            revoked_refs: &[],
            retained_refs: &[],
        });
        TestPlanFixture {
            requester_ref: requester_ref.clone(),
            object_ref,
            evidence: DestructiveRetentionEvidence {
                requester_ref: Some(requester_ref),
                policy_refs: vec![policy],
                authority_refs: vec![authority],
                evidence_refs: vec![support],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![peer_ref],
                remote_refs: vec![remote_ref],
                reference_index_refs: vec![index],
                remote_gc_refs: vec![remote_gc],
                remote_clearance_refs: vec![remote_clearance],
                is_reference_index_complete: true,
            },
        }
    }

    fn fake_ref(label: &str) -> String {
        canonical_hash(&record("retention-test-ref", vec![string(label)])).expect("fake ref")
    }

    fn store_file_count(dir: &Path) -> usize {
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir).expect("read store dir").filter_map(std::result::Result::ok).count()
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
