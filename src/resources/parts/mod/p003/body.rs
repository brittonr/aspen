#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_consumption_throttle_and_revocation_are_receipted() {
        let grant_value = sample_grant(KIND_EFFECT_CALLS, 2, None).expect("grant");
        let grant = parse_resource_grant(&grant_value).expect("parse grant");
        let first = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("first consume");
        assert_eq!(first.decision, "pass");
        let consumption = parse_consumption(&resource_consumption_value(&grant, 1, 0).expect("consumption"))
            .expect("parse consumption");
        let prior_consumptions = [consumption];
        let second = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &prior_consumptions,
            amount: 2,
            logical_time: 0,
            sequence: 1,
            is_revoked: false,
        })
        .expect("over consume");
        assert_eq!(second.decision, "throttle");
        let revoked = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 2,
            is_revoked: true,
        })
        .expect("revoked consume");
        assert_eq!(revoked.decision, "deny");
    }

    #[test]
    fn mailbox_overflow_is_deterministic_and_not_silent() {
        let first = ref_for("message-1");
        let second = ref_for("message-2");
        let accepted = apply_mailbox_backpressure(&[], &first, 1).expect("accepted");
        assert!(accepted.accepted);
        let denied = apply_mailbox_backpressure(&accepted.queue, &second, 1).expect("overflow");
        assert!(!denied.accepted);
        assert_eq!(denied.overflow, Some(second));
        assert!(crate::preserves_rail::to_text(&denied.receipt_value).expect("receipt").contains("mailbox-full"));
    }

    #[test]
    fn turn_assertion_adapter_and_job_budgets_are_enforced() {
        let turn_grant = sample_grant(KIND_TURNS, 1, None).expect("turn grant");
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 1,
                logical_time: 0,
                sequence: 0,
                is_revoked: false,
            })
            .expect("turn")
            .decision,
            "pass"
        );
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 2,
                logical_time: 0,
                sequence: 1,
                is_revoked: false,
            })
            .expect("turn over")
            .decision,
            "throttle"
        );
        assert_eq!(enforce_assertion_bound(1, 1, &ref_for("assertion")).expect("assertion").decision, "deny");
        assert_eq!(adapter_budget_decision(KIND_CPU_FUEL, 10, 8, "wasmtime-fuel").expect("wasm").decision, "deny");
        assert_eq!(
            adapter_budget_decision(KIND_CPU_FUEL, 4, 8, "steel-native-budget").expect("steel").decision,
            "pass"
        );
        assert_eq!(
            adapter_budget_decision(KIND_BLOB_BYTES, 9, 8, "blob-storage-network").expect("blob").decision,
            "deny"
        );
        assert_eq!(plan_job_stages(&[("a", 1), ("b", 2)], 2).expect("plan"), vec!["place:a:1", "defer:b:2"]);
    }

    #[test]
    fn deterministic_scheduler_is_os_timing_independent() {
        let tasks = vec![
            SchedulerTask {
                actor: "a".to_string(),
                priority: 0,
                sequence: 1,
                budget_class: "normal".to_string(),
            },
            SchedulerTask {
                actor: "b".to_string(),
                priority: 0,
                sequence: 2,
                budget_class: "normal".to_string(),
            },
        ];
        let first = deterministic_schedule(&tasks, 1).expect("schedule");
        let second = deterministic_schedule(&tasks, 1).expect("schedule");
        assert_eq!(first, second);
        assert!(crate::preserves_rail::to_text(&first).expect("schedule text").contains("os-timing-independent"));
    }

    #[test]
    fn expired_grants_deny_future_work_and_receipts_replay() {
        let grant_value = sample_grant(KIND_NETWORK_MESSAGES, 1, Some(5)).expect("grant");
        let before = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 4,
            sequence: 0,
            is_revoked: false,
        })
        .expect("before expiry");
        let after = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("after expiry");
        assert_eq!(before.decision, "pass");
        assert_eq!(after.decision, "deny");
        let replay = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("replay");
        assert_eq!(after.receipt_value, replay.receipt_value);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_budget_monotonicity_queue_bounds_and_no_silent_drop(tc: hegel::TestCase) {
        let amount = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(16));
        let request = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(20));
        let grant_value = sample_grant(KIND_TRACE_BYTES, amount, None).expect("grant");
        let decision = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: request,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("consume");
        if request <= amount {
            assert_eq!(decision.decision, "pass");
        } else {
            assert_eq!(decision.decision, "throttle");
            assert_eq!(decision.consumed, 0);
        }
        let max_slots = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(4));
        let max_slots_usize = usize::try_from(max_slots).expect("bounded max slots");
        let queue = (0..max_slots_usize).map(|index| ref_for(&format!("queued-{index}"))).collect::<Vec<_>>();
        let mailbox = apply_mailbox_backpressure(&queue, &ref_for("new-message"), max_slots).expect("mailbox");
        assert_eq!(mailbox.queue.len(), max_slots_usize);
        assert!(!mailbox.accepted);
        assert!(mailbox.overflow.is_some());
    }

    fn sample_grant(kind: &str, amount: u64, expires_at: Option<u64>) -> Result<IoValue> {
        resource_grant_value(&ResourceGrantInput {
            subject_ref: ref_for("subject"),
            scope: "scope".to_string(),
            kind: kind.to_string(),
            amount,
            rate: None,
            window: None,
            not_before: None,
            expires_at,
            parent_ref: None,
            revocation_refs: Vec::new(),
            policy_refs: vec![ref_for("policy")],
            evidence_refs: vec![ref_for("evidence")],
        })
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("resource-test-ref", vec![string(label)])).expect("test ref")
    }

    // --- Declarative resource records ---

    #[test]
    fn valid_resource_identity_produces_stable_ref() {
        let identity = ResourceIdentity {
            resource_type: "molten.test.service.v1".to_string(),
            scope_ref: ref_for("scope"),
            scoped_name: "my-service".to_string(),
        };
        identity.validate().expect("valid identity");
        let ref1 = identity.canonical_ref().expect("canonical ref");
        let ref2 = identity.canonical_ref().expect("repeat canonical ref");
        assert_eq!(ref1, ref2, "same identity produces same ref during replay");
    }

    #[test]
    fn valid_resource_record_with_metadata() {
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("app".to_string(), "my-app".to_string());
        labels.insert("env".to_string(), "prod".to_string());

        let metadata = ResourceMetadata {
            labels,
            annotations: std::collections::BTreeMap::new(),
            owner_refs: vec![],
            finalizers: vec![],
            evidence_refs: vec![],
        };

        let record = ResourceRecord {
            resource_type: "molten.test.service.v1".to_string(),
            resource_ref: ref_for("resource"),
            scope_ref: ref_for("scope"),
            name: "my-service".to_string(),
            generation: 1,
            desired_ref: ref_for("desired"),
            observed_ref: None,
            metadata,
            evidence_refs: vec![],
        };

        let validated = validate_resource_record(&record).expect("valid resource record");
        assert_eq!(validated.name, "my-service");
        assert_eq!(validated.generation, 1);
    }

    #[test]
    fn valid_resource_record_with_observed_state() {
        let observed_ref = ref_for("observed");
        let record = ResourceRecord {
            resource_type: "molten.test.service.v1".to_string(),
            resource_ref: ref_for("resource"),
            scope_ref: ref_for("scope"),
            name: "my-service".to_string(),
            generation: 1,
            desired_ref: ref_for("desired"),
            observed_ref: Some(observed_ref),
            metadata: ResourceMetadata {
                labels: std::collections::BTreeMap::new(),
                annotations: std::collections::BTreeMap::new(),
                owner_refs: vec![],
                finalizers: vec![],
                evidence_refs: vec![],
            },
            evidence_refs: vec![],
        };
        validate_resource_record(&record).expect("resource with observed state");
    }

    #[test]
    fn valid_status_condition_passes_validation() {
        let condition = StatusCondition {
            observed_generation: 1,
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            reason: "ServiceStarted".to_string(),
            severity: ConditionSeverity::Info,
            message: "Service is ready".to_string(),
            evidence_refs: vec![ref_for("evidence")],
            observed_state_ref: None,
        };
        validate_status_condition(&condition, 1).expect("valid status condition");
    }

    #[test]
    fn deletion_ready_when_all_blockers_cleared() {
        let input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![],
            finalizers: vec!["controller-cleanup".to_string()],
            finalizer_cleanup_receipts: vec!["controller-cleanup-receipt".to_string()],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion gate");
        assert_eq!(decision.decision, "deletion-ready");
        assert!(decision.unresolved_blockers.is_empty());
    }

    #[test]
    fn empty_resource_type_denies() {
        let identity = ResourceIdentity {
            resource_type: "".to_string(),
            scope_ref: ref_for("scope"),
            scoped_name: "my-service".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn invalid_scope_ref_denies() {
        let identity = ResourceIdentity {
            resource_type: "molten.test.v1".to_string(),
            scope_ref: "not-a-content-ref".to_string(),
            scoped_name: "my-service".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn invalid_scoped_name_denies() {
        let identity = ResourceIdentity {
            resource_type: "molten.test.v1".to_string(),
            scope_ref: ref_for("scope"),
            scoped_name: "UPPERCASE-INVALID".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn generation_zero_denies() {
        let record = ResourceRecord {
            resource_type: "molten.test.v1".to_string(),
            resource_ref: ref_for("resource"),
            scope_ref: ref_for("scope"),
            name: "my-service".to_string(),
            generation: 0,
            desired_ref: ref_for("desired"),
            observed_ref: None,
            metadata: ResourceMetadata {
                labels: std::collections::BTreeMap::new(),
                annotations: std::collections::BTreeMap::new(),
                owner_refs: vec![],
                finalizers: vec![],
                evidence_refs: vec![],
            },
            evidence_refs: vec![],
        };
        assert!(validate_resource_record(&record).is_err());
    }

    #[test]
    fn invalid_label_key_denies() {
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("invalid label key with spaces".to_string(), "value".to_string());
        let metadata = ResourceMetadata {
            labels,
            annotations: std::collections::BTreeMap::new(),
            owner_refs: vec![],
            finalizers: vec![],
            evidence_refs: vec![],
        };
        assert!(validate_metadata(&metadata).is_err());
    }

    #[test]
    fn invalid_label_value_denies() {
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("valid-key".to_string(), "value with spaces".to_string());
        let metadata = ResourceMetadata {
            labels,
            annotations: std::collections::BTreeMap::new(),
            owner_refs: vec![],
            finalizers: vec![],
            evidence_refs: vec![],
        };
        assert!(validate_metadata(&metadata).is_err());
    }

    #[test]
    fn too_many_labels_denies() {
        let mut labels = std::collections::BTreeMap::new();
        for i in 0..MAX_LABEL_COUNT + 1 {
            labels.insert(format!("key-{i}"), "value".to_string());
        }
        let metadata = ResourceMetadata {
            labels,
            annotations: std::collections::BTreeMap::new(),
            owner_refs: vec![],
            finalizers: vec![],
            evidence_refs: vec![],
        };
        assert!(validate_metadata(&metadata).is_err());
    }

    #[test]
    fn stale_observed_generation_denies() {
        let condition = StatusCondition {
            observed_generation: 3,
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            reason: "Started".to_string(),
            severity: ConditionSeverity::Info,
            message: "Ready".to_string(),
            evidence_refs: vec![ref_for("evidence")],
            observed_state_ref: None,
        };
        let result = validate_status_condition(&condition, 2);
        assert!(result.is_err(), "stale observed generation should deny");
        let error = result.unwrap_err();
        assert!(error.to_string().contains("observed generation"), "error: {error}");
    }

    #[test]
    fn missing_evidence_refs_denies_status_condition() {
        let condition = StatusCondition {
            observed_generation: 1,
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            reason: "Started".to_string(),
            severity: ConditionSeverity::Info,
            message: "Ready".to_string(),
            evidence_refs: vec![],
            observed_state_ref: None,
        };
        assert!(validate_status_condition(&condition, 1).is_err());
    }

    #[test]
    fn missing_finalizer_cleanup_blocks_deletion() {
        let input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![],
            finalizers: vec!["controller-cleanup".to_string()],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("controller-cleanup")));
    }

    #[test]
    fn missing_deletion_authority_blocks_deletion() {
        let input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("deletion authority")));
    }

    #[test]
    fn active_pin_blocks_deletion() {
        let input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec![ref_for("pin")],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("pin")));
    }

    #[test]
    fn live_owner_blocks_deletion() {
        let owner = OwnerRef {
            resource_ref: ref_for("owner"),
            resource_type: "molten.test.parent.v1".to_string(),
            block_delete_on_deletion: true,
        };
        let input = DeletionGateInput {
            resource_ref: ref_for("resource"),
            owner_refs: vec![owner.clone()],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![owner.resource_ref],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec![ref_for("auth")],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("live owner")));
    }

    // --- Admission chain resource gates ---

    #[test]
    fn admitted_resource_update_records_every_phase() {
        let input = AdmissionChainInput {
            operation: ResourceOperation::Update,
            resource_ref: ref_for("resource"),
            candidate_ref: ref_for("candidate"),
            envelope_decode_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("env")] }),
            schema_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("schema")] }),
            authority_preflight_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("authority")] }),
            defaulting_evidence: Some(MutationEvidence {
                rule_ref: ref_for("default-rule"),
                pre_mutation_ref: ref_for("pre-default"),
                post_mutation_ref: ref_for("post-default"),
            }),
            mutation_evidence: Some(MutationEvidence {
                rule_ref: ref_for("mut-rule"),
                pre_mutation_ref: ref_for("pre-mut"),
                post_mutation_ref: ref_for("post-mut"),
            }),
            final_validation_passed: Some(PhaseEvidence { evidence_refs: vec![ref_for("final")] }),
            policy_evidence_gates: vec![ref_for("policy")],
        };

        let result = evaluate_admission_chain(&input);
        assert!(result.pass, "admission chain should pass for valid input");
        assert_eq!(result.phase_results.len(), 8);
        assert!(result.commit_plan_ref.is_some(), "commit plan ref should be generated");
    }

    #[test]
    fn status_operation_defaulting_and_mutation_skipped() {
        let input = AdmissionChainInput {
            operation: ResourceOperation::Status,
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
        assert!(result.pass);
        assert_eq!(result.phase_results[3].decision, PhaseDecision::Skip);
        assert_eq!(result.phase_results[4].decision, PhaseDecision::Skip);
    }

    #[test]
    fn valid_status_operation_isolates_status() {
        let input = StatusOperationInput {
            current_generation: 1,
            proposed_generation: 1,
            changes_desired_ref: false,
            changes_desired_generation: false,
            changes_finalizers: false,
            changes_authority_metadata: false,
            has_status_condition_evidence: true,
        };
        let decision = validate_status_operation(&input);
        assert!(decision.pass);
        assert!(decision.diagnostics.is_empty());
    }

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
}