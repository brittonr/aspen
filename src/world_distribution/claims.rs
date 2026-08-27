use molten_core::world_distribution::*;
use molten_core::world_head::WorldCommitHistoryNode;
use molten_core::world_head::WorldHeadAuthenticationObservation;
use molten_core::world_head::WorldHeadAuthorityObservation;
use molten_core::world_head::WorldHeadBounds;
use molten_core::world_head::WorldHeadClaim;
use molten_core::world_head::WorldHeadClaimRef;
use molten_core::world_head::WorldHeadCurrentnessObservation;
use molten_core::world_head::WorldHeadPolicy;
use molten_core::world_head::WorldHeadState;

use super::CanonicalWorldDistributionRecord;
use super::WorldDistributionReceiptPort;
use super::canonical_world_claim_admission;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimCarrier {
    pub peer_ref: String,
    pub claim_ref: WorldHeadClaimRef,
    pub claim: WorldHeadClaim,
    pub encoded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimAuthorityFacts {
    pub authority: WorldHeadAuthorityObservation,
    pub currentness: WorldHeadCurrentnessObservation,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimAdmissionContext {
    pub current: Option<WorldHeadState>,
    pub history: Vec<WorldCommitHistoryNode>,
    pub policy: WorldHeadPolicy,
    pub bounds: WorldHeadBounds,
    pub max_claims: usize,
}

pub trait WorldClaimTransportPort {
    fn receive_claims(&mut self, maximum: usize) -> Result<Vec<WorldClaimCarrier>>;
}

pub trait WorldClaimAuthenticationPort {
    fn authenticate_claim(&mut self, carrier: &WorldClaimCarrier) -> Result<WorldHeadAuthenticationObservation>;
}

pub trait WorldClaimAuthorityPort {
    fn observe_claim_authority(&mut self, carrier: &WorldClaimCarrier) -> Result<WorldClaimAuthorityFacts>;
}

pub struct WorldClaimPorts<'a, T, A, U, R> {
    pub transport: &'a mut T,
    pub authentication: &'a mut A,
    pub authority: &'a mut U,
    pub receipts: &'a mut R,
}

#[derive(Debug, Clone)]
pub struct WorldClaimExchangeOutcome {
    pub admission: WorldClaimAdmission,
    pub evidence_refs: Vec<String>,
    pub canonical_receipt: CanonicalWorldDistributionRecord,
}

// r[impl molten.world_distribution.head_claims]
pub fn run_world_claim_exchange<T, A, U, R>(
    context: &WorldClaimAdmissionContext,
    ports: WorldClaimPorts<'_, T, A, U, R>,
) -> Result<WorldClaimExchangeOutcome>
where
    T: WorldClaimTransportPort,
    A: WorldClaimAuthenticationPort,
    U: WorldClaimAuthorityPort,
    R: WorldDistributionReceiptPort,
{
    if context.max_claims == 0 || context.max_claims > MAX_WORLD_DISTRIBUTION_CLAIMS {
        return Err(MoltenError::invalid_harness("world claim maximum is invalid"));
    }
    let mut carriers = ports.transport.receive_claims(context.max_claims)?;
    if carriers.len() > context.max_claims {
        return Err(MoltenError::invalid_harness("world claim transport exceeded the requested bound"));
    }
    carriers.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));
    let mut claims = Vec::with_capacity(carriers.len());
    let mut evidence_refs = Vec::new();
    for carrier in carriers {
        let authentication = ports.authentication.authenticate_claim(&carrier)?;
        let authority = ports.authority.observe_claim_authority(&carrier)?;
        validate_evidence_ref(&carrier.peer_ref, "claim peer")?;
        validate_evidence_ref(authentication.statement_ref.as_str(), "claim statement")?;
        validate_evidence_ref(authentication.decision_ref.as_str(), "claim authentication")?;
        validate_evidence_ref(authority.authority.authority_ref.as_str(), "claim authority")?;
        validate_evidence_ref(&authority.evidence_ref, "claim authority evidence")?;
        evidence_refs.extend([
            carrier.peer_ref.clone(),
            authentication.statement_ref.as_str().to_string(),
            authentication.decision_ref.as_str().to_string(),
            authority.authority.authority_ref.as_str().to_string(),
            authority.evidence_ref,
        ]);
        claims.push(RemoteWorldHeadClaim {
            peer_ref: carrier.peer_ref,
            claim_ref: carrier.claim_ref,
            claim: carrier.claim,
            authentication,
            authority: authority.authority,
            currentness: authority.currentness,
            encoded_bytes: carrier.encoded_bytes,
        });
    }
    let admission = admit_remote_head_claims(&WorldClaimAdmissionRequest {
        claims,
        current: context.current.clone(),
        history: context.history.clone(),
        policy: context.policy.clone(),
        bounds: context.bounds.clone(),
        max_claims: context.max_claims,
    })
    .map_err(|issues| MoltenError::invalid_harness(format!("world claim admission denied: {issues:?}")))?;
    let canonical_receipt = canonical_world_claim_admission(&admission)?;
    ports.receipts.publish_world_distribution_receipt(&canonical_receipt)?;
    evidence_refs.push(canonical_receipt.record_ref.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(WorldClaimExchangeOutcome {
        admission,
        evidence_refs,
        canonical_receipt,
    })
}

fn validate_evidence_ref(reference: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness(format!("{field} is not a canonical content reference")))
}
