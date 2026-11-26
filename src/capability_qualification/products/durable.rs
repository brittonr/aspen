//! Durable state recovery over the live capability-rooted Redb adapter. The
//! restart boundary is a full close and reopen of the store in this process.

use crate::fabric_durability::AdapterHealth;
use crate::fabric_durability::AppendRequest;
use crate::fabric_durability::AtomicityDomain;
use crate::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA;
use crate::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA;
use crate::fabric_durability::DurabilityLevel;
use crate::fabric_durability::DurableAdapterKind;
use crate::fabric_durability::DurableNamespaceDescriptor;
use crate::fabric_durability::DurableObjectClass;
use crate::fabric_durability::DurableProfile;
use crate::fabric_durability::DurableStateProfile;
use crate::fabric_durability::QuarantineReason;
use crate::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS;
use crate::fabric_durability::RecoveryDisposition;
use crate::fabric_durability::RecoveryInventory;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::SnapshotKind;
use crate::fabric_durability::SnapshotRequest;

const ADAPTER_ID: &str = "qualification-redb";
const NAMESPACE_ID: &str = "qualification-namespace";
const DOMAIN_ID: &str = "qualification-domain";
const GENERATION: u64 = 1;
const PROFILE_LIMIT: u64 = 32;
const OPERATION_BYTE_LIMIT: u64 = 4_096;
const NAMESPACE_BYTE_LIMIT: u64 = 65_536;
const FIRST_SEQUENCE: u64 = 0;
const RECORD: &[u8] = b"qualification durable record";
const SNAPSHOT_DIRECTORY: &str = "snapshots";

pub(super) fn run(scenario: super::super::Scenario) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::DurableRestartRecovery => restart_recovery(),
        super::super::Scenario::DurableCorruptSnapshotDenied => corrupt_snapshot_denied(),
        _ => Err(super::super::invalid("scenario is not a durable-state recovery scenario")),
    }
}

// r[impl molten.fabric_durability.snapshot_recovery]
fn restart_recovery() -> crate::error::Result<super::Evidence> {
    let scratch = super::scratch("durable-restart")?;
    let mut evidence = super::Evidence::default();
    let payload = snapshot_payload()?;
    {
        let mut adapter = RedbDurableStateAdapter::open(scratch.path(), profile()?, descriptor())?;
        write_record_and_checkpoint(&mut adapter, &payload, &mut evidence)?;
    }
    let reopened = RedbDurableStateAdapter::open(scratch.path(), profile()?, descriptor())?;
    evidence.check("reopen-serving", reopened.health() == AdapterHealth::Serving);
    evidence.check("acknowledged-log-recovered", reopened.state().durable_log.len() == 1);
    evidence.check("checkpoint-inventoried", reopened.state().snapshots.contains_key(&snapshot_ref()));
    let recovery = reopened.recovery(&inventory(GENERATION))?;
    evidence.check("recovery-admitted", recovery.decision.disposition == RecoveryDisposition::Admit);
    evidence.check("recovery-diagnostics-empty", recovery.decision.diagnostics.is_empty());
    evidence.runtime(recovery.value);
    let (plan, bytes) = reopened.load_snapshot_bytes(&snapshot_ref(), GENERATION)?;
    evidence.check("restored-bytes-match-content-ref", bytes == payload);
    evidence.check("restored-state-ref-bound", plan.restored_state_ref == ordered_state_ref());
    evidence.runtime(reopened.status()?.value);
    Ok(evidence)
}

// r[impl molten.fabric_durability.snapshot_recovery]
fn corrupt_snapshot_denied() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let payload = snapshot_payload()?;

    let tampered_root = super::scratch("durable-tampered-create")?;
    let mut adapter = RedbDurableStateAdapter::open(tampered_root.path(), profile()?, descriptor())?;
    // The covered log record exists first, so only the identity mismatch can deny.
    append_record(&mut adapter)?;
    let mut request = snapshot_request(&payload);
    request.content_ref = crate::preserves_rail::content_ref_from_bytes(b"bytes the caller claims");
    let tampered = adapter.create_snapshot(&request, &payload);
    evidence.check("mismatched-snapshot-identity-denied", tampered.is_err());
    evidence.check("mismatched-snapshot-not-inventoried", adapter.state().snapshots.is_empty());
    // Untampered control on the same store: the matching identity is admitted.
    let control = adapter.create_snapshot(&snapshot_request(&payload), &payload);
    evidence.check("untampered-snapshot-control-admitted", control.is_ok());
    evidence.check("untampered-snapshot-control-inventoried", adapter.state().snapshots.contains_key(&snapshot_ref()));
    drop(adapter);

    let corrupt_root = super::scratch("durable-corrupt")?;
    {
        let mut adapter = RedbDurableStateAdapter::open(corrupt_root.path(), profile()?, descriptor())?;
        write_record_and_checkpoint(&mut adapter, &payload, &mut evidence)?;
    }
    corrupt_snapshot_payload(corrupt_root.path())?;
    let reopened = RedbDurableStateAdapter::open(corrupt_root.path(), profile()?, descriptor())?;
    evidence.check(
        "corrupt-payload-quarantined-on-reopen",
        reopened.health() == AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent),
    );
    let refused_recovery = reopened.recovery(&inventory(GENERATION));
    evidence.check("quarantined-recovery-refused", refused_recovery.is_err());
    if let Err(error) = &refused_recovery {
        evidence.runtime(super::error_class(error));
    }
    evidence.check(
        "quarantined-snapshot-read-refused",
        reopened.load_snapshot_bytes(&snapshot_ref(), GENERATION).is_err(),
    );
    evidence.runtime(reopened.status()?.value);
    drop(reopened);

    let stale_root = super::scratch("durable-stale")?;
    {
        let mut adapter = RedbDurableStateAdapter::open(stale_root.path(), profile()?, descriptor())?;
        write_record_and_checkpoint(&mut adapter, &payload, &mut evidence)?;
    }
    let reopened = RedbDurableStateAdapter::open(stale_root.path(), profile()?, descriptor())?;
    let stale = reopened.recovery(&inventory(GENERATION + 1))?;
    evidence.check("stale-generation-recovery-not-admitted", stale.decision.disposition != RecoveryDisposition::Admit);
    evidence.check("stale-generation-diagnosed", !stale.decision.diagnostics.is_empty());
    evidence.runtime(stale.value);
    Ok(evidence)
}

fn write_record_and_checkpoint(
    adapter: &mut RedbDurableStateAdapter,
    payload: &[u8],
    evidence: &mut super::Evidence,
) -> crate::error::Result<()> {
    let append = append_record(adapter)?;
    let snapshot = adapter.create_snapshot(&snapshot_request(payload), payload)?;
    // The same fixed writes appear in several scenarios; record them once per run.
    if !evidence.runtime.iter().any(|value| value == &append.value) {
        evidence.runtime(append.value);
        evidence.runtime(snapshot.value);
    }
    Ok(())
}

fn append_record(
    adapter: &mut RedbDurableStateAdapter,
) -> crate::error::Result<crate::fabric_durability::TransitionEnvelope> {
    adapter.append(&AppendRequest {
        adapter_id: ADAPTER_ID.to_string(),
        namespace_id: NAMESPACE_ID.to_string(),
        generation: GENERATION,
        expected_sequence: FIRST_SEQUENCE,
        value: RECORD.to_vec(),
        value_ref: crate::preserves_rail::content_ref_from_bytes(RECORD),
        durability: DurabilityLevel::ProcessLoss,
    })
}

fn corrupt_snapshot_payload(root: &std::path::Path) -> crate::error::Result<()> {
    let mut payloads = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).map_err(crate::error::Failure::from)? {
            let path = entry.map_err(crate::error::Failure::from)?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "bin")
                && path.parent().and_then(std::path::Path::file_name).is_some_and(|name| name == SNAPSHOT_DIRECTORY)
            {
                payloads.push(path);
            }
        }
    }
    let [payload] = payloads.as_slice() else {
        return Err(super::super::invalid("expected exactly one published snapshot payload to corrupt"));
    };
    std::fs::write(payload, b"corrupted snapshot payload").map_err(crate::error::Failure::from)
}

fn snapshot_payload() -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("qualification-durable-snapshot-v1", vec![
        crate::preserves_rail::string(NAMESPACE_ID),
        crate::preserves_rail::u64_value(GENERATION),
        crate::preserves_rail::u64_value(FIRST_SEQUENCE),
    ]))
}

fn snapshot_request(payload: &[u8]) -> SnapshotRequest {
    SnapshotRequest {
        kind: SnapshotKind::Checkpoint,
        generation: GENERATION,
        snapshot_ref: snapshot_ref(),
        content_ref: crate::preserves_rail::content_ref_from_bytes(payload),
        ordered_state_ref: ordered_state_ref(),
        covered_log_sequence: Some(FIRST_SEQUENCE),
        durability: DurabilityLevel::ProcessLoss,
    }
}

fn inventory(active_generation: u64) -> RecoveryInventory {
    RecoveryInventory {
        active_generation,
        expected_schema_ref: reference("qualification durable value schema"),
        permit_repair: false,
        permit_quarantine: false,
    }
}

fn profile() -> crate::error::Result<DurableProfile> {
    crate::fabric_durability::durable_profile(&DurableStateProfile {
        schema: DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: DurableAdapterKind::LiveRedb.as_str().to_string(),
        profile_ref: reference("qualification durable profile declaration"),
        adapter_kind: DurableAdapterKind::LiveRedb,
        supported_levels: vec![
            DurabilityLevel::Buffered,
            DurabilityLevel::ProcessLoss,
            DurabilityLevel::MachineLoss,
        ],
        max_namespaces: PROFILE_LIMIT,
        max_log_records: PROFILE_LIMIT,
        max_ordered_entries: PROFILE_LIMIT,
        max_operation_bytes: OPERATION_BYTE_LIMIT,
        max_namespace_bytes: NAMESPACE_BYTE_LIMIT,
        max_batch_operations: PROFILE_LIMIT,
        max_snapshots: PROFILE_LIMIT,
        max_effect_transactions: PROFILE_LIMIT,
        non_claims: REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    })
}

fn descriptor() -> DurableNamespaceDescriptor {
    let levels = vec![
        DurabilityLevel::Buffered,
        DurabilityLevel::ProcessLoss,
        DurabilityLevel::MachineLoss,
    ];
    DurableNamespaceDescriptor {
        schema: DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: reference("qualification durable profile declaration"),
        adapter_id: ADAPTER_ID.to_string(),
        namespace_id: NAMESPACE_ID.to_string(),
        generation: GENERATION,
        value_schema_ref: reference("qualification durable value schema"),
        atomicity_domain: AtomicityDomain {
            domain_id: DOMAIN_ID.to_string(),
            adapter_id: ADAPTER_ID.to_string(),
            namespace_id: NAMESPACE_ID.to_string(),
            generation: GENERATION,
            object_classes: vec![
                DurableObjectClass::LogRecord,
                DurableObjectClass::OrderedValue,
                DurableObjectClass::Snapshot,
                DurableObjectClass::Checkpoint,
                DurableObjectClass::EffectTransaction,
            ],
            max_operations: PROFILE_LIMIT,
            max_bytes: OPERATION_BYTE_LIMIT,
            supported_levels: levels,
        },
        retention_authority_ref: Some(reference("qualification durable retention authority")),
        quota_bytes: NAMESPACE_BYTE_LIMIT,
    }
}

fn snapshot_ref() -> String {
    reference("qualification durable checkpoint")
}

fn ordered_state_ref() -> String {
    reference("qualification durable ordered state")
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
