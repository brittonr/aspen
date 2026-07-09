
    const FIRST_EFFECT_SEQUENCE: u64 = 0;
    const SECOND_EFFECT_SEQUENCE: u64 = 1;

    fn effect_entry(sequence: u64, request_ref: &str, response_ref: &str) -> EffectLogEntry {
        EffectLogEntry {
            sequence,
            effect_kind: "clock".to_string(),
            run_identity_ref: DEFAULT_RUNTIME_REF.to_string(),
            handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
            turn_ref: DEFAULT_ARTIFACT_REF.to_string(),
            boundary_ref: DEFAULT_SCHEMA_REF.to_string(),
            request_ref: request_ref.to_string(),
            response_ref: response_ref.to_string(),
        }
    }

    fn consumed_effect(sequence: u64, request_ref: &str, response_ref: &str) -> ConsumedEffect {
        ConsumedEffect {
            sequence,
            effect_kind: "clock".to_string(),
            request_ref: request_ref.to_string(),
            response_ref: response_ref.to_string(),
            boundary_ref: DEFAULT_SCHEMA_REF.to_string(),
            used_live_fallback: false,
        }
    }

    fn validation(entries: &[EffectLogEntry], consumed: &[ConsumedEffect]) -> EffectLogValidation {
        validate_effect_log(EffectLogValidationInput {
            expected_run_identity_ref: DEFAULT_RUNTIME_REF,
            expected_handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF,
            entries,
            consumed,
        })
        .expect("effect log validation")
    }

    #[test]
    fn effect_log_validation_accepts_ordered_fully_consumed_entries() {
        let entries = vec![
            effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF),
            effect_entry(SECOND_EFFECT_SEQUENCE, DEFAULT_CAPABILITY_REF, DEFAULT_REVOCATION_REF),
        ];
        let consumed = vec![
            consumed_effect(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF),
            consumed_effect(SECOND_EFFECT_SEQUENCE, DEFAULT_CAPABILITY_REF, DEFAULT_REVOCATION_REF),
        ];
        let first = validation(&entries, &consumed);
        let second = validation(&entries, &consumed);
        assert_eq!(first.decision, "pass");
        assert_eq!(first.validation_ref, second.validation_ref);
        let text = to_text(&first.value).expect("validation text");
        assert!(text.contains("ordered-effect-log"));
        assert!(text.contains("evidence-only-no-authority"));
    }

    #[test]
    fn effect_log_validation_denies_sequence_gap_duplicate_and_reorder() {
        let gap_entries = vec![effect_entry(SECOND_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF)];
        let gap = validation(
            &gap_entries,
            &[consumed_effect(SECOND_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF)],
        );
        assert_eq!(gap.decision, "deny");
        assert!(gap.diagnostics[0].contains("expected 0"));

        let duplicate_entries = vec![
            effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF),
            effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_CAPABILITY_REF, DEFAULT_REVOCATION_REF),
        ];
        let duplicate = validation(&duplicate_entries, &[]);
        assert_eq!(duplicate.decision, "deny");
        assert!(duplicate.diagnostics[0].contains("duplicate effect sequence"));
    }

    #[test]
    fn effect_log_validation_denies_binding_profile_and_run_mismatches() {
        let mut wrong_profile = effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF);
        wrong_profile.handler_profile_ref = DEFAULT_TOOL_REF.to_string();
        let profile = validation(&[wrong_profile], &[]);
        assert_eq!(profile.decision, "deny");
        assert!(profile.diagnostics[0].contains("handler profile"));

        let entry = effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF);
        let mismatch = validation(
            &[entry],
            &[consumed_effect(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_CAPABILITY_REF)],
        );
        assert_eq!(mismatch.decision, "deny");
        assert!(mismatch.diagnostics[0].contains("response ref mismatch"));
    }

    #[test]
    fn effect_log_validation_denies_extra_missing_and_live_fallback() {
        let entry = effect_entry(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF);
        let extra = validation(&[entry.clone()], &[]);
        assert_eq!(extra.decision, "deny");
        assert!(extra.diagnostics[0].contains("unconsumed"));

        let missing = validation(&[], &[consumed_effect(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF)]);
        assert_eq!(missing.decision, "deny");
        assert!(missing.diagnostics[0].contains("missing recorded"));

        let mut live = consumed_effect(FIRST_EFFECT_SEQUENCE, DEFAULT_ARTIFACT_REF, DEFAULT_POLICY_REF);
        live.used_live_fallback = true;
        let fallback = validation(&[entry], &[live]);
        assert_eq!(fallback.decision, "deny");
        assert!(fallback.diagnostics[0].contains("live effect fallback"));
    }
