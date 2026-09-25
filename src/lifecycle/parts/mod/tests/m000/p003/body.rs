
    fn assert_transition_denies(from_state: super::State, to_state: super::State) {
        let input = matrix_transition_input(from_state, to_state, super::Action::SupervisorDecision);
        let receipt = super::transition_receipt(&input).expect("denial receipt");
        let expected = format!("invalid transition {} -> {}", from_state.as_str(), to_state.as_str());

        assert_eq!(receipt.decision, "deny");
        assert!(contains_diagnostic(&receipt, &expected));
    }

    #[test]
    fn lifecycle_reachability_from_declared_covers_expected_paths() {
        // r[verify molten.lifecycle_state_machine_proof.reachability]
        let reachable = super::reachable_lifecycle_states(super::State::Declared);
        for state in super::lifecycle_states() {
            assert!(reachable.contains(state), "state must be reachable: {}", state.as_str());
            assert!(super::lifecycle_state_reachable(super::State::Declared, *state));
        }

        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
        ]);
        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
            super::State::Degraded,
            super::State::Stopping,
            super::State::Stopped,
            super::State::Cleaned,
        ]);
        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
            super::State::Failed,
            super::State::Restarting,
            super::State::Starting,
        ]);
    }

    #[test]
    fn forbidden_lifecycle_shortcuts_deny() {
        // r[verify molten.lifecycle_state_machine_proof.reachability]
        assert_transition_denies(super::State::Declared, super::State::Ready);
        assert_transition_denies(super::State::Ready, super::State::Cleaned);
    }

    #[test]
    fn cleaned_state_has_no_outgoing_passing_transition() {
        // r[verify molten.lifecycle_state_machine_proof.terminal_cleanup]
        assert!(super::lifecycle_successor_states(super::State::Cleaned).is_empty());
        for to_state in super::lifecycle_states() {
            assert_transition_denies(super::State::Cleaned, *to_state);
        }
    }

    #[test]
    fn terminal_and_cleanup_boundary_successors_are_closed() {
        // r[verify molten.lifecycle_state_machine_proof.terminal_cleanup]
        assert_eq!(
            super::lifecycle_successor_states(super::State::Stopped),
            vec![super::State::Cleaned]
        );
        assert_eq!(
            super::lifecycle_successor_states(super::State::Failed),
            vec![super::State::Restarting, super::State::Cleaned]
        );
        assert_eq!(
            super::lifecycle_successor_states(super::State::Restarting),
            vec![super::State::Starting, super::State::Cleaned]
        );

        assert_transition_denies(super::State::Stopped, super::State::Starting);
        assert_transition_denies(super::State::Failed, super::State::Ready);
        assert_transition_denies(super::State::Restarting, super::State::Ready);
    }

    fn assert_error_contains(error: crate::error::MoltenError, expected: &str) {
        assert!(
            error.to_string().contains(expected),
            "expected error to contain {expected:?}, got {error}"
        );
    }

    #[test]
    fn lifecycle_valid_transition_diagnostics_are_empty_and_receipts_pass() {
        // r[verify molten.lifecycle_state_machine_proof.denial_diagnostics]
        // r[verify molten.lifecycle_state_machine_proof.denial_receipt_binding]
        for transition in super::allowed_transition_relation() {
            let input = matrix_transition_input(
                transition.from_state,
                transition.to_state,
                matching_action_for_target(transition.to_state),
            );
            let diagnostics = super::transition_diagnostics(&input);
            let receipt = super::transition_receipt(&input).expect("valid receipt");

            assert!(diagnostics.is_empty());
            assert_eq!(receipt.decision, "pass");
            assert!(receipt.diagnostics.is_empty());
        }
    }

    #[test]
    fn lifecycle_denial_diagnostics_are_stable_for_invalid_edges_and_action_mismatches() {
        // r[verify molten.lifecycle_state_machine_proof.denial_diagnostics]
        let invalid_edge = matrix_transition_input(
            super::State::Declared,
            super::State::Ready,
            matching_action_for_target(super::State::Ready),
        );
        let invalid_edge_receipt = super::transition_receipt(&invalid_edge).expect("invalid edge receipt");
        assert_eq!(invalid_edge_receipt.decision, "deny");
        assert_eq!(
            invalid_edge_receipt.diagnostics,
            vec!["invalid transition declared -> ready".to_owned()]
        );

        let action_mismatch = matrix_transition_input(
            super::State::Declared,
            super::State::Spawning,
            mismatched_action_for_target(super::State::Spawning),
        );
        let action_mismatch_receipt = super::transition_receipt(&action_mismatch).expect("action mismatch receipt");
        assert_eq!(action_mismatch_receipt.decision, "deny");
        assert_eq!(
            action_mismatch_receipt.diagnostics,
            vec!["action start does not match target state spawning".to_owned()]
        );
    }

    #[test]
    fn lifecycle_combined_denial_diagnostics_keep_order_and_receipt_refs_stable() {
        // r[verify molten.lifecycle_state_machine_proof.denial_diagnostics]
        // r[verify molten.lifecycle_state_machine_proof.denial_receipt_binding]
        let input = matrix_transition_input(super::State::Declared, super::State::Cleaned, super::Action::Start);
        let first = super::transition_receipt(&input).expect("first denial receipt");
        let second = super::transition_receipt(&input).expect("second denial receipt");
        let rendered = to_text(&first.value).expect("render denial receipt");

        assert_eq!(first.decision, "deny");
        assert_eq!(
            first.diagnostics,
            vec![
                "action start does not match target state cleaned".to_owned(),
                "invalid transition declared -> cleaned".to_owned(),
            ]
        );
        assert_eq!(first.diagnostics, second.diagnostics);
        assert_eq!(first.transition_ref, second.transition_ref);
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert!(first.transition_ref.starts_with("blake3:"));
        assert!(rendered.contains(&first.transition_ref));
        assert!(rendered.contains("deny"));
    }

    #[test]
    fn lifecycle_malformed_transition_inputs_fail_closed_before_receipts() {
        // r[verify molten.lifecycle_state_machine_proof.denial_receipt_binding]
        let mut empty_entity = matrix_transition_input(
            super::State::Declared,
            super::State::Spawning,
            super::Action::Spawn,
        );
        empty_entity.entity_id = " ".to_owned();
        assert_error_contains(
            super::transition_receipt(&empty_entity).expect_err("empty entity id denied"),
            "lifecycle entity id must be non-empty",
        );

        let mut empty_cause = matrix_transition_input(
            super::State::Declared,
            super::State::Spawning,
            super::Action::Spawn,
        );
        empty_cause.cause.clear();
        assert_error_contains(
            super::transition_receipt(&empty_cause).expect_err("empty cause denied"),
            "lifecycle transition cause must be non-empty",
        );

        let mut malformed_ref = matrix_transition_input(
            super::State::Declared,
            super::State::Spawning,
            super::Action::Spawn,
        );
        malformed_ref.policy_refs = vec!["not-a-content-ref".to_owned()];
        assert_error_contains(
            super::transition_receipt(&malformed_ref).expect_err("malformed ref denied"),
            "content ref must start with blake3:",
        );
    }

    fn deterministic_lifecycle_input() -> super::TransitionInput {
        let mut input = matrix_transition_input(super::State::Ready, super::State::Failed, super::Action::Fail);
        input.policy_refs = vec![content_ref_from_bytes(b"determinism-policy")];
        input.resource_refs = vec![content_ref_from_bytes(b"determinism-resource")];
        input.evidence_refs = vec![content_ref_from_bytes(b"determinism-evidence")];
        input.supervisor_ref = Some(content_ref_from_bytes(b"determinism-supervisor"));
        input
    }

    fn assert_drift_changes_lifecycle_evidence(
        label: &str,
        base_record: &super::TransitionRecord,
        base_receipt: &super::TransitionReceipt,
        drifted: &super::TransitionInput,
    ) {
        let drifted_record = super::transition_record(drifted).expect("drifted transition record");
        let drifted_receipt = super::transition_receipt(drifted).expect("drifted transition receipt");

        assert_ne!(
            base_record.transition_ref, drifted_record.transition_ref,
            "transition ref must change for {label}"
        );
        assert_ne!(
            base_receipt.receipt_ref, drifted_receipt.receipt_ref,
            "receipt ref must change for {label}"
        );
    }

    fn lifecycle_receipt_value_for_test(
        transition_ref: &str,
        decision: &str,
        diagnostics: &[String],
        checks: preserves::IOValue,
    ) -> preserves::IOValue {
        crate::preserves_rail::record("lifecycle-transition-receipt-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::LIFECYCLE_TRANSITION_RECEIPT_SCHEMA),
            crate::preserves_rail::record("transition", vec![crate::preserves_rail::string(transition_ref)]),
            crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
            crate::preserves_rail::record(
                "diagnostics",
                vec![crate::preserves_rail::sequence(
                    diagnostics.iter().map(crate::preserves_rail::string).collect(),
                )],
            ),
            checks,
        ])
    }

    fn lifecycle_tampered_checks_value() -> preserves::IOValue {
        crate::preserves_rail::record("checks", vec![
            crate::preserves_rail::bool_value(false),
            crate::preserves_rail::sequence(Vec::new()),
        ])
    }

    #[test]
    fn lifecycle_receipts_are_deterministic_for_identical_inputs() {
        // r[verify molten.lifecycle_state_machine_proof.receipt_determinism]
        // r[verify molten.lifecycle_state_machine_proof.receipt_evidence_binding]
        let input = deterministic_lifecycle_input();
        let first_record = super::transition_record(&input).expect("first transition record");
        let second_record = super::transition_record(&input).expect("second transition record");
        let first_receipt = super::transition_receipt(&input).expect("first transition receipt");
        let second_receipt = super::transition_receipt(&input).expect("second transition receipt");
        let validation = super::validate_transition_receipt(
            &first_record.value,
            &first_receipt.value,
            Some(&first_receipt.receipt_ref),
        )
        .expect("receipt validation");

        assert_eq!(first_record, second_record);
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(validation.transition_ref, first_record.transition_ref);
        assert_eq!(validation.receipt_ref, first_receipt.receipt_ref);
        assert_eq!(validation.decision, first_receipt.decision);
        assert_eq!(validation.diagnostics, first_receipt.diagnostics);
    }
