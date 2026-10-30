use super::*;

pub struct AdmissionContext<'a> {
    pub descriptors: &'a [crate::fabric::FabricPortDescriptor],
    pub tier: &'a crate::fabric::CanonicalExtensionTierAdmission,
    pub execution_profiles: &'a [crate::system_extension::ExecutionProfile],
}

/// No fixture policy, executable identity, signature or capability is installed
/// here. The operator supplies the immutable cohort to ordinary tier admission.
// r[impl molten.collaboration_scope.versioned_extension]
pub fn admit_extension(
    input: &crate::system_extension::SystemExtensionManifestInput,
    context: AdmissionContext<'_>,
) -> crate::error::Result<crate::system_extension::AdmittedManifest> {
    if input.extension_id != EXTENSION_ID
        || input.state_schema != SCHEMA
        || input.resources.max_inflight_bytes == 0
        || input.resources.max_inflight_bytes > MAX_RECORD_BYTES
        || input.resources.max_queued_events == 0
        || input.resources.max_queued_events > MAX_OPERATIONS as u64
    {
        return Err(denied(Denial::Bound));
    }
    let admitted =
        crate::system_extension::admit_manifest(input, context.descriptors, context.tier, context.execution_profiles)?;
    for callback in [
        crate::system_extension::CallbackKind::Initialize,
        crate::system_extension::CallbackKind::Start,
        crate::system_extension::CallbackKind::Request,
        crate::system_extension::CallbackKind::Health,
        crate::system_extension::CallbackKind::Checkpoint,
        crate::system_extension::CallbackKind::Recover,
        crate::system_extension::CallbackKind::Drain,
        crate::system_extension::CallbackKind::Shutdown,
    ] {
        if !admitted.manifest().declares_callback(callback) {
            return Err(denied(Denial::Missing));
        }
    }
    for class in [
        crate::fabric::FabricPortClass::Authority,
        crate::fabric::FabricPortClass::Consistency,
        crate::fabric::FabricPortClass::DurableState,
        crate::fabric::FabricPortClass::Time,
        crate::fabric::FabricPortClass::Resources,
        crate::fabric::FabricPortClass::Evidence,
    ] {
        if !input.required_ports.iter().any(|port| port.class == class) {
            return Err(denied(Denial::Missing));
        }
    }
    Ok(admitted)
}
