/// Writes `state` as the live head of its branch, as another committed transition would.
fn force_head(store: &LocalWorldPromotionStore, state: &molten_core::world_head::WorldHeadState) {
    let (_, bytes) = crate::world_head::canonical_world_head_state(state).expect("canonical head");
    let write = store.database().begin_write().expect("write");
    {
        let mut heads = write.open_table(crate::world_promotion::store::WORLD_HEADS_TABLE).expect("heads");
        heads.insert(state.branch_id.as_str(), bytes.as_slice()).expect("insert head");
    }
    write.commit().expect("commit head");
}

fn remove_row(store: &LocalWorldPromotionStore, table: redb::TableDefinition<&str, &[u8]>, key: &str) {
    let write = store.database().begin_write().expect("write");
    {
        let mut rows = write.open_table(table).expect("table");
        rows.remove(key).expect("remove row");
    }
    write.commit().expect("commit removal");
}

fn promote(state: &mut TestState, request: &WorldPromotionRequest) -> WorldPromotionOutcome {
    promote_world(request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion")
}

fn promote_with(
    state: &mut TestState,
    request: &WorldPromotionRequest,
    grant: WorldPromotionGrant,
    observed_unix_seconds: u64,
) -> crate::error::Result<WorldPromotionOutcome> {
    promote_world(request, WorldPromotionPorts {
        current: &mut PresentedGrant {
            grant,
            observed_unix_seconds,
        },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
}

fn assert_unpromoted(state: &mut TestState, request: &WorldPromotionRequest) {
    let head = state.store.read_head(&request.branch_id).expect("head").expect("head");
    assert_eq!(head.head, request.expected_head);
    assert_eq!(head.generation, request.expected_generation);
    let plan = plan_world_promotion(request).expect("plan");
    for reservation in &plan.reservations {
        assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), None);
    }
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn deny_retry_or_abandon_grant_cannot_authorize_promotion() {
    let request = promotion_request();
    let plan = plan_world_promotion(&request).expect("plan");
    let reservation_ref = plan.reservations[0].reservation_ref.clone();
    let attempt_ref = WorldReleaseAttemptRef::new(reference("grant-attempt")).expect("attempt");
    for target in [
        WorldPromotionGrantTarget::DenyReservation {
            reservation_ref: reservation_ref.clone(),
        },
        WorldPromotionGrantTarget::PlanRetry {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref.clone(),
        },
        WorldPromotionGrantTarget::AbandonUncertain {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref.clone(),
        },
    ] {
        let mut state = test_state(&request);
        // Same branch, authority, policy and generation as a valid promote grant; only the action differs.
        let mut grant = promote_grant(&plan, true);
        grant.target = target;
        let error = promote_with(&mut state, &request, grant, OBSERVED_UNIX_SECONDS).expect_err("denied");
        assert!(error.to_string().contains("AuthorityActionMismatch"), "{error}");
        assert_unpromoted(&mut state, &request);
    }
    let mut state = test_state(&request);
    let mut other_candidate = promote_grant(&plan, true);
    other_candidate.target = WorldPromotionGrantTarget::Promote {
        candidate_head: WorldCommitRef::new(reference("other-candidate")).expect("candidate"),
    };
    let error = promote_with(&mut state, &request, other_candidate, OBSERVED_UNIX_SECONDS).expect_err("denied");
    assert!(error.to_string().contains("AuthorityTargetMismatch"), "{error}");
    assert_unpromoted(&mut state, &request);
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn expired_or_consumed_promotion_grant_is_denied() {
    let request = promotion_request();
    let plan = plan_world_promotion(&request).expect("plan");
    let mut state = test_state(&request);
    let error =
        promote_with(&mut state, &request, promote_grant(&plan, true), GRANT_EXPIRES_AT).expect_err("expired");
    assert!(error.to_string().contains("AuthorityExpired"), "{error}");
    assert_unpromoted(&mut state, &request);

    // A second branch in the same store, promoted under a grant reusing the first grant's nonce.
    let second = promotion_request_for_branch("release-two");
    force_head(&state.store, &plan_world_promotion(&second).expect("second plan").before);
    promote(&mut state, &request);
    let second_plan = plan_world_promotion(&second).expect("second plan");
    let mut reused = promote_grant(&second_plan, true);
    reused.nonce = promote_grant(&plan, true).nonce;
    let error = promote_with(&mut state, &second, reused, OBSERVED_UNIX_SECONDS).expect_err("consumed");
    assert!(error.to_string().contains("AuthorityNonceConsumed"), "{error}");
    assert_unpromoted(&mut state, &second);
    // The same grant with a fresh nonce promotes.
    promote_with(&mut state, &second, promote_grant(&second_plan, true), OBSERVED_UNIX_SECONDS).expect("fresh grant");
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn operator_grant_is_single_use_and_bound_to_the_live_branch_head() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let reservation = outcome.committed_reservations[0].clone();

    // A later transition advanced the branch: the grant issued at the promoted generation is stale.
    let mut advanced = outcome.plan.after.clone();
    advanced.generation += 1;
    advanced.head = WorldCommitRef::new(reference("later-head")).expect("head");
    force_head(&state.store, &advanced);
    let stale = deny_grant(&request, &reservation.reservation_ref, "stale-deny");
    let error = deny_reservation(DenyInput {
        store: &mut state.store,
        request: &request,
        operator: &stale,
        reservation_ref: &reservation.reservation_ref,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect_err("stale head");
    assert!(error.to_string().contains("AuthorityGenerationMismatch"), "{error}");
    assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), Some(reservation.clone()));

    let mut current = deny_grant(&request, &reservation.reservation_ref, "current-deny");
    current.generation = advanced.generation;
    deny_reservation(DenyInput {
        store: &mut state.store,
        request: &request,
        operator: &current,
        reservation_ref: &reservation.reservation_ref,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("current grant denies");
    // Repeating the consumed grant is denied even though the denial itself is idempotent.
    let error = deny_reservation(DenyInput {
        store: &mut state.store,
        request: &request,
        operator: &current,
        reservation_ref: &reservation.reservation_ref,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect_err("consumed");
    assert!(error.to_string().contains("AuthorityNonceConsumed"), "{error}");
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn operator_mutation_rechecks_the_validated_head_inside_its_write_transaction() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let reservation = outcome.committed_reservations[0].clone();
    let grant = deny_grant(&request, &reservation.reservation_ref, "raced-deny");
    // The grant was decided against the predecessor head, which is no longer live.
    let raced = WorldOperatorMutation {
        grant: grant.clone(),
        validated_head: outcome.plan.before.clone(),
        reservation: Some(denied_reservation(&reservation)),
        attempt: None,
    };
    assert!(state.store.apply_operator_mutation(&raced).is_err());
    assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), Some(reservation.clone()));
    let live = state.store.observe_live(&request.branch_id, &grant.nonce).expect("live");
    assert!(!live.grant_nonce_consumed);
    assert_eq!(live.head.as_ref(), Some(&outcome.plan.after));
}
