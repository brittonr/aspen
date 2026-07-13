use preserves::IOValue;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

pub const FABRIC_CRYPTO_KEY_PORT_ID: &str = "molten.fabric.crypto-identity.keys";
pub const FABRIC_CRYPTO_SIGNATURE_PORT_ID: &str = "molten.fabric.crypto-identity.signatures";
pub const FABRIC_CRYPTO_PORT_VERSION: &str = "v1";

const CRYPTO_PROFILE_RECORD: &str = "fabric-crypto-adapter-profile-v1";
const CRYPTO_HANDLE_RECORD: &str = "fabric-crypto-key-handle-v1";
const SIGNATURE_DOMAIN_RECORD: &str = "fabric-crypto-signature-domain-v1";
const SIGNATURE_OUTCOME_RECORD: &str = "fabric-crypto-signature-outcome-v1";
const VERIFICATION_OUTCOME_RECORD: &str = "fabric-crypto-verification-outcome-v1";
const CRYPTO_STATUS_RECORD: &str = "fabric-crypto-status-v1";
const FABRIC_CRYPTO_PORT_COUNT: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCryptoProfile {
    pub profile: CryptoAdapterProfile,
    pub admission_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSignatureDomain {
    pub domain: SignatureDomain,
    pub domain_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalKeyHandle {
    pub handle: OpaqueKeyHandle,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSignatureOutcome {
    pub metadata: SignatureMetadata,
    pub signature: Vec<u8>,
    pub outcome_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalVerificationOutcome {
    pub decision: VerificationDecision,
    pub outcome_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoStatusReadback {
    pub status: RedactedAdapterStatus,
    pub status_ref: String,
    pub value: IOValue,
}

// r[impl molten.crypto_identity.adapter_contract]
// r[impl molten.crypto_identity.fixture_profile_boundary]
pub fn canonical_crypto_profile(profile: &CryptoAdapterProfile) -> Result<CanonicalCryptoProfile> {
    let issues = validate_crypto_profile(profile);
    if !issues.is_empty() {
        return Err(validation_error("crypto adapter profile", &issues));
    }
    let value = record(CRYPTO_PROFILE_RECORD, vec![
        string(CRYPTO_ADAPTER_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("class", string(profile.class.as_str())),
        field("algorithm", string(profile.algorithm.as_str())),
        field("backend-classes", strings_value(profile.backend_classes.iter().map(|backend| backend.as_str()))),
        field("allowed-purposes", strings_value(profile.allowed_purposes.iter().map(|purpose| purpose.as_str()))),
        field("entropy-profile-ref", optional_string(profile.entropy_profile_ref.as_deref())),
        field("domain-version", string(&profile.domain_version)),
        field("allow-key-sharing", bool_value(profile.allow_key_sharing)),
        field("sharing-policy-ref", optional_string(profile.sharing_policy_ref.as_deref())),
        field("max-signature-bytes", u64_value(profile.max_signature_bytes)),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "opaque-key-handles-only",
            "private-material-excluded",
            "purpose-and-domain-explicit",
            "fixture-production-boundary-explicit",
        ]),
    ]);
    let admission_ref = canonical_hash(&value)?;
    Ok(CanonicalCryptoProfile {
        profile: profile.clone(),
        admission_ref,
        value,
    })
}

// r[impl molten.crypto_identity.canonical_signature_binding]
// r[impl molten.crypto_identity.purpose_domain_separation]
pub fn canonical_signature_domain(
    profile: &CanonicalCryptoProfile,
    domain: &SignatureDomain,
) -> Result<CanonicalSignatureDomain> {
    let issues = validate_signature_domain(&profile.profile, domain);
    if !issues.is_empty() {
        return Err(validation_error("signature domain", &issues));
    }
    let value = record(SIGNATURE_DOMAIN_RECORD, vec![
        string(SIGNATURE_DOMAIN_SCHEMA),
        field("profile-admission-ref", string(&profile.admission_ref)),
        field("profile-ref", string(&profile.profile.profile_ref)),
        field("domain-id", string(&domain.domain_id)),
        field("domain-version", string(&domain.domain_version)),
        field("purpose", string(domain.purpose.as_str())),
        field("payload-schema", string(&domain.payload_schema)),
        field("canonical-payload-ref", string(&domain.payload_ref)),
        field("signer-public-ref", string(&domain.signer_public_ref)),
        field("verifier-context-ref", string(&domain.verifier_context_ref)),
        checks(&[
            "canonical-preserves-ref-bound",
            "rust-layout-excluded",
            "rendered-output-excluded",
            "transport-frame-excluded",
        ]),
    ]);
    let domain_ref = canonical_hash(&value)?;
    let bytes = canonical_bytes(&value)?;
    Ok(CanonicalSignatureDomain {
        domain: domain.clone(),
        domain_ref,
        value,
        bytes,
    })
}

// r[impl molten.crypto_identity.adapter_contract]
pub fn canonical_key_handle(
    profile: &CanonicalCryptoProfile,
    purpose: KeyPurpose,
    generation: u64,
    public_key_ref: &str,
    backend_class: KeyBackendClass,
    backend_ref: &str,
    currentness: KeyCurrentness,
    currentness_evidence_ref: &str,
) -> Result<CanonicalKeyHandle> {
    let value = record(CRYPTO_HANDLE_RECORD, vec![
        string(OPAQUE_KEY_HANDLE_SCHEMA),
        field("profile-ref", string(&profile.profile.profile_ref)),
        field("purpose", string(purpose.as_str())),
        field("generation", u64_value(generation)),
        field("public-key-ref", string(public_key_ref)),
        field("backend-class", string(backend_class.as_str())),
        field("backend-ref", string(backend_ref)),
        field("currentness", string(currentness.as_str())),
        field("currentness-evidence-ref", string(currentness_evidence_ref)),
        checks(&["opaque-handle", "private-material-excluded", "generation-fenced"]),
    ]);
    let handle_ref = canonical_hash(&value)?;
    let handle = OpaqueKeyHandle {
        schema: OPAQUE_KEY_HANDLE_SCHEMA.to_string(),
        handle_ref,
        profile_ref: profile.profile.profile_ref.clone(),
        purpose,
        generation,
        public_key_ref: public_key_ref.to_string(),
        backend_class,
        backend_ref: backend_ref.to_string(),
        currentness,
        currentness_evidence_ref: currentness_evidence_ref.to_string(),
        secret_material_exposed: false,
    };
    let request = SignRequest {
        operation_id: "handle-self-check".to_string(),
        profile_ref: handle.profile_ref.clone(),
        handle: handle.clone(),
        domain: SignatureDomain {
            schema: SIGNATURE_DOMAIN_SCHEMA.to_string(),
            domain_id: "handle-self-check".to_string(),
            domain_version: profile.profile.domain_version.clone(),
            purpose,
            payload_schema: "handle-self-check-v1".to_string(),
            payload_ref: currentness_evidence_ref.to_string(),
            signer_public_ref: public_key_ref.to_string(),
            verifier_context_ref: backend_ref.to_string(),
        },
        current_generation: generation,
        current_handle_ref: handle.handle_ref.clone(),
        policy_ref: currentness_evidence_ref.to_string(),
    };
    plan_sign(&profile.profile, &request).map_err(|issues| validation_error("canonical key handle", &issues))?;
    Ok(CanonicalKeyHandle { handle, value })
}

// r[impl molten.crypto_identity.canonical_signature_binding]
pub fn canonical_signature_outcome(
    profile: &CanonicalCryptoProfile,
    plan: &SignPlan,
    domain: &CanonicalSignatureDomain,
    signature: Vec<u8>,
) -> Result<CanonicalSignatureOutcome> {
    let signature_bytes = u64::try_from(signature.len())
        .map_err(|_| MoltenError::invalid_harness("signature length does not fit u64"))?;
    if signature_bytes == 0 || signature_bytes > profile.profile.max_signature_bytes {
        return Err(MoltenError::invalid_harness("signature length exceeds admitted profile"));
    }
    let signature_ref = crate::preserves_rail::content_ref_from_bytes(&signature);
    let metadata = SignatureMetadata {
        profile_ref: profile.profile.profile_ref.clone(),
        algorithm: profile.profile.algorithm,
        purpose: plan.purpose,
        generation: plan.generation,
        signer_public_ref: domain.domain.signer_public_ref.clone(),
        domain_ref: domain.domain_ref.clone(),
        payload_ref: domain.domain.payload_ref.clone(),
        verifier_context_ref: domain.domain.verifier_context_ref.clone(),
        signature_ref: signature_ref.clone(),
        signature_bytes,
    };
    let value = canonical_signature_value(&metadata, &signature);
    let outcome_ref = canonical_hash(&value)?;
    Ok(CanonicalSignatureOutcome {
        metadata,
        signature,
        outcome_ref,
        value,
    })
}

pub(crate) fn canonical_signature_value(metadata: &SignatureMetadata, signature: &[u8]) -> IOValue {
    record(SIGNATURE_OUTCOME_RECORD, vec![
        string(CRYPTO_OUTCOME_SCHEMA),
        field("profile-ref", string(&metadata.profile_ref)),
        field("algorithm", string(metadata.algorithm.as_str())),
        field("purpose", string(metadata.purpose.as_str())),
        field("generation", u64_value(metadata.generation)),
        field("signer-public-ref", string(&metadata.signer_public_ref)),
        field("domain-ref", string(&metadata.domain_ref)),
        field("payload-ref", string(&metadata.payload_ref)),
        field("verifier-context-ref", string(&metadata.verifier_context_ref)),
        field("signature-ref", string(&metadata.signature_ref)),
        field("signature-bytes", u64_value(metadata.signature_bytes)),
        field("signature", bytes_value(signature)),
        checks(&[
            "ed25519-signature-public",
            "private-material-excluded",
            "canonical-domain-signed",
        ]),
    ])
}

// r[impl molten.crypto_identity.canonical_signature_binding]
pub fn canonical_verification_outcome(decision: VerificationDecision) -> Result<CanonicalVerificationOutcome> {
    let value = record(VERIFICATION_OUTCOME_RECORD, vec![
        string(CRYPTO_OUTCOME_SCHEMA),
        field("decision", string(decision.kind.as_str())),
        field("profile-ref", string(&decision.profile_ref)),
        field("purpose", string(decision.purpose.as_str())),
        field("payload-ref", string(&decision.payload_ref)),
        field("signature-ref", string(&decision.signature_ref)),
        field("issues", strings_value(decision.issues.iter().map(|issue| issue_label(issue)))),
        checks(&[
            "cryptographic-outcome-supplied",
            "purpose-domain-and-payload-compared",
            "verification-grants-no-authority",
        ]),
    ]);
    let outcome_ref = canonical_hash(&value)?;
    Ok(CanonicalVerificationOutcome {
        decision,
        outcome_ref,
        value,
    })
}

// r[impl molten.crypto_identity.redaction]
pub fn canonical_crypto_status(input: &AdapterDiagnosticInput) -> Result<CryptoStatusReadback> {
    let status = redact_adapter_status(input).map_err(|issues| validation_error("crypto status", &issues))?;
    let value = record(CRYPTO_STATUS_RECORD, vec![
        string(CRYPTO_STATUS_SCHEMA),
        field("profile-ref", string(&status.profile_ref)),
        field("purpose", string(status.purpose.as_str())),
        field("generation", optional_u64(status.generation)),
        field("currentness", optional_string(status.currentness.map(KeyCurrentness::as_str))),
        field("permission-status", string(status.permission_status.as_str())),
        field("backend-class", string(status.backend_class.as_str())),
        field("public-key-ref", optional_string(status.public_key_ref.as_deref())),
        field("receipt-refs", strings_value(status.receipt_refs.iter().map(String::as_str))),
        field("backend-locator-redacted", bool_value(status.has_redacted_backend_locator)),
        field("raw-error-redacted", bool_value(status.has_redacted_error)),
        field("bearer-token-redacted", bool_value(status.has_redacted_bearer_token)),
        checks(&[
            "private-key-excluded",
            "backend-locator-excluded",
            "credentials-excluded",
            "public-status-bounded",
        ]),
    ]);
    let status_ref = canonical_hash(&value)?;
    Ok(CryptoStatusReadback {
        status,
        status_ref,
        value,
    })
}

pub fn fabric_crypto_port_descriptors(profile: &CanonicalCryptoProfile) -> Vec<FabricPortDescriptor> {
    let definitions = [
        (
            FABRIC_CRYPTO_KEY_PORT_ID,
            vec!["generate", "resolve", "public-key", "rotate", "revoke", "status"],
            vec![
                FabricAuthority::DurableState,
                FabricAuthority::Time,
                FabricAuthority::Policy,
            ],
        ),
        (FABRIC_CRYPTO_SIGNATURE_PORT_ID, vec!["sign", "verify"], vec![
            FabricAuthority::Evidence,
            FabricAuthority::Policy,
        ]),
    ];
    let mut descriptors = Vec::with_capacity(FABRIC_CRYPTO_PORT_COUNT);
    for (port_id, operations, authorities) in definitions {
        descriptors.push(FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_CRYPTO_PORT_VERSION.to_string(),
            class: FabricPortClass::Authority,
            operation_classes: operations.into_iter().map(str::to_string).collect(),
            input_schema_refs: vec![CRYPTO_OPERATION_SCHEMA.to_string()],
            output_schema_refs: vec![CRYPTO_OUTCOME_SCHEMA.to_string()],
            authority_requirements: authorities,
            resource_requirements: vec![FabricResource::StorageBytes, FabricResource::Memory],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.admission_ref.clone(), profile.profile.profile_ref.clone()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

fn issue_label(issue: &CryptoIdentityIssue) -> &'static str {
    match issue {
        CryptoIdentityIssue::PurposeMismatch => "purpose-mismatch",
        CryptoIdentityIssue::PayloadRefMismatch => "payload-ref-mismatch",
        CryptoIdentityIssue::SignerPublicRefMismatch => "signer-public-ref-mismatch",
        CryptoIdentityIssue::VerifierContextMismatch => "verifier-context-mismatch",
        CryptoIdentityIssue::CryptographicVerificationFailed => "cryptographic-verification-failed",
        CryptoIdentityIssue::HandleNotCurrent(_) => "handle-not-current",
        CryptoIdentityIssue::HandleGenerationStale { .. } => "handle-generation-stale",
        CryptoIdentityIssue::SignatureTooLarge { .. } => "signature-size-invalid",
        CryptoIdentityIssue::SignatureMalformed => "signature-malformed",
        _ => "crypto-admission-issue",
    }
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}

fn field(name: &str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> IOValue {
    sequence(values.into_iter().map(string).collect())
}

fn optional_string(value: Option<&str>) -> IOValue {
    value.map_or_else(|| sequence(Vec::new()), |value| sequence(vec![string(value)]))
}

fn optional_u64(value: Option<u64>) -> IOValue {
    value.map_or_else(|| sequence(Vec::new()), |value| sequence(vec![u64_value(value)]))
}

fn bytes_value(bytes: &[u8]) -> IOValue {
    sequence(bytes.iter().map(|byte| u64_value(u64::from(*byte))).collect())
}

fn checks(values: &[&str]) -> IOValue {
    strings_value(values.iter().copied())
}
