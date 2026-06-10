use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::EVIDENCE_LEDGER_EXPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA;
use crate::preserves_rail::EVIDENCE_LEDGER_IMPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;
use crate::retention;

const MAX_LEDGER_SCAN_ENTRIES: usize = 100_000;
const _: () = assert!(MAX_LEDGER_SCAN_ENTRIES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    pub artifact_ref: String,
    pub artifact_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerImport {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerExport {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerGc {
    pub dry_run: bool,
    pub decision: String,
    pub removed_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerGcInput<'a> {
    pub dry_run: bool,
    pub retention_evidence: &'a retention::DestructiveRetentionEvidence,
    pub apply_refs: &'a [String],
}

pub fn import_artifact(root: &Path, artifact: &IOValue) -> Result<LedgerImport> {
    ensure_dirs(root)?;
    let artifact_ref = canonical_hash(artifact)?;
    let artifact_kind = artifact_kind(artifact).to_string();
    let bytes = canonical_bytes(artifact)?;
    let path = content_path(root, &artifact_ref)?;
    if path.exists() {
        let existing = fs::read(&path).map_err(MoltenError::from)?;
        let existing_value = parse_canonical_bytes(&existing)?;
        let existing_ref = canonical_hash(&existing_value)?;
        if existing_ref != artifact_ref {
            return Err(MoltenError::invalid_harness(format!(
                "ledger content path for {artifact_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        fs::write(&path, bytes).map_err(MoltenError::from)?;
    }
    let receipt_value = ledger_import_receipt_value(&artifact_ref, &artifact_kind);
    Ok(LedgerImport {
        artifact_ref,
        artifact_kind,
        receipt_value,
    })
}

pub fn export_artifact(root: &Path, artifact_ref: &str, out: &Path) -> Result<LedgerExport> {
    let artifact = read_artifact(root, artifact_ref)?;
    let artifact_kind = artifact_kind(&artifact).to_string();
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(out, crate::preserves_rail::to_text(&artifact)?).map_err(MoltenError::from)?;
    let receipt_value = record("ledger-export-receipt-v1", vec![
        string(EVIDENCE_LEDGER_EXPORT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("artifact-kind", vec![string(&artifact_kind)]),
        record("artifact", vec![string(artifact_ref)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-ref-found"), string("pass")]),
            record("check", vec![string("canonical-export"), string("pass")]),
        ])]),
    ]);
    Ok(LedgerExport {
        artifact_ref: artifact_ref.to_string(),
        artifact_kind,
        receipt_value,
    })
}

pub fn read_artifact(root: &Path, artifact_ref: &str) -> Result<IOValue> {
    let path = content_path(root, artifact_ref)?;
    let bytes = fs::read(&path).map_err(MoltenError::from)?;
    let value = parse_canonical_bytes(&bytes)?;
    let actual_ref = canonical_hash(&value)?;
    if actual_ref != artifact_ref {
        return Err(MoltenError::invalid_harness(format!(
            "ledger content hash mismatch: got {actual_ref}, expected {artifact_ref}"
        )));
    }
    Ok(value)
}

pub fn list_artifacts(root: &Path) -> Result<Vec<LedgerEntry>> {
    let content = root.join("content");
    if !content.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(content).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if !entry.file_type().map_err(MoltenError::from)?.is_file() {
            continue;
        }
        let Some(artifact_ref) = ref_from_filename(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let value = read_artifact(root, &artifact_ref)?;
        push_bounded(
            &mut entries,
            LedgerEntry {
                artifact_ref,
                artifact_kind: artifact_kind(&value).to_string(),
            },
            MAX_LEDGER_SCAN_ENTRIES,
            "ledger artifact entries",
        )?;
    }
    entries.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(entries)
}

pub fn pin_artifact(root: &Path, artifact_ref: &str) -> Result<()> {
    ensure_dirs(root)?;
    read_artifact(root, artifact_ref)?;
    fs::write(pin_path(root, artifact_ref)?, artifact_ref).map_err(MoltenError::from)
}

pub fn gc(root: &Path, input: LedgerGcInput<'_>) -> Result<LedgerGc> {
    ensure_dirs(root)?;
    let pins = pinned_refs(root)?;
    let mut candidates = Vec::new();
    for entry in list_artifacts(root)? {
        if pins.iter().any(|pin| pin == &entry.artifact_ref) {
            continue;
        }
        push_bounded(&mut candidates, entry, MAX_LEDGER_SCAN_ENTRIES, "ledger gc candidates")?;
    }
    let action = if input.dry_run {
        retention::ACTION_ELIGIBILITY
    } else {
        retention::ACTION_DELETE
    };
    let requester_ref =
        retention::destructive_retention_requester_ref(input.retention_evidence, "ledger-gc-missing-requester")?;
    let evidence_summary = retention::destructive_retention_evidence_value(input.retention_evidence)?;
    let mut admission_diagnostics = Vec::new();
    let mut execution_diagnostics = Vec::new();
    let mut admission_refs = Vec::new();
    let mut retention_receipt_refs = Vec::new();
    let mut execution_gate_refs = Vec::new();
    let mut retention_denials = Vec::new();
    for entry in &candidates {
        let retention_class = ledger_retention_class(&entry.artifact_kind);
        let admission =
            retention::admit_destructive_retention_evidence(retention::DestructiveRetentionAdmissionInput {
                root,
                evidence: input.retention_evidence,
                object_ref: &entry.artifact_ref,
                object_kind: &entry.artifact_kind,
                retention_class,
                action,
            })?;
        for diagnostic in &admission.diagnostics {
            push_bounded(
                &mut admission_diagnostics,
                diagnostic.clone(),
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger retention admission diagnostics",
            )?;
        }
        for reference in &admission.admitted_refs {
            push_bounded(
                &mut admission_refs,
                reference.clone(),
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger retention admission refs",
            )?;
        }
        let evaluation = retention::evaluate_retention(retention::RetentionEvaluationInput {
            root,
            object_ref: &entry.artifact_ref,
            object_kind: &entry.artifact_kind,
            retention_class,
            action,
            requester_ref: &requester_ref,
            is_reference_index_complete: input.retention_evidence.is_reference_index_complete,
            retained_refs: &input.retention_evidence.retained_refs,
            remote_refs: &input.retention_evidence.remote_refs,
            policy_refs: &input.retention_evidence.policy_refs,
            evidence_refs: &input.retention_evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })?;
        push_bounded(
            &mut retention_receipt_refs,
            evaluation.receipt.receipt_ref.clone(),
            MAX_LEDGER_SCAN_ENTRIES,
            "ledger retention receipt refs",
        )?;
        let mut is_execution_denied = false;
        if !input.dry_run {
            let apply_ref = matching_apply_ref(ApplyRefMatchInput {
                root,
                apply_refs: input.apply_refs,
                subsystem: "ledger-gc",
                action,
                object_ref: &entry.artifact_ref,
                object_kind: &entry.artifact_kind,
                retention_class,
            });
            let execution_gate =
                retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
                    root,
                    subsystem: "ledger-gc",
                    action,
                    object_ref: &entry.artifact_ref,
                    object_kind: &entry.artifact_kind,
                    retention_class,
                    apply_ref,
                })?;
            push_bounded(
                &mut execution_gate_refs,
                execution_gate.execution_ref.clone(),
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger retention execution gate refs",
            )?;
            if execution_gate.decision != "pass" {
                is_execution_denied = true;
                for diagnostic in &execution_gate.diagnostics {
                    push_bounded(
                        &mut execution_diagnostics,
                        diagnostic.clone(),
                        MAX_LEDGER_SCAN_ENTRIES,
                        "ledger retention execution diagnostics",
                    )?;
                }
            }
        }
        if admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_execution_denied {
            push_bounded(
                &mut retention_denials,
                entry.artifact_ref.clone(),
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger retention denials",
            )?;
        }
    }
    let decision = if retention_denials.is_empty() { "pass" } else { "deny" };
    let mut removed_refs = Vec::new();
    if decision == "pass" {
        for entry in &candidates {
            push_bounded(
                &mut removed_refs,
                entry.artifact_ref.clone(),
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger removed refs",
            )?;
            if !input.dry_run {
                fs::remove_file(content_path(root, &entry.artifact_ref)?).map_err(MoltenError::from)?;
            }
        }
    }
    let receipt_value = record("ledger-gc-receipt-v1", vec![
        string(EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("mode", vec![string(if input.dry_run { "dry-run" } else { "apply" })]),
        record("removed", vec![sequence(removed_refs.iter().map(string).collect())]),
        record("retention", vec![sequence(retention_receipt_refs.iter().map(string).collect())]),
        record("retention-execution", vec![sequence(execution_gate_refs.iter().map(string).collect())]),
        record("denied", vec![sequence(retention_denials.iter().map(string).collect())]),
        record("retention-evidence", vec![evidence_summary]),
        record("retention-admission", vec![sequence(admission_refs.iter().map(string).collect())]),
        record("retention-diagnostics", vec![sequence(admission_diagnostics.iter().map(string).collect())]),
        record("retention-execution-diagnostics", vec![sequence(
            execution_diagnostics.iter().map(string).collect(),
        )]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("pin-preservation"), string("pass")]),
            record("check", vec![string("derived-index-scan"), string("pass")]),
            record("check", vec![string("retention-receipt-bound"), string("pass")]),
            record("check", vec![
                string("retention-execution-gate"),
                string(pass_or_fail(input.dry_run || execution_diagnostics.is_empty())),
            ]),
            record("check", vec![
                string("retention-authority-evidence"),
                string(pass_or_fail(admission_diagnostics.is_empty())),
            ]),
            record("check", vec![
                string("deny-before-removal"),
                string(if decision == "pass" { "pass" } else { "fail" }),
            ]),
        ])]),
    ]);
    Ok(LedgerGc {
        dry_run: input.dry_run,
        decision: decision.to_string(),
        removed_refs,
        retention_receipt_refs,
        execution_gate_refs,
        receipt_value,
    })
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
        let Ok(apply) = retention::read_retention_gc_apply(input.root, apply_ref) else {
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

fn ledger_retention_class(artifact_kind: &str) -> &'static str {
    if artifact_kind.contains("secret") || artifact_kind.contains("encrypted") || artifact_kind.contains("redaction") {
        retention::CLASS_PRIVATE_SECRET_REF
    } else if artifact_kind.contains("cache") {
        retention::CLASS_EPHEMERAL_CACHE
    } else if artifact_kind.contains("artifact") || artifact_kind.contains("manifest") {
        retention::CLASS_PUBLIC_ARTIFACT
    } else {
        retention::CLASS_AUDIT_RECEIPT
    }
}

pub fn ledger_import_receipt_value(artifact_ref: &str, artifact_kind: &str) -> IOValue {
    record("ledger-import-receipt-v1", vec![
        string(EVIDENCE_LEDGER_IMPORT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("artifact-kind", vec![string(artifact_kind)]),
        record("artifact", vec![string(artifact_ref)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-content-hash"), string("pass")]),
            record("check", vec![string("immutable-content"), string("pass")]),
            record("check", vec![string("derived-index-ready"), string("pass")]),
        ])]),
    ])
}

const ARTIFACT_KIND_RECORDS: &[(&str, &str)] = &[
    ("octet-command-artifact-v1", "octet-command-artifact"),
    ("octet-status-artifact-v1", "octet-status-artifact"),
    ("octet-summary-artifact-v1", "octet-summary-artifact"),
    ("octet-object-corpus-artifact-v1", "octet-object-corpus-artifact"),
    ("octet-artifact-ledger-receipt-v1", "octet-artifact-ledger-receipt"),
    ("octet-gate-policy-v1", "octet-gate-policy"),
    ("octet-gate-receipt-v1", "octet-gate-receipt"),
    ("octet-structured-findings-v1", "octet-structured-findings"),
    ("octet-fingerprint-evidence-v1", "octet-fingerprint-evidence"),
    ("octet-warning-baseline-v1", "octet-warning-baseline"),
    ("octet-baseline-receipt-v1", "octet-baseline-receipt"),
    ("octet-review-manifest-v1", "octet-review-manifest"),
    ("octet-source-gate-requirement-v1", "octet-source-gate-requirement"),
    ("octet-source-gate-validation-v1", "octet-source-gate-validation"),
    ("octet-remediation-plan-v1", "octet-remediation-plan"),
    ("catalog-summary-v1", "catalog-summary"),
    ("catalog-view-v1", "catalog-view"),
    ("catalog-query-v1", "catalog-query"),
    ("catalog-result-v1", "catalog-result"),
    ("catalog-receipt-v1", "catalog-receipt"),
    ("short-id-resolution-v1", "catalog-short-id-resolution"),
    ("catalog-mcp-request-v1", "catalog-mcp-request"),
    ("catalog-mcp-response-v1", "catalog-mcp-response"),
    ("catalog-mcp-receipt-v1", "catalog-mcp-receipt"),
    ("job-dag-v1", "job-dag"),
    ("job-node-v1", "job-dag-node"),
    ("job-edge-v1", "job-dag-edge"),
    ("job-output-request-v1", "job-output-request"),
    ("job-dag-receipt-v1", "job-dag-receipt"),
    ("job-stage-operation-v1", "job-stage-operation"),
    ("job-plan-v1", "job-plan"),
    ("job-profile-v1", "job-profile"),
    ("job-fusion-plan-v1", "job-fusion-plan"),
    ("job-plan-receipt-v1", "job-plan-receipt"),
    ("job-profile-receipt-v1", "job-profile-receipt"),
    ("job-fusion-receipt-v1", "job-fusion-receipt"),
    ("job-sync-request-v1", "job-sync-request"),
    ("job-sync-plan-v1", "job-sync-plan"),
    ("job-sync-receipt-v1", "job-sync-receipt"),
    ("job-admission-request-v1", "job-admission-request"),
    ("job-admission-plan-v1", "job-admission-plan"),
    ("job-admission-receipt-v1", "job-admission-receipt"),
    ("job-execution-request-v1", "job-execution-request"),
    ("job-execution-receipt-v1", "job-execution-receipt"),
    ("job-ref-submission-v1", "job-ref-submission"),
    ("job-ref-status-v1", "job-ref-status"),
    ("job-ref-receipt-v1", "job-ref-receipt"),
    ("job-worker-request-v1", "job-worker-request"),
    ("job-worker-assignment-v1", "job-worker-assignment"),
    ("job-worker-status-v1", "job-worker-status"),
    ("job-worker-result-v1", "job-worker-result"),
    ("job-worker-receipt-v1", "job-worker-receipt"),
    ("job-worker-schedule-receipt-v1", "job-worker-schedule-receipt"),
    ("artifact-v1", "artifact-registry-artifact"),
    ("artifact-name-pointer-v1", "artifact-registry-name-pointer"),
    ("artifact-receipt-v1", "artifact-registry-receipt"),
    ("artifact-closure-v1", "artifact-registry-closure"),
    ("schema-identity-v1", "schema-identity"),
    ("schema-alias-v1", "schema-alias"),
    ("schema-compatibility-v1", "schema-compatibility"),
    ("schema-compatibility-receipt-v1", "schema-compatibility-receipt"),
    ("eval-cache-key-v1", "eval-cache-key"),
    ("eval-cache-value-v1", "eval-cache-value"),
    ("eval-cache-receipt-v1", "eval-cache-receipt"),
    ("transcript-artifact-v1", "transcript-artifact"),
    ("transcript-stanza-v1", "transcript-stanza"),
    ("transcript-stanza-outcome-v1", "transcript-stanza-outcome"),
    ("transcript-run-receipt-v1", "transcript-run-receipt"),
    ("rewrite-query-v1", "rewrite-query"),
    ("rewrite-match-v1", "rewrite-match"),
    ("rewrite-diff-v1", "rewrite-diff"),
    ("rewrite-plan-v1", "rewrite-plan"),
    ("rewrite-receipt-v1", "rewrite-receipt"),
    ("harness-report-v1", "report"),
    ("harness-repro-bundle-v1", "repro-bundle"),
    ("gate-receipt-v1", "gate-receipt"),
    ("repro-verify-receipt-v1", "repro-verify-receipt"),
    ("harness-failure-v1", "failure"),
    ("signed-receipt-v1", "signed-receipt"),
    ("signed-receipt-key-v1", "signed-receipt-key"),
    ("signed-receipt-key-revocation-v1", "signed-receipt-key-revocation"),
    ("retention-class-v1", "retention-class"),
    ("retention-pin-v1", "retention-pin"),
    ("retention-reference-index-v1", "retention-reference-index"),
    ("retention-evidence-admission-v1", "retention-evidence-admission"),
    ("retention-remote-gc-clearance-v1", "retention-remote-gc-clearance"),
    ("retention-remote-gc-clearance-request-v1", "retention-remote-gc-clearance-request"),
    ("retention-remote-gc-clearance-response-v1", "retention-remote-gc-clearance-response"),
    ("retention-remote-gc-clearance-import-v1", "retention-remote-gc-clearance-import"),
    ("retention-remote-gc-clearance-live-workflow-v1", "retention-remote-gc-clearance-live-workflow"),
    ("retention-gc-plan-v1", "retention-gc-plan"),
    ("retention-gc-apply-v1", "retention-gc-apply"),
    ("retention-gc-execute-v1", "retention-gc-execute"),
    ("retention-gc-audit-v1", "retention-gc-audit"),
    ("retention-candidate-explain-v1", "retention-candidate-explain"),
    ("retention-candidate-bundle-v1", "retention-candidate-bundle"),
    ("retention-candidate-bundle-profile-v1", "retention-candidate-bundle-profile"),
    ("retention-candidate-bundle-verify-v1", "retention-candidate-bundle-verify"),
    ("retention-receipt-v1", "retention-receipt"),
    ("retention-tombstone-v1", "retention-tombstone"),
    ("chain-link-v1", "chain-link"),
    ("chain-append-receipt-v1", "chain-append-receipt"),
    ("chain-verify-receipt-v1", "chain-verify-receipt"),
    ("chain-predicate-receipt-v1", "chain-predicate-receipt"),
    ("chain-fork-evidence-v1", "chain-fork-evidence"),
    ("chain-anchor-v1", "chain-anchor"),
    ("chain-checkpoint-v1", "chain-checkpoint"),
    ("chain-segment-bundle-v1", "chain-segment-bundle"),
    ("iroh-repro-exchange-receipt-v1", "iroh-repro-exchange-receipt"),
    ("iroh-chain-exchange-receipt-v1", "iroh-chain-exchange-receipt"),
    ("operation-id-v1", "delivery-operation-id"),
    ("delivery-scope-profile-v1", "delivery-scope-profile"),
    ("delivery-window-v1", "delivery-window"),
    ("dedup-entry-v1", "delivery-dedup-entry"),
    ("delivery-idempotency-receipt-v1", "delivery-idempotency-receipt"),
    ("retry-receipt-v1", "delivery-retry-receipt"),
    ("remote-dataspace-envelope-v1", "remote-dataspace-envelope"),
    ("remote-dataspace-transport-receipt-v1", "remote-dataspace-transport-receipt"),
    ("remote-dataspace-admission-receipt-v1", "remote-dataspace-admission-receipt"),
    ("remote-dataspace-delivery-log-v1", "remote-dataspace-delivery-log"),
    ("remote-dataspace-gate-receipt-v1", "remote-dataspace-gate-receipt"),
    ("federation-announcement-v1", "federation-announcement"),
    ("federation-inventory-v1", "federation-inventory"),
    ("federation-receipt-v1", "federation-receipt"),
    ("node-identity-v1", "node-identity"),
    ("node-identity-receipt-v1", "node-identity-receipt"),
    ("node-identity-bootstrap-v1", "node-identity-bootstrap"),
    ("node-identity-startup-v1", "node-identity-startup"),
    ("node-config-v1", "node-config"),
    ("node-startup-receipt-v1", "node-startup-receipt"),
    ("node-adapter-receipt-v1", "node-adapter-receipt"),
    ("node-control-request-v1", "node-control-request"),
    ("node-control-receipt-v1", "node-control-receipt"),
    ("node-control-lock-v1", "node-control-lock"),
    ("node-control-queue-receipt-v1", "node-control-queue-receipt"),
    ("node-control-operation-receipt-v1", "node-control-operation-receipt"),
    ("node-control-heartbeat-receipt-v1", "node-control-heartbeat-receipt"),
    ("node-control-loop-receipt-v1", "node-control-loop-receipt"),
    ("node-control-service-lock-v1", "node-control-service-lock"),
    ("node-control-service-heartbeat-receipt-v1", "node-control-service-heartbeat-receipt"),
    ("node-control-service-run-receipt-v1", "node-control-service-run-receipt"),
    ("node-control-supervisor-policy-v1", "node-control-supervisor-policy"),
    ("node-control-supervisor-receipt-v1", "node-control-supervisor-receipt"),
    ("node-control-ingress-envelope-v1", "node-control-ingress-envelope"),
    ("node-control-ingress-receipt-v1", "node-control-ingress-receipt"),
    ("node-control-live-transport-receipt-v1", "node-control-live-transport-receipt"),
    ("node-control-live-send-receipt-v1", "node-control-live-send-receipt"),
    ("node-control-live-send-retry-receipt-v1", "node-control-live-send-retry-receipt"),
    ("node-control-live-send-duplicate-receipt-v1", "node-control-live-send-duplicate-receipt"),
    ("node-control-live-workflow-receipt-v1", "node-control-live-workflow-receipt"),
    ("node-control-live-workflow-bundle-v1", "node-control-live-workflow-bundle"),
    (
        "node-control-live-workflow-bundle-export-receipt-v1",
        "node-control-live-workflow-bundle-export-receipt",
    ),
    (
        "node-control-live-workflow-bundle-import-receipt-v1",
        "node-control-live-workflow-bundle-import-receipt",
    ),
    (
        "node-control-live-workflow-bundle-verify-receipt-v1",
        "node-control-live-workflow-bundle-verify-receipt",
    ),
    (
        "node-control-live-workflow-bundle-gate-receipt-v1",
        "node-control-live-workflow-bundle-gate-receipt",
    ),
    (
        "node-control-live-workflow-bundle-apply-receipt-v1",
        "node-control-live-workflow-bundle-apply-receipt",
    ),
    (
        "node-control-live-workflow-bundle-reconcile-receipt-v1",
        "node-control-live-workflow-bundle-reconcile-receipt",
    ),
    ("node-control-live-workflow-bundle-ack-v1", "node-control-live-workflow-bundle-ack"),
    (
        "node-control-live-workflow-bundle-ack-export-receipt-v1",
        "node-control-live-workflow-bundle-ack-export-receipt",
    ),
    (
        "node-control-live-workflow-bundle-ack-import-receipt-v1",
        "node-control-live-workflow-bundle-ack-import-receipt",
    ),
    ("node-control-live-listener-receipt-v1", "node-control-live-listener-receipt"),
    ("node-control-authority-grant-v1", "node-control-authority-grant"),
    ("node-control-authority-receipt-v1", "node-control-authority-receipt"),
    ("node-control-authority-grant-import-receipt-v1", "node-control-authority-grant-import-receipt"),
    ("node-control-live-ticket-v1", "node-control-live-ticket"),
    ("node-control-live-peer-admission-v1", "node-control-live-peer-admission"),
    ("node-control-live-ticket-import-receipt-v1", "node-control-live-ticket-import-receipt"),
    ("node-health-receipt-v1", "node-health-receipt"),
    ("node-shutdown-receipt-v1", "node-shutdown-receipt"),
    ("operator-workflow-v1", "operator-workflow"),
    ("operator-step-v1", "operator-step"),
    ("operator-checkpoint-v1", "operator-checkpoint"),
    ("dogfood-report-v1", "dogfood-report"),
    ("release-gate-receipt-v1", "release-gate-receipt"),
    ("nix-dogfood-release-evidence-v1", "nix-dogfood-release-evidence"),
    ("nix-dogfood-release-verify-receipt-v1", "nix-dogfood-release-verify-receipt"),
    ("release-evidence-bundle-v1", "release-evidence-bundle"),
    ("release-evidence-bundle-verify-receipt-v1", "release-evidence-bundle-verify-receipt"),
    ("release-promotion-gate-receipt-v1", "release-promotion-gate-receipt"),
    ("release-promotion-summary-v1", "release-promotion-summary"),
    ("release-export-manifest-v1", "release-export-manifest"),
    ("release-export-verify-receipt-v1", "release-export-verify-receipt"),
    ("plugin-manifest-v1", "plugin-manifest"),
    ("plugin-host-abi-result-v1", "plugin-host-abi-result"),
    ("plugin-install-receipt-v1", "plugin-install-receipt"),
    ("plugin-permission-receipt-v1", "plugin-permission-receipt"),
    ("plugin-lifecycle-receipt-v1", "plugin-lifecycle-receipt"),
    ("plugin-hostcall-receipt-v1", "plugin-hostcall-receipt"),
    ("plugin-health-receipt-v1", "plugin-health-receipt"),
    ("plugin-upgrade-receipt-v1", "plugin-upgrade-receipt"),
    ("plugin-removal-receipt-v1", "plugin-removal-receipt"),
    ("plugin-fixture-report-v1", "plugin-fixture-report"),
    ("coordination-service-manifest-v1", "coordination-service-manifest"),
    ("coordination-request-v1", "coordination-request"),
    ("coordination-receipt-v1", "coordination-receipt"),
    ("fencing-token-v1", "coordination-fencing-token"),
    ("coordination-state-snapshot-v1", "coordination-state-snapshot"),
    ("coordination-status-assertion-v1", "coordination-status-assertion"),
    ("coordination-fixture-report-v1", "coordination-fixture-report"),
    ("coordination-apply-report-v1", "coordination-apply-report"),
    ("confidential-label-v1", "confidential-label"),
    ("secret-ref-v1", "secret-ref"),
    ("encrypted-ref-v1", "encrypted-ref"),
    ("redaction-marker-v1", "redaction-marker"),
    ("reveal-receipt-v1", "reveal-receipt"),
    ("decrypt-receipt-v1", "decrypt-receipt"),
    ("redaction-transform-receipt-v1", "redaction-transform-receipt"),
    ("secret-cleanup-receipt-v1", "secret-cleanup-receipt"),
    ("commitment-replay-receipt-v1", "commitment-replay-receipt"),
    ("private-bundle-profile-v1", "private-bundle-profile"),
    ("secrets-fixture-report-v1", "secrets-fixture-report"),
    ("peer-bootstrap-input-v1", "peer-bootstrap-input"),
    ("peer-handshake-v1", "peer-handshake"),
    ("peer-agreement-v1", "peer-agreement"),
    ("peer-bootstrap-receipt-v1", "peer-bootstrap-receipt"),
    ("provenance-record-v1", "provenance-record"),
    ("provenance-receipt-v1", "provenance-receipt"),
    ("provenance-build-record-v1", "provenance-build-record"),
    ("provenance-build-verify-receipt-v1", "provenance-build-verify-receipt"),
    ("authority-identity-v1", "authority-identity"),
    ("authority-context-v1", "authority-context"),
    ("authority-revocation-v1", "authority-revocation"),
    ("authority-receipt-v1", "authority-receipt"),
    ("authority-live-ref-v1", "authority-live-ref"),
    ("resource-grant-v1", "resource-grant"),
    ("resource-consumption-v1", "resource-consumption"),
    ("resource-receipt-v1", "resource-receipt"),
    ("resource-scheduler-v1", "resource-scheduler"),
    ("service-manifest-v1", "service-manifest"),
    ("service-demand-v1", "service-demand"),
    ("service-status-v1", "service-status"),
    ("service-supervisor-v1", "service-supervisor"),
    ("service-link-v1", "service-link"),
    ("service-monitor-v1", "service-monitor"),
    ("service-restart-policy-v1", "service-restart-policy"),
    ("service-restart-decision-v1", "service-restart-decision"),
    ("service-lifecycle-receipt-v1", "service-lifecycle-receipt"),
    ("service-cleanup-receipt-v1", "service-cleanup-receipt"),
    ("service-supervision-suite-v1", "service-supervision-suite"),
    ("service-supervision-report-v1", "service-supervision-report"),
    ("service-supervision-gate-receipt-v1", "service-supervision-gate-receipt"),
    ("service-monitor-notification-v1", "service-monitor-notification"),
    ("service-failure-v1", "service-failure"),
    ("service-retraction-v1", "service-retraction"),
    ("service-retention-input-v1", "service-retention-input"),
    ("service-owned-state-v1", "service-owned-state"),
    ("service-runtime-suite-v1", "service-runtime-suite"),
    ("service-runtime-report-v1", "service-runtime-report"),
    ("service-readiness-v1", "service-readiness"),
    ("service-replay-identity-v1", "service-replay-identity"),
    ("service-turn-context-v1", "service-turn-context"),
    ("protocol-manifest-v1", "protocol-manifest"),
    ("protocol-install-receipt-v1", "protocol-install-receipt"),
    ("protocol-endpoint-v1", "protocol-endpoint"),
    ("protocol-local-state-v1", "protocol-local-state"),
    ("protocol-session-state-v1", "protocol-session-state"),
    ("protocol-message-v1", "protocol-message"),
    ("protocol-operation-receipt-v1", "protocol-operation-receipt"),
    ("protocol-session-gate-receipt-v1", "protocol-session-gate-receipt"),
    ("raft-group-manifest-v1", "raft-group-manifest"),
    ("raft-command-envelope-v1", "raft-command-envelope"),
    ("raft-log-entry-v1", "raft-log-entry"),
    ("raft-commit-receipt-v1", "raft-commit-receipt"),
    ("raft-read-receipt-v1", "raft-read-receipt"),
    ("raft-snapshot-v1", "raft-snapshot"),
    ("raft-recovery-receipt-v1", "raft-recovery-receipt"),
    ("raft-predicate-receipt-v1", "raft-predicate-receipt"),
    ("control-registry-command-v1", "control-registry-command"),
    ("control-registry-state-v1", "control-registry-state"),
    ("control-registry-receipt-v1", "control-registry-receipt"),
    ("typed-storage-ref-v1", "typed-storage-ref"),
    ("typed-storage-receipt-v1", "typed-storage-receipt"),
    ("storage-effect-manifest-v1", "typed-storage-effect-manifest"),
    ("storage-schema-artifact-v1", "typed-storage-schema-artifact"),
    ("storage-migration-recipe-v1", "typed-storage-migration-recipe"),
    ("upgrade-plan-v1", "upgrade-plan"),
    ("upgrade-receipt-v1", "upgrade-receipt"),
    ("upgrade-name-pointer-v1", "upgrade-name-pointer"),
    ("chunk-manifest-v1", "chunk-manifest"),
    ("chunk-store-receipt-v1", "chunk-store-receipt"),
    ("chunk-lineage-v1", "chunk-lineage"),
];

pub fn artifact_kind(value: &IOValue) -> &'static str {
    for &(record_label, kind) in ARTIFACT_KIND_RECORDS {
        if value.collect_simple_record(record_label, None).is_some() {
            return kind;
        }
    }
    "artifact"
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("content")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("pins")).map_err(MoltenError::from)
}

fn content_path(root: &Path, artifact_ref: &str) -> Result<PathBuf> {
    Ok(root.join("content").join(filename_for_ref(artifact_ref)?))
}

fn pin_path(root: &Path, artifact_ref: &str) -> Result<PathBuf> {
    Ok(root.join("pins").join(filename_for_ref(artifact_ref)?))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn filename_for_ref(artifact_ref: &str) -> Result<String> {
    validate_content_ref(artifact_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("unsupported ledger artifact ref {artifact_ref}: {error}"))
    })?;
    let hex = artifact_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| MoltenError::invalid_harness("validated ledger artifact ref missing blake3 prefix"))?;
    Ok(format!("blake3_{hex}.bin"))
}

fn ref_from_filename(filename: &str) -> Option<String> {
    let reference = filename
        .strip_prefix("blake3_")
        .and_then(|hex| hex.strip_suffix(".bin"))
        .map(|hex| format!("blake3:{hex}"))?;
    validate_content_ref(&reference).ok()?;
    Some(reference)
}

fn pinned_refs(root: &Path) -> Result<Vec<String>> {
    let pins = root.join("pins");
    if !pins.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in fs::read_dir(pins).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if entry.file_type().map_err(MoltenError::from)?.is_file() {
            let reference = fs::read_to_string(entry.path()).map_err(MoltenError::from)?;
            validate_content_ref(&reference).map_err(|error| {
                MoltenError::invalid_harness(format!(
                    "ledger pin file contains invalid content ref {reference}: {error}"
                ))
            })?;
            push_bounded(&mut refs, reference, MAX_LEDGER_SCAN_ENTRIES, "ledger pinned refs")?;
        }
    }
    Ok(refs)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::preserves_rail::parse_text;

    #[test]
    fn ledger_import_is_immutable_and_gc_preserves_pins() {
        let root = temp_dir("ledger");
        let artifact = parse_text("<example \"ok\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let duplicate = import_artifact(&root, &artifact).expect("import duplicate");
        assert_eq!(imported.artifact_ref, duplicate.artifact_ref);
        assert_eq!(list_artifacts(&root).expect("list artifacts").len(), 1);
        pin_artifact(&root, &imported.artifact_ref).expect("pin artifact");
        let retention_evidence = retention::DestructiveRetentionEvidence::default();
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn ledger_rejects_malformed_and_missing_content_refs_before_path_use() {
        let root = temp_dir("ledger-ref-shape");
        ensure_dirs(&root).expect("ledger dirs");
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ] {
            assert!(read_artifact(&root, invalid).is_err(), "invalid read ref accepted: {invalid}");
            assert!(pin_artifact(&root, invalid).is_err(), "invalid pin ref accepted: {invalid}");
            assert!(export_artifact(&root, invalid, &root.join("out.preserves")).is_err());
        }
        let missing = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let error = read_artifact(&root, missing).expect_err("valid-shaped missing ref is not materialized");
        assert!(error.to_string().contains("No such file") || error.to_string().contains("os error"));
    }

    #[test]
    fn ledger_read_detects_tampered_materialized_bytes() {
        let root = temp_dir("ledger-tampered-bytes");
        let artifact = parse_text("<example \"original\">").expect("parse original");
        let imported = import_artifact(&root, &artifact).expect("import original");
        let tampered = parse_text("<example \"tampered\">").expect("parse tampered");
        fs::write(
            content_path(&root, &imported.artifact_ref).expect("content path"),
            canonical_bytes(&tampered).expect("tampered canonical bytes"),
        )
        .expect("tamper ledger bytes");
        let error = read_artifact(&root, &imported.artifact_ref).expect_err("tampered bytes denied");
        assert!(error.to_string().contains("ledger content hash mismatch"));
    }

    #[test]
    fn ledger_gc_requires_retention_pass_before_removal() {
        let root = temp_dir("ledger-retention");
        let artifact = parse_text("<example \"retained\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let owner_ref = canonical_hash(&record("ledger-test-ref", vec![string("owner")])).expect("owner ref");
        let policy_refs = vec![canonical_hash(&record("ledger-test-ref", vec![string("policy")])).expect("policy ref")];
        let evidence_refs =
            vec![canonical_hash(&record("ledger-test-ref", vec![string("evidence")])).expect("evidence ref")];
        crate::retention::pin_object(&root, crate::retention::RetentionPinInput {
            object_ref: imported.artifact_ref.clone(),
            object_kind: imported.artifact_kind.clone(),
            retention_class: crate::retention::CLASS_AUDIT_RECEIPT.to_string(),
            source: crate::retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold".to_string(),
            owner_ref,
            expiry_ref: None,
            policy_refs,
            evidence_refs,
            has_authority: true,
        })
        .expect("retention pin");
        let retention_evidence = retention_evidence(
            &root,
            "retention-pin",
            &imported.artifact_ref,
            &imported.artifact_kind,
            ledger_retention_class(&imported.artifact_kind),
            retention::ACTION_DELETE,
        );
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn ledger_gc_denies_missing_retention_authority_evidence() {
        let root = temp_dir("ledger-retention-missing-authority");
        let artifact = parse_text("<example \"missing-authority\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_evidence = retention_evidence_without_authority(
            &root,
            "missing-authority",
            &imported.artifact_ref,
            &imported.artifact_kind,
            ledger_retention_class(&imported.artifact_kind),
            retention::ACTION_DELETE,
        );
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn ledger_gc_denies_missing_policy_and_supporting_evidence() {
        let root = temp_dir("ledger-retention-missing-policy-evidence");
        let artifact = parse_text("<example \"missing-policy-evidence\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_evidence = retention_evidence_without_policy_evidence(
            &root,
            "missing-policy-evidence",
            &imported.artifact_ref,
            &imported.artifact_kind,
            ledger_retention_class(&imported.artifact_kind),
            retention::ACTION_DELETE,
        );
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn ledger_gc_requires_per_remote_clearance_before_removal() {
        let root = temp_dir("ledger-retention-remote-clearance");
        let artifact = parse_text("<example \"remote-clearance\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = ledger_retention_class(&imported.artifact_kind);
        let mut retention_evidence = retention_evidence(
            &root,
            "remote-clearance",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            retention::ACTION_DELETE,
        );
        let peer_ref = ledger_test_ref("remote-peer", "remote-clearance");
        let remote_ref = ledger_test_ref("remote-cache", "remote-clearance");
        retention_evidence.remote_peer_refs = vec![peer_ref.clone()];
        retention_evidence.remote_refs = vec![remote_ref.clone()];
        retention_evidence.remote_gc_refs = vec![store_admission(
            &root,
            retention::ADMISSION_KIND_REMOTE_GC,
            "remote-clearance",
            retention_evidence.requester_ref.as_deref().expect("requester"),
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            retention::ACTION_DELETE,
            &retention_evidence.remote_refs,
            true,
        )];
        let denied = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("remote clearance missing denies");
        assert_eq!(denied.decision, "deny");
        assert!(denied.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        let clearance =
            retention::store_retention_remote_gc_clearance(&root, &retention::RetentionRemoteGcClearanceInput {
                decision: "pass",
                requester_ref: retention_evidence.requester_ref.as_deref().expect("requester"),
                peer_ref: &peer_ref,
                object_ref: &imported.artifact_ref,
                object_kind: &imported.artifact_kind,
                retention_class,
                action: retention::ACTION_DELETE,
                remote_ref: &remote_ref,
                policy_ref: &retention_evidence.policy_refs[0],
                authority_ref: &retention_evidence.authority_refs[0],
                evidence_refs: &retention_evidence.evidence_refs,
                retained_refs: &[],
                is_current: true,
                revoked_refs: &[],
                diagnostics: &[],
            })
            .expect("store remote clearance")
            .clearance_ref;
        retention_evidence.remote_clearance_refs = vec![clearance];
        let apply_refs = vec![apply_ref_for(
            &root,
            "ledger-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        let passed = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("remote clearance pass removes");
        assert_eq!(passed.decision, "pass");
        assert_eq!(passed.removed_refs, vec![imported.artifact_ref.clone()]);
        assert!(read_artifact(&root, &imported.artifact_ref).is_err());
    }

    #[test]
    fn ledger_gc_requires_apply_ref_before_removal() {
        let root = temp_dir("ledger-execution-missing-apply");
        let artifact = parse_text("<example \"missing-apply\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = ledger_retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "missing-apply",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            retention::ACTION_DELETE,
        );
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc denied without apply");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        let gate = retention::read_retention_gc_execution_gate(&root, &gc.execution_gate_refs[0])
            .expect("read execution gate");
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-execute-apply-missing"));
    }

    #[test]
    fn ledger_gc_rejects_wrong_scope_apply_ref_before_removal() {
        let root = temp_dir("ledger-execution-wrong-scope");
        let artifact = parse_text("<example \"wrong-scope\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = ledger_retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "wrong-scope",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            retention::ACTION_DELETE,
        );
        let apply_refs = vec![apply_ref_for(
            &root,
            "chunk-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc denied with wrong apply scope");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        let gate = retention::read_retention_gc_execution_gate(&root, &gc.execution_gate_refs[0])
            .expect("read execution gate");
        assert!(
            gate.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-execute-apply-scope-mismatch"),
            "{:?}",
            gate.diagnostics
        );
    }

    #[test]
    fn ledger_gc_rejects_drift_after_apply_before_removal() {
        let root = temp_dir("ledger-execution-drift");
        let artifact = parse_text("<example \"drift\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = ledger_retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "drift",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            retention::ACTION_DELETE,
        );
        let apply_refs = vec![apply_ref_for(
            &root,
            "ledger-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        retention::pin_object(&root, retention::RetentionPinInput {
            object_ref: imported.artifact_ref.clone(),
            object_kind: imported.artifact_kind.clone(),
            retention_class: retention_class.to_string(),
            source: retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "post-apply drift".to_string(),
            owner_ref: ledger_test_ref("owner", "drift"),
            expiry_ref: None,
            policy_refs: vec![ledger_test_ref("pin-policy", "drift")],
            evidence_refs: vec![ledger_test_ref("pin-evidence", "drift")],
            has_authority: true,
        })
        .expect("post-apply retention pin");
        let gc = gc(&root, LedgerGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc denied after drift");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        assert!(!gc.execution_gate_refs.is_empty());
    }

    #[test]
    fn ledger_detects_corrupted_content_bytes() {
        let root = temp_dir("ledger-corrupt");
        let artifact = parse_text("<example \"ok\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        fs::write(content_path(&root, &imported.artifact_ref).expect("content path"), b"not preserves")
            .expect("corrupt artifact");
        let error = read_artifact(&root, &imported.artifact_ref).expect_err("corruption fails");
        assert!(["Preserves", "hash mismatch"].iter().any(|needle| error.to_string().contains(needle)));
    }

    fn apply_ref_for(
        root: &std::path::Path,
        subsystem: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        evidence: &retention::DestructiveRetentionEvidence,
    ) -> String {
        let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
            root,
            subsystem,
            object_ref,
            object_kind,
            retention_class,
            action: retention::ACTION_DELETE,
            evidence,
        })
        .expect("store ledger GC plan");
        retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply ledger GC plan")
        .apply_ref
    }

    fn retention_evidence(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> retention::DestructiveRetentionEvidence {
        let requester_ref = ledger_test_ref("requester", label);
        let policy_refs = vec![store_admission(
            root,
            retention::ADMISSION_KIND_POLICY,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let authority_refs = vec![store_admission(
            root,
            retention::ADMISSION_KIND_AUTHORITY,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let evidence_refs = vec![store_admission(
            root,
            retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let reference_index_refs = vec![store_admission(
            root,
            retention::ADMISSION_KIND_REFERENCE_INDEX,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        retention::DestructiveRetentionEvidence {
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

    fn retention_evidence_without_authority(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> retention::DestructiveRetentionEvidence {
        let mut evidence = retention_evidence(root, label, object_ref, object_kind, retention_class, action);
        evidence.authority_refs.clear();
        evidence
    }

    fn retention_evidence_without_policy_evidence(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> retention::DestructiveRetentionEvidence {
        let mut evidence = retention_evidence(root, label, object_ref, object_kind, retention_class, action);
        evidence.policy_refs.clear();
        evidence.evidence_refs.clear();
        evidence
    }

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
        remote_refs: &[String],
        is_reference_index_complete: bool,
    ) -> String {
        retention::store_retention_evidence_admission(root, &retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            bound_refs: &[ledger_test_ref(kind, label)],
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

    fn ledger_test_ref(kind: &str, label: &str) -> String {
        canonical_hash(&record("ledger-test-ref", vec![string(kind), string(label)])).expect("ledger test ref")
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
