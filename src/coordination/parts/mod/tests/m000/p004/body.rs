
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
