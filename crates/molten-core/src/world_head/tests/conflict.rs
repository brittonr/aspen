use super::super::*;
use super::support::*;

#[test]
fn competing_claims_are_preserved_without_order_based_selection() {
    // r[verify molten.world_heads.conflicts]
    let left = admitted(&advance_request());
    let mut right_request = advance_request();
    right_request.claim_ref = WorldHeadClaimRef::new(reference("right-claim")).expect("right claim ref");
    right_request.claim.successor_head = commit("right");
    let right = admitted(&right_request);

    let first = classify_world_head_conflict(&[left.clone(), right.clone()], MAX_WORLD_HEAD_CONFLICTS)
        .expect("conflict classification")
        .expect("conflict set");
    let second = classify_world_head_conflict(&[right, left], MAX_WORLD_HEAD_CONFLICTS)
        .expect("conflict classification")
        .expect("conflict set");
    assert_eq!(first, second);
    assert_eq!(first.members.len(), EXPECTED_CONFLICT_MEMBERS);
    assert!(first.conflict_ref.starts_with("blake3:"));
}

#[test]
fn malformed_history_and_bounds_are_rejected_before_choregraph_planning() {
    // r[verify molten.world_heads.verification]
    let mut missing = advance_request();
    missing.history[1].parents = vec![commit("absent")];
    assert!(denied_issues(&missing).contains(&WorldHeadIssue::MissingHistoryParent));

    let mut duplicate = advance_request();
    duplicate.history.push(duplicate.history[0].clone());
    assert!(denied_issues(&duplicate).contains(&WorldHeadIssue::DuplicateHistoryNode));

    let mut duplicate_parent = advance_request();
    duplicate_parent.history[1].parents.push(commit("root"));
    assert!(denied_issues(&duplicate_parent).contains(&WorldHeadIssue::DuplicateHistoryNode));

    let mut cycle = advance_request();
    cycle.history[0].parents = vec![commit("left")];
    assert!(denied_issues(&cycle).contains(&WorldHeadIssue::HistoryCycle));

    let mut bounded = advance_request();
    bounded.bounds.max_history_nodes = 1;
    assert!(denied_issues(&bounded).contains(&WorldHeadIssue::HistoryLimitExceeded));

    let mut wrong_purpose = advance_request();
    wrong_purpose.policy.allowed_purposes.remove(&WorldHeadPurpose::Advance);
    assert!(denied_issues(&wrong_purpose).contains(&WorldHeadIssue::PurposeDenied));
}

#[test]
fn branch_and_digest_references_reject_unsafe_or_malformed_values() {
    // r[verify molten.world_heads.verification]
    assert!(WorldBranchId::new("../main").is_err());
    assert!(WorldBranchId::new("Release").is_err());
    assert!(WorldHeadPolicyRef::new("sha256:bad").is_err());
    assert!(WorldHeadClaimRef::new("blake3:abcd").is_err());
}
