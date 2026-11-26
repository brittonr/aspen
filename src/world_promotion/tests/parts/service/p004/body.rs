/// Promotes `request` and drives its release reservation through a real dispatch shape to an
/// uncertain attempt: claimed, then reservation and attempt `attempting` together, then both
/// `uncertain` together.
fn promoted_with_uncertain_attempt(
    request: &WorldPromotionRequest,
) -> (TestState, WorldReleaseReservation, WorldAttemptRecord) {
    let mut state = test_state(request);
    promote_world(request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion");
    let committed = state.store.list_reservations().expect("reservations").pop().expect("reservation");
    let mut reservation = state.store.claim_reservation(&committed.reservation_ref).expect("claim").expect("claimed");
    let mut attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: WorldReleaseAttemptRef::new(reference("operator-attempt")).expect("attempt"),
        state: WorldReleaseState::Attempting,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    reservation.state = WorldReleaseState::Attempting;
    state.store.record_attempt_transition(&reservation, &attempt).expect("attempting");
    reservation.state = WorldReleaseState::Uncertain;
    attempt.state = WorldReleaseState::Uncertain;
    state.store.record_attempt_transition(&reservation, &attempt).expect("uncertain");
    (state, reservation, attempt)
}

type GrantEdit = fn(&mut WorldPromotionGrant);

/// Edits that each make an otherwise current operator grant fail with exactly the named issue.
fn denying_grant_edits() -> [(GrantEdit, &'static str); 5] {
    [
        (|grant| grant.generation = CURRENT_GENERATION, "AuthorityGenerationMismatch"),
        (|grant| grant.admitted = false, "AuthorityDenied"),
        (
            |grant| {
                grant.authority_ref =
                    WorldPromotionAuthorityRef::new(reference("foreign-authority")).expect("authority");
            },
            "AuthorityDenied",
        ),
        (|grant| grant.expires_at = OBSERVED_UNIX_SECONDS, "AuthorityExpired"),
        (
            |grant| {
                grant.target = WorldPromotionGrantTarget::Promote {
                    candidate_head: promotion_request().candidate_head,
                };
            },
            "AuthorityActionMismatch",
        ),
    ]
}

fn edited(mut grant: WorldPromotionGrant, edit: GrantEdit) -> WorldPromotionGrant {
    edit(&mut grant);
    grant
}

fn assert_authority_denied<T: std::fmt::Debug>(result: crate::error::Result<T>, operation: &str, issue: &str) {
    let error = result.expect_err("operator authority denied");
    assert!(error.to_string().contains(&format!("{operation} authority denied: [{issue}]")), "{error}");
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn stale_revoked_or_foreign_operator_authority_cannot_deny_retry_or_abandon() {
    let request = promotion_request();
    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);
    // Deny applies to a committed reservation, so its authority is exercised on one.
    let mut committed_state = test_state(&request);
    let committed = promote(&mut committed_state, &request).committed_reservations[0].clone();
    let deny = |store: &mut LocalWorldPromotionStore, operator: &WorldPromotionGrant| {
        deny_reservation(DenyInput {
            store,
            request: &request,
            operator,
            reservation_ref: &committed.reservation_ref,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        })
    };
    for (edit, issue) in denying_grant_edits() {
        let retry = plan_retry_for_reservation(RetryPlanInput {
            store: &mut state.store,
            request: &request,
            operator: &edited(retry_grant(&request, &attempt, "retry"), edit),
            reservation_ref: &reservation.reservation_ref,
            attempt_ref: &attempt.attempt_ref,
            next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
            is_duplicate_risk_acknowledged: true,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        });
        assert_authority_denied(retry, "retry", issue);
        let abandon = abandon_uncertain_for_attempt(AbandonInput {
            store: &mut state.store,
            request: &request,
            operator: &edited(abandon_grant(&request, &attempt, "abandon"), edit),
            attempt_ref: &attempt.attempt_ref,
            is_unknown_outcome_acknowledged: true,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        });
        assert_authority_denied(abandon, "abandon", issue);
        let denied = deny(&mut committed_state.store, &edited(deny_grant(&request, &committed.reservation_ref, "deny"), edit));
        assert_authority_denied(denied, "deny", issue);
    }
    assert_eq!(state.store.read_attempt(&attempt.attempt_ref).expect("read"), Some(attempt));
    assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), Some(reservation));
    assert_eq!(committed_state.store.read_reservation(&committed.reservation_ref).expect("read"), Some(committed.clone()));
    // The unedited grant is current: the same deny succeeds.
    deny(&mut committed_state.store, &deny_grant(&request, &committed.reservation_ref, "deny")).expect("current grant denies");
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.promotion_identity.restart_binding]
#[test]
fn operator_requests_rebuild_only_the_committed_original_plan() {
    let request = promotion_request();
    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);

    // A request naming another candidate or operation is not the committed plan.
    let mut other_candidate = request.clone();
    other_candidate.candidate_head = WorldCommitRef::new(reference("other-candidate")).expect("candidate");
    let mut other_operation = request.clone();
    other_operation.operation_ref =
        WorldPromotionOperationRef::new(reference("replacement-operation")).expect("operation");
    for replacement in [&other_candidate, &other_operation] {
        assert!(
            plan_retry_for_reservation(RetryPlanInput {
                store: &mut state.store,
                request: replacement,
                operator: &retry_grant(&request, &attempt, "retry-replacement"),
                reservation_ref: &reservation.reservation_ref,
                attempt_ref: &attempt.attempt_ref,
                next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
                is_duplicate_risk_acknowledged: true,
                observed_unix_seconds: OBSERVED_UNIX_SECONDS,
            })
            .is_err()
        );
        assert!(
            abandon_uncertain_for_attempt(AbandonInput {
                store: &mut state.store,
                request: replacement,
                operator: &abandon_grant(&request, &attempt, "abandon-replacement"),
                attempt_ref: &attempt.attempt_ref,
                is_unknown_outcome_acknowledged: true,
                observed_unix_seconds: OBSERVED_UNIX_SECONDS,
            })
            .is_err()
        );
    }
    let stored_attempt = state.store.read_attempt(&attempt.attempt_ref).expect("read").expect("attempt");
    assert_eq!(stored_attempt.state, WorldReleaseState::Uncertain);

    // The document's authority and closure booleans are planning inputs, not authority.
    let mut unadmitted_document = request.clone();
    unadmitted_document.authority.admitted = false;
    unadmitted_document.intent_closure_complete = false;
    let retry = plan_retry_for_reservation(RetryPlanInput {
        store: &mut state.store,
        request: &unadmitted_document,
        operator: &retry_grant(&request, &attempt, "retry-unadmitted-document"),
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &attempt.attempt_ref,
        next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
        is_duplicate_risk_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("retry plan");
    assert!(retry.same_logical_release);
    assert!(retry.duplicate_risk_acknowledged);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn current_operator_cannot_deny_inflight_or_uncertain_effects() {
    let request = promotion_request();
    let paths: &[&[WorldReleaseState]] = &[
        &[],
        &[WorldReleaseState::Attempting],
        &[WorldReleaseState::Attempting, WorldReleaseState::Observed],
        &[WorldReleaseState::Attempting, WorldReleaseState::Observed, WorldReleaseState::Acknowledged],
        &[WorldReleaseState::Attempting, WorldReleaseState::Conflict],
    ];
    for path in paths {
        let mut state = test_state(&request);
        promote_world(&request, WorldPromotionPorts {
            current: &mut Current { is_admitted: true },
            transaction: &mut state.store,
            receipts: &mut receipts(),
        })
        .expect("promotion");
        let committed = state.store.list_reservations().expect("reservations").pop().expect("reservation");
        let mut existing =
            state.store.claim_reservation(&committed.reservation_ref).expect("claim").expect("committed reservation");
        for next in *path {
            existing.state = *next;
            state.store.update_reservation(&existing).expect("valid effect state transition");
        }
        assert!(
            deny_reservation(DenyInput {
                store: &mut state.store,
                request: &request,
                operator: &deny_grant(&request, &existing.reservation_ref, "deny-inflight"),
                reservation_ref: &existing.reservation_ref,
                observed_unix_seconds: OBSERVED_UNIX_SECONDS,
            })
            .is_err()
        );
        assert_eq!(state.store.read_reservation(&existing.reservation_ref).expect("read"), Some(existing));
    }
    for terminal in [WorldReleaseState::Uncertain, WorldReleaseState::Reconciled, WorldReleaseState::Abandoned] {
        let (mut state, mut reservation, attempt) = promoted_with_uncertain_attempt(&request);
        if terminal != WorldReleaseState::Uncertain {
            reservation.state = terminal;
            state.store.update_reservation(&reservation).expect("settled reservation");
        }
        assert!(
            deny_reservation(DenyInput {
                store: &mut state.store,
                request: &request,
                operator: &deny_grant(&request, &reservation.reservation_ref, "deny-uncertain"),
                reservation_ref: &reservation.reservation_ref,
                observed_unix_seconds: OBSERVED_UNIX_SECONDS,
            })
            .is_err()
        );
        assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), Some(reservation));
        assert_eq!(state.store.read_attempt(&attempt.attempt_ref).expect("read"), Some(attempt));
    }
}
