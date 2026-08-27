use std::collections::BTreeSet;

use super::WorldHeadIssue;
use super::WorldHeadPlanRequest;
use super::WorldHeadPurpose;

const VALIDATION_STAGE_COUNT: usize = 6;
const FIXED_ISSUE_CAPACITY: usize = 8;
const SIGNER_FIXED_ISSUE_CAPACITY: usize = 5;
const MINIMUM_MERGE_SOURCE_COUNT: usize = 2;

pub(super) fn validate_request(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(VALIDATION_STAGE_COUNT.saturating_mul(FIXED_ISSUE_CAPACITY));
    issues.extend(validate_bounds(request));
    issues.extend(validate_policy_and_claim(request));
    issues.extend(validate_generation(request));
    issues.extend(validate_authentication(request));
    issues.extend(validate_authority_currentness(request));
    issues.extend(validate_ancestry(request));
    issues
}

fn validate_bounds(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    if request.bounds.max_history_nodes == 0 {
        issues.push(WorldHeadIssue::InvalidBounds("max_history_nodes"));
    }
    if request.bounds.max_parents_per_commit == 0 {
        issues.push(WorldHeadIssue::InvalidBounds("max_parents_per_commit"));
    }
    if request.bounds.max_signers == 0 {
        issues.push(WorldHeadIssue::InvalidBounds("max_signers"));
    }
    if request.bounds.max_conflicts == 0 {
        issues.push(WorldHeadIssue::InvalidBounds("max_conflicts"));
    }
    if request.policy.signature_threshold == 0 || request.policy.signature_threshold > request.bounds.max_signers {
        issues.push(WorldHeadIssue::InvalidBounds("signature_threshold"));
    }
    if request.policy.max_conflicts == 0 || request.policy.max_conflicts > request.bounds.max_conflicts {
        issues.push(WorldHeadIssue::InvalidBounds("max_conflicts"));
    }
    issues
}

fn validate_policy_and_claim(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    if request.claim.policy_ref != request.policy.policy_ref {
        issues.push(WorldHeadIssue::PolicyMismatch);
    }
    if !request.policy.allowed_branch_classes.contains(&request.claim.branch_class) {
        issues.push(WorldHeadIssue::BranchClassMismatch);
    }
    if !request.policy.allowed_purposes.contains(&request.claim.purpose) {
        issues.push(WorldHeadIssue::PurposeDenied);
    }
    if request.claim.purpose == WorldHeadPurpose::Recovery && !request.policy.allow_recovery {
        issues.push(WorldHeadIssue::RecoveryDenied);
    }
    match (&request.current, request.claim.purpose) {
        (None, WorldHeadPurpose::Create) => {}
        (Some(_), WorldHeadPurpose::Create) => {
            issues.push(WorldHeadIssue::CurrentHeadUnexpected);
        }
        (None, _) => issues.push(WorldHeadIssue::CurrentHeadRequired),
        (Some(current), _) => {
            if current.branch_id != request.claim.branch_id {
                issues.push(WorldHeadIssue::BranchMismatch);
            }
            if current.branch_class != request.claim.branch_class {
                issues.push(WorldHeadIssue::BranchClassMismatch);
            }
            if current.policy_ref != request.claim.policy_ref {
                issues.push(WorldHeadIssue::PolicyMismatch);
            }
        }
    }
    issues
}

fn validate_generation(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    let Some(next_generation) = request.claim.expected_generation.checked_add(1) else {
        issues.push(WorldHeadIssue::GenerationOverflow);
        return issues;
    };
    if request.claim.successor_generation < next_generation {
        issues.push(WorldHeadIssue::RepeatedGeneration);
    } else if request.claim.successor_generation > next_generation {
        issues.push(WorldHeadIssue::SkippedGeneration);
    }

    if request.claim.purpose == WorldHeadPurpose::Create {
        if request.claim.expected_head.is_some() {
            issues.push(WorldHeadIssue::CurrentHeadUnexpected);
        }
        if request.claim.expected_generation != 0 {
            issues.push(WorldHeadIssue::SkippedGeneration);
        }
        return issues;
    }

    let Some(current) = request.current.as_ref() else {
        return issues;
    };
    let Some(expected_head) = request.claim.expected_head.as_ref() else {
        issues.push(WorldHeadIssue::ExpectedHeadMissing);
        return issues;
    };
    if expected_head != &current.head {
        issues.push(WorldHeadIssue::StaleExpectedHead);
    }
    if request.claim.expected_generation < current.generation {
        issues.push(WorldHeadIssue::OldGeneration);
    } else if request.claim.expected_generation > current.generation {
        issues.push(WorldHeadIssue::SkippedGeneration);
    }
    issues
}

fn validate_authentication(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let capacity = request.bounds.max_signers.saturating_add(SIGNER_FIXED_ISSUE_CAPACITY);
    let mut issues = Vec::with_capacity(capacity);
    let observation = &request.authentication;
    if !observation.passed {
        issues.push(WorldHeadIssue::AuthenticationDenied);
    }
    if !observation.purpose_matches {
        issues.push(WorldHeadIssue::AuthenticationPurposeMismatch);
    }
    if !observation.policy_matches {
        issues.push(WorldHeadIssue::AuthenticationPolicyMismatch);
    }
    if observation.signers.len() > request.bounds.max_signers {
        issues.push(WorldHeadIssue::SignerLimitExceeded);
    }

    let mut seen = BTreeSet::new();
    let mut admitted = 0_usize;
    for signer in &observation.signers {
        if seen.len() >= request.bounds.max_signers {
            issues.push(WorldHeadIssue::SignerLimitExceeded);
            break;
        }
        if !seen.insert(signer.key_identity_ref.as_str()) {
            issues.push(WorldHeadIssue::DuplicateSigner);
            continue;
        }
        let mut is_signer_admitted = true;
        if !signer.authenticated {
            issues.push(WorldHeadIssue::AuthenticationDenied);
            is_signer_admitted = false;
        }
        if !request.policy.allowed_signer_roles.contains(&signer.role) {
            issues.push(WorldHeadIssue::UnknownSignerRole);
            is_signer_admitted = false;
        }
        if !signer.current {
            issues.push(WorldHeadIssue::SignerNotCurrent);
            is_signer_admitted = false;
        }
        if signer.revoked {
            issues.push(WorldHeadIssue::SignerRevoked);
            is_signer_admitted = false;
        }
        if !signer.authority_admitted {
            issues.push(WorldHeadIssue::SignerAuthorityDenied);
            is_signer_admitted = false;
        }
        if is_signer_admitted {
            admitted = admitted.saturating_add(1);
        }
    }
    if admitted < request.policy.signature_threshold {
        issues.push(WorldHeadIssue::SignerThresholdMiss);
    }
    issues
}

fn validate_authority_currentness(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    if !request.authority.admitted {
        issues.push(WorldHeadIssue::AuthorityDenied);
    }
    if request.authority.policy_ref != request.claim.policy_ref {
        issues.push(WorldHeadIssue::AuthorityPolicyMismatch);
    }
    if request.authority.observed_generation != request.claim.expected_generation {
        issues.push(WorldHeadIssue::AuthorityGenerationMismatch);
    }
    if !request.currentness.durable_generation_observed {
        issues.push(WorldHeadIssue::DurableGenerationUnavailable);
    }
    if request.claim.purpose == WorldHeadPurpose::Recovery
        && request.policy.require_independent_recovery_currentness
        && request.currentness.independent_ref.is_none()
    {
        issues.push(WorldHeadIssue::IndependentCurrentnessRequired);
    }
    issues
}

fn validate_ancestry(request: &WorldHeadPlanRequest) -> Vec<WorldHeadIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    let successor = request.history.iter().find(|node| node.commit == request.claim.successor_head);
    let Some(successor) = successor else {
        issues.push(WorldHeadIssue::SuccessorMissing);
        return issues;
    };
    match request.claim.purpose {
        WorldHeadPurpose::Create | WorldHeadPurpose::Recovery => {}
        WorldHeadPurpose::Advance => {
            let Some(expected) = request.claim.expected_head.as_ref() else {
                issues.push(WorldHeadIssue::ExpectedHeadMissing);
                return issues;
            };
            if !successor.parents.contains(expected) {
                issues.push(WorldHeadIssue::UnrelatedSuccessor);
            }
            if !request.claim.source_heads.is_empty() {
                issues.push(WorldHeadIssue::MergeSourceMissing);
            }
        }
        WorldHeadPurpose::Merge => {
            let unique = request.claim.source_heads.iter().collect::<BTreeSet<_>>();
            if unique.len() != request.claim.source_heads.len() {
                issues.push(WorldHeadIssue::DuplicateMergeSource);
            }
            if unique.len() < MINIMUM_MERGE_SOURCE_COUNT {
                issues.push(WorldHeadIssue::MergeNeedsMultipleSources);
            }
            let is_expected_present =
                request.claim.expected_head.as_ref().is_some_and(|expected| unique.contains(expected));
            if !is_expected_present || unique.iter().any(|source| !successor.parents.contains(source)) {
                issues.push(WorldHeadIssue::MergeSourceMissing);
            }
        }
    }
    issues
}
