
// Storage faults over a simulated medium. The medium keeps the bytes the
// process sees apart from the bytes a synchronization persisted, so a failed
// synchronization can leave the process seeing a write the medium lacks, and
// a power loss discards everything not synchronized.

#[derive(Debug, Default)]
struct Medium {
    persisted: Vec<u8>,
    visible: Vec<u8>,
    is_sync_failing: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FaultDisk(std::sync::Arc<std::sync::Mutex<Medium>>);

impl FaultDisk {
    pub(crate) fn source(&self) -> DatabaseSource {
        let disk = self.clone();
        DatabaseSource::Backend(std::sync::Arc::new(move || Box::new(FaultBackend(disk.clone()))))
    }

    fn medium(&self) -> std::sync::MutexGuard<'_, Medium> {
        self.0.lock().expect("medium lock")
    }

    pub(crate) fn fail_syncs(&self, is_failing: bool) {
        self.medium().is_sync_failing = is_failing;
    }

    /// Lose everything not synchronized, in place.
    fn lose_unsynchronized(&self) {
        let mut medium = self.medium();
        medium.visible = medium.persisted.clone();
    }

    /// A separate disk holding only what was synchronized: the medium as a
    /// machine finds it after power loss while the store was open.
    pub(crate) fn power_loss_image(&self) -> Self {
        let medium = self.medium();
        Self(std::sync::Arc::new(std::sync::Mutex::new(Medium {
            persisted: medium.persisted.clone(),
            visible: medium.persisted.clone(),
            is_sync_failing: false,
        })))
    }

    /// Flip every persisted byte past the first page.
    fn corrupt_persisted(&self) {
        const HEADER_PAGE_BYTES: usize = 4096;
        let mut medium = self.medium();
        for byte in medium.persisted.iter_mut().skip(HEADER_PAGE_BYTES) {
            *byte ^= 0xA5;
        }
        medium.visible = medium.persisted.clone();
    }
}

#[derive(Debug)]
struct FaultBackend(FaultDisk);

impl redb::StorageBackend for FaultBackend {
    fn len(&self) -> Result<u64, std::io::Error> {
        Ok(self.0.medium().visible.len() as u64)
    }

    fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), std::io::Error> {
        let medium = self.0.medium();
        let start = usize::try_from(offset).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let bytes = medium
            .visible
            .get(start..start + out.len())
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        out.copy_from_slice(bytes);
        Ok(())
    }

    fn set_len(&self, len: u64) -> Result<(), std::io::Error> {
        let len = usize::try_from(len).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        self.0.medium().visible.resize(len, 0);
        Ok(())
    }

    fn sync_data(&self) -> Result<(), std::io::Error> {
        let mut medium = self.0.medium();
        if medium.is_sync_failing {
            return Err(std::io::Error::other("injected synchronization failure"));
        }
        medium.persisted = medium.visible.clone();
        Ok(())
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
        let mut medium = self.0.medium();
        let start = usize::try_from(offset).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        if medium.visible.len() < start + data.len() {
            medium.visible.resize(start + data.len(), 0);
        }
        medium.visible[start..start + data.len()].copy_from_slice(data);
        Ok(())
    }
}

fn fault_adapter(root: &std::path::Path, disk: &FaultDisk) -> RedbDurableStateAdapter {
    RedbDurableStateAdapter::open_with(
        root,
        profile(DurableAdapterKind::LiveRedb),
        descriptor(),
        disk.source(),
        PublicationSource::Linux,
    )
    .expect("open adapter over the simulated medium")
}

fn sequenced_append(sequence: u64) -> AppendRequest {
    AppendRequest {
        expected_sequence: sequence,
        durability: DurabilityLevel::MachineLoss,
        ..append_request(DurabilityLevel::MachineLoss)
    }
}

// r[verify molten.fabric_durability.storage_fault_outcomes]
#[test]
fn a_synchronized_commit_is_acknowledged_and_survives_power_loss() {
    let root = crate::test_support::process_workspace("fabric-durability-synchronized").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    let appended = adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("synchronized append");
    assert_eq!(appended.outcome, MutationOutcome::Durable);
    assert_eq!(adapter.health(), AdapterHealth::Serving);
    let recovered = fault_adapter(&root, &disk.power_loss_image());
    assert_eq!(recovered.state().durable_log, adapter.state().durable_log, "the acknowledged record survived");
}

// r[verify molten.fabric_durability.storage_fault_outcomes]
// r[verify molten.fabric_durability.adapter_quarantine]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn a_failed_synchronization_quarantines_and_a_successful_retry_does_not_clear_it() {
    let root = crate::test_support::process_workspace("fabric-durability-sync-failure").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("first append");
    disk.fail_syncs(true);
    adapter.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect_err("a failed synchronization is not success");
    assert_eq!(adapter.health(), AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown));
    assert_eq!(adapter.state().durable_log.len(), 1, "no durable acknowledgement for the uncertain record");
    assert!(adapter.read_log(FIRST_SEQUENCE).is_err(), "a quarantined adapter refuses reads");
    assert!(adapter.scan_log(FIRST_SEQUENCE, 1).is_err());
    assert!(adapter.status().is_ok(), "status stays readable");
    disk.fail_syncs(false);
    adapter.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect_err("a retry cannot clear quarantine");
    assert_eq!(adapter.health(), AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown));
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn reopen_reconciles_an_unknown_outcome_and_makes_the_result_durable() {
    let root = crate::test_support::process_workspace("fabric-durability-reconcile-cache").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("first append");
    disk.fail_syncs(true);
    adapter.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect_err("uncertain append");
    disk.fail_syncs(false);
    let reconciled = adapter.reopen_and_reconcile(ReconciliationReference::Acknowledged).expect("reopen and reconcile");
    assert_eq!(reconciled.health(), AdapterHealth::Serving);
    let length = reconciled.state().durable_log.len();
    assert!([1, 2].contains(&length), "the read-back is the acknowledged or the uncertain state");
    let recovered = fault_adapter(&root, &disk.power_loss_image());
    assert_eq!(recovered.state().durable_log, reconciled.state().durable_log, "the reconciled state is durable");
}

// r[verify molten.fabric_durability.adapter_quarantine]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn a_write_the_medium_lost_reconciles_to_the_acknowledged_state() {
    let root = crate::test_support::process_workspace("fabric-durability-reconcile-loss").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("first append");
    let acknowledged = adapter.state().durable_log.clone();
    disk.fail_syncs(true);
    adapter.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect_err("uncertain append");
    disk.fail_syncs(false);
    disk.lose_unsynchronized();
    let mut reconciled =
        adapter.reopen_and_reconcile(ReconciliationReference::Acknowledged).expect("reopen and reconcile");
    assert_eq!(reconciled.health(), AdapterHealth::Serving);
    assert_eq!(reconciled.state().durable_log, acknowledged, "the uncertain record never reached the medium");
    reconciled.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect("the record can be written again");
}

// r[verify molten.fabric_durability.adapter_quarantine]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn a_repaired_or_corrupt_store_never_opens_serving() {
    let root = crate::test_support::process_workspace("fabric-durability-repaired").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("append");
    let expected = adapter.state().clone();
    let image = disk.power_loss_image();
    let opened = fault_adapter(&root, &image);
    assert_eq!(
        opened.health(),
        AdapterHealth::Quarantined(QuarantineReason::RepairedOnOpen),
        "a store that was open at power loss needs a local repair, and a repair never opens serving"
    );
    let unreferenced =
        opened.reopen_and_reconcile(ReconciliationReference::Acknowledged).expect("reopen without a reference");
    assert_eq!(unreferenced.health(), AdapterHealth::Quarantined(QuarantineReason::ReferenceUnavailable));
    let referenced = unreferenced.reopen_and_reconcile(ReconciliationReference::Supplied(&expected)).expect("reopen");
    assert_eq!(referenced.health(), AdapterHealth::Serving, "a supplied reference reconciles a repaired store");
    assert!(referenced.is_repair_observed(), "the repair stays recorded after reconciliation");
    let self_consistent = fault_adapter(&root, &disk.power_loss_image())
        .reopen_and_reconcile(ReconciliationReference::SelfConsistent)
        .expect("explicit self-consistency reconciliation");
    assert_eq!(self_consistent.health(), AdapterHealth::Serving);
    assert_eq!(self_consistent.state().durable_log, expected.durable_log);

    let corrupt = disk.power_loss_image();
    corrupt.corrupt_persisted();
    let opened = RedbDurableStateAdapter::open_with(
        &root,
        profile(DurableAdapterKind::LiveRedb),
        descriptor(),
        corrupt.source(),
        PublicationSource::Linux,
    );
    if let Ok(opened) = opened {
        assert_ne!(opened.health(), AdapterHealth::Serving, "a corrupt store never serves");
    }
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn a_reopen_while_synchronization_still_fails_never_serves() {
    let root = crate::test_support::process_workspace("fabric-durability-reopen-failing").expect("workspace");
    let disk = FaultDisk::default();
    let mut adapter = fault_adapter(&root, &disk);
    adapter.append(&sequenced_append(FIRST_SEQUENCE)).expect("first append");
    disk.fail_syncs(true);
    adapter.append(&sequenced_append(FIRST_SEQUENCE + 1)).expect_err("uncertain append");
    let failure = adapter
        .reopen_and_reconcile(ReconciliationReference::Acknowledged)
        .expect_err("the store cannot recover while synchronization fails");
    assert!(failure.error.to_string().contains("injected synchronization failure"), "{failure:?}");
}
