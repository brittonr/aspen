
    const GENERATED_TRACE_SEED_OFFSET: u64 = 10_000;
    const GENERATED_TRACE_MIN_LEN: usize = 2;
    const GENERATED_TRACE_ACTION_KIND_COUNT: usize = 4;
    const GENERATED_TRACE_OBSERVE_KIND: usize = 0;
    const GENERATED_TRACE_ASSERT_KIND: usize = 1;
    const GENERATED_TRACE_SEND_KIND: usize = 2;
    const GENERATED_TRACE_OUTCOME_PERIOD: usize = 2;
    const GENERATED_TRACE_ROLLBACK_REASON: &str = "generated-denial";

    fn draw_generated_trace_len(tc: &TestCase) -> usize {
        tc.draw(
            hegel::generators::integers::<usize>()
                .min_value(GENERATED_TRACE_MIN_LEN)
                .max_value(PROPERTY_MAX_COLLECTION_LEN),
        )
    }

    fn generated_trace_step(salt: u64, index: usize) -> RuntimeStep {
        let value = RuntimeValue::string(format!("trace-value-{salt}-{index}")).expect("trace value");
        match index % GENERATED_TRACE_ACTION_KIND_COUNT {
            GENERATED_TRACE_OBSERVE_KIND => RuntimeStep::Observe {
                actor: format!("trace-observer-{salt}"),
                pattern: value,
            },
            GENERATED_TRACE_ASSERT_KIND => RuntimeStep::Assert {
                actor: format!("trace-owner-{salt}"),
                value,
            },
            GENERATED_TRACE_SEND_KIND => RuntimeStep::Send {
                from: format!("trace-sender-{salt}"),
                to: format!("trace-receiver-{salt}"),
                body: value,
            },
            _retract_kind => RuntimeStep::Retract {
                actor: format!("trace-owner-{salt}"),
                value,
            },
        }
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_assertion_turn_and_pattern_predicates_are_bounded_and_deterministic(tc: TestCase) {
        let salt = draw_property_salt(&tc);
        let owner_count = draw_property_collection_len(&tc);
        let retract_count = draw_property_collection_len(&tc);
        let value = RuntimeValue::string(format!("property-ready-{salt}")).expect("runtime value");
        let mut state = RuntimeState::new(salt);
        let mut live = OrderedSet::new();
        for index in 0..owner_count {
            let actor = format!("owner-{salt}-{index}");
            live.insert(actor.clone());
            state.apply_step(&RuntimeStep::Assert {
                actor,
                value: value.clone(),
            });
        }
        for index in 0..std::cmp::min(owner_count, retract_count) {
            state.apply_step(&RuntimeStep::Retract {
                actor: format!("owner-{salt}-{index}"),
                value: value.clone(),
            });
        }
        let visibility = evaluate_assertion_visibility(&state.snapshot(), &value, &live).expect("visibility");
        let expected_visible = owner_count.saturating_sub(std::cmp::min(owner_count, retract_count));
        assert_eq!(visibility.visible_owner_refs.len(), expected_visible);
        assert_eq!(visibility.is_visible, expected_visible > 0);
        assert_eq!(visibility.receipt.decision, PredicateDecision::Pass);

        let mut turn_state = RuntimeState::new(salt.saturating_add(1));
        let before = turn_state.snapshot();
        let step = RuntimeStep::Assert {
            actor: format!("turn-actor-{salt}"),
            value: value.clone(),
        };
        let turn = turn_state.begin_turn(&step);
        let rollback = evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Denied).expect("rollback");
        assert_eq!(rollback.decision, PredicateDecision::Pass);
        let _events = turn_state.commit_turn(turn.clone());
        let after = turn_state.snapshot();
        let commit = evaluate_turn_transition(&before, &turn, &after, TurnOutcome::Committed).expect("commit");
        assert_eq!(commit.decision, PredicateDecision::Pass);
        let stale = evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Committed).expect("stale commit");
        assert_eq!(stale.decision, PredicateDecision::Deny);

        let exact = evaluate_pattern_match(&RuntimePattern::exact(value.clone()), &value).expect("exact pattern");
        let wildcard = evaluate_pattern_match(&RuntimePattern::wildcard("binding"), &value).expect("wildcard pattern");
        let other = RuntimeValue::string(format!("property-other-{salt}")).expect("other runtime value");
        let mismatch = evaluate_pattern_match(&RuntimePattern::exact(other), &value).expect("mismatch pattern");
        assert!(exact.is_match);
        assert!(wildcard.is_match);
        assert_eq!(wildcard.bindings, vec![("binding".to_string(), value.value_ref().to_string())]);
        assert!(!mismatch.is_match);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_mixed_turn_commit_and_rollback_trace_preserves_transition_laws(tc: TestCase) {
        // r[verify molten.runtime_spine.canonical_content_refs.runtime_values]
        // r[verify molten.runtime_state_machine_proof.turn_commit_delta]
        // r[verify molten.runtime_state_machine_proof.turn_rollback_no_mutation]
        // r[verify molten.runtime_state_machine_proof.turn_predicate_receipts]
        // r[verify molten.runtime_state_machine_proof.generated_turn_traces]
        let salt = draw_property_salt(&tc);
        let trace_len = draw_generated_trace_len(&tc);
        let invert_outcomes = draw_property_bool(&tc);
        let seed = salt.saturating_add(GENERATED_TRACE_SEED_OFFSET);
        let mut state = RuntimeState::new(seed);
        let mut replay = RuntimeState::new(seed);

        for index in 0..trace_len {
            let step = generated_trace_step(salt, index);
            let before = state.snapshot();
            let replay_before = replay.snapshot();
            assert_eq!(before, replay_before);
            let turn = state.begin_turn(&step);
            let replay_turn = replay.begin_turn(&step);
            assert_eq!(turn, replay_turn);
            let should_commit = (index % GENERATED_TRACE_OUTCOME_PERIOD == 0) != invert_outcomes;

            if should_commit {
                let expected = expected_turn_snapshot(&before, &turn, TurnOutcome::Committed);
                assert_eq!(expected, committed_turn_snapshot(&before, &turn));
                let (events, receipt) = state
                    .commit_turn_with_predicate_receipt(turn.clone())
                    .expect("generated commit receipt");
                let (replay_events, replay_receipt) = replay
                    .commit_turn_with_predicate_receipt(replay_turn)
                    .expect("generated replay commit receipt");
                assert_eq!(events, replay_events);
                assert_eq!(receipt.receipt_ref, replay_receipt.receipt_ref);
                assert_eq!(state.snapshot(), expected);
                assert_eq!(state.snapshot(), replay.snapshot());
                assert_turn_receipt_binds_transition(
                    &receipt,
                    &before,
                    &turn,
                    &expected,
                    TurnOutcome::Committed,
                    PredicateDecision::Pass,
                );
            } else {
                let expected = expected_turn_snapshot(&before, &turn, TurnOutcome::Denied);
                assert_eq!(expected, rolled_back_turn_snapshot(&before));
                let (events, receipt) = state
                    .rollback_turn_with_predicate_receipt(
                        turn.clone(),
                        step.primary_actor(),
                        GENERATED_TRACE_ROLLBACK_REASON,
                    )
                    .expect("generated rollback receipt");
                let (replay_events, replay_receipt) = replay
                    .rollback_turn_with_predicate_receipt(
                        replay_turn,
                        step.primary_actor(),
                        GENERATED_TRACE_ROLLBACK_REASON,
                    )
                    .expect("generated replay rollback receipt");
                assert_eq!(events, replay_events);
                assert_eq!(receipt.receipt_ref, replay_receipt.receipt_ref);
                assert_eq!(state.snapshot(), expected);
                assert_eq!(state.snapshot(), replay.snapshot());
                assert_turn_receipt_binds_transition(
                    &receipt,
                    &before,
                    &turn,
                    &expected,
                    TurnOutcome::Denied,
                    PredicateDecision::Pass,
                );
            }
        }
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_promise_pipeline_revocation_and_snapshot_predicates_are_monotone(tc: TestCase) {
        let salt = draw_property_salt(&tc);
        let queue_len = draw_property_collection_len(&tc);
        let max_queue = draw_property_collection_len(&tc);
        let mut entries = Vec::with_capacity(queue_len);
        for index in 0..queue_len {
            entries.push(RuntimePromisePipelineEntry::new(
                u64::try_from(index + 1).expect("bounded sequence"),
                property_ref("pipeline-target", salt, index),
                format!("op-{index}"),
            ));
        }
        let pipeline = RuntimePromisePipelineState::new(
            RuntimePromiseState::pending(format!("promise-{salt}")),
            u64::try_from(max_queue).expect("bounded max queue"),
            entries,
        );
        let pipeline_result = evaluate_promise_pipeline(&pipeline).expect("pipeline");
        assert_eq!(pipeline_result.is_allowed, queue_len <= max_queue);
        if queue_len > max_queue {
            assert!(
                pipeline_result
                    .receipt
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic == "pipeline-queue-bound-exceeded")
            );
        }
        let terminal_with_queue = RuntimePromisePipelineState::new(
            RuntimePromiseState::resolved(format!("promise-{salt}"), property_ref("resolved", salt, 0)),
            4,
            vec![RuntimePromisePipelineEntry::new(
                1,
                property_ref("late-target", salt, 0),
                "late",
            )],
        );
        let terminal_result = evaluate_promise_pipeline(&terminal_with_queue).expect("terminal pipeline");
        assert!(!terminal_result.is_allowed);
        assert!(
            terminal_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "terminal-promise-pipeline-not-cleaned")
        );

        let revoked = property_ref("revoked", salt, 0);
        let has_attempted_use = draw_property_bool(&tc);
        let has_remaining_assertion = draw_property_bool(&tc);
        let revocation_state = RuntimeRevocationCleanupState {
            revoked_refs: vec![revoked.clone()],
            attempted_use_refs: if has_attempted_use {
                vec![revoked.clone()]
            } else {
                Vec::new()
            },
            remaining_assertion_refs: if has_remaining_assertion {
                vec![revoked.clone()]
            } else {
                vec![property_ref("live-assertion", salt, 0)]
            },
            remaining_subscription_refs: vec![property_ref("live-subscription", salt, 0)],
            remaining_pending_call_refs: vec![property_ref("live-call", salt, 0)],
            remaining_child_refs: vec![property_ref("live-child", salt, 0)],
        };
        let revocation = evaluate_revocation_cleanup(&revocation_state).expect("revocation");
        assert_eq!(revocation.is_allowed, !has_attempted_use && !has_remaining_assertion);

        let snapshot_ref = property_ref("snapshot", salt, 0);
        let readable = property_refs("snapshot-readable", salt, 2);
        let redacted = property_refs("snapshot-redacted", salt, 2);
        let mut requested = readable.clone();
        requested.extend(redacted.clone());
        requested.sort();
        let snapshot_pass = RuntimeSnapshotAuthorityState {
            snapshot_ref: snapshot_ref.clone(),
            admitted_authority_refs: readable.clone(),
            claimed_authority_refs: readable.clone(),
            requested_assertion_refs: requested,
            readable_assertion_refs: readable.clone(),
            redacted_assertion_refs: redacted,
        };
        let admitted = evaluate_snapshot_authority(&snapshot_pass).expect("snapshot pass");
        assert!(admitted.is_allowed);
        let uncovered = property_ref("snapshot-uncovered", salt, 0);
        let snapshot_denied = RuntimeSnapshotAuthorityState {
            snapshot_ref,
            admitted_authority_refs: readable.clone(),
            claimed_authority_refs: readable.clone(),
            requested_assertion_refs: sorted_refs(vec![readable[0].clone(), uncovered]),
            readable_assertion_refs: readable,
            redacted_assertion_refs: Vec::new(),
        };
        let denied = evaluate_snapshot_authority(&snapshot_denied).expect("snapshot denied");
        assert!(!denied.is_allowed);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "snapshot-requested-assertion-uncovered")
        );
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_service_dependency_and_reference_predicates_fail_closed(tc: TestCase) {
        let salt = draw_property_salt(&tc);
        let dependency_count = draw_property_collection_len(&tc);
        let ready_count = draw_property_collection_len(&tc);
        let service = property_ref("service", salt, 0);
        let dependencies = property_refs("dependency", salt, dependency_count);
        let mut ready =
            dependencies.iter().take(std::cmp::min(dependency_count, ready_count)).cloned().collect::<Vec<_>>();
        ready.push(service.clone());
        ready.sort();
        let service_state = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: dependencies.clone(),
            ready_service_refs: ready,
            failed_service_refs: Vec::new(),
            force_run_refs: Vec::new(),
            restart_refs: Vec::new(),
            reverse_dependency_refs: Vec::new(),
            shutdown_refs: Vec::new(),
        };
        let service_result = evaluate_service_dependencies(&service_state).expect("service dependencies");
        let is_dependencies_ready = ready_count >= dependency_count;
        assert_eq!(service_result.is_allowed, is_dependencies_ready);
        if !is_dependencies_ready {
            assert!(
                service_result
                    .receipt
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic == "service-dependencies-not-ready")
            );
        }

        let failed_dependency = dependencies.first().cloned().unwrap_or_else(|| property_ref("dependency", salt, 99));
        let admitted_failure = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: vec![failed_dependency.clone()],
            ready_service_refs: vec![service.clone()],
            failed_service_refs: vec![failed_dependency.clone()],
            force_run_refs: vec![service.clone()],
            restart_refs: vec![failed_dependency],
            reverse_dependency_refs: Vec::new(),
            shutdown_refs: Vec::new(),
        };
        let force_run = evaluate_service_dependencies(&admitted_failure).expect("force-run dependency");
        assert!(force_run.is_allowed);

        let reference_ref = property_ref("reference", salt, 0);
        let near_sync = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-a".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        assert!(evaluate_near_far_refs(&near_sync).expect("near sync").is_allowed);
        let far_sync = RuntimeNearFarRefState {
            reference_ref,
            reference_kind: RuntimeReferenceKind::Far,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let denied = evaluate_near_far_refs(&far_sync).expect("far sync");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "far-ref-synchronous-call-denied"));
    }
