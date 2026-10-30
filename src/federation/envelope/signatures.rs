#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRecord {
    pub purpose: molten_core::federation::RecordPurpose,
    pub generation: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub payload: preserves::IOValue,
    pub signature: crate::fabric_crypto_identity::SignatureOutcome,
}

pub struct SigningInput<'a> {
    pub profile: &'a crate::fabric_crypto_identity::Profile,
    pub handle: &'a crate::fabric_crypto_identity::OpaqueKeyHandle,
    pub context_ref: &'a str,
    pub policy_ref: &'a str,
    pub purpose: molten_core::federation::RecordPurpose,
    pub issued_at: u64,
    pub expires_at: u64,
    pub payload: &'a preserves::IOValue,
}

pub struct VerificationInput<'a> {
    pub profile: &'a crate::fabric_crypto_identity::Profile,
    pub public_key: &'a str,
    pub signer_ref: &'a str,
    pub generation: u64,
    pub currentness: crate::fabric_crypto_identity::KeyCurrentness,
    pub context_ref: &'a str,
    pub policy_ref: &'a str,
    pub purpose: molten_core::federation::RecordPurpose,
    pub now: u64,
    pub max_age: u64,
}

// r[impl molten.federated_pull_runtime.crypto]
pub fn sign_record(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    input: &SigningInput<'_>,
) -> crate::error::Result<SignedRecord> {
    production(input.profile)?;
    if input.expires_at < input.issued_at {
        return Err(crate::error::Failure::invalid_harness("federation signature freshness interval reversed"));
    }
    let value =
        signed_payload(input.purpose, input.handle.generation, input.issued_at, input.expires_at, input.payload);
    let domain = payload_domain(input.profile, input.purpose, &value, &input.handle.public_key_ref, input.context_ref)?;
    let signature =
        crate::fabric_crypto_identity::sign_federation_payload(adapter, input.handle, &domain, input.policy_ref)?;
    Ok(SignedRecord {
        purpose: input.purpose,
        generation: input.handle.generation,
        issued_at: input.issued_at,
        expires_at: input.expires_at,
        payload: input.payload.clone(),
        signature,
    })
}

// r[impl molten.federated_pull_runtime.crypto]
pub fn verify_record(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    record: &SignedRecord,
    input: &VerificationInput<'_>,
) -> crate::error::Result<crate::fabric_crypto_identity::VerificationOutcome> {
    production(input.profile)?;
    if record.purpose != input.purpose
        || record.generation != input.generation
        || record.issued_at > input.now
        || record.expires_at < input.now
        || record.expires_at < record.issued_at
        || input.now - record.issued_at > input.max_age
    {
        return Err(crate::error::Failure::invalid_harness(
            "federation record purpose, generation or freshness denied",
        ));
    }
    let value = signed_payload(record.purpose, record.generation, record.issued_at, record.expires_at, &record.payload);
    let domain = payload_domain(input.profile, record.purpose, &value, input.signer_ref, input.context_ref)?;
    let outcome = adapter.verify(input.public_key, crate::fabric_crypto_identity::VerificationInput {
        expected_domain: &domain,
        signature: &record.signature,
        signer_currentness: input.currentness,
        signer_generation: input.generation,
        policy_ref: input.policy_ref,
    })?;
    crate::fabric_crypto_identity::admit_federation_verification(&outcome)?;
    Ok(outcome)
}

fn production(profile: &crate::fabric_crypto_identity::Profile) -> crate::error::Result<()> {
    if profile.profile.class != crate::fabric_crypto_identity::CryptoProfileClass::Production
        || profile.profile.algorithm != crate::fabric_crypto_identity::CryptoAlgorithm::Ed25519Iroh
    {
        return Err(crate::error::Failure::invalid_harness("live federation requires admitted production Ed25519"));
    }
    Ok(())
}

pub(super) fn signed_payload(
    purpose: molten_core::federation::RecordPurpose,
    generation: u64,
    issued_at: u64,
    expires_at: u64,
    payload: &preserves::IOValue,
) -> preserves::IOValue {
    crate::preserves_rail::record("federation-signed-record-v1", vec![
        crate::preserves_rail::string(purpose.domain()),
        crate::preserves_rail::u64_value(generation),
        crate::preserves_rail::u64_value(issued_at),
        crate::preserves_rail::u64_value(expires_at),
        payload.clone(),
    ])
}

pub(super) fn payload_domain(
    profile: &crate::fabric_crypto_identity::Profile,
    purpose: molten_core::federation::RecordPurpose,
    value: &preserves::IOValue,
    signer: &str,
    context: &str,
) -> crate::error::Result<crate::fabric_crypto_identity::DomainEnvelope> {
    crate::fabric_crypto_identity::signature_domain(profile, &crate::fabric_crypto_identity::SignatureDomain {
        schema: crate::fabric_crypto_identity::SIGNATURE_DOMAIN_SCHEMA.to_string(),
        domain_id: purpose.domain().to_string(),
        domain_version: profile.profile.domain_version.clone(),
        purpose: crate::fabric_crypto_identity::KeyPurpose::FederationOrigin,
        payload_schema: purpose.domain().to_string(),
        payload_ref: crate::preserves_rail::canonical_hash(value)?,
        signer_public_ref: signer.to_string(),
        verifier_context_ref: context.to_string(),
    })
}
