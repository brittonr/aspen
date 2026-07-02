    use super::*;

    type RuntimeState = crate::runtime::RuntimeState;
    type RuntimeStep = crate::runtime::RuntimeStep;
    type TestCase = hegel::TestCase;

    const PROPERTY_MAX_COLLECTION_LEN: usize = 4;
    const PROPERTY_MAX_SALT: u64 = 1_000_000;

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
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        let mut state = RuntimeState::new(1);
        let step = RuntimeStep::Assert {
            actor: "svc".into(),
            value,
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let rollback =
            evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Denied).expect("rollback receipt");
        assert_eq!(rollback.decision, PredicateDecision::Pass);

        let (_events, runtime_commit_receipt) =
            state.commit_turn_with_predicate_receipt(turn.clone()).expect("runtime predicate commit");
        assert_eq!(runtime_commit_receipt.decision, PredicateDecision::Pass);
        let after = state.snapshot();
        let commit = evaluate_turn_transition(&before, &turn, &after, TurnOutcome::Committed).expect("commit receipt");
        assert_eq!(commit.decision, PredicateDecision::Pass);
        let stale = evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Committed).expect("stale receipt");
        assert_eq!(stale.decision, PredicateDecision::Deny);
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
