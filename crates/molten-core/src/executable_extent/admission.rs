//! Deterministic executable-extent consumer admission.

// r[impl molten.world_extents.admission]
// r[impl molten.world_extents.wx]

const SUPPORTED_FORMAT: &str = "mantle-flat-page-v1";

/// Admits an ordinary or executable-extent code profile.
///
/// # Errors
///
/// Returns the earliest deterministic profile, remeasurement, layout,
/// compatibility, or W^X transition denial.
pub fn admit_code_profile(
    profile: &crate::executable_extent::CodeProfile,
    extent_profile_required: bool,
    remeasured: &[crate::executable_extent::RemeasuredExtent],
    consumer: &crate::executable_extent::ConsumerProfile,
    activation: crate::executable_extent::ActivationFacts,
) -> Result<crate::executable_extent::AdmissionDecision, crate::executable_extent::AdmissionError> {
    match profile {
        crate::executable_extent::CodeProfile::OrdinaryArtifact(identity) => {
            if extent_profile_required {
                return Err(crate::executable_extent::AdmissionError::ExtentProfileRequired);
            }
            Ok(crate::executable_extent::AdmissionDecision::OrdinaryArtifact(*identity))
        }
        crate::executable_extent::CodeProfile::ExecutableExtent(bundle) => {
            admit_extents(bundle, remeasured, consumer, activation)
                .map(|plan| crate::executable_extent::AdmissionDecision::ExecutableExtents(Box::new(plan)))
        }
    }
}

fn admit_extents(
    bundle: &crate::executable_extent::ProducerBundleFacts,
    remeasured: &[crate::executable_extent::RemeasuredExtent],
    consumer: &crate::executable_extent::ConsumerProfile,
    activation: crate::executable_extent::ActivationFacts,
) -> Result<crate::executable_extent::ExtentPlan, crate::executable_extent::AdmissionError> {
    validate_profile(bundle, consumer)?;
    validate_remeasurements(&bundle.extents, remeasured)?;
    let shared_extents = bundle
        .extents
        .iter()
        .map(|extent| executable_extent_core::Extent {
            offset_bytes: extent.virtual_offset_bytes,
            length_bytes: extent.length_bytes,
            content_digest: executable_extent_core::ContentDigest::from_bytes(*extent.identity.as_bytes()),
            permission: extent.permission,
        })
        .collect::<Vec<_>>();
    let layout = executable_extent_core::Layout {
        schema_version: executable_extent_core::LAYOUT_SCHEMA_VERSION_V1,
        page_profile: executable_extent_core::PageProfile {
            page_size_bytes: bundle.page_size_bytes,
            maximum_virtual_bytes: bundle.maximum_virtual_bytes,
        },
        target: executable_extent_core::TargetProfile {
            architecture: &bundle.architecture,
            abi: &bundle.abi,
            endianness: bundle.endianness,
        },
        extents: &shared_extents,
    };
    let layout_identity =
        executable_extent_core::identify_layout(&layout).map_err(crate::executable_extent::AdmissionError::Layout)?;
    if layout_identity.as_bytes() != &bundle.layout_identity {
        return Err(crate::executable_extent::AdmissionError::LayoutIdentityMismatch);
    }
    executable_extent_core::validate_compatibility(&layout, &executable_extent_core::CompatibilityProfile {
        page_size_bytes: consumer.page_size_bytes,
        target: executable_extent_core::TargetProfile {
            architecture: &consumer.architecture,
            abi: &consumer.abi,
            endianness: consumer.endianness,
        },
    })
    .map_err(crate::executable_extent::AdmissionError::Compatibility)?;
    let mappings = bundle
        .extents
        .iter()
        .map(mapping_intent)
        .collect::<Result<Vec<_>, crate::executable_extent::AdmissionError>>()?;
    assert_eq!(mappings.len(), bundle.extents.len());
    assert!(!mappings.is_empty());
    Ok(crate::executable_extent::ExtentPlan {
        code_root: bundle.code_root.clone(),
        layout_identity: *layout_identity.as_bytes(),
        mappings,
        activation: activation_decision(activation),
    })
}

fn validate_profile(
    bundle: &crate::executable_extent::ProducerBundleFacts,
    consumer: &crate::executable_extent::ConsumerProfile,
) -> Result<(), crate::executable_extent::AdmissionError> {
    if bundle.format != SUPPORTED_FORMAT {
        return Err(crate::executable_extent::AdmissionError::UnsupportedFormat);
    }
    if !bundle.closure_complete {
        return Err(crate::executable_extent::AdmissionError::IncompleteClosure);
    }
    if bundle.extents.is_empty() {
        return Err(crate::executable_extent::AdmissionError::EmptyExtents);
    }
    if bundle.code_root.runtime_cohort != consumer.runtime_cohort {
        return Err(crate::executable_extent::AdmissionError::RuntimeCohortMismatch);
    }
    if bundle.code_root.policy != consumer.policy {
        return Err(crate::executable_extent::AdmissionError::PolicyIdentityMismatch);
    }
    assert!(!bundle.format.is_empty());
    assert!(!bundle.extents.is_empty());
    Ok(())
}

fn validate_remeasurements(
    extents: &[crate::executable_extent::ExtentDescriptor],
    remeasured: &[crate::executable_extent::RemeasuredExtent],
) -> Result<(), crate::executable_extent::AdmissionError> {
    if extents.is_empty() || remeasured.is_empty() {
        return Err(crate::executable_extent::AdmissionError::EmptyExtents);
    }
    if extents.len() != remeasured.len() {
        return Err(crate::executable_extent::AdmissionError::RemeasurementShapeMismatch);
    }
    for (descriptor, observation) in extents.iter().zip(remeasured) {
        if descriptor.ordinal != observation.ordinal {
            return Err(crate::executable_extent::AdmissionError::ExtentOrdinalMismatch);
        }
        if descriptor.length_bytes != observation.length_bytes {
            return Err(crate::executable_extent::AdmissionError::ExtentLengthMismatch);
        }
        if descriptor.identity != observation.identity {
            return Err(crate::executable_extent::AdmissionError::ExtentIdentityMismatch);
        }
    }
    assert_eq!(extents.len(), remeasured.len());
    assert!(!extents.is_empty());
    Ok(())
}

fn mapping_intent(
    extent: &crate::executable_extent::ExtentDescriptor,
) -> Result<crate::executable_extent::ExtentMappingIntent, crate::executable_extent::AdmissionError> {
    let map_read_only = executable_extent_core::plan_transition(
        executable_extent_core::PermissionState::SealedReadOnly,
        executable_extent_core::PermissionState::MappedReadOnly,
    )
    .map_err(crate::executable_extent::AdmissionError::Transition)?;
    let protect_executable = if extent.permission == executable_extent_core::ExtentPermission::ExecutableReadOnly {
        Some(
            executable_extent_core::plan_transition(
                executable_extent_core::PermissionState::MappedReadOnly,
                executable_extent_core::PermissionState::ExecutableReadOnly,
            )
            .map_err(crate::executable_extent::AdmissionError::Transition)?,
        )
    } else {
        None
    };
    let final_state = if protect_executable.is_some() {
        executable_extent_core::PermissionState::ExecutableReadOnly
    } else {
        executable_extent_core::PermissionState::MappedReadOnly
    };
    let unmap = executable_extent_core::plan_transition(final_state, executable_extent_core::PermissionState::Unmapped)
        .map_err(crate::executable_extent::AdmissionError::Transition)?;
    assert_eq!(map_read_only.effect, executable_extent_core::EffectIntent::MapReadOnly);
    assert_eq!(unmap.effect, executable_extent_core::EffectIntent::Unmap);
    Ok(crate::executable_extent::ExtentMappingIntent {
        ordinal: extent.ordinal,
        map_read_only: crate::executable_extent::MappingTransition { plan: map_read_only },
        protect_executable: protect_executable.map(|plan| crate::executable_extent::MappingTransition { plan }),
        unmap: crate::executable_extent::MappingTransition { plan: unmap },
    })
}

const fn activation_decision(
    facts: crate::executable_extent::ActivationFacts,
) -> crate::executable_extent::ActivationDecision {
    if !facts.artifact_current {
        return crate::executable_extent::ActivationDecision::Deny(
            crate::executable_extent::ActivationDenial::ArtifactNotCurrent,
        );
    }
    if !facts.runtime_current {
        return crate::executable_extent::ActivationDecision::Deny(
            crate::executable_extent::ActivationDenial::RuntimeNotCurrent,
        );
    }
    if !facts.resources_available {
        return crate::executable_extent::ActivationDecision::Deny(
            crate::executable_extent::ActivationDenial::ResourcesUnavailable,
        );
    }
    if !facts.policy_current {
        return crate::executable_extent::ActivationDecision::Deny(
            crate::executable_extent::ActivationDenial::PolicyNotCurrent,
        );
    }
    if !facts.execution_authorized {
        return crate::executable_extent::ActivationDecision::Deny(
            crate::executable_extent::ActivationDenial::ExecutionUnauthorized,
        );
    }
    crate::executable_extent::ActivationDecision::Admit
}
