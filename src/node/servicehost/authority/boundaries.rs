pub(crate) struct Verification<'a> {
    pub root: &'a crate::node_state::DirectoryRoot,
    pub now: u64,
    pub resource: &'a str,
    pub ability: &'a str,
    pub contract: &'a str,
    pub operation: basalt::ucan::OperationBinding,
}

pub(crate) fn validate_keys(trust: &super::wire::Trust) -> crate::error::Result<()> {
    let issuer = basalt::ucan::Ed25519PublicKey::from_bytes(trust.issuer_key).issuer().map_err(|_| super::denied())?;
    let holder = basalt::ucan::Ed25519PublicKey::from_bytes(trust.holder_key).issuer().map_err(|_| super::denied())?;
    if issuer.as_str() != trust.issuer || holder.as_str() != trust.holder {
        return Err(super::denied());
    }
    Ok(())
}

pub(crate) fn verify(
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
    input: Verification<'_>,
) -> crate::error::Result<String> {
    verify_grant(trust, credential, &input)?;
    verify_holder(trust, credential, input)
}

/// Rechecks current token, effective expiry, revocations and Basalt policy without replay
/// admission.
pub(crate) fn verify_grant(
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
    input: &Verification<'_>,
) -> crate::error::Result<u64> {
    let resolver = Keys(trust);
    let proofs = credential
        .proofs
        .iter()
        .map(|token| basalt::ucan::CompactToken::new(token).map_err(|_| super::denied()))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let proofs = basalt::ucan::ProofCollection::from_tokens(proofs);
    let revocations = basalt::ucan::RevocationList::from_references(
        trust.revoked.iter().map(|bytes| basalt::ucan::ProofReference::new(bytes.to_vec())).collect(),
    );
    let time = basalt::ucan::VerificationTime::from_unix_seconds(input.now);
    let limits = super::verification_limits();
    let token = basalt::ucan::verify_compact_token_str_with_resolvers_and_revocations(
        &credential.token,
        time,
        &resolver,
        &proofs,
        &revocations,
        limits,
    )
    .map_err(|_| super::denied())?;
    if token.issuer().as_str() != trust.issuer || token.audience().as_str() != trust.holder {
        return Err(super::denied());
    }
    let grant =
        match token.authorize_with_policies(input.resource, input.ability, &basalt::ucan::token::DenyUnknownCaveats) {
            basalt::ucan::AuthorizationDecision::Allowed { matched_capability } => matched_capability,
            basalt::ucan::AuthorizationDecision::Denied { .. } => return Err(super::denied()),
        };
    let policy = basalt::parse_policy_json(&trust.policy).map_err(|_| super::denied())?;
    let request = basalt::EnforcementRequest::new(input.contract, input.resource, input.ability)
        .with_capabilities(vec![basalt::CapabilityGrant::new(grant.resource, grant.ability)]);
    if !basalt::enforce(&policy, &request).map_err(|_| super::denied())?.is_allowed() {
        return Err(super::denied());
    }
    Ok(token.effective_delegation().time_bounds().expires_at_seconds())
}

fn verify_holder(
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
    input: Verification<'_>,
) -> crate::error::Result<String> {
    let resolver = Keys(trust);
    let proofs = credential
        .proofs
        .iter()
        .map(|token| basalt::ucan::CompactToken::new(token).map_err(|_| super::denied()))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let proofs = basalt::ucan::ProofCollection::from_tokens(proofs);
    let revocations = basalt::ucan::RevocationList::from_references(
        trust.revoked.iter().map(|bytes| basalt::ucan::ProofReference::new(bytes.to_vec())).collect(),
    );
    let time = basalt::ucan::VerificationTime::from_unix_seconds(input.now);
    let limits = super::verification_limits();
    let signature = basalt::ucan::SignerSignature::from_bytes(credential.signature);
    let nonce = basalt::ucan::HolderInvocationNonce::try_from_slice(&credential.nonce).map_err(|_| super::denied())?;
    let invocation = basalt::ucan::HolderInvocationRequest::new(
        &credential.token,
        input.resource,
        input.ability,
        input.operation,
        nonce,
        &signature,
    );
    let profiles = Profile(input.operation.profile());
    let replay = Replay(input.root);
    let token_context =
        basalt::ucan::HolderTokenVerificationContext::new(time, &resolver, &proofs, &revocations, limits);
    let policy_context = basalt::ucan::HolderInvocationPolicyContext::new(
        &basalt::ucan::token::DenyUnknownCaveats,
        &profiles,
        &replay,
        super::holder_limits()?,
    );
    let admitted = basalt::ucan::verify_holder_signed_invocation(&invocation, &token_context, &policy_context)
        .map_err(|_| super::denied())?;
    Ok(format!("blake3:{}", blake3::Hash::from(*admitted.request_digest().as_bytes())))
}

struct Keys<'a>(&'a super::wire::Trust);
impl basalt::ucan::KeyResolver for Keys<'_> {
    fn resolve_key(
        &self,
        issuer: &basalt::ucan::IssuerDid,
    ) -> Result<basalt::ucan::Ed25519PublicKey, basalt::ucan::KeyResolverError> {
        if issuer.as_str() == self.0.issuer {
            return Ok(basalt::ucan::Ed25519PublicKey::from_bytes(self.0.issuer_key));
        }
        if issuer.as_str() == self.0.holder {
            return Ok(basalt::ucan::Ed25519PublicKey::from_bytes(self.0.holder_key));
        }
        Err(basalt::ucan::KeyResolverError::MissingKey {
            issuer: issuer.as_str().to_owned(),
        })
    }
}

struct Profile(basalt::ucan::OperationProfileId);
impl basalt::ucan::OperationProfileAdmission for Profile {
    fn admit_operation_profile(
        &self,
        profile: &basalt::ucan::OperationProfileId,
    ) -> Result<(), basalt::ucan::OperationProfileAdmissionError> {
        if profile == &self.0 {
            Ok(())
        } else {
            Err(basalt::ucan::OperationProfileAdmissionError::UnknownProfile)
        }
    }
}

struct Replay<'a>(&'a crate::node_state::DirectoryRoot);
impl basalt::ucan::HolderInvocationReplayAdmission for Replay<'_> {
    fn admit_holder_invocation(
        &self,
        candidate: &basalt::ucan::HolderInvocationReplayCandidate<'_>,
    ) -> Result<(), basalt::ucan::HolderInvocationReplayError> {
        let key = crate::preserves_rail::record("node-service-host-replay-key-v1", vec![
            crate::preserves_rail::string(candidate.audience().as_str()),
            preserves::IOValue::bytes(*candidate.nonce().as_bytes()),
        ]);
        let key = crate::preserves_rail::canonical_hash(&key)
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        let path = crate::node_state::RelativePath::parse(&format!("service-host-replay-{}.preserves", &key[7..]))
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        let value = crate::preserves_rail::record("node-service-host-replay-admission-v1", vec![
            crate::preserves_rail::string(key),
            preserves::IOValue::bytes(*candidate.request_digest().as_bytes()),
        ]);
        let bytes = crate::preserves_rail::canonical_bytes(&value)
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        // Atomic no-replace publication with file and parent sync; concurrent processes cannot both win.
        self.0.write_new(&path, &bytes).map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)
    }
}
