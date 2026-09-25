
/// The admitted fixture manifest, its upgraded successor, and the rollback manifest, in that order.
fn fixture_manifests(
    profile: super::ExecutionProfile,
) -> crate::error::Result<[super::CanonicalAdmittedSystemExtensionManifest; 3]> {
    let tier = crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: vec![
            crate::fabric::FabricAuthority::Transport,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Supervision,
            crate::fabric::FabricAuthority::Evidence,
        ],
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })?;
    let descriptors = [port_descriptor()];
    let admitted =
        super::canonical_admit_system_extension_manifest(&manifest_input(profile), &descriptors, &tier, &[profile])?;
    let mut upgrade_input = manifest_input(profile);
    upgrade_input.implementation_ref = HASH_B.to_string();
    upgrade_input.state_schema = UPGRADED_STATE_SCHEMA.to_string();
    upgrade_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
    let upgrade_manifest =
        super::canonical_admit_system_extension_manifest(&upgrade_input, &descriptors, &tier, &[profile])?;
    let mut rollback_input = manifest_input(profile);
    rollback_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
    let rollback_manifest =
        super::canonical_admit_system_extension_manifest(&rollback_input, &descriptors, &tier, &[profile])?;
    Ok([admitted, upgrade_manifest, rollback_manifest])
}

/// A retryable request fails the host into the failed phase, and a restart recovers it to running.
fn fail_and_recover(
    host: &mut super::SystemExtensionHost<EchoExecutor>,
) -> crate::error::Result<super::CanonicalOperatorStatus> {
    match host.dispatch_request(HASH_C, REQUEST_BYTES, FAILURE_TICK)? {
        super::HostDispatchResult::Failed { .. } => {}
        other => {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "fixture retryable request did not fail: {other:?}"
            )));
        }
    }
    if host.state().phase != super::LifecyclePhase::Failed {
        return Err(crate::error::MoltenError::invalid_harness("fixture retryable failure did not enter failed phase"));
    }
    host.restart(RECOVERY_TICK)?;
    let recovered_status = host.operator_status()?;
    if recovered_status.status.phase != super::LifecyclePhase::Running {
        return Err(crate::error::MoltenError::invalid_harness("fixture recovery did not return to running phase"));
    }
    host.dispatch_request(HASH_C, REQUEST_BYTES, POST_RECOVERY_TICK)?
        .require_executed("post-recovery request")?;
    Ok(recovered_status)
}

fn validated_conformance(
    host: &super::SystemExtensionHost<EchoExecutor>,
) -> crate::error::Result<super::ExecutableConformanceInput> {
    let required_callbacks = vec![
        super::CallbackKind::Initialize,
        super::CallbackKind::Start,
        super::CallbackKind::Request,
        super::CallbackKind::Health,
        super::CallbackKind::Checkpoint,
        super::CallbackKind::Recover,
        super::CallbackKind::Drain,
        super::CallbackKind::Shutdown,
    ];
    let conformance = host.executable_conformance_input(required_callbacks);
    let issues = super::validate_executable_conformance(&conformance);
    if !issues.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "executable fixture conformance denied: {issues:?}"
        )));
    }
    Ok(conformance)
}

fn manifest_input(profile: super::ExecutionProfile) -> super::SystemExtensionManifestInput {
    super::SystemExtensionManifestInput {
        schema: super::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: EXTENSION_ID.to_string(),
        service_id: SERVICE_ID.to_string(),
        implementation_ref: HASH_A.to_string(),
        callback_groups: vec![
            "initialize".to_string(),
            "start".to_string(),
            "request".to_string(),
            "health".to_string(),
            "checkpoint".to_string(),
            "recover".to_string(),
            "drain".to_string(),
            "shutdown".to_string(),
        ],
        required_ports: vec![port_requirement()],
        optional_ports: Vec::new(),
        capability_refs: vec![HASH_B.to_string()],
        policy_refs: vec![HASH_C.to_string()],
        provenance_refs: vec![HASH_D.to_string()],
        resources: super::ResourceEnvelope {
            max_concurrent_callbacks: MAX_CONCURRENT_CALLBACKS,
            max_queued_events: MAX_QUEUED_EVENTS,
            max_inflight_bytes: MAX_INFLIGHT_BYTES,
            max_open_streams: MAX_OPEN_STREAMS,
            max_timers: MAX_TIMERS,
            max_effect_requests: MAX_EFFECT_REQUESTS,
            callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
            shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
            max_restart_attempts: MAX_RESTART_ATTEMPTS,
            overload_policy: super::OverloadPolicy::UpstreamBackpressure,
        },
        execution_profile: profile,
        state_schema: STATE_SCHEMA.to_string(),
        compatible_state_schemas: vec![STATE_SCHEMA.to_string()],
        evidence_profile_ref: HASH_E.to_string(),
        initial_generation: INITIAL_GENERATION,
        non_claims: super::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}

fn port_descriptor() -> crate::fabric::FabricPortDescriptor {
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Transport,
        operation_classes: vec![PORT_OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![crate::fabric::FabricAuthority::Transport],
        resource_requirements: vec![
            crate::fabric::FabricResource::Concurrency,
            crate::fabric::FabricResource::NetworkBytes,
        ],
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: PORT_PROFILE.to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn port_requirement() -> crate::fabric::FabricPortRequirement {
    crate::fabric::FabricPortRequirement {
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Transport,
        operation_classes: vec![PORT_OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![crate::fabric::FabricAuthority::Transport],
        available_resources: vec![
            crate::fabric::FabricResource::Concurrency,
            crate::fabric::FabricResource::NetworkBytes,
        ],
        expected_determinism: crate::fabric::DeterminismClass::ExternalEffect,
        expected_replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        expected_profile: PORT_PROFILE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.system_extension.lifecycle]
    #[test]
    fn stale_checkpoint_denies_upgrade_before_generation_or_evidence_changes() {
        let profile = super::super::ExecutionProfile::InProcessNative;
        let tier = crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
            tier: crate::fabric::ExtensionTier::SystemExtension,
            requested_authorities: vec![
                crate::fabric::FabricAuthority::Transport,
                crate::fabric::FabricAuthority::Resources,
                crate::fabric::FabricAuthority::Supervision,
                crate::fabric::FabricAuthority::Evidence,
            ],
            admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
        })
        .expect("tier admission");
        let descriptors = [port_descriptor()];
        let admitted =
            super::super::canonical_admit_system_extension_manifest(&manifest_input(profile), &descriptors, &tier, &[
                profile,
            ])
            .expect("initial manifest");
        let mut upgrade_input = manifest_input(profile);
        upgrade_input.state_schema = UPGRADED_STATE_SCHEMA.to_string();
        upgrade_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
        let upgrade =
            super::super::canonical_admit_system_extension_manifest(&upgrade_input, &descriptors, &tier, &[profile])
                .expect("upgrade manifest");
        let mut host = super::super::SystemExtensionHost::new(admitted, EchoExecutor::new(profile).expect("executor"))
            .expect("extension host");
        host.activate(START_TICK).expect("activation");
        host.checkpoint(CHECKPOINT_TICK).expect("checkpoint");
        let before_state = host.state().clone();
        let before_evidence_count = host.evidence().len();

        let error = host
            .upgrade(upgrade, EchoExecutor::new(profile).expect("upgrade executor"), HASH_A, UPGRADE_TICK)
            .expect_err("stale checkpoint must deny upgrade");

        assert!(error.to_string().contains("checkpoint does not match"));
        assert_eq!(host.state(), &before_state);
        assert_eq!(host.evidence().len(), before_evidence_count);
    }
}
