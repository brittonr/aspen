
    #[test]
    fn lifecycle_input_drift_changes_transition_or_receipt_evidence() {
        // r[verify molten.lifecycle_state_machine_proof.receipt_determinism]
        let input = deterministic_lifecycle_input();
        let base_record = super::transition_record(&input).expect("base transition record");
        let base_receipt = super::transition_receipt(&input).expect("base transition receipt");

        let mut changed_state = input.clone();
        changed_state.to_state = super::State::Degraded;
        changed_state.action = super::Action::Degrade;
        assert_drift_changes_lifecycle_evidence("state", &base_record, &base_receipt, &changed_state);

        let mut changed_action = input.clone();
        changed_action.action = super::Action::SupervisorDecision;
        assert_drift_changes_lifecycle_evidence("action", &base_record, &base_receipt, &changed_action);

        let mut changed_cause = input.clone();
        changed_cause.cause = "drifted-cause".to_owned();
        assert_drift_changes_lifecycle_evidence("cause", &base_record, &base_receipt, &changed_cause);

        let mut changed_refs = input.clone();
        changed_refs.policy_refs = vec![content_ref_from_bytes(b"drifted-policy")];
        assert_drift_changes_lifecycle_evidence("refs", &base_record, &base_receipt, &changed_refs);

        let mut changed_supervisor = input.clone();
        changed_supervisor.supervisor_ref = Some(content_ref_from_bytes(b"drifted-supervisor"));
        assert_drift_changes_lifecycle_evidence("supervisor", &base_record, &base_receipt, &changed_supervisor);

        let mut changed_step = input.clone();
        changed_step.logical_step += 1;
        assert_drift_changes_lifecycle_evidence("logical step", &base_record, &base_receipt, &changed_step);
    }

    #[test]
    fn lifecycle_receipt_validator_rejects_tampered_evidence_bindings() {
        // r[verify molten.lifecycle_state_machine_proof.receipt_evidence_binding]
        let input = matrix_transition_input(
            super::State::Declared,
            super::State::Ready,
            matching_action_for_target(super::State::Ready),
        );
        let transition = super::transition_record(&input).expect("transition record");
        let receipt = super::transition_receipt(&input).expect("transition receipt");
        super::validate_transition_receipt(&transition.value, &receipt.value, Some(&receipt.receipt_ref))
            .expect("original denial receipt validates");

        let wrong_transition_ref = content_ref_from_bytes(b"wrong-transition");
        let tampered_transition = lifecycle_receipt_value_for_test(
            &wrong_transition_ref,
            &receipt.decision,
            &receipt.diagnostics,
            super::checks_value(),
        );
        assert_error_contains(
            super::validate_transition_receipt(&transition.value, &tampered_transition, None)
                .expect_err("wrong transition ref denied"),
            "lifecycle receipt transition ref mismatch",
        );

        let tampered_decision = lifecycle_receipt_value_for_test(
            &receipt.transition_ref,
            "pass",
            &receipt.diagnostics,
            super::checks_value(),
        );
        assert_error_contains(
            super::validate_transition_receipt(&transition.value, &tampered_decision, None)
                .expect_err("wrong decision denied"),
            "lifecycle receipt decision mismatch",
        );

        let tampered_diagnostics = lifecycle_receipt_value_for_test(
            &receipt.transition_ref,
            &receipt.decision,
            &[],
            super::checks_value(),
        );
        assert_error_contains(
            super::validate_transition_receipt(&transition.value, &tampered_diagnostics, None)
                .expect_err("dropped diagnostics denied"),
            "lifecycle receipt diagnostics mismatch",
        );

        let tampered_checks = lifecycle_receipt_value_for_test(
            &receipt.transition_ref,
            &receipt.decision,
            &receipt.diagnostics,
            lifecycle_tampered_checks_value(),
        );
        assert_error_contains(
            super::validate_transition_receipt(&transition.value, &tampered_checks, None)
                .expect_err("tampered checks denied"),
            "lifecycle receipt checks mismatch",
        );
    }
