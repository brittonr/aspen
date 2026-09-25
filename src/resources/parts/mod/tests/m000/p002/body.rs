
    #[test]
    fn missing_authority_preflight_denies_commit() {
        let input = AdmissionChainInput {
            operation: ResourceOperation::Update,
            resource_ref: ref_for("resource"),
            candidate_ref: ref_for("candidate"),
            envelope_decode_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("env")] }),
            schema_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("schema")] }),
            authority_preflight_passed: None,
            defaulting_evidence: None,
            mutation_evidence: None,
            final_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("final")] }),
            policy_evidence_gates: vec![ref_for("policy")],
        };
        let result = evaluate_admission_chain(&input);
        assert!(!result.pass, "missing authority preflight should deny");
        assert_eq!(result.phase_results[2].decision, PhaseDecision::Deny);
        for phase_result in &result.phase_results[3..] {
            assert_eq!(phase_result.decision, PhaseDecision::Skip);
        }
    }

    #[test]
    fn missing_mutation_evidence_for_create_denies() {
        let input = AdmissionChainInput {
            operation: ResourceOperation::Create,
            resource_ref: ref_for("resource"),
            candidate_ref: ref_for("candidate"),
            envelope_decode_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("env")] }),
            schema_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("schema")] }),
            authority_preflight_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("authority")] }),
            defaulting_evidence: None,
            mutation_evidence: None,
            final_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("final")] }),
            policy_evidence_gates: vec![ref_for("policy")],
        };
        let result = evaluate_admission_chain(&input);
        assert!(!result.pass, "missing mutation evidence for create should deny");
    }

    #[test]
    fn status_operation_attempting_desired_mutation_denies() {
        let input = StatusOperationInput {
            current_generation: 1,
            proposed_generation: 2,
            changes_desired_ref: true,
            changes_desired_generation: true,
            changes_finalizers: false,
            changes_authority_metadata: false,
            has_status_condition_evidence: true,
        };
        let decision = validate_status_operation(&input);
        assert!(!decision.pass);
        assert!(decision.diagnostics.iter().any(|d| d.contains("desired-state ref")));
        assert!(decision.diagnostics.iter().any(|d| d.contains("desired generation")));
    }

    #[test]
    fn status_operation_cannot_alter_finalizers_or_authority() {
        let input = StatusOperationInput {
            current_generation: 1,
            proposed_generation: 1,
            changes_desired_ref: false,
            changes_desired_generation: false,
            changes_finalizers: true,
            changes_authority_metadata: true,
            has_status_condition_evidence: true,
        };
        let decision = validate_status_operation(&input);
        assert!(!decision.pass);
        assert!(decision.diagnostics.iter().any(|d| d.contains("finalizers")));
        assert!(decision.diagnostics.iter().any(|d| d.contains("authority-bearing")));
    }

    #[test]
    fn status_operation_must_have_condition_evidence() {
        let input = StatusOperationInput {
            current_generation: 1,
            proposed_generation: 1,
            changes_desired_ref: false,
            changes_desired_generation: false,
            changes_finalizers: false,
            changes_authority_metadata: false,
            has_status_condition_evidence: false,
        };
        let decision = validate_status_operation(&input);
        assert!(!decision.pass);
        assert!(decision.diagnostics.iter().any(|d| d.contains("condition evidence")));
    }

    // --- Dataspace watch informers ---

    #[test]
    fn ordered_watch_events_advance_cursor() {
        let events = vec![
            WatchEvent {
                resource_ref: ref_for("resource"),
                resource_type: "molten.test.v1".to_string(),
                scope_ref: ref_for("scope"),
                generation: 1,
                kind: WatchEventKind::Added,
                prior_cursor: RevisionCursor::new(0),
                next_cursor: RevisionCursor::new(1),
                admission_receipt_refs: vec![],
                selector_refs: vec![],
                observer_authority_refs: vec![],
                event_body_ref: ref_for("body-1"),
                evidence_refs: vec![],
            },
            WatchEvent {
                resource_ref: ref_for("resource"),
                resource_type: "molten.test.v1".to_string(),
                scope_ref: ref_for("scope"),
                generation: 2,
                kind: WatchEventKind::Modified,
                prior_cursor: RevisionCursor::new(1),
                next_cursor: RevisionCursor::new(2),
                admission_receipt_refs: vec![],
                selector_refs: vec![],
                observer_authority_refs: vec![],
                event_body_ref: ref_for("body-2"),
                evidence_refs: vec![],
            },
        ];
        let refs = validate_watch_events(&events).expect("ordered events");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn cursor_gap_denies() {
        let events = vec![
            WatchEvent {
                resource_ref: ref_for("resource"),
                resource_type: "molten.test.v1".to_string(),
                scope_ref: ref_for("scope"),
                generation: 1,
                kind: WatchEventKind::Added,
                prior_cursor: RevisionCursor::new(0),
                next_cursor: RevisionCursor::new(1),
                admission_receipt_refs: vec![],
                selector_refs: vec![],
                observer_authority_refs: vec![],
                event_body_ref: ref_for("body-1"),
                evidence_refs: vec![],
            },
            WatchEvent {
                resource_ref: ref_for("resource"),
                resource_type: "molten.test.v1".to_string(),
                scope_ref: ref_for("scope"),
                generation: 2,
                kind: WatchEventKind::Modified,
                prior_cursor: RevisionCursor::new(3),
                next_cursor: RevisionCursor::new(4),
                admission_receipt_refs: vec![],
                selector_refs: vec![],
                observer_authority_refs: vec![],
                event_body_ref: ref_for("body-2"),
                evidence_refs: vec![],
            },
        ];
        assert!(validate_watch_events(&events).is_err());
    }

    #[test]
    fn informer_snapshot_validates_consistency() {
        let events = vec![WatchEvent {
            resource_ref: ref_for("resource"),
            resource_type: "molten.test.v1".to_string(),
            scope_ref: ref_for("scope"),
            generation: 1,
            kind: WatchEventKind::Added,
            prior_cursor: RevisionCursor::new(0),
            next_cursor: RevisionCursor::new(1),
            admission_receipt_refs: vec![],
            selector_refs: vec![],
            observer_authority_refs: vec![],
            event_body_ref: ref_for("body"),
            evidence_refs: vec![],
        }];
        let snapshot = InformerSnapshot {
            initial_list_ref: ref_for("list"),
            starting_cursor: RevisionCursor::new(0),
            applied_watch_event_refs: vec![ref_for("body")],
            final_cursor: RevisionCursor::new(1),
            selector_refs: vec![],
            observer_authority_refs: vec![],
            cache_state_ref: ref_for("cache"),
        };
        let input = InformerValidationInput {
            initial_list_ref: ref_for("list"),
            starting_cursor: RevisionCursor::new(0),
            watch_events: events,
            final_cursor: RevisionCursor::new(1),
            snapshot,
        };
        let result = validate_informer_snapshot(&input);
        assert!(result.pass);
        assert!(result.cache_current);
    }

    #[test]
    fn cross_scope_selector_denied_without_authority() {
        let selector = WatchSelector {
            scope_ref: ref_for("scope"),
            resource_types: vec!["molten.test.v1".to_string()],
            label_selectors: vec![],
            field_selectors: vec![],
            is_cross_scope: true,
        };
        assert!(validate_selector_authority(&selector, false, &[]).is_err());
    }

    // --- Placement governance ---

    #[test]
    fn placement_fits_admitted_capacity() {
        let request = PlacementRequest {
            workload_ref: ref_for("workload"),
            workload_type: "molten.test.actor.v1".to_string(),
            requests: ResourceAmounts { cpu_millis: 100, memory_bytes: 1024, storage_bytes: 0, network_mbps: 0 },
            limits: ResourceAmounts { cpu_millis: 200, memory_bytes: 2048, storage_bytes: 0, network_mbps: 0 },
            quota_ref: ref_for("quota"),
            priority: 0,
            priority_policy_ref: None,
            constraints: vec![],
            taints: vec![],
            tolerations: vec![],
            target_capacity_evidence: Some(CapacityEvidence {
                target_ref: ref_for("target"),
                available_cpu_millis: 500,
                available_memory_bytes: 4096,
                available_storage_bytes: 10000,
                available_network_mbps: 100,
                evidence_refs: vec![ref_for("capacity")],
            }),
            assignment_authority_ref: ref_for("auth"),
        };
        let decision = evaluate_placement_fit(&request).expect("placement fit");
        assert_eq!(decision.decision, "pass");
    }

    #[test]
    fn over_quota_placement_denies() {
        let request = PlacementRequest {
            workload_ref: ref_for("workload"),
            workload_type: "molten.test.actor.v1".to_string(),
            requests: ResourceAmounts { cpu_millis: 1000, memory_bytes: 10000, storage_bytes: 0, network_mbps: 0 },
            limits: ResourceAmounts { cpu_millis: 2000, memory_bytes: 20000, storage_bytes: 0, network_mbps: 0 },
            quota_ref: ref_for("quota"),
            priority: 0,
            priority_policy_ref: None,
            constraints: vec![],
            taints: vec![],
            tolerations: vec![],
            target_capacity_evidence: Some(CapacityEvidence {
                target_ref: ref_for("target"),
                available_cpu_millis: 100,
                available_memory_bytes: 512,
                available_storage_bytes: 0,
                available_network_mbps: 0,
                evidence_refs: vec![ref_for("capacity")],
            }),
            assignment_authority_ref: ref_for("auth"),
        };
        let decision = evaluate_placement_fit(&request).expect("placement fit");
        assert_eq!(decision.decision, "deny");
    }

    #[test]
    fn tainted_target_without_toleration_denied() {
        let request = PlacementRequest {
            workload_ref: ref_for("workload"),
            workload_type: "molten.test.actor.v1".to_string(),
            requests: ResourceAmounts { cpu_millis: 100, memory_bytes: 512, storage_bytes: 0, network_mbps: 0 },
            limits: ResourceAmounts { cpu_millis: 200, memory_bytes: 1024, storage_bytes: 0, network_mbps: 0 },
            quota_ref: ref_for("quota"),
            priority: 0,
            priority_policy_ref: None,
            constraints: vec![],
            taints: vec![],
            tolerations: vec![],
            target_capacity_evidence: None,
            assignment_authority_ref: ref_for("auth"),
        };
        let props = vec![("taint.no-schedule".to_string(), "production".to_string())];
        let decision = evaluate_placement(&request, &props);
        assert_eq!(decision.decision, "deny");
    }
