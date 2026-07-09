
    fn fixture_value(variant: ReplayFixtureVariant) -> IoValue {
        match variant {
            ReplayFixtureVariant::Baseline => record_fixture_value().expect("baseline fixture").value,
            _ => tampered_fixture_record_value(variant).expect("tampered fixture").value,
        }
    }

    fn prefix_manifest(summary_root_ref: &str) -> ReplayPrefixManifest {
        ReplayPrefixManifest {
            manifest_ref: DEFAULT_ARTIFACT_REF.to_string(),
            run_identity_ref: DEFAULT_RUNTIME_REF.to_string(),
            summary_root_ref: summary_root_ref.to_string(),
            turn_chunk_refs: vec![DEFAULT_SCHEMA_REF.to_string()],
            effect_log_chunk_refs: vec![DEFAULT_HANDLER_PROFILE_REF.to_string()],
        }
    }

    #[test]
    fn replay_multiturn_comparison_accepts_matching_fixture() {
        let fixture = fixture_value(ReplayFixtureVariant::Baseline);
        let first = compare_replay_fixture_values(fixture.clone(), fixture.clone()).expect("compare fixture");
        let second = compare_replay_fixture_values(fixture.clone(), fixture).expect("compare fixture");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert!(first.first_divergence.is_none());
        let text = to_text(&first.value).expect("comparison text");
        assert!(text.contains("deterministic-replay-comparison-v1"));
        assert!(text.contains("refs-only"));
    }

    #[test]
    fn replay_multiturn_comparison_reports_first_semantic_divergence_paths() {
        let baseline = fixture_value(ReplayFixtureVariant::Baseline);
        let cases = [
            (ReplayFixtureVariant::ChangedScheduler, "scheduler", "turns[0].scheduler-key-ref"),
            (ReplayFixtureVariant::ChangedInput, "input", "turns[0].input-ref"),
            (
                ReplayFixtureVariant::ChangedEffectRequest,
                "effect-request",
                "turns[0].effect-request-ref",
            ),
            (
                ReplayFixtureVariant::ChangedEffectResponse,
                "effect-response",
                "turns[0].effect-response-ref",
            ),
            (
                ReplayFixtureVariant::ChangedPolicyDecision,
                "policy-decision",
                "turns[0].policy-decision-ref",
            ),
            (ReplayFixtureVariant::ChangedAction, "action", "turns[0].action-ref"),
            (ReplayFixtureVariant::ChangedReceipt, "receipt", "turns[0].receipt-ref"),
            (ReplayFixtureVariant::ChangedOutput, "output", "output-refs[0]"),
            (ReplayFixtureVariant::ChangedStateHash, "final-state", "final-state-ref"),
        ];
        for (variant, kind, path) in cases {
            let actual = fixture_value(variant);
            let comparison = compare_replay_fixture_values(baseline.clone(), actual).expect("compare tampered fixture");
            assert_eq!(comparison.decision, "deny");
            let divergence = comparison.first_divergence.expect("first divergence");
            let text = to_text(&divergence).expect("divergence text");
            assert!(text.contains(kind), "missing kind {kind}");
            assert!(text.contains(path), "missing path {path}");
            assert!(text.contains("refs-only-no-payloads"));
        }
    }

    #[test]
    fn replay_prefix_comparison_binds_manifest_chunks_for_large_traces() {
        let pass = compare_replay_prefix_manifests(
            prefix_manifest(DEFAULT_POLICY_REF),
            prefix_manifest(DEFAULT_POLICY_REF),
        )
        .expect("prefix pass");
        assert_eq!(pass.decision, "pass");
        assert!(pass.first_mismatch_ref.is_none());

        let deny = compare_replay_prefix_manifests(
            prefix_manifest(DEFAULT_POLICY_REF),
            prefix_manifest(DEFAULT_CAPABILITY_REF),
        )
        .expect("prefix deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.first_mismatch_ref.is_some());
        let text = to_text(&deny.value).expect("prefix text");
        assert!(text.contains("range-receipt-required"));
        assert!(text.contains("manifest-backed-prefix"));
    }

    #[test]
    fn replay_explain_receipt_is_canonical_and_rejects_malformed_input() {
        let baseline = fixture_value(ReplayFixtureVariant::Baseline);
        let actual = fixture_value(ReplayFixtureVariant::ChangedEffectResponse);
        let comparison = compare_replay_fixture_values(baseline, actual).expect("comparison");
        let explain = explain_replay_comparison_value(comparison.value).expect("explain receipt");
        assert_eq!(explain.decision, "deny");
        assert!(explain.first_divergence_ref.is_some());
        let text = to_text(&explain.value).expect("explain text");
        assert!(text.contains("canonical-receipt-before-render"));
        assert!(text.contains("refs-only-no-payloads"));

        let malformed = record("not-a-replay-comparison", Vec::new());
        let error = explain_replay_comparison_value(malformed).expect_err("malformed explain input");
        assert!(error.to_string().contains("deterministic-replay-comparison-v1"));
    }
