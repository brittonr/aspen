
    #[test]
    fn actormap_transaction_predicate_commits_rolls_back_and_invalidates_removed_objects() {
        let existing = deterministic_ref("existing-object");
        let spawned = deterministic_ref("spawned-object");
        let removed = deterministic_ref("removed-object");
        let committed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            used_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
        };
        let pass = evaluate_actormap_transaction(&committed).expect("actormap transaction predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let stale_removed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), removed.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            used_object_refs: vec![removed.clone()],
        };
        let denied = evaluate_actormap_transaction(&stale_removed).expect("denied actormap predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "actormap-commit-delta-mismatch"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-present-after-commit")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-used-after-removal")
        );

        let rollback = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::RolledBack,
            before_object_refs: vec![existing],
            after_object_refs: vec![spawned.clone()],
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: Vec::new(),
            visible_object_refs: vec![spawned.clone()],
            used_object_refs: vec![spawned],
        };
        let rollback_denied = evaluate_actormap_transaction(&rollback).expect("rollback actormap predicate");
        assert!(!rollback_denied.is_allowed);
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "actormap-rollback-state-changed")
        );
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "spawned-object-visible-after-rollback")
        );
    }

    #[test]
    fn rights_amplification_predicate_denies_unsealed_authority_recovery() {
        // r[verify molten.vat_ref_state_proof.rollback_cleanup]
        let holder_ref = deterministic_ref("rights-holder-object");
        let sealed_value_ref = deterministic_ref("rights-sealed-value");
        let brand_ref = deterministic_ref("rights-brand");
        let wrong_brand_ref = deterministic_ref("rights-wrong-brand");
        let sealed_authority_ref = deterministic_ref("rights-sealed-authority");
        let unsealed_authority_ref = deterministic_ref("rights-unsealed-authority");

        let admitted = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref.clone(),
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: brand_ref.clone(),
            sealed_authority_refs: vec![sealed_authority_ref.clone()],
            recovered_authority_refs: vec![sealed_authority_ref.clone()],
        };
        let admitted_result = evaluate_rights_amplification(&admitted).expect("admitted rights amplification");
        assert!(admitted_result.is_allowed);
        assert_eq!(admitted_result.receipt.decision, PredicateDecision::Pass);

        let extra_recovery = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref.clone(),
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: brand_ref.clone(),
            sealed_authority_refs: vec![sealed_authority_ref.clone()],
            recovered_authority_refs: sorted_refs(vec![sealed_authority_ref, unsealed_authority_ref]),
        };
        let extra_result = evaluate_rights_amplification(&extra_recovery).expect("extra rights amplification");
        assert!(!extra_result.is_allowed);
        assert!(
            extra_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "rights-amplification-recovered-authority-not-sealed")
        );

        let wrong_brand = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref,
            sealed_value_ref,
            sealer_brand_ref: brand_ref,
            unsealer_brand_ref: wrong_brand_ref,
            sealed_authority_refs: vec![deterministic_ref("rights-sealed-authority")],
            recovered_authority_refs: vec![deterministic_ref("rights-sealed-authority")],
        };
        let wrong_brand_result = evaluate_rights_amplification(&wrong_brand).expect("wrong brand amplification");
        assert!(!wrong_brand_result.is_allowed);
        assert!(
            wrong_brand_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "rights-amplification-brand-mismatch")
        );
    }

    #[test]
    fn vat_rollback_cleanup_binds_snapshot_and_removes_dependent_state() {
        // r[verify molten.vat_ref_state_proof.rollback_cleanup]
        let staged_value = RuntimeValue::string("rollback-staged-assertion").expect("runtime value");
        let actor = "rollback-owner".to_string();
        let staged_assertion = crate::runtime::RuntimeAssertion {
            actor: actor.clone(),
            value: staged_value.clone(),
        };
        let staged_assertion_ref = staged_assertion.assertion_ref().expect("staged assertion ref");
        let staged_observer_ref = deterministic_ref("rollback-staged-observer");
        let staged_pending_call_ref = deterministic_ref("rollback-staged-pending-call");
        let staged_authority_snapshot_ref = deterministic_ref("rollback-staged-authority-snapshot");
        let state = RuntimeState::new(TURN_COMMIT_TEST_SEED);
        let before = state.snapshot();
        let before_ref = before.snapshot_ref().expect("before snapshot ref");
        let step = RuntimeStep::Assert {
            actor,
            value: staged_value,
        };
        let turn = state.begin_turn(&step);
        let (_events, rollback_receipt) = state
            .rollback_turn_with_predicate_receipt(turn, step.primary_actor(), "policy denied")
            .expect("rollback receipt");
        let final_snapshot = state.snapshot();
        assert_eq!(final_snapshot, before);
        assert!(
            !final_snapshot
                .assertions
                .iter()
                .any(|assertion| assertion.assertion_ref().expect("assertion ref") == staged_assertion_ref)
        );

        let cleaned = RuntimeVatRollbackCleanupState {
            rollback_receipt_ref: rollback_receipt.receipt_ref.clone(),
            before_snapshot_ref: before_ref.clone(),
            final_snapshot_ref: before_ref.clone(),
            rolled_back_refs: sorted_refs(vec![
                staged_assertion_ref.clone(),
                staged_observer_ref.clone(),
                staged_pending_call_ref.clone(),
                staged_authority_snapshot_ref.clone(),
            ]),
            remaining_assertion_refs: Vec::new(),
            remaining_observer_refs: Vec::new(),
            remaining_pending_call_refs: Vec::new(),
            remaining_authority_snapshot_refs: Vec::new(),
        };
        let cleaned_result = evaluate_vat_rollback_cleanup(&cleaned).expect("rollback cleanup");
        assert!(cleaned_result.is_allowed);
        assert_eq!(cleaned_result.receipt.decision, PredicateDecision::Pass);

        let leaked = RuntimeVatRollbackCleanupState {
            final_snapshot_ref: deterministic_ref("rollback-mutated-final-snapshot"),
            remaining_assertion_refs: vec![staged_assertion_ref],
            remaining_observer_refs: vec![staged_observer_ref],
            remaining_pending_call_refs: vec![staged_pending_call_ref],
            remaining_authority_snapshot_refs: vec![staged_authority_snapshot_ref],
            ..cleaned
        };
        let leaked_result = evaluate_vat_rollback_cleanup(&leaked).expect("leaked rollback cleanup");
        assert!(!leaked_result.is_allowed);
        for expected in [
            "vat-rollback-final-snapshot-changed",
            "vat-rollback-assertion-leak",
            "vat-rollback-observer-leak",
            "vat-rollback-pending-call-leak",
            "vat-rollback-authority-snapshot-leak",
        ] {
            assert!(leaked_result.receipt.diagnostics.iter().any(|diagnostic| diagnostic == expected), "{expected}");
        }
    }
