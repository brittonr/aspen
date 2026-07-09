
    #[derive(Clone, Copy)]
    enum IdentityMutation {
        Artifact,
        DependencyClosure,
        InitialState,
        Schema,
        Policy,
        Capability,
        Revocation,
        HandlerProfile,
        SeedOrEffectLog,
        Runtime,
        Tool,
        ReplayProfile,
    }

    const FRESHNESS_MUTATIONS: &[IdentityMutation] = &[
        IdentityMutation::Artifact,
        IdentityMutation::DependencyClosure,
        IdentityMutation::InitialState,
        IdentityMutation::Schema,
        IdentityMutation::Policy,
        IdentityMutation::Capability,
        IdentityMutation::Revocation,
        IdentityMutation::HandlerProfile,
        IdentityMutation::SeedOrEffectLog,
        IdentityMutation::Runtime,
        IdentityMutation::Tool,
        IdentityMutation::ReplayProfile,
    ];

    fn freshness_identity() -> ReplayRunIdentity {
        ReplayRunIdentity {
            artifact_ref: DEFAULT_ARTIFACT_REF.to_string(),
            dependency_closure_ref: DEFAULT_CLOSURE_REF.to_string(),
            initial_state_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            schema_refs: vec![DEFAULT_SCHEMA_REF.to_string()],
            policy_refs: vec![DEFAULT_POLICY_REF.to_string()],
            capability_refs: vec![DEFAULT_CAPABILITY_REF.to_string()],
            revocation_refs: vec![DEFAULT_REVOCATION_REF.to_string()],
            handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
            seed_or_effect_log_ref: DEFAULT_SEED_REF.to_string(),
            runtime_refs: vec![DEFAULT_RUNTIME_REF.to_string()],
            tool_refs: vec![DEFAULT_TOOL_REF.to_string()],
            replay_profile: "deterministic".to_string(),
        }
    }

    fn freshness_input(evidence_identity: ReplayRunIdentity) -> ReplayFreshnessInput {
        ReplayFreshnessInput {
            subject_ref: DEFAULT_ARTIFACT_REF.to_string(),
            evidence_ref: DEFAULT_TOOL_REF.to_string(),
            expected_identity: freshness_identity(),
            evidence_identity,
        }
    }

    fn mutated_identity(kind: IdentityMutation) -> (ReplayRunIdentity, &'static str) {
        let mut identity = freshness_identity();
        match kind {
            IdentityMutation::Artifact => {
                identity.artifact_ref = DEFAULT_CLOSURE_REF.to_string();
                (identity, "artifact-ref mismatch")
            }
            IdentityMutation::DependencyClosure => {
                identity.dependency_closure_ref = DEFAULT_ARTIFACT_REF.to_string();
                (identity, "dependency-closure-ref mismatch")
            }
            IdentityMutation::InitialState => {
                identity.initial_state_ref = DEFAULT_ARTIFACT_REF.to_string();
                (identity, "initial-state-ref mismatch")
            }
            IdentityMutation::Schema => {
                identity.schema_refs = vec![DEFAULT_ARTIFACT_REF.to_string()];
                (identity, "schema-refs mismatch")
            }
            IdentityMutation::Policy => {
                identity.policy_refs = vec![DEFAULT_REVOCATION_REF.to_string()];
                (identity, "policy-refs mismatch")
            }
            IdentityMutation::Capability => {
                identity.capability_refs = vec![DEFAULT_POLICY_REF.to_string()];
                (identity, "capability-refs mismatch")
            }
            IdentityMutation::Revocation => {
                identity.revocation_refs = vec![DEFAULT_CAPABILITY_REF.to_string()];
                (identity, "revocation-refs mismatch")
            }
            IdentityMutation::HandlerProfile => {
                identity.handler_profile_ref = DEFAULT_TOOL_REF.to_string();
                (identity, "handler-profile-ref mismatch")
            }
            IdentityMutation::SeedOrEffectLog => {
                identity.seed_or_effect_log_ref = DEFAULT_TOOL_REF.to_string();
                (identity, "seed-or-effect-log-ref mismatch")
            }
            IdentityMutation::Runtime => {
                identity.runtime_refs = vec![DEFAULT_TOOL_REF.to_string()];
                (identity, "runtime-refs mismatch")
            }
            IdentityMutation::Tool => {
                identity.tool_refs = vec![DEFAULT_RUNTIME_REF.to_string()];
                (identity, "tool-refs mismatch")
            }
            IdentityMutation::ReplayProfile => {
                identity.replay_profile = "release".to_string();
                (identity, "replay-profile mismatch")
            }
        }
    }

    #[test]
    fn replay_freshness_accepts_matching_identity() {
        let first = validate_replay_freshness(freshness_input(freshness_identity())).expect("freshness receipt");
        let second = validate_replay_freshness(freshness_input(freshness_identity())).expect("freshness receipt");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.freshness_ref, second.freshness_ref);
        let text = to_text(&first.value).expect("freshness text");
        assert!(text.contains("replay-freshness-validation-v1"));
        assert!(text.contains("evidence-only-no-authority"));
    }

    #[test]
    fn replay_freshness_denies_each_stale_identity_component() {
        for mutation in FRESHNESS_MUTATIONS {
            let (identity, expected_diagnostic) = mutated_identity(*mutation);
            let receipt = validate_replay_freshness(freshness_input(identity)).expect("freshness receipt");
            assert_eq!(receipt.decision, "deny");
            assert_eq!(receipt.diagnostics, vec![expected_diagnostic.to_string()]);
        }
    }

    #[test]
    fn replay_index_preserves_member_identity_refs() {
        let fixture = record_fixture_value().expect("fixture record");
        let verify = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("verify receipt");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(verify.receipt_ref.clone()),
            value: verify.value,
        }])
        .expect("rollup receipt");
        let index = index_replay_evidence(&[ReplayIndexInput {
            expected_ref: Some(rollup.rollup_ref.clone()),
            value: rollup.value,
        }])
        .expect("index receipt");
        assert_eq!(index.decision, "pass");
        let text = to_text(&index.value).expect("index text");
        assert!(text.contains("identity-refs"));
        assert!(text.contains(&fixture.identity_ref));
    }

    #[test]
    fn release_replay_freshness_and_catalog_terms_report_stale_components() {
        let (identity, expected_diagnostic) = mutated_identity(IdentityMutation::Policy);
        let receipt = validate_release_replay_freshness(freshness_input(identity)).expect("release freshness receipt");
        assert_eq!(receipt.decision, "deny");
        let terms = replay_freshness_catalog_terms(receipt);
        assert!(terms.iter().any(|term| term == "replay-freshness:deny"));
        assert!(terms.iter().any(|term| term == "evidence-only-no-authority"));
        assert!(terms
            .iter()
            .any(|term| term == &format!("stale-component:{expected_diagnostic}")));
    }
