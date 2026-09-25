
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
