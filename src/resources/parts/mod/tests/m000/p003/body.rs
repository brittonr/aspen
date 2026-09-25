
    // --- Reconciliation controllers ---

    #[test]
    fn reconcile_noop_when_desired_matches_observed() {
        let same_ref = ref_for("same-state");
        let input = ReconcileInput {
            resource_ref: ref_for("resource"),
            resource_type: "molten.test.v1".to_string(),
            generation: 1,
            desired_state_ref: same_ref.clone(),
            observed_state_summary_ref: Some(same_ref),
            status_summary_ref: None,
            dependency_refs: vec![],
            policy_refs: vec![],
            authority_refs: vec![],
            prior_plan_refs: vec![],
            prior_effect_refs: vec![],
            prior_status_refs: vec![],
            retry_attempt: 0,
            backoff_profile: None,
        };
        let plan = evaluate_reconcile(&input).expect("reconcile");
        assert!(matches!(plan, ReconcilePlan::NoOp { .. }));
    }

    #[test]
    fn reconcile_plans_action_when_observed_missing() {
        let input = ReconcileInput {
            resource_ref: ref_for("resource"),
            resource_type: "molten.test.v1".to_string(),
            generation: 1,
            desired_state_ref: ref_for("desired"),
            observed_state_summary_ref: None,
            status_summary_ref: None,
            dependency_refs: vec![],
            policy_refs: vec![],
            authority_refs: vec![],
            prior_plan_refs: vec![],
            prior_effect_refs: vec![],
            prior_status_refs: vec![],
            retry_attempt: 0,
            backoff_profile: None,
        };
        let plan = evaluate_reconcile(&input).expect("reconcile");
        assert!(matches!(plan, ReconcilePlan::ActionPlan { .. }));
    }

    #[test]
    fn work_queue_coalesces_events() {
        let decision = coalesce_work_queue_item(
            &ref_for("resource"),
            1,
            &["event-1".to_string(), "event-2".to_string()],
        )
        .expect("coalesce");
        assert!(decision.pass);
        let item = decision.item.expect("queue item");
        assert_eq!(item.coalesced_event_refs.len(), 2);
    }

    #[test]
    fn unbounded_retry_denies() {
        let item = WorkQueueItem {
            resource_ref: ref_for("resource"),
            generation: 1,
            causes: vec!["watch".to_string()],
            coalesced_event_refs: vec![],
            retry_attempt: 0,
            backoff_profile: None,
            terminal: false,
            terminal_reason: None,
        };
        let decision = schedule_retry(&item, "default", MAX_BACKOFF_ATTEMPTS + 1).expect("retry schedule");
        assert!(!decision.pass);
    }

    #[test]
    fn reconcile_success_requires_effect_receipts() {
        let input = ReconcileCompletionInput {
            resource_ref: ref_for("resource"),
            claimed_generation: 1,
            current_generation: 1,
            has_admitted_plan: true,
            has_effect_receipts: vec!["receipt-for-eff-1".to_string()],
            required_effect_intents: vec!["eff-1".to_string(), "eff-2".to_string()],
            has_status_update: true,
        };
        let decision = validate_reconcile_completion(&input);
        assert!(!decision.pass);
    }

    #[test]
    fn stale_generation_reconcile_denies() {
        let input = ReconcileCompletionInput {
            resource_ref: ref_for("resource"),
            claimed_generation: 1,
            current_generation: 2,
            has_admitted_plan: true,
            has_effect_receipts: vec![],
            required_effect_intents: vec![],
            has_status_update: true,
        };
        let decision = validate_reconcile_completion(&input);
        assert!(!decision.pass);
    }
