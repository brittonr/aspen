pub const LEGACY_FIXTURE_SIGNATURE_ALGORITHM: &str = "blake3-local-fixture-v1";

pub fn sign_federation_payload(
    adapter: &super::IrohEd25519FileAdapter<'_>,
    handle: &molten_core::fabric_crypto_identity::OpaqueKeyHandle,
    domain: &super::CanonicalSignatureDomain,
    policy_ref: &str,
) -> crate::error::Result<super::CanonicalSignatureOutcome> {
    require_purpose(
        handle.purpose,
        domain.domain.purpose,
        molten_core::fabric_crypto_identity::KeyPurpose::FederationOrigin,
    )?;
    adapter.sign(handle, domain, policy_ref)
}

pub fn sign_evidence_payload(
    adapter: &super::IrohEd25519FileAdapter<'_>,
    handle: &molten_core::fabric_crypto_identity::OpaqueKeyHandle,
    domain: &super::CanonicalSignatureDomain,
    policy_ref: &str,
) -> crate::error::Result<super::CanonicalSignatureOutcome> {
    require_purpose(
        handle.purpose,
        domain.domain.purpose,
        molten_core::fabric_crypto_identity::KeyPurpose::EvidenceSigning,
    )?;
    adapter.sign(handle, domain, policy_ref)
}

pub fn admit_federation_verification(outcome: &super::CanonicalVerificationOutcome) -> crate::error::Result<()> {
    require_accepted_verification(outcome, molten_core::fabric_crypto_identity::KeyPurpose::FederationOrigin)
}

pub fn admit_evidence_verification(outcome: &super::CanonicalVerificationOutcome) -> crate::error::Result<()> {
    require_accepted_verification(outcome, molten_core::fabric_crypto_identity::KeyPurpose::EvidenceSigning)
}

pub fn admit_signature_algorithm(profile: &super::CanonicalCryptoProfile, algorithm: &str) -> crate::error::Result<()> {
    if profile.profile.class == molten_core::fabric_crypto_identity::CryptoProfileClass::Production
        && algorithm == LEGACY_FIXTURE_SIGNATURE_ALGORITHM
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "deterministic BLAKE3 fixture signatures are denied by production cryptographic identity profiles",
        ));
    }
    let expected = match profile.profile.algorithm {
        molten_core::fabric_crypto_identity::CryptoAlgorithm::Ed25519Iroh => "ed25519-iroh-v1",
        molten_core::fabric_crypto_identity::CryptoAlgorithm::Blake3Fixture => LEGACY_FIXTURE_SIGNATURE_ALGORITHM,
    };
    if algorithm != expected {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cryptographic signature algorithm mismatch: expected {expected}, observed {algorithm}"
        )));
    }
    Ok(())
}

fn require_accepted_verification(
    outcome: &super::CanonicalVerificationOutcome,
    expected_purpose: molten_core::fabric_crypto_identity::KeyPurpose,
) -> crate::error::Result<()> {
    if outcome.decision.purpose != expected_purpose {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cryptographic verification purpose mismatch: expected {}, observed {}",
            expected_purpose.as_str(),
            outcome.decision.purpose.as_str()
        )));
    }
    if outcome.decision.kind != molten_core::fabric_crypto_identity::VerificationDecisionKind::Accept {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cryptographic verification was denied: {:?}",
            outcome.decision.issues
        )));
    }
    Ok(())
}

fn require_purpose(
    handle_purpose: molten_core::fabric_crypto_identity::KeyPurpose,
    domain_purpose: molten_core::fabric_crypto_identity::KeyPurpose,
    expected_purpose: molten_core::fabric_crypto_identity::KeyPurpose,
) -> crate::error::Result<()> {
    if handle_purpose != expected_purpose || domain_purpose != expected_purpose {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "cryptographic purpose mismatch: expected {}, handle {}, domain {}",
            expected_purpose.as_str(),
            handle_purpose.as_str(),
            domain_purpose.as_str()
        )));
    }
    Ok(())
}
