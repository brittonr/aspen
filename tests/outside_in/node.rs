//! The receiver node, driven through public entry points only.
//!
//! The receiver is an ordinary node state root; every step is a `molten`
//! command. Request identities in observations are the logical labels the
//! scenario declared; a request reference the scenario did not build stays a
//! raw reference, so unexpected processing remains visible.

use super::shell::*;

const RECEIVER_NODE: &str = "node:outside-in-receiver";
pub(crate) const CLIENT_PEER: &str = "node:outside-in-client";
const LISTENER_MAX_EVENTS: &str = "64";
const LISTENER_EVENT_TIMEOUT_MS: &str = "15000";
const BOUND_LINE: &str = "node serve live-iroh bound endpoint=";

pub(crate) fn arguments(parts: &[&dyn AsRef<std::ffi::OsStr>]) -> Vec<std::ffi::OsString> {
    parts.iter().map(|part| part.as_ref().to_os_string()).collect()
}

/// The reference a command printed as `<label> <ref> written to <path>`.
fn written_ref(output: &str, label: &str) -> StepResult<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(label)?.split_whitespace().next().map(ToString::to_string))
        .ok_or_else(|| ScenarioStop::StartupFailed(format!("{label} reference")))
}

pub(crate) struct Receiver {
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) artifacts: std::path::PathBuf,
    pub(crate) peer_ref: String,
    pub(crate) authority_ref: String,
    pub(crate) labels: std::collections::BTreeMap<String, String>,
    listener_runs: u32,
}

pub(crate) struct Request {
    pub(crate) path: std::path::PathBuf,
    pub(crate) label: String,
    pub(crate) sequence: u64,
}

pub(crate) struct Listener {
    handle: ProcessHandle,
    pub(crate) ticket: std::path::PathBuf,
    pub(crate) endpoint: String,
}

/// Initialize and start the receiver node, export its ticket, and admit the client peer.
pub(crate) fn prepare_receiver(harness: &mut Harness) -> StepResult<Receiver> {
    let state_root = harness.scratch_dir("receiver-state")?;
    let artifacts = harness.scratch_dir("client-artifacts")?;
    harness.run(
        "init",
        &arguments(&[
            &"node",
            &"init",
            &"--state-root",
            &state_root,
            &"--node-id",
            &RECEIVER_NODE,
        ]),
    )?;
    harness.run("run", &arguments(&[&"node", &"run", &"--state-root", &state_root]))?;
    let export = artifacts.join("ticket-export.preserves");
    harness.run(
        "ticket-export",
        &arguments(&[
            &"node",
            &"live-ticket-export",
            &"--state-root",
            &state_root,
            &"--out",
            &export,
        ]),
    )?;
    let admission = artifacts.join("admission.preserves");
    let admitted = harness.run(
        "peer-admit",
        &arguments(&[
            &"node",
            &"live-peer-admit",
            &"--state-root",
            &state_root,
            &"--peer",
            &CLIENT_PEER,
            &"--receipt-out",
            &admission,
            &export,
        ]),
    )?;
    let grant = artifacts.join("grant.preserves");
    let granted = harness.run(
        "authority-grant",
        &arguments(&[
            &"node",
            &"authority-grant-fixture",
            &"--state-root",
            &state_root,
            &"--peer",
            &CLIENT_PEER,
            &"--node",
            &RECEIVER_NODE,
            &"--operation",
            &"status",
            &"--out",
            &grant,
        ]),
    )?;
    Ok(Receiver {
        peer_ref: written_ref(&admitted, "node live peer admission")?,
        authority_ref: written_ref(&granted, "node authority grant")?,
        state_root,
        artifacts,
        labels: std::collections::BTreeMap::new(),
        listener_runs: 0,
    })
}

/// Build a distinct status request. The ordinal selects its payload and sequence.
pub(crate) fn build_request(harness: &mut Harness, receiver: &mut Receiver, ordinal: u64) -> StepResult<Request> {
    let label = format!("request-{ordinal}");
    let path = receiver.artifacts.join(format!("{label}.preserves"));
    let (authority, peer) = (receiver.authority_ref.clone(), receiver.peer_ref.clone());
    let mut parts = arguments(&[
        &"node",
        &"control-request",
        &"--operation",
        &"status",
        &"--authority",
        &authority,
        &"--policy",
        &peer,
        &"--resource",
        &authority,
        &"--out",
        &path,
    ]);
    match ordinal {
        1 => {}
        2 => parts.extend(arguments(&[&"--payload", &authority])),
        _ => parts.extend(arguments(&[&"--payload", &peer])),
    }
    let output = harness.run("control-request", &parts)?;
    receiver.labels.insert(written_ref(&output, "node control request")?, label.clone());
    Ok(Request {
        path,
        label,
        sequence: ordinal,
    })
}

/// Start a live listener on the receiver's storage and wait for its bound ticket.
pub(crate) fn start_listener(harness: &mut Harness, receiver: &mut Receiver) -> StepResult<Listener> {
    receiver.listener_runs += 1;
    let ticket = receiver.artifacts.join(format!("ticket-{}.preserves", receiver.listener_runs));
    start_listener_at(harness, &receiver.state_root, ticket)
}

/// Start a live listener on a state root and wait until it publishes its bound ticket.
pub(crate) fn start_listener_at(
    harness: &mut Harness,
    state_root: &std::path::Path,
    ticket: std::path::PathBuf,
) -> StepResult<Listener> {
    let handle = harness.start(
        "listener",
        &arguments(&[
            &"node",
            &"serve",
            &"--state-root",
            &state_root,
            &"--live-iroh",
            &"--live-max-events",
            &LISTENER_MAX_EVENTS,
            &"--live-event-timeout-ms",
            &LISTENER_EVENT_TIMEOUT_MS,
            &"--max-requests-per-tick",
            &"1",
            &"--live-ticket-out",
            &ticket,
        ]),
    )?;
    harness.await_ready(handle, |output| output.contains(BOUND_LINE) && ticket.exists())?;
    let endpoint = harness
        .output(handle)
        .lines()
        .find_map(|line| line.strip_prefix(BOUND_LINE).map(ToString::to_string))
        .ok_or_else(|| ScenarioStop::StartupFailed("listener endpoint".to_string()))?;
    Ok(Listener {
        handle,
        ticket,
        endpoint,
    })
}

/// Overwrite every processed-request receipt with bytes no reader accepts.
pub(crate) fn corrupt_processed_receipts(receiver: &Receiver) -> StepResult<()> {
    let outbox = receiver.state_root.join("control").join("outbox");
    let entries = std::fs::read_dir(&outbox).map_err(|_| ScenarioStop::Unsupported("outbox-listing".to_string()))?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().ends_with(".control-receipt.preserves") {
            std::fs::write(entry.path(), b"<not-a-control-receipt")
                .map_err(|_| ScenarioStop::Unsupported("outbox-corruption".to_string()))?;
        }
    }
    Ok(())
}

pub(crate) fn listener_handle(listener: &Listener) -> ProcessHandle {
    listener.handle
}

/// Wait for the listener to drain and exit on its own.
pub(crate) fn finish_listener(harness: &mut Harness, listener: &Listener) -> StepResult<()> {
    harness.wait(listener.handle).map(|_| ())
}
