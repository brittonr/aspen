use super::*;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

// r[impl molten.crypto_identity.adapter_contract]
// r[impl molten.crypto_identity.fixture_profile_boundary]
pub fn validate_crypto_profile(profile: &CryptoAdapterProfile) -> Vec<CryptoIdentityIssue> {
    let mut issues = required_non_claim_issues(&profile.non_claims);
    if profile.schema != CRYPTO_ADAPTER_PROFILE_SCHEMA {
        issues.push(CryptoIdentityIssue::SchemaMismatch("crypto-adapter-profile"));
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    validate_ref("profile-ref", &profile.profile_ref, &mut issues);
    validate_token("domain-version", &profile.domain_version, &mut issues);
    validate_sorted_unique("backend-classes", &profile.backend_classes, &mut issues);
    validate_sorted_unique("allowed-purposes", &profile.allowed_purposes, &mut issues);
    if profile.backend_classes.is_empty() {
        issues.push(CryptoIdentityIssue::CollectionLimitExceeded("backend-classes"));
    }
    if profile.allowed_purposes.is_empty() {
        issues.push(CryptoIdentityIssue::CollectionLimitExceeded("allowed-purposes"));
    }
    if profile.max_signature_bytes == 0 || profile.max_signature_bytes > MAX_SIGNATURE_BYTES {
        issues.push(CryptoIdentityIssue::SignatureTooLarge {
            actual: profile.max_signature_bytes,
            maximum: MAX_SIGNATURE_BYTES,
        });
    }
    match profile.class {
        CryptoProfileClass::Production => validate_production_profile(profile, &mut issues),
        CryptoProfileClass::FixtureSimulation => validate_fixture_profile(profile, &mut issues),
    }
    if profile.allow_key_sharing {
        match profile.sharing_policy_ref.as_deref() {
            Some(policy_ref) => validate_ref("sharing-policy-ref", policy_ref, &mut issues),
            None => issues.push(CryptoIdentityIssue::KeySharingPolicyRequired),
        }
    } else if let Some(policy_ref) = profile.sharing_policy_ref.as_deref() {
        validate_ref("sharing-policy-ref", policy_ref, &mut issues);
    }
    issues
}

fn validate_production_profile(profile: &CryptoAdapterProfile, issues: &mut Vec<CryptoIdentityIssue>) {
    if profile.algorithm != CryptoAlgorithm::Ed25519Iroh {
        issues.push(CryptoIdentityIssue::UnsupportedProductionAlgorithm);
    }
    if profile.backend_classes.iter().any(|backend| !backend.is_production_eligible()) {
        issues.push(CryptoIdentityIssue::FixtureBackendDeniedInProduction);
    }
    match profile.entropy_profile_ref.as_deref() {
        Some(entropy_ref) => validate_ref("entropy-profile-ref", entropy_ref, issues),
        None => issues.push(CryptoIdentityIssue::MissingEntropyProfile),
    }
}

fn validate_fixture_profile(profile: &CryptoAdapterProfile, issues: &mut Vec<CryptoIdentityIssue>) {
    if let Some(entropy_ref) = profile.entropy_profile_ref.as_deref() {
        validate_ref("fixture-entropy-profile-ref", entropy_ref, issues);
    }
}

// r[impl molten.crypto_identity.adapter_contract]
pub fn admit_key_generation(
    profile: &CryptoAdapterProfile,
    request: &KeyGenerationRequest,
) -> Result<KeyGenerationPlan, Vec<CryptoIdentityIssue>> {
    let mut issues = validate_crypto_profile(profile);
    validate_token("generation-operation-id", &request.operation_id, &mut issues);
    validate_ref("generation-profile-ref", &request.profile_ref, &mut issues);
    validate_ref("generation-backend-ref", &request.backend_ref, &mut issues);
    validate_ref("generation-entropy-profile-ref", &request.entropy_profile_ref, &mut issues);
    validate_ref("generation-policy-ref", &request.policy_ref, &mut issues);
    if request.profile_ref != profile.profile_ref {
        issues.push(CryptoIdentityIssue::ProfileMismatch);
    }
    if !profile.allowed_purposes.contains(&request.purpose) {
        issues.push(CryptoIdentityIssue::UnsupportedPurpose(request.purpose));
    }
    if !profile.backend_classes.contains(&request.backend_class) {
        issues.push(CryptoIdentityIssue::UnsupportedBackend(request.backend_class));
    }
    if profile.entropy_profile_ref.as_deref() != Some(request.entropy_profile_ref.as_str()) {
        issues.push(CryptoIdentityIssue::MissingEntropyProfile);
    }
    if request.generation == 0 {
        issues.push(CryptoIdentityIssue::ZeroGeneration);
    }
    if !request.permit_first_boot_generation {
        issues.push(CryptoIdentityIssue::BackendUnavailable);
    }
    if profile.class == CryptoProfileClass::Production && !request.backend_class.is_production_eligible() {
        issues.push(CryptoIdentityIssue::FixtureBackendDeniedInProduction);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(KeyGenerationPlan {
        request: request.clone(),
        algorithm: profile.algorithm,
        persist_restricted: profile.class == CryptoProfileClass::Production,
        replace_existing: false,
    })
}

// r[impl molten.crypto_identity.purpose_domain_separation]
// r[impl molten.crypto_identity.canonical_signature_binding]
pub fn validate_signature_domain(profile: &CryptoAdapterProfile, domain: &SignatureDomain) -> Vec<CryptoIdentityIssue> {
    let mut issues = validate_crypto_profile(profile);
    if domain.schema != SIGNATURE_DOMAIN_SCHEMA {
        issues.push(CryptoIdentityIssue::SchemaMismatch("signature-domain"));
    }
    validate_token("signature-domain-id", &domain.domain_id, &mut issues);
    validate_token("signature-domain-version", &domain.domain_version, &mut issues);
    validate_token("payload-schema", &domain.payload_schema, &mut issues);
    validate_ref("payload-ref", &domain.payload_ref, &mut issues);
    validate_ref("signer-public-ref", &domain.signer_public_ref, &mut issues);
    validate_ref("verifier-context-ref", &domain.verifier_context_ref, &mut issues);
    if domain.domain_version != profile.domain_version {
        issues.push(CryptoIdentityIssue::DomainVersionMismatch);
    }
    if !profile.allowed_purposes.contains(&domain.purpose) {
        issues.push(CryptoIdentityIssue::UnsupportedPurpose(domain.purpose));
    }
    issues
}

// r[impl molten.crypto_identity.purpose_domain_separation]
// r[impl molten.crypto_identity.rotation_revocation]
pub fn plan_sign(profile: &CryptoAdapterProfile, request: &SignRequest) -> Result<SignPlan, Vec<CryptoIdentityIssue>> {
    let mut issues = validate_signature_domain(profile, &request.domain);
    validate_token("sign-operation-id", &request.operation_id, &mut issues);
    validate_ref("sign-profile-ref", &request.profile_ref, &mut issues);
    validate_ref("current-handle-ref", &request.current_handle_ref, &mut issues);
    validate_ref("sign-policy-ref", &request.policy_ref, &mut issues);
    validate_handle(profile, &request.handle, &mut issues);
    if request.profile_ref != profile.profile_ref || request.handle.profile_ref != profile.profile_ref {
        issues.push(CryptoIdentityIssue::ProfileMismatch);
    }
    if request.handle.purpose != request.domain.purpose {
        issues.push(CryptoIdentityIssue::PurposeMismatch);
    }
    if request.handle.public_key_ref != request.domain.signer_public_ref {
        issues.push(CryptoIdentityIssue::SignerPublicRefMismatch);
    }
    if request.handle.generation != request.current_generation {
        issues.push(CryptoIdentityIssue::HandleGenerationStale {
            expected: request.current_generation,
            actual: request.handle.generation,
        });
    }
    if request.handle.handle_ref != request.current_handle_ref {
        issues.push(CryptoIdentityIssue::HandleRefStale);
    }
    if !request.handle.currentness.permits_signing() {
        issues.push(CryptoIdentityIssue::HandleNotCurrent(request.handle.currentness));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(SignPlan {
        operation_id: request.operation_id.clone(),
        handle_ref: request.handle.handle_ref.clone(),
        profile_ref: profile.profile_ref.clone(),
        algorithm: profile.algorithm,
        purpose: request.handle.purpose,
        generation: request.handle.generation,
        domain: request.domain.clone(),
        policy_ref: request.policy_ref.clone(),
    })
}

fn validate_handle(profile: &CryptoAdapterProfile, handle: &OpaqueKeyHandle, issues: &mut Vec<CryptoIdentityIssue>) {
    if handle.schema != OPAQUE_KEY_HANDLE_SCHEMA {
        issues.push(CryptoIdentityIssue::SchemaMismatch("opaque-key-handle"));
    }
    for (field, value) in [
        ("handle-ref", handle.handle_ref.as_str()),
        ("handle-profile-ref", handle.profile_ref.as_str()),
        ("public-key-ref", handle.public_key_ref.as_str()),
        ("backend-ref", handle.backend_ref.as_str()),
        ("currentness-evidence-ref", handle.currentness_evidence_ref.as_str()),
    ] {
        validate_ref(field, value, issues);
    }
    if handle.generation == 0 {
        issues.push(CryptoIdentityIssue::ZeroGeneration);
    }
    if !profile.backend_classes.contains(&handle.backend_class) {
        issues.push(CryptoIdentityIssue::UnsupportedBackend(handle.backend_class));
    }
    if !profile.allowed_purposes.contains(&handle.purpose) {
        issues.push(CryptoIdentityIssue::UnsupportedPurpose(handle.purpose));
    }
    if handle.secret_material_exposed {
        issues.push(CryptoIdentityIssue::SecretMaterialExposed);
    }
}

// r[impl molten.crypto_identity.canonical_signature_binding]
pub fn evaluate_verification(profile: &CryptoAdapterProfile, request: &VerificationRequest) -> VerificationDecision {
    let mut issues = validate_signature_domain(profile, &request.expected_domain);
    validate_token("verify-operation-id", &request.operation_id, &mut issues);
    validate_ref("verify-profile-ref", &request.profile_ref, &mut issues);
    validate_ref("observed-domain-ref", &request.observed.domain_ref, &mut issues);
    validate_ref("observed-signature-ref", &request.observed.signature_ref, &mut issues);
    validate_ref("verify-policy-ref", &request.policy_ref, &mut issues);
    if request.profile_ref != profile.profile_ref || request.observed.profile_ref != profile.profile_ref {
        issues.push(CryptoIdentityIssue::ProfileMismatch);
    }
    if request.observed.algorithm != profile.algorithm {
        issues.push(CryptoIdentityIssue::SignatureMalformed);
    }
    if request.observed.purpose != request.expected_domain.purpose {
        issues.push(CryptoIdentityIssue::PurposeMismatch);
    }
    if request.observed.payload_ref != request.expected_domain.payload_ref {
        issues.push(CryptoIdentityIssue::PayloadRefMismatch);
    }
    if request.observed.signer_public_ref != request.expected_domain.signer_public_ref {
        issues.push(CryptoIdentityIssue::SignerPublicRefMismatch);
    }
    if request.observed.verifier_context_ref != request.expected_domain.verifier_context_ref {
        issues.push(CryptoIdentityIssue::VerifierContextMismatch);
    }
    if request.observed.generation != request.signer_generation {
        issues.push(CryptoIdentityIssue::HandleGenerationStale {
            expected: request.signer_generation,
            actual: request.observed.generation,
        });
    }
    if !request.signer_currentness.permits_signing() {
        issues.push(CryptoIdentityIssue::HandleNotCurrent(request.signer_currentness));
    }
    if request.observed.signature_bytes == 0 || request.observed.signature_bytes > profile.max_signature_bytes {
        issues.push(CryptoIdentityIssue::SignatureTooLarge {
            actual: request.observed.signature_bytes,
            maximum: profile.max_signature_bytes,
        });
    }
    if !request.cryptographic_verification_passed {
        issues.push(CryptoIdentityIssue::CryptographicVerificationFailed);
    }
    VerificationDecision {
        kind: if issues.is_empty() {
            VerificationDecisionKind::Accept
        } else {
            VerificationDecisionKind::Deny
        },
        issues,
        profile_ref: profile.profile_ref.clone(),
        purpose: request.expected_domain.purpose,
        payload_ref: request.expected_domain.payload_ref.clone(),
        signature_ref: request.observed.signature_ref.clone(),
    }
}

// r[impl molten.crypto_identity.fixture_profile_boundary]
pub fn admit_profile_for_production(profile: &CryptoAdapterProfile) -> Result<(), Vec<CryptoIdentityIssue>> {
    let mut issues = validate_crypto_profile(profile);
    if profile.class != CryptoProfileClass::Production {
        issues.push(CryptoIdentityIssue::FixtureProfileDeniedInProduction);
    }
    if profile.algorithm != CryptoAlgorithm::Ed25519Iroh {
        issues.push(CryptoIdentityIssue::UnsupportedProductionAlgorithm);
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

// r[impl molten.crypto_identity.redaction]
pub fn redact_adapter_status(
    input: &AdapterDiagnosticInput,
) -> Result<RedactedAdapterStatus, Vec<CryptoIdentityIssue>> {
    let mut issues = Vec::new();
    validate_ref("status-profile-ref", &input.profile_ref, &mut issues);
    if let Some(public_key_ref) = input.public_key_ref.as_deref() {
        validate_ref("status-public-key-ref", public_key_ref, &mut issues);
    }
    if input.receipt_refs.len() > MAX_CRYPTO_COLLECTION_ITEMS {
        issues.push(CryptoIdentityIssue::CollectionLimitExceeded("status-receipt-refs"));
    }
    for receipt_ref in &input.receipt_refs {
        validate_ref("status-receipt-ref", receipt_ref, &mut issues);
    }
    if input.private_material_present {
        issues.push(CryptoIdentityIssue::DiagnosticSecretLeak);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(RedactedAdapterStatus {
        profile_ref: input.profile_ref.clone(),
        purpose: input.purpose,
        generation: input.generation,
        currentness: input.currentness,
        permission_status: input.permission_status,
        backend_class: input.backend_class,
        public_key_ref: input.public_key_ref.clone(),
        receipt_refs: input.receipt_refs.clone(),
        has_redacted_backend_locator: input.backend_locator.is_some(),
        has_redacted_error: input.raw_error.is_some(),
        has_redacted_bearer_token: input.bearer_token.is_some(),
        denied_private_material: false,
    })
}

fn validate_sorted_unique<T: Ord + Copy>(field: &'static str, values: &[T], issues: &mut Vec<CryptoIdentityIssue>) {
    if values.len() > MAX_CRYPTO_COLLECTION_ITEMS {
        issues.push(CryptoIdentityIssue::CollectionLimitExceeded(field));
    }
    if values.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(CryptoIdentityIssue::DuplicateValue(field));
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<CryptoIdentityIssue>) {
    if value.is_empty() {
        issues.push(CryptoIdentityIssue::EmptyField(field));
    } else if value.len() > MAX_CRYPTO_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(CryptoIdentityIssue::MalformedToken(field));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<CryptoIdentityIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(CryptoIdentityIssue::MalformedRef(field));
    }
}
