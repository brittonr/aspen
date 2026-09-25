
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
        let labels = (0..=MAX_LABEL_COUNT)
            .map(|i| (format!("key-{i}"), "value".to_string()))
            .collect();
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
