pub(super) fn authenticate(
    claim: &crate::world_head::CanonicalWorldHeadClaim,
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
) -> crate::error::Result<crate::world_head::WorldHeadAuthenticationResult> {
    let role = match trust.role.as_str() {
        "maintainer" => molten_core::world_head::WorldHeadSignerRole::Maintainer,
        "recovery" => molten_core::world_head::WorldHeadSignerRole::Recovery,
        _ => return Err(super::denied()),
    };
    let currentness_ref = artifact_auth_core::ArtifactRef {
        profile: "molten-world-head-operator-trust-v1".into(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.into(),
        digest_hex: crate::preserves_rail::content_ref_hex(&trust.identity()?)?.to_owned(),
    };
    let policy = artifact_auth_core::AuthenticationPolicy {
        schema: artifact_auth_core::POLICY_SCHEMA_V1.into(),
        profile_id: molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PROFILE.into(),
        threshold: 1,
        trusted_keys: vec![artifact_auth_core::TrustedKeyObservation {
            producer_id: trust.producer_id.clone(),
            key_id: trust.key_id.clone(),
            key_identity: artifact_auth_ed25519::public_key_identity(&trust.artifact_public_key),
            allowed_purposes: vec![molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE.into()],
            generation: trust.key_generation,
            currentness: artifact_auth_core::KeyCurrentness::Current,
            currentness_ref,
        }],
    };
    let carrier = crate::world_head::WorldHeadSignatureCarrier {
        producer_id: trust.producer_id.clone(),
        key_id: trust.key_id.clone(),
        public_key_bytes: trust.artifact_public_key.to_vec(),
        signature_bytes: credential.artifact_signature.clone(),
        key_generation: trust.key_generation,
        role,
        authority_admitted: true,
    };
    let result = crate::world_head::evaluate_world_head_authentication(claim, &policy, &[carrier])?;
    if !result.observation.passed || !result.observation.purpose_matches || !result.observation.policy_matches {
        return Err(super::denied());
    }
    Ok(result)
}
