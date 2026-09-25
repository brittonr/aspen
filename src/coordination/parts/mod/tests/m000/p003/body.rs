
    #[hegel::test(test_cases = 12)]
    fn hegel_generated_coordination_trace_preserves_state_machine_invariants(tc: TestCase) {
        // r[verify molten.coordination_state_machine_proof.generated_traces]
        // r[verify molten.coordination_state_machine_proof.deny_no_mutation]
        // r[verify molten.coordination_state_machine_proof.duplicate_no_advance]
        // r[verify molten.coordination_state_machine_proof.replay_transition_kind]
        // r[verify molten.coordination_state_machine_proof.transition_receipt_binding]
        // r[verify molten.coordination_state_machine_proof.transition_matrix_tests]
        let salt = draw_coordination_trace_salt(&tc);
        let mut sequence = trace_sequence_start(salt);
        let mut runtime = runtime();
        let lock_key = format!("resource:generated:{salt}");
        let queue_key = format!("queue:generated:{salt}");
        let semaphore_key = format!("sem:generated:{salt}");
        let rate_key = format!("rate:generated:{salt}");
        let election_key = format!("election:generated:{salt}");
        let barrier_key = format!("barrier:generated:{salt}");
        let mut expected_queue = Vec::<String>::new();

        let lock_acquire = request(
            SERVICE_LOCK,
            OP_ACQUIRE,
            &lock_key,
            "lock-owner",
            next_trace_sequence(&mut sequence),
            None,
        );
        let first_lock = apply_generated_request(&mut runtime, &lock_acquire, COORDINATION_DECISION_PASS);
        let lock_token = first_lock.token.as_ref().expect("lock token").token;
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let before_duplicate_ref = state_ref(&runtime);
        let before_receipts = runtime.receipts.len();
        let before_applied = runtime.applied_operations.len();
        let duplicate_lock = apply_generated_request(&mut runtime, &lock_acquire, COORDINATION_DECISION_PASS);
        assert_eq!(duplicate_lock.receipt.transition_kind, TRANSITION_KIND_DUPLICATE_REPLAY);
        assert_eq!(duplicate_lock.receipt.prior_receipt_ref.as_deref(), Some(first_lock.receipt.receipt_ref.as_str()));
        assert_eq!(duplicate_lock.receipt.preserved_state_ref.as_deref(), Some(before_duplicate_ref.as_str()));
        assert_eq!(state_ref(&runtime), before_duplicate_ref);
        assert_eq!(
            runtime.receipts.len(),
            before_receipts.saturating_add(usize::try_from(MIN_FENCING_TOKEN).expect("fencing token fits usize"))
        );
        assert_eq!(runtime.applied_operations.len(), before_applied);
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let stale_release = request(
            SERVICE_LOCK,
            OP_RELEASE,
            &lock_key,
            "lock-owner",
            next_trace_sequence(&mut sequence),
            Some(record("token", vec![u64_value(0)])),
        );
        apply_generated_denial(&mut runtime, &stale_release, "stale fencing token");
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let release = request(
            SERVICE_LOCK,
            OP_RELEASE,
            &lock_key,
            "lock-owner",
            next_trace_sequence(&mut sequence),
            Some(record("token", vec![u64_value(lock_token)])),
        );
        apply_generated_request(&mut runtime, &release, COORDINATION_DECISION_PASS);
        assert!(!runtime.state.locks.contains_key(&lock_key));
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let enqueue_first = request(
            SERVICE_QUEUE,
            OP_ENQUEUE,
            &queue_key,
            "queue-producer",
            next_trace_sequence(&mut sequence),
            Some(record("item", vec![string(COORDINATION_TRACE_FIRST_ITEM)])),
        );
        apply_generated_request(&mut runtime, &enqueue_first, COORDINATION_DECISION_PASS);
        expected_queue.push(COORDINATION_TRACE_FIRST_ITEM.to_string());
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let enqueue_second = request(
            SERVICE_QUEUE,
            OP_ENQUEUE,
            &queue_key,
            "queue-producer",
            next_trace_sequence(&mut sequence),
            Some(record("item", vec![string(COORDINATION_TRACE_SECOND_ITEM)])),
        );
        apply_generated_request(&mut runtime, &enqueue_second, COORDINATION_DECISION_PASS);
        expected_queue.push(COORDINATION_TRACE_SECOND_ITEM.to_string());
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        let dequeue = request(
            SERVICE_QUEUE,
            OP_DEQUEUE,
            &queue_key,
            "queue-consumer",
            next_trace_sequence(&mut sequence),
            None,
        );
        apply_generated_request(&mut runtime, &dequeue, COORDINATION_DECISION_PASS);
        let removed = expected_queue.remove(QUEUE_FRONT_INDEX);
        assert_eq!(removed, COORDINATION_TRACE_FIRST_ITEM);
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_SEMAPHORE,
                OP_ACQUIRE,
                &semaphore_key,
                "sem-a",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_SEMAPHORE,
                OP_ACQUIRE,
                &semaphore_key,
                "sem-b",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        let semaphore_exhausted = request(
            SERVICE_SEMAPHORE,
            OP_ACQUIRE,
            &semaphore_key,
            "sem-c",
            next_trace_sequence(&mut sequence),
            None,
        );
        apply_generated_denial(&mut runtime, &semaphore_exhausted, "semaphore exhausted");
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_RATE_LIMIT,
                OP_ACQUIRE,
                &rate_key,
                "rate-a",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_RATE_LIMIT,
                OP_ACQUIRE,
                &rate_key,
                "rate-b",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        let rate_exhausted = request(
            SERVICE_RATE_LIMIT,
            OP_ACQUIRE,
            &rate_key,
            "rate-c",
            next_trace_sequence(&mut sequence),
            None,
        );
        apply_generated_denial(&mut runtime, &rate_exhausted, "rate limit exhausted");
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_ELECTION,
                OP_ELECT,
                &election_key,
                "leader-a",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        let second_leader = request(
            SERVICE_ELECTION,
            OP_ELECT,
            &election_key,
            "leader-b",
            next_trace_sequence(&mut sequence),
            None,
        );
        apply_generated_denial(&mut runtime, &second_leader, "already led");
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);

        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_BARRIER,
                OP_ARRIVE,
                &barrier_key,
                "barrier-a",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        assert!(!runtime.state.barriers.get(&barrier_key).expect("barrier").is_released);
        apply_generated_request(
            &mut runtime,
            &request(
                SERVICE_BARRIER,
                OP_ARRIVE,
                &barrier_key,
                "barrier-b",
                next_trace_sequence(&mut sequence),
                None,
            ),
            COORDINATION_DECISION_PASS,
        );
        assert!(runtime.state.barriers.get(&barrier_key).expect("barrier").is_released);
        assert_coordination_invariants(&runtime, &queue_key, &expected_queue);
    }
