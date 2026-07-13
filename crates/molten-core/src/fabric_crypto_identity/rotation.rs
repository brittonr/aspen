use super::*;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationOverlapPolicy {
    None,
    VerifyOnly,
    SignAndVerify,
}

impl RotationOverlapPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::VerifyOnly => "verify-only",
            Self::SignAndVerify => "sign-and-verify",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRotationRequest {
    pub operation_id: String,
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub backend_class: KeyBackendClass,
    pub backend_ref: String,
    pub old_handle_ref: String,
    pub old_public_key_ref: String,
    pub old_generation: u64,
    pub new_generation: u64,
    pub policy_ref: String,
    pub activation_boundary_ref: String,
    pub overlap: RotationOverlapPolicy,
    pub revocation_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRotationPlan {
    pub request: KeyRotationRequest,
    pub generate_new_key: bool,
    pub activate_new_generation: bool,
    pub old_key_next_currentness: KeyCurrentness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRotationOutcome {
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub old_handle_ref: String,
    pub old_public_key_ref: String,
    pub new_handle_ref: String,
    pub new_public_key_ref: String,
    pub old_generation: u64,
    pub new_generation: u64,
    pub old_key_currentness: KeyCurrentness,
    pub new_key_currentness: KeyCurrentness,
    pub activation_boundary_ref: String,
    pub policy_ref: String,
    pub revocation_evidence_ref: Option<String>,
}

// r[impl molten.crypto_identity.rotation_revocation]
pub fn plan_key_rotation(
    profile: &CryptoAdapterProfile,
    current: &OpaqueKeyHandle,
    request: &KeyRotationRequest,
) -> Result<KeyRotationPlan, Vec<CryptoIdentityIssue>> {
    let mut issues = validate_crypto_profile(profile);
    validate_token("rotation-operation-id", &request.operation_id, &mut issues);
    for (field, value) in [
        ("rotation-profile-ref", request.profile_ref.as_str()),
        ("rotation-backend-ref", request.backend_ref.as_str()),
        ("old-handle-ref", request.old_handle_ref.as_str()),
        ("old-public-key-ref", request.old_public_key_ref.as_str()),
        ("rotation-policy-ref", request.policy_ref.as_str()),
        ("activation-boundary-ref", request.activation_boundary_ref.as_str()),
    ] {
        validate_ref(field, value, &mut issues);
    }
    if request.profile_ref != profile.profile_ref || current.profile_ref != profile.profile_ref {
        issues.push(CryptoIdentityIssue::ProfileMismatch);
    }
    if request.purpose != current.purpose {
        issues.push(CryptoIdentityIssue::PurposeMismatch);
    }
    if request.backend_class != current.backend_class || request.backend_ref != current.backend_ref {
        issues.push(CryptoIdentityIssue::RotationPolicyMismatch);
    }
    if request.old_handle_ref != current.handle_ref || request.old_public_key_ref != current.public_key_ref {
        issues.push(CryptoIdentityIssue::HandleRefStale);
    }
    if request.old_generation != current.generation {
        issues.push(CryptoIdentityIssue::HandleGenerationStale {
            expected: current.generation,
            actual: request.old_generation,
        });
    }
    if request.new_generation <= request.old_generation {
        issues.push(CryptoIdentityIssue::GenerationNotAdvanced);
    }
    if !current.currentness.permits_signing() {
        issues.push(CryptoIdentityIssue::HandleNotCurrent(current.currentness));
    }
    if matches!(request.overlap, RotationOverlapPolicy::None) && request.revocation_evidence_ref.is_none() {
        issues.push(CryptoIdentityIssue::RevocationEvidenceRequired);
    }
    if let Some(revocation_ref) = request.revocation_evidence_ref.as_deref() {
        validate_ref("revocation-evidence-ref", revocation_ref, &mut issues);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let old_key_next_currentness = match request.overlap {
        RotationOverlapPolicy::None => KeyCurrentness::Revoked,
        RotationOverlapPolicy::VerifyOnly | RotationOverlapPolicy::SignAndVerify => KeyCurrentness::Overlap,
    };
    Ok(KeyRotationPlan {
        request: request.clone(),
        generate_new_key: true,
        activate_new_generation: true,
        old_key_next_currentness,
    })
}

// r[impl molten.crypto_identity.rotation_revocation]
pub fn complete_key_rotation(
    plan: &KeyRotationPlan,
    new_handle: &OpaqueKeyHandle,
) -> Result<KeyRotationOutcome, Vec<CryptoIdentityIssue>> {
    let mut issues = Vec::new();
    if new_handle.profile_ref != plan.request.profile_ref {
        issues.push(CryptoIdentityIssue::ProfileMismatch);
    }
    if new_handle.purpose != plan.request.purpose {
        issues.push(CryptoIdentityIssue::PurposeMismatch);
    }
    if new_handle.backend_class != plan.request.backend_class || new_handle.backend_ref != plan.request.backend_ref {
        issues.push(CryptoIdentityIssue::RotationPolicyMismatch);
    }
    if new_handle.generation != plan.request.new_generation {
        issues.push(CryptoIdentityIssue::HandleGenerationStale {
            expected: plan.request.new_generation,
            actual: new_handle.generation,
        });
    }
    if new_handle.currentness != KeyCurrentness::Current {
        issues.push(CryptoIdentityIssue::HandleNotCurrent(new_handle.currentness));
    }
    if new_handle.secret_material_exposed {
        issues.push(CryptoIdentityIssue::SecretMaterialExposed);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(KeyRotationOutcome {
        profile_ref: plan.request.profile_ref.clone(),
        purpose: plan.request.purpose,
        old_handle_ref: plan.request.old_handle_ref.clone(),
        old_public_key_ref: plan.request.old_public_key_ref.clone(),
        new_handle_ref: new_handle.handle_ref.clone(),
        new_public_key_ref: new_handle.public_key_ref.clone(),
        old_generation: plan.request.old_generation,
        new_generation: plan.request.new_generation,
        old_key_currentness: plan.old_key_next_currentness,
        new_key_currentness: KeyCurrentness::Current,
        activation_boundary_ref: plan.request.activation_boundary_ref.clone(),
        policy_ref: plan.request.policy_ref.clone(),
        revocation_evidence_ref: plan.request.revocation_evidence_ref.clone(),
    })
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
