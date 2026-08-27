use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use preserves::IOValue;
use preserves::Value;
use preserves::ValueImpl;

use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::*;

pub const DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS: u64 = 30_000;
pub const MAX_DISTINCT_PROCESS_TIMEOUT_MS: u64 = 300_000;

const CHILD_POLL_INTERVAL_MS: u64 = 10;
const IROH_SECRET_KEY_BYTES_LOCAL: usize = IROH_SECRET_KEY_BYTES;
const LISTENER_SECRET_BYTE: u8 = 17;
const CLIENT_SECRET_BYTE: u8 = 29;
const PROFILE_LIMIT: u64 = 8;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_LIMIT: u64 = 16_384;
const INFLIGHT_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW: u64 = 64;
const LENGTH_PREFIX_BYTES: u64 = 8;
const GENERATION: u64 = 1;
const VALID_FROM_TICK: u64 = 1;
const VALID_UNTIL_TICK: u64 = 100;
const OBSERVED_TICK: u64 = 10;
const PROFILE_REF: &str = "blake3:7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a";
const FRAMING_REF: &str = "blake3:7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b";
const AUTHORITY_REF: &str = "blake3:7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c";
const LISTENER_CAPABILITY_REF: &str = "blake3:7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d";
const CLIENT_CAPABILITY_REF: &str = "blake3:7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e";
const LISTENER_IDENTITY_REF: &str = "blake3:7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f";
const PEER_CONTEXT_REF: &str = "blake3:8080808080808080808080808080808080808080808080808080808080808080";
const LOCATOR_COHORT_REF: &str = "blake3:8181818181818181818181818181818181818181818181818181818181818181";
const VALIDITY_REF: &str = "blake3:8282828282828282828282828282828282828282828282828282828282828282";
const SESSION_REF: &str = "blake3:8383838383838383838383838383838383838383838383838383838383838383";
pub const DEFAULT_DISTINCT_PROCESS_REQUEST_REF: &str =
    "blake3:8484848484848484848484848484848484848484848484848484848484848484";
pub const DEFAULT_DISTINCT_PROCESS_PAYLOAD: &[u8] = b"distinct-process-bounded-frame";
const INVOCATION_DOMAIN: &str = "molten.fabric.transport.distinct-process-invocation.v1";
const COMMAND_PROFILE_DOMAIN: &str = "molten.fabric.transport.distinct-process-command.v1";
const RUN_INDEX_DOMAIN: &str = "molten.fabric.transport.distinct-process-index.v1";
const PARTICIPANT_SCHEMA: &str = "molten.fabric.transport.distinct-process-participant.v1";
const START_SCHEMA: &str = "molten.fabric.transport.distinct-process-start.v1";
const CLEANUP_SCHEMA: &str = "molten.fabric.transport.distinct-process-cleanup.v1";
const RUN_SCHEMA: &str = "molten.fabric.transport.distinct-process-run.v1";
const VERIFICATION_SCHEMA: &str = "molten.fabric.transport.distinct-process-verification.v1";
const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const LISTENER_ROLE: &str = "listener";
const CLIENT_ROLE: &str = "client";
const NOT_APPLICABLE: &str = "not-applicable";
const HANDOFF_FILE: &str = "endpoint-handoff.preserves";
const LISTENER_START_FILE: &str = "listener-start.preserves";
const CLIENT_START_FILE: &str = "client-start.preserves";
const LISTENER_TERMINAL_FILE: &str = "listener-terminal.preserves";
const CLIENT_TERMINAL_FILE: &str = "client-terminal.preserves";
const CLEANUP_FILE: &str = "cleanup.preserves";
const PARENT_RUN_FILE: &str = "parent-run.preserves";
const VERIFICATION_FILE: &str = "verification.preserves";
const FAILURE_FILE: &str = "failure.preserves";
const INDEX_FILE: &str = "artifact-index.tsv";
const LISTENER_LOG_FILE: &str = "logs/listener.log";
const CLIENT_LOG_FILE: &str = "logs/client.log";
const REQUEST_INPUT_FILE: &str = "request-ref.txt";
const PAYLOAD_INPUT_FILE: &str = "payload.bin";
const INDEX_HEADER: &str = "molten.fabric-transport-distinct-process-index.v1";
const PARTICIPANT_FIELD_COUNT: usize = 24;
const START_FIELD_COUNT: usize = 6;
const CLEANUP_FIELD_COUNT: usize = 9;
const MAX_RUN_FILES: usize = 16;
const MAX_ARTIFACT_BYTES: u64 = 1_048_576;
const EXPECTED_MEMBER_COUNT: usize = 13;

#[derive(Debug, Clone)]
pub struct DistinctProcessTransportRunInput {
    pub run_directory: PathBuf,
    pub process_binary: PathBuf,
    pub child_timeout_ms: u64,
    pub force: bool,
    pub request_ref: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessTransportRun {
    pub decision: String,
    pub parent_ref: String,
    pub verification_ref: String,
    pub diagnostics: Vec<String>,
    pub run_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessTransportVerification {
    pub decision: String,
    pub parent_ref: String,
    pub verification_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParticipantArtifact {
    role: EndpointParticipantRole,
    invocation_ref: String,
    descriptor_ref: String,
    handoff_ref: String,
    profile_id: String,
    protocol_id: String,
    alpn: String,
    service_id: String,
    generation: u64,
    request_ref: String,
    payload_ref: String,
    acknowledgement_ref: String,
    remote_transport_identity_ref: String,
    payload_bytes: u64,
    delivery: DeliveryOutcome,
    retry: RetryDisposition,
    automatic_retry_count: u64,
    session_cleanup_ref: String,
    endpoint_cleanup_ref: String,
    drain_reason: String,
    value: IOValue,
    artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartArtifact {
    role: EndpointParticipantRole,
    invocation_ref: String,
    command_profile_ref: String,
    parent_observed: bool,
    value: IOValue,
    artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupArtifact {
    listener_terminal_ref: String,
    client_terminal_ref: String,
    listener_cleanup_ref: String,
    client_cleanup_ref: String,
    listener_exited: bool,
    client_exited: bool,
    no_orphans: bool,
    value: IOValue,
    artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedArtifact {
    relative_path: String,
    artifact_kind: String,
    expected_ref: String,
    format: String,
}

struct ReapingChild {
    child: Child,
    finished: bool,
}

impl ReapingChild {
    fn spawn(binary: &Path, role_command: &str, run_directory: &Path, log_path: &Path) -> Result<Self> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
        }
        let stdout = File::create(log_path).map_err(MoltenError::from)?;
        let stderr = stdout.try_clone().map_err(MoltenError::from)?;
        let child = Command::new(binary)
            .args(["cluster", role_command, "--run-dir"])
            .arg(run_directory)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(MoltenError::from)?;
        Ok(Self { child, finished: false })
    }

    fn id(&self) -> u32 {
        self.child.id()
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().map_err(MoltenError::from)
    }

    fn wait_bounded(&mut self, timeout: Duration, label: &str) -> Result<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                self.finished = true;
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                let _kill = self.child.kill();
                let _wait = self.child.wait();
                self.finished = true;
                return Err(MoltenError::invalid_harness(format!("distinct-process {label} child timed out")));
            }
            std::thread::sleep(Duration::from_millis(CHILD_POLL_INTERVAL_MS));
        }
    }
}

impl Drop for ReapingChild {
    fn drop(&mut self) {
        if !self.finished {
            let _kill = self.child.kill();
            let _wait = self.child.wait();
            self.finished = true;
        }
    }
}

// r[impl molten.fabric_transport.distinct_process_evidence]
// r[impl molten.fabric_transport.cross_process_validation]
pub fn execute_distinct_process_transport_run(
    input: &DistinctProcessTransportRunInput,
) -> Result<DistinctProcessTransportRun> {
    validate_run_input(input)?;
    prepare_run_directory(&input.run_directory, input.force)?;
    match execute_prepared_distinct_process_transport_run(input) {
        Ok(run) => Ok(run),
        Err(error) => {
            let failure =
                failure_value(&text_ref("molten.fabric.transport.distinct-process-error.v1", &error.to_string()));
            let _failure_write = write_preserves(&input.run_directory.join(FAILURE_FILE), &failure);
            Err(error)
        }
    }
}

fn execute_prepared_distinct_process_transport_run(
    input: &DistinctProcessTransportRunInput,
) -> Result<DistinctProcessTransportRun> {
    let timeout = Duration::from_millis(input.child_timeout_ms);
    write_transport_input(&input.run_directory, &input.request_ref, &input.payload)?;
    let listener_invocation_ref = invocation_ref(LISTENER_ROLE);
    let client_invocation_ref = invocation_ref(CLIENT_ROLE);

    let mut listener = ReapingChild::spawn(
        &input.process_binary,
        "fabric-transport-listener-child",
        &input.run_directory,
        &input.run_directory.join(LISTENER_LOG_FILE),
    )?;
    let listener_start = start_artifact(EndpointParticipantRole::Listener, &listener_invocation_ref)?;
    write_preserves(&input.run_directory.join(LISTENER_START_FILE), &listener_start.value)?;
    wait_for_handoff(&mut listener, &input.run_directory.join(HANDOFF_FILE), timeout)?;
    let handoff = read_endpoint_handoff(&input.run_directory.join(HANDOFF_FILE))?;
    validate_fixture_endpoint(&handoff)?;

    let mut client = ReapingChild::spawn(
        &input.process_binary,
        "fabric-transport-client-child",
        &input.run_directory,
        &input.run_directory.join(CLIENT_LOG_FILE),
    )?;
    let child_handles_distinct = listener.id() != client.id();
    let client_start = start_artifact(EndpointParticipantRole::Client, &client_invocation_ref)?;
    write_preserves(&input.run_directory.join(CLIENT_START_FILE), &client_start.value)?;

    let client_status = client.wait_bounded(timeout, CLIENT_ROLE)?;
    if !client_status.success() {
        return Err(MoltenError::invalid_harness(format!("distinct-process client child exited with {client_status}")));
    }
    let listener_status = listener.wait_bounded(timeout, LISTENER_ROLE)?;
    if !listener_status.success() {
        return Err(MoltenError::invalid_harness(format!(
            "distinct-process listener child exited with {listener_status}"
        )));
    }

    let listener_terminal = read_participant(&input.run_directory.join(LISTENER_TERMINAL_FILE))?;
    let client_terminal = read_participant(&input.run_directory.join(CLIENT_TERMINAL_FILE))?;
    let cleanup = cleanup_artifact(&listener_terminal, &client_terminal, true, true, true)?;
    write_preserves(&input.run_directory.join(CLEANUP_FILE), &cleanup.value)?;
    let assessment_input = assessment_input(
        &listener_start,
        &client_start,
        &listener_terminal,
        &client_terminal,
        &cleanup,
        child_handles_distinct,
    );
    let assessment = assess_distinct_process_transport_evidence(&assessment_input);
    let parent_value = parent_run_value(
        &listener_start,
        &client_start,
        &listener_terminal,
        &client_terminal,
        &cleanup,
        &assessment_input,
        &assessment,
    )?;
    let parent_ref = crate::preserves_rail::canonical_hash(&parent_value)?;
    write_preserves(&input.run_directory.join(PARENT_RUN_FILE), &parent_value)?;
    write_index(&input.run_directory)?;
    let verification = verify_distinct_process_run_directory_inner(&input.run_directory, false)?;
    write_preserves(&input.run_directory.join(VERIFICATION_FILE), &verification.value)?;
    Ok(DistinctProcessTransportRun {
        decision: verification.decision,
        parent_ref,
        verification_ref: verification.verification_ref,
        diagnostics: verification.diagnostics,
        run_directory: input.run_directory.clone(),
    })
}

// r[impl molten.fabric_transport.cross_process_listener]
pub fn run_distinct_process_listener_child(run_directory: &Path) -> Result<()> {
    validate_child_directory(run_directory)?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| MoltenError::invalid_harness(format!("listener runtime creation failed: {error}")))?;
    runtime.block_on(async {
        let mut listener = IrohCrossProcessListener::bind(IrohCrossProcessListenerInput {
            profile: fixture_profile(),
            protocol: fixture_protocol(),
            capability: fixture_capability(LISTENER_SECRET_BYTE, LISTENER_CAPABILITY_REF)?,
            bind_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            listener_identity_ref: LISTENER_IDENTITY_REF.to_string(),
            expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
            locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
            disclosure: fixture_disclosure(),
            validity: fixture_validity(),
            admission: EndpointAdmissionState::fully_active(),
            observed_tick: OBSERVED_TICK,
        })
        .await?;
        write_preserves_atomic(&run_directory.join(HANDOFF_FILE), &listener.handoff().value)?;
        let (request_ref, _payload) = read_transport_input(run_directory)?;
        let frame = listener
            .accept_one(SESSION_REF, &request_ref, Duration::from_millis(DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS))
            .await?;
        let endpoint_cleanup = listener.drain_and_close(ListenerDrainReason::OperatorRequest).await?;
        let participant = participant_artifact(
            EndpointParticipantRole::Listener,
            &invocation_ref(LISTENER_ROLE),
            &frame,
            &endpoint_cleanup.cleanup_evidence_ref,
            Some(endpoint_cleanup.drain_reason),
            &fixture_profile().profile,
            &fixture_protocol(),
            &crate::preserves_rail::canonical_hash(&read_endpoint_handoff(&run_directory.join(HANDOFF_FILE))?.value)?,
        )?;
        write_preserves(&run_directory.join(LISTENER_TERMINAL_FILE), &participant.value)
    })
}

// r[impl molten.fabric_transport.cross_process_session]
pub fn run_distinct_process_client_child(run_directory: &Path) -> Result<()> {
    validate_child_directory(run_directory)?;
    let handoff = read_endpoint_handoff(&run_directory.join(HANDOFF_FILE))?;
    validate_fixture_endpoint(&handoff)?;
    let (request_ref, payload) = read_transport_input(run_directory)?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| MoltenError::invalid_harness(format!("client runtime creation failed: {error}")))?;
    runtime.block_on(async {
        let frame = exchange_cross_process_frame(
            IrohCrossProcessClientInput {
                profile: fixture_profile(),
                protocol: fixture_protocol(),
                capability: fixture_capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF)?,
                bind_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                expected: expected_binding(&handoff),
                endpoint: handoff.clone(),
                admission: EndpointAdmissionState::fully_active(),
                session_ref: SESSION_REF.to_string(),
                request_ref,
            },
            &payload,
            Duration::from_millis(DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS),
        )
        .await?;
        let participant = participant_artifact(
            EndpointParticipantRole::Client,
            &invocation_ref(CLIENT_ROLE),
            &frame,
            &frame.cleanup_evidence_ref,
            None,
            &fixture_profile().profile,
            &fixture_protocol(),
            &handoff.handoff_ref,
        )?;
        write_preserves(&run_directory.join(CLIENT_TERMINAL_FILE), &participant.value)
    })
}

// r[impl molten.fabric_transport.distinct_process_evidence]
pub fn verify_distinct_process_transport_run(run_directory: &Path) -> Result<DistinctProcessTransportVerification> {
    verify_distinct_process_run_directory_inner(run_directory, true)
}

fn verify_distinct_process_run_directory_inner(
    run_directory: &Path,
    require_companion: bool,
) -> Result<DistinctProcessTransportVerification> {
    let mut diagnostics = validate_run_membership(run_directory, require_companion)?;
    let listener_start = read_start(&run_directory.join(LISTENER_START_FILE))?;
    let client_start = read_start(&run_directory.join(CLIENT_START_FILE))?;
    let listener_terminal = read_participant(&run_directory.join(LISTENER_TERMINAL_FILE))?;
    let client_terminal = read_participant(&run_directory.join(CLIENT_TERMINAL_FILE))?;
    let actual_cleanup = read_cleanup(&run_directory.join(CLEANUP_FILE))?;
    let expected_cleanup = cleanup_artifact(&listener_terminal, &client_terminal, true, true, true)?;
    if actual_cleanup.value != expected_cleanup.value {
        diagnostics.push("cleanup-artifact-mismatch".to_string());
    }
    let assessment_input =
        assessment_input(&listener_start, &client_start, &listener_terminal, &client_terminal, &actual_cleanup, true);
    let assessment = assess_distinct_process_transport_evidence(&assessment_input);
    diagnostics.extend(assessment.issues.iter().map(|issue| issue.code().to_string()));
    let expected_parent = parent_run_value(
        &listener_start,
        &client_start,
        &listener_terminal,
        &client_terminal,
        &actual_cleanup,
        &assessment_input,
        &assessment,
    )?;
    let actual_parent = read_preserves(&run_directory.join(PARENT_RUN_FILE))?;
    if actual_parent != expected_parent {
        diagnostics.push("parent-run-artifact-mismatch".to_string());
    }
    let parent_ref = crate::preserves_rail::canonical_hash(&actual_parent)?;
    let index_text = std::fs::read_to_string(run_directory.join(INDEX_FILE)).map_err(MoltenError::from)?;
    let expected_index = render_index(&collect_indexed_artifacts(run_directory)?);
    if index_text != expected_index {
        diagnostics.push("artifact-index-mismatch".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let index_ref = text_ref(RUN_INDEX_DOMAIN, &index_text);
    let initial_decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    };
    let initial_value = verification_value(initial_decision, &index_ref, &parent_ref, &diagnostics);
    if require_companion {
        let companion = read_preserves(&run_directory.join(VERIFICATION_FILE))?;
        if companion != initial_value {
            diagnostics.push("verification-companion-mismatch".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = verification_value(&decision, &index_ref, &parent_ref, &diagnostics);
    let verification_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DistinctProcessTransportVerification {
        decision,
        parent_ref,
        verification_ref,
        diagnostics,
        value,
    })
}

fn validate_run_input(input: &DistinctProcessTransportRunInput) -> Result<()> {
    if input.child_timeout_ms == 0 || input.child_timeout_ms > MAX_DISTINCT_PROCESS_TIMEOUT_MS {
        return Err(MoltenError::invalid_harness(format!(
            "distinct-process child timeout must be between 1 and {MAX_DISTINCT_PROCESS_TIMEOUT_MS} milliseconds"
        )));
    }
    if input.run_directory.as_os_str().is_empty() || input.process_binary.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness(
            "distinct-process run requires explicit run directory and process binary",
        ));
    }
    crate::preserves_rail::validate_content_ref(&input.request_ref)?;
    let payload_bytes = u64::try_from(input.payload.len())
        .map_err(|_| MoltenError::invalid_harness("distinct-process payload length exceeds u64"))?;
    if input.payload.is_empty() || payload_bytes > FRAME_LIMIT {
        return Err(MoltenError::invalid_harness(
            "distinct-process payload must be nonempty and within the frame bound",
        ));
    }
    Ok(())
}

fn write_transport_input(run_directory: &Path, request_ref: &str, payload: &[u8]) -> Result<()> {
    std::fs::write(run_directory.join(REQUEST_INPUT_FILE), request_ref.as_bytes()).map_err(MoltenError::from)?;
    std::fs::write(run_directory.join(PAYLOAD_INPUT_FILE), payload).map_err(MoltenError::from)
}

fn read_transport_input(run_directory: &Path) -> Result<(String, Vec<u8>)> {
    let request_ref = std::fs::read_to_string(run_directory.join(REQUEST_INPUT_FILE)).map_err(MoltenError::from)?;
    crate::preserves_rail::validate_content_ref(&request_ref)?;
    let payload = std::fs::read(run_directory.join(PAYLOAD_INPUT_FILE)).map_err(MoltenError::from)?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("distinct-process payload length exceeds u64"))?;
    if payload.is_empty() || payload_bytes > FRAME_LIMIT {
        return Err(MoltenError::invalid_harness("distinct-process payload input is empty or over-bound"));
    }
    Ok((request_ref, payload))
}

fn prepare_run_directory(run_directory: &Path, force: bool) -> Result<()> {
    if run_directory.exists() {
        if !force {
            return Err(MoltenError::invalid_harness(format!(
                "distinct-process run directory already exists: {}",
                run_directory.display()
            )));
        }
        std::fs::remove_dir_all(run_directory).map_err(MoltenError::from)?;
    }
    std::fs::create_dir_all(run_directory).map_err(MoltenError::from)
}

fn validate_child_directory(run_directory: &Path) -> Result<()> {
    if !run_directory.is_dir() {
        return Err(MoltenError::invalid_harness("distinct-process child requires an existing run directory"));
    }
    Ok(())
}

fn wait_for_handoff(child: &mut ReapingChild, handoff_path: &Path, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    loop {
        if handoff_path.is_file() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            child.finished = true;
            return Err(MoltenError::invalid_harness(format!("listener exited before endpoint handoff with {status}")));
        }
        if started.elapsed() >= timeout {
            return Err(MoltenError::invalid_harness("listener did not publish endpoint handoff before timeout"));
        }
        std::thread::sleep(Duration::from_millis(CHILD_POLL_INTERVAL_MS));
    }
}

fn fixture_profile() -> CanonicalTransportProfile {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-distinct-process-v1".to_string(),
        profile_ref: PROFILE_REF.to_string(),
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
    .expect("static distinct-process profile must be valid")
}

fn fixture_protocol() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "distinct-process-echo".to_string(),
        version: "v1".to_string(),
        alpn: "molten/distinct-process-echo/1".to_string(),
        extension_id: "distinct-process-extension".to_string(),
        service_id: "distinct-process-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        framing: FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: FRAMING_REF.to_string(),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW,
        },
        registration_authority_ref: AUTHORITY_REF.to_string(),
        profile_ref: PROFILE_REF.to_string(),
    }
}

fn fixture_disclosure() -> EndpointDisclosurePolicy {
    EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

fn fixture_validity() -> EndpointValidityCohort {
    EndpointValidityCohort {
        cohort_ref: VALIDITY_REF.to_string(),
        not_before_tick: VALID_FROM_TICK,
        expires_at_tick: VALID_UNTIL_TICK,
    }
}

fn fixture_capability(secret_byte: u8, capability_ref: &str) -> Result<IrohEndpointCapability> {
    IrohEndpointCapability::from_secret_bytes([secret_byte; IROH_SECRET_KEY_BYTES_LOCAL], capability_ref.to_string())
}

fn expected_binding(endpoint: &CanonicalCrossProcessEndpoint) -> ExpectedEndpointBinding {
    let descriptor = &endpoint.descriptor;
    ExpectedEndpointBinding {
        profile_id: descriptor.profile_id.clone(),
        profile_ref: descriptor.profile_ref.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        protocol_version: descriptor.protocol_version.clone(),
        alpn: descriptor.alpn.clone(),
        extension_id: descriptor.extension_id.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        listener_identity_ref: descriptor.listener_identity_ref.clone(),
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        observed_tick: OBSERVED_TICK,
    }
}

fn validate_fixture_endpoint(endpoint: &CanonicalCrossProcessEndpoint) -> Result<()> {
    validate_cross_process_endpoint(&fixture_profile().profile, &fixture_protocol(), &endpoint.descriptor)
        .map_err(|issues| MoltenError::invalid_harness(format!("fixture endpoint denied: {issues:?}")))
}

fn participant_artifact(
    role: EndpointParticipantRole,
    invocation_ref: &str,
    frame: &CrossProcessFrameEvidence,
    endpoint_cleanup_ref: &str,
    drain_reason: Option<ListenerDrainReason>,
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    handoff_ref: &str,
) -> Result<ParticipantArtifact> {
    for reference in [invocation_ref, endpoint_cleanup_ref, handoff_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let value = crate::preserves_rail::record("fabric-transport-participant-terminal-v1", vec![
        crate::preserves_rail::string(PARTICIPANT_SCHEMA),
        crate::preserves_rail::string(PASS_DECISION),
        crate::preserves_rail::string(role.as_str()),
        crate::preserves_rail::string(invocation_ref),
        crate::preserves_rail::string(&frame.descriptor_ref),
        crate::preserves_rail::string(handoff_ref),
        crate::preserves_rail::string(&profile.profile_id),
        crate::preserves_rail::string(&protocol.protocol_id),
        crate::preserves_rail::string(&protocol.alpn),
        crate::preserves_rail::string(&protocol.service_id),
        crate::preserves_rail::u64_value(protocol.generation),
        crate::preserves_rail::string(&frame.request_ref),
        crate::preserves_rail::string(&frame.payload_ref),
        crate::preserves_rail::string(&frame.acknowledgement_ref),
        crate::preserves_rail::string(&frame.remote_transport_identity_ref),
        crate::preserves_rail::u64_value(frame.payload_bytes),
        crate::preserves_rail::string(frame.delivery.as_str()),
        crate::preserves_rail::string(frame.retry.as_str()),
        crate::preserves_rail::u64_value(frame.automatic_retry_count),
        crate::preserves_rail::string(&frame.cleanup_evidence_ref),
        crate::preserves_rail::string(endpoint_cleanup_ref),
        crate::preserves_rail::string(drain_reason.map_or(NOT_APPLICABLE, ListenerDrainReason::as_str)),
        strings_value(profile.non_claims.iter().map(|claim| claim.as_str())),
        checks(&[
            "canonical-frame-observed",
            "terminal-cleanup-observed",
            "payload-bytes-excluded",
            "runtime-handles-excluded",
        ]),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ParticipantArtifact {
        role,
        invocation_ref: invocation_ref.to_string(),
        descriptor_ref: frame.descriptor_ref.clone(),
        handoff_ref: handoff_ref.to_string(),
        profile_id: profile.profile_id.clone(),
        protocol_id: protocol.protocol_id.clone(),
        alpn: protocol.alpn.clone(),
        service_id: protocol.service_id.clone(),
        generation: protocol.generation,
        request_ref: frame.request_ref.clone(),
        payload_ref: frame.payload_ref.clone(),
        acknowledgement_ref: frame.acknowledgement_ref.clone(),
        remote_transport_identity_ref: frame.remote_transport_identity_ref.clone(),
        payload_bytes: frame.payload_bytes,
        delivery: frame.delivery,
        retry: frame.retry,
        automatic_retry_count: frame.automatic_retry_count,
        session_cleanup_ref: frame.cleanup_evidence_ref.clone(),
        endpoint_cleanup_ref: endpoint_cleanup_ref.to_string(),
        drain_reason: drain_reason.map_or(NOT_APPLICABLE, ListenerDrainReason::as_str).to_string(),
        value,
        artifact_ref,
    })
}

fn start_artifact(role: EndpointParticipantRole, invocation_ref: &str) -> Result<StartArtifact> {
    crate::preserves_rail::validate_content_ref(invocation_ref)?;
    let command_profile_ref = command_profile_ref(role.as_str());
    let value = crate::preserves_rail::record("fabric-transport-child-start-v1", vec![
        crate::preserves_rail::string(START_SCHEMA),
        crate::preserves_rail::string(role.as_str()),
        crate::preserves_rail::string(invocation_ref),
        crate::preserves_rail::string(&command_profile_ref),
        crate::preserves_rail::bool_value(true),
        checks(&["parent-observed-start", "raw-process-id-excluded"]),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(StartArtifact {
        role,
        invocation_ref: invocation_ref.to_string(),
        command_profile_ref,
        parent_observed: true,
        value,
        artifact_ref,
    })
}

fn cleanup_artifact(
    listener: &ParticipantArtifact,
    client: &ParticipantArtifact,
    listener_exited: bool,
    client_exited: bool,
    no_orphans: bool,
) -> Result<CleanupArtifact> {
    let value = crate::preserves_rail::record("fabric-transport-distinct-cleanup-v1", vec![
        crate::preserves_rail::string(CLEANUP_SCHEMA),
        crate::preserves_rail::string(&listener.artifact_ref),
        crate::preserves_rail::string(&client.artifact_ref),
        crate::preserves_rail::string(&listener.endpoint_cleanup_ref),
        crate::preserves_rail::string(&client.endpoint_cleanup_ref),
        crate::preserves_rail::bool_value(listener_exited),
        crate::preserves_rail::bool_value(client_exited),
        crate::preserves_rail::bool_value(no_orphans),
        checks(&[
            "listener-reaped",
            "client-reaped",
            "no-orphaned-children",
            "cleanup-refs-bound",
        ]),
    ]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CleanupArtifact {
        listener_terminal_ref: listener.artifact_ref.clone(),
        client_terminal_ref: client.artifact_ref.clone(),
        listener_cleanup_ref: listener.endpoint_cleanup_ref.clone(),
        client_cleanup_ref: client.endpoint_cleanup_ref.clone(),
        listener_exited,
        client_exited,
        no_orphans,
        value,
        artifact_ref,
    })
}

fn assessment_input(
    listener_start: &StartArtifact,
    client_start: &StartArtifact,
    listener: &ParticipantArtifact,
    client: &ParticipantArtifact,
    cleanup: &CleanupArtifact,
    child_handles_distinct: bool,
) -> DistinctProcessTransportEvidenceInput {
    DistinctProcessTransportEvidenceInput {
        listener: participant_evidence(listener_start, listener, cleanup.listener_exited),
        client: participant_evidence(client_start, client, cleanup.client_exited),
        handoff_ref: listener.handoff_ref.clone(),
        child_handles_distinct,
        handoff_observed_before_client_start: true,
        cleanup_succeeded: cleanup.listener_exited && cleanup.client_exited && cleanup.no_orphans,
        same_process_loopback: false,
        child_only_separation_claim: false,
        default_readback_redacted: true,
        payloads_excluded: true,
        accepted_sessions: GENERATION,
        max_sessions: PROFILE_LIMIT,
        exchanged_bytes: listener.payload_bytes,
        max_frame_bytes: FRAME_LIMIT,
    }
}

fn participant_evidence(
    start: &StartArtifact,
    participant: &ParticipantArtifact,
    exited: bool,
) -> DistinctProcessParticipantEvidence {
    let expected_command_profile_ref = command_profile_ref(participant.role.as_str());
    let parent_observed_start = start.parent_observed
        && start.role == participant.role
        && start.invocation_ref == participant.invocation_ref
        && start.command_profile_ref == expected_command_profile_ref;
    DistinctProcessParticipantEvidence {
        role: participant.role,
        invocation_ref: participant.invocation_ref.clone(),
        parent_start_ref: start.artifact_ref.clone(),
        terminal_ref: participant.artifact_ref.clone(),
        cleanup_ref: participant.endpoint_cleanup_ref.clone(),
        descriptor_ref: participant.descriptor_ref.clone(),
        profile_id: participant.profile_id.clone(),
        protocol_id: participant.protocol_id.clone(),
        alpn: participant.alpn.clone(),
        service_id: participant.service_id.clone(),
        generation: participant.generation,
        request_ref: participant.request_ref.clone(),
        payload_ref: participant.payload_ref.clone(),
        acknowledgement_ref: participant.acknowledgement_ref.clone(),
        parent_observed_start,
        parent_observed_terminal: true,
        parent_observed_exit: exited,
        automatic_retry_count: participant.automatic_retry_count,
    }
}

fn parent_run_value(
    listener_start: &StartArtifact,
    client_start: &StartArtifact,
    listener: &ParticipantArtifact,
    client: &ParticipantArtifact,
    cleanup: &CleanupArtifact,
    input: &DistinctProcessTransportEvidenceInput,
    assessment: &DistinctProcessTransportAssessment,
) -> Result<IOValue> {
    if listener.handoff_ref != client.handoff_ref {
        return Err(MoltenError::invalid_harness("participant handoff refs do not match"));
    }
    if listener.drain_reason == NOT_APPLICABLE || client.drain_reason != NOT_APPLICABLE {
        return Err(MoltenError::invalid_harness("participant drain reason does not match its listener/client role"));
    }
    let decision = if assessment.admitted {
        PASS_DECISION
    } else {
        DENY_DECISION
    };
    Ok(crate::preserves_rail::record("fabric-transport-distinct-process-run-v1", vec![
        crate::preserves_rail::string(RUN_SCHEMA),
        crate::preserves_rail::string(decision),
        crate::preserves_rail::string(&listener_start.artifact_ref),
        crate::preserves_rail::string(&client_start.artifact_ref),
        crate::preserves_rail::string(&listener.artifact_ref),
        crate::preserves_rail::string(&client.artifact_ref),
        crate::preserves_rail::string(&cleanup.artifact_ref),
        crate::preserves_rail::string(&listener.handoff_ref),
        crate::preserves_rail::string(&listener.descriptor_ref),
        crate::preserves_rail::string(&listener.profile_id),
        crate::preserves_rail::string(&listener.protocol_id),
        crate::preserves_rail::string(&listener.alpn),
        crate::preserves_rail::string(&listener.service_id),
        crate::preserves_rail::u64_value(listener.generation),
        crate::preserves_rail::string(&listener.request_ref),
        crate::preserves_rail::string(&listener.payload_ref),
        crate::preserves_rail::string(&listener.acknowledgement_ref),
        crate::preserves_rail::u64_value(input.accepted_sessions),
        crate::preserves_rail::u64_value(input.max_sessions),
        crate::preserves_rail::u64_value(input.exchanged_bytes),
        crate::preserves_rail::u64_value(input.max_frame_bytes),
        strings_value(REQUIRED_TRANSPORT_NON_CLAIMS.iter().map(|claim| claim.as_str())),
        strings_value(assessment.issues.iter().map(|issue| issue.code())),
        checks(&[
            "parent-observed-distinct-child-handles",
            "handoff-precedes-client-start",
            "participant-bindings-match",
            "terminal-cleanup-and-exits-observed",
            "same-process-loopback-insufficient",
            "connectivity-only-non-claims",
        ]),
    ]))
}

fn verification_value(decision: &str, index_ref: &str, parent_ref: &str, diagnostics: &[String]) -> IOValue {
    crate::preserves_rail::record("fabric-transport-distinct-process-verification-v1", vec![
        crate::preserves_rail::string(VERIFICATION_SCHEMA),
        crate::preserves_rail::string(decision),
        crate::preserves_rail::string(index_ref),
        crate::preserves_rail::string(parent_ref),
        strings_value(diagnostics.iter().map(String::as_str)),
        checks(&[
            "offline-canonical-artifact-verification",
            "fixed-run-directory-membership",
            "child-only-claims-denied",
        ]),
    ])
}

fn failure_value(error_ref: &str) -> IOValue {
    crate::preserves_rail::record("fabric-transport-distinct-process-failure-v1", vec![
        crate::preserves_rail::string("molten.fabric.transport.distinct-process-failure.v1"),
        crate::preserves_rail::string(DENY_DECISION),
        crate::preserves_rail::string("execution-error"),
        crate::preserves_rail::string(error_ref),
        strings_value(REQUIRED_TRANSPORT_NON_CLAIMS.iter().map(|claim| claim.as_str())),
        checks(&[
            "raw-error-excluded",
            "owned-child-lifetimes-scope-bound",
            "cleanup-success-not-claimed",
            "failure-does-not-establish-process-separation",
        ]),
    ])
}

fn read_participant(path: &Path) -> Result<ParticipantArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-participant-terminal-v1", PARTICIPANT_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "participant schema")?, PARTICIPANT_SCHEMA)?;
    require_decision(next(&mut fields, "participant decision")?)?;
    let role = parse_role(&required_string(next(&mut fields, "participant role")?, "participant role")?)?;
    let invocation_ref = required_ref(next(&mut fields, "invocation ref")?, "invocation ref")?;
    let descriptor_ref = required_ref(next(&mut fields, "descriptor ref")?, "descriptor ref")?;
    let handoff_ref = required_ref(next(&mut fields, "handoff ref")?, "handoff ref")?;
    let profile_id = required_string(next(&mut fields, "profile id")?, "profile id")?;
    let protocol_id = required_string(next(&mut fields, "protocol id")?, "protocol id")?;
    let alpn = required_string(next(&mut fields, "ALPN")?, "ALPN")?;
    let service_id = required_string(next(&mut fields, "service id")?, "service id")?;
    let generation = required_u64(next(&mut fields, "generation")?, "generation")?;
    let request_ref = required_ref(next(&mut fields, "request ref")?, "request ref")?;
    let payload_ref = required_ref(next(&mut fields, "payload ref")?, "payload ref")?;
    let acknowledgement_ref = required_ref(next(&mut fields, "ack ref")?, "ack ref")?;
    let remote_transport_identity_ref = required_ref(next(&mut fields, "remote ref")?, "remote ref")?;
    let payload_bytes = required_u64(next(&mut fields, "payload bytes")?, "payload bytes")?;
    let delivery = parse_delivery(&required_string(next(&mut fields, "delivery")?, "delivery")?)?;
    let retry = parse_retry(&required_string(next(&mut fields, "retry")?, "retry")?)?;
    let automatic_retry_count = required_u64(next(&mut fields, "retry count")?, "retry count")?;
    let session_cleanup_ref = required_ref(next(&mut fields, "session cleanup")?, "session cleanup")?;
    let endpoint_cleanup_ref = required_ref(next(&mut fields, "endpoint cleanup")?, "endpoint cleanup")?;
    let drain_reason = required_string(next(&mut fields, "drain reason")?, "drain reason")?;
    let _non_claims = next(&mut fields, "non-claims")?;
    let _checks = next(&mut fields, "checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ParticipantArtifact {
        role,
        invocation_ref,
        descriptor_ref,
        handoff_ref,
        profile_id,
        protocol_id,
        alpn,
        service_id,
        generation,
        request_ref,
        payload_ref,
        acknowledgement_ref,
        remote_transport_identity_ref,
        payload_bytes,
        delivery,
        retry,
        automatic_retry_count,
        session_cleanup_ref,
        endpoint_cleanup_ref,
        drain_reason,
        value,
        artifact_ref,
    })
}

fn read_start(path: &Path) -> Result<StartArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-child-start-v1", START_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "start schema")?, START_SCHEMA)?;
    let role = parse_role(&required_string(next(&mut fields, "start role")?, "start role")?)?;
    let invocation_ref = required_ref(next(&mut fields, "start invocation")?, "start invocation")?;
    let command_profile_ref = required_ref(next(&mut fields, "command profile")?, "command profile")?;
    let parent_observed = required_bool(next(&mut fields, "parent observed")?, "parent observed")?;
    let _checks = next(&mut fields, "start checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(StartArtifact {
        role,
        invocation_ref,
        command_profile_ref,
        parent_observed,
        value,
        artifact_ref,
    })
}

fn read_cleanup(path: &Path) -> Result<CleanupArtifact> {
    let value = read_preserves(path)?;
    let fields = simple_record(&value, "fabric-transport-distinct-cleanup-v1", CLEANUP_FIELD_COUNT)?;
    let mut fields = fields.as_slice().iter();
    require_schema(next(&mut fields, "cleanup schema")?, CLEANUP_SCHEMA)?;
    let listener_terminal_ref = required_ref(next(&mut fields, "listener terminal")?, "listener terminal")?;
    let client_terminal_ref = required_ref(next(&mut fields, "client terminal")?, "client terminal")?;
    let listener_cleanup_ref = required_ref(next(&mut fields, "listener cleanup")?, "listener cleanup")?;
    let client_cleanup_ref = required_ref(next(&mut fields, "client cleanup")?, "client cleanup")?;
    let listener_exited = required_bool(next(&mut fields, "listener exited")?, "listener exited")?;
    let client_exited = required_bool(next(&mut fields, "client exited")?, "client exited")?;
    let no_orphans = required_bool(next(&mut fields, "no orphans")?, "no orphans")?;
    let _checks = next(&mut fields, "cleanup checks")?;
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CleanupArtifact {
        listener_terminal_ref,
        client_terminal_ref,
        listener_cleanup_ref,
        client_cleanup_ref,
        listener_exited,
        client_exited,
        no_orphans,
        value,
        artifact_ref,
    })
}

fn read_endpoint_handoff(path: &Path) -> Result<CanonicalCrossProcessEndpoint> {
    parse_canonical_cross_process_endpoint(&read_preserves(path)?)
}

fn read_preserves(path: &Path) -> Result<IOValue> {
    ensure_regular_file(path)?;
    let bytes = std::fs::read(path).map_err(MoltenError::from)?;
    crate::preserves_rail::parse_canonical_bytes(&bytes)
}

fn write_preserves(path: &Path, value: &IOValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    std::fs::write(path, bytes).map_err(MoltenError::from)
}

fn write_preserves_atomic(path: &Path, value: &IOValue) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| MoltenError::invalid_harness("endpoint handoff path requires a parent directory"))?;
    std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
    let temporary = path.with_extension("preserves.tmp");
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    let mut file = File::create(&temporary).map_err(MoltenError::from)?;
    file.write_all(&bytes).map_err(MoltenError::from)?;
    file.sync_all().map_err(MoltenError::from)?;
    std::fs::rename(&temporary, path).map_err(MoltenError::from)
}

fn write_index(run_directory: &Path) -> Result<()> {
    let entries = collect_indexed_artifacts(run_directory)?;
    std::fs::write(run_directory.join(INDEX_FILE), render_index(&entries)).map_err(MoltenError::from)
}

fn collect_indexed_artifacts(run_directory: &Path) -> Result<Vec<IndexedArtifact>> {
    let definitions = [
        (CLIENT_START_FILE, "parent-client-start", "preserves"),
        (CLIENT_TERMINAL_FILE, "client-terminal", "preserves"),
        (CLEANUP_FILE, "distinct-process-cleanup", "preserves"),
        (HANDOFF_FILE, "endpoint-handoff", "preserves"),
        (LISTENER_START_FILE, "parent-listener-start", "preserves"),
        (LISTENER_TERMINAL_FILE, "listener-terminal", "preserves"),
        (PARENT_RUN_FILE, "distinct-process-run", "preserves"),
        (PAYLOAD_INPUT_FILE, "transport-payload", "binary"),
        (REQUEST_INPUT_FILE, "transport-request-ref", "text"),
        (CLIENT_LOG_FILE, "diagnostic-log", "text"),
        (LISTENER_LOG_FILE, "diagnostic-log", "text"),
    ];
    let mut entries = Vec::with_capacity(definitions.len());
    for (relative_path, artifact_kind, format) in definitions {
        let path = run_directory.join(relative_path);
        ensure_regular_file(&path)?;
        let bytes = std::fs::read(&path).map_err(MoltenError::from)?;
        let expected_ref = if format == "preserves" {
            let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
            crate::preserves_rail::canonical_hash(&value)?
        } else if format == "binary" {
            crate::preserves_rail::content_ref_from_bytes(&bytes)
        } else if relative_path == REQUEST_INPUT_FILE {
            text_ref("molten.fabric.transport.distinct-process-request-input.v1", &String::from_utf8_lossy(&bytes))
        } else {
            text_ref("molten.fabric.transport.distinct-process-log.v1", &String::from_utf8_lossy(&bytes))
        };
        entries.push(IndexedArtifact {
            relative_path: relative_path.to_string(),
            artifact_kind: artifact_kind.to_string(),
            expected_ref,
            format: format.to_string(),
        });
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn render_index(entries: &[IndexedArtifact]) -> String {
    let mut output = String::from(INDEX_HEADER);
    output.push('\n');
    for entry in entries {
        output.push_str(&entry.relative_path);
        output.push('\t');
        output.push_str(&entry.artifact_kind);
        output.push('\t');
        output.push_str(&entry.expected_ref);
        output.push('\t');
        output.push_str(&entry.format);
        output.push('\n');
    }
    output
}

fn validate_run_membership(run_directory: &Path, require_companion: bool) -> Result<Vec<String>> {
    let mut expected = expected_members();
    if !require_companion {
        expected.remove(VERIFICATION_FILE);
    }
    let mut observed = BTreeSet::new();
    collect_relative_files(run_directory, run_directory, &mut observed)?;
    let mut diagnostics = Vec::new();
    for missing in expected.difference(&observed) {
        diagnostics.push(format!("missing-run-member:{missing}"));
    }
    for extra in observed.difference(&expected) {
        diagnostics.push(format!("unexpected-run-member:{extra}"));
    }
    for path in &expected {
        let absolute = run_directory.join(path);
        if absolute.exists() {
            ensure_regular_file(&absolute)?;
        }
    }
    diagnostics.sort();
    Ok(diagnostics)
}

fn expected_members() -> BTreeSet<String> {
    let members = [
        HANDOFF_FILE,
        LISTENER_START_FILE,
        CLIENT_START_FILE,
        LISTENER_TERMINAL_FILE,
        CLIENT_TERMINAL_FILE,
        CLEANUP_FILE,
        PARENT_RUN_FILE,
        VERIFICATION_FILE,
        INDEX_FILE,
        PAYLOAD_INPUT_FILE,
        REQUEST_INPUT_FILE,
        LISTENER_LOG_FILE,
        CLIENT_LOG_FILE,
    ];
    debug_assert_eq!(members.len(), EXPECTED_MEMBER_COUNT);
    members.into_iter().map(str::to_string).collect()
}

fn collect_relative_files(root: &Path, current: &Path, output: &mut BTreeSet<String>) -> Result<()> {
    if output.len() > MAX_RUN_FILES {
        return Err(MoltenError::invalid_harness("distinct-process run directory file count exceeds bound"));
    }
    for entry in std::fs::read_dir(current).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        let file_type = entry.file_type().map_err(MoltenError::from)?;
        if file_type.is_symlink() {
            return Err(MoltenError::invalid_harness("distinct-process run directory must not contain symlinks"));
        }
        if file_type.is_dir() {
            collect_relative_files(root, &entry.path(), output)?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| MoltenError::invalid_harness(format!("run path strip failed: {error}")))?
                .to_string_lossy()
                .into_owned();
            output.insert(relative);
        } else {
            return Err(MoltenError::invalid_harness("distinct-process run directory contains a non-regular entry"));
        }
    }
    Ok(())
}

fn ensure_regular_file(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(MoltenError::from)?;
    if !metadata.file_type().is_file() {
        return Err(MoltenError::invalid_harness(format!(
            "distinct-process artifact is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "distinct-process artifact exceeds {MAX_ARTIFACT_BYTES} bytes: {}",
            path.display()
        )));
    }
    Ok(())
}

fn invocation_ref(role: &str) -> String {
    text_ref(INVOCATION_DOMAIN, role)
}

fn command_profile_ref(role: &str) -> String {
    text_ref(COMMAND_PROFILE_DOMAIN, role)
}

fn text_ref(domain: &str, text: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(text.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(names: &[&str]) -> IOValue {
    crate::preserves_rail::record("checks", vec![strings_value(names.iter().copied())])
}

fn simple_record(value: &IOValue, label: &str, field_count: usize) -> Result<Vec<Value<IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(field_count))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(fields.iter().collect())
}

fn next<'a>(fields: &mut impl Iterator<Item = &'a Value<IOValue>>, label: &str) -> Result<&'a Value<IOValue>> {
    fields.next().ok_or_else(|| MoltenError::invalid_harness(format!("missing {label}")))
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = required_string(value, label)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
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

fn require_schema(value: &Value<IOValue>, expected: &str) -> Result<()> {
    let actual = required_string(value, "schema")?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("schema mismatch: expected {expected}, got {actual}")))
    }
}

fn require_decision(value: &Value<IOValue>) -> Result<()> {
    let decision = required_string(value, "decision")?;
    if decision == PASS_DECISION {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("participant decision must pass, got {decision}")))
    }
}

fn parse_role(value: &str) -> Result<EndpointParticipantRole> {
    match value {
        LISTENER_ROLE => Ok(EndpointParticipantRole::Listener),
        CLIENT_ROLE => Ok(EndpointParticipantRole::Client),
        other => Err(MoltenError::invalid_harness(format!("unsupported participant role {other}"))),
    }
}

fn parse_delivery(value: &str) -> Result<DeliveryOutcome> {
    match value {
        "not-attempted" => Ok(DeliveryOutcome::NotAttempted),
        "pending" => Ok(DeliveryOutcome::Pending),
        "delivered" => Ok(DeliveryOutcome::Delivered),
        "not-delivered" => Ok(DeliveryOutcome::NotDelivered),
        "uncertain" => Ok(DeliveryOutcome::Uncertain),
        other => Err(MoltenError::invalid_harness(format!("unsupported delivery outcome {other}"))),
    }
}

fn parse_retry(value: &str) -> Result<RetryDisposition> {
    match value {
        "not-applicable" => Ok(RetryDisposition::NotApplicable),
        "higher-level-policy-required" => Ok(RetryDisposition::HigherLevelPolicyRequired),
        "unsafe-without-reconciliation" => Ok(RetryDisposition::UnsafeWithoutReconciliation),
        other => Err(MoltenError::invalid_harness(format!("unsupported retry disposition {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_membership_remains_bounded() {
        assert_eq!(expected_members().len(), EXPECTED_MEMBER_COUNT);
    }
}
