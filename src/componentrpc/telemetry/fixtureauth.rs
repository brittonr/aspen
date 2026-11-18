//! Deterministic actual credentials for telemetry fixtures only; no production fallback.
use basalt::ucan::TokenSigner;

pub const POLICY: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/component-rpc/telemetry-policy.json"));
pub fn lease(scope: &super::Scope, nonce: u8, allowed: bool) -> super::Result<super::Lease> {
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([81; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([82; 32]);
    let audience = basalt::ucan::AudienceDid::from(holder.issuer().map_err(super::super::failure)?);
    let issuer_id = issuer.issuer().map_err(super::super::failure)?;
    let keys = basalt::ucan::KeyResolutionContext::new(vec![
        issuer.verification_key().map_err(super::super::failure)?,
        holder.verification_key().map_err(super::super::failure)?,
    ]);
    let resource = super::resource(scope);
    let ability = if allowed {
        super::ABILITY
    } else {
        super::super::authority::ABILITY
    };
    let capabilities = basalt::ucan::CapabilitySet::new(vec![
        basalt::ucan::CapabilityDocument::new(resource.clone(), ability.into()).map_err(super::super::failure)?,
    ])
    .map_err(super::super::failure)?;
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(100, 200).map_err(super::super::failure)?,
    )
    .map_err(super::super::failure)?;
    let nonce = basalt::ucan::HolderInvocationNonce::from_bytes([nonce; 32]);
    let signature = basalt::ucan::sign_holder_invocation(
        &basalt::ucan::HolderInvocationSigningRequest::new(
            token.as_str(),
            &audience,
            &resource,
            super::ABILITY,
            super::binding(scope)?,
            nonce,
        ),
        &holder,
        super::super::fixtureauth::verification_limits(),
        super::super::fixtureauth::holder_limits()?,
    )
    .map_err(super::super::failure)?;
    let replay = super::super::fixtureauth::Replay(std::cell::Cell::new(false));
    let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
        verification_time: basalt::ucan::VerificationTime::from_unix_seconds(150),
        key_resolver: &keys,
        proof_store: &super::super::fixtureauth::PROOFS,
        revocations: &basalt::ucan::NoRevocations,
        caveat_policies: &super::super::fixtureauth::NoCaveats,
        replay: &replay,
        verification_limits: super::super::fixtureauth::verification_limits().with_clock_skew_tolerance_seconds(0),
        holder_limits: super::super::fixtureauth::holder_limits()?,
    };
    super::admit(scope, &super::super::authority::Input {
        policy_json: POLICY,
        credential: super::super::authority::Credential {
            token: token.as_str(),
            trusted_issuer: issuer_id.as_str(),
            expected_holder: audience.as_str(),
            nonce,
            signature: &signature,
        },
        services: &services,
        tick: 10,
        generation: 1,
    })
}
