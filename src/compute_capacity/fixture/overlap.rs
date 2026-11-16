// Authored TT physical-charge overlap fixture.
// r[verify molten.compute_capacity.fixtures]
// r[verify molten.compute_capacity.backing_pools]
// r[verify molten.compute_capacity.durable_reconciliation]
#[test]
fn authored_tt_overlap_uses_one_durable_physical_charge_per_chip() {
    let fixture = export_authored("tt-overlap.ncl");
    let authored = entries(field(&fixture, "offers"))
        .iter()
        .map(offer)
        .find(|offer| offer.host_id == "host-b")
        .expect("authored TT host");
    let owner = &authored.pools[0].authority_id;
    assert!(authored.pools.iter().all(|pool| &pool.authority_id == owner));
    let base = request(field(&entries(field(&fixture, "steps"))[0], "request"));
    let (_workspace, root) = store_root("compute-authored-tt-overlap");
    let mut store = super::persistence::Redb::open(&root, owner).expect("selected TT owner");
    let mut supplied = super::ports::SuppliedOffers::new(vec![authored.clone()]).expect("supplied authored TT offer");
    let offered = supplied.next_offer().unwrap().expect("one authored observation");
    assert!(supplied.next_offer().unwrap().is_none());
    let initial = store.load().unwrap();
    let observed = molten_core::compute_capacity::observe(&initial, offered).expect("core accepts authored TT offer");
    store.compare_and_store(&initial, &observed).expect("canonical offer persistence");
    let state = store.load().expect("canonical offer readback");
    assert_eq!(state.offers["host-b"], authored);
    let tray = request_for_view(&base, &authored, "tt-tray-0-1");
    let mesh = request_for_view(&base, &authored, "tt-mesh-0-1");
    let unsupported = request_for_view(&base, &authored, "tt-mesh-1-2");
    assert_eq!(molten_core::compute_capacity::eligibility(&state, &tray), molten_core::compute_capacity::Eligibility::Eligible);
    assert_eq!(molten_core::compute_capacity::eligibility(&state, &mesh), molten_core::compute_capacity::Eligibility::Eligible);
    assert_eq!(
        molten_core::compute_capacity::eligibility(&state, &unsupported),
        molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::Unsupported)
    );
    assert_eq!(molten_core::compute_capacity::eligibility(&state, &base), molten_core::compute_capacity::Eligibility::Eligible);
    let reserved = molten_core::compute_capacity::reserve(&state, &base);
    assert!(matches!(reserved.decision, molten_core::compute_capacity::ReservationOutcome::Reserved { .. }));
    store.compare_and_store(&state, &reserved.state).expect("canonical MMIO charge persistence");
    drop(store);
    let mut reopened = super::persistence::Redb::open(&root, owner).expect("reopen selected TT owner");
    let charged = reopened.load().expect("canonical MMIO charge readback");
    assert_eq!(charged.allocations["serving-b"].claims, base.claims);
    assert_eq!(
        molten_core::compute_capacity::eligibility(&charged, &tray),
        molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::InsufficientCapacity)
    );
    assert_eq!(
        molten_core::compute_capacity::eligibility(&charged, &mesh),
        molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::InsufficientCapacity)
    );
    assert_eq!(
        molten_core::compute_capacity::eligibility(&charged, &unsupported),
        molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::Unsupported)
    );
    let remote = request_for_view(&base, &authored, "tt-remote-1");
    assert_eq!(molten_core::compute_capacity::eligibility(&charged, &remote), molten_core::compute_capacity::Eligibility::Eligible);
    let second = molten_core::compute_capacity::reserve(&charged, &remote);
    assert!(matches!(second.decision, molten_core::compute_capacity::ReservationOutcome::Reserved { .. }));
    reopened.compare_and_store(&charged, &second.state).expect("independent remote chip charge");
    let final_state = reopened.load().expect("canonical independent charge readback");
    assert_eq!(final_state.allocations.len(), 2);
    assert_eq!(final_state.allocations["attempt-tt-remote-1"].claims[0].pool_id, "tt-chip-1");
    assert_eq!(
        molten_core::compute_capacity::eligibility(&final_state, &mesh),
        molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::InsufficientCapacity)
    );
}
