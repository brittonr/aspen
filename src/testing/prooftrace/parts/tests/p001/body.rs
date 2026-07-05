    fn validate_steps(steps: &[Step], initial: &str, final_state: &str) -> Validation {
        validate(&Run {
            initial_state_ref: initial,
            final_state_ref: final_state,
            steps,
        })
    }

    #[test]
    fn replay_accepts_valid_lifecycle_steps_and_renders_summary() {
        // r[verify molten.testing.state_machine_proof.trace_contract]
        // r[verify molten.testing.state_machine_proof.trace_validator]
        let steps = valid_steps();
        let initial = state_ref("declared");
        let final_state = state_ref("spawning");
        let first = validate_steps(&steps, &initial, &final_state);
        let second = validate_steps(&steps, &initial, &final_state);

        assert_eq!(first.decision, "pass");
        assert_eq!(first.accepted_steps, steps.len());
        assert_eq!(first.final_state_ref, final_state);
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.summary, second.summary);
        assert!(first.summary.contains(&format!("accepted_steps={}", steps.len())));
    }

    #[test]
    fn replay_rejects_missing_receipt_ref() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_steps();
        steps[0].receipt_ref.clear();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("proof trace receipt ref is not canonical"));
    }

    #[test]
    fn replay_rejects_tampered_diagnostics() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_steps();
        steps[1].diagnostics.clear();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("diagnostics do not match receipt"));
    }

    #[test]
    fn replay_rejects_stale_before_state_and_wrong_final_state() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut stale_steps = valid_steps();
        stale_steps[1].before_state_ref = state_ref("stale-before");
        let stale = validate_steps(&stale_steps, &state_ref("declared"), &state_ref("spawning"));
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics[0].contains("before-state mismatch"));

        let steps = valid_steps();
        let wrong_final = validate_steps(&steps, &state_ref("declared"), &state_ref("wrong-final"));
        assert_eq!(wrong_final.decision, "deny");
        assert!(wrong_final.diagnostics[0].contains("final-state mismatch"));
    }

    #[test]
    fn replay_rejects_out_of_order_steps() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_steps();
        steps.reverse();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("before-state mismatch"));
    }
