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

// r[impl molten.modularity.fabric_boundary.compatibility]
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

// r[impl molten.modularity.fabric_boundary.compatibility]
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
