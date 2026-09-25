
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
