use std::collections::BTreeSet;

use super::*;

pub const CROSS_PROCESS_ENDPOINT_SCHEMA: &str = "molten.fabric.transport.cross-process-endpoint.v1";
pub const MAX_ENDPOINT_LOCATORS: usize = 16;
pub const MAX_ENDPOINT_LOCATOR_BYTES: usize = 512;

const IROH_ENDPOINT_PREFIX: &str = "iroh:";
const IP_LOCATOR_PREFIX: &str = "ip:";
const RELAY_LOCATOR_PREFIX: &str = "relay:";
const CUSTOM_LOCATOR_PREFIX: &str = "custom:";
const FORBIDDEN_SECRET_MARKERS: [&str; 5] = ["private-key", "secret-key", "seed:", "bearer:", "token:"];
const FORBIDDEN_HANDLE_MARKERS: [&str; 5] = ["iroh::", "quic::", "socket:", "executor:", "raw-handle:"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EndpointParticipantRole {
    Listener,
    Client,
}

impl EndpointParticipantRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Listener => "listener",
            Self::Client => "client",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EndpointLocatorClass {
    Ip,
    Relay,
    Custom,
    Private,
}

impl EndpointLocatorClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::Relay => "relay",
            Self::Custom => "custom",
            Self::Private => "private",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EndpointLocator {
    pub class: EndpointLocatorClass,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDisclosurePolicy {
    pub explicit_handoff_classes: Vec<EndpointLocatorClass>,
    pub default_readback_redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointValidityCohort {
    pub cohort_ref: String,
    pub not_before_tick: u64,
    pub expires_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointResourceBounds {
    pub max_sessions: u64,
    pub max_frame_bytes: u64,
    pub max_queued_bytes: u64,
    pub max_inflight_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessEndpointDescriptor {
    pub schema: String,
    pub descriptor_ref: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub protocol_id: String,
    pub protocol_version: String,
    pub alpn: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub public_endpoint_identity: String,
    pub listener_identity_ref: String,
    pub expected_peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub locators: Vec<EndpointLocator>,
    pub disclosure: EndpointDisclosurePolicy,
    pub framing_profile_ref: String,
    pub resources: EndpointResourceBounds,
    pub validity: EndpointValidityCohort,
    pub non_claims: Vec<TransportNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointAdmissionState {
    pub registration_active: bool,
    pub transport_capability_active: bool,
    pub protocol_capability_active: bool,
    pub profile_active: bool,
    pub listener_ready: bool,
}

impl EndpointAdmissionState {
    pub const fn fully_active() -> Self {
        Self {
            registration_active: true,
            transport_capability_active: true,
            protocol_capability_active: true,
            profile_active: true,
            listener_ready: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedEndpointBinding {
    pub profile_id: String,
    pub profile_ref: String,
    pub protocol_id: String,
    pub protocol_version: String,
    pub alpn: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub public_endpoint_identity: String,
    pub listener_identity_ref: String,
    pub peer_context_ref: String,
    pub observed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDialPlan {
    pub descriptor_ref: String,
    pub public_endpoint_identity: String,
    pub locators: Vec<EndpointLocator>,
    pub profile_id: String,
    pub protocol_id: String,
    pub alpn: String,
    pub service_id: String,
    pub generation: u64,
    pub peer_context_ref: String,
    pub resources: EndpointResourceBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointStatusReadback {
    pub descriptor_ref: String,
    pub public_endpoint_identity: String,
    pub profile_id: String,
    pub protocol_id: String,
    pub service_id: String,
    pub generation: u64,
    pub locator_cohort_ref: String,
    pub locator_classes: Vec<EndpointLocatorClass>,
    pub validity_cohort_ref: String,
    pub non_claims: Vec<TransportNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointExportPlan {
    pub descriptor: CrossProcessEndpointDescriptor,
    pub status: EndpointStatusReadback,
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn validate_cross_process_endpoint(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    descriptor: &CrossProcessEndpointDescriptor,
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    let mut issues = Vec::new();
    if validate_transport_profile(profile).is_err() {
        issues.push(CrossProcessTransportIssue::InvalidTransportProfile);
    }
    if validate_protocol_descriptor(profile, protocol).is_err() {
        issues.push(CrossProcessTransportIssue::InvalidProtocolDescriptor);
    }
    if profile.adapter_kind != TransportAdapterKind::IrohLive {
        issues.push(CrossProcessTransportIssue::ProfileAdapterMismatch);
    }
    if descriptor.schema != CROSS_PROCESS_ENDPOINT_SCHEMA {
        issues.push(CrossProcessTransportIssue::EndpointSchemaMismatch);
    }
    validate_ref("descriptor-ref", &descriptor.descriptor_ref, &mut issues);
    validate_token("endpoint-profile-id", &descriptor.profile_id, &mut issues);
    validate_ref("endpoint-profile-ref", &descriptor.profile_ref, &mut issues);
    validate_token("endpoint-protocol-id", &descriptor.protocol_id, &mut issues);
    validate_token("endpoint-protocol-version", &descriptor.protocol_version, &mut issues);
    validate_alpn(&descriptor.alpn, &mut issues);
    validate_token("endpoint-extension-id", &descriptor.extension_id, &mut issues);
    validate_token("endpoint-service-id", &descriptor.service_id, &mut issues);
    validate_ref("listener-identity-ref", &descriptor.listener_identity_ref, &mut issues);
    validate_ref("peer-context-ref", &descriptor.expected_peer_context_ref, &mut issues);
    validate_ref("locator-cohort-ref", &descriptor.locator_cohort_ref, &mut issues);
    validate_ref("framing-profile-ref", &descriptor.framing_profile_ref, &mut issues);
    validate_ref("validity-cohort-ref", &descriptor.validity.cohort_ref, &mut issues);
    validate_endpoint_identity(&descriptor.public_endpoint_identity, &mut issues);

    if descriptor.profile_id != profile.profile_id || descriptor.profile_ref != profile.profile_ref {
        issues.push(CrossProcessTransportIssue::ProfileIdentityMismatch);
    }
    if descriptor.protocol_id != protocol.protocol_id || descriptor.protocol_version != protocol.version {
        issues.push(CrossProcessTransportIssue::ProtocolIdentityMismatch);
    }
    if descriptor.alpn != protocol.alpn {
        issues.push(CrossProcessTransportIssue::AlpnMismatch);
    }
    if descriptor.extension_id != protocol.extension_id {
        issues.push(CrossProcessTransportIssue::ExtensionIdentityMismatch);
    }
    if descriptor.service_id != protocol.service_id {
        issues.push(CrossProcessTransportIssue::ServiceIdentityMismatch);
    }
    if descriptor.generation != protocol.generation {
        issues.push(CrossProcessTransportIssue::GenerationMismatch);
    }
    if descriptor.framing_profile_ref != protocol.framing.profile_ref
        || descriptor.resources.max_frame_bytes != protocol.framing.max_frame_bytes
    {
        issues.push(CrossProcessTransportIssue::FramingProfileMismatch);
    }

    validate_resources(profile, descriptor, &mut issues);
    validate_validity(&descriptor.validity, None, &mut issues);
    validate_disclosure(descriptor, &mut issues);
    for required in REQUIRED_TRANSPORT_NON_CLAIMS {
        if !descriptor.non_claims.contains(&required) {
            issues.push(CrossProcessTransportIssue::MissingNonClaim(required));
        }
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn plan_endpoint_export(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    descriptor: &CrossProcessEndpointDescriptor,
    listener: &CrossProcessListenerState,
    admission: EndpointAdmissionState,
    observed_tick: u64,
) -> Result<EndpointExportPlan, Vec<CrossProcessTransportIssue>> {
    let mut issues = validate_cross_process_endpoint(profile, protocol, descriptor).err().unwrap_or_default();
    validate_admission(admission, &mut issues);
    if !listener.is_ready() {
        issues.push(CrossProcessTransportIssue::ListenerNotReady);
    }
    validate_listener_binding(listener, descriptor, &mut issues);
    validate_validity(&descriptor.validity, Some(observed_tick), &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(EndpointExportPlan {
        descriptor: descriptor.clone(),
        status: endpoint_status_readback(descriptor),
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn admit_endpoint_import(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    descriptor: &CrossProcessEndpointDescriptor,
    expected: &ExpectedEndpointBinding,
    admission: EndpointAdmissionState,
) -> Result<EndpointDialPlan, Vec<CrossProcessTransportIssue>> {
    let mut issues = validate_cross_process_endpoint(profile, protocol, descriptor).err().unwrap_or_default();
    validate_admission(admission, &mut issues);
    validate_expected_binding(descriptor, expected, &mut issues);
    validate_validity(&descriptor.validity, Some(expected.observed_tick), &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(EndpointDialPlan {
        descriptor_ref: descriptor.descriptor_ref.clone(),
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        locators: descriptor.locators.clone(),
        profile_id: descriptor.profile_id.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        resources: descriptor.resources.clone(),
    })
}

// r[impl molten.fabric_transport.cross_process_endpoint]
pub fn endpoint_status_readback(descriptor: &CrossProcessEndpointDescriptor) -> EndpointStatusReadback {
    let mut locator_classes = descriptor.locators.iter().map(|locator| locator.class).collect::<Vec<_>>();
    locator_classes.sort();
    locator_classes.dedup();
    EndpointStatusReadback {
        descriptor_ref: descriptor.descriptor_ref.clone(),
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        profile_id: descriptor.profile_id.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        locator_cohort_ref: descriptor.locator_cohort_ref.clone(),
        locator_classes,
        validity_cohort_ref: descriptor.validity.cohort_ref.clone(),
        non_claims: descriptor.non_claims.clone(),
    }
}

fn validate_endpoint_identity(value: &str, issues: &mut Vec<CrossProcessTransportIssue>) {
    let Some(identity) = value.strip_prefix(IROH_ENDPOINT_PREFIX) else {
        issues.push(CrossProcessTransportIssue::MalformedField("public-endpoint-identity"));
        return;
    };
    if identity.is_empty()
        || identity.len() > MAX_TRANSPORT_TEXT_BYTES
        || !identity.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        issues.push(CrossProcessTransportIssue::MalformedField("public-endpoint-identity"));
    }
}

fn validate_resources(
    profile: &TransportProfile,
    descriptor: &CrossProcessEndpointDescriptor,
    issues: &mut Vec<CrossProcessTransportIssue>,
) {
    for (field, value, maximum) in [
        ("max-sessions", descriptor.resources.max_sessions, profile.limits.max_sessions),
        ("max-frame-bytes", descriptor.resources.max_frame_bytes, profile.limits.max_frame_bytes),
        ("max-queued-bytes", descriptor.resources.max_queued_bytes, profile.limits.max_queued_bytes),
        ("max-inflight-bytes", descriptor.resources.max_inflight_bytes, profile.limits.max_inflight_bytes),
    ] {
        if value == 0 {
            issues.push(CrossProcessTransportIssue::InvalidResourceBound(field));
        } else if value > maximum {
            issues.push(CrossProcessTransportIssue::ResourceBoundExceeded(field));
        }
    }
    if descriptor.resources.max_frame_bytes > descriptor.resources.max_queued_bytes
        || descriptor.resources.max_frame_bytes > descriptor.resources.max_inflight_bytes
    {
        issues.push(CrossProcessTransportIssue::ResourceBoundExceeded("frame-accounting"));
    }
}

fn validate_validity(
    validity: &EndpointValidityCohort,
    observed_tick: Option<u64>,
    issues: &mut Vec<CrossProcessTransportIssue>,
) {
    if validity.not_before_tick >= validity.expires_at_tick {
        issues.push(CrossProcessTransportIssue::InvalidValidityCohort);
        return;
    }
    if let Some(observed_tick) = observed_tick {
        if observed_tick < validity.not_before_tick {
            issues.push(CrossProcessTransportIssue::ValidityCohortNotStarted);
        }
        if observed_tick >= validity.expires_at_tick {
            issues.push(CrossProcessTransportIssue::ValidityCohortExpired);
        }
    }
}

fn validate_disclosure(descriptor: &CrossProcessEndpointDescriptor, issues: &mut Vec<CrossProcessTransportIssue>) {
    if !descriptor.disclosure.default_readback_redacted {
        issues.push(CrossProcessTransportIssue::DefaultReadbackNotRedacted);
    }
    if descriptor.locators.is_empty() {
        issues.push(CrossProcessTransportIssue::MissingLocator);
    }
    if descriptor.locators.len() > MAX_ENDPOINT_LOCATORS {
        issues.push(CrossProcessTransportIssue::LocatorLimitExceeded);
    }
    let allowed = descriptor.disclosure.explicit_handoff_classes.iter().copied().collect::<BTreeSet<_>>();
    if allowed.len() != descriptor.disclosure.explicit_handoff_classes.len() {
        issues.push(CrossProcessTransportIssue::DuplicateLocatorClass);
    }
    if allowed.contains(&EndpointLocatorClass::Custom) {
        issues.push(CrossProcessTransportIssue::UnsupportedLocatorClass);
    }
    if allowed.contains(&EndpointLocatorClass::Private) {
        issues.push(CrossProcessTransportIssue::PrivateLocatorDisclosure);
    }

    let mut observed = BTreeSet::new();
    for locator in &descriptor.locators {
        validate_locator(locator, &allowed, issues);
        if !observed.insert(locator.clone()) {
            issues.push(CrossProcessTransportIssue::DuplicateLocator);
        }
    }
}

fn validate_locator(
    locator: &EndpointLocator,
    allowed: &BTreeSet<EndpointLocatorClass>,
    issues: &mut Vec<CrossProcessTransportIssue>,
) {
    if locator.value.is_empty() || locator.value.len() > MAX_ENDPOINT_LOCATOR_BYTES {
        issues.push(CrossProcessTransportIssue::MalformedField("locator"));
    }
    let normalized = locator.value.to_ascii_lowercase();
    if FORBIDDEN_SECRET_MARKERS.iter().any(|marker| normalized.contains(marker)) {
        issues.push(CrossProcessTransportIssue::SecretBearingLocator);
    }
    if FORBIDDEN_HANDLE_MARKERS.iter().any(|marker| normalized.contains(marker)) {
        issues.push(CrossProcessTransportIssue::RawHandleLocator);
    }
    if locator.class == EndpointLocatorClass::Private {
        issues.push(CrossProcessTransportIssue::PrivateLocatorDisclosure);
    }
    if locator.class == EndpointLocatorClass::Custom {
        issues.push(CrossProcessTransportIssue::UnsupportedLocatorClass);
    }
    if !allowed.contains(&locator.class) {
        issues.push(CrossProcessTransportIssue::UndeclaredLocatorClass);
    }
    let class_matches = match locator.class {
        EndpointLocatorClass::Ip => locator.value.starts_with(IP_LOCATOR_PREFIX),
        EndpointLocatorClass::Relay => locator.value.starts_with(RELAY_LOCATOR_PREFIX),
        EndpointLocatorClass::Custom => locator.value.starts_with(CUSTOM_LOCATOR_PREFIX),
        EndpointLocatorClass::Private => false,
    };
    if !class_matches {
        issues.push(CrossProcessTransportIssue::LocatorClassMismatch);
    }
}

fn validate_admission(admission: EndpointAdmissionState, issues: &mut Vec<CrossProcessTransportIssue>) {
    if !admission.registration_active {
        issues.push(CrossProcessTransportIssue::RegistrationRevoked);
    }
    if !admission.transport_capability_active {
        issues.push(CrossProcessTransportIssue::TransportCapabilityRevoked);
    }
    if !admission.protocol_capability_active {
        issues.push(CrossProcessTransportIssue::ProtocolCapabilityRevoked);
    }
    if !admission.profile_active {
        issues.push(CrossProcessTransportIssue::ProfileRevoked);
    }
    if !admission.listener_ready {
        issues.push(CrossProcessTransportIssue::ListenerNotReady);
    }
}

fn validate_listener_binding(
    listener: &CrossProcessListenerState,
    descriptor: &CrossProcessEndpointDescriptor,
    issues: &mut Vec<CrossProcessTransportIssue>,
) {
    if listener.identity.listener_identity_ref != descriptor.listener_identity_ref
        || listener.identity.descriptor_ref != descriptor.descriptor_ref
    {
        issues.push(CrossProcessTransportIssue::ListenerIdentityMismatch);
    }
    if listener.identity.profile_id != descriptor.profile_id {
        issues.push(CrossProcessTransportIssue::ProfileIdentityMismatch);
    }
    if listener.identity.protocol_id != descriptor.protocol_id || listener.identity.alpn != descriptor.alpn {
        issues.push(CrossProcessTransportIssue::ProtocolIdentityMismatch);
    }
    if listener.identity.extension_id != descriptor.extension_id {
        issues.push(CrossProcessTransportIssue::ExtensionIdentityMismatch);
    }
    if listener.identity.service_id != descriptor.service_id {
        issues.push(CrossProcessTransportIssue::ServiceIdentityMismatch);
    }
    if listener.identity.generation != descriptor.generation {
        issues.push(CrossProcessTransportIssue::GenerationMismatch);
    }
}

fn validate_expected_binding(
    descriptor: &CrossProcessEndpointDescriptor,
    expected: &ExpectedEndpointBinding,
    issues: &mut Vec<CrossProcessTransportIssue>,
) {
    if descriptor.profile_id != expected.profile_id || descriptor.profile_ref != expected.profile_ref {
        issues.push(CrossProcessTransportIssue::ProfileIdentityMismatch);
    }
    if descriptor.protocol_id != expected.protocol_id || descriptor.protocol_version != expected.protocol_version {
        issues.push(CrossProcessTransportIssue::ProtocolIdentityMismatch);
    }
    if descriptor.alpn != expected.alpn {
        issues.push(CrossProcessTransportIssue::AlpnMismatch);
    }
    if descriptor.extension_id != expected.extension_id {
        issues.push(CrossProcessTransportIssue::ExtensionIdentityMismatch);
    }
    if descriptor.service_id != expected.service_id {
        issues.push(CrossProcessTransportIssue::ServiceIdentityMismatch);
    }
    if descriptor.generation != expected.generation {
        issues.push(CrossProcessTransportIssue::GenerationMismatch);
    }
    if descriptor.public_endpoint_identity != expected.public_endpoint_identity {
        issues.push(CrossProcessTransportIssue::EndpointIdentityMismatch);
    }
    if descriptor.listener_identity_ref != expected.listener_identity_ref {
        issues.push(CrossProcessTransportIssue::ListenerIdentityMismatch);
    }
    if descriptor.expected_peer_context_ref != expected.peer_context_ref {
        issues.push(CrossProcessTransportIssue::PeerContextMismatch);
    }
}
