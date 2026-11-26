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
        let mut committed = Vec::with_capacity(steps.len());
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
