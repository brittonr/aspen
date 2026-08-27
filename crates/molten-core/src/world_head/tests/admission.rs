use super::super::*;
use super::support::*;

#[test]
fn explicit_creation_and_linear_advance_are_stable_and_generation_fenced() {
    // r[verify molten.world_heads.cas]
    let mut create = advance_request();
    create.claim_ref = WorldHeadClaimRef::new(reference("create-claim")).expect("create claim ref");
    create.claim.expected_head = None;
    create.claim.successor_head = commit("root");
    create.claim.expected_generation = 0;
    create.claim.successor_generation = CURRENT_GENERATION;
    create.claim.purpose = WorldHeadPurpose::Create;
    create.current = None;
    create.authority.observed_generation = 0;
    let created = admitted(&create);
    assert!(created.before.is_none());
    assert_eq!(created.after.head, commit("root"));
    assert_eq!(created.after.generation, CURRENT_GENERATION);

    let request = advance_request();
    let first = admitted(&request);
    let second = admitted(&request);
    assert_eq!(first, second);
    assert_eq!(first.after.head, commit("left"));
    assert_eq!(first.after.generation, NEXT_GENERATION);
    assert_eq!(first.currentness, WorldHeadCurrentnessClass::RelativeToObservedStore);
}

#[test]
fn authorized_merge_requires_every_declared_immediate_source() {
    // r[verify molten.world_heads.claim]
    let mut request = advance_request();
    request.claim_ref = WorldHeadClaimRef::new(reference("merge-claim")).expect("merge claim ref");
    request.claim.expected_head = Some(commit("left"));
    request.claim.successor_head = commit("merge");
    request.claim.expected_generation = NEXT_GENERATION;
    request.claim.successor_generation = MERGE_GENERATION;
    request.claim.purpose = WorldHeadPurpose::Merge;
    request.claim.source_heads = vec![commit("left"), commit("right")];
    request.current = Some(WorldHeadState {
        branch_id: branch(),
        branch_class: WorldBranchClass::Local,
        head: commit("left"),
        generation: NEXT_GENERATION,
        policy_ref: policy_ref(),
    });
    request.authority.observed_generation = NEXT_GENERATION;

    let plan = admitted(&request);
    assert_eq!(plan.after.head, commit("merge"));
    assert_eq!(plan.after.generation, MERGE_GENERATION);

    request.claim.source_heads.pop();
    let issues = denied_issues(&request);
    assert!(issues.contains(&WorldHeadIssue::MergeNeedsMultipleSources));
}

#[test]
fn stale_unrelated_and_unobservable_claims_fail_closed() {
    // r[verify molten.world_heads.rollback]
    let mut stale_head = advance_request();
    stale_head.claim.expected_head = Some(commit("right"));
    assert!(denied_issues(&stale_head).contains(&WorldHeadIssue::StaleExpectedHead));

    let mut old = advance_request();
    old.claim.expected_generation = 0;
    old.claim.successor_generation = CURRENT_GENERATION;
    old.authority.observed_generation = 0;
    assert!(denied_issues(&old).contains(&WorldHeadIssue::OldGeneration));

    let mut skipped = advance_request();
    skipped.claim.successor_generation = MERGE_GENERATION;
    assert!(denied_issues(&skipped).contains(&WorldHeadIssue::SkippedGeneration));

    let mut unrelated = advance_request();
    unrelated.claim.successor_head = commit("right");
    unrelated
        .history
        .iter_mut()
        .find(|node| node.commit == commit("right"))
        .expect("right node")
        .parents
        .clear();
    assert!(denied_issues(&unrelated).contains(&WorldHeadIssue::UnrelatedSuccessor));

    let mut unavailable = advance_request();
    unavailable.currentness.durable_generation_observed = false;
    assert!(denied_issues(&unavailable).contains(&WorldHeadIssue::DurableGenerationUnavailable));
}

#[test]
fn authentication_authority_and_signer_failures_never_promote_a_claim() {
    // r[verify molten.world_heads.authentication]
    let mut denied = advance_request();
    denied.authentication.passed = false;
    denied.authority.admitted = false;
    let issues = denied_issues(&denied);
    assert!(issues.contains(&WorldHeadIssue::AuthenticationDenied));
    assert!(issues.contains(&WorldHeadIssue::AuthorityDenied));

    let mut revoked = advance_request();
    revoked.authentication.signers[0].revoked = true;
    let issues = denied_issues(&revoked);
    assert!(issues.contains(&WorldHeadIssue::SignerRevoked));
    assert!(issues.contains(&WorldHeadIssue::SignerThresholdMiss));

    let mut duplicate = advance_request();
    let duplicate_signer = duplicate.authentication.signers[0].clone();
    duplicate.authentication.signers.push(duplicate_signer);
    assert!(denied_issues(&duplicate).contains(&WorldHeadIssue::DuplicateSigner));

    let mut wrong_role = advance_request();
    wrong_role.authentication.signers[0].role = WorldHeadSignerRole::Release;
    assert!(denied_issues(&wrong_role).contains(&WorldHeadIssue::UnknownSignerRole));
}

#[test]
fn recovery_requires_independent_currentness_and_never_implies_whole_store_detection() {
    // r[verify molten.world_heads.rollback]
    let mut request = advance_request();
    request.claim_ref = WorldHeadClaimRef::new(reference("recovery-claim")).expect("recovery claim ref");
    request.claim.purpose = WorldHeadPurpose::Recovery;
    request.claim.successor_head = commit("right");
    request.authentication.signers[0].role = WorldHeadSignerRole::Recovery;
    let issues = denied_issues(&request);
    assert!(issues.contains(&WorldHeadIssue::IndependentCurrentnessRequired));

    request.currentness.independent_ref =
        Some(WorldHeadCurrentnessRef::new(reference("independent-currentness")).expect("currentness ref"));
    let plan = admitted(&request);
    assert_eq!(plan.currentness, WorldHeadCurrentnessClass::IndependentObservation);
}
