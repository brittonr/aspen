// Backend reconciliation and lifecycle scenarios.
// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn backend_observation_is_required_and_exact() {
    let (_workspace, root) = store_root("compute-backend-observation");
    let mut service = make_service(observed_store(&root), RecordingBackend {
        is_lost_activation_ack: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Uncertain(_)
    ));
    service.backend.is_forged_activation_observation = true;
    assert!(matches!(
        service.reconcile_activation(&owner()).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::InsufficientEvidence)
    ));
    assert_eq!(service.store.load().unwrap().allocations["operation-1"].status, ReservationStatus::Intent);
    service.backend.is_forged_activation_observation = false;
    assert!(matches!(
        service.reconcile_activation(&owner()).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::InsufficientEvidence)
    ));
    service.backend.is_active_observed = true;
    assert!(matches!(service.reconcile_activation(&owner()).unwrap(), ReserveOutcome::Reserved(_)));
    assert!(matches!(service.reconcile_activation(&owner()).unwrap(), ReserveOutcome::Existing(_)));
    assert_eq!(service.backend.activations, 1);
    drop(service);
    let reopened = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut service = make_service(reopened, RecordingBackend {
        is_active_observed: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(service.reconcile_activation(&owner()).unwrap(), ReserveOutcome::Existing(_)));
    assert_eq!(service.backend.activations, 0);

    service.backend.is_lost_release_ack = true;
    assert!(matches!(service.release(&owner()).unwrap(), ReleaseOutcome::Uncertain(_)));
    service.backend.is_forged_release_observation = true;
    assert!(matches!(service.reconcile_release(&owner()).unwrap(), ReleaseOutcome::Uncertain(_)));
    assert!(service.store.load().unwrap().allocations.contains_key("operation-1"));
    service.backend.is_forged_release_observation = false;
    service.backend.is_release_observed = true;
    assert!(matches!(service.reconcile_release(&owner()).unwrap(), ReleaseOutcome::Released(_)));
    assert!(matches!(service.reconcile_release(&owner()).unwrap(), ReleaseOutcome::AlreadyReleased(_)));
    assert_eq!(service.backend.releases, 1);
    drop(service);
    let reopened = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut service = make_service(reopened, RecordingBackend::default());
    assert!(matches!(service.reconcile_release(&owner()).unwrap(), ReleaseOutcome::AlreadyReleased(_)));
    assert_eq!(service.backend.releases, 0);
    assert_eq!(service.store.load().unwrap().retired_allocations.len(), 1);
}

// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn lifecycle_persists_quarantine_without_replaying_stop() {
    let (_workspace, root) = store_root("compute-lifecycle-events");
    let mut service = make_service(observed_store(&root), RecordingBackend::default());
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Reserved(_)
    ));
    let pending = service.event(&owner(), LifecycleEvent::Cancel).unwrap().unwrap();
    assert_eq!(pending.effects, vec![EffectIntent::RequestBackendRelease("operation-1".into())]);
    assert_eq!(pending.allocation.status, ReservationStatus::Quarantined);
    assert_eq!(service.backend.releases, 0);
    let replay = service.event(&owner(), LifecycleEvent::LostStopAcknowledgement).unwrap().unwrap();
    assert_eq!(replay.effects, vec![EffectIntent::Reconcile("operation-1".into())]);
    assert_eq!(service.backend.releases, 0);
    assert!(matches!(service.release(&owner()).unwrap(), ReleaseOutcome::Uncertain(_)));
    service.backend.is_release_observed = true;
    assert!(matches!(service.reconcile_release(&owner()).unwrap(), ReleaseOutcome::Released(_)));
}

// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn definite_no_effect_retires_without_permanent_charge() {
    let (_workspace, root) = store_root("compute-definite-no-effect");
    let mut service = make_service(observed_store(&root), RecordingBackend {
        is_definite_activation_refusal: true,
        ..RecordingBackend::default()
    });
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::PortDenied(crate::fabric::FabricPortError::CapabilityUnavailable { .. })
    ));
    let state = service.store.load().unwrap();
    assert!(state.allocations.is_empty());
    assert_eq!(state.retired_allocations.len(), 1);
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::WrongOwner)
    ));
    service.backend.is_definite_activation_refusal = false;
    assert!(matches!(
        service.reserve(&fixture_request("operation-2")).unwrap(),
        ReserveOutcome::Reserved(_)
    ));
}

struct RejectFirstCommit {
    store: persistence::Redb,
    error: crate::fabric::FabricPortError,
}

impl ComputeStorePort for RejectFirstCommit {
    fn load(&self) -> crate::fabric::FabricPortResult<CapacityState> {
        self.store.load()
    }

    fn selected_authority_id(&self) -> &str {
        self.store.selected_authority_id()
    }

    fn compare_and_store(
        &mut self,
        _expected: &CapacityState,
        _next: &CapacityState,
    ) -> crate::fabric::FabricPortResult<()> {
        Err(self.error.clone())
    }
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn pre_effect_storage_conflict_is_not_uncertain_effect() {
    let (_workspace, root) = store_root("compute-cas-conflict-class");
    let store = observed_store(&root);
    let mut service = make_service(
        RejectFirstCommit {
            store,
            error: crate::fabric::FabricPortError::storage("stale compare"),
        },
        RecordingBackend::default(),
    );
    let outcome = service.reserve(&fixture_request("operation-1")).unwrap();
    assert!(matches!(outcome, ReserveOutcome::PortDenied(crate::fabric::FabricPortError::StorageFailure { .. })));
    assert_eq!(service.backend.activations, 0);
    assert!(service.store.load().unwrap().allocations.is_empty());
    service.store.error = crate::fabric::FabricPortError::uncertain("commit acknowledgement missing");
    let outcome = service.reserve(&fixture_request("operation-1")).unwrap();
    assert!(matches!(
        outcome,
        ReserveOutcome::Uncertain(UncertainOutcome {
            phase: Phase::IntentPersistence,
            has_possible_effect: false,
            port_error: Some(crate::fabric::FabricPortError::UncertainExternalOutcome { .. }),
            ..
        })
    ));
    assert_eq!(service.backend.activations, 0);
}

// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn all_lifecycle_events_keep_durable_charge() {
    for (index, event, is_release_request) in [
        (0, LifecycleEvent::Drain, true),
        (1, LifecycleEvent::Cancel, true),
        (2, LifecycleEvent::Timeout, false),
        (3, LifecycleEvent::LostStopAcknowledgement, false),
        (4, LifecycleEvent::BackendDisappeared, false),
    ] {
        let (_workspace, root) = store_root(&format!("compute-event-{index}"));
        let mut service = make_service(observed_store(&root), RecordingBackend::default());
        assert!(matches!(
            service.reserve(&fixture_request("operation-1")).unwrap(),
            ReserveOutcome::Reserved(_)
        ));
        let outcome = service.event(&owner(), event).unwrap().unwrap();
        let expected = if is_release_request {
            EffectIntent::RequestBackendRelease("operation-1".into())
        } else {
            EffectIntent::Reconcile("operation-1".into())
        };
        assert_eq!(outcome.effects, vec![expected]);
        assert_eq!(outcome.allocation.status, ReservationStatus::Quarantined);
        assert_eq!(service.backend.releases, 0);
        drop(service);
        let reopened = persistence::Redb::open(&root, "owner-a").unwrap();
        assert_eq!(reopened.load().unwrap().allocations["operation-1"].status, ReservationStatus::Quarantined);
    }
}
