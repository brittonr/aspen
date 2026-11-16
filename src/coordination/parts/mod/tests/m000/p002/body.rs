
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
        assert_eq!(receipt_check(&read.receipt.value, "local-stale-non-authoritative").as_deref(), Some("pass"));
        assert_eq!(receipt_check(&read.receipt.value, "committed-state-bound").as_deref(), Some("diagnostic"));

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

    fn receipt_check(receipt: &IoValue, name: &str) -> Option<String> {
        let fields = simple_record(receipt, "coordination-receipt-v2", COORDINATION_RECEIPT_FIELD_COUNT).expect("fields");
        parse_checks(&fields[12])
            .expect("receipt checks")
            .into_iter()
            .find_map(|(check, status)| (check == name).then_some(status))
    }

    fn mode_request(service: &str, operation: &str, key: &str, label: &str, mode: &str) -> IoValue {
        let (auth, resources, policies) = refs();
        coordination_request_value(&CoordinationRequestInput {
            service: service.to_string(),
            operation: operation.to_string(),
            key: key.to_string(),
            client_session: format!("session:{label}"),
            operation_id_ref: fixture_ref(label),
            read_consistency_mode: mode.to_string(),
            payload: None,
            authority_refs: auth,
            resource_refs: resources,
            policy_refs: policies,
        })
        .expect("coordination request")
    }

    #[test]
    fn in_process_runtime_admits_local_committed_and_denies_linearizable() {
        // r[verify molten.coordination.read_consistency_modes]
        // r[verify molten.coordination.local_stale_boundaries]
        const UNSUPPORTED: &str = crate::raft_control_plane::IN_PROCESS_REGISTRY_LINEARIZABLE_UNSUPPORTED;
        let mut runtime = runtime();
        let committed = mode_request(SERVICE_LOCK, OP_ACQUIRE, "resource:committed", "committed-acquire", LOCAL_COMMITTED);
        let committed = apply_coordination_request(&mut runtime, &committed).expect("local-committed acquire");
        assert_eq!(committed.receipt.decision, "pass");
        assert!(committed.token.is_some());
        assert!(committed.raft_commit_ref.is_some());

        let before = snapshot_from_state(&runtime.state).expect("before").state_ref;
        for (label, mode, boundary) in [
            ("linearizable-acquire", READ_CONSISTENCY_LINEARIZABLE, UNSUPPORTED),
            ("stale-acquire", READ_CONSISTENCY_LOCAL_STALE, "local-stale"),
        ] {
            let request = mode_request(SERVICE_LOCK, OP_ACQUIRE, "resource:other", label, mode);
            let denied = apply_coordination_request(&mut runtime, &request).expect("protected acquire");
            assert_eq!(denied.receipt.decision, "deny", "{mode}");
            assert!(denied.token.is_none(), "{mode}");
            assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains(boundary)), "{mode}");
            assert_eq!(snapshot_from_state(&runtime.state).expect("after").state_ref, before, "{mode}");
        }

        let register = request(
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:committed",
            "registrar",
            1,
            Some(record("endpoint", vec![string(fixture_ref("endpoint")), string(fixture_ref("evidence"))])),
        );
        assert_eq!(apply_coordination_request(&mut runtime, &register).expect("register").receipt.decision, "pass");
        let read = mode_request(SERVICE_REGISTRY, OP_READ, "svc:committed", "committed-read", LOCAL_COMMITTED);
        let read = apply_coordination_request(&mut runtime, &read).expect("local-committed read");
        assert_eq!(read.receipt.decision, "pass");
        assert_eq!(receipt_check(&read.receipt.value, "committed-state-bound").as_deref(), Some("pass"));
        assert_eq!(read.raft_read_receipt.as_ref().expect("registry read").decision, "pass");
        assert_eq!(read.assertions[0].read_consistency_mode, LOCAL_COMMITTED);

        let linearizable =
            mode_request(SERVICE_REGISTRY, OP_READ, "svc:committed", "linearizable-read", READ_CONSISTENCY_LINEARIZABLE);
        let linearizable = apply_coordination_request(&mut runtime, &linearizable).expect("linearizable read");
        assert_eq!(linearizable.receipt.decision, "deny");
        assert!(linearizable.assertions.is_empty());
        assert_ne!(receipt_check(&linearizable.receipt.value, "committed-state-bound").as_deref(), Some("pass"));
        assert!(linearizable.receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains(UNSUPPORTED)));
    }

    const LOCAL_COMMITTED: &str = READ_CONSISTENCY_LOCAL_COMMITTED;

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

    // r[verify molten.coordination.rate_limit_semaphore]
    #[test]
    fn rate_limit_is_a_per_key_quota_that_later_operations_do_not_reset() {
        let mut runtime = runtime();
        let quota = runtime.manifest.rate_limit;
        let mut sequence = 0;
        let mut acquire = |runtime: &mut CoordinationRuntime, key: &str, label: &str, decision: &str| {
            let request_value = request(
                SERVICE_RATE_LIMIT,
                OP_ACQUIRE,
                key,
                label,
                next_trace_sequence(&mut sequence),
                None,
            );
            if decision == COORDINATION_DECISION_DENY {
                apply_generated_denial(runtime, &request_value, "rate-limit quota exhausted; it does not reset");
            } else {
                apply_generated_request(runtime, &request_value, decision);
            }
        };
        for index in 0..quota {
            acquire(&mut runtime, "rate:quota", &format!("quota-{index}"), COORDINATION_DECISION_PASS);
        }
        acquire(&mut runtime, "rate:quota", "quota-over", COORDINATION_DECISION_DENY);
        // Later committed operations on another key do not reset the exhausted key, and that key has
        // its own full quota.
        for index in 0..quota {
            acquire(&mut runtime, "rate:other", &format!("other-{index}"), COORDINATION_DECISION_PASS);
        }
        acquire(&mut runtime, "rate:quota", "quota-after", COORDINATION_DECISION_DENY);
        assert_eq!(runtime.state.rates.get("rate:quota"), Some(&quota));
    }
