
    #[test]
    fn local_stale_reads_are_labeled_and_cannot_authorize_mutations() {
        // r[verify molten.coordination.read_consistency_modes]
        // r[verify molten.coordination.local_stale_boundaries]
        let mut runtime = runtime();
        let endpoint = fixture_ref("endpoint-stale");
        let evidence = fixture_ref("evidence-stale");
        let register = request(
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:stale",
            "registrar",
            LOCAL_STALE_REGISTER_SEQUENCE,
            Some(record("endpoint", vec![string(&endpoint), string(&evidence)])),
        );
        let register = apply_coordination_request(&mut runtime, &register).expect("register");
        assert_eq!(register.receipt.decision, "pass");

        let (auth, resources, policies) = refs();
        let local_stale_read = coordination_request_value(&CoordinationRequestInput {
            service: SERVICE_REGISTRY.to_string(),
            operation: OP_READ.to_string(),
            key: "svc:stale".to_string(),
            client_session: "reader".to_string(),
            operation_id_ref: fixture_ref("local-stale-read"),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            payload: None,
            authority_refs: auth.clone(),
            resource_refs: resources.clone(),
            policy_refs: policies.clone(),
        })
        .expect("local stale read request");
        let read = apply_coordination_request(&mut runtime, &local_stale_read).expect("local stale read");
        assert_eq!(read.receipt.decision, "pass");
        assert_eq!(read.receipt.read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert_eq!(read.assertions[0].read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert!(to_text(&read.receipt.value).expect("receipt text").contains("local-stale-non-authoritative"));

        let stale_mutation = coordination_request_value(&CoordinationRequestInput {
            service: SERVICE_LOCK.to_string(),
            operation: OP_ACQUIRE.to_string(),
            key: "resource:stale".to_string(),
            client_session: "owner".to_string(),
            operation_id_ref: fixture_ref("local-stale-mutation"),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            payload: None,
            authority_refs: auth,
            resource_refs: resources,
            policy_refs: policies,
        })
        .expect("stale mutation request");
        let denied = apply_coordination_request(&mut runtime, &stale_mutation).expect("stale mutation denied");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.join(";").contains("local-stale"));
    }

    #[test]
    fn batch_envelopes_apply_deterministically_and_deny_invalid_batches() {
        // r[verify molten.coordination.batched_control_plane_operations]
        let mut batch_runtime = runtime();
        let initial_state = snapshot_from_state(&batch_runtime.state).expect("initial snapshot");
        let enqueue = request(
            SERVICE_QUEUE,
            OP_ENQUEUE,
            "queue:batch",
            "producer",
            BATCH_ENQUEUE_SEQUENCE,
            Some(record("item", vec![string("batch-one")])),
        );
        let read = request(SERVICE_QUEUE, OP_READ, "queue:batch", "reader", BATCH_READ_SEQUENCE, None);
        let batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-pass"),
            compare_state_ref: Some(initial_state.state_ref.clone()),
            requests: vec![enqueue.clone(), read],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("batch envelope");
        assert_eq!(crate::ledger::artifact_kind(&batch), "coordination-batch-envelope");
        let applied = apply_coordination_batch(&mut batch_runtime, &batch).expect("apply batch");
        assert_eq!(applied.decision, "pass");
        assert_eq!(applied.results.len(), BATCH_EXPECTED_RESULTS);
        assert_eq!(batch_runtime.state.queues.get("queue:batch").expect("queue")[QUEUE_FRONT_INDEX], "batch-one");

        let mut duplicate_runtime = runtime();
        let before_duplicate = snapshot_from_state(&duplicate_runtime.state).expect("before duplicate");
        let duplicate_batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-duplicate"),
            compare_state_ref: Some(before_duplicate.state_ref.clone()),
            requests: vec![enqueue.clone(), enqueue],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("duplicate batch");
        let duplicate = apply_coordination_batch(&mut duplicate_runtime, &duplicate_batch).expect("duplicate batch deny");
        assert_eq!(duplicate.decision, "deny");
        assert!(duplicate.results.is_empty());
        assert_eq!(snapshot_from_state(&duplicate_runtime.state).expect("after duplicate").state_ref, before_duplicate.state_ref);

        let stale_compare_batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-stale-compare"),
            compare_state_ref: Some(fixture_ref("stale-state")),
            requests: vec![request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                "queue:stale-compare",
                "producer",
                STALE_COMPARE_SEQUENCE,
                Some(record("item", vec![string("stale")])),
            )],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("stale compare batch");
        let stale_compare = apply_coordination_batch(&mut duplicate_runtime, &stale_compare_batch).expect("stale compare deny");
        assert_eq!(stale_compare.decision, "deny");
        assert!(stale_compare.results.is_empty());
    }

    #[test]
    fn semaphore_rate_election_barrier_and_registry_primitives_are_deterministic() {
        // r[verify molten.coordination_state_machine_proof.primitive_transition_cores]
        // r[verify molten.coordination_state_machine_proof.transition_matrix_tests]
        let mut runtime = runtime();
        let first =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "a", 1, None))
                .expect("sem a");
        let second =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "b", 2, None))
                .expect("sem b");
        let exhausted =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "c", 3, None))
                .expect("sem deny");
        assert_eq!(first.receipt.decision, "pass");
        assert_eq!(second.receipt.decision, "pass");
        assert_eq!(exhausted.receipt.decision, "deny");
        assert!(exhausted.receipt.diagnostics.join(";").contains("semaphore exhausted"));
        let rate_a = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "a", 4, None),
        )
        .expect("rate a");
        let rate_b = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "b", 5, None),
        )
        .expect("rate b");
        let rate_c = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "c", 6, None),
        )
        .expect("rate deny");
        assert_eq!(rate_a.receipt.decision, "pass");
        assert_eq!(rate_b.receipt.decision, "pass");
        assert_eq!(rate_c.receipt.decision, "deny");
        let elect = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, "election:test", "leader", 7, None),
        )
        .expect("elect");
        assert_eq!(elect.receipt.decision, "pass");
        assert!(elect.token.is_some());
        let barrier_wait = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "a", 8, None),
        )
        .expect("barrier wait");
        let barrier_release = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "b", 9, None),
        )
        .expect("barrier release");
        assert_eq!(barrier_wait.receipt.decision, "pass");
        assert_eq!(barrier_release.receipt.decision, "pass");
        assert!(runtime.state.barriers.get("barrier:test").expect("barrier").is_released);
    }
