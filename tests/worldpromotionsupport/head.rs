use molten::world_head::LocalWorldHeadStore;
use molten::world_head::WorldHeadFreshAdmission;
use molten::world_head::WorldHeadStatePort;
use molten::world_head::WorldHeadTransitionReceiptInput;
use molten::world_head::canonical_world_head_transition_receipt;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldHeadAuthorityObservation;
use molten_core::world_head::WorldHeadClaimRef;
use molten_core::world_head::WorldHeadCurrentnessClass;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadStatementRef;
use molten_core::world_head::WorldHeadTransitionPlan;

use super::*;

pub fn seed(root: &DirectoryRoot, policy_ref: &WorldHeadPolicyRef) {
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let mut heads = LocalWorldHeadStore::open(&storage).expect("head store");
    let state = WorldHeadState {
        branch_id: WorldBranchId::new("release").expect("branch"),
        branch_class: WorldBranchClass::Candidate,
        head: WorldCommitRef::new(reference("active")).expect("head"),
        generation: 1,
        policy_ref: policy_ref.clone(),
    };
    let claim_ref = WorldHeadClaimRef::new(reference("seed-claim")).expect("claim");
    let plan = WorldHeadTransitionPlan {
        claim_ref: claim_ref.clone(),
        before: None,
        after: state,
        choregraph_before_identity: None,
        choregraph_after_identity: reference("seed-choregraph"),
        currentness: WorldHeadCurrentnessClass::RelativeToObservedStore,
    };
    let statement_ref = WorldHeadStatementRef::new(reference("seed-statement")).expect("statement");
    let receipt = canonical_world_head_transition_receipt(&WorldHeadTransitionReceiptInput {
        decision: "admitted",
        plan: Some(&plan),
        claim_ref: &claim_ref,
        statement_ref: &statement_ref,
        authentication_decision_ref: &reference("seed-authentication"),
        authority_ref: &reference("seed-authority"),
        issue_codes: &[],
    })
    .expect("receipt");
    heads
        .apply_transition(&plan, &receipt, |_observed| {
            Ok(WorldHeadFreshAdmission {
                authentication_passed: true,
                authority: WorldHeadAuthorityObservation {
                    authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(reference("seed-authority"))
                        .expect("authority"),
                    policy_ref: policy_ref.clone(),
                    admitted: true,
                    observed_generation: 0,
                },
            })
        })
        .expect("seed");
}

pub fn assert_unchanged(root: &DirectoryRoot) {
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let store = LocalWorldPromotionStore::open(&storage).expect("store");
    assert!(store.list_reservations().expect("reservations").is_empty());
    let head = store.read_head(&WorldBranchId::new("release").expect("branch")).expect("head").expect("head");
    assert_eq!(head.head.as_str(), reference("active"));
    assert!(
        storage
            .existing_subdir(&RelativePath::parse("world-promotion/receipts").expect("path"))
            .expect("dir")
            .is_none_or(|dir| dir.list_entries().expect("entries").is_empty())
    );
}
