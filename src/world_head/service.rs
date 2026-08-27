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
use super::WorldHeadSigningPort;
use super::WorldHeadStatePort;
use super::WorldHeadTransitionReceiptInput;
use super::canonical_world_head_claim;
use super::canonical_world_head_conflict;
use super::canonical_world_head_transition_receipt;
use super::world_head_artifact_statement;
use super::world_head_authentication_scope;
use crate::error::MoltenError;
use crate::error::Result;

const STATEMENT_SET_IDENTITY_DOMAIN: &str = "molten.world-head.statement-set.v1";
const DECISION_ADMITTED: &str = "admitted";
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

pub fn evaluate_world_head_authentication(
    claim: &CanonicalWorldHeadClaim,
    policy: &AuthenticationPolicy,
    carriers: &[WorldHeadSignatureCarrier],
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
        .map(|((statement, _), carrier)| signer_observation(policy, statement, carrier))
        .collect::<Vec<_>>();
    let statement_ref = statement_set_ref(&statements)?;
    let decision_ref = WorldHeadAuthenticationDecisionRef::new(format!("blake3:{}", decision.decision_blake3))
        .map_err(|error| MoltenError::invalid_harness(format!("invalid authentication decision ref: {error}")))?;
    let policy_matches = policy.profile_id == scope.profile_id
        && scope.verifier_context.digest_hex
            == crate::preserves_rail::content_ref_hex(claim.claim.policy_ref.as_str())?;
    Ok(WorldHeadAuthenticationResult {
        observation: WorldHeadAuthenticationObservation {
            statement_ref: statement_ref.clone(),
            decision_ref,
            passed: decision.passed,
            purpose_matches: scope.purpose == molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE,
            policy_matches,
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
    let authentication =
        evaluate_world_head_authentication(&canonical, &request.authentication_policy, &request.signatures)?;
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
        let issues = decision_issues(decision);
        let receipt = transition_receipt(
            DECISION_DENIED,
            None,
            &canonical,
            &authentication,
            authority_observation.authority_ref.as_str(),
            &issues,
        )?;
        return Ok(WorldHeadExecutionResult {
            status: WorldHeadExecutionStatus::Denied,
            plan: None,
            issues,
            receipt,
        });
    };
    let admitted_receipt = transition_receipt(
        DECISION_ADMITTED,
        Some(&plan),
        &canonical,
        &authentication,
        authority_observation.authority_ref.as_str(),
        &[],
    )?;
    let fresh_authentication_policy = &request.authentication_policy;
    let fresh_signatures = &request.signatures;
    let fresh_canonical = &canonical;
    let mutation = store
        .apply_transition(&plan, &admitted_receipt, |observed| {
            if observed != plan.before.as_ref() {
                return Err(WorldHeadPortError::new("fresh-head-mismatch", "head changed inside mutation boundary"));
            }
            let fresh_authentication =
                evaluate_world_head_authentication(fresh_canonical, fresh_authentication_policy, fresh_signatures)
                    .map_err(|error| WorldHeadPortError::new("fresh-authentication", error.to_string()))?;
            let fresh_authority = authority.observe_authority(
                &plan.after.branch_id,
                &plan.after.policy_ref,
                plan.before.as_ref().map_or(0, |state| state.generation),
            )?;
            Ok(WorldHeadFreshAdmission {
                authentication_passed: fresh_authentication.observation.passed,
                authority: fresh_authority,
            })
        })
        .map_err(port_error)?;
    let status = match mutation {
        WorldHeadMutationOutcome::Applied => WorldHeadExecutionStatus::Applied,
        WorldHeadMutationOutcome::AlreadyApplied => WorldHeadExecutionStatus::AlreadyApplied,
        WorldHeadMutationOutcome::Stale => WorldHeadExecutionStatus::Stale,
        WorldHeadMutationOutcome::Uncertain => {
            store.record_uncertain_transition(&plan, &admitted_receipt).map_err(port_error)?;
            WorldHeadExecutionStatus::Uncertain
        }
    };
    Ok(WorldHeadExecutionResult {
        status,
        plan: Some(plan),
        issues: Vec::new(),
        receipt: admitted_receipt,
    })
}

pub fn record_world_head_conflict<S: WorldHeadConflictPort>(
    store: &mut S,
    plans: &[WorldHeadTransitionPlan],
    maximum: u32,
) -> Result<Option<(WorldHeadConflictSet, CanonicalWorldHeadConflict)>> {
    let conflict = classify_world_head_conflict(plans, maximum)
        .map_err(|issues| MoltenError::invalid_harness(format!("world-head conflict denied: {issues:?}")))?;
    let Some(conflict) = conflict else {
        return Ok(None);
    };
    let canonical = canonical_world_head_conflict(&conflict)?;
    store.record_conflict(&conflict, &canonical).map_err(port_error)?;
    Ok(Some((conflict, canonical)))
}

fn signer_observation(
    policy: &AuthenticationPolicy,
    statement: &artifact_auth_core::ArtifactStatement,
    carrier: &WorldHeadSignatureCarrier,
) -> WorldHeadSignerObservation {
    let cryptographic =
        artifact_auth_ed25519::verify_statement(statement, &carrier.public_key_bytes, &carrier.signature_bytes);
    let trusted = policy.trusted_keys.iter().find(|trusted| trusted.key_identity == statement.key_identity);
    let currentness = trusted.map(|trusted| trusted.currentness);
    WorldHeadSignerObservation {
        key_identity_ref: format!("blake3:{}", statement.key_identity.digest_hex),
        role: carrier.role,
        authenticated: cryptographic.verified,
        current: matches!(currentness, Some(KeyCurrentness::Current | KeyCurrentness::VerificationOverlap)),
        revoked: matches!(currentness, Some(KeyCurrentness::Revoked)),
        authority_admitted: carrier.authority_admitted,
    }
}

fn statement_set_ref(
    statements: &[(artifact_auth_core::ArtifactStatement, WorldHeadStatementRef)],
) -> Result<WorldHeadStatementRef> {
    let mut refs = statements.iter().map(|(_, statement_ref)| statement_ref.as_str()).collect::<Vec<_>>();
    refs.sort_unstable();
    let mut hasher = blake3::Hasher::new_derive_key(STATEMENT_SET_IDENTITY_DOMAIN);
    let count = u64::try_from(refs.len()).map_err(|_| MoltenError::invalid_harness("statement set count overflow"))?;
    hasher.update(&count.to_le_bytes());
    for reference in refs {
        let length = u64::try_from(reference.len())
            .map_err(|_| MoltenError::invalid_harness("statement ref length overflow"))?;
        hasher.update(&length.to_le_bytes());
        hasher.update(reference.as_bytes());
    }
    WorldHeadStatementRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|error| MoltenError::invalid_harness(format!("statement set identity failed: {error}")))
}

fn transition_receipt(
    decision: &str,
    plan: Option<&WorldHeadTransitionPlan>,
    claim: &CanonicalWorldHeadClaim,
    authentication: &WorldHeadAuthenticationResult,
    authority_ref: &str,
    issues: &[WorldHeadIssue],
) -> Result<CanonicalWorldHeadTransitionReceipt> {
    let issue_codes = issues.iter().map(|issue| format!("{issue:?}")).collect::<Vec<_>>();
    canonical_world_head_transition_receipt(&WorldHeadTransitionReceiptInput {
        decision,
        plan,
        claim_ref: &claim.claim_ref,
        statement_ref: &authentication.statement_ref,
        authentication_decision_ref: authentication.observation.decision_ref.as_str(),
        authority_ref,
        issue_codes: &issue_codes,
    })
}

fn decision_issues(decision: WorldHeadDecision) -> Vec<WorldHeadIssue> {
    match decision {
        WorldHeadDecision::Denied(issues) => issues,
        WorldHeadDecision::Conflict(_) => vec![WorldHeadIssue::ConflictStateMismatch],
        WorldHeadDecision::Admitted(_) => Vec::new(),
    }
}

fn port_error(error: WorldHeadPortError) -> MoltenError {
    MoltenError::invalid_harness(format!("world-head port failed: {error}"))
}

pub fn conflict_receipt(
    claim: &CanonicalWorldHeadClaim,
    authentication: &WorldHeadAuthenticationResult,
    authority_ref: &str,
    conflict: &WorldHeadConflictSet,
) -> Result<CanonicalWorldHeadTransitionReceipt> {
    let issue_codes = vec![format!("conflict:{}", conflict.conflict_ref)];
    canonical_world_head_transition_receipt(&WorldHeadTransitionReceiptInput {
        decision: DECISION_CONFLICT,
        plan: None,
        claim_ref: &claim.claim_ref,
        statement_ref: &authentication.statement_ref,
        authentication_decision_ref: authentication.observation.decision_ref.as_str(),
        authority_ref,
        issue_codes: &issue_codes,
    })
}
