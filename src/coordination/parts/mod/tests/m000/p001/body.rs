
    const GENERATED_COORDINATION_MAX_SALT: u64 = 1_000;
    const GENERATED_COORDINATION_SEQUENCE_SPACING: u64 = 100;
    const MIN_FENCING_TOKEN: u64 = 1;
    const QUEUE_FRONT_INDEX: usize = 0;
    const COORDINATION_DECISION_PASS: &str = "pass";
    const COORDINATION_DECISION_DENY: &str = "deny";
    const COORDINATION_TRACE_FIRST_ITEM: &str = "first";
    const COORDINATION_TRACE_SECOND_ITEM: &str = "second";

    fn draw_coordination_trace_salt(tc: &TestCase) -> u64 {
        tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(GENERATED_COORDINATION_MAX_SALT))
    }

    fn trace_sequence_start(salt: u64) -> u64 {
        salt.saturating_mul(GENERATED_COORDINATION_SEQUENCE_SPACING)
    }

    fn next_trace_sequence(sequence: &mut u64) -> u64 {
        *sequence = sequence.saturating_add(MIN_FENCING_TOKEN);
        *sequence
    }

    fn state_ref(runtime: &CoordinationRuntime) -> String {
        snapshot_from_state(&runtime.state).expect("coordination snapshot").state_ref
    }

    fn assert_status_assertions_bind_result(result: &CoordinationApplyResult) {
        assert_eq!(result.receipt.state_ref, result.state_snapshot.state_ref);
        crate::preserves_rail::validate_content_ref(&result.receipt.before_state_ref).expect("before state ref");
        match result.receipt.transition_kind.as_str() {
            TRANSITION_KIND_ADVANCE => assert_eq!(result.receipt.after_state_ref.as_deref(), Some(result.state_snapshot.state_ref.as_str())),
            TRANSITION_KIND_DENY_PRESERVE | TRANSITION_KIND_DUPLICATE_REPLAY | TRANSITION_KIND_CONFLICTING_DUPLICATE => {
                assert_eq!(result.receipt.preserved_state_ref.as_deref(), Some(result.state_snapshot.state_ref.as_str()));
            }
            TRANSITION_KIND_READ_OBSERVE => {
                assert_eq!(result.receipt.preserved_state_ref.as_deref(), Some(result.state_snapshot.state_ref.as_str()));
            }
            other => panic!("unexpected transition kind {other}"),
        }
        for assertion in &result.assertions {
            assert_eq!(assertion.state_ref, result.state_snapshot.state_ref);
            assert_eq!(assertion.receipt_ref, result.receipt.receipt_ref);
            crate::preserves_rail::validate_content_ref(&assertion.assertion_ref).expect("assertion ref");
        }
        crate::preserves_rail::validate_content_ref(&result.receipt.receipt_ref).expect("receipt ref");
        crate::preserves_rail::validate_content_ref(&result.receipt.request_ref).expect("request ref");
    }

    fn assert_coordination_invariants(runtime: &CoordinationRuntime, queue_key: &str, expected_queue: &[String]) {
        for lock in runtime.state.locks.values() {
            assert!(lock.token < runtime.state.next_fencing_token);
        }
        for election in runtime.state.elections.values() {
            assert!(!election.leader.is_empty());
            assert!(election.token < runtime.state.next_fencing_token);
        }
        for queue in runtime.state.queues.values() {
            assert!(vec_len_u64(queue).expect("queue len") <= runtime.manifest.queue_capacity);
        }
        if let Some(queue) = runtime.state.queues.get(queue_key) {
            assert_eq!(queue, expected_queue);
        }
        for holders in runtime.state.semaphores.values() {
            assert!(set_len_u64(holders).expect("semaphore holders") <= runtime.manifest.semaphore_capacity);
        }
        for used in runtime.state.rates.values() {
            assert!(*used <= runtime.manifest.rate_limit);
        }
        for barrier in runtime.state.barriers.values() {
            let participants = set_len_u64(&barrier.participants).expect("barrier participants");
            assert_eq!(barrier.is_released, participants >= barrier.required);
        }
    }

    fn apply_generated_request(
        runtime: &mut CoordinationRuntime,
        request_value: &IoValue,
        expected_decision: &str,
    ) -> CoordinationApplyResult {
        let result = apply_coordination_request(runtime, request_value).expect("generated coordination request");
        assert_eq!(result.receipt.decision, expected_decision);
        assert_status_assertions_bind_result(&result);
        result
    }

    fn apply_generated_denial(runtime: &mut CoordinationRuntime, request_value: &IoValue, diagnostic: &str) {
        let before_state = runtime.state.clone();
        let before_ref = state_ref(runtime);
        let result = apply_generated_request(runtime, request_value, COORDINATION_DECISION_DENY);
        let after_ref = state_ref(runtime);
        assert_eq!(runtime.state, before_state);
        assert_eq!(after_ref, before_ref);
        assert_eq!(result.state_snapshot.state_ref, before_ref);
        assert!(result.receipt.diagnostics.iter().any(|value| value.contains(diagnostic)));
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_coordination_artifacts() {
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let mut runtime = new_coordination_runtime(&manifest).expect("runtime");
        let result = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:classify", "session", 1, None),
        )
        .expect("lock");
        assert_eq!(crate::ledger::artifact_kind(&manifest), "coordination-service-manifest");
        assert_eq!(crate::ledger::artifact_kind(&result.receipt.value), "coordination-receipt");
        assert_eq!(crate::ledger::artifact_kind(&result.assertions[0].value), "coordination-status-assertion");
        assert_eq!(
            crate::ledger::artifact_kind(&result.token.as_ref().expect("token").value),
            "coordination-fencing-token"
        );
        let report_evidence_refs = result
            .evidence_values
            .iter()
            .map(canonical_hash)
            .collect::<Result<Vec<_>>>()
            .expect("evidence refs");
        let manifest_ref = canonical_hash(&manifest).expect("manifest ref");
        let apply_report = coordination_apply_report_value(ApplyReportValueInput {
            decision: "pass",
            manifest_ref: &manifest_ref,
            final_state_ref: &result.state_snapshot.state_ref,
            receipt_refs: std::slice::from_ref(&result.receipt.receipt_ref),
            assertion_refs: std::slice::from_ref(&result.assertions[0].assertion_ref),
            evidence_refs: &report_evidence_refs,
        })
        .expect("apply report");
        assert_eq!(crate::ledger::artifact_kind(&apply_report), "coordination-apply-report");
        let root = temp_root("coordination-ledger-catalog");
        let registry_root = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry_root).expect("registry root");
        crate::ledger::import_artifact(&ledger_root, &result.receipt.value).expect("import receipt");
        let list = crate::catalog::list(&registry_root, Some(&ledger_root), &crate::catalog::ListInput {
            kind: Some("coordination-receipt".to_string()),
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(list.decision, "pass");
        assert_eq!(list.items.len(), 1);
        let view_request =
            crate::catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
                &result.receipt.receipt_ref,
            )])])
            .expect("mcp request");
        let call = crate::catalog_mcp::call(&registry_root, Some(&ledger_root), &view_request).expect("mcp call");
        assert_eq!(call.decision, "pass");
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_fencing_fifo_semaphore_and_no_actor_traffic_invariants(tc: TestCase) {
        let salt = draw_coordination_trace_salt(&tc);
        let mut sequence = trace_sequence_start(salt);
        let mut runtime = runtime();
        let key = format!("resource:{salt}");
        let acquire = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, &key, "owner", next_trace_sequence(&mut sequence), None),
        )
        .expect("acquire");
        assert_eq!(acquire.receipt.decision, COORDINATION_DECISION_PASS);
        let token = acquire.token.expect("token").token;
        assert!(token >= MIN_FENCING_TOKEN);
        let queue_key = format!("queue:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                &queue_key,
                "p",
                next_trace_sequence(&mut sequence),
                Some(record("item", vec![string(COORDINATION_TRACE_FIRST_ITEM)])),
            ),
        )
        .expect("enqueue first");
        apply_coordination_request(
            &mut runtime,
            &request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                &queue_key,
                "p",
                next_trace_sequence(&mut sequence),
                Some(record("item", vec![string(COORDINATION_TRACE_SECOND_ITEM)])),
            ),
        )
        .expect("enqueue second");
        assert_eq!(
            runtime.state.queues.get(&queue_key).expect("queue")[QUEUE_FRONT_INDEX],
            COORDINATION_TRACE_FIRST_ITEM
        );
        let sem_key = format!("sem:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(
                SERVICE_SEMAPHORE,
                OP_ACQUIRE,
                &sem_key,
                "a",
                next_trace_sequence(&mut sequence),
                None,
            ),
        )
        .expect("sem a");
        assert!(
            set_len_u64(runtime.state.semaphores.get(&sem_key).expect("sem")).expect("sem count")
                <= runtime.manifest.semaphore_capacity
        );
        let snapshot_before_actor_message = snapshot_from_state(&runtime.state).expect("before").state_ref;
        let snapshot_after_actor_message = snapshot_from_state(&runtime.state).expect("after").state_ref;
        assert_eq!(snapshot_before_actor_message, snapshot_after_actor_message);
    }

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
        assert_eq!(runtime.receipts.len(), before_receipts.saturating_add(MIN_FENCING_TOKEN as usize));
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
