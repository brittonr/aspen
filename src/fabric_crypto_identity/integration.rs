use molten_core::fabric_crypto_identity::CryptoAlgorithm;
use molten_core::fabric_crypto_identity::CryptoProfileClass;
use molten_core::fabric_crypto_identity::KeyPurpose;
use molten_core::fabric_crypto_identity::OpaqueKeyHandle;
use molten_core::fabric_crypto_identity::VerificationDecisionKind;

use super::CanonicalCryptoProfile;
use super::CanonicalSignatureDomain;
use super::CanonicalSignatureOutcome;
use super::CanonicalVerificationOutcome;
use crate::error::MoltenError;
use crate::error::Result;

pub const LEGACY_FIXTURE_SIGNATURE_ALGORITHM: &str = "blake3-local-fixture-v1";

pub fn sign_federation_payload(
    adapter: &super::IrohEd25519FileAdapter<'_>,
    handle: &OpaqueKeyHandle,
    domain: &CanonicalSignatureDomain,
    policy_ref: &str,
) -> Result<CanonicalSignatureOutcome> {
    require_purpose(handle.purpose, domain.domain.purpose, KeyPurpose::FederationOrigin)?;
    adapter.sign(handle, domain, policy_ref)
}

pub fn sign_evidence_payload(
    adapter: &super::IrohEd25519FileAdapter<'_>,
    handle: &OpaqueKeyHandle,
    domain: &CanonicalSignatureDomain,
    policy_ref: &str,
) -> Result<CanonicalSignatureOutcome> {
    require_purpose(handle.purpose, domain.domain.purpose, KeyPurpose::EvidenceSigning)?;
    adapter.sign(handle, domain, policy_ref)
}

pub fn admit_federation_verification(outcome: &CanonicalVerificationOutcome) -> Result<()> {
    require_accepted_verification(outcome, KeyPurpose::FederationOrigin)
}

pub fn admit_evidence_verification(outcome: &CanonicalVerificationOutcome) -> Result<()> {
    require_accepted_verification(outcome, KeyPurpose::EvidenceSigning)
}

pub fn admit_signature_algorithm(profile: &CanonicalCryptoProfile, algorithm: &str) -> Result<()> {
    if profile.profile.class == CryptoProfileClass::Production && algorithm == LEGACY_FIXTURE_SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(
            "deterministic BLAKE3 fixture signatures are denied by production cryptographic identity profiles",
        ));
    }
    let expected = match profile.profile.algorithm {
        CryptoAlgorithm::Ed25519Iroh => "ed25519-iroh-v1",
        CryptoAlgorithm::Blake3Fixture => LEGACY_FIXTURE_SIGNATURE_ALGORITHM,
    };
    if algorithm != expected {
        return Err(MoltenError::invalid_harness(format!(
            "cryptographic signature algorithm mismatch: expected {expected}, observed {algorithm}"
        )));
    }
    Ok(())
}

fn require_accepted_verification(outcome: &CanonicalVerificationOutcome, expected_purpose: KeyPurpose) -> Result<()> {
    if outcome.decision.purpose != expected_purpose {
        return Err(MoltenError::invalid_harness(format!(
            "cryptographic verification purpose mismatch: expected {}, observed {}",
            expected_purpose.as_str(),
            outcome.decision.purpose.as_str()
        )));
    }
    if outcome.decision.kind != VerificationDecisionKind::Accept {
        return Err(MoltenError::invalid_harness(format!(
            "cryptographic verification was denied: {:?}",
            outcome.decision.issues
        )));
    }
    Ok(())
}

fn require_purpose(handle_purpose: KeyPurpose, domain_purpose: KeyPurpose, expected_purpose: KeyPurpose) -> Result<()> {
    if handle_purpose != expected_purpose || domain_purpose != expected_purpose {
        return Err(MoltenError::invalid_harness(format!(
            "cryptographic purpose mismatch: expected {}, handle {}, domain {}",
            expected_purpose.as_str(),
            handle_purpose.as_str(),
            domain_purpose.as_str()
        )));
    }
    Ok(())
}
