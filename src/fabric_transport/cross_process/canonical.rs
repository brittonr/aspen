use preserves::IOValue;
use preserves::Value;
use preserves::ValueImpl;

use super::super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEndpointStatus {
    pub status: EndpointStatusReadback,
    pub status_ref: String,
    pub value: IOValue,
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn canonical_cross_process_endpoint(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    bindings: &EndpointDescriptorBindings,
) -> Result<CanonicalCrossProcessEndpoint> {
    let binding_value = endpoint_binding_value(profile, protocol, bindings);
    let descriptor_ref = canonical_hash(&binding_value)?;
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
    let handoff_ref = canonical_hash(&value)?;
    Ok(CanonicalCrossProcessEndpoint {
        descriptor,
        descriptor_ref,
        handoff_ref,
        value,
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn parse_canonical_cross_process_endpoint(value: &IOValue) -> Result<CanonicalCrossProcessEndpoint> {
    let outer = simple_record(value, ENDPOINT_DESCRIPTOR_RECORD, ENDPOINT_DESCRIPTOR_FIELD_COUNT)?;
    let mut outer = outer.as_slice().iter();
    let schema = required_string(next_field(&mut outer, "endpoint handoff schema")?, "endpoint handoff schema")?;
    if schema != CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA {
        return Err(MoltenError::invalid_harness("cross-process endpoint handoff schema mismatch"));
    }
    let declared_descriptor_ref =
        required_ref(next_field(&mut outer, "endpoint descriptor ref")?, "endpoint descriptor ref")?;
    let binding_value = crate::preserves_rail::value_to_iovalue(next_field(&mut outer, "endpoint binding")?);
    let actual_descriptor_ref = canonical_hash(&binding_value)?;
    if declared_descriptor_ref != actual_descriptor_ref {
        return Err(MoltenError::invalid_harness("cross-process endpoint descriptor ref mismatch"));
    }
    let descriptor = parse_endpoint_binding(&binding_value, &declared_descriptor_ref)?;
    let handoff_ref = canonical_hash(value)?;
    Ok(CanonicalCrossProcessEndpoint {
        descriptor,
        descriptor_ref: declared_descriptor_ref,
        handoff_ref,
        value: value.clone(),
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn canonical_endpoint_status(descriptor: &CrossProcessEndpointDescriptor) -> Result<CanonicalEndpointStatus> {
    let status = endpoint_status_readback(descriptor);
    let value = record(ENDPOINT_STATUS_RECORD, vec![
        string(CROSS_PROCESS_ENDPOINT_STATUS_SCHEMA),
        field("descriptor-ref", string(&status.descriptor_ref)),
        field("public-endpoint-identity", string(&status.public_endpoint_identity)),
        field("profile-id", string(&status.profile_id)),
        field("protocol-id", string(&status.protocol_id)),
        field("service-id", string(&status.service_id)),
        field("generation", u64_value(status.generation)),
        field("locator-cohort-ref", string(&status.locator_cohort_ref)),
        field("locator-classes", strings_value(status.locator_classes.iter().map(|class| class.as_str()))),
        field("validity-cohort-ref", string(&status.validity_cohort_ref)),
        field("non-claims", strings_value(status.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "raw-locators-redacted",
            "secrets-excluded",
            "runtime-handles-excluded",
            "connectivity-is-not-authority",
        ]),
    ]);
    let status_ref = canonical_hash(&value)?;
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
) -> IOValue {
    record(ENDPOINT_BINDING_RECORD, vec![
        string(CROSS_PROCESS_ENDPOINT_SCHEMA),
        string(&profile.profile_id),
        string(&profile.profile_ref),
        string(&protocol.protocol_id),
        string(&protocol.version),
        string(&protocol.alpn),
        string(&protocol.extension_id),
        string(&protocol.service_id),
        u64_value(protocol.generation),
        string(&bindings.public_endpoint_identity),
        string(&bindings.listener_identity_ref),
        string(&bindings.expected_peer_context_ref),
        string(&bindings.locator_cohort_ref),
        sequence(
            bindings
                .locators
                .iter()
                .map(|locator| record(LOCATOR_RECORD, vec![string(locator.class.as_str()), string(&locator.value)]))
                .collect(),
        ),
        strings_value(bindings.disclosure.explicit_handoff_classes.iter().map(|class| class.as_str())),
        bool_value(bindings.disclosure.default_readback_redacted),
        string(&protocol.framing.profile_ref),
        record(RESOURCES_RECORD, vec![
            u64_value(bindings.resources.max_sessions),
            u64_value(bindings.resources.max_frame_bytes),
            u64_value(bindings.resources.max_queued_bytes),
            u64_value(bindings.resources.max_inflight_bytes),
        ]),
        record(VALIDITY_RECORD, vec![
            string(&bindings.validity.cohort_ref),
            u64_value(bindings.validity.not_before_tick),
            u64_value(bindings.validity.expires_at_tick),
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

fn endpoint_descriptor_value(descriptor_ref: &str, binding_value: IOValue) -> IOValue {
    record(ENDPOINT_DESCRIPTOR_RECORD, vec![
        string(CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA),
        string(descriptor_ref),
        binding_value,
        checks(&[
            "binding-ref-recomputed-on-import",
            "explicit-handoff-only",
            "possession-is-not-authority",
        ]),
    ])
}

fn parse_endpoint_binding(value: &IOValue, descriptor_ref: &str) -> Result<CrossProcessEndpointDescriptor> {
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
    let default_readback_redacted = required_bool(next_field(&mut fields, "default redaction")?, "default redaction")?;
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
            default_readback_redacted,
        },
        framing_profile_ref,
        resources,
        validity,
        non_claims,
    })
}

fn parse_locators(value: &Value<IOValue>) -> Result<Vec<EndpointLocator>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("cross-process endpoint locators must be a sequence"))?;
    if values.len() > MAX_CANONICAL_LOCATORS {
        return Err(MoltenError::invalid_harness("cross-process endpoint locator count exceeds bound"));
    }
    values
        .iter()
        .map(|value| {
            let value = crate::preserves_rail::value_to_iovalue(&value);
            let fields = simple_record(&value, LOCATOR_RECORD, LOCATOR_FIELD_COUNT)?;
            Ok(EndpointLocator {
                class: parse_locator_class(&required_string(&fields[0], "locator class")?)?,
                value: required_string(&fields[1], "locator value")?,
            })
        })
        .collect()
}

fn parse_locator_classes(value: &Value<IOValue>) -> Result<Vec<EndpointLocatorClass>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("endpoint disclosure classes must be a sequence"))?;
    if values.len() > MAX_CANONICAL_LOCATORS {
        return Err(MoltenError::invalid_harness("endpoint disclosure class count exceeds bound"));
    }
    values
        .iter()
        .map(|value| parse_locator_class(&required_string(&value, "endpoint disclosure class")?))
        .collect()
}

fn parse_locator_class(value: &str) -> Result<EndpointLocatorClass> {
    match value {
        "ip" => Ok(EndpointLocatorClass::Ip),
        "relay" => Ok(EndpointLocatorClass::Relay),
        "custom" => Ok(EndpointLocatorClass::Custom),
        "private" => Ok(EndpointLocatorClass::Private),
        other => Err(MoltenError::invalid_harness(format!("unsupported endpoint locator class {other}"))),
    }
}

fn parse_resources(value: &Value<IOValue>) -> Result<EndpointResourceBounds> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, RESOURCES_RECORD, RESOURCE_FIELD_COUNT)?;
    Ok(EndpointResourceBounds {
        max_sessions: required_u64(&fields[0], "max sessions")?,
        max_frame_bytes: required_u64(&fields[1], "max frame bytes")?,
        max_queued_bytes: required_u64(&fields[RESOURCE_QUEUED_INDEX], "max queued bytes")?,
        max_inflight_bytes: required_u64(&fields[RESOURCE_INFLIGHT_INDEX], "max inflight bytes")?,
    })
}

fn parse_validity(value: &Value<IOValue>) -> Result<EndpointValidityCohort> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, VALIDITY_RECORD, VALIDITY_FIELD_COUNT)?;
    Ok(EndpointValidityCohort {
        cohort_ref: required_ref(&fields[0], "validity cohort ref")?,
        not_before_tick: required_u64(&fields[1], "valid from tick")?,
        expires_at_tick: required_u64(&fields[VALIDITY_EXPIRY_INDEX], "valid until tick")?,
    })
}

fn parse_non_claims(value: &Value<IOValue>) -> Result<Vec<TransportNonClaim>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("endpoint non-claims must be a sequence"))?;
    values
        .iter()
        .map(|value| parse_non_claim(&required_string(&value, "endpoint non-claim")?))
        .collect()
}

fn parse_non_claim(value: &str) -> Result<TransportNonClaim> {
    REQUIRED_TRANSPORT_NON_CLAIMS
        .into_iter()
        .find(|claim| claim.as_str() == value)
        .ok_or_else(|| MoltenError::invalid_harness(format!("unsupported endpoint non-claim {value}")))
}

fn field(name: &str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn checks(values: &[&str]) -> IOValue {
    record(CHECKS_RECORD, vec![strings_value(values.iter().copied())])
}

fn simple_record(value: &IOValue, label: &str, field_count: usize) -> Result<Vec<Value<IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(field_count))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(fields.iter().collect())
}

fn next_field<'a, 'b>(
    fields: &mut impl Iterator<Item = &'a Value<IOValue>>,
    label: &'b str,
) -> Result<&'a Value<IOValue>> {
    fields
        .next()
        .ok_or_else(|| MoltenError::invalid_harness(format!("cross-process endpoint missing {label}")))
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = required_string(value, label)?;
    crate::preserves_rail::validate_content_ref(&value)?;
    Ok(value)
}

fn required_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    value.as_boolean().ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}
