use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use preserves::Value;

use crate::bounded::VecSink;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::RETENTION_CLASS_SCHEMA;
use crate::preserves_rail::RETENTION_EVIDENCE_ADMISSION_SCHEMA;
use crate::preserves_rail::RETENTION_PIN_SCHEMA;
use crate::preserves_rail::RETENTION_RECEIPT_SCHEMA;
use crate::preserves_rail::RETENTION_REFERENCE_INDEX_SCHEMA;
use crate::preserves_rail::RETENTION_TOMBSTONE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
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
const RECEIPT_DIR: &str = "receipts";
const TOMBSTONE_DIR: &str = "tombstones";
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
    pub remote_refs: Vec<String>,
    pub reference_index_refs: Vec<String>,
    pub remote_gc_refs: Vec<String>,
    pub is_reference_index_complete: bool,
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

fn admit_evidence_refs(input: AdmissionRefsInput<'_>) -> Result<AdmissionRefsResult> {
    let mut diagnostics = Vec::new();
    let mut admitted_refs = Vec::new();
    let mut remote_refs = Vec::new();
    let mut scope_mismatches = 0usize;
    for reference in input.refs {
        let admission = match read_retention_evidence_admission(input.root, reference) {
            Ok(admission) => admission,
            Err(error) => {
                push_bounded(
                    &mut diagnostics,
                    format!("{}-admission-unreadable:{}:{}", input.expected_kind, reference, error),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention admission diagnostics",
                )?;
                continue;
            }
        };
        let mut is_admitted = true;
        if admission.admission_ref != *reference {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-ref-mismatch:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if admission.kind != input.expected_kind {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-kind-mismatch:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if admission.decision != "pass" {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-not-pass:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if !admission.is_current {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-stale:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if !admission.revoked_refs.is_empty() {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-revoked:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if admission.bound_refs.is_empty() {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("{}-admission-empty-bound-refs:{}", input.expected_kind, reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if input.scope.requester_ref != Some(admission.requester_ref.as_str()) {
            is_admitted = false;
            scope_mismatches += 1;
        }
        if admission.object_ref != input.scope.object_ref || admission.object_kind != input.scope.object_kind {
            is_admitted = false;
            scope_mismatches += 1;
        }
        if admission.retention_class != input.scope.retention_class {
            is_admitted = false;
            scope_mismatches += 1;
        }
        if admission.action != input.scope.action {
            is_admitted = false;
            scope_mismatches += 1;
        }
        if input.expected_kind == ADMISSION_KIND_REFERENCE_INDEX && !admission.is_reference_index_complete {
            is_admitted = false;
            push_bounded(
                &mut diagnostics,
                format!("reference-index-admission-incomplete:{}", reference),
                MAX_RETENTION_DIAGNOSTICS,
                "retention admission diagnostics",
            )?;
        }
        if input.expected_kind == ADMISSION_KIND_REMOTE_GC {
            for required in input.required_remote_refs {
                if !admission.remote_refs.iter().any(|remote| remote == required) {
                    is_admitted = false;
                    push_bounded(
                        &mut diagnostics,
                        format!("remote-gc-admission-missing-remote:{}:{}", reference, required),
                        MAX_RETENTION_DIAGNOSTICS,
                        "retention admission diagnostics",
                    )?;
                }
            }
        }
        if is_admitted {
            push_bounded(&mut admitted_refs, admission.admission_ref, MAX_RETENTION_REFS, "retention admitted refs")?;
            for remote_ref in admission.remote_refs {
                push_bounded(&mut remote_refs, remote_ref, MAX_RETENTION_REFS, "retention admitted remote refs")?;
            }
        }
    }
    if !input.refs.is_empty() && admitted_refs.is_empty() && scope_mismatches > 0 {
        push_bounded(
            &mut diagnostics,
            format!("{}-admission-scope-mismatch", input.expected_kind),
            MAX_RETENTION_DIAGNOSTICS,
            "retention admission diagnostics",
        )?;
    }
    Ok(AdmissionRefsResult {
        diagnostics,
        admitted_refs,
        remote_refs,
    })
}

fn read_retention_evidence_admission(root: &Path, admission_ref: &str) -> Result<RetentionEvidenceAdmission> {
    require_ref(admission_ref, "retention evidence admission ref")?;
    let value = read_store_value(&admission_path(root, admission_ref)?)?;
    parse_retention_evidence_admission(&value)
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
    {
        push_bounded(&mut admitted_refs, reference, MAX_RETENTION_REFS, "retention admitted refs")?;
    }
    let has_remote_refs_clearance = input.evidence.remote_refs.is_empty()
        || input
            .evidence
            .remote_refs
            .iter()
            .all(|reference| remote_gc.remote_refs.iter().any(|remote| remote == reference));
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
    validate_refs(&input.remote_refs, "retention remote ref")?;
    validate_refs(&input.reference_index_refs, "retention reference-index ref")?;
    validate_refs(&input.remote_gc_refs, "retention remote-gc ref")
}

pub fn destructive_retention_evidence_diagnostics(
    input: &DestructiveRetentionEvidence,
    action: &str,
) -> Result<Vec<String>> {
    validate_destructive_retention_evidence(input)?;
    validate_action(action)?;
    let mut diagnostics = Vec::new();
    if input.requester_ref.is_none() {
        push_bounded(
            &mut diagnostics,
            "retention-requester-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retention-policy-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if is_destructive_action(action) && input.authority_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "delete-authority-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if is_destructive_action(action) && input.evidence_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retention-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if !input.is_reference_index_complete {
        push_bounded(
            &mut diagnostics,
            "incomplete-reference-proof".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if is_destructive_action(action) && input.is_reference_index_complete && input.reference_index_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "reference-index-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if !input.retained_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retained-dependencies-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
    if is_destructive_action(action) && !input.remote_refs.is_empty() && input.remote_gc_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "remote-gc-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention destructive evidence diagnostics",
        )?;
    }
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
        record("remote", vec![strings_sequence(&input.remote_refs)]),
        record("reference-index", vec![strings_sequence(&input.reference_index_refs)]),
        record("remote-gc", vec![strings_sequence(&input.remote_gc_refs)]),
        record("reference-index-complete", vec![string(pass_or_deny(input.is_reference_index_complete))]),
        checks_value(&[
            ("requester-bound", pass_or_deny(input.requester_ref.is_some())),
            ("policy-bound", pass_or_deny(!input.policy_refs.is_empty())),
            ("authority-bound", pass_or_deny(!input.authority_refs.is_empty())),
            ("evidence-bound", pass_or_deny(!input.evidence_refs.is_empty())),
            ("reference-index-bound", pass_or_deny(!input.reference_index_refs.is_empty())),
            ("remote-gc-bound", pass_or_deny(input.remote_refs.is_empty() || !input.remote_gc_refs.is_empty())),
        ]),
    ]))
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

pub fn retention_summary(value: &IOValue) -> Result<String> {
    if let Ok(profile) = parse_retention_class_profile(value) {
        return Ok(format!(
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
        return Ok(format!(
            "retention pin ref={} object={} kind={} class={} source={} owner={}",
            pin.pin_ref, pin.object_ref, pin.object_kind, pin.retention_class, pin.source, pin.owner_ref
        ));
    }
    if let Ok(index) = parse_reference_index(value) {
        return Ok(format!(
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
    if let Ok(admission) = parse_retention_evidence_admission(value) {
        return Ok(format!(
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
    if let Ok(receipt) = parse_retention_receipt(value) {
        return Ok(format!(
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
        return Ok(format!(
            "retention tombstone ref={} object={} class={} action={} receipt={}",
            tombstone.tombstone_ref,
            tombstone.object_ref,
            tombstone.retention_class,
            tombstone.action,
            tombstone.receipt_ref
        ));
    }
    Err(MoltenError::invalid_harness("unsupported retention artifact"))
}

pub fn run_fixture(out: &Path) -> Result<Vec<(String, IOValue)>> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    let root = out.join("state");
    ensure_store(&root)?;
    let object_ref = synthetic_ref("retention-object")?;
    let owner_ref = synthetic_ref("owner")?;
    let policy_refs = vec![synthetic_ref("policy")?];
    let evidence_refs = vec![synthetic_ref("evidence")?];
    let class = retention_class_profile_value(&RetentionClassProfileInput {
        class_name: CLASS_PRIVATE_SECRET_REF.to_string(),
        minimum_age_seconds: 0,
        maximum_age_seconds: Some(86_400),
        deletion_authority_ref: synthetic_ref("authority")?,
        policy_refs: policy_refs.clone(),
        has_secret_redaction_hook: true,
        has_remote_gc_plan: true,
        can_compact: true,
    })?;
    let pin = pin_object(&root, RetentionPinInput {
        object_ref: object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: CLASS_PRIVATE_SECRET_REF.to_string(),
        source: SOURCE_SECRET_REDACTION.to_string(),
        reason: "private repro reveal pending".to_string(),
        owner_ref: owner_ref.clone(),
        expiry_ref: None,
        policy_refs: policy_refs.clone(),
        evidence_refs: evidence_refs.clone(),
        has_authority: true,
    })?;
    let deny = evaluate_retention(RetentionEvaluationInput {
        root: &root,
        object_ref: &object_ref,
        object_kind: "encrypted-ref",
        retention_class: CLASS_PRIVATE_SECRET_REF,
        action: ACTION_DELETE,
        requester_ref: &owner_ref,
        is_reference_index_complete: true,
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        has_delete_authority: true,
        has_remote_gc_clearance: true,
    })?;
    let unpin = unpin_object(UnpinObjectInput {
        root: &root,
        pin_ref: &pin.pin.pin_ref,
        requester_ref: &owner_ref,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        has_authority: true,
    })?;
    let delete = evaluate_retention(RetentionEvaluationInput {
        root: &root,
        object_ref: &object_ref,
        object_kind: "encrypted-ref",
        retention_class: CLASS_PRIVATE_SECRET_REF,
        action: ACTION_TOMBSTONE,
        requester_ref: &owner_ref,
        is_reference_index_complete: true,
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        has_delete_authority: true,
        has_remote_gc_clearance: true,
    })?;
    let mut artifacts = Vec::new();
    push_named(&mut artifacts, "retention-class.preserves", class)?;
    push_named(&mut artifacts, "pin.preserves", pin.pin.value)?;
    push_named(&mut artifacts, "pin-receipt.preserves", pin.receipt.value)?;
    push_named(&mut artifacts, "delete-denied.preserves", deny.receipt.value)?;
    push_named(&mut artifacts, "unpin-receipt.preserves", unpin.value)?;
    push_named(&mut artifacts, "tombstone-receipt.preserves", delete.receipt.value)?;
    if let Some(tombstone) = delete.tombstone {
        push_named(&mut artifacts, "tombstone.preserves", tombstone.value)?;
    }
    for (name, value) in &artifacts {
        write_store_value(&out.join(name), value)?;
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
    let mut diagnostics = Vec::new();
    if !input.is_reference_index_complete {
        push_bounded(
            &mut diagnostics,
            "incomplete-reference-proof".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if !index.pin_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "active-pins-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if !input.retained_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retained-dependencies-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retention-policy-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if is_destructive_action(input.action) && input.evidence_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "retention-evidence-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if is_destructive_action(input.action) && !input.has_delete_authority {
        push_bounded(
            &mut diagnostics,
            "delete-authority-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if is_destructive_action(input.action) && !input.remote_refs.is_empty() && !input.has_remote_gc_clearance {
        push_bounded(
            &mut diagnostics,
            "remote-cache-refs-present".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if input.retention_class == CLASS_LEGAL_HOLD && is_destructive_action(input.action) {
        push_bounded(
            &mut diagnostics,
            "legal-hold-class-not-deletable".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if input.retention_class == CLASS_PRIVATE_SECRET_REF && input.action == ACTION_COMPACT {
        push_bounded(
            &mut diagnostics,
            "private-secret-ref-compaction-denied".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    Ok(diagnostics)
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

fn ensure_store(root: &Path) -> Result<()> {
    fs::create_dir_all(pins_dir(root)).map_err(MoltenError::from)?;
    fs::create_dir_all(admissions_dir(root)).map_err(MoltenError::from)?;
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

fn require_ref(value: &str, label: &str) -> Result<()> {
    validate_name(value, label)?;
    if value.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} must be a blake3 ref")))
    }
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
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
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
            remote_refs: Vec::new(),
            reference_index_refs: vec![fake_ref("forged-index")],
            remote_gc_refs: Vec::new(),
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
        let stale_authority = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_AUTHORITY,
            label: "stale-authority",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: false,
            revoked_refs: &[fake_ref("revocation")],
        });
        let policy = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_POLICY,
            label: "policy",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let support = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label: "support",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let index = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_REFERENCE_INDEX,
            label: "index",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
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
            policy_refs: vec![policy],
            authority_refs: vec![stale_authority],
            evidence_refs: vec![support],
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![index],
            remote_gc_refs: Vec::new(),
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
        let requester_ref = fake_ref("requester");
        let object_ref = fake_ref("object");
        let remote_refs = vec![fake_ref("remote-cache")];
        let policy = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_POLICY,
            label: "policy",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let authority = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_AUTHORITY,
            label: "authority",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let support = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label: "support",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let index = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_REFERENCE_INDEX,
            label: "index",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let remote_gc = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_REMOTE_GC,
            label: "remote-gc",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: &remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let evidence = DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref.clone()),
            policy_refs: vec![policy],
            authority_refs: vec![authority],
            evidence_refs: vec![support],
            retained_refs: Vec::new(),
            remote_refs: remote_refs.clone(),
            reference_index_refs: vec![index],
            remote_gc_refs: vec![remote_gc],
            is_reference_index_complete: true,
        };
        let admission = admit_destructive_retention_evidence(DestructiveRetentionAdmissionInput {
            root: &root,
            evidence: &evidence,
            object_ref: &object_ref,
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
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &remote_refs,
            policy_refs: &evidence.policy_refs,
            evidence_refs: &evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })
        .expect("evaluate remote clearance");
        assert_eq!(evaluation.receipt.decision, "pass");
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

    fn fake_ref(label: &str) -> String {
        canonical_hash(&record("retention-test-ref", vec![string(label)])).expect("fake ref")
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
