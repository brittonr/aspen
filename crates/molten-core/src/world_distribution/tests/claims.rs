use super::super::*;
use super::support::*;

// r[verify molten.world_distribution.head_claims]
#[test]
fn competing_authenticated_claims_remain_a_conflict_without_selection() {
    let request = claim_request(true);
    let admission = admit_remote_head_claims(&request).expect("claim admission");
    assert_eq!(admission.admitted.len(), DESIRED_REPLICAS);
    assert!(admission.denied.is_empty());
    assert!(admission.conflict.is_some());
    assert!(admission.selected_claim.is_none());
    assert!(!admission.head_mutation_authorized);

    let denied = admit_remote_head_claims(&claim_request(false)).expect("denied claim set remains inspectable");
    assert!(denied.admitted.is_empty());
    assert_eq!(denied.denied.len(), DESIRED_REPLICAS);
    assert!(denied.conflict.is_none());
}
