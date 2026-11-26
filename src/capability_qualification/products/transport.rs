//! Cross-process Iroh transport between this producer (client) and a distinct
//! listener peer process on the same host (loopback, relay disabled). The
//! listener hands off its canonical endpoint descriptor through the run
//! directory; the producer independently derives the expected listener identity
//! from the fixed listener key before importing it.

use preserves::ValueImpl as _;

use crate::fabric_transport::CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA;
use crate::fabric_transport::ClientInput;
use crate::fabric_transport::DeliveryOutcome;
use crate::fabric_transport::Endpoint;
use crate::fabric_transport::EndpointAdmissionState;
use crate::fabric_transport::EndpointCapability;
use crate::fabric_transport::EndpointDisclosurePolicy;
use crate::fabric_transport::EndpointLocatorClass;
use crate::fabric_transport::EndpointValidityCohort;
use crate::fabric_transport::ExpectedEndpointBinding;
use crate::fabric_transport::FrameEvidence;
use crate::fabric_transport::FramingProfile;
use crate::fabric_transport::IROH_SECRET_KEY_BYTES;
use crate::fabric_transport::Listener;
use crate::fabric_transport::ListenerCleanupPolicy;
use crate::fabric_transport::ListenerDrainReason;
use crate::fabric_transport::ListenerInput;
use crate::fabric_transport::ListenerPoll;
use crate::fabric_transport::ListenerTerminalClass;
use crate::fabric_transport::Profile;
use crate::fabric_transport::ProtocolDescriptor;
use crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS;
use crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA;
use crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA;
use crate::fabric_transport::TransportAdapterKind;
use crate::fabric_transport::TransportCapability;
use crate::fabric_transport::TransportLimits;
use crate::fabric_transport::TransportProfile;

const LISTENER_ROLE: &str = "transport-listener";
const WITHHOLDING_LISTENER_ROLE: &str = "transport-listener-withhold-ack";
const ENDPOINT_FILE: &str = "endpoint.preserves";
const LISTENER_REPORT_FILE: &str = "listener.preserves";
const LISTENER_SECRET: [u8; IROH_SECRET_KEY_BYTES] = [0x51; IROH_SECRET_KEY_BYTES];
const CLIENT_SECRET: [u8; IROH_SECRET_KEY_BYTES] = [0x53; IROH_SECRET_KEY_BYTES];
/// The admitted-peer id of the producer's fixed client endpoint.
const CLIENT_PEER_ID: &str = "qualification-client";
const OTHER_SECRET: [u8; IROH_SECRET_KEY_BYTES] = [0x55; IROH_SECRET_KEY_BYTES];
const PAYLOAD: &[u8] = b"aspen capability qualification cross-process frame";
const GENERATION: u64 = 1;
const PROFILE_LIMIT: u64 = 8;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_LIMIT: u64 = 16_384;
const INFLIGHT_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW: u64 = 64;
const LENGTH_PREFIX_BYTES: u64 = 8;
const VALID_FROM_TICK: u64 = 1;
const VALID_UNTIL_TICK: u64 = 100;
const OBSERVED_TICK: u64 = 10;
const EXCHANGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const PEER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// After a withheld acknowledgement the listener keeps accepting this long; a
/// retrying client would open a second session inside it.
const RETRY_WINDOW: std::time::Duration = std::time::Duration::from_secs(3);

pub(super) fn run(
    scenario: super::super::Scenario,
    launcher: &super::Launcher,
) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::TransportEndpointImport => endpoint_import(launcher),
        super::super::Scenario::TransportEndpointTamperDenied => endpoint_tamper_denied(launcher),
        super::super::Scenario::TransportSessionAcknowledged => session_acknowledged(launcher),
        super::super::Scenario::TransportSessionLostAcknowledgement => session_lost_acknowledgement(launcher),
        _ => Err(super::super::invalid("scenario is not a cross-process transport scenario")),
    }
}

pub(super) fn peer(role: &str, run_directory: &std::path::Path) -> Option<crate::error::Result<()>> {
    let withhold = match role {
        LISTENER_ROLE => false,
        WITHHOLDING_LISTENER_ROLE => true,
        _ => return None,
    };
    Some(runtime().and_then(|runtime| runtime.block_on(serve(run_directory, withhold))))
}

// r[impl molten.fabric_transport.cross_process_endpoint]
fn endpoint_import(launcher: &super::Launcher) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut peer = PeerListener::start(launcher, LISTENER_ROLE)?;
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    evidence.check("handoff-strictly-decoded", endpoint.value == peer.handoff);
    evidence.check(
        "listener-identity-matches-independent-key",
        endpoint.descriptor.public_endpoint_identity == public_identity(LISTENER_SECRET),
    );
    let status = crate::fabric_transport::endpoint_status(&endpoint.descriptor)?;
    let status_text = crate::preserves_rail::to_text(&status.value)?;
    evidence.check(
        "default-status-redacts-raw-locators",
        endpoint.descriptor.locators.iter().all(|locator| !status_text.contains(&locator.value)),
    );
    evidence.check(
        "handoff-excludes-private-key",
        !crate::preserves_rail::to_text(&endpoint.value)?.contains(&hex::encode(LISTENER_SECRET)),
    );
    evidence.runtime(endpoint_projection(&endpoint));
    let client = exchange(client_input(endpoint.clone(), expected(public_identity(LISTENER_SECRET))?)?)?;
    evidence.check("imported-endpoint-dialed", client.delivery == DeliveryOutcome::Delivered);
    evidence.check(
        "remote-identity-is-imported-listener",
        client.remote_transport_identity_ref
            == crate::preserves_rail::content_ref_from_bytes(listener_id(LISTENER_SECRET).as_bytes()),
    );
    evidence.runtime(frame_projection(&client));
    let report = peer.finish()?;
    evidence.check("listener-observed-one-delivered-session", report.delivered && !report.second_session);
    evidence.runtime(report.value);
    Ok(evidence)
}

// r[impl molten.fabric_transport.cross_process_endpoint]
fn endpoint_tamper_denied(launcher: &super::Launcher) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut peer = PeerListener::start(launcher, LISTENER_ROLE)?;
    let fields = peer
        .handoff
        .collect_simple_record("fabric-transport-endpoint-descriptor-v1", Some(4))
        .ok_or_else(|| super::super::invalid("listener handoff is not an endpoint descriptor"))?;
    let tampered = crate::preserves_rail::record("fabric-transport-endpoint-descriptor-v1", vec![
        crate::preserves_rail::string(CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA),
        crate::preserves_rail::string(reference("qualification substituted descriptor")),
        crate::preserves_rail::value_to_iovalue(&fields[2]),
        crate::preserves_rail::value_to_iovalue(&fields[3]),
    ]);
    let rejected = crate::fabric_transport::parse_endpoint(&tampered);
    evidence.check("tampered-descriptor-ref-denied", rejected.is_err());
    if let Err(error) = &rejected {
        evidence.runtime(super::error_class(error));
    }
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    let impostor = exchange(client_input(endpoint.clone(), expected(public_identity(OTHER_SECRET))?)?);
    evidence.check("unexpected-listener-identity-denied-before-dial", impostor.is_err());
    if let Err(error) = &impostor {
        evidence.runtime(super::error_class(error));
    }
    let mut stale = client_input(endpoint.clone(), expected(public_identity(LISTENER_SECRET))?)?;
    stale.expected.generation = GENERATION + 1;
    evidence.check("stale-generation-import-denied-before-dial", exchange(stale).is_err());
    // Only the admitted import may reach the listener: it accepts exactly one
    // session, so any denied attempt that had dialed would have consumed it.
    let admitted = exchange(client_input(endpoint.clone(), expected(public_identity(LISTENER_SECRET))?)?)?;
    evidence.check("admitted-import-still-delivers", admitted.delivery == DeliveryOutcome::Delivered);
    let report = peer.finish()?;
    evidence.check("denied-imports-never-reached-listener", report.delivered && !report.second_session);
    evidence.runtime(report.value);
    Ok(evidence)
}

// r[impl molten.fabric_transport.cross_process_session]
fn session_acknowledged(launcher: &super::Launcher) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut peer = PeerListener::start(launcher, LISTENER_ROLE)?;
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    let client = exchange(client_input(endpoint.clone(), expected(public_identity(LISTENER_SECRET))?)?)?;
    evidence.check("client-frame-delivered", client.delivery == DeliveryOutcome::Delivered);
    evidence.check("acknowledgement-binds-payload", client.acknowledgement_ref == client.payload_ref);
    evidence.check(
        "payload-ref-is-canonical-frame-ref",
        client.payload_ref == crate::fabric_transport::frame_ref(&request_ref(), PAYLOAD),
    );
    evidence.check("no-automatic-retry", client.automatic_retry_count == 0);
    evidence.runtime(frame_projection(&client));
    let report = peer.finish()?;
    evidence.check("listener-frame-delivered", report.delivered);
    evidence.check(
        "listener-payload-matches-client",
        report.payload_ref.as_deref() == Some(client.payload_ref.as_str()),
    );
    evidence.check("listener-cleanup-clean", report.clean);
    evidence.runtime(report.value);
    Ok(evidence)
}

// r[impl molten.fabric_transport.cross_process_session]
fn session_lost_acknowledgement(launcher: &super::Launcher) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut peer = PeerListener::start(launcher, WITHHOLDING_LISTENER_ROLE)?;
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    let client = exchange(client_input(endpoint.clone(), expected(public_identity(LISTENER_SECRET))?)?);
    evidence.check("unacknowledged-frame-not-reported-delivered", client.is_err());
    if let Err(error) = &client {
        evidence.runtime(super::error_class(error));
    }
    let report = peer.finish()?;
    evidence.check("listener-withheld-acknowledgement-after-reading-frame", report.withheld);
    evidence.check(
        "withheld-payload-is-the-submitted-frame",
        report.payload_ref.as_deref() == Some(crate::fabric_transport::frame_ref(&request_ref(), PAYLOAD).as_str()),
    );
    evidence.check("listener-did-not-report-delivery", !report.delivered);
    evidence.check("client-did-not-retry-automatically", !report.second_session);
    evidence.check("listener-cleanup-clean", report.clean);
    evidence.runtime(report.value);
    Ok(evidence)
}

/// A started listener peer and its canonical endpoint handoff.
struct PeerListener {
    peer: super::peer::Peer,
    directory: tempfile::TempDir,
    handoff: preserves::IOValue,
}

struct ListenerReport {
    withheld: bool,
    delivered: bool,
    second_session: bool,
    clean: bool,
    payload_ref: Option<String>,
    value: preserves::IOValue,
}

impl PeerListener {
    fn start(launcher: &super::Launcher, role: &str) -> crate::error::Result<Self> {
        let directory = super::scratch("transport")?;
        let peer = launcher.spawn(role, directory.path())?;
        let handoff = super::peer::wait_value(&directory.path().join(ENDPOINT_FILE), PEER_TIMEOUT)?;
        Ok(Self {
            peer,
            directory,
            handoff,
        })
    }

    fn finish(&mut self) -> crate::error::Result<ListenerReport> {
        let value = super::peer::wait_value(&self.directory.path().join(LISTENER_REPORT_FILE), PEER_TIMEOUT)?;
        self.peer.wait_success(PEER_TIMEOUT)?;
        let fields = value
            .collect_simple_record("qualification-transport-listener-v1", Some(5))
            .ok_or_else(|| super::super::invalid("malformed listener peer report"))?;
        let outcome = super::super::text(&fields[0])?;
        let payload_ref = fields[1].as_string().map(|text| text.to_string()).filter(|text| !text.is_empty());
        Ok(ListenerReport {
            withheld: outcome == "acknowledgement-withheld",
            delivered: outcome == "delivered",
            second_session: fields[3].as_boolean() != Some(false),
            clean: super::super::text(&fields[4])? == format!("{:?}", ListenerTerminalClass::Clean),
            payload_ref,
            value,
        })
    }
}

/// Listener peer: bind, hand off the endpoint, serve one session, report.
async fn serve(run_directory: &std::path::Path, withhold: bool) -> crate::error::Result<()> {
    let mut listener = Listener::bind(ListenerInput {
        profile: profile()?,
        protocol: protocol(),
        capability: EndpointCapability::from_secret_bytes(
            LISTENER_SECRET,
            reference("qualification listener capability"),
        )?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        listener_identity_ref: reference("qualification listener identity"),
        expected_peer_context_ref: reference("qualification expected peer context"),
        locator_cohort_ref: reference("qualification locator cohort"),
        disclosure: disclosure(),
        validity: validity(),
        admission: EndpointAdmissionState::fully_active(),
        observed_tick: OBSERVED_TICK,
        // Exactly the producer's fixed client endpoint is admitted, under this
        // listener's own peer context; any other authenticated endpoint is refused.
        admitted_peers: std::collections::BTreeMap::from([(
            CLIENT_PEER_ID.to_string(),
            crate::fabric_transport::AdmittedPeer {
                public_endpoint_identity: public_identity(CLIENT_SECRET),
                peer_context_ref: reference("qualification expected peer context"),
            },
        )]),
    })
    .await?;
    super::peer::write_value(&run_directory.join(ENDPOINT_FILE), &listener.handoff().value)?;
    let (outcome, frame, payload_ref) = if withhold {
        // The frame is read, then the acknowledgement is withheld: the
        // submission outcome is unknown to the client. Only a frame actually
        // read inside the derivation counts as a withheld acknowledgement; an
        // accept timeout or transport error is a distinct outcome.
        let mut withheld_payload_ref = None;
        let refused = listener
            .accept_one_derived_frame(&session_ref(), EXCHANGE_TIMEOUT, |payload| {
                withheld_payload_ref = Some(crate::fabric_transport::frame_ref(&request_ref(), payload));
                Err(super::super::invalid("qualification listener withholds the acknowledgement"))
            })
            .await;
        match (refused, withheld_payload_ref) {
            (Err(_), Some(payload_ref)) => ("acknowledgement-withheld", None, payload_ref),
            (Err(_), None) => ("no-frame-received", None, String::new()),
            (Ok(received), _) => {
                let payload_ref = received.evidence.payload_ref.clone();
                ("delivered", Some(received.evidence), payload_ref)
            }
        }
    } else {
        let received = listener.accept_one_frame(&session_ref(), &request_ref(), EXCHANGE_TIMEOUT).await?;
        let payload_ref = received.evidence.payload_ref.clone();
        ("delivered", Some(received.evidence), payload_ref)
    };
    // Any connection inside the window, admitted or refused, would be a retry.
    let second = listener
        .poll_one_derived_frame(&session_ref(), RETRY_WINDOW, |_peer_id, _payload| Ok(request_ref()))
        .await?;
    let cleanup = listener.drain_and_close(ListenerDrainReason::OperatorRequest).await?;
    super::peer::write_value(
        &run_directory.join(LISTENER_REPORT_FILE),
        &crate::preserves_rail::record("qualification-transport-listener-v1", vec![
            crate::preserves_rail::string(outcome),
            crate::preserves_rail::string(&payload_ref),
            frame.as_ref().map_or_else(|| crate::preserves_rail::sequence(Vec::new()), frame_projection),
            crate::preserves_rail::bool_value(!matches!(second, ListenerPoll::Idle)),
            crate::preserves_rail::string(format!("{:?}", cleanup.terminal_class)),
        ]),
    )
}

fn exchange(input: ClientInput) -> crate::error::Result<FrameEvidence> {
    runtime()?.block_on(crate::fabric_transport::exchange_frame(input, PAYLOAD, EXCHANGE_TIMEOUT))
}

fn runtime() -> crate::error::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(crate::error::Failure::from)
}

fn client_input(endpoint: Endpoint, expected: ExpectedEndpointBinding) -> crate::error::Result<ClientInput> {
    Ok(ClientInput {
        profile: profile()?,
        protocol: protocol(),
        capability: EndpointCapability::from_secret_bytes(CLIENT_SECRET, reference("qualification client capability"))?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        endpoint,
        expected,
        admission: EndpointAdmissionState::fully_active(),
        session_ref: session_ref(),
        request_ref: request_ref(),
    })
}

/// The binding the client independently expects. The listener identity comes
/// from the fixed listener key, never from the handed-off descriptor.
fn expected(public_endpoint_identity: String) -> crate::error::Result<ExpectedEndpointBinding> {
    let profile = profile()?;
    let protocol = protocol();
    Ok(ExpectedEndpointBinding {
        profile_id: profile.profile.profile_id.clone(),
        profile_ref: profile.profile.profile_ref.clone(),
        protocol_id: protocol.protocol_id.clone(),
        protocol_version: protocol.version.clone(),
        alpn: protocol.alpn.clone(),
        extension_id: protocol.extension_id.clone(),
        service_id: protocol.service_id.clone(),
        generation: GENERATION,
        public_endpoint_identity,
        listener_identity_ref: reference("qualification listener identity"),
        peer_context_ref: reference("qualification expected peer context"),
        observed_tick: OBSERVED_TICK,
    })
}

fn listener_id(secret: [u8; IROH_SECRET_KEY_BYTES]) -> String {
    iroh::SecretKey::from_bytes(&secret).public().to_string()
}

fn public_identity(secret: [u8; IROH_SECRET_KEY_BYTES]) -> String {
    format!("iroh:{}", listener_id(secret))
}

/// Deterministic projection of a canonical endpoint descriptor: everything
/// except the OS-assigned loopback locator value and the refs that bind it.
fn endpoint_projection(endpoint: &Endpoint) -> preserves::IOValue {
    let descriptor = &endpoint.descriptor;
    crate::preserves_rail::record("qualification-transport-endpoint-v1", vec![
        crate::preserves_rail::string(&descriptor.schema),
        crate::preserves_rail::string(&descriptor.profile_id),
        crate::preserves_rail::string(&descriptor.profile_ref),
        crate::preserves_rail::string(&descriptor.protocol_id),
        crate::preserves_rail::string(&descriptor.protocol_version),
        crate::preserves_rail::string(&descriptor.alpn),
        crate::preserves_rail::string(&descriptor.service_id),
        crate::preserves_rail::u64_value(descriptor.generation),
        crate::preserves_rail::string(&descriptor.public_endpoint_identity),
        crate::preserves_rail::string(&descriptor.listener_identity_ref),
        crate::preserves_rail::string(&descriptor.locator_cohort_ref),
        crate::preserves_rail::sequence(
            descriptor
                .locators
                .iter()
                .map(|locator| crate::preserves_rail::string(locator.class.as_str()))
                .collect(),
        ),
        crate::preserves_rail::string(&descriptor.framing_profile_ref),
        crate::preserves_rail::string(&descriptor.validity.cohort_ref),
    ])
}

/// Deterministic projection of shell frame evidence (the descriptor/cleanup
/// refs bind the OS-assigned port and are excluded).
fn frame_projection(frame: &FrameEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-transport-frame-v1", vec![
        crate::preserves_rail::string(format!("{:?}", frame.role)),
        crate::preserves_rail::string(&frame.session_ref),
        crate::preserves_rail::string(&frame.request_ref),
        crate::preserves_rail::string(&frame.payload_ref),
        crate::preserves_rail::string(&frame.acknowledgement_ref),
        crate::preserves_rail::string(&frame.remote_transport_identity_ref),
        crate::preserves_rail::u64_value(frame.payload_bytes),
        crate::preserves_rail::string(format!("{:?}", frame.delivery)),
        crate::preserves_rail::string(format!("{:?}", frame.retry)),
        crate::preserves_rail::u64_value(frame.automatic_retry_count),
        crate::preserves_rail::string(format!("{:?}", frame.terminal_class)),
    ])
}

fn profile() -> crate::error::Result<Profile> {
    crate::fabric_transport::profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-cross-process-v1".to_string(),
        profile_ref: reference("qualification transport profile declaration"),
        adapter_kind: TransportAdapterKind::IrohLive,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        limits: TransportLimits {
            max_listeners: PROFILE_LIMIT,
            max_sessions: PROFILE_LIMIT,
            max_streams_per_session: PROFILE_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: PROFILE_LIMIT,
            max_queued_bytes: QUEUE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

fn protocol() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "qualification-echo".to_string(),
        version: "v1".to_string(),
        alpn: "aspen/qualification-echo/1".to_string(),
        extension_id: "qualification-transport-extension".to_string(),
        service_id: "qualification-transport-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        framing: FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: reference("qualification transport framing"),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW,
        },
        registration_authority_ref: reference("qualification transport registration authority"),
        profile_ref: reference("qualification transport profile declaration"),
    }
}

fn validity() -> EndpointValidityCohort {
    EndpointValidityCohort {
        cohort_ref: reference("qualification endpoint validity cohort"),
        not_before_tick: VALID_FROM_TICK,
        expires_at_tick: VALID_UNTIL_TICK,
    }
}

fn disclosure() -> EndpointDisclosurePolicy {
    EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

fn session_ref() -> String {
    reference("qualification transport session")
}

fn request_ref() -> String {
    reference("qualification transport request")
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
