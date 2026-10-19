
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

    #[test]
    fn resource_lifecycle_preflights_required_effect_intents() {
        let mut input = ReconcileCompletionInput {
            resource_ref: ref_for("resource"),
            claimed_generation: 1,
            current_generation: 1,
            has_admitted_plan: true,
            has_effect_receipts: vec!["receipt-for-eff".to_string()],
            required_effect_intents: vec!["eff".to_string(); MAX_EFFECT_INTENTS],
            has_status_update: true,
        };
        let exact = validate_reconcile_completion(&input);
        assert!(exact.pass);
        assert!(exact.diagnostics.is_empty());

        input.current_generation = 2;
        input.has_effect_receipts.clear();
        let admitted_invalid = validate_reconcile_completion(&input);
        assert!(!admitted_invalid.pass);
        assert_eq!(admitted_invalid.diagnostics.len(), MAX_EFFECT_INTENTS + 1);
        assert_eq!(admitted_invalid.diagnostics[0], "stale generation: claimed 1 but current is 2");
        assert_eq!(admitted_invalid.diagnostics[1], "missing effect receipt for: eff");

        input.required_effect_intents.push("extra".to_string());
        let excess = validate_reconcile_completion(&input);
        assert!(!excess.pass);
        assert_eq!(excess.diagnostics, ["required effect intent count 129 exceeds maximum 128"]);
    }

    #[test]
    fn resource_lifecycle_preflights_placement_constraints() {
        let preferred = PlacementConstraint {
            kind: ConstraintKind::Preferred,
            key: "zone".to_string(),
            operator: ConstraintOperator::In,
            values: vec!["east".to_string()],
        };
        let mut request = PlacementRequest {
            workload_ref: ref_for("workload"),
            workload_type: "molten.test.actor.v1".to_string(),
            requests: ResourceAmounts { cpu_millis: 1, memory_bytes: 1, storage_bytes: 0, network_mbps: 0 },
            limits: ResourceAmounts { cpu_millis: 1, memory_bytes: 1, storage_bytes: 0, network_mbps: 0 },
            quota_ref: ref_for("quota"),
            priority: 0,
            priority_policy_ref: None,
            constraints: vec![preferred.clone(); MAX_CONSTRAINTS],
            taints: vec![],
            tolerations: vec![],
            target_capacity_evidence: None,
            assignment_authority_ref: ref_for("auth"),
        };
        let exact = evaluate_placement(&request, &[]);
        assert_eq!(exact.decision, "pass");
        assert_eq!(exact.target_ref.as_deref(), Some("target"));
        assert!(exact.diagnostics.is_empty());

        request.constraints[0].kind = ConstraintKind::Required;
        let admitted_invalid = evaluate_placement(&request, &[]);
        assert_eq!(admitted_invalid.decision, "deny");
        assert_eq!(admitted_invalid.diagnostics.len(), 1);
        assert!(admitted_invalid.diagnostics[0].starts_with("required constraint not satisfied: zone"));

        request.constraints.push(preferred);
        let excess = evaluate_placement(&request, &[]);
        assert_eq!(excess.decision, "deny");
        assert_eq!(excess.target_ref, None);
        assert_eq!(excess.diagnostics, ["constraint count 65 exceeds maximum 64"]);
    }

// r[verify molten.lifecycle.gc_cleanup_gates.bounded_facts]
    #[test]
    fn deletion_gate_enforces_fact_bounds_before_deciding_cleanup() {
        let mut input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: (0..MAX_EVIDENCE_REFS).map(|index| ref_for(&format!("pin-{index}"))).collect(),
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };

        let blocked = evaluate_deletion_gate(&input).expect("exactly bounded pins must be evaluated");
        assert_eq!(blocked.decision, "blocked");
        assert_eq!(blocked.unresolved_blockers.len(), MAX_EVIDENCE_REFS);
        input.pin_refs.push(ref_for("one-pin-over-bound"));
        assert!(evaluate_deletion_gate(&input).is_err(), "overbound pins cannot produce a deletion decision");

        input.pin_refs.clear();
        input.finalizers.push("x".repeat(MAX_DELETION_FACT_BYTES));
        input.finalizer_cleanup_receipts.push("x".repeat(MAX_DELETION_FACT_BYTES));
        let ready = evaluate_deletion_gate(&input).expect("exactly bounded finalizer fact must be evaluated");
        assert_eq!(ready.decision, "deletion-ready");
        assert!(ready.unresolved_blockers.is_empty());
        input.finalizers[0].push('x');
        assert!(evaluate_deletion_gate(&input).is_err(), "oversized fact cannot produce a deletion decision");
    }

    // r[verify molten.lifecycle.gc_cleanup_gates.bounded_facts]
    #[test]
    fn deletion_gate_limits_owner_refs_even_without_live_owners() {
        let mut input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: (0..MAX_OWNER_REFS)
                .map(|index| OwnerRef {
                    resource_ref: ref_for(&format!("owner-{index}")),
                    resource_type: "molten.test.parent.v1".to_string(),
                    block_delete_on_deletion: true,
                })
                .collect(),
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };
        assert_eq!(evaluate_deletion_gate(&input).expect("exact owner bound").decision, "deletion-ready");
        input.live_owner_refs.push(input.owner_refs[0].resource_ref.clone());
        assert_eq!(evaluate_deletion_gate(&input).expect("live owner").decision, "blocked");
        input.owner_refs.push(input.owner_refs[0].clone());
        assert!(evaluate_deletion_gate(&input).is_err(), "overbound owners cannot produce a deletion decision");
    }
