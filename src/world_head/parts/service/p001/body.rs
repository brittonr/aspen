
struct SignerObservationInput<'a> {
    policy: &'a AuthenticationPolicy,
    claim: &'a WorldHeadClaim,
    statement: &'a artifact_auth_core::ArtifactStatement,
    carrier: &'a WorldHeadSignatureCarrier,
}

/// Observes one signer without trusting unsigned carrier metadata.
///
/// Trust is the policy key whose identity, producer, and key id the signed statement carries; its
/// policy-bound key id names the role. Only a cryptographically verified, current, unrevoked,
/// role-bearing signer is offered to the authority port, which alone decides authority.
// r[impl molten.world_commit.world_head_truthful_receipts_and_verified_signers]
fn signer_observation<P: WorldHeadSignerAuthorityPort + ?Sized>(
    input: SignerObservationInput<'_>,
    signer_authority: &mut P,
) -> Result<WorldHeadSignerObservation> {
    let SignerObservationInput {
        policy,
        claim,
        statement,
        carrier,
    } = input;
    let cryptographic =
        artifact_auth_ed25519::verify_statement(statement, &carrier.public_key_bytes, &carrier.signature_bytes);
    let trusted = policy.trusted_keys.iter().find(|trusted| {
        trusted.key_identity == statement.key_identity
            && trusted.producer_id == statement.producer_id
            && trusted.key_id == statement.key_id
    });
    let currentness = trusted.map(|trusted| trusted.currentness);
    let is_current = matches!(currentness, Some(KeyCurrentness::Current | KeyCurrentness::VerificationOverlap));
    let is_revoked = matches!(currentness, Some(KeyCurrentness::Revoked));
    let role = trusted.and_then(|trusted| world_head_signer_role(&trusted.key_id));
    let key_identity_ref = format!("blake3:{}", statement.key_identity.digest_hex);
    let is_authority_admitted = match (trusted, role) {
        (Some(trusted), Some(role)) if cryptographic.verified && is_current && !is_revoked => signer_authority
            .signer_authority(claim, &WorldHeadVerifiedSigner {
                key_identity_ref: &key_identity_ref,
                producer_id: &trusted.producer_id,
                key_id: &trusted.key_id,
                role,
            })
            .map_err(port_error)?,
        _ => false,
    };
    Ok(WorldHeadSignerObservation {
        key_identity_ref,
        role,
        authenticated: cryptographic.verified,
        current: is_current,
        revoked: is_revoked,
        authority_admitted: is_authority_admitted,
    })
}

fn statement_set_ref(
    statements: &[(artifact_auth_core::ArtifactStatement, WorldHeadStatementRef)],
) -> Result<WorldHeadStatementRef> {
    let mut refs = statements.iter().map(|(_, statement_ref)| statement_ref.as_str()).collect::<Vec<_>>();
    refs.sort_unstable();
    let mut hasher = blake3::Hasher::new_derive_key(STATEMENT_SET_IDENTITY_DOMAIN);
    let count = u64::try_from(refs.len()).map_err(|_| Failure::invalid_harness("statement set count overflow"))?;
    hasher.update(&count.to_le_bytes());
    for reference in refs {
        let length = u64::try_from(reference.len())
            .map_err(|_| Failure::invalid_harness("statement ref length overflow"))?;
        hasher.update(&length.to_le_bytes());
        hasher.update(reference.as_bytes());
    }
    WorldHeadStatementRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|error| Failure::invalid_harness(format!("statement set identity failed: {error}")))
}

struct TransitionReceiptInput<'a> {
    decision: &'a str,
    plan: Option<&'a WorldHeadTransitionPlan>,
    claim: &'a CanonicalWorldHeadClaim,
    authentication: &'a WorldHeadAuthenticationResult,
    authority_ref: &'a str,
    issues: &'a [WorldHeadIssue],
}

fn transition_receipt(input: TransitionReceiptInput<'_>) -> Result<CanonicalWorldHeadTransitionReceipt> {
    let TransitionReceiptInput {
        decision,
        plan,
        claim,
        authentication,
        authority_ref,
        issues,
    } = input;
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

fn port_error(error: WorldHeadPortError) -> Failure {
    Failure::invalid_harness(format!("world-head port failed: {error}"))
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
