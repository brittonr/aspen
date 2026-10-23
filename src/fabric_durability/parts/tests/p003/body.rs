
// Snapshot payload publication: payload and parent synchronization precede
// metadata, and a failed phase never commits metadata.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicationFault {
    None,
    Write,
    ParentSync,
}

/// Wraps the Linux publication adapter and fails one phase while the shared
/// switch selects it.
struct FaultPublication {
    inner: durable_file_publication::shell::LinuxDirectory,
    fault: std::sync::Arc<std::sync::Mutex<PublicationFault>>,
}

impl FaultPublication {
    fn fault(&self) -> PublicationFault {
        *self.fault.lock().expect("fault lock")
    }
}

impl durable_file_publication::shell::PublicationIo for FaultPublication {
    fn supports_no_replace(&self) -> bool {
        self.inner.supports_no_replace()
    }

    fn supports_parent_sync(&self) -> bool {
        self.inner.supports_parent_sync()
    }

    fn create_stage(
        &mut self,
        leaf: &str,
        private_mode: u32,
    ) -> Result<durable_file_publication::shell::StageCreateDisposition, durable_file_publication::shell::MechanicalIoFailure>
    {
        self.inner.create_stage(leaf, private_mode)
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        if self.fault() == PublicationFault::Write {
            return Err(durable_file_publication::shell::MechanicalIoFailure);
        }
        self.inner.write_all(bytes)
    }

    fn apply_permissions(&mut self, mode: u32) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.inner.apply_permissions(mode)
    }

    fn sync_payload(&mut self) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.inner.sync_payload()
    }

    fn rename_stage(
        &mut self,
        stage_leaf: &str,
        destination_leaf: &str,
        replacement: durable_file_publication::core::ReplacementMode,
    ) -> Result<durable_file_publication::shell::RenameDisposition, durable_file_publication::shell::MechanicalIoFailure>
    {
        self.inner.rename_stage(stage_leaf, destination_leaf, replacement)
    }

    fn sync_parent(&mut self) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        if self.fault() == PublicationFault::ParentSync {
            return Err(durable_file_publication::shell::MechanicalIoFailure);
        }
        self.inner.sync_parent()
    }

    fn cleanup_stage(&mut self, stage_leaf: &str) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.inner.cleanup_stage(stage_leaf)
    }
}

const SNAPSHOT_BYTES: &[u8] = b"published snapshot payload";

fn snapshot_request(bytes: &[u8]) -> SnapshotRequest {
    SnapshotRequest {
        kind: SnapshotKind::Snapshot,
        generation: GENERATION,
        snapshot_ref: SNAPSHOT_REF.to_string(),
        content_ref: format!("blake3:{}", blake3::hash(bytes).to_hex()),
        ordered_state_ref: ORDERED_REF.to_string(),
        covered_log_sequence: None,
        durability: DurabilityLevel::MachineLoss,
    }
}

fn payload_path(root: &std::path::Path, bytes: &[u8]) -> std::path::PathBuf {
    root.join("snapshots").join(format!("{}.bin", blake3::hash(bytes).to_hex()))
}

fn publishing_adapter(
    root: &std::path::Path,
    disk: &FaultDisk,
) -> (RedbDurableStateAdapter, std::sync::Arc<std::sync::Mutex<PublicationFault>>) {
    let fault = std::sync::Arc::new(std::sync::Mutex::new(PublicationFault::None));
    let switch = std::sync::Arc::clone(&fault);
    let publication = PublicationSource::Wrapped(std::sync::Arc::new(move |inner| {
        Box::new(FaultPublication {
            inner,
            fault: std::sync::Arc::clone(&switch),
        })
    }));
    let adapter = RedbDurableStateAdapter::open_with(
        root,
        profile(DurableAdapterKind::LiveRedb),
        descriptor(),
        disk.source(),
        publication,
    )
    .expect("open publishing adapter");
    (adapter, fault)
}

// r[verify molten.fabric_durability.snapshot_publication]
#[test]
fn a_snapshot_becomes_visible_only_after_its_payload_and_parent_synchronize() {
    let root = crate::test_support::process_workspace("fabric-durability-snapshot-publish").expect("workspace");
    let disk = FaultDisk::default();
    let (mut adapter, _fault) = publishing_adapter(&root, &disk);
    adapter.create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES).expect("published snapshot");
    assert!(payload_path(&root, SNAPSHOT_BYTES).exists());
    let (_plan, bytes) = adapter.load_snapshot_bytes(SNAPSHOT_REF, GENERATION).expect("load the snapshot");
    assert_eq!(bytes, SNAPSHOT_BYTES);
    let recovered = fault_adapter(&root, &disk.power_loss_image());
    assert!(recovered.state().snapshots.contains_key(SNAPSHOT_REF), "the metadata survived power loss");
}

// r[verify molten.fabric_durability.snapshot_publication]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn a_partial_publication_commits_no_payload_and_no_metadata() {
    let root = crate::test_support::process_workspace("fabric-durability-snapshot-partial").expect("workspace");
    let disk = FaultDisk::default();
    let (mut adapter, fault) = publishing_adapter(&root, &disk);
    *fault.lock().expect("fault lock") = PublicationFault::Write;
    adapter.create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES).expect_err("partial publication");
    assert_eq!(adapter.health(), AdapterHealth::Serving, "nothing became visible");
    assert!(!payload_path(&root, SNAPSHOT_BYTES).exists());
    assert!(adapter.state().snapshots.is_empty());
}

// r[verify molten.fabric_durability.snapshot_publication]
// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn a_failed_parent_synchronization_withholds_metadata_until_reconciliation() {
    let root = crate::test_support::process_workspace("fabric-durability-snapshot-parent").expect("workspace");
    let disk = FaultDisk::default();
    let (mut adapter, fault) = publishing_adapter(&root, &disk);
    *fault.lock().expect("fault lock") = PublicationFault::ParentSync;
    adapter.create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES).expect_err("unknown durability");
    assert_eq!(adapter.health(), AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown));
    assert!(payload_path(&root, SNAPSHOT_BYTES).exists(), "the payload is visible");
    assert!(adapter.state().snapshots.is_empty(), "no metadata names it");
    *fault.lock().expect("fault lock") = PublicationFault::None;
    let mut reconciled =
        adapter.reopen_and_reconcile(ReconciliationReference::Acknowledged).expect("reopen and reconcile");
    assert_eq!(reconciled.health(), AdapterHealth::Serving);
    reconciled
        .create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES)
        .expect("the existing destination is verified, then metadata commits");
    assert!(reconciled.state().snapshots.contains_key(SNAPSHOT_REF));
}

// r[verify molten.fabric_durability.snapshot_publication]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn a_payload_that_does_not_match_its_identity_quarantines() {
    let root = crate::test_support::process_workspace("fabric-durability-snapshot-identity").expect("workspace");
    let disk = FaultDisk::default();
    let (mut adapter, _fault) = publishing_adapter(&root, &disk);
    std::fs::create_dir_all(root.join("snapshots")).expect("snapshot directory");
    std::fs::write(payload_path(&root, SNAPSHOT_BYTES), b"other bytes").expect("plant a mismatched payload");
    adapter.create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES).expect_err("identity mismatch");
    assert_eq!(adapter.health(), AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent));

    let other_root = crate::test_support::process_workspace("fabric-durability-snapshot-startup").expect("workspace");
    let other_disk = FaultDisk::default();
    let (mut published, _fault) = publishing_adapter(&other_root, &other_disk);
    published.create_snapshot(&snapshot_request(SNAPSHOT_BYTES), SNAPSHOT_BYTES).expect("published snapshot");
    drop(published);
    std::fs::write(payload_path(&other_root, SNAPSHOT_BYTES), b"tampered").expect("tamper the payload");
    let reopened = fault_adapter(&other_root, &other_disk);
    assert_eq!(
        reopened.health(),
        AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent),
        "startup checks every snapshot payload's identity"
    );
}
