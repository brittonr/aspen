use super::WorldEffectIntent;
use super::WorldPromotionIssue;
use super::WorldPromotionOperationRef;
use super::WorldPromotionRequest;
use super::WorldPromotionRequestRef;
use super::WorldReleaseAttemptRef;

const PROMOTION_REQUEST_CONTEXT: &str = "onixresearch.molten.world-promotion.request.v1";
const PROMOTION_ATTEMPT_CONTEXT: &str = "onixresearch.molten.world-promotion.attempt.v1";

/// The four promotion identity roles, evaluated together at the origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionIdentity {
    pub operation_ref: WorldPromotionOperationRef,
    pub canonical_request_ref: WorldPromotionRequestRef,
    pub attempt_ref: WorldReleaseAttemptRef,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldPromotionRetryDecision {
    New,
    Replay,
    IncompatibleRequest,
}

/// Replay is bound to the logical operation and its canonical request. Two
/// distinct logical operations never merge, even when their request identities
/// or payload hashes are equal.
// r[impl molten.promotion_identity.request_binding]
// r[impl molten.promotion_identity.no_payload_merge]
pub fn evaluate_promotion_retry(
    candidate: &WorldPromotionIdentity,
    existing: Option<&WorldPromotionIdentity>,
) -> WorldPromotionRetryDecision {
    match existing {
        None => WorldPromotionRetryDecision::New,
        Some(existing) if existing.operation_ref != candidate.operation_ref => WorldPromotionRetryDecision::New,
        Some(existing) if existing.canonical_request_ref == candidate.canonical_request_ref => {
            WorldPromotionRetryDecision::Replay
        }
        Some(_) => WorldPromotionRetryDecision::IncompatibleRequest,
    }
}

// r[impl molten.promotion_identity.roles]
pub fn promotion_identity(
    request: &WorldPromotionRequest,
    intents: &[WorldEffectIntent],
    generation: u64,
) -> Result<WorldPromotionIdentity, Vec<WorldPromotionIssue>> {
    let canonical_request_ref = canonical_request_identity(request, intents)?;
    let attempt_ref = derived_reference(PROMOTION_ATTEMPT_CONTEXT, &[
        request.operation_ref.as_str(),
        canonical_request_ref.as_str(),
    ])?;
    Ok(WorldPromotionIdentity {
        operation_ref: request.operation_ref.clone(),
        canonical_request_ref,
        attempt_ref: WorldReleaseAttemptRef::new(attempt_ref)
            .map_err(|error| vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))])?,
        generation,
    })
}

fn canonical_request_identity(
    request: &WorldPromotionRequest,
    intents: &[WorldEffectIntent],
) -> Result<WorldPromotionRequestRef, Vec<WorldPromotionIssue>> {
    let mut hasher = blake3::Hasher::new_derive_key(PROMOTION_REQUEST_CONTEXT);
    update(&mut hasher, request.branch_id.as_str())?;
    update(&mut hasher, request.branch_class.as_str())?;
    update(&mut hasher, request.expected_head.as_str())?;
    update(&mut hasher, request.candidate_head.as_str())?;
    hasher.update(&request.expected_generation.to_be_bytes());
    update(&mut hasher, request.policy_ref.as_str())?;
    update(&mut hasher, request.authority.authority_ref.as_str())?;
    hasher.update(&request.authority.observed_generation.to_be_bytes());
    hasher.update(&[u8::from(request.authority.admitted)]);
    hasher.update(&[
        u8::from(request.intent_closure_complete),
        u8::from(request.simulation_only),
    ]);
    for intent in intents {
        update(&mut hasher, intent.intent_ref.as_str())?;
        update(&mut hasher, intent.semantic_ref.as_str())?;
        update(&mut hasher, intent.handler_ref.as_str())?;
        update(&mut hasher, intent.adapter_ref.as_str())?;
        let release_class = intent.release_class.ok_or_else(|| {
            vec![WorldPromotionIssue::IntentUnclassified(
                intent.intent_ref.as_str().to_string(),
            )]
        })?;
        update(&mut hasher, release_class.as_str())?;
    }
    WorldPromotionRequestRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|error| vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))])
}

fn derived_reference(context: &str, fields: &[&str]) -> Result<String, Vec<WorldPromotionIssue>> {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    for field in fields {
        update(&mut hasher, field)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), Vec<WorldPromotionIssue>> {
    let length = u64::try_from(value.len())
        .map_err(|_| vec![WorldPromotionIssue::TransactionalMapping("frame-length".to_string())])?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}
