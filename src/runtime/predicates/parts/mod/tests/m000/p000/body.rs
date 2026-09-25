    use super::*;

    type RuntimeEvent = crate::runtime::RuntimeEvent;
    type RuntimeState = crate::runtime::RuntimeState;
    type RuntimeStep = crate::runtime::RuntimeStep;
    type TestCase = hegel::TestCase;

    const PROPERTY_MAX_COLLECTION_LEN: usize = 4;
    const PROPERTY_MAX_SALT: u64 = 1_000_000;
    const TURN_COMMIT_TEST_SEED: u64 = 1;
    const TURN_RECEIPT_REQUIRED_CHECKS: &[&str] = &[
        "trellis-bounded-turn-delta",
        "pending-actions-invisible-before-commit",
        "atomic-commit",
        "rollback-preserves-committed-state",
        "turn-event-refs-bound",
    ];

    fn draw_property_salt(tc: &TestCase) -> u64 {
        tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_MAX_SALT))
    }

    fn draw_property_collection_len(tc: &TestCase) -> usize {
        tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(PROPERTY_MAX_COLLECTION_LEN))
    }

    fn draw_property_bool(tc: &TestCase) -> bool {
        tc.draw(hegel::generators::booleans())
    }

    fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
        refs.sort();
        refs
    }

    fn deterministic_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("deterministic ref")
    }

    fn property_ref(label: &str, salt: u64, index: usize) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(format!("{label}-{salt}-{index}")))
            .expect("property ref")
    }

    fn property_refs(label: &str, salt: u64, count: usize) -> Vec<String> {
        let mut refs = (0..count).map(|index| property_ref(label, salt, index)).collect::<Vec<_>>();
        refs.sort();
        refs
    }

    fn assert_turn_receipt_binds_transition(
        receipt: &RuntimePredicateReceipt,
        before: &RuntimeSnapshot,
        turn: &PendingTurn,
        after: &RuntimeSnapshot,
        outcome: TurnOutcome,
        decision: PredicateDecision,
    ) {
        let before_ref = before.snapshot_ref().expect("before snapshot ref");
        let after_ref = after.snapshot_ref().expect("after snapshot ref");
        assert_eq!(receipt.state_refs, vec![before_ref, after_ref]);
        assert_eq!(
            receipt.input_ref,
            turn_transition_input_ref(before, turn, after, outcome).expect("turn transition input ref")
        );
        assert_eq!(receipt.decision, decision);
        for required_check in TURN_RECEIPT_REQUIRED_CHECKS.iter().copied() {
            assert!(
                receipt.checks.iter().any(|check| check == required_check),
                "missing turn receipt check {required_check}"
            );
        }
        crate::preserves_rail::validate_content_ref(&receipt.input_ref).expect("input ref");
        crate::preserves_rail::validate_content_ref(&receipt.receipt_ref).expect("receipt ref");
        assert_eq!(
            crate::preserves_rail::canonical_hash(&receipt.value).expect("receipt value ref"),
            receipt.receipt_ref
        );
    }

    #[test]
    fn assertion_visibility_preserves_duplicates_until_final_owner() {
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-b".into(),
            value: ready.clone(),
        });
        let mut live = OrderedSet::new();
        live.insert("owner-a".to_string());
        live.insert("owner-b".to_string());
        let both = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert!(both.is_visible);
        assert_eq!(both.visible_owner_refs.len(), 2);
        crate::preserves_rail::validate_content_ref(&both.receipt.receipt_ref).expect("receipt ref");

        state.apply_step(&RuntimeStep::Retract {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        let one = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert!(one.is_visible);
        assert_eq!(one.visible_owner_refs.len(), 1);

        state.apply_step(&RuntimeStep::Retract {
            actor: "owner-b".into(),
            value: ready.clone(),
        });
        let none = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert_eq!(none.visible_owner_refs.len(), 0);
        assert!(!none.is_visible);
    }

    #[test]
    fn turn_commit_and_rollback_predicates_bind_state_transition() {
        // r[verify molten.runtime_state_machine_proof.turn_commit_delta]
        // r[verify molten.runtime_state_machine_proof.turn_rollback_no_mutation]
        // r[verify molten.runtime_state_machine_proof.turn_predicate_receipts]
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        let message_body = RuntimeValue::string("service.payload").expect("message body");
        let mut state = RuntimeState::new(TURN_COMMIT_TEST_SEED);

        let assert = commit_bound_turn(&mut state, &RuntimeStep::Assert {
            actor: "svc".into(),
            value: ready.clone(),
        });
        assert!(matches!(assert.events.as_slice(), [RuntimeEvent::AssertionCommitted { .. }]));
        let direct_assert = evaluate_turn_transition(&assert.before, &assert.turn, &assert.after, TurnOutcome::Committed)
            .expect("direct assert commit receipt");
        assert_eq!(assert.receipt.receipt_ref, direct_assert.receipt_ref);

        let observe = commit_bound_turn(&mut state, &RuntimeStep::Observe {
            actor: "watcher".into(),
            pattern: ready.clone(),
        });
        assert!(observe.events.iter().any(|event| matches!(event, RuntimeEvent::ObserveRegistered { .. })));
        assert!(observe.events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));

        let send = commit_bound_turn(&mut state, &RuntimeStep::Send {
            from: "svc".into(),
            to: "watcher".into(),
            body: message_body,
        });
        assert!(matches!(send.events.as_slice(), [RuntimeEvent::MessageDelivered { .. }]));

        let retract = commit_bound_turn(&mut state, &RuntimeStep::Retract {
            actor: "svc".into(),
            value: ready.clone(),
        });
        assert!(retract.events.iter().any(|event| matches!(event, RuntimeEvent::AssertionRetracted { .. })));
        assert!(
            retract
                .events
                .iter()
                .any(|event| matches!(event, RuntimeEvent::AssertionRetractionObserved { .. }))
        );

        let denied_step = RuntimeStep::Assert {
            actor: "denied".into(),
            value: RuntimeValue::string("denied.pending").expect("denied value"),
        };
        let before_denied = state.snapshot();
        let denied_turn = state.begin_turn(&denied_step);
        let (rollback_events, rollback_receipt) = state
            .rollback_turn_with_predicate_receipt(denied_turn.clone(), denied_step.primary_actor(), "policy denied")
            .expect("runtime predicate rollback");
        assert_eq!(state.snapshot(), before_denied);
        assert!(matches!(rollback_events.as_slice(), [RuntimeEvent::TurnRolledBack { .. }]));
        assert_turn_receipt_binds_transition(
            &rollback_receipt,
            &before_denied,
            &denied_turn,
            &before_denied,
            TurnOutcome::Denied,
            PredicateDecision::Pass,
        );
        assert_eq!(rolled_back_turn_snapshot(&before_denied), before_denied);
        assert_unchanged_turn_outcomes(&before_denied, &denied_turn);
    }

    /// The snapshots, pending turn, events, and predicate receipt of one committed turn.
    struct CommittedTurn {
        before: RuntimeSnapshot,
        turn: PendingTurn,
        after: RuntimeSnapshot,
        events: Vec<RuntimeEvent>,
        receipt: RuntimePredicateReceipt,
    }

    /// Stages and commits `step`; staging leaves the state unchanged, the commit reaches the expected snapshot, and
    /// the receipt binds the committed transition.
    fn commit_bound_turn(state: &mut RuntimeState, step: &RuntimeStep) -> CommittedTurn {
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        assert_eq!(state.snapshot(), before);
        let after = committed_turn_snapshot(&before, &turn);
        let (events, receipt) = state.commit_turn_with_predicate_receipt(turn.clone()).expect("runtime predicate commit");
        assert_eq!(state.snapshot(), after);
        assert_turn_receipt_binds_transition(
            &receipt,
            &before,
            &turn,
            &after,
            TurnOutcome::Committed,
            PredicateDecision::Pass,
        );
        CommittedTurn {
            before,
            turn,
            after,
            events,
            receipt,
        }
    }

    /// Failed and rolled-back outcomes over an unchanged snapshot pass, and a committed outcome over it is stale.
    fn assert_unchanged_turn_outcomes(before: &RuntimeSnapshot, turn: &PendingTurn) {
        for outcome in [TurnOutcome::Failed, TurnOutcome::RolledBack] {
            let receipt = evaluate_turn_transition(before, turn, before, outcome).expect("unchanged turn receipt");
            assert_turn_receipt_binds_transition(&receipt, before, turn, before, outcome, PredicateDecision::Pass);
        }
        let stale = evaluate_turn_transition(before, turn, before, TurnOutcome::Committed).expect("stale receipt");
        assert_turn_receipt_binds_transition(&stale, before, turn, before, TurnOutcome::Committed, PredicateDecision::Deny);
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic == "turn-transition-state-mismatch"));
    }

    #[test]
    fn bounded_pattern_matching_is_deterministic() {
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        let exact = evaluate_pattern_match(&RuntimePattern::exact(value.clone()), &value).expect("exact match");
        assert!(exact.is_match);
        assert_eq!(exact.bindings.len(), 0);
        let wildcard = evaluate_pattern_match(&RuntimePattern::wildcard("subject"), &value).expect("wildcard");
        assert!(wildcard.is_match);
        assert_eq!(wildcard.bindings, vec![("subject".to_string(), value.value_ref().to_string())]);
    }

    #[test]
    fn observe_initial_delivery_identifies_current_visible_assertions() {
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        let other = RuntimeValue::string("service.other").expect("runtime value");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-b".into(),
            value: other,
        });
        let observer = RuntimeObserver {
            actor: "watcher".to_string(),
            pattern: ready,
        };
        let result = evaluate_observe_initial_delivery(&state.snapshot(), &observer).expect("delivery");
        assert_eq!(result.delivered_assertion_refs.len(), 1);
        assert_eq!(result.receipt.decision, PredicateDecision::Pass);
    }

    #[test]
    fn promise_state_predicate_enforces_terminal_and_causal_rules() {
        let value_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("resolved-value")).expect("value ref");
        let cause_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("upstream-promise"))
            .expect("cause ref");
        let pending = RuntimePromiseState::pending("promise-1");
        let resolved = RuntimePromiseState::resolved("promise-1", value_ref.clone());
        let pass = evaluate_promise_state_transition(&pending, &resolved).expect("promise transition");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let changed_terminal = RuntimePromiseState::broken("promise-1", "late failure", vec![cause_ref.clone()]);
        let terminal = evaluate_promise_state_transition(&resolved, &changed_terminal).expect("terminal transition");
        assert!(!terminal.is_allowed);
        assert_eq!(terminal.receipt.decision, PredicateDecision::Deny);
        assert!(terminal.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "terminal-promise-state-changed"));

        let mut unsorted_causes = vec![
            cause_ref,
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("aaa")).expect("second cause"),
        ];
        unsorted_causes.sort();
        unsorted_causes.reverse();
        let unsorted_broken = RuntimePromiseState::broken("promise-2", "causal failure", unsorted_causes);
        let causal = evaluate_promise_state_transition(&RuntimePromiseState::pending("promise-2"), &unsorted_broken)
            .expect("causal transition");
        assert!(!causal.is_allowed);
        assert!(
            causal
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "after-causal-failure-refs-not-sorted-unique")
        );
    }

    #[test]
    fn promise_pipeline_predicate_bounds_order_and_cleanup() {
        let target_a =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("target-a")).expect("target a");
        let target_b =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("target-b")).expect("target b");
        let pending = RuntimePromiseState::pending("promise-pipeline");
        let pipeline = RuntimePromisePipelineState::new(pending.clone(), 2, vec![
            RuntimePromisePipelineEntry::new(1, target_a.clone(), "get:field"),
            RuntimePromisePipelineEntry::new(2, target_b.clone(), "call:method"),
        ]);
        let pass = evaluate_promise_pipeline(&pipeline).expect("pipeline predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let over_bound = RuntimePromisePipelineState::new(pending, 1, vec![
            RuntimePromisePipelineEntry::new(2, target_a.clone(), "second"),
            RuntimePromisePipelineEntry::new(1, "not-a-ref", "first"),
        ]);
        let denied = evaluate_promise_pipeline(&over_bound).expect("denied pipeline predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "pipeline-queue-bound-exceeded"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "pipeline-forwarding-order-violation")
        );
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "pipeline-target-ref-noncanonical"));

        let resolved = RuntimePromiseState::resolved("promise-pipeline", target_b);
        let stale = RuntimePromisePipelineState::new(resolved, 2, vec![RuntimePromisePipelineEntry::new(
            3,
            target_a,
            "late-forward",
        )]);
        let cleanup = evaluate_promise_pipeline(&stale).expect("cleanup predicate");
        assert!(!cleanup.is_allowed);
        assert!(
            cleanup
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "terminal-promise-pipeline-not-cleaned")
        );
    }

    #[test]
    fn promise_use_predicate_denies_unresolved_value_without_pipeline_proof() {
        // r[verify molten.vat_ref_state_proof.promise_lifecycle]
        let value_ref = deterministic_ref("promise-use-value");
        let call_ref = deterministic_ref("promise-use-dependent-call");
        let pipeline_ref = deterministic_ref("promise-use-pipeline-proof");
        let pending = RuntimePromiseState::pending("promise-use");
        let resolved = RuntimePromiseState::resolved("promise-use", value_ref.clone());

        let resolved_use = evaluate_promise_use(&RuntimePromiseUseState {
            source: resolved,
            use_kind: RuntimePromiseUseKind::ResolvedValue,
            dependent_call_ref: call_ref.clone(),
            admitted_resolution_ref: Some(value_ref),
            admitted_pipeline_ref: None,
        })
        .expect("resolved promise use");
        assert!(resolved_use.is_allowed);
        assert_eq!(resolved_use.receipt.decision, PredicateDecision::Pass);

        let unresolved_use = evaluate_promise_use(&RuntimePromiseUseState {
            source: pending.clone(),
            use_kind: RuntimePromiseUseKind::ResolvedValue,
            dependent_call_ref: call_ref.clone(),
            admitted_resolution_ref: None,
            admitted_pipeline_ref: None,
        })
        .expect("unresolved promise use");
        assert!(!unresolved_use.is_allowed);
        assert!(
            unresolved_use
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "promise-use-requires-resolved-source")
        );
        assert!(
            unresolved_use
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "promise-use-resolution-proof-missing")
        );

        let forwarded_use = evaluate_promise_use(&RuntimePromiseUseState {
            source: pending.clone(),
            use_kind: RuntimePromiseUseKind::PipelineForward,
            dependent_call_ref: call_ref.clone(),
            admitted_resolution_ref: None,
            admitted_pipeline_ref: Some(pipeline_ref),
        })
        .expect("forwarded promise use");
        assert!(forwarded_use.is_allowed);

        let missing_pipeline = evaluate_promise_use(&RuntimePromiseUseState {
            source: pending,
            use_kind: RuntimePromiseUseKind::PipelineForward,
            dependent_call_ref: call_ref,
            admitted_resolution_ref: None,
            admitted_pipeline_ref: None,
        })
        .expect("missing pipeline proof");
        assert!(!missing_pipeline.is_allowed);
        assert!(
            missing_pipeline
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "promise-use-pipeline-proof-missing")
        );
    }

    #[test]
    fn revocation_cleanup_predicate_denies_future_use_and_requires_cleanup() {
        let revoked =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("revoked-ref")).expect("revoked ref");
        let live_assertion = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("live-assertion"))
            .expect("live assertion");
        let live_subscription =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("live-subscription"))
                .expect("live subscription");
        let live_call =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("live-call")).expect("live call");
        let live_child =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("live-child")).expect("live child");
        let pass_state = RuntimeRevocationCleanupState {
            revoked_refs: vec![revoked.clone()],
            attempted_use_refs: Vec::new(),
            remaining_assertion_refs: vec![live_assertion],
            remaining_subscription_refs: vec![live_subscription],
            remaining_pending_call_refs: vec![live_call],
            remaining_child_refs: vec![live_child],
        };
        let pass = evaluate_revocation_cleanup(&pass_state).expect("revocation cleanup predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let denied_state = RuntimeRevocationCleanupState {
            revoked_refs: vec![revoked.clone()],
            attempted_use_refs: vec![revoked.clone()],
            remaining_assertion_refs: vec![revoked.clone()],
            remaining_subscription_refs: vec![revoked.clone()],
            remaining_pending_call_refs: vec![revoked.clone()],
            remaining_child_refs: vec![revoked],
        };
        let denied = evaluate_revocation_cleanup(&denied_state).expect("denied cleanup predicate");
        assert!(!denied.is_allowed);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-ref-used-after-revocation")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-dependent-assertion-not-cleaned")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-dependent-subscription-not-cleaned")
        );
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "revoked-pending-call-not-cleaned"));
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "revoked-child-ref-not-cleaned"));
    }
