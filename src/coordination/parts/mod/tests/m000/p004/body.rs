
    fn lease(ttl: u64) -> IoValue {
        record("lease", vec![u64_value(ttl)])
    }

    fn presented_time(ticks: u64) -> IoValue {
        record("now", vec![u64_value(ticks)])
    }

    /// Moves coordination time the only way it moves: the shell installs a later reading of its clock.
    fn set_clock(runtime: &mut CoordinationRuntime, ticks: u64) {
        runtime.clock = CoordinationClock::fixture(ticks);
    }

    /// Moves time on an explicit basis, for runtimes that went through a clock basis transition.
    fn set_clock_on(runtime: &mut CoordinationRuntime, time_basis_ref: &str, ticks: u64) {
        runtime.clock = CoordinationClock::fixture_on_basis(time_basis_ref, ticks).expect("fixture clock");
    }

    /// A clock port whose reads fail, as a live adapter does when its capability is unavailable.
    #[derive(Debug)]
    struct FailingClockAdapter;

    impl crate::fabric_time::TimerClockAdapter for FailingClockAdapter {
        fn profile_ref(&self) -> &str {
            "failing-clock"
        }

        fn timer_domain(&self) -> crate::fabric_time::TimeDomain {
            crate::fabric_time::TimeDomain::Monotonic
        }

        fn now_ticks(&mut self) -> crate::fabric::FabricPortResult<u64> {
            Err(crate::fabric::FabricPortError::capability("coordination test clock unavailable"))
        }

        fn await_ticks(&mut self, _target_ticks: u64) -> crate::fabric::FabricPortResult<u64> {
            Err(crate::fabric::FabricPortError::capability("coordination test clock unavailable"))
        }
    }

    /// Commits `count` unrelated registry updates, the hostile stream a non-holder can produce.
    fn commit_unrelated(runtime: &mut CoordinationRuntime, first_sequence: u64, count: u64) {
        let endpoint = record("endpoint", vec![string(fixture_ref("clock-endpoint")), string(fixture_ref("clock-evidence"))]);
        for sequence in first_sequence..first_sequence + count {
            apply_generated_request(
                runtime,
                &request(SERVICE_REGISTRY, OP_REGISTER, "svc:clock", "hostile", sequence, Some(endpoint.clone())),
                COORDINATION_DECISION_PASS,
            );
        }
    }

    fn token_payload(token: u64) -> IoValue {
        record("token", vec![u64_value(token)])
    }

    fn commit_intent(result: &CoordinationApplyResult) -> &IoValue {
        result
            .evidence_values
            .iter()
            .find(|value| crate::ledger::artifact_kind(value) == "coordination-commit-intent")
            .expect("commit intent evidence")
    }

    fn registry_target_ref(runtime: &CoordinationRuntime, service: &str, key: &str) -> Option<String> {
        let (auth, resources, _policies) = refs();
        crate::raft_control_plane::read_control_registry(&crate::raft_control_plane::ControlRegistryReadInput {
            runtime: &runtime.raft,
            read_consistency_mode: READ_CONSISTENCY_LOCAL_COMMITTED.to_string(),
            namespace: coordination_namespace(service),
            name: key.to_string(),
            authority_refs: auth,
            resource_refs: resources,
        })
        .expect("registry read")
        .target_ref
    }

    #[test]
    fn committed_lock_and_election_state_persist_the_issued_commit_bound_token_ref() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        let lock = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:parity", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        let token = lock.token.as_ref().expect("lock token");
        let committed = runtime.state.locks.get("resource:parity").expect("committed lock");
        assert_eq!(committed.token_ref, token.token_ref);
        assert_eq!(committed.token, token.token);
        assert_eq!(Some(&token.commit_receipt_ref), lock.raft_commit_ref.as_ref());
        assert_eq!(lock.receipt.token_ref.as_ref(), Some(&token.token_ref));
        assert!(lock.state_snapshot.retention_refs.contains(&token.token_ref));
        assert!(to_text(&lock.assertions[0].value).expect("assertion text").contains(&token.token_ref));
        // A token bound to any other commit ref, such as the old pending placeholder, has a different ref.
        let pending = parse_fencing_token(
            &fencing_token_value("resource:parity", "holder", token.token, token.lease_epoch, &fixture_ref("pending-coordination-commit"))
                .expect("pending token value"),
        )
        .expect("pending token");
        assert_ne!(committed.token_ref, pending.token_ref);

        // The control plane commits a token-free intent; the committed state adds the commit-bound entry.
        let intent = commit_intent(&lock);
        let intent_text = to_text(intent).expect("intent text");
        assert!(!intent_text.contains(&token.token_ref));
        assert_eq!(registry_target_ref(&runtime, SERVICE_LOCK, "resource:parity"), Some(canonical_hash(intent).expect("intent ref")));
        let mut rebuilt = runtime.state.clone();
        rebuilt.locks.remove("resource:parity");
        let claim = CoordinationTokenClaim {
            service: SERVICE_LOCK.to_string(),
            key: "resource:parity".to_string(),
            owner: "holder".to_string(),
            token: token.token,
            lease_epoch: token.lease_epoch,
            lease_expires_at: committed.lease_expires_at,
        };
        let recomputed_intent = coordination_commit_intent_value(CommitIntentValueInput {
            request_ref: &lock.request.request_ref,
            before_state_ref: &lock.receipt.before_state_ref,
            unbound_after_state_ref: &snapshot_from_state(&rebuilt).expect("unbound snapshot").state_ref,
            token_claim: Some(&claim),
            time_sample: lock.time_sample.as_ref().expect("committed clock sample"),
        })
        .expect("recomputed intent");
        assert_eq!(&recomputed_intent, intent);
        let rebound = bind_token_claim(&mut rebuilt, &claim, &token.commit_receipt_ref).expect("rebind claim");
        assert_eq!(rebound, *token);
        assert_eq!(rebuilt, runtime.state);
        assert!(bind_token_claim(&mut rebuilt, &claim, &token.commit_receipt_ref)
            .expect_err("claim over a current holder")
            .to_string()
            .contains("would displace a current holder"));

        let election = apply_generated_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, "election:parity", "leader", 2, None),
            COORDINATION_DECISION_PASS,
        );
        let election_token = election.token.as_ref().expect("election token");
        let leader = runtime.state.elections.get("election:parity").expect("committed election");
        assert_eq!(leader.token_ref, election_token.token_ref);
        assert_eq!(Some(&election_token.commit_receipt_ref), election.raft_commit_ref.as_ref());
        assert!(election_token.token > token.token);
    }

    #[test]
    fn receipt_assertion_refs_bind_the_emitted_status_assertions() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        let acquire = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:assert", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        let endpoint = Some(record("endpoint", vec![string(fixture_ref("endpoint")), string(fixture_ref("evidence"))]));
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_REGISTRY, OP_REGISTER, "svc:assert", "registrar", 2, endpoint),
            COORDINATION_DECISION_PASS,
        );
        let read = apply_generated_request(
            &mut runtime,
            &request(SERVICE_REGISTRY, OP_READ, "svc:assert", "reader", 3, None),
            COORDINATION_DECISION_PASS,
        );
        for result in [&acquire, &read] {
            assert_eq!(result.assertions.len(), 1);
            let assertion = &result.assertions[0];
            assert_eq!(result.receipt.dataspace_assertion_refs, vec![assertion.assertion_ref.clone()]);
            assert_eq!(canonical_hash(&assertion.value).expect("assertion ref"), assertion.assertion_ref);
            assert!(result.evidence_values.contains(&assertion.value));
            assert!(runtime.assertions.contains(assertion));
        }
        assert_eq!(acquire.assertions[0].control_plane_receipt_ref, acquire.raft_commit_ref.clone().expect("commit ref"));
        assert_eq!(read.assertions[0].control_plane_receipt_ref, read.raft_read_receipt.as_ref().expect("read").receipt_ref);

        // A denied mutation emits no assertion and its receipt names none.
        let denied = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:assert", "contender", 4, None),
            COORDINATION_DECISION_DENY,
        );
        assert!(denied.assertions.is_empty());
        assert!(denied.receipt.dataspace_assertion_refs.is_empty());
        // v2 assertions, which bound the coordination receipt instead, no longer parse.
        let fields = simple_record(&acquire.assertions[0].value, "coordination-status-assertion-v3", 8).expect("fields");
        let mut v2_fields = (0..8).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
        v2_fields[0] = string("molten.coordination.status-assertion.v2");
        let v2 = record("coordination-status-assertion-v2", v2_fields);
        assert!(parse_coordination_status_assertion(&v2).is_err());
    }

    #[test]
    fn hostile_mutations_do_not_expire_a_live_lock_but_the_clock_expires_an_idle_one() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        let key = "resource:lease";
        set_clock(&mut runtime, 100);
        let first = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, key, "crashed", 1, Some(lease(50))),
            COORDINATION_DECISION_PASS,
        );
        let first_token = first.token.expect("first token").token;
        assert_eq!(runtime.state.time_ticks, 100);
        assert_eq!(runtime.state.locks.get(key).expect("lock").lease_expires_at, 150);

        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_ACQUIRE, key, "successor", 2, None), "already held by crashed");
        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_EXPIRE, key, "successor", 3, None), "is unexpired");
        // A request cannot present time to expire the live lease early.
        for (sequence, presented) in [(4, 150), (5, u64::MAX)] {
            apply_generated_denial(
                &mut runtime,
                &request(SERVICE_LOCK, OP_EXPIRE, key, "successor", sequence, Some(presented_time(presented))),
                "takes no payload",
            );
        }
        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_EXPIRE, "resource:free", "successor", 6, None), "not held");

        // A hostile stream of committed unrelated mutations does not age the lease.
        commit_unrelated(&mut runtime, 1_000, 200);
        assert_eq!(runtime.state.time_ticks, 100);
        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_EXPIRE, key, "successor", 7, None), "is unexpired");
        set_clock(&mut runtime, 149);
        commit_unrelated(&mut runtime, 2_000, 50);
        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_EXPIRE, key, "successor", 8, None), "coordination clock reads 149");
        assert!(runtime.state.locks.contains_key(key));

        // The crashed holder stays idle; only the clock passing its expiry makes the lease expirable.
        set_clock(&mut runtime, 150);
        let expired = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, key, "successor", 9, None),
            COORDINATION_DECISION_PASS,
        );
        assert!(expired.token.is_none());
        assert!(!runtime.state.locks.contains_key(key));
        assert_eq!(runtime.state.time_ticks, 150);

        apply_generated_denial(&mut runtime, &request(SERVICE_LOCK, OP_ACQUIRE, key, "successor", 10, Some(lease(0))), "lease ttl 0");
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, key, "successor", 11, Some(lease(MAX_COORDINATION_LEASE_TICKS + 1))),
            "lease ttl",
        );
        let successor = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, key, "successor", 12, Some(lease(5))),
            COORDINATION_DECISION_PASS,
        );
        let successor_token = successor.token.expect("successor token");
        assert!(successor_token.token > first_token);
        let lock = runtime.state.locks.get(key).expect("lock");
        assert_eq!(lock.token_ref, successor_token.token_ref);
        assert_eq!(lock.lease_expires_at, 155);
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_RELEASE, key, "crashed", 13, Some(token_payload(first_token))),
            "stale fencing token",
        );
    }

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

        // Back-to-back recreations with a fabricated authority ref: 7 ticks pass during each validation and
        // each new basis is opened 2 ticks before its reading. The remaining TTL shrinks by exactly the elapsed
        // time and never grows.
        let mut last_instant = 155_u64;
        let mut retiring_origin = 150_u64;
        for round in 1..=3_u64 {
            let before_remaining = runtime.state.locks.get("resource:held").expect("held lock").lease_expires_at - runtime.state.time_ticks;
            let instant = last_instant + 7;
            let next_origin = instant - 2;
            let next_clock =
                CoordinationClock::fixture_on_basis(fixture_ref(&format!("recreated-{round}")), 2).expect("next clock");
            commit_clock_transition(&mut runtime, next_clock, &fabricated_authority, |_, _, _| {
                Ok(shared_reading(instant, retiring_origin, next_origin))
            })
            .expect("repeated transition");
            let after_remaining = runtime.state.locks.get("resource:held").expect("held lock").lease_expires_at - runtime.state.time_ticks;
            assert_eq!(after_remaining, before_remaining - 7, "round {round}");
            assert_eq!(runtime.state.locks.get("resource:expiring").expect("expired lock").lease_expires_at, runtime.state.time_ticks);
            last_instant = instant;
            retiring_origin = next_origin;
        }
        let basis = runtime.state.time_basis_ref.clone();

        // Mutations stay admitted on the final basis, the expired lease is expirable at once, and fencing
        // tokens keep increasing; the original basis is now the mismatched one.
        set_clock(&mut runtime, 400);
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:expiring", "contender", 3, None),
            "does not match committed time basis",
        );
        set_clock_on(&mut runtime, &basis, 2);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:expiring", "contender", 4, None),
            COORDINATION_DECISION_PASS,
        );
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:held", "contender", 5, None),
            "is unexpired",
        );
        let successor = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:expiring", "contender", 6, Some(lease(5))),
            COORDINATION_DECISION_PASS,
        );
        assert!(successor.token.expect("successor token").token > expiring_token.token);
    }

    #[test]
    fn clock_transition_accepts_only_a_live_clock_and_repeated_recreation_never_extends_leases() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let (auth, _resources, _policies) = refs();
        let fabricated_authority = vec![fixture_ref("fabricated-clock-authority")];
        let mut runtime = new_coordination_runtime(&manifest, CoordinationClock::live().expect("live clock")).expect("live runtime");
        let first = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:live-epoch", "holder", 1, Some(lease(MAX_COORDINATION_LEASE_TICKS))),
            COORDINATION_DECISION_PASS,
        );
        let first_token = first.token.expect("first token");
        let original_remaining = MAX_COORDINATION_LEASE_TICKS;
        let before_state = runtime.state.clone();

        // A frozen or non-live clock is refused, whatever authority ref accompanies it.
        for frozen in [
            CoordinationClock::fixture_on_basis(fixture_ref("frozen-clock"), 0).expect("frozen clock"),
            CoordinationClock::fixture(u64::MAX),
        ] {
            let error = transition_coordination_clock(&mut runtime, frozen, &fabricated_authority).expect_err("frozen clock denied");
            assert!(error.to_string().contains("only a live node monotonic clock"), "{error}");
        }
        // A live clock whose basis ref does not match its provenance is refused.
        let mut forged = CoordinationClock::live().expect("live clock");
        forged.time_basis_ref = fixture_ref("forged-live-basis");
        let error = transition_coordination_clock(&mut runtime, forged, &fabricated_authority).expect_err("forged basis denied");
        assert!(error.to_string().contains("provenance does not match"), "{error}");
        assert_eq!(runtime.state, before_state);

        // A simulation runtime keeps its deterministic clock.
        let mut simulation = new_coordination_simulation_runtime(&manifest, CoordinationClock::fixture(0)).expect("simulation runtime");
        let error = recreate_coordination_live_clock(&mut simulation, &auth).expect_err("simulation runtime denied");
        assert!(error.to_string().contains("simulation runtimes"), "{error}");
        assert!(!simulation.clock.is_live());

        // One live reading converts into both live bases; a retiring clock that no longer reads on the committed
        // basis (restart) is credited only up to the last committed sample, and a non-live next clock is refused.
        let next_live = CoordinationClock::live().expect("live clock");
        let reading = live_shared_reading(&runtime.clock, &runtime.state, &next_live).expect("shared reading");
        assert!(reading.retiring_ticks >= runtime.state.time_ticks);
        let restarted = live_shared_reading(&CoordinationClock::fixture(0), &runtime.state, &next_live).expect("restart reading");
        assert_eq!(restarted.retiring_ticks, runtime.state.time_ticks);
        assert!(live_shared_reading(&runtime.clock, &runtime.state, &CoordinationClock::fixture(0)).is_err());

        // Repeated live-to-live recreations, even with a fabricated authority ref, credit elapsed time and never
        // push the lease past its remaining TTL.
        let mut remaining = original_remaining;
        let mut last_basis = before_state.time_basis_ref.clone();
        for _ in 0..3 {
            let transition = recreate_coordination_live_clock(&mut runtime, &fabricated_authority).expect("live-to-live transition");
            assert_eq!(transition.from.time_basis_ref, last_basis);
            assert_ne!(transition.to.time_basis_ref, last_basis);
            assert!(runtime.clock.is_live());
            assert_eq!(runtime.state.time_basis_ref, transition.to.time_basis_ref);
            let lock = runtime.state.locks.get("resource:live-epoch").expect("lock survives");
            assert_eq!(lock.token_ref, first_token.token_ref);
            let after = lock.lease_expires_at - transition.to.ticks;
            assert!(after <= remaining, "recreation extended the lease from {remaining} to {after}");
            remaining = after;
            last_basis = transition.to.time_basis_ref;
        }
        assert!(remaining <= original_remaining);
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:live-epoch", "contender", 2, None),
            "is unexpired",
        );
        let next = apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:live-other", "contender", 3, None),
            COORDINATION_DECISION_PASS,
        );
        assert!(next.token.expect("next token").token > first_token.token);
        assert_eq!(next.time_sample.expect("sample").time_basis_ref, last_basis);
    }

    #[test]
    fn production_constructors_accept_only_live_clocks() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let control_group = crate::raft_control_plane::control_registry_fixture_manifest_value().expect("group");
        let error = new_coordination_runtime(&manifest, CoordinationClock::fixture(0)).expect_err("fixture clock denied");
        assert!(error.to_string().contains("only a live node monotonic clock"), "{error}");
        let frozen = CoordinationClock::fixture_on_basis(fixture_ref("frozen-production-clock"), 0).expect("frozen clock");
        let error = new_coordination_runtime_with_control_group(&manifest, &control_group, frozen).expect_err("frozen clock denied");
        assert!(error.to_string().contains("only a live node monotonic clock"), "{error}");
        let mut forged = CoordinationClock::live().expect("live clock");
        forged.time_basis_ref = fixture_ref("forged-production-basis");
        let error = new_coordination_runtime(&manifest, forged).expect_err("forged live clock denied");
        assert!(error.to_string().contains("provenance does not match"), "{error}");
        let error =
            new_coordination_simulation_runtime(&manifest, CoordinationClock::live().expect("live clock")).expect_err("live simulation");
        assert!(error.to_string().contains("simulation runtimes take a deterministic clock"), "{error}");
        assert!(new_coordination_runtime(&manifest, CoordinationClock::live().expect("live clock")).expect("live runtime").clock.is_live());
        assert!(!new_coordination_simulation_runtime(&manifest, CoordinationClock::fixture(0)).expect("simulation").clock.is_live());
    }

    enum ReplayStep {
        Request(u64, IoValue),
        ClockTransition(String, u64),
    }

    enum Committed {
        Request(IoValue, CoordinationTimeSample),
        ClockTransition(CoordinationTimeSample, CoordinationTimeSample),
    }

    #[test]
    fn replaying_committed_requests_with_their_clock_samples_reproduces_state() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let (auth, _resources, _policies) = refs();
        let restarted = fixture_ref("replayed-restarted-clock");
        let item = Some(record("item", vec![string("replayed")]));
        let steps = vec![
            ReplayStep::Request(10, request(SERVICE_LOCK, OP_ACQUIRE, "resource:replay", "a", 1, Some(lease(20)))),
            ReplayStep::Request(12, request(SERVICE_ELECTION, OP_ELECT, "election:replay", "leader", 2, Some(lease(5)))),
            ReplayStep::Request(15, request(SERVICE_QUEUE, OP_ENQUEUE, "queue:replay", "producer", 3, item.clone())),
            ReplayStep::Request(18, request(SERVICE_LOCK, OP_ACQUIRE, "resource:replay", "b", 4, None)),
            ReplayStep::Request(14, request(SERVICE_QUEUE, OP_ENQUEUE, "queue:replay", "producer", 5, item)),
            ReplayStep::ClockTransition(restarted.clone(), 3),
            ReplayStep::Request(22, request(SERVICE_LOCK, OP_EXPIRE, "resource:replay", "b", 6, None)),
            ReplayStep::Request(23, request(SERVICE_LOCK, OP_EXPIRE, "resource:replay", "b", 7, None)),
            ReplayStep::Request(24, request(SERVICE_LOCK, OP_ACQUIRE, "resource:replay", "b", 8, Some(lease(9)))),
            ReplayStep::Request(30, request(SERVICE_ELECTION, OP_EXPIRE, "election:replay", "observer", 9, None)),
        ];
        let mut original = runtime();
        let mut basis = coordination_fixture_time_basis_ref();
        let mut committed = Vec::new();
        for step in &steps {
            match step {
                ReplayStep::Request(ticks, request_value) => {
                    set_clock_on(&mut original, &basis, *ticks);
                    let result = apply_coordination_request(&mut original, request_value).expect("original apply");
                    if let Some(sample) = result.time_sample.clone() {
                        assert_eq!(result.receipt.decision, COORDINATION_DECISION_PASS);
                        let intent_text = to_text(commit_intent(&result)).expect("intent text");
                        assert!(intent_text.contains(&sample.time_basis_ref));
                        committed.push(Committed::Request(request_value.clone(), sample));
                    } else {
                        assert_eq!(result.receipt.decision, COORDINATION_DECISION_DENY);
                    }
                }
                ReplayStep::ClockTransition(next_basis, start) => {
                    let clock = CoordinationClock::fixture_on_basis(next_basis.clone(), *start).expect("clock");
                    let reading = SharedClockReading { retiring_ticks: original.state.time_ticks, next_ticks: *start };
                    let transition = commit_clock_transition(&mut original, clock, &auth, |_, _, _| Ok(reading)).expect("transition");
                    assert!(to_text(&transition.value).expect("transition text").contains(next_basis.as_str()));
                    committed.push(Committed::ClockTransition(transition.from, transition.to));
                    basis = next_basis.clone();
                }
            }
        }
        assert_eq!(committed.len(), 7);

        let mut replica = runtime();
        for entry in &committed {
            match entry {
                Committed::Request(request_value, sample) => {
                    set_clock_on(&mut replica, &sample.time_basis_ref, sample.ticks);
                    let result = apply_coordination_request(&mut replica, request_value).expect("replayed apply");
                    assert_eq!(result.receipt.decision, COORDINATION_DECISION_PASS);
                    assert_eq!(result.time_sample.as_ref(), Some(sample));
                }
                Committed::ClockTransition(from, to) => {
                    let clock = CoordinationClock::fixture_on_basis(to.time_basis_ref.clone(), to.ticks).expect("clock");
                    let reading = SharedClockReading { retiring_ticks: from.ticks, next_ticks: to.ticks };
                    commit_clock_transition(&mut replica, clock, &auth, |_, _, _| Ok(reading)).expect("replayed transition");
                }
            }
        }
        assert_eq!(replica.state, original.state);
        assert_eq!(state_ref(&replica), state_ref(&original));
        assert_eq!(replica.tokens, original.tokens);
    }

    #[test]
    fn extreme_clock_samples_are_denied_and_leases_stay_representable() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut runtime = runtime();
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:extreme", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        let before = runtime.state.time_ticks;
        apply_generated_denial(
            &mut runtime,
            &request(SERVICE_LOCK, OP_EXPIRE, "resource:extreme", "attacker", 2, Some(presented_time(u64::MAX))),
            "takes no payload",
        );
        assert_eq!(runtime.state.time_ticks, before);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:after", "attacker", 3, None),
            COORDINATION_DECISION_PASS,
        );

        // At the ceiling a maximal lease is still representable without overflow.
        set_clock(&mut runtime, MAX_COORDINATION_TIME_TICKS);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:ceiling", "holder", 4, Some(lease(MAX_COORDINATION_LEASE_TICKS))),
            COORDINATION_DECISION_PASS,
        );
        assert_eq!(runtime.state.time_ticks, MAX_COORDINATION_TIME_TICKS);
        assert_eq!(runtime.state.locks.get("resource:ceiling").expect("lock").lease_expires_at, u64::MAX);
        // Samples past the ceiling are denied with state preserved instead of overflowing.
        for (sequence, ticks) in [(5, MAX_COORDINATION_TIME_TICKS + 1), (6, u64::MAX)] {
            set_clock(&mut runtime, ticks);
            apply_generated_denial(
                &mut runtime,
                &request(SERVICE_LOCK, OP_ACQUIRE, "resource:beyond", "holder", sequence, None),
                "coordination time exhausted",
            );
        }
        assert!(lease_expiry(None, u64::MAX).is_err());
        // The committed time is unchanged, so an in-range sample still admits a grant.
        set_clock(&mut runtime, MAX_COORDINATION_TIME_TICKS);
        apply_generated_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:still", "holder", 7, Some(lease(1))),
            COORDINATION_DECISION_PASS,
        );
    }

    #[test]
    fn coordination_runtime_binds_the_callers_control_group() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let fixture_group = crate::raft_control_plane::control_registry_fixture_manifest_value().expect("fixture group");
        let matching = runtime();
        assert_eq!(matching.raft.manifest.manifest_ref, matching.manifest.control_group_ref);

        let parsed_group = crate::raft_control_plane::parse_raft_group_manifest(&fixture_group).expect("group");
        let other_group = crate::raft_control_plane::raft_group_manifest_value(&crate::raft_control_plane::RaftGroupManifestInput {
            group_id: "raft:coordination-other".to_string(),
            members: parsed_group.members,
            state_machine: parsed_group.state_machine,
            command_schemas: parsed_group.command_schemas,
            read_mode: parsed_group.read_mode,
            snapshot_policy_ref: parsed_group.snapshot_policy_ref,
            policy_refs: parsed_group.policy_refs,
            resource_refs: parsed_group.resource_refs,
        })
        .expect("other group");
        let other_group_ref = canonical_hash(&other_group).expect("other group ref");
        let base = parse_coordination_service_manifest(&coordination_fixture_manifest_value().expect("manifest")).expect("parse");
        let other_manifest = coordination_service_manifest_value(&CoordinationServiceManifestInput {
            service_id: base.service_id,
            services: base.services,
            control_group_ref: other_group_ref.clone(),
            queue_capacity: base.queue_capacity,
            semaphore_capacity: base.semaphore_capacity,
            rate_limit: base.rate_limit,
            barrier_parties: base.barrier_parties,
            policy_refs: base.policy_refs,
            resource_refs: base.resource_refs,
        })
        .expect("other manifest");

        let error =
            new_coordination_runtime(&other_manifest, CoordinationClock::live().expect("live clock")).expect_err("unbound control group denied");
        assert!(error.to_string().contains("does not match the bound control-registry group"), "{error}");
        let fixture_manifest = coordination_fixture_manifest_value().expect("fixture manifest");
        let mismatched =
            new_coordination_runtime_with_control_group(&fixture_manifest, &other_group, CoordinationClock::live().expect("live clock"))
                .expect_err("mismatched group denied");
        assert!(mismatched.to_string().contains(&other_group_ref), "{mismatched}");

        let mut bound = new_coordination_runtime_with_control_group(&other_manifest, &other_group, CoordinationClock::live().expect("live clock"))
            .expect("bound group");
        assert_eq!(bound.raft.manifest.manifest_ref, other_group_ref);
        let acquire = apply_generated_request(
            &mut bound,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:group", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        assert!(to_text(&acquire.evidence_values[0]).expect("envelope text").contains(&other_group_ref));
    }

    #[test]
    fn live_coordination_clock_samples_a_non_regressing_node_monotonic_basis() {
        // r[verify molten.coordination.truthful_tokens_assertions_and_expiry]
        let mut clock = CoordinationClock::live().expect("live clock");
        let first = clock.sample().expect("first sample");
        let second = clock.sample().expect("second sample");
        assert_eq!(first.time_basis_ref, second.time_basis_ref);
        assert!(second.ticks >= first.ticks);
        crate::preserves_rail::validate_content_ref(clock.time_basis_ref()).expect("live basis ref");
        assert_ne!(clock.time_basis_ref(), coordination_fixture_time_basis_ref());
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let mut live = new_coordination_runtime(&manifest, clock).expect("live runtime");
        apply_generated_request(
            &mut live,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:live", "holder", 1, None),
            COORDINATION_DECISION_PASS,
        );
        assert_eq!(live.state.time_basis_ref, first.time_basis_ref);
        assert!(live.state.time_ticks >= second.ticks);
    }
