// Durable reopening and uncertain backend-effect scenarios.
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn reopen_after_intent_does_not_replay_backend() {
    let (_workspace, root) = store_root("compute-intent-reopen");
    let mut store = observed_store(&root);
    let current = store.load().unwrap();
    let transition = molten_core::compute_capacity::reserve(&current, &fixture_request("operation-1"));
    assert!(matches!(transition.decision, ReservationOutcome::Reserved { .. }));
    store.compare_and_store(&current, &transition.state).unwrap();
    drop(store);

    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut service = make_service(store, RecordingBackend::default());
    let replay = service.reserve(&fixture_request("operation-1")).unwrap();
    assert!(matches!(replay, ReserveOutcome::Uncertain(_)));
    assert_eq!(service.backend.activations, 0);
    assert_eq!(service.store.load().unwrap().allocations["operation-1"].status, ReservationStatus::Intent);
    let another = service.reserve(&fixture_request("operation-2")).unwrap();
    assert!(matches!(another, ReserveOutcome::Denied(CapacityIssue::InsufficientCapacity)));
}

// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn reopen_after_possible_start_requires_explicit_reconciliation() {
    let (_workspace, root) = store_root("compute-possible-start");
    let store = observed_store(&root);
    let backend = RecordingBackend {
        is_lost_activation_ack: true,
        ..RecordingBackend::default()
    };
    let mut service = make_service(store, backend);
    let outcome = service.reserve(&fixture_request("operation-1")).unwrap();
    assert!(matches!(&outcome, ReserveOutcome::Uncertain(issue)
        if issue.phase == Phase::BackendActivation && issue.has_possible_effect));
    assert_eq!(service.backend.activations, 1);
    drop(service);

    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut reopened = make_service(store, RecordingBackend {
        is_active_observed: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(
        reopened.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Uncertain(_)
    ));
    assert_eq!(reopened.backend.activations, 0);
    let acknowledged = reopened.reconcile_activation(&owner()).unwrap();
    assert!(matches!(acknowledged, ReserveOutcome::Reserved(_)));
    assert_eq!(reopened.backend.activations, 0);
}

struct FailSecondCommit {
    store: persistence::Redb,
    writes: usize,
}

impl ComputeStorePort for FailSecondCommit {
    fn load(&self) -> crate::fabric::FabricPortResult<CapacityState> {
        self.store.load()
    }
    fn selected_authority_id(&self) -> &str {
        self.store.selected_authority_id()
    }
    fn compare_and_store(
        &mut self,
        expected: &CapacityState,
        next: &CapacityState,
    ) -> crate::fabric::FabricPortResult<()> {
        self.writes += 1;
        if self.writes == 2 {
            return Err(crate::fabric::FabricPortError::uncertain("simulated interruption after possible backend start"));
        }
        self.store.compare_and_store(expected, next)
    }
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn reopen_after_commit_failure_preserves_intent_and_no_false_success() {
    let (_workspace, root) = store_root("compute-commit-interrupt");
    let store = observed_store(&root);
    let mut service = make_service(FailSecondCommit { store, writes: 0 }, RecordingBackend::default());
    let outcome = service.reserve(&fixture_request("operation-1")).unwrap();
    assert!(matches!(&outcome, ReserveOutcome::Uncertain(issue)
        if issue.phase == Phase::CommitPersistence && issue.has_possible_effect));
    assert_eq!(service.backend.activations, 1);
    drop(service);
    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    assert_eq!(store.load().unwrap().allocations["operation-1"].status, ReservationStatus::Intent);
    let mut reopened = make_service(store, RecordingBackend {
        is_active_observed: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(reopened.reconcile_activation(&owner()).unwrap(), ReserveOutcome::Reserved(_)));
    assert!(matches!(
        reopened.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Existing(_)
    ));
    assert_eq!(reopened.backend.activations, 0);
}

// r[verify molten.compute_capacity.readback]
// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn reopen_after_ack_and_lost_stop_keeps_charge_until_release_evidence() {
    let (_workspace, root) = store_root("compute-ack-stop");
    let store = observed_store(&root);
    let backend = RecordingBackend {
        is_lost_release_ack: true,
        ..RecordingBackend::default()
    };
    let mut service = make_service(store, backend);
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Reserved(_)
    ));
    drop(service);
    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut reopened = make_service(store, RecordingBackend {
        is_lost_release_ack: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(
        reopened.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Existing(_)
    ));
    assert_eq!(reopened.backend.activations, 0);
    let mut stale_owner = owner();
    stale_owner.owner_generation = 0;
    assert!(matches!(
        reopened.release(&stale_owner).unwrap(),
        ReleaseOutcome::Denied(CapacityIssue::WrongOwner)
    ));
    assert_eq!(reopened.backend.releases, 0);
    assert!(matches!(reopened.release(&owner()).unwrap(), ReleaseOutcome::Uncertain(_)));
    assert_eq!(reopened.backend.releases, 1);
    drop(reopened);

    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut after_stop = make_service(store, RecordingBackend {
        is_release_observed: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(after_stop.release(&owner()).unwrap(), ReleaseOutcome::Uncertain(_)));
    assert_eq!(after_stop.backend.releases, 0);
    let readback = after_stop.inspect("host-a", 20).unwrap().unwrap();
    assert_eq!(readback.allocations[0].status, ReservationStatus::Quarantined);
    assert!(readback.allocations[0].is_logically_fenced);
    let record = after_stop.inspect_record("host-a", 20).unwrap().unwrap();
    assert_eq!(canonical::parse_readback(&record.value).unwrap(), readback);
    assert_eq!(record.reference, crate::preserves_rail::canonical_hash(&record.value).unwrap());
    let next = after_stop.reserve(&fixture_request("operation-2")).unwrap();
    assert!(matches!(next, ReserveOutcome::Denied(CapacityIssue::InsufficientCapacity)));
    assert!(matches!(after_stop.reconcile_release(&owner()).unwrap(), ReleaseOutcome::Released(_)));
    assert!(matches!(
        after_stop.reserve(&fixture_request("operation-2")).unwrap(),
        ReserveOutcome::Reserved(_)
    ));
}

