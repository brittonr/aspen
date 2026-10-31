//! Collaboration-scope manifest admission through ordinary system-tier and exact port admission.

use super::*;

const PROFILE: &str = "collaboration-scope-test-port-v1";
const OPERATION: &str = "operate";
const INPUT_SCHEMA: &str = "molten.collaboration-scope.test-input.v1";
const OUTPUT_SCHEMA: &str = "molten.collaboration-scope.test-output.v1";
const CALLBACKS: [&str; 8] = [
    "initialize",
    "start",
    "request",
    "health",
    "checkpoint",
    "recover",
    "drain",
    "shutdown",
];

struct Port {
    port_id: String,
    class: crate::fabric::FabricPortClass,
    authority: crate::fabric::FabricAuthority,
    resource: crate::fabric::FabricResource,
}

fn ports() -> Vec<Port> {
    [
        ("authority", crate::fabric::FabricPortClass::Authority, crate::fabric::FabricAuthority::Policy),
        (
            "consistency",
            crate::fabric::FabricPortClass::Consistency,
            crate::fabric::FabricAuthority::Consistency,
        ),
        (
            "durable-state",
            crate::fabric::FabricPortClass::DurableState,
            crate::fabric::FabricAuthority::DurableState,
        ),
        ("time", crate::fabric::FabricPortClass::Time, crate::fabric::FabricAuthority::Time),
        ("resources", crate::fabric::FabricPortClass::Resources, crate::fabric::FabricAuthority::Resources),
        ("evidence", crate::fabric::FabricPortClass::Evidence, crate::fabric::FabricAuthority::Evidence),
    ]
    .into_iter()
    .map(|(name, class, authority)| Port {
        port_id: format!("molten.collaboration-scope.test.{name}"),
        class,
        authority,
        resource: crate::fabric::FabricResource::Diagnostics,
    })
    .collect()
}

fn descriptor(port: &Port) -> crate::fabric::FabricPortDescriptor {
    crate::fabric::FabricPortDescriptor {
        schema: molten_core::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: port.port_id.clone(),
        version: "v1".to_string(),
        class: port.class,
        operation_classes: vec![OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![port.authority],
        resource_requirements: vec![port.resource],
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: PROFILE.to_string(),
        conformance_refs: vec![fixtures::reference(&port.port_id)],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn requirement(port: &Port) -> crate::fabric::FabricPortRequirement {
    crate::fabric::FabricPortRequirement {
        port_id: port.port_id.clone(),
        version: "v1".to_string(),
        class: port.class,
        operation_classes: vec![OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![port.authority],
        available_resources: vec![port.resource],
        expected_determinism: crate::fabric::DeterminismClass::ExternalEffect,
        expected_replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        expected_profile: PROFILE.to_string(),
    }
}

fn tier() -> crate::fabric::CanonicalExtensionTierAdmission {
    let requested = ports().iter().map(|port| port.authority).collect();
    crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: requested,
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("system-extension tier")
}

fn input(ports: &[Port]) -> crate::system_extension::SystemExtensionManifestInput {
    crate::system_extension::SystemExtensionManifestInput {
        schema: crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: EXTENSION_ID.to_string(),
        service_id: "molten.collaboration-scope.service".to_string(),
        implementation_ref: fixtures::reference("collaboration-implementation"),
        callback_groups: CALLBACKS.iter().map(ToString::to_string).collect(),
        required_ports: ports.iter().map(requirement).collect(),
        optional_ports: Vec::new(),
        capability_refs: vec![fixtures::reference("collaboration-capability")],
        policy_refs: vec![fixtures::reference("collaboration-policy")],
        provenance_refs: vec![fixtures::reference("collaboration-provenance")],
        resources: crate::system_extension::ResourceEnvelope {
            max_concurrent_callbacks: 4,
            max_queued_events: 64,
            max_inflight_bytes: 65_536,
            max_open_streams: 4,
            max_timers: 4,
            max_effect_requests: 16,
            callback_deadline_ticks: 64,
            shutdown_grace_ticks: 16,
            max_restart_attempts: 2,
            overload_policy: crate::system_extension::OverloadPolicy::UpstreamBackpressure,
        },
        recovery_group: None,
        execution_profile: crate::system_extension::ExecutionProfile::InProcessNative,
        state_schema: SCHEMA.to_string(),
        compatible_state_schemas: vec![SCHEMA.to_string()],
        evidence_profile_ref: fixtures::reference("collaboration-evidence-profile"),
        initial_generation: 1,
        non_claims: crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}

fn admit(
    input: &crate::system_extension::SystemExtensionManifestInput,
    ports: &[Port],
) -> Result<crate::system_extension::AdmittedManifest, String> {
    let descriptors: Vec<_> = ports.iter().map(descriptor).collect();
    let tier = tier();
    let profiles = [crate::system_extension::ExecutionProfile::InProcessNative];
    admit_extension(input, AdmissionContext {
        descriptors: &descriptors,
        tier: &tier,
        execution_profiles: &profiles,
    })
    .map_err(|error| error.to_string())
}

/// The rendered refusal of a manifest that admission must deny.
fn refused(input: &crate::system_extension::SystemExtensionManifestInput, ports: &[Port]) -> String {
    admit(input, ports).map(|_| ()).expect_err("manifest refused")
}

/// The exact collaboration manifest admitted through ordinary tier and port admission.
pub(super) fn admitted() -> crate::system_extension::AdmittedManifest {
    let ports = ports();
    admit(&input(&ports), &ports).expect("exact collaboration manifest")
}

// r[verify molten.collaboration_scope.versioned_extension]
// r[verify molten.collaboration_scope.versioned_extension.admitted]
#[test]
fn valid_manifest_admits_one_generation_without_fabric_membership_ports() {
    let ports = ports();
    let admitted = admitted();
    let manifest = admitted.manifest();
    assert_eq!((manifest.extension_id.as_str(), manifest.state_schema.as_str()), (EXTENSION_ID, SCHEMA));
    assert_eq!(manifest.initial_generation, 1);
    let classes: Vec<_> = admitted.required_port_bindings().iter().map(|binding| binding.binding.class).collect();
    assert_eq!(classes.len(), ports.len());
    assert!(ports.iter().all(|port| classes.contains(&port.class)), "exactly the declared ports are bound");
    for fabric in [
        crate::fabric::FabricPortClass::Membership,
        crate::fabric::FabricPortClass::Placement,
    ] {
        assert!(!classes.contains(&fabric), "collaboration never binds fabric node membership");
    }
}

// r[verify molten.collaboration_scope.versioned_extension]
#[test]
fn manifest_admission_denies_foreign_identity_schema_bounds_callbacks_and_ports() {
    let ports = ports();
    let bound = "collaboration-scope/bound";
    let mut foreign = input(&ports);
    foreign.extension_id = "molten.fabric.membership".to_string();
    assert!(refused(&foreign, &ports).ends_with(bound));
    let mut schema = input(&ports);
    schema.state_schema = "molten.fabric-membership.v1".to_string();
    assert!(refused(&schema, &ports).ends_with(bound));
    for bytes in [0, MAX_RECORD_BYTES + 1] {
        let mut oversized = input(&ports);
        oversized.resources.max_inflight_bytes = bytes;
        assert!(refused(&oversized, &ports).ends_with(bound));
    }
    let mut queue = input(&ports);
    queue.resources.max_queued_events = u64::try_from(MAX_OPERATIONS).expect("operation bound") + 1;
    assert!(refused(&queue, &ports).ends_with(bound));
    let mut callbacks = input(&ports);
    callbacks.callback_groups.retain(|callback| callback != "recover");
    assert!(refused(&callbacks, &ports).ends_with("collaboration-scope/missing"));
    let without_time: Vec<_> =
        ports.into_iter().filter(|port| port.class != crate::fabric::FabricPortClass::Time).collect();
    let missing = refused(&input(&without_time), &without_time);
    assert!(missing.ends_with("collaboration-scope/missing"), "{missing}");
}
