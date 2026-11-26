use artifact_auth_core::AuthenticationPolicy;
use artifact_auth_core::KeyCurrentness;
use artifact_auth_ed25519::Ed25519Evidence;
use molten_core::world_head::WorldCommitHistoryNode;
use molten_core::world_head::WorldHeadAuthenticationDecisionRef;
use molten_core::world_head::WorldHeadAuthenticationObservation;
use molten_core::world_head::WorldHeadBounds;
use molten_core::world_head::WorldHeadClaim;
use molten_core::world_head::WorldHeadConflictSet;
use molten_core::world_head::WorldHeadCurrentnessObservation;
use molten_core::world_head::WorldHeadDecision;
use molten_core::world_head::WorldHeadIssue;
use molten_core::world_head::WorldHeadPlanRequest;
use molten_core::world_head::WorldHeadPolicy;
use molten_core::world_head::WorldHeadSignerObservation;
use molten_core::world_head::WorldHeadSignerRole;
use molten_core::world_head::WorldHeadStatementRef;
use molten_core::world_head::WorldHeadTransitionPlan;
use molten_core::world_head::classify_world_head_conflict;
use molten_core::world_head::plan_world_head_transition;

use super::CanonicalWorldHeadClaim;
use super::CanonicalWorldHeadConflict;
use super::CanonicalWorldHeadTransitionReceipt;
use super::WorldHeadArtifactAuthInput;
use super::WorldHeadAuthorityPort;
use super::WorldHeadConflictPort;
use super::WorldHeadFreshAdmission;
use super::WorldHeadMutationOutcome;
use super::WorldHeadPortError;
use super::WorldHeadReconciliationPort;
use super::WorldHeadSignatureCarrier;
use super::WorldHeadSignerAuthorityPort;
use super::WorldHeadVerifiedSigner;
use super::WorldHeadSigningPort;
use super::WorldHeadStatePort;
use super::WorldHeadTransitionReceiptInput;
use super::canonical_world_head_claim;
use super::canonical_world_head_conflict;
use super::canonical_world_head_transition_receipt;
use super::world_head_artifact_statement;
use super::world_head_authentication_scope;
use super::world_head_signer_role;
use crate::error::Failure;
use crate::error::Result;

const STATEMENT_SET_IDENTITY_DOMAIN: &str = "molten.world-head.statement-set.v1";
const DECISION_DENIED: &str = "denied";
const DECISION_CONFLICT: &str = "conflict";

#[derive(Debug, Clone)]
pub struct WorldHeadExecutionRequest {
    pub claim: WorldHeadClaim,
    pub history: Vec<WorldCommitHistoryNode>,
    pub policy: WorldHeadPolicy,
    pub authentication_policy: AuthenticationPolicy,
    pub signatures: Vec<WorldHeadSignatureCarrier>,
    pub currentness: WorldHeadCurrentnessObservation,
    pub bounds: WorldHeadBounds,
}

#[derive(Debug, Clone)]
pub struct WorldHeadAuthenticationResult {
    pub observation: WorldHeadAuthenticationObservation,
    pub statement_ref: WorldHeadStatementRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldHeadExecutionStatus {
    Applied,
    AlreadyApplied,
    Denied,
    Stale,
    Uncertain,
}

impl WorldHeadExecutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyApplied => "already-applied",
            Self::Denied => "denied",
            Self::Stale => "stale",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldHeadExecutionResult {
    pub status: WorldHeadExecutionStatus,
    pub plan: Option<WorldHeadTransitionPlan>,
    pub issues: Vec<WorldHeadIssue>,
    pub receipt: CanonicalWorldHeadTransitionReceipt,
}

pub fn sign_world_head_claim<S: WorldHeadSigningPort>(
    signer: &mut S,
    claim: &WorldHeadClaim,
    role: WorldHeadSignerRole,
) -> Result<(CanonicalWorldHeadClaim, WorldHeadSignatureCarrier, WorldHeadStatementRef)> {
    let canonical = canonical_world_head_claim(claim)?;
    let identity = signer.signer_identity(role, &claim.policy_ref).map_err(port_error)?;
    let (statement, statement_ref) = world_head_artifact_statement(&canonical, WorldHeadArtifactAuthInput {
        producer_id: &identity.producer_id,
        key_id: &identity.key_id,
        key_identity: identity.key_identity,
    })?;
    let signature = signer.sign_statement(&statement, role, &claim.policy_ref).map_err(port_error)?;
    Ok((canonical, signature, statement_ref))
}

/// Verifies detached carriers and derives every signer's role and authority.
///
/// A signer's role comes from the trusted policy key that its signed statement names, and its
/// authority comes from `signer_authority`; neither is taken from the unsigned carrier.
// r[impl molten.world_commit.world_head_truthful_receipts_and_verified_signers]
pub fn evaluate_world_head_authentication<P: WorldHeadSignerAuthorityPort + ?Sized>(
    claim: &CanonicalWorldHeadClaim,
    policy: &AuthenticationPolicy,
    carriers: &[WorldHeadSignatureCarrier],
    signer_authority: &mut P,
) -> Result<WorldHeadAuthenticationResult> {
    let scope = world_head_authentication_scope(claim)?;
    let statements = carriers
        .iter()
        .map(|carrier| {
            let key_identity = artifact_auth_ed25519::public_key_identity(&carrier.public_key_bytes);
            world_head_artifact_statement(claim, WorldHeadArtifactAuthInput {
                producer_id: &carrier.producer_id,
                key_id: &carrier.key_id,
                key_identity,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let evidence = statements
        .iter()
        .zip(carriers)
        .map(|((statement, _), carrier)| Ed25519Evidence {
            statement,
            generation: carrier.key_generation,
            public_key_bytes: &carrier.public_key_bytes,
            signature_bytes: &carrier.signature_bytes,
        })
        .collect::<Vec<_>>();
    let decision = artifact_auth_ed25519::verify_and_evaluate(policy, &scope, &evidence);
    let signers = statements
        .iter()
        .zip(carriers)
        .map(|((statement, _), carrier)| {
            signer_observation(
                SignerObservationInput {
                    policy,
                    claim: &claim.claim,
                    statement,
                    carrier,
                },
                &mut *signer_authority,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let statement_ref = statement_set_ref(&statements)?;
    let decision_ref = WorldHeadAuthenticationDecisionRef::new(format!("blake3:{}", decision.decision_blake3))
        .map_err(|error| Failure::invalid_harness(format!("invalid authentication decision ref: {error}")))?;
    let is_policy_matches = policy.profile_id == scope.profile_id
        && scope.verifier_context.digest_hex
            == crate::preserves_rail::content_ref_hex(claim.claim.policy_ref.as_str())?;
    Ok(WorldHeadAuthenticationResult {
        observation: WorldHeadAuthenticationObservation {
            statement_ref: statement_ref.clone(),
            decision_ref,
            passed: decision.passed,
            purpose_matches: scope.purpose == molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE,
            policy_matches: is_policy_matches,
            signers,
        },
        statement_ref,
    })
}

pub fn execute_world_head_transition<S, A>(
    store: &mut S,
    authority: &mut A,
    request: &WorldHeadExecutionRequest,
) -> Result<WorldHeadExecutionResult>
where
    S: WorldHeadStatePort + WorldHeadReconciliationPort,
    A: WorldHeadAuthorityPort,
{
    let canonical = canonical_world_head_claim(&request.claim)?;
    let current = store.read_head(&request.claim.branch_id).map_err(port_error)?;
    let authentication = evaluate_world_head_authentication(
        &canonical,
        &request.authentication_policy,
        &request.signatures,
        authority,
    )?;
    let authority_observation = authority
        .observe_authority(&request.claim.branch_id, &request.claim.policy_ref, request.claim.expected_generation)
        .map_err(port_error)?;
    let plan_request = WorldHeadPlanRequest {
        claim_ref: canonical.claim_ref.clone(),
        claim: request.claim.clone(),
        current,
        history: request.history.clone(),
        policy: request.policy.clone(),
        authentication: authentication.observation.clone(),
        authority: authority_observation.clone(),
        currentness: request.currentness.clone(),
        bounds: request.bounds.clone(),
    };
    let decision = plan_world_head_transition(&plan_request);
    let WorldHeadDecision::Admitted(plan) = decision else {
        return denied_execution(decision, &canonical, &authentication, authority_observation.authority_ref.as_str());
    };
    let receipt_context = OutcomeReceiptContext {
        plan: &plan,
        claim: &canonical,
        authentication: &authentication,
        authority_ref: authority_observation.authority_ref.as_str(),
    };
    // The store persists this receipt only inside the transaction that commits the successor
    // head, so a durable `applied` receipt is never written for a stale or failed mutation.
    // r[impl molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let applied_receipt = receipt_context.receipt(WorldHeadExecutionStatus::Applied, &[])?;
    let fresh_authentication_policy = &request.authentication_policy;
    let fresh_signatures = &request.signatures;
    let fresh_canonical = &canonical;
    let mutation = store
        .apply_transition(&plan, &applied_receipt, |observed| {
            if observed != plan.before.as_ref() {
                return Err(WorldHeadPortError::new("fresh-head-mismatch", "head changed inside mutation boundary"));
            }
            let fresh_authentication = evaluate_world_head_authentication(
                fresh_canonical,
                fresh_authentication_policy,
                fresh_signatures,
                &mut *authority,
            )
            .map_err(|error| WorldHeadPortError::new("fresh-authentication", error.to_string()))?;
            // Re-run pure admission over the fresh signer observations so a signer role, authority,
            // currentness, or threshold withdrawn since planning denies inside the mutation boundary.
            // Branch authority and its generation are rechecked by the store from `fresh_authority`.
            // r[impl molten.world_commit.world_head_truthful_receipts_and_verified_signers]
            let fresh_request = WorldHeadPlanRequest {
                current: observed.cloned(),
                authentication: fresh_authentication.observation,
                ..plan_request.clone()
            };
            let is_fresh_authentication_admitted = matches!(
                plan_world_head_transition(&fresh_request),
                WorldHeadDecision::Admitted(fresh_plan) if fresh_plan == plan
            );
            let fresh_authority = authority.observe_authority(
                &plan.after.branch_id,
                &plan.after.policy_ref,
                plan.before.as_ref().map_or(0, |state| state.generation),
            )?;
            Ok(WorldHeadFreshAdmission {
                authentication_passed: is_fresh_authentication_admitted,
                authority: fresh_authority,
            })
        })
        .map_err(port_error)?;
    // The returned receipt reflects the observed CAS outcome, never the pre-mutation admission.
    let (status, issues) = match mutation {
        WorldHeadMutationOutcome::Applied => (WorldHeadExecutionStatus::Applied, Vec::new()),
        WorldHeadMutationOutcome::AlreadyApplied => (WorldHeadExecutionStatus::AlreadyApplied, Vec::new()),
        WorldHeadMutationOutcome::Stale => (WorldHeadExecutionStatus::Stale, vec![WorldHeadIssue::StaleExpectedHead]),
        WorldHeadMutationOutcome::Uncertain => (WorldHeadExecutionStatus::Uncertain, Vec::new()),
    };
    let receipt = if status == WorldHeadExecutionStatus::Applied {
        applied_receipt
    } else {
        receipt_context.receipt(status, &issues)?
    };
    if status == WorldHeadExecutionStatus::Uncertain {
        store.record_uncertain_transition(&plan, &receipt).map_err(port_error)?;
    }
    Ok(WorldHeadExecutionResult {
        status,
        plan: Some(plan),
        issues,
        receipt,
    })
}

struct OutcomeReceiptContext<'a> {
    plan: &'a WorldHeadTransitionPlan,
    claim: &'a CanonicalWorldHeadClaim,
    authentication: &'a WorldHeadAuthenticationResult,
    authority_ref: &'a str,
}

impl OutcomeReceiptContext<'_> {
    fn receipt(
        &self,
        status: WorldHeadExecutionStatus,
        issues: &[WorldHeadIssue],
    ) -> Result<CanonicalWorldHeadTransitionReceipt> {
        transition_receipt(TransitionReceiptInput {
            decision: status.as_str(),
            plan: Some(self.plan),
            claim: self.claim,
            authentication: self.authentication,
            authority_ref: self.authority_ref,
            issues,
        })
    }
}

fn denied_execution(
    decision: WorldHeadDecision,
    canonical: &CanonicalWorldHeadClaim,
    authentication: &WorldHeadAuthenticationResult,
    authority_ref: &str,
) -> Result<WorldHeadExecutionResult> {
    let issues = decision_issues(decision);
    let receipt = transition_receipt(TransitionReceiptInput {
        decision: DECISION_DENIED,
        plan: None,
        claim: canonical,
        authentication,
        authority_ref,
        issues: &issues,
    })?;
    Ok(WorldHeadExecutionResult {
        status: WorldHeadExecutionStatus::Denied,
        plan: None,
        issues,
        receipt,
    })
}

pub fn record_world_head_conflict<S: WorldHeadConflictPort>(
    store: &mut S,
    plans: &[WorldHeadTransitionPlan],
    maximum: u32,
) -> Result<Option<(WorldHeadConflictSet, CanonicalWorldHeadConflict)>> {
    let conflict = classify_world_head_conflict(plans, maximum)
        .map_err(|issues| Failure::invalid_harness(format!("world-head conflict denied: {issues:?}")))?;
    let Some(conflict) = conflict else {
        return Ok(None);
    };
    let canonical = canonical_world_head_conflict(&conflict)?;
    store.record_conflict(&conflict, &canonical).map_err(port_error)?;
    Ok(Some((conflict, canonical)))
}
