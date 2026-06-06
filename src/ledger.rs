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
    pub removed_refs: Vec<String>,
    pub receipt_value: IOValue,
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

pub fn gc(root: &Path, dry_run: bool) -> Result<LedgerGc> {
    ensure_dirs(root)?;
    let pins = pinned_refs(root)?;
    let mut removed_refs = Vec::new();
    for entry in list_artifacts(root)? {
        if pins.iter().any(|pin| pin == &entry.artifact_ref) {
            continue;
        }
        push_bounded(&mut removed_refs, entry.artifact_ref.clone(), MAX_LEDGER_SCAN_ENTRIES, "ledger removed refs")?;
        if !dry_run {
            fs::remove_file(content_path(root, &entry.artifact_ref)?).map_err(MoltenError::from)?;
        }
    }
    let receipt_value = record("ledger-gc-receipt-v1", vec![
        string(EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("mode", vec![string(if dry_run { "dry-run" } else { "apply" })]),
        record("removed", vec![sequence(removed_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("pin-preservation"), string("pass")]),
            record("check", vec![string("derived-index-scan"), string("pass")]),
        ])]),
    ]);
    Ok(LedgerGc {
        dry_run,
        removed_refs,
        receipt_value,
    })
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
    ("job-worker-request-v1", "job-worker-request"),
    ("job-worker-assignment-v1", "job-worker-assignment"),
    ("job-worker-status-v1", "job-worker-status"),
    ("job-worker-result-v1", "job-worker-result"),
    ("job-worker-receipt-v1", "job-worker-receipt"),
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
    artifact_ref.strip_prefix("blake3:").map(|hex| format!("blake3_{hex}.bin")).ok_or_else(|| {
        MoltenError::invalid_harness(format!("unsupported ledger artifact ref {artifact_ref}; expected blake3 ref"))
    })
}

fn ref_from_filename(filename: &str) -> Option<String> {
    filename
        .strip_prefix("blake3_")
        .and_then(|hex| hex.strip_suffix(".bin"))
        .map(|hex| format!("blake3:{hex}"))
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
            push_bounded(
                &mut refs,
                fs::read_to_string(entry.path()).map_err(MoltenError::from)?,
                MAX_LEDGER_SCAN_ENTRIES,
                "ledger pinned refs",
            )?;
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
        let gc = gc(&root, false).expect("gc ledger");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
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
