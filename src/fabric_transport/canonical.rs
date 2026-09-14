use super::*;
use crate::system_extension::SystemExtensionExecutor;

pub const FABRIC_TRANSPORT_PORT_ID: &str = "molten.fabric.transport.session";
pub const FABRIC_TRANSPORT_PORT_VERSION: &str = "v1";

const TRANSPORT_PROFILE_RECORD: &str = "fabric-transport-profile-v1";
const TRANSPORT_TRANSITION_RECORD: &str = "fabric-transport-transition-v1";
const TRANSPORT_EVENT_RECORD: &str = "fabric-transport-event-v1";
const TRANSPORT_STATUS_RECORD: &str = "fabric-transport-status-v1";
const TRANSPORT_INPUT_SCHEMA: &str = "molten.fabric.transport.command.v1";
const TRANSPORT_OUTPUT_SCHEMA: &str = "molten.fabric.transport.event.v1";
const MAX_CANONICAL_TRANSPORT_EVENTS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTransportProfile {
    pub profile: TransportProfile,
    pub profile_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTransportEvent {
    pub event_ref: String,
    pub event: TransportEvent,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTransportTransition {
    pub transition_ref: String,
    pub profile_ref: String,
    pub decision: TransportTransitionDecision,
    pub events: Vec<CanonicalTransportEvent>,
    pub state: TransportState,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportStatusReadback {
    pub profile_ref: String,
    pub adapter_kind: TransportAdapterKind,
    pub active_protocols: u64,
    pub draining_protocols: u64,
    pub active_sessions: u64,
    pub active_streams: u64,
    pub inflight_bytes: u64,
    pub failures: u64,
    pub cancellations: u64,
    pub latest_evidence_ref: Option<String>,
    pub non_claims: Vec<TransportNonClaim>,
    pub status_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_transport.port_contract]
// r[impl molten.fabric_transport.live_sim_parity]
// r[impl molten.fabric_transport.failure_semantics]
pub fn canonical_transport_profile(profile: &TransportProfile) -> crate::error::Result<CanonicalTransportProfile> {
    validate_transport_profile(profile).map_err(|issues| validation_error("transport profile", &issues))?;
    let value = transport_profile_value(profile);
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalTransportProfile {
        profile: profile.clone(),
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_transport.port_contract]
pub fn fabric_transport_port_descriptor(profile: &CanonicalTransportProfile) -> crate::fabric::FabricPortDescriptor {
    let (determinism, replay) = match profile.profile.adapter_kind {
        TransportAdapterKind::IrohLive => {
            (crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)
        }
        TransportAdapterKind::DeterministicSimulation => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
    };
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: FABRIC_TRANSPORT_PORT_ID.to_string(),
        version: FABRIC_TRANSPORT_PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Transport,
        operation_classes: vec![
            "register-protocol".to_string(),
            "transfer-protocol".to_string(),
            "dial".to_string(),
            "accept".to_string(),
            "open-stream".to_string(),
            "send-frame".to_string(),
            "receive-frame".to_string(),
            "send-datagram".to_string(),
            "grant-credit".to_string(),
            "cancel".to_string(),
            "close".to_string(),
            "fail".to_string(),
            "drain".to_string(),
            "cleanup".to_string(),
        ],
        input_schema_refs: vec![TRANSPORT_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![TRANSPORT_OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![
            crate::fabric::FabricAuthority::Transport,
            crate::fabric::FabricAuthority::ProtocolOwnership,
        ],
        resource_requirements: vec![
            crate::fabric::FabricResource::NetworkBytes,
            crate::fabric::FabricResource::Concurrency,
            crate::fabric::FabricResource::QueueDepth,
            crate::fabric::FabricResource::LogicalTime,
        ],
        determinism,
        replay,
        implementation_profile: profile.profile.profile_id.clone(),
        conformance_refs: vec![profile.profile_ref.clone()],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

// r[impl molten.fabric_transport.evidence]
// r[impl molten.fabric_transport.failure_semantics]
pub fn canonical_transport_transition(
    profile: &CanonicalTransportProfile,
    transition: TransportTransition,
) -> crate::error::Result<CanonicalTransportTransition> {
    validate_transport_profile(&profile.profile).map_err(|issues| validation_error("transport profile", &issues))?;
    if transition.events.is_empty() || transition.events.len() > MAX_CANONICAL_TRANSPORT_EVENTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "transport transition event count {} outside canonical bound",
            transition.events.len()
        )));
    }
    for registered in transition.next.protocols.values() {
        validate_protocol_descriptor(&profile.profile, &registered.descriptor)
            .map_err(|issues| validation_error("registered transport protocol", &issues))?;
    }
    let mut events = Vec::with_capacity(transition.events.len());
    for event in transition.events {
        events.push(canonical_event(event)?);
    }
    let event_refs = events.iter().map(|event| event.event_ref.as_str());
    let value = crate::preserves_rail::record(TRANSPORT_TRANSITION_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_EVENT_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("decision", crate::preserves_rail::string(transition.decision.as_str())),
        field("event-refs", strings_value(event_refs)),
        field("automatic-retry-count", crate::preserves_rail::u64_value(transition.automatic_retry_count)),
        field("active-protocols", count_value(transition.next.protocols.len())?),
        field("known-sessions", count_value(transition.next.sessions.len())?),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "pure-transition-admitted",
            "adapter-handles-excluded",
            "generation-correlated",
            "bounds-enforced-before-callback",
            "automatic-retries-disabled",
        ]),
    ]);
    let transition_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalTransportTransition {
        transition_ref,
        profile_ref: profile.profile_ref.clone(),
        decision: transition.decision,
        events,
        state: transition.next,
        value,
    })
}

// r[impl molten.fabric_transport.evidence]
pub fn transport_status_readback(
    profile: &CanonicalTransportProfile,
    state: &TransportState,
    latest_evidence_ref: Option<&str>,
) -> crate::error::Result<TransportStatusReadback> {
    validate_transport_profile(&profile.profile).map_err(|issues| validation_error("transport profile", &issues))?;
    if let Some(reference) = latest_evidence_ref {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let active_protocols = count(
        state
            .protocols
            .values()
            .filter(|protocol| protocol.phase == ProtocolRegistrationPhase::Active)
            .count(),
    )?;
    let draining_protocols = count(
        state
            .protocols
            .values()
            .filter(|protocol| protocol.phase == ProtocolRegistrationPhase::Draining)
            .count(),
    )?;
    let active_sessions = count(state.sessions.values().filter(|session| !session.phase.is_terminal()).count())?;
    let active_streams = count(
        state
            .sessions
            .values()
            .flat_map(|session| session.streams.values())
            .filter(|stream| !stream.phase.is_terminal())
            .count(),
    )?;
    let inflight_bytes = state.sessions.values().try_fold(0_u64, |total, session| {
        total
            .checked_add(session.inflight_bytes)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("transport status inflight-byte overflow"))
    })?;
    let value = crate::preserves_rail::record(TRANSPORT_STATUS_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_STATUS_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("adapter-kind", crate::preserves_rail::string(profile.profile.adapter_kind.as_str())),
        field("active-protocols", crate::preserves_rail::u64_value(active_protocols)),
        field("draining-protocols", crate::preserves_rail::u64_value(draining_protocols)),
        field("active-sessions", crate::preserves_rail::u64_value(active_sessions)),
        field("active-streams", crate::preserves_rail::u64_value(active_streams)),
        field("inflight-bytes", crate::preserves_rail::u64_value(inflight_bytes)),
        field("failures", crate::preserves_rail::u64_value(state.counters.failures)),
        field("cancellations", crate::preserves_rail::u64_value(state.counters.cancellations)),
        field("latest-evidence-ref", optional_string(latest_evidence_ref)),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "bounded-aggregate-readback",
            "payloads-excluded",
            "secrets-excluded",
            "transport-identity-is-not-authority",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TransportStatusReadback {
        profile_ref: profile.profile_ref.clone(),
        adapter_kind: profile.profile.adapter_kind,
        active_protocols,
        draining_protocols,
        active_sessions,
        active_streams,
        inflight_bytes,
        failures: state.counters.failures,
        cancellations: state.counters.cancellations,
        latest_evidence_ref: latest_evidence_ref.map(str::to_string),
        non_claims: profile.profile.non_claims.clone(),
        status_ref,
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTransportContext {
    service_id: String,
    generation: u64,
    profile_id: String,
    max_frame_bytes: u64,
}

impl ExtensionTransportContext {
    // r[impl molten.fabric_transport.protocol_registration]
    // r[impl molten.fabric_transport.session_streams]
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &crate::system_extension::SystemExtensionHost<E>,
        profile: &CanonicalTransportProfile,
    ) -> crate::error::Result<Self> {
        let key = crate::fabric::FabricPortKey {
            port_id: FABRIC_TRANSPORT_PORT_ID.to_string(),
            version: FABRIC_TRANSPORT_PORT_VERSION.to_string(),
        };
        let binding = host.manifest().binding_for(&key).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("system extension has no admitted transport port binding")
        })?;
        if binding.binding.implementation_profile != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension transport profile substitution denied",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            max_frame_bytes: profile.profile.limits.max_frame_bytes,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(service_id: &str, generation: u64, profile: &CanonicalTransportProfile) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            max_frame_bytes: profile.profile.limits.max_frame_bytes,
        }
    }

    pub fn admit_command(
        &self,
        profile: &CanonicalTransportProfile,
        command: &TransportCommand,
        accounted_bytes: u64,
    ) -> crate::error::Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness("transport profile substitution denied"));
        }
        if command.generation() != self.generation {
            return Err(crate::error::MoltenError::invalid_harness(
                "transport command uses a stale service generation",
            ));
        }
        let command_service = command_service_id(command);
        if command_service != self.service_id {
            return Err(crate::error::MoltenError::invalid_harness("transport command service identity mismatch"));
        }
        if accounted_bytes > self.max_frame_bytes {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "transport command bytes {accounted_bytes} exceed {}",
                self.max_frame_bytes
            )));
        }
        Ok(())
    }
}

fn command_service_id(command: &TransportCommand) -> &str {
    match command {
        TransportCommand::Register { descriptor, .. } | TransportCommand::TransferOwnership { descriptor, .. } => {
            &descriptor.service_id
        }
        TransportCommand::BeginDrain { service_id, .. } | TransportCommand::CleanupListener { service_id, .. } => {
            service_id
        }
        TransportCommand::OpenSession { session_id, .. }
        | TransportCommand::OpenStream { session_id, .. }
        | TransportCommand::SendFrame { session_id, .. }
        | TransportCommand::ReceiveFrame { session_id, .. }
        | TransportCommand::AcknowledgeFrame { session_id, .. }
        | TransportCommand::SendDatagram { session_id, .. }
        | TransportCommand::CompleteDatagram { session_id, .. }
        | TransportCommand::GrantCredit { session_id, .. }
        | TransportCommand::HalfCloseStream { session_id, .. }
        | TransportCommand::CloseStream { session_id, .. }
        | TransportCommand::CloseSession { session_id, .. }
        | TransportCommand::FailSession { session_id, .. } => &session_id.service_id,
        TransportCommand::Cancel { target, .. } => match target {
            CancelTarget::Session(session_id) | CancelTarget::Stream { session_id, .. } => &session_id.service_id,
        },
    }
}

fn canonical_event(event: TransportEvent) -> crate::error::Result<CanonicalTransportEvent> {
    let value = crate::preserves_rail::record(TRANSPORT_EVENT_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_EVENT_SCHEMA),
        field("kind", crate::preserves_rail::string(event.kind.as_str())),
        field("operation-id", crate::preserves_rail::string(&event.operation_id)),
        field("protocol-id", crate::preserves_rail::string(&event.protocol_id)),
        field("session-id", optional_string(event.session_id.as_deref())),
        field("stream-id", optional_string(event.stream_id.as_deref())),
        field("generation", crate::preserves_rail::u64_value(event.generation)),
        field("sequence", optional_u64(event.sequence)),
        field("payload-ref", optional_string(event.payload_ref.as_deref())),
        field("payload-bytes", crate::preserves_rail::u64_value(event.payload_bytes)),
        field(
            "transport-identity-ref",
            optional_string(event.peer.as_ref().map(|peer| peer.transport_identity_ref.as_str())),
        ),
        field(
            "membership-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.membership_ref.as_deref())),
        ),
        field(
            "application-principal-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.application_principal_ref.as_deref())),
        ),
        field(
            "trust-decision-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.trust_decision_ref.as_deref())),
        ),
        field(
            "capability-authority-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.capability_authority_ref.as_deref())),
        ),
        field(
            "bootstrap-policy-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.bootstrap_policy_ref.as_deref())),
        ),
        field("failure", optional_string(event.failure.map(|failure| failure.as_str()))),
        field("delivery", crate::preserves_rail::string(event.delivery.as_str())),
        field("retry", crate::preserves_rail::string(event.retry.as_str())),
        field("terminal", crate::preserves_rail::bool_value(event.terminal)),
        checks(&[
            "opaque-generation-scoped-handles",
            "identity-classes-separated",
            "delivery-semantics-explicit",
            "payload-bytes-excluded",
        ]),
    ]);
    let event_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalTransportEvent {
        event_ref,
        event,
        value,
    })
}

fn transport_profile_value(profile: &TransportProfile) -> preserves::IOValue {
    crate::preserves_rail::record(TRANSPORT_PROFILE_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&profile.profile_id)),
        field("declared-profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("adapter-kind", crate::preserves_rail::string(profile.adapter_kind.as_str())),
        field("capabilities", strings_value(profile.capabilities.iter().map(|capability| capability.as_str()))),
        field("max-listeners", crate::preserves_rail::u64_value(profile.limits.max_listeners)),
        field("max-sessions", crate::preserves_rail::u64_value(profile.limits.max_sessions)),
        field("max-streams-per-session", crate::preserves_rail::u64_value(profile.limits.max_streams_per_session)),
        field("max-frame-bytes", crate::preserves_rail::u64_value(profile.limits.max_frame_bytes)),
        field("max-datagram-bytes", crate::preserves_rail::u64_value(profile.limits.max_datagram_bytes)),
        field("max-queued-events", crate::preserves_rail::u64_value(profile.limits.max_queued_events)),
        field("max-queued-bytes", crate::preserves_rail::u64_value(profile.limits.max_queued_bytes)),
        field("max-inflight-bytes", crate::preserves_rail::u64_value(profile.limits.max_inflight_bytes)),
        field(
            "operation-deadline-ticks",
            crate::preserves_rail::u64_value(profile.limits.operation_deadline_ticks),
        ),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-adapter-neutral-profile",
            "framing-and-resource-bounds-explicit",
            "capabilities-versioned",
            "delivery-non-claims-complete",
        ]),
    ])
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn count_value(value: usize) -> crate::error::Result<preserves::IOValue> {
    count(value).map(crate::preserves_rail::u64_value)
}

fn count(value: usize) -> crate::error::Result<u64> {
    u64::try_from(value).map_err(|_| crate::error::MoltenError::invalid_harness("transport collection count overflow"))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}
