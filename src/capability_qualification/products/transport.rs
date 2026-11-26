//! Cross-process Iroh transport between this producer (client) and a distinct
//! listener peer process on the same host (loopback, relay disabled). The
//! listener hands off its canonical endpoint descriptor through the run
//! directory; the producer independently derives the expected listener identity
//! from the fixed listener key before importing it.

mod fixture;
mod listener;

use preserves::ValueImpl as _;

const LISTENER_ROLE: &str = "transport-listener";
const WITHHOLDING_LISTENER_ROLE: &str = "transport-listener-withhold-ack";
const ENDPOINT_FILE: &str = "endpoint.preserves";
const LISTENER_REPORT_FILE: &str = "listener.preserves";
const LISTENER_SECRET: [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES] =
    [0x51; crate::fabric_transport::IROH_SECRET_KEY_BYTES];
const CLIENT_SECRET: [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES] =
    [0x53; crate::fabric_transport::IROH_SECRET_KEY_BYTES];
/// The admitted-peer id of the producer's fixed client endpoint.
const CLIENT_PEER_ID: &str = "qualification-client";
const OTHER_SECRET: [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES] =
    [0x55; crate::fabric_transport::IROH_SECRET_KEY_BYTES];
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
        super::super::Scenario::GrantedSend
        | super::super::Scenario::DeniedSend
        | super::super::Scenario::DurableRestartRecovery
        | super::super::Scenario::DurableCorruptSnapshotDenied
        | super::super::Scenario::ExecutionUnknownAfterStart
        | super::super::Scenario::ExecutionPreStartDefinite
        | super::super::Scenario::NativeWorkUnknownAfterEffect
        | super::super::Scenario::NativeUncertainWorkNotReadmitted
        | super::super::Scenario::WorldReplayAdmitted
        | super::super::Scenario::WorldReplayAuthorityDenied
        | super::super::Scenario::WorldRestoreCurrentAdmission
        | super::super::Scenario::WorldRestoreStaleAdmissionDenied
        | super::super::Scenario::WorldActivationCurrentFacts
        | super::super::Scenario::WorldActivationRevokedDenied
        | super::super::Scenario::RaftFailoverCommitSurvives
        | super::super::Scenario::RaftStaleLeaderFenced
        | super::super::Scenario::RaftFinalQuorumEvidence
        | super::super::Scenario::RaftFalseQuorumDenied => {
            Err(super::super::invalid("scenario is not a cross-process transport scenario"))
        }
    }
}

pub(super) fn peer(role: &str, run_directory: &std::path::Path) -> Option<crate::error::Result<()>> {
    let should_withhold = match role {
        LISTENER_ROLE => false,
        WITHHOLDING_LISTENER_ROLE => true,
        _ => return None,
    };
    Some(fixture::runtime().and_then(|runtime| runtime.block_on(listener::serve(run_directory, should_withhold))))
}

// r[impl molten.fabric_transport.cross_process_endpoint]
fn endpoint_import(launcher: &super::Launcher) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut peer = PeerListener::start(launcher, LISTENER_ROLE)?;
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    evidence.check("handoff-strictly-decoded", endpoint.value == peer.handoff);
    evidence.check(
        "listener-identity-matches-independent-key",
        endpoint.descriptor.public_endpoint_identity == fixture::public_identity(LISTENER_SECRET),
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
    evidence.runtime(fixture::endpoint_projection(&endpoint));
    let client = fixture::exchange(fixture::client_input(
        endpoint.clone(),
        fixture::expected(fixture::public_identity(LISTENER_SECRET))?,
    )?)?;
    evidence.check("imported-endpoint-dialed", client.delivery == crate::fabric_transport::DeliveryOutcome::Delivered);
    evidence.check(
        "remote-identity-is-imported-listener",
        client.remote_transport_identity_ref
            == crate::preserves_rail::content_ref_from_bytes(fixture::listener_id(LISTENER_SECRET).as_bytes()),
    );
    evidence.runtime(fixture::frame_projection(&client));
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
        crate::preserves_rail::string(crate::fabric_transport::CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA),
        crate::preserves_rail::string(fixture::reference("qualification substituted descriptor")),
        crate::preserves_rail::value_to_iovalue(&fields[2]),
        crate::preserves_rail::value_to_iovalue(&fields[3]),
    ]);
    let rejected = crate::fabric_transport::parse_endpoint(&tampered);
    evidence.check("tampered-descriptor-ref-denied", rejected.is_err());
    if let Err(error) = &rejected {
        evidence.runtime(super::error_class(error));
    }
    let endpoint = crate::fabric_transport::parse_endpoint(&peer.handoff)?;
    let impostor = fixture::exchange(fixture::client_input(
        endpoint.clone(),
        fixture::expected(fixture::public_identity(OTHER_SECRET))?,
    )?);
    evidence.check("unexpected-listener-identity-denied-before-dial", impostor.is_err());
    if let Err(error) = &impostor {
        evidence.runtime(super::error_class(error));
    }
    let mut stale =
        fixture::client_input(endpoint.clone(), fixture::expected(fixture::public_identity(LISTENER_SECRET))?)?;
    stale.expected.generation = GENERATION + 1;
    evidence.check("stale-generation-import-denied-before-dial", fixture::exchange(stale).is_err());
    // Only the admitted import may reach the listener: it accepts exactly one
    // session, so any denied attempt that had dialed would have consumed it.
    let admitted = fixture::exchange(fixture::client_input(
        endpoint.clone(),
        fixture::expected(fixture::public_identity(LISTENER_SECRET))?,
    )?)?;
    evidence.check(
        "admitted-import-still-delivers",
        admitted.delivery == crate::fabric_transport::DeliveryOutcome::Delivered,
    );
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
    let client = fixture::exchange(fixture::client_input(
        endpoint.clone(),
        fixture::expected(fixture::public_identity(LISTENER_SECRET))?,
    )?)?;
    evidence.check("client-frame-delivered", client.delivery == crate::fabric_transport::DeliveryOutcome::Delivered);
    evidence.check("acknowledgement-binds-payload", client.acknowledgement_ref == client.payload_ref);
    evidence.check(
        "payload-ref-is-canonical-frame-ref",
        client.payload_ref == crate::fabric_transport::frame_ref(&fixture::request_ref(), PAYLOAD),
    );
    evidence.check("no-automatic-retry", client.automatic_retry_count == 0);
    evidence.runtime(fixture::frame_projection(&client));
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
    let client = fixture::exchange(fixture::client_input(
        endpoint.clone(),
        fixture::expected(fixture::public_identity(LISTENER_SECRET))?,
    )?);
    evidence.check("unacknowledged-frame-not-reported-delivered", client.is_err());
    if let Err(error) = &client {
        evidence.runtime(super::error_class(error));
    }
    let report = peer.finish()?;
    evidence.check("listener-withheld-acknowledgement-after-reading-frame", report.withheld);
    evidence.check(
        "withheld-payload-is-the-submitted-frame",
        report.payload_ref.as_deref()
            == Some(crate::fabric_transport::frame_ref(&fixture::request_ref(), PAYLOAD).as_str()),
    );
    evidence.check("listener-did-not-report-delivery", !report.delivered);
    evidence.check("client-did-not-retry-automatically", !report.second_session);
    evidence.check("listener-cleanup-clean", report.clean);
    evidence.runtime(report.value);
    Ok(evidence)
}

/// A started listener peer and its canonical endpoint handoff.
struct PeerListener {
    peer: super::peer::Process,
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
            clean: super::super::text(&fields[4])?
                == format!("{:?}", crate::fabric_transport::ListenerTerminalClass::Clean),
            payload_ref,
            value,
        })
    }
}
