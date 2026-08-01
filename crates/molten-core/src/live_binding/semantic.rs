use std::fmt::Write as _;

use kamacite_core::CompatibilityContext;
use kamacite_core::Identity;
use kamacite_core::IdentityDomain;
use kamacite_core::admit_semantic_operation_compatibility;
use kamacite_core::compute_identity;
use kamacite_core::require_exact_semantic_operation;

use super::DeployDiagnostic;
use super::LiveBindingError;
use super::SEMANTIC_NON_CLAIMS;
use super::SemanticCompatibilityAdmission;
use super::SemanticCompatibilityAdmissionInput;
use super::SemanticDerivedIdentityInput;
use super::SemanticSurfaceBindings;

fn semantic_error(error: impl std::fmt::Display) -> LiveBindingError {
    LiveBindingError::SemanticOperation(error.to_string())
}

// r[impl molten.effects.semantic_operation_identity]
// r[impl molten.effects.semantic_handler_matching]
pub fn validate_semantic_surfaces(
    declared_operation: &Identity,
    surfaces: &SemanticSurfaceBindings,
) -> Result<(), LiveBindingError> {
    declared_operation.validate_domain(IdentityDomain::EffectOperation).map_err(semantic_error)?;
    for identity in surfaces.identities() {
        require_exact_semantic_operation(declared_operation, identity).map_err(semantic_error)?;
    }
    Ok(())
}

pub fn validate_exact_handler(
    requested_operation: &Identity,
    handled_operation: &Identity,
    surfaces: &SemanticSurfaceBindings,
) -> Result<(), LiveBindingError> {
    require_exact_semantic_operation(requested_operation, handled_operation).map_err(semantic_error)?;
    validate_semantic_surfaces(requested_operation, surfaces)
}

fn validate_molten_compatibility_context(input: &SemanticCompatibilityAdmissionInput) -> Result<(), LiveBindingError> {
    let context_is_live =
        matches!(input.context, CompatibilityContext::LiveHostExecution | CompatibilityContext::RemoteRequest);
    if input.live_execution != context_is_live {
        return Err(LiveBindingError::SemanticContextDenied);
    }
    let required = [
        input.molten_policy_admitted,
        input.capability_admitted,
        input.provenance_admitted,
    ];
    if required.iter().any(|admitted| !admitted) {
        return Err(LiveBindingError::SemanticContextDenied);
    }
    Ok(())
}

// r[impl molten.effects.semantic_compatibility]
// r[impl molten.effects.semantic_identity_non_authority]
pub fn admit_directional_compatibility(
    input: &SemanticCompatibilityAdmissionInput,
) -> Result<SemanticCompatibilityAdmission, LiveBindingError> {
    validate_molten_compatibility_context(input)?;
    admit_semantic_operation_compatibility(
        &input.compatibility,
        &input.source_operation,
        &input.target_operation,
        input.context.clone(),
    )
    .map_err(semantic_error)?;
    Ok(SemanticCompatibilityAdmission {
        source_operation: input.source_operation.clone(),
        target_operation: input.target_operation.clone(),
        context: input.context.clone(),
        compatibility_admitted: true,
        runtime_authorized_by_identity: false,
        non_claims: SEMANTIC_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    })
}

fn append_field(material: &mut String, label: &str, value: &str) {
    let _ = write!(material, "{}:{}:{};", label.len(), label, value.len());
    material.push_str(value);
    material.push(';');
}

// r[impl molten.effects.semantic_replay_cache_binding]
pub fn derive_semantic_subject_identity(input: &SemanticDerivedIdentityInput) -> Result<Identity, LiveBindingError> {
    input.operation.validate_domain(IdentityDomain::EffectOperation).map_err(semantic_error)?;
    for (field, value) in [
        ("subject-ref", input.subject_ref.as_str()),
        ("handler-profile-ref", input.handler_profile_ref.as_str()),
        ("dependency-closure-ref", input.dependency_closure_ref.as_str()),
    ] {
        if value.is_empty() {
            return Err(LiveBindingError::SemanticOperation(format!("{field} must not be empty")));
        }
    }
    let mut material = String::new();
    append_field(&mut material, "kind", input.kind.label());
    append_field(&mut material, "operation-domain", &input.operation.domain.to_string());
    append_field(&mut material, "operation-algorithm", &input.operation.algorithm);
    append_field(&mut material, "operation-hex", input.operation.hex.as_str());
    append_field(&mut material, "subject-ref", &input.subject_ref);
    append_field(&mut material, "handler-profile-ref", &input.handler_profile_ref);
    append_field(&mut material, "dependency-closure-ref", &input.dependency_closure_ref);
    Ok(compute_identity(IdentityDomain::CanonicalValue, material.as_bytes()))
}

pub fn diagnose_semantic_mismatch(
    requested_operation: &Identity,
    handled_operation: &Identity,
) -> Vec<DeployDiagnostic> {
    if requested_operation == handled_operation {
        Vec::new()
    } else {
        vec![DeployDiagnostic::SemanticHandlerMismatch]
    }
}
