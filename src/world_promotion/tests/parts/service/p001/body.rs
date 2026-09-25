
#[test]
fn stale_current_authority_leaves_head_and_outbox_unchanged() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let before = state.store.head_store_mut().read_head(&request.branch_id).expect("head read");
    let mut current = Current { is_admitted: false };
    let mut receipt_port = receipts();
    assert!(
        promote_world(&request, WorldPromotionPorts {
            current: &mut current,
            transaction: &mut state.store,
            receipts: &mut receipt_port,
        })
        .is_err()
    );
    let after = state.store.head_store_mut().read_head(&request.branch_id).expect("head read");
    assert_eq!(before, after);
    assert!(state.store.list_reservations().expect("reservations").is_empty());
    assert_eq!(receipt_port.count, 0);
}

fn promotion_request_for_branch(branch: &str) -> WorldPromotionRequest {
    let mut request = promotion_request();
    request.branch_id = molten_core::world_head::WorldBranchId::new(branch).expect("branch");
    request.operation_ref =
        WorldPromotionOperationRef::new(reference(&format!("operation:{branch}"))).expect("operation ref");
    request.expected_head = WorldCommitRef::new(reference(&format!("active:{branch}"))).expect("head");
    request.candidate_head = WorldCommitRef::new(reference(&format!("candidate:{branch}"))).expect("candidate");
    request
}

fn expected_initial_attempt(reservation: &WorldReleaseReservation) -> WorldReleaseAttemptRef {
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-promotion.transaction-attempt.v1");
    for field in [reservation.reservation_ref.as_str(), reservation.operation_ref.as_str()] {
        let length = u64::try_from(field.len()).expect("field length");
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    WorldReleaseAttemptRef::new(format!("blake3:{}", hasher.finalize().to_hex())).expect("attempt ref")
}
