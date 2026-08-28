use super::*;

// r[verify molten.prolly_map.profile]
#[test]
fn exact_profile_is_stable_and_structural_drift_fails_closed() {
    let profile = profile();
    assert!(validate_profile(&profile).is_empty());
    assert_eq!(derive_profile_ref(&profile).expect("profile ref"), profile.profile_ref);

    let mut missing_seed = profile.clone();
    missing_seed.boundary_seed_ref.clear();
    assert!(
        validate_profile(&missing_seed)
            .iter()
            .any(|issue| matches!(issue, ProllyIssue::MalformedReference(_)))
    );

    let mut bad_bounds = profile.clone();
    bad_bounds.target_node_bytes = bad_bounds.max_node_bytes;
    bad_bounds.profile_ref = derive_profile_ref(&bad_bounds).expect("drifted identity");
    assert!(validate_profile(&bad_bounds).contains(&ProllyIssue::ProfileBoundInvalid("node-bytes")));

    let mut wrong_domain = profile;
    wrong_domain.leaf_domain = "unreviewed:v1".to_string();
    wrong_domain.profile_ref = derive_profile_ref(&wrong_domain).expect("wrong domain identity");
    assert!(validate_profile(&wrong_domain).contains(&ProllyIssue::ProfileFieldMismatch("leaf-domain")));
}

// r[verify molten.prolly_map.proof_boundary]
#[test]
fn proof_obligations_are_closed_scoped_and_do_not_overclaim() {
    let obligations = standard_proof_obligations();
    assert!(validate_proof_obligations(&obligations));
    assert!(obligations.iter().any(|item| item.status == ProofObligationStatus::OpenFormal));
    assert!(
        obligations
            .iter()
            .all(|item| !item.proves_collision_impossibility && !item.proves_database_correctness)
    );
    assert_eq!(TRELLIS_PROOF_REFERENCE_REVISION.len(), GIT_REVISION_HEX_CHARS);
}
