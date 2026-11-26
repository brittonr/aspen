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
