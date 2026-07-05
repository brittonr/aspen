
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

    // --- Declarative resource records: identity ---

    #[test]
    fn valid_resource_identity_produces_stable_ref() {
        // r[verify molten.resource_model.canonical_resource_records]
        let identity = ResourceIdentity {
            resource_type: "molten.test.service.v1".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scoped_name: "my-service".to_string(),
        };
        identity.validate().expect("valid identity");
        let ref1 = identity.canonical_ref().expect("canonical ref");
        let ref2 = identity.canonical_ref().expect("repeat canonical ref");
        assert_eq!(ref1, ref2, "same identity produces same ref during replay");
    }

    #[test]
    fn valid_resource_record_with_metadata() {
        // r[verify molten.resource_model.canonical_resource_records]
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
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            name: "my-service".to_string(),
            generation: 1,
            desired_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
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
        // r[verify molten.resource_model.canonical_resource_records]
        let observed_ref = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
        let record = ResourceRecord {
            resource_type: "molten.test.service.v1".to_string(),
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            name: "my-service".to_string(),
            generation: 1,
            desired_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            observed_ref: Some(observed_ref.to_string()),
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
        // r[verify molten.resource_model.status_conditions_observed_generation]
        let condition = StatusCondition {
            observed_generation: 1,
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            reason: "ServiceStarted".to_string(),
            severity: ConditionSeverity::Info,
            message: "Service is ready".to_string(),
            evidence_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
            observed_state_ref: None,
        };
        validate_status_condition(&condition, 1).expect("valid status condition");
    }

    #[test]
    fn deletion_ready_when_all_blockers_cleared() {
        // r[verify molten.resource_model.owner_refs_finalizers_gc]
        let input = DeletionGateInput {
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            owner_refs: vec![],
            finalizers: vec!["controller-cleanup".to_string()],
            finalizer_cleanup_receipts: vec!["controller-cleanup-receipt".to_string()],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion gate");
        assert_eq!(decision.decision, "deletion-ready");
        assert!(decision.unresolved_blockers.is_empty());
    }

    #[test]
    fn empty_resource_type_denies() {
        // r[verify molten.resource_model.canonical_resource_records]
        let identity = ResourceIdentity {
            resource_type: "".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scoped_name: "my-service".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn invalid_scope_ref_denies() {
        // r[verify molten.resource_model.canonical_resource_records]
        let identity = ResourceIdentity {
            resource_type: "molten.test.v1".to_string(),
            scope_ref: "not-a-content-ref".to_string(),
            scoped_name: "my-service".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn invalid_scoped_name_denies() {
        // r[verify molten.resource_model.canonical_resource_records]
        let identity = ResourceIdentity {
            resource_type: "molten.test.v1".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scoped_name: "UPPERCASE-INVALID".to_string(),
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn generation_zero_denies() {
        // r[verify molten.resource_model.canonical_resource_records]
        let record = ResourceRecord {
            resource_type: "molten.test.v1".to_string(),
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            scope_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            name: "my-service".to_string(),
            generation: 0,
            desired_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
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
        // r[verify molten.resource_model.canonical_resource_records]
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
        // r[verify molten.resource_model.canonical_resource_records]
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
        // r[verify molten.resource_model.canonical_resource_records]
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
        // r[verify molten.resource_model.status_conditions_observed_generation]
        let condition = StatusCondition {
            observed_generation: 3,
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            reason: "Started".to_string(),
            severity: ConditionSeverity::Info,
            message: "Ready".to_string(),
            evidence_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
            observed_state_ref: None,
        };
        let result = validate_status_condition(&condition, 2);
        assert!(result.is_err(), "stale observed generation should deny");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("observed generation"),
            "error should mention observed generation: {error}"
        );
    }

    #[test]
    fn missing_evidence_refs_denies_status_condition() {
        // r[verify molten.resource_model.status_conditions_observed_generation]
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
        // r[verify molten.resource_model.owner_refs_finalizers_gc]
        let input = DeletionGateInput {
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            owner_refs: vec![],
            finalizers: vec!["controller-cleanup".to_string()],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("controller-cleanup")));
    }

    #[test]
    fn missing_deletion_authority_blocks_deletion() {
        // r[verify molten.resource_model.owner_refs_finalizers_gc]
        let input = DeletionGateInput {
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
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
        // r[verify molten.resource_model.owner_refs_finalizers_gc]
        let input = DeletionGateInput {
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            owner_refs: vec![],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![],
            pin_refs: vec!["blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string()],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("pin")));
    }

    #[test]
    fn live_owner_blocks_deletion() {
        // r[verify molten.resource_model.owner_refs_finalizers_gc]
        let owner = OwnerRef {
            resource_ref: "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            resource_type: "molten.test.parent.v1".to_string(),
            block_delete_on_deletion: true,
        };
        let input = DeletionGateInput {
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            owner_refs: vec![owner.clone()],
            finalizers: vec![],
            finalizer_cleanup_receipts: vec![],
            live_owner_refs: vec![owner.resource_ref],
            pin_refs: vec![],
            retention_policy_refs: vec![],
            deletion_authority_refs: vec!["blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()],
        };
        let decision = evaluate_deletion_gate(&input).expect("deletion evaluation");
        assert_eq!(decision.decision, "blocked");
        assert!(decision.unresolved_blockers.iter().any(|d| d.contains("live owner")));
    }

    // --- Admission chain resource gates ---

    #[test]
    fn admitted_resource_update_records_every_phase() {
        // r[verify molten.resource_admission.ordered_chain_receipts]
        let input = AdmissionChainInput {
            operation: ResourceOperation::Update,
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            candidate_ref: "blake3:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            envelope_decode_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:2222222222222222222222222222222222222222222222222222222222222222".to_string()],
            }),
            schema_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:3333333333333333333333333333333333333333333333333333333333333333".to_string()],
            }),
            authority_preflight_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:4444444444444444444444444444444444444444444444444444444444444444".to_string()],
            }),
            defaulting_evidence: Some(MutationEvidence {
                rule_ref: "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                pre_mutation_ref: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                post_mutation_ref: "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string(),
            }),
            mutation_evidence: Some(MutationEvidence {
                rule_ref: "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string(),
                pre_mutation_ref: "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
                post_mutation_ref: "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            }),
            final_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:5555555555555555555555555555555555555555555555555555555555555555".to_string()],
            }),
            policy_evidence_gates: vec!["blake3:6666666666666666666666666666666666666666666666666666666666666666".to_string()],
        };

        let result = evaluate_admission_chain(&input);
        assert!(result.pass, "admission chain should pass for valid input");
        assert_eq!(result.phase_results.len(), 8, "all 8 phases should be evaluated");
        assert!(result.commit_plan_ref.is_some(), "commit plan ref should be generated");

        for (i, phase_result) in result.phase_results.iter().enumerate() {
            assert_eq!(phase_result.phase.index(), i, "phase at index {i} should be {:?}", phase_result.phase);
        }
    }

    #[test]
    fn status_operation_defaulting_and_mutation_skipped() {
        // r[verify molten.resource_admission.ordered_chain_receipts]
        let input = AdmissionChainInput {
            operation: ResourceOperation::Status,
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            candidate_ref: "blake3:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            envelope_decode_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:2222222222222222222222222222222222222222222222222222222222222222".to_string()],
            }),
            schema_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:3333333333333333333333333333333333333333333333333333333333333333".to_string()],
            }),
            authority_preflight_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:4444444444444444444444444444444444444444444444444444444444444444".to_string()],
            }),
            defaulting_evidence: None,
            mutation_evidence: None,
            final_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:5555555555555555555555555555555555555555555555555555555555555555".to_string()],
            }),
            policy_evidence_gates: vec!["blake3:6666666666666666666666666666666666666666666666666666666666666666".to_string()],
        };

        let result = evaluate_admission_chain(&input);
        assert!(result.pass);
        assert_eq!(result.phase_results[3].decision, PhaseDecision::Skip);
        assert_eq!(result.phase_results[4].decision, PhaseDecision::Skip);
    }

    #[test]
    fn valid_status_operation_isolates_status() {
        // r[verify molten.resource_admission.status_subresource_isolated]
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
        // r[verify molten.resource_admission.ordered_chain_receipts]
        let input = AdmissionChainInput {
            operation: ResourceOperation::Update,
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            candidate_ref: "blake3:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            envelope_decode_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:2222222222222222222222222222222222222222222222222222222222222222".to_string()],
            }),
            schema_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:3333333333333333333333333333333333333333333333333333333333333333".to_string()],
            }),
            authority_preflight_passed: None,
            defaulting_evidence: None,
            mutation_evidence: None,
            final_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:5555555555555555555555555555555555555555555555555555555555555555".to_string()],
            }),
            policy_evidence_gates: vec!["blake3:6666666666666666666666666666666666666666666666666666666666666666".to_string()],
        };

        let result = evaluate_admission_chain(&input);
        assert!(!result.pass, "missing authority preflight should deny");
        assert_eq!(result.phase_results[2].decision, PhaseDecision::Deny);
        for phase_result in &result.phase_results[3..] {
            assert_eq!(phase_result.decision, PhaseDecision::Skip,
                "phase {} should be skipped after deny", phase_result.phase.as_str());
        }
    }

    #[test]
    fn missing_mutation_evidence_for_create_denies() {
        // r[verify molten.resource_admission.mutation_requires_reviewed_rule]
        let input = AdmissionChainInput {
            operation: ResourceOperation::Create,
            resource_ref: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            candidate_ref: "blake3:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            envelope_decode_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:2222222222222222222222222222222222222222222222222222222222222222".to_string()],
            }),
            schema_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:3333333333333333333333333333333333333333333333333333333333333333".to_string()],
            }),
            authority_preflight_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:4444444444444444444444444444444444444444444444444444444444444444".to_string()],
            }),
            defaulting_evidence: None,
            mutation_evidence: None,
            final_validation_passed: Some(PhaseEvidence {
                evidence_refs: vec!["blake3:5555555555555555555555555555555555555555555555555555555555555555".to_string()],
            }),
            policy_evidence_gates: vec!["blake3:6666666666666666666666666666666666666666666666666666666666666666".to_string()],
        };

        let result = evaluate_admission_chain(&input);
        assert!(!result.pass, "missing mutation evidence for create should deny");
    }

    #[test]
    fn status_operation_attempting_desired_mutation_denies() {
        // r[verify molten.resource_admission.status_subresource_isolated]
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
        // r[verify molten.resource_admission.status_subresource_isolated]
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
        // r[verify molten.resource_admission.status_subresource_isolated]
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
}
