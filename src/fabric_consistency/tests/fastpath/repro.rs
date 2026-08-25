use super::*;

#[test]
fn repro_bundle_preserves_model_only_claim_and_expected_failure() {
    let expected = InvariantViolation::ConflictingPredecessor("stale-command".to_owned());
    let evidence = ModelRunEvidence {
        profile_ref: "blake3:profile".to_owned(),
        source_revision: JETPACK_ARTIFACT_REVISION.to_owned(),
        claim_profile: MODEL_ONLY_CLAIM.to_owned(),
        steps: vec![ModelStep {
            sequence: FIRST_DIVERGENCE,
            kind: ModelStepKind::OriginalCommit,
            operation_ref: Some("stale-command".to_owned()),
            view: NEXT_VIEW,
            causal: true,
        }],
        violations: vec![expected.clone()],
        coverage: Coverage {
            explored_transitions: 1,
            eligible_transitions: 1,
        },
        first_divergence: Some(FIRST_DIVERGENCE),
        non_claims: strings(required_non_claims()),
    };
    let bundle = export_repro_bundle(&evidence, expected.clone()).expect("model repro bundle");
    assert_eq!(bundle.expected_violation, expected);
    assert_eq!(bundle.claim_profile, MODEL_ONLY_CLAIM);
    assert!(!bundle.live_engine_claim);
    assert!(!bundle.measured_performance_claim);
}
