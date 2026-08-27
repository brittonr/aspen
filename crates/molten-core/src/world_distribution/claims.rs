use std::collections::BTreeSet;

use super::*;
use crate::world_head::WorldHeadDecision;
use crate::world_head::WorldHeadPlanRequest;
use crate::world_head::classify_world_head_conflict;
use crate::world_head::plan_world_head_transition;

// r[impl molten.world_distribution.head_claims]
pub fn admit_remote_head_claims(
    request: &WorldClaimAdmissionRequest,
) -> Result<WorldClaimAdmission, Vec<WorldDistributionIssue>> {
    if request.max_claims == 0 || request.max_claims > MAX_WORLD_DISTRIBUTION_CLAIMS {
        return Err(vec![WorldDistributionIssue::InvalidBounds("max-claims")]);
    }
    if request.claims.len() > request.max_claims {
        return Err(vec![WorldDistributionIssue::ClaimLimitExceeded]);
    }
    let mut claims = request.claims.clone();
    claims.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));
    let mut seen = BTreeSet::new();
    for claim in &claims {
        validate_claim_envelope(claim)?;
        if !seen.insert(claim.claim_ref.clone()) {
            return Err(vec![WorldDistributionIssue::ClaimEnvelopeInvalid(format!(
                "duplicate-claim:{}",
                claim.claim_ref
            ))]);
        }
    }

    let claim_capacity = claims.len();
    let mut admitted = Vec::with_capacity(claim_capacity);
    let mut denied = Vec::with_capacity(claim_capacity);
    for remote in claims {
        let decision = plan_world_head_transition(&WorldHeadPlanRequest {
            claim_ref: remote.claim_ref.clone(),
            claim: remote.claim,
            current: request.current.clone(),
            history: request.history.clone(),
            policy: request.policy.clone(),
            authentication: remote.authentication,
            authority: remote.authority,
            currentness: remote.currentness,
            bounds: request.bounds.clone(),
        });
        match decision {
            WorldHeadDecision::Admitted(plan) => admitted.push(plan),
            WorldHeadDecision::Denied(mut issues) => {
                issues.sort();
                issues.dedup();
                denied.push(WorldClaimDenial {
                    claim_ref: remote.claim_ref,
                    issues,
                });
            }
            WorldHeadDecision::Conflict(conflict) => {
                return Err(vec![WorldDistributionIssue::ClaimConflictInvalid(conflict.conflict_ref)]);
            }
        }
    }
    admitted.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));
    denied.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));
    let conflict = classify_world_head_conflict(&admitted, request.bounds.max_conflicts)
        .map_err(|issues| vec![WorldDistributionIssue::ClaimConflictInvalid(format!("{issues:?}"))])?;
    Ok(WorldClaimAdmission {
        admitted,
        denied,
        conflict,
        selected_claim: None,
        head_mutation_authorized: false,
        non_claims: distribution_non_claims(),
    })
}

fn validate_claim_envelope(claim: &RemoteWorldHeadClaim) -> Result<(), Vec<WorldDistributionIssue>> {
    if claim.peer_ref.is_empty()
        || claim.peer_ref.len() > crate::dag_sync::MAX_DAG_DOMAIN_BYTES
        || claim.peer_ref.chars().any(char::is_control)
    {
        return Err(vec![WorldDistributionIssue::ClaimEnvelopeInvalid(
            claim.claim_ref.as_str().to_string(),
        )]);
    }
    if claim.encoded_bytes == 0 || claim.encoded_bytes > MAX_WORLD_DISTRIBUTION_CLAIM_BYTES {
        return Err(vec![WorldDistributionIssue::ClaimEnvelopeInvalid(format!(
            "claim-bytes:{}",
            claim.claim_ref
        ))]);
    }
    Ok(())
}
