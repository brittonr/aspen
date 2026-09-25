use preserves::ValueImpl;

use super::super::*;

pub const CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA: &str = "molten.fabric.transport.endpoint-handoff.v1";
pub const CROSS_PROCESS_ENDPOINT_STATUS_SCHEMA: &str = "molten.fabric.transport.endpoint-status.v1";

const ENDPOINT_BINDING_RECORD: &str = "fabric-transport-endpoint-binding-v1";
const ENDPOINT_DESCRIPTOR_RECORD: &str = "fabric-transport-endpoint-descriptor-v1";
const ENDPOINT_STATUS_RECORD: &str = "fabric-transport-endpoint-status-v1";
const LOCATOR_RECORD: &str = "locator";
const RESOURCES_RECORD: &str = "resources";
const VALIDITY_RECORD: &str = "validity";
const CHECKS_RECORD: &str = "checks";
const ENDPOINT_BINDING_FIELD_COUNT: usize = 21;
const ENDPOINT_DESCRIPTOR_FIELD_COUNT: usize = 4;
const LOCATOR_FIELD_COUNT: usize = 2;
const RESOURCE_FIELD_COUNT: usize = 4;
const RESOURCE_QUEUED_INDEX: usize = 2;
const RESOURCE_INFLIGHT_INDEX: usize = 3;
const VALIDITY_FIELD_COUNT: usize = 3;
const VALIDITY_EXPIRY_INDEX: usize = 2;
const MAX_CANONICAL_LOCATORS: usize = MAX_ENDPOINT_LOCATORS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDescriptorBindings {
    pub public_endpoint_identity: String,
    pub listener_identity_ref: String,
    pub expected_peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub locators: Vec<EndpointLocator>,
    pub disclosure: EndpointDisclosurePolicy,
    pub resources: EndpointResourceBounds,
    pub validity: EndpointValidityCohort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCrossProcessEndpoint {
    pub descriptor: CrossProcessEndpointDescriptor,
    pub descriptor_ref: String,
    pub handoff_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEndpointStatus {
    pub status: EndpointStatusReadback,
    pub status_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn canonical_cross_process_endpoint(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    bindings: &EndpointDescriptorBindings,
) -> crate::error::Result<CanonicalCrossProcessEndpoint> {
    let binding_value = endpoint_binding_value(profile, protocol, bindings);
    let descriptor_ref = crate::preserves_rail::canonical_hash(&binding_value)?;
    let descriptor = CrossProcessEndpointDescriptor {
        schema: CROSS_PROCESS_ENDPOINT_SCHEMA.to_string(),
        descriptor_ref: descriptor_ref.clone(),
        profile_id: profile.profile_id.clone(),
        profile_ref: profile.profile_ref.clone(),
        protocol_id: protocol.protocol_id.clone(),
        protocol_version: protocol.version.clone(),
        alpn: protocol.alpn.clone(),
        extension_id: protocol.extension_id.clone(),
        service_id: protocol.service_id.clone(),
        generation: protocol.generation,
        public_endpoint_identity: bindings.public_endpoint_identity.clone(),
        listener_identity_ref: bindings.listener_identity_ref.clone(),
        expected_peer_context_ref: bindings.expected_peer_context_ref.clone(),
        locator_cohort_ref: bindings.locator_cohort_ref.clone(),
        locators: bindings.locators.clone(),
        disclosure: bindings.disclosure.clone(),
        framing_profile_ref: protocol.framing.profile_ref.clone(),
        resources: bindings.resources.clone(),
        validity: bindings.validity.clone(),
        non_claims: profile.non_claims.clone(),
    };
    validate_cross_process_endpoint(profile, protocol, &descriptor)
        .map_err(|issues| validation_error("canonical endpoint descriptor", &issues))?;
    let value = endpoint_descriptor_value(&descriptor_ref, binding_value);
    let handoff_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalCrossProcessEndpoint {
        descriptor,
        descriptor_ref,
        handoff_ref,
        value,
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn parse_canonical_cross_process_endpoint(
    value: &preserves::IOValue,
) -> crate::error::Result<CanonicalCrossProcessEndpoint> {
    let outer = simple_record(value, ENDPOINT_DESCRIPTOR_RECORD, ENDPOINT_DESCRIPTOR_FIELD_COUNT)?;
    let mut outer = outer.as_slice().iter();
    let schema = required_string(next_field(&mut outer, "endpoint handoff schema")?, "endpoint handoff schema")?;
    if schema != CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("cross-process endpoint handoff schema mismatch"));
    }
    let declared_descriptor_ref =
        required_ref(next_field(&mut outer, "endpoint descriptor ref")?, "endpoint descriptor ref")?;
    let binding_value = crate::preserves_rail::value_to_iovalue(next_field(&mut outer, "endpoint binding")?);
    let actual_descriptor_ref = crate::preserves_rail::canonical_hash(&binding_value)?;
    if declared_descriptor_ref != actual_descriptor_ref {
        return Err(crate::error::MoltenError::invalid_harness("cross-process endpoint descriptor ref mismatch"));
    }
    let descriptor = parse_endpoint_binding(&binding_value, &declared_descriptor_ref)?;
    let handoff_ref = crate::preserves_rail::canonical_hash(value)?;
    Ok(CanonicalCrossProcessEndpoint {
        descriptor,
        descriptor_ref: declared_descriptor_ref,
        handoff_ref,
        value: value.clone(),
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn canonical_endpoint_status(
    descriptor: &CrossProcessEndpointDescriptor,
) -> crate::error::Result<CanonicalEndpointStatus> {
    let status = endpoint_status_readback(descriptor);
    let value = crate::preserves_rail::record(ENDPOINT_STATUS_RECORD, vec![
        crate::preserves_rail::string(CROSS_PROCESS_ENDPOINT_STATUS_SCHEMA),
        field("descriptor-ref", crate::preserves_rail::string(&status.descriptor_ref)),
        field("public-endpoint-identity", crate::preserves_rail::string(&status.public_endpoint_identity)),
        field("profile-id", crate::preserves_rail::string(&status.profile_id)),
        field("protocol-id", crate::preserves_rail::string(&status.protocol_id)),
        field("service-id", crate::preserves_rail::string(&status.service_id)),
        field("generation", crate::preserves_rail::u64_value(status.generation)),
        field("locator-cohort-ref", crate::preserves_rail::string(&status.locator_cohort_ref)),
        field("locator-classes", strings_value(status.locator_classes.iter().map(|class| class.as_str()))),
        field("validity-cohort-ref", crate::preserves_rail::string(&status.validity_cohort_ref)),
        field("non-claims", strings_value(status.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "raw-locators-redacted",
            "secrets-excluded",
            "runtime-handles-excluded",
            "connectivity-is-not-authority",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalEndpointStatus {
        status,
        status_ref,
        value,
    })
}

fn endpoint_binding_value(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    bindings: &EndpointDescriptorBindings,
) -> preserves::IOValue {
    crate::preserves_rail::record(ENDPOINT_BINDING_RECORD, vec![
        crate::preserves_rail::string(CROSS_PROCESS_ENDPOINT_SCHEMA),
        crate::preserves_rail::string(&profile.profile_id),
        crate::preserves_rail::string(&profile.profile_ref),
        crate::preserves_rail::string(&protocol.protocol_id),
        crate::preserves_rail::string(&protocol.version),
        crate::preserves_rail::string(&protocol.alpn),
        crate::preserves_rail::string(&protocol.extension_id),
        crate::preserves_rail::string(&protocol.service_id),
        crate::preserves_rail::u64_value(protocol.generation),
        crate::preserves_rail::string(&bindings.public_endpoint_identity),
        crate::preserves_rail::string(&bindings.listener_identity_ref),
        crate::preserves_rail::string(&bindings.expected_peer_context_ref),
        crate::preserves_rail::string(&bindings.locator_cohort_ref),
        crate::preserves_rail::sequence(
            bindings
                .locators
                .iter()
                .map(|locator| {
                    crate::preserves_rail::record(LOCATOR_RECORD, vec![
                        crate::preserves_rail::string(locator.class.as_str()),
                        crate::preserves_rail::string(&locator.value),
                    ])
                })
                .collect(),
        ),
        strings_value(bindings.disclosure.explicit_handoff_classes.iter().map(|class| class.as_str())),
        crate::preserves_rail::bool_value(bindings.disclosure.default_readback_redacted),
        crate::preserves_rail::string(&protocol.framing.profile_ref),
        crate::preserves_rail::record(RESOURCES_RECORD, vec![
            crate::preserves_rail::u64_value(bindings.resources.max_sessions),
            crate::preserves_rail::u64_value(bindings.resources.max_frame_bytes),
            crate::preserves_rail::u64_value(bindings.resources.max_queued_bytes),
            crate::preserves_rail::u64_value(bindings.resources.max_inflight_bytes),
        ]),
        crate::preserves_rail::record(VALIDITY_RECORD, vec![
            crate::preserves_rail::string(&bindings.validity.cohort_ref),
            crate::preserves_rail::u64_value(bindings.validity.not_before_tick),
            crate::preserves_rail::u64_value(bindings.validity.expires_at_tick),
        ]),
        strings_value(profile.non_claims.iter().map(|claim| claim.as_str())),
        checks(&[
            "exact-profile-and-protocol-bound",
            "locator-disclosure-explicit",
            "validity-and-resource-cohorts-bound",
            "secrets-and-runtime-handles-excluded",
        ]),
    ])
}

fn endpoint_descriptor_value(descriptor_ref: &str, binding_value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(ENDPOINT_DESCRIPTOR_RECORD, vec![
        crate::preserves_rail::string(CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA),
        crate::preserves_rail::string(descriptor_ref),
        binding_value,
        checks(&[
            "binding-ref-recomputed-on-import",
            "explicit-handoff-only",
            "possession-is-not-authority",
        ]),
    ])
}

fn parse_endpoint_binding(
    value: &preserves::IOValue,
    descriptor_ref: &str,
) -> crate::error::Result<CrossProcessEndpointDescriptor> {
    let fields = simple_record(value, ENDPOINT_BINDING_RECORD, ENDPOINT_BINDING_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    let schema = required_string(next_field(&mut fields, "endpoint schema")?, "endpoint schema")?;
    let profile_id = required_string(next_field(&mut fields, "profile id")?, "profile id")?;
    let profile_ref = required_ref(next_field(&mut fields, "profile ref")?, "profile ref")?;
    let protocol_id = required_string(next_field(&mut fields, "protocol id")?, "protocol id")?;
    let protocol_version = required_string(next_field(&mut fields, "protocol version")?, "protocol version")?;
    let alpn = required_string(next_field(&mut fields, "ALPN")?, "ALPN")?;
    let extension_id = required_string(next_field(&mut fields, "extension id")?, "extension id")?;
    let service_id = required_string(next_field(&mut fields, "service id")?, "service id")?;
    let generation = required_u64(next_field(&mut fields, "generation")?, "generation")?;
    let public_endpoint_identity =
        required_string(next_field(&mut fields, "public endpoint identity")?, "public endpoint identity")?;
    let listener_identity_ref =
        required_ref(next_field(&mut fields, "listener identity ref")?, "listener identity ref")?;
    let expected_peer_context_ref = required_ref(next_field(&mut fields, "peer context ref")?, "peer context ref")?;
    let locator_cohort_ref = required_ref(next_field(&mut fields, "locator cohort ref")?, "locator cohort ref")?;
    let locators = parse_locators(next_field(&mut fields, "locators")?)?;
    let explicit_handoff_classes = parse_locator_classes(next_field(&mut fields, "disclosure classes")?)?;
    let is_default_readback_redacted =
        required_bool(next_field(&mut fields, "default redaction")?, "default redaction")?;
    let framing_profile_ref = required_ref(next_field(&mut fields, "framing ref")?, "framing ref")?;
    let resources = parse_resources(next_field(&mut fields, "resources")?)?;
    let validity = parse_validity(next_field(&mut fields, "validity")?)?;
    let non_claims = parse_non_claims(next_field(&mut fields, "non-claims")?)?;
    let _checks = next_field(&mut fields, "checks")?;
    Ok(CrossProcessEndpointDescriptor {
        schema,
        descriptor_ref: descriptor_ref.to_string(),
        profile_id,
        profile_ref,
        protocol_id,
        protocol_version,
        alpn,
        extension_id,
        service_id,
        generation,
        public_endpoint_identity,
        listener_identity_ref,
        expected_peer_context_ref,
        locator_cohort_ref,
        locators,
        disclosure: EndpointDisclosurePolicy {
            explicit_handoff_classes,
            default_readback_redacted: is_default_readback_redacted,
        },
        framing_profile_ref,
        resources,
        validity,
        non_claims,
    })
}
