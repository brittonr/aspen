use std::collections::BTreeSet;

use super::super::*;
use crate::world_commit::WorldCommitRef;

pub(super) const CURRENT_GENERATION: u64 = 1;
pub(super) const NEXT_GENERATION: u64 = 2;
pub(super) const MERGE_GENERATION: u64 = 3;
pub(super) const EXPECTED_CONFLICT_MEMBERS: usize = 2;
const SIGNATURE_THRESHOLD: usize = 1;

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(super) fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("world commit ref")
}

pub(super) fn policy_ref() -> WorldHeadPolicyRef {
    WorldHeadPolicyRef::new(reference("world-head-policy")).expect("policy ref")
}

pub(super) fn branch() -> WorldBranchId {
    WorldBranchId::new("main").expect("branch")
}

pub(super) fn history() -> Vec<WorldCommitHistoryNode> {
    vec![
        WorldCommitHistoryNode {
            commit: commit("root"),
            parents: Vec::new(),
        },
        WorldCommitHistoryNode {
            commit: commit("left"),
            parents: vec![commit("root")],
        },
        WorldCommitHistoryNode {
            commit: commit("right"),
            parents: vec![commit("root")],
        },
        WorldCommitHistoryNode {
            commit: commit("merge"),
            parents: vec![commit("left"), commit("right")],
        },
    ]
}

pub(super) fn policy() -> WorldHeadPolicy {
    WorldHeadPolicy {
        policy_ref: policy_ref(),
        allowed_branch_classes: BTreeSet::from([WorldBranchClass::Local]),
        allowed_purposes: BTreeSet::from([
            WorldHeadPurpose::Create,
            WorldHeadPurpose::Advance,
            WorldHeadPurpose::Merge,
            WorldHeadPurpose::Recovery,
        ]),
        allowed_signer_roles: BTreeSet::from([WorldHeadSignerRole::Maintainer, WorldHeadSignerRole::Recovery]),
        signature_threshold: SIGNATURE_THRESHOLD,
        max_conflicts: MAX_WORLD_HEAD_CONFLICTS,
        allow_recovery: true,
        require_independent_recovery_currentness: true,
    }
}

fn authentication() -> WorldHeadAuthenticationObservation {
    WorldHeadAuthenticationObservation {
        statement_ref: WorldHeadStatementRef::new(reference("statement")).expect("statement ref"),
        decision_ref: WorldHeadAuthenticationDecisionRef::new(reference("authentication-decision"))
            .expect("authentication decision ref"),
        passed: true,
        purpose_matches: true,
        policy_matches: true,
        signers: vec![WorldHeadSignerObservation {
            key_identity_ref: reference("key"),
            role: WorldHeadSignerRole::Maintainer,
            authenticated: true,
            current: true,
            revoked: false,
            authority_admitted: true,
        }],
    }
}

fn authority(generation: u64) -> WorldHeadAuthorityObservation {
    WorldHeadAuthorityObservation {
        authority_ref: WorldHeadAuthorityRef::new(reference("authority")).expect("authority ref"),
        policy_ref: policy_ref(),
        admitted: true,
        observed_generation: generation,
    }
}

fn currentness() -> WorldHeadCurrentnessObservation {
    WorldHeadCurrentnessObservation {
        durable_generation_observed: true,
        independent_ref: None,
    }
}

pub(super) fn advance_request() -> WorldHeadPlanRequest {
    WorldHeadPlanRequest {
        claim_ref: WorldHeadClaimRef::new(reference("advance-claim")).expect("claim ref"),
        claim: WorldHeadClaim {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            expected_head: Some(commit("root")),
            successor_head: commit("left"),
            expected_generation: CURRENT_GENERATION,
            successor_generation: NEXT_GENERATION,
            purpose: WorldHeadPurpose::Advance,
            policy_ref: policy_ref(),
            source_heads: Vec::new(),
        },
        current: Some(WorldHeadState {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            head: commit("root"),
            generation: CURRENT_GENERATION,
            policy_ref: policy_ref(),
        }),
        history: history(),
        policy: policy(),
        authentication: authentication(),
        authority: authority(CURRENT_GENERATION),
        currentness: currentness(),
        bounds: WorldHeadBounds::standard(),
    }
}

pub(super) fn admitted(request: &WorldHeadPlanRequest) -> WorldHeadTransitionPlan {
    match plan_world_head_transition(request) {
        WorldHeadDecision::Admitted(plan) => plan,
        decision => panic!("expected admitted plan, got {decision:?}"),
    }
}

pub(super) fn denied_issues(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    match plan_world_head_transition(request) {
        WorldHeadDecision::Denied(issues) => issues,
        decision => panic!("expected denial, got {decision:?}"),
    }
}
