// Admission, authority, supplied observations and store CAS scenarios.
// r[verify molten.compute_capacity.offers]
#[test]
fn supplied_observations_are_bounded_and_not_probes() {
    assert!(SuppliedOffers::new(vec![fixture_offer(); molten_core::compute_capacity::MAX_ITEMS + 1]).is_err());
    let (_workspace, root) = store_root("compute-supplied-offer");
    let store = persistence::Redb::open(&root, "owner-a").unwrap();
    let mut service = Service::new(
        store,
        SuppliedOffers::new(vec![fixture_offer()]).unwrap(),
        RecordingBackend::default(),
        TestAdmission::default(),
        TestClock(20),
    );
    assert!(service.inspect("host-a", 20).unwrap().is_none());
    assert!(service.observe().unwrap().is_some());
    assert!(service.observe().unwrap().is_none());
    assert!(service.inspect("host-a", 20).unwrap().unwrap().is_fresh);
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn store_cas_and_shell_denials_precede_backend() {
    let (_workspace, root) = store_root("compute-cas-denials");
    let mut store = observed_store(&root);
    let stale = CapacityState::default();
    assert!(store.compare_and_store(&stale, &stale).is_err(), "stale snapshots cannot overwrite offers");
    let current = store.load().unwrap();
    assert!(store.compare_and_store(&current, &current).is_ok());
    let mut service = make_service(store, RecordingBackend::default());
    let revoked = fixture_request("operation-1");
    service.admission.grant.is_revoked = true;
    assert!(matches!(
        service.reserve(&revoked).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::RevokedGrant)
    ));
    service.admission.grant.is_revoked = false;
    let expired = fixture_request("operation-1");
    service.time.0 = 100;
    assert!(matches!(service.reserve(&expired).unwrap(), ReserveOutcome::Denied(CapacityIssue::Stale)));
    assert_eq!(service.backend.activations, 0);
    assert!(service.store.load().unwrap().allocations.is_empty());
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn serving_release_waits_for_dependent_execution() {
    let (_workspace, root) = store_root("compute-serving-dependent");
    let store = observed_store(&root);
    let mut service = make_service(store, RecordingBackend::default());
    let mut serving = fixture_request("operation-1");
    serving.kind = AllocationKind::ServingDeployment;
    assert!(matches!(service.reserve(&serving).unwrap(), ReserveOutcome::Reserved(_)));
    let mut agent = fixture_request("operation-2");
    agent.claims[0].quantity = 20;
    agent.serving_allocation_id = Some("operation-1".into());
    assert!(matches!(service.reserve(&agent).unwrap(), ReserveOutcome::Reserved(_)));
    assert!(matches!(
        service.release(&owner()).unwrap(),
        ReleaseOutcome::Denied(CapacityIssue::InsufficientEvidence)
    ));
    assert_eq!(service.backend.releases, 0);
    let agent_owner = OwnerInput {
        allocation_id: "operation-2".into(),
        consumer_id: "worker".into(),
        owner_generation: 1,
    };
    assert!(matches!(service.release(&agent_owner).unwrap(), ReleaseOutcome::Released(_)));
    assert!(matches!(service.release(&owner()).unwrap(), ReleaseOutcome::Released(_)));
    assert_eq!(service.backend.releases, 2);
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn admitted_grant_clock_label_and_selected_owner_precede_backend() {
    let (_workspace, root) = store_root("compute-admitted-facts");
    let store = observed_store(&root);
    drop(store);
    assert!(matches!(
        persistence::Redb::open(&root, "other-owner"),
        Err(crate::fabric::FabricPortError::CapabilityUnavailable { .. })
    ));
    let mut store = persistence::Redb::open(&root, "owner-a").unwrap();
    let current = store.load().unwrap();
    let mut weaker = current.clone();
    weaker.offers.get_mut("host-a").unwrap().label_authority = molten_core::fabric_membership::LabelAuthority::Measured;
    store.compare_and_store(&current, &weaker).unwrap();
    let mut service = make_service(store, RecordingBackend::default());
    let mut spoofed = fixture_request("operation-1");
    spoofed.now = 1;
    spoofed.minimum_label_authority = molten_core::fabric_membership::LabelAuthority::Advisory;
    assert_eq!(service.eligibility(&spoofed).unwrap(), Eligibility::Rejected(CapacityIssue::WeakAuthority));
    assert!(matches!(
        service.reserve(&spoofed).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::WeakAuthority)
    ));
    assert_eq!(service.backend.activations, 0);
    let current = service.store.load().unwrap();
    let mut strong = current.clone();
    strong.offers.get_mut("host-a").unwrap().label_authority =
        molten_core::fabric_membership::LabelAuthority::Authoritative;
    service.store.compare_and_store(&current, &strong).unwrap();
    service.admission.grant.is_revoked = true;
    assert!(matches!(
        service.reserve(&spoofed).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::RevokedGrant)
    ));
    service.admission.grant.is_revoked = false;
    spoofed.grant_ref = format!("blake3:{}", "b".repeat(64));
    assert!(matches!(
        service.reserve(&spoofed).unwrap(),
        ReserveOutcome::Denied(CapacityIssue::InvalidGrant)
    ));
    spoofed.grant_ref = reference();
    service.time.0 = 100;
    assert!(matches!(service.reserve(&spoofed).unwrap(), ReserveOutcome::Denied(CapacityIssue::Stale)));
    assert_eq!(service.backend.activations, 0);
    assert!(service.store.load().unwrap().allocations.is_empty());

    let (_other_workspace, other_root) = store_root("compute-wrong-selected-owner");
    let mut other = persistence::Redb::open(&other_root, "other-owner").unwrap();
    let empty = other.load().unwrap();
    other
        .compare_and_store(&empty, &molten_core::compute_capacity::observe(&empty, fixture_offer()).unwrap())
        .unwrap();
    let mut service = make_service(other, RecordingBackend::default());
    assert!(matches!(
        service.reserve(&fixture_request("operation-1")).unwrap(),
        ReserveOutcome::PortDenied(crate::fabric::FabricPortError::CapabilityUnavailable { .. })
    ));
    assert_eq!(service.backend.activations, 0);
    assert!(service.store.load().unwrap().allocations.is_empty());
}

