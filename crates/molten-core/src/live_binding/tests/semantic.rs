use kamacite_core::CompatibilityContext;
use kamacite_core::Identity;
use kamacite_core::IdentityDomain;
use kamacite_core::SEMANTIC_OPERATION_COMPATIBILITY_SCHEMA_VERSION;
use kamacite_core::SemanticOperationCompatibility;
use kamacite_core::compute_identity;

use super::super::*;

fn operation(value: &[u8]) -> Identity {
    compute_identity(IdentityDomain::EffectOperation, value)
}

fn surfaces(identity: &Identity) -> SemanticSurfaceBindings {
    SemanticSurfaceBindings {
        manifest: identity.clone(),
        handler_binding: identity.clone(),
        handle: identity.clone(),
        request: identity.clone(),
        response: identity.clone(),
        effect_log: identity.clone(),
        adapter_import: identity.clone(),
        remote_execution: identity.clone(),
        runtime_receipt: identity.clone(),
        replay_identity: identity.clone(),
        evaluation_cache_key: identity.clone(),
        job: identity.clone(),
        upgrade_check: identity.clone(),
    }
}

fn compatibility(
    source: &Identity,
    target: &Identity,
    contexts: Vec<CompatibilityContext>,
) -> SemanticOperationCompatibility {
    SemanticOperationCompatibility {
        schema_version: SEMANTIC_OPERATION_COMPATIBILITY_SCHEMA_VERSION,
        source_operation_identity: source.clone(),
        target_operation_identity: target.clone(),
        allowed_contexts: contexts,
        supporting_refs: vec![compute_identity(IdentityDomain::Source, b"reviewed-source")],
        diagnostics: Vec::new(),
        non_claims: vec!["directional compatibility is not semantic equivalence".to_string()],
    }
}

#[test]
fn exact_semantic_identity_crosses_every_runtime_surface() {
    let declared = operation(b"declared-operation");
    let all_surfaces = surfaces(&declared);
    assert!(validate_semantic_surfaces(&declared, &all_surfaces).is_ok());
    assert!(validate_exact_handler(&declared, &declared, &all_surfaces).is_ok());
    assert!(diagnose_semantic_mismatch(&declared, &declared).is_empty());
}

#[test]
fn behavior_drift_name_fallback_and_wrong_domain_fail_closed() {
    let declared = operation(b"declared-operation");
    let changed_behavior = operation(b"changed-default-behavior");
    let mut drifted = surfaces(&declared);
    drifted.handler_binding = changed_behavior.clone();
    assert_eq!(
        validate_semantic_surfaces(&declared, &drifted),
        Err(LiveBindingError::SemanticOperation(format!(
            "semantic operation mismatch: requested {} but handler provides {}",
            declared.hex.as_str(),
            changed_behavior.hex.as_str()
        )))
    );
    assert_eq!(diagnose_semantic_mismatch(&declared, &changed_behavior), vec![
        DeployDiagnostic::SemanticHandlerMismatch
    ]);

    let name_only = compute_identity(IdentityDomain::CanonicalValue, b"display-name");
    assert!(validate_exact_handler(&declared, &name_only, &surfaces(&declared)).is_err());
}

#[test]
fn replay_compatibility_is_directional_and_never_authorizes_live_use() {
    let source = operation(b"source-operation");
    let target = operation(b"target-operation");
    let artifact = compatibility(&source, &target, vec![CompatibilityContext::Replay]);
    let admitted = admit_directional_compatibility(&SemanticCompatibilityAdmissionInput {
        compatibility: artifact.clone(),
        source_operation: source.clone(),
        target_operation: target.clone(),
        context: CompatibilityContext::Replay,
        molten_policy_admitted: true,
        capability_admitted: true,
        provenance_admitted: true,
        live_execution: false,
    })
    .expect("replay compatibility admitted");
    assert!(admitted.compatibility_admitted);
    assert!(!admitted.runtime_authorized_by_identity);

    let reverse = admit_directional_compatibility(&SemanticCompatibilityAdmissionInput {
        compatibility: artifact.clone(),
        source_operation: target.clone(),
        target_operation: source.clone(),
        context: CompatibilityContext::Replay,
        molten_policy_admitted: true,
        capability_admitted: true,
        provenance_admitted: true,
        live_execution: false,
    });
    assert!(reverse.is_err());

    let live = admit_directional_compatibility(&SemanticCompatibilityAdmissionInput {
        compatibility: artifact,
        source_operation: source,
        target_operation: target,
        context: CompatibilityContext::LiveHostExecution,
        molten_policy_admitted: true,
        capability_admitted: true,
        provenance_admitted: true,
        live_execution: true,
    });
    assert!(live.is_err());
}

#[test]
fn compatibility_requires_molten_policy_capability_and_provenance() {
    let source = operation(b"source-operation");
    let target = operation(b"target-operation");
    let artifact = compatibility(&source, &target, vec![CompatibilityContext::LiveHostExecution]);
    for denied_gate in ["policy", "capability", "provenance"] {
        let mut input = SemanticCompatibilityAdmissionInput {
            compatibility: artifact.clone(),
            source_operation: source.clone(),
            target_operation: target.clone(),
            context: CompatibilityContext::LiveHostExecution,
            molten_policy_admitted: true,
            capability_admitted: true,
            provenance_admitted: true,
            live_execution: true,
        };
        match denied_gate {
            "policy" => input.molten_policy_admitted = false,
            "capability" => input.capability_admitted = false,
            "provenance" => input.provenance_admitted = false,
            _ => unreachable!("closed fixture gate set"),
        }
        assert_eq!(admit_directional_compatibility(&input), Err(LiveBindingError::SemanticContextDenied));
    }
}

#[test]
fn replay_cache_job_remote_and_upgrade_identities_rekey_on_operation_drift() {
    let first = operation(b"operation-v1");
    let second = operation(b"operation-v2");
    for kind in [
        SemanticDerivedKind::Replay,
        SemanticDerivedKind::Transcript,
        SemanticDerivedKind::EvaluationCache,
        SemanticDerivedKind::Job,
        SemanticDerivedKind::RemoteExecution,
        SemanticDerivedKind::UpgradeCheck,
    ] {
        let base = SemanticDerivedIdentityInput {
            kind,
            operation: first.clone(),
            subject_ref: "blake3:subject".to_string(),
            handler_profile_ref: "blake3:handler-profile".to_string(),
            dependency_closure_ref: "blake3:dependency-closure".to_string(),
        };
        let first_identity = derive_semantic_subject_identity(&base).expect("first semantic derived identity");
        let mut changed = base;
        changed.operation = second.clone();
        let second_identity = derive_semantic_subject_identity(&changed).expect("second semantic derived identity");
        assert_ne!(first_identity, second_identity);
    }
}

#[test]
fn derived_identity_rejects_empty_subject_context() {
    let input = SemanticDerivedIdentityInput {
        kind: SemanticDerivedKind::EvaluationCache,
        operation: operation(b"operation"),
        subject_ref: String::new(),
        handler_profile_ref: "blake3:handler-profile".to_string(),
        dependency_closure_ref: "blake3:dependency-closure".to_string(),
    };
    assert!(derive_semantic_subject_identity(&input).is_err());
}
