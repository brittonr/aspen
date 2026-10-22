use super::*;

const OBJECT_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OBJECT_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OBJECT_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const PEER_A: &str = "peer-a";
const PEER_B: &str = "peer-b";
const PEER_C: &str = "peer-c";
const ZONE_A: &str = "zone-a";
const ZONE_B: &str = "zone-b";
const ZONE_C: &str = "zone-c";
const MAX_TRANSFERS: usize = 8;

fn fact(peer_id: &str, fault_domain: &str, object_ref: &str, intact: bool) -> ReplicaObjectFact {
    ReplicaObjectFact {
        peer_id: peer_id.to_string(),
        fault_domain: fault_domain.to_string(),
        object_ref: object_ref.to_string(),
        present: intact,
        identity_verified: intact,
    }
}

fn input(required: &[&str], inventory: Vec<ReplicaObjectFact>) -> ClosureRepairInput {
    ClosureRepairInput {
        required_objects: required.iter().map(|object| (*object).to_string()).collect(),
        inventory,
        max_transfers: MAX_TRANSFERS,
    }
}

// r[verify molten.closure_repair.per_object]
// r[verify molten.closure_repair.donor_eligibility]
#[test]
fn several_partial_donors_repair_the_whole_closure() {
    let plan = plan_closure_repair(&input(&[OBJECT_A, OBJECT_B, OBJECT_C], vec![
        fact(PEER_A, ZONE_A, OBJECT_A, true),
        fact(PEER_A, ZONE_A, OBJECT_B, false),
        fact(PEER_B, ZONE_B, OBJECT_B, true),
        fact(PEER_B, ZONE_B, OBJECT_C, false),
        fact(PEER_C, ZONE_C, OBJECT_C, true),
    ]))
    .expect("the closure repair plans");

    assert!(plan.completeness.is_complete());
    assert_eq!(plan.transfers.len(), 3);
    assert_eq!(plan.repaired_objects, vec![OBJECT_A, OBJECT_B, OBJECT_C]);
    assert!(plan.unrecoverable_objects.is_empty());
    let donors = plan
        .transfers
        .iter()
        .map(|transfer| (transfer.object_ref.as_str(), transfer.donor_peer_id.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(donors, vec![(OBJECT_A, PEER_A), (OBJECT_B, PEER_B), (OBJECT_C, PEER_C)]);
    assert!(plan.plan_ref.starts_with("blake3:"));
}

// r[verify molten.closure_repair.incomplete_outcome]
#[test]
fn an_object_without_an_intact_copy_yields_a_typed_incomplete_repair() {
    let plan = plan_closure_repair(&input(&[OBJECT_A, OBJECT_B], vec![
        fact(PEER_A, ZONE_A, OBJECT_A, true),
        fact(PEER_B, ZONE_B, OBJECT_B, false),
    ]))
    .expect("the closure repair plans");

    assert!(!plan.completeness.is_complete());
    assert_eq!(plan.completeness.as_str(), "incomplete");
    assert_eq!(plan.unrecoverable_objects, vec![OBJECT_B]);
    assert_eq!(plan.repaired_objects, vec![OBJECT_A]);
    assert_eq!(plan.transfers.len(), 1, "the repair keeps the progress it made");
}

// r[verify molten.closure_repair.per_object]
#[test]
fn an_assembly_identity_mismatch_rejects_only_that_object() {
    assert!(verify_assembled_object(OBJECT_A, OBJECT_A).is_ok());
    assert!(verify_assembled_object(OBJECT_A, OBJECT_B).is_err());
    assert!(verify_assembled_object(OBJECT_B, OBJECT_B).is_ok());
}

// r[verify molten.closure_repair.validation]
#[test]
fn a_flapping_donor_is_refused_at_transfer_time() {
    let plan = plan_closure_repair(&input(&[OBJECT_A], vec![fact(PEER_A, ZONE_A, OBJECT_A, true)]))
        .expect("the closure repair plans");
    let transfer = plan.transfers.first().expect("the planned transfer");

    let intact = fact(PEER_A, ZONE_A, OBJECT_A, true);
    assert!(verify_transfer_donor(transfer, &intact).is_ok());

    let flapped = fact(PEER_A, ZONE_A, OBJECT_A, false);
    assert!(verify_transfer_donor(transfer, &flapped).is_err());

    let other_peer = fact(PEER_B, ZONE_B, OBJECT_A, true);
    assert!(verify_transfer_donor(transfer, &other_peer).is_err());
}

// r[verify molten.closure_repair.validation]
#[test]
fn unbounded_inventories_and_malformed_facts_are_refused() {
    let mut unbounded = input(&[OBJECT_A], Vec::new());
    unbounded.inventory = (0..MAX_REPAIR_INVENTORY + 1).map(|_| fact(PEER_A, ZONE_A, OBJECT_A, true)).collect();
    assert!(plan_closure_repair(&unbounded).is_err());

    let malformed = input(&[OBJECT_A], vec![ReplicaObjectFact {
        peer_id: String::new(),
        fault_domain: ZONE_A.to_string(),
        object_ref: OBJECT_A.to_string(),
        present: true,
        identity_verified: true,
    }]);
    assert!(plan_closure_repair(&malformed).is_err());

    let duplicated = input(&[OBJECT_A], vec![
        fact(PEER_A, ZONE_A, OBJECT_A, true),
        fact(PEER_A, ZONE_A, OBJECT_A, true),
    ]);
    assert!(plan_closure_repair(&duplicated).is_err());
}
