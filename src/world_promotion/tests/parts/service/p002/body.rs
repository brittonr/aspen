// r[verify molten.promotion_identity.roles]
// r[verify molten.promotion_identity.restart_binding]
#[test]
fn promotion_identity_roles_survive_reopen_and_replay_the_original_operation() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let first = promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("first promotion");
    assert!(!first.replayed);
    let identity = first.plan.identity.clone();
    drop(state.store);

    let mut restarted = LocalWorldPromotionStore::open(&state.storage).expect("restarted store");
    let stored = restarted.read_promotion_identity(&request.operation_ref).expect("identity read").expect("identity");
    let StoredPromotionIdentity::Current(stored_identity) = stored else {
        panic!("new promotions must carry identity roles");
    };
    assert_eq!(stored_identity, identity);
    assert_eq!(stored_identity.generation, request.expected_generation + 1);

    let replay = promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut restarted,
        receipts: &mut receipts(),
    })
    .expect("retry with the original identity");
    assert!(replay.replayed);
    assert_eq!(replay.plan.identity, identity);
    assert_eq!(restarted.list_reservations().expect("reservations").len(), EXPECTED_RESERVATIONS);
}

// r[verify molten.promotion_identity.request_binding]
#[test]
fn promotion_identity_rejects_incompatible_parameters_after_reopen() {
    let request = promotion_request();
    let mut state = test_state(&request);
    promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("first promotion");
    drop(state.store);
    let mut restarted = LocalWorldPromotionStore::open(&state.storage).expect("restarted store");
    let before = restarted.head_store_mut().read_head(&request.branch_id).expect("head").expect("active head");
    let mut incompatible = request.clone();
    incompatible.candidate_head =
        WorldCommitRef::new(reference("other-candidate")).expect("other candidate");
    let denied = promote_world(&incompatible, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut restarted,
        receipts: &mut receipts(),
    });
    assert!(denied.is_err(), "same logical identity with different parameters must deny");
    let after = restarted.head_store_mut().read_head(&request.branch_id).expect("head").expect("active head");
    assert_eq!(before, after);
    assert_eq!(restarted.list_reservations().expect("reservations").len(), EXPECTED_RESERVATIONS);
}

// r[verify molten.promotion_identity.restart_binding]
#[test]
fn promotion_record_without_an_identity_row_replays_as_retained_uncertainty() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let outcome = promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("first promotion");
    {
        let write = state.store.database().begin_write().expect("write");
        {
            let mut identities =
                write.open_table(crate::world_promotion::store::PROMOTION_IDENTITIES_TABLE).expect("identities");
            identities.remove(request.operation_ref.as_str()).expect("remove identity");
        }
        write.commit().expect("commit removal");
    }
    let replay = promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("legacy replay");
    assert!(replay.replayed);
    assert_eq!(replay.plan.plan_ref, outcome.plan.plan_ref);
    assert!(replay.persistence.shared.quarantine() != transactional_reconciliation_core::QuarantineStatus::Clear);
}

