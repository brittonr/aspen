pub(super) fn validate_keys(trust: &super::wire::Trust) -> crate::error::Result<()> {
    let issuer = basalt::ucan::Ed25519PublicKey::from_bytes(trust.issuer_key).issuer().map_err(|_| super::denied())?;
    let holder = basalt::ucan::Ed25519PublicKey::from_bytes(trust.holder_key).issuer().map_err(|_| super::denied())?;
    if issuer.as_str() != trust.issuer
        || holder.as_str() != trust.holder
        || trust.key_generation == 0
        || trust.revoked.len() > 4096
        || trust.not_before >= trust.expires
    {
        return Err(super::denied());
    }
    Ok(())
}

pub(super) fn authorize<C>(
    authority: &super::Current<C>,
    claim: &crate::world_head::CanonicalWorldHeadClaim,
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
    now: u64,
) -> crate::error::Result<String> {
    if credential.proofs.len() > 32 {
        return Err(super::denied());
    }
    let keys = Keys(trust);
    let proofs = credential
        .proofs
        .iter()
        .map(|token| basalt::ucan::CompactToken::new(token).map_err(|_| super::denied()))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let proofs = basalt::ucan::ProofCollection::from_tokens(proofs);
    let revocations = super::state::current(&authority.revocations, trust, now)?;
    let time = basalt::ucan::VerificationTime::from_unix_seconds(now);
    let limits = super::verification_limits();
    let token = basalt::ucan::verify_compact_token_str_with_resolvers_and_revocations(
        &credential.token,
        time,
        &keys,
        &proofs,
        &revocations,
        limits,
    )
    .map_err(|_| super::denied())?;
    if token.issuer().as_str() != trust.issuer || token.audience().as_str() != trust.holder {
        return Err(super::denied());
    }
    let resource = super::resource(&claim.claim.branch_id);
    let grant = match token.authorize_with_policies(&resource, "mutate", &basalt::ucan::token::DenyUnknownCaveats) {
        basalt::ucan::AuthorizationDecision::Allowed { matched_capability } => matched_capability,
        basalt::ucan::AuthorizationDecision::Denied { .. } => return Err(super::denied()),
    };
    let policy = basalt::parse_policy_json(&trust.policy).map_err(|_| super::denied())?;
    let request = basalt::EnforcementRequest::new("world-head-witness", &resource, "mutate")
        .with_capabilities(vec![basalt::CapabilityGrant::new(grant.resource, grant.ability)]);
    if !basalt::enforce(&policy, &request).map_err(|_| super::denied())?.is_allowed() {
        return Err(super::denied());
    }
    verify_holder(authority, claim, credential, Context {
        keys,
        proofs,
        revocations,
        time,
    })
}

struct Context<'a> {
    keys: Keys<'a>,
    proofs: basalt::ucan::ProofCollection,
    revocations: basalt::ucan::DurableRevocationChecker<'a, durable_authority_state_redb::view::RevocationRedbStore>,
    time: basalt::ucan::VerificationTime,
}

fn verify_holder<C>(
    authority: &super::Current<C>,
    claim: &crate::world_head::CanonicalWorldHeadClaim,
    credential: &super::wire::Credential,
    context: Context<'_>,
) -> crate::error::Result<String> {
    let signature = basalt::ucan::SignerSignature::from_bytes(
        credential.signature.as_slice().try_into().map_err(|_| super::denied())?,
    );
    let nonce = basalt::ucan::HolderInvocationNonce::try_from_slice(&credential.nonce).map_err(|_| super::denied())?;
    let resource = super::resource(&claim.claim.branch_id);
    let operation = super::operation_binding(claim)?;
    let request = basalt::ucan::HolderInvocationRequest::new(
        &credential.token,
        &resource,
        "mutate",
        operation,
        nonce,
        &signature,
    );
    let profiles = Profile(operation.profile());
    let replay = super::replay::Guard {
        store: &authority.replay,
        trust: context.keys.0,
        claim_ref: claim.claim_ref.as_str(),
    };
    let token = basalt::ucan::HolderTokenVerificationContext::new(
        context.time,
        &context.keys,
        &context.proofs,
        &context.revocations,
        super::verification_limits(),
    );
    let policy = basalt::ucan::HolderInvocationPolicyContext::new(
        &basalt::ucan::token::DenyUnknownCaveats,
        &profiles,
        &replay,
        super::holder_limits()?,
    );
    let admitted =
        basalt::ucan::verify_holder_signed_invocation(&request, &token, &policy).map_err(|_| super::denied())?;
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
