    #[test]
    fn resigned_or_expired_election_admits_a_successor_and_non_holder_resignation_is_denied() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        let key = "election:resign";
        set_clock(&mut runtime, 10);
        let first = apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, key, "leader-a", 1, None),
            COORDINATION_DECISION_PASS,
        );
        let first_token = first.token.expect("leader token").token;
        assert_eq!(runtime.state.elections.get(key).expect("leader").lease_expires_at, 10 + DEFAULT_COORDINATION_LEASE_TICKS);
        apply_generated_denial(&mut runtime, &request(SERVICE_ELECTION, OP_ELECT, key, "leader-b", 2, None), "already led by leader-a");
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_RESIGN, key, "leader-b", 3, Some(token_payload(first_token))),
            "resignation by non-leader leader-b",
        );
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_RESIGN, key, "leader-a", 4, Some(token_payload(first_token + 1))),
            "stale fencing token",
        );
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_RESIGN, key, "leader-a", 5, Some(token_payload(first_token))),
            COORDINATION_DECISION_PASS,
        );
        assert!(!runtime.state.elections.contains_key(key));
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_RESIGN, key, "leader-a", 6, Some(token_payload(first_token))),
            "has no leader",
        );

        set_clock(&mut runtime, 20);
        let candidacy = record("candidate", vec![string("leader-b"), lease(10)]);
        let successor = apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, key, "operator", 7, Some(candidacy)),
            COORDINATION_DECISION_PASS,
        );
        let successor_token = successor.token.expect("successor token");
        assert!(successor_token.token > first_token);
        let leader = runtime.state.elections.get(key).expect("leader");
        assert_eq!(leader.leader, "leader-b");
        assert_eq!(leader.lease_expires_at, 30);
        assert_eq!(leader.token_ref, successor_token.token_ref);

        // A crashed leader that never resigns is stepped down once the clock passes its term; neither a
        // presented time nor unrelated commits do it.
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_EXPIRE, key, "leader-c", 8, Some(presented_time(30))),
            "takes no payload",
        );
        commit_unrelated(&mut runtime, 1_000, 100);
        set_clock(&mut runtime, 29);
        apply_generated_denial(&mut runtime, &request(SERVICE_ELECTION, OP_EXPIRE, key, "leader-c", 9, None), "is unexpired");
        set_clock(&mut runtime, 30);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_EXPIRE, key, "leader-c", 10, None),
            COORDINATION_DECISION_PASS,
        );
        let third = apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, key, "leader-c", 11, Some(lease(10))),
            COORDINATION_DECISION_PASS,
        );
        assert!(third.token.expect("third token").token > successor_token.token);
    }

    #[test]
    fn mismatched_basis_or_regressing_clock_sample_is_denied() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        set_clock(&mut runtime, 50);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:basis", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        set_clock(&mut runtime, 49);
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:regress", "holder", 2, None),
            "regresses below committed time 50",
        );
        let other_basis = fixture_ref("other-coordination-clock");
        runtime.clock = CoordinationClock::new(
            other_basis.clone(),
            Box::new(FixtureClockAdapter {
                profile_ref: other_basis,
                ticks: 60,
            }),
        )
        .expect("other clock");
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:other-basis", "holder", 3, None),
            "does not match committed time basis",
        );
        set_clock(&mut runtime, 60);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:other-basis", "holder", 4, None),
            COORDINATION_DECISION_PASS,
        );
        assert_eq!(runtime.state.time_ticks, 60);
    }

    #[test]
    fn transient_clock_failure_is_unrecorded_and_a_retry_with_the_same_operation_id_commits() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        runtime.clock =
            CoordinationClock::new(coordination_fixture_time_basis_ref(), Box::new(FailingClockAdapter)).expect("failing clock");
        let acquire = request(SERVICE_LOCK, OP_ACQUIRE, "resource:retry", "holder", 1, None);
        let operation_id = parse_coordination_request(&acquire).expect("request").operation_id_ref;
        let before_state = runtime.state.clone();
        let before_receipts = runtime.receipts.len();
        let error = apply_coordination_request(&mut runtime, &acquire).expect_err("clock failure is an error");
        assert_eq!(error.class(), Some(crate::error::FailureClass::CapabilityUnavailable), "{error}");
        assert!(error.to_string().contains("coordination test clock unavailable"), "{error}");
        assert_eq!(runtime.state, before_state);
        assert_eq!(runtime.receipts.len(), before_receipts);
        assert!(!runtime.applied_operations.contains_key(&operation_id));

        set_clock(&mut runtime, 10);
        let retried = apply_generated_request(&mut runtime, &acquire, COORDINATION_DECISION_PASS);
        assert_eq!(retried.receipt.transition_kind, TRANSITION_KIND_ADVANCE);
        assert!(retried.token.is_some());
        assert_eq!(runtime.state.time_ticks, 10);
    }

    /// Both bases' ticks at one modelled node-monotonic instant, for clocks whose origins sit on that source.
    fn shared_reading(instant: u64, retiring_origin: u64, next_origin: u64) -> SharedClockReading {
        SharedClockReading {
            retiring_ticks: instant - retiring_origin,
            next_ticks: instant - next_origin,
        }
    }

    /// Deterministic re-anchoring through the transition commit core with an injected instant source. Fixture
    /// clocks stand in for live clocks whose origins sit on one modelled node monotonic source. The public
    /// entry point's live-only admission is covered separately.
    #[test]
    fn clock_transition_translates_through_one_shared_reading_and_never_extends_leases() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        let (auth, _resources, _policies) = refs();
        let fabricated_authority = vec![fixture_ref("fabricated-clock-authority")];
        // The fixture basis counts from origin 0 on the modelled source.
        set_clock(&mut runtime, 100);
        let expiring = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:expiring", "crashed", 1, Some(lease(50))),
            COORDINATION_DECISION_PASS,
        );
        let expiring_token = expiring.token.expect("expiring token");
        set_clock(&mut runtime, 120);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:held", "holder", 2, Some(lease(200))),
            COORDINATION_DECISION_PASS,
        );
        set_clock(&mut runtime, 145);
        commit_unrelated(&mut runtime, 1_000, 1);
        let fencing_before = runtime.state.next_fencing_token;
        let before_state = runtime.state.clone();

        // Every check runs before the single reading: a refused transition never reads the source.
        let read = std::cell::Cell::new(false);
        let restarted = fixture_ref("restarted-node-monotonic-clock");
        let restarted_clock = |ticks| CoordinationClock::fixture_on_basis(restarted.clone(), ticks).expect("restarted clock");
        let reading_spy = |_: &CoordinationClock, _: &CoordinationState, _: &CoordinationClock| -> Result<SharedClockReading> {
            read.set(true);
            Ok(shared_reading(146, 0, 140))
        };
        assert!(commit_clock_transition(&mut runtime, CoordinationClock::fixture(146), &auth, reading_spy).is_err());
        assert!(commit_clock_transition(&mut runtime, restarted_clock(6), &[], reading_spy).is_err());
        assert!(!read.get());
        let regressing = commit_clock_transition(&mut runtime, restarted_clock(6), &auth, |_, _, _| Ok(shared_reading(144, 0, 140)));
        assert!(regressing.expect_err("regressing reading").to_string().contains("regresses below committed time"));
        assert_eq!(runtime.state, before_state);

        // Time passes while the transition validates: the old lease expired at 150 and the shared reading is
        // taken at 155. The new basis was opened at 150, so the same instant reads 5 on it.
        let transition = commit_clock_transition(&mut runtime, restarted_clock(5), &auth, |_, _, _| Ok(shared_reading(155, 0, 150)))
            .expect("clock transition");
        assert_eq!(transition.from, CoordinationTimeSample { time_basis_ref: coordination_fixture_time_basis_ref(), ticks: 155 });
        assert_eq!(transition.to, CoordinationTimeSample { time_basis_ref: restarted.clone(), ticks: 5 });
        assert_eq!(runtime.state.time_basis_ref, restarted);
        assert_eq!(runtime.state.time_ticks, 5);
        assert_eq!(transition.after_state_ref, state_ref(&runtime));
        assert_eq!(registry_target_ref(&runtime, "clock", "time-basis"), Some(transition.transition_ref.clone()));
        assert_eq!(runtime.state.next_fencing_token, fencing_before);
        let expired = runtime.state.locks.get("resource:expiring").expect("expired lock still recorded");
        assert_eq!(expired.token_ref, expiring_token.token_ref);
        assert_eq!(expired.lease_expires_at, 5, "an already-expired lease stays expired on the new basis");
        assert_eq!(runtime.state.locks.get("resource:held").expect("held lock").lease_expires_at, 5 + (320 - 155));

        recreate_clock_three_times_without_extending_leases(&mut runtime, &fabricated_authority);
        expired_lease_is_expirable_on_the_final_basis(&mut runtime, &expiring_token);
    }

    /// Back-to-back recreations with a fabricated authority ref: 7 ticks pass during each validation and
    /// each new basis is opened 2 ticks before its reading. The remaining TTL shrinks by exactly the elapsed
    /// time and never grows.
    fn recreate_clock_three_times_without_extending_leases(runtime: &mut CoordinationRuntime, fabricated_authority: &[String]) {
        let mut last_instant = 155_u64;
        let mut retiring_origin = 150_u64;
        for round in 1..=3_u64 {
            let before_remaining = runtime.state.locks.get("resource:held").expect("held lock").lease_expires_at - runtime.state.time_ticks;
            let instant = last_instant + 7;
            let next_origin = instant - 2;
            let next_clock =
                CoordinationClock::fixture_on_basis(fixture_ref(&format!("recreated-{round}")), 2).expect("next clock");
            commit_clock_transition(runtime, next_clock, fabricated_authority, |_, _, _| {
                Ok(shared_reading(instant, retiring_origin, next_origin))
            })
            .expect("repeated transition");
            let after_remaining = runtime.state.locks.get("resource:held").expect("held lock").lease_expires_at - runtime.state.time_ticks;
            assert_eq!(after_remaining, before_remaining - 7, "round {round}");
            assert_eq!(runtime.state.locks.get("resource:expiring").expect("expired lock").lease_expires_at, runtime.state.time_ticks);
            last_instant = instant;
            retiring_origin = next_origin;
        }
    }

    /// Mutations stay admitted on the final basis, the expired lease is expirable at once, and fencing
    /// tokens keep increasing; the original basis is now the mismatched one.
    fn expired_lease_is_expirable_on_the_final_basis(runtime: &mut CoordinationRuntime, expiring_token: &FencingToken) {
        let basis = runtime.state.time_basis_ref.clone();
        set_clock(runtime, 400);
        apply_generated_denial(
            runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:expiring", "contender", 3, None),
            "does not match committed time basis",
        );
        set_clock_on(runtime, &basis, 2);
        apply_generated_request(
            runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:expiring", "contender", 4, None),
            COORDINATION_DECISION_PASS,
        );
        apply_generated_denial(
            runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:held", "contender", 5, None),
            "is unexpired",
        );
        let successor = apply_generated_request(
            runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:expiring", "contender", 6, Some(lease(5))),
            COORDINATION_DECISION_PASS,
        );
        assert!(successor.token.expect("successor token").token > expiring_token.token);
    }
