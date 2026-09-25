use std::io::Write;

use preserves::ValueImpl;

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
    pub run_directory: std::path::PathBuf,
    pub process_binary: std::path::PathBuf,
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
    pub run_directory: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessTransportVerification {
    pub decision: String,
    pub parent_ref: String,
    pub verification_ref: String,
    pub diagnostics: Vec<String>,
    pub value: preserves::IOValue,
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
    value: preserves::IOValue,
    artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartArtifact {
    role: EndpointParticipantRole,
    invocation_ref: String,
    command_profile_ref: String,
    parent_observed: bool,
    value: preserves::IOValue,
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
    value: preserves::IOValue,
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
    child: std::process::Child,
    finished: bool,
}

impl ReapingChild {
    fn spawn(
        binary: &std::path::Path,
        role_command: &str,
        run_directory: &std::path::Path,
        log_path: &std::path::Path,
    ) -> crate::error::Result<Self> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
        }
        let stdout = std::fs::File::create(log_path).map_err(crate::error::MoltenError::from)?;
        let stderr = stdout.try_clone().map_err(crate::error::MoltenError::from)?;
        let child = std::process::Command::new(binary)
            .args(["cluster", role_command, "--run-dir"])
            .arg(run_directory)
            .stdout(std::process::Stdio::from(stdout))
            .stderr(std::process::Stdio::from(stderr))
            .spawn()
            .map_err(crate::error::MoltenError::from)?;
        Ok(Self { child, finished: false })
    }

    fn id(&self) -> u32 {
        self.child.id()
    }

    fn try_wait(&mut self) -> crate::error::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait().map_err(crate::error::MoltenError::from)
    }

    fn wait_bounded(
        &mut self,
        timeout: std::time::Duration,
        label: &str,
    ) -> crate::error::Result<std::process::ExitStatus> {
        let mut deadline = crate::fabric_time::SupervisionDeadline::after(timeout)?;
        loop {
            if let Some(status) = self.try_wait()? {
                self.finished = true;
                return Ok(status);
            }
            if deadline.is_expired()? {
                let _kill = self.child.kill();
                let _wait = self.child.wait();
                self.finished = true;
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "distinct-process {label} child timed out"
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(CHILD_POLL_INTERVAL_MS));
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
) -> crate::error::Result<DistinctProcessTransportRun> {
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
