//! The client side: live sends, ticket readdressing, and history reads.

use molten_core::outside_in::*;

use super::node::*;
use super::shell::*;

const LOOPBACK_PREFIX: &str = "127.0.0.1:";
const SEND_RECEIPT_RECORD: &str = "<node-control-live-send-receipt-v1";
const DECISION_FIELD: &str = "<decision \"";

/// The loopback address the bound ticket names.
pub(crate) fn ticket_address(ticket: &std::path::Path) -> StepResult<std::net::SocketAddr> {
    let text =
        std::fs::read_to_string(ticket).map_err(|_| ScenarioStop::StartupFailed("listener ticket".to_string()))?;
    let start = text
        .find(LOOPBACK_PREFIX)
        .ok_or_else(|| ScenarioStop::Unsupported("non-loopback-ticket".to_string()))?;
    let end = text[start..].find('"').map_or(text.len(), |offset| start + offset);
    text[start..end].parse().map_err(|_| ScenarioStop::Unsupported("ticket-address".to_string()))
}

/// A copy of the ticket that names another address for the same endpoint.
pub(crate) fn readdressed_ticket(
    ticket: &std::path::Path,
    address: std::net::SocketAddr,
) -> StepResult<std::path::PathBuf> {
    let original = ticket_address(ticket)?.to_string();
    let text =
        std::fs::read_to_string(ticket).map_err(|_| ScenarioStop::StartupFailed("listener ticket".to_string()))?;
    let copy = ticket.with_extension("forwarded.preserves");
    std::fs::write(&copy, text.replace(&original, &address.to_string()))
        .map_err(|_| ScenarioStop::Unsupported("forwarded-ticket".to_string()))?;
    Ok(copy)
}

/// Start a client send of `request` to the ticket's endpoint.
pub(crate) fn start_send(
    harness: &mut Harness,
    receiver: &Receiver,
    request: &Request,
    ticket: &std::path::Path,
    join_timeout_ms: u64,
) -> StepResult<ProcessHandle> {
    let sequence = request.sequence.to_string();
    let join_timeout = join_timeout_ms.to_string();
    harness.start(
        "client",
        &arguments(&[
            &"node",
            &"control-ingress-live-send",
            &request.path,
            &ticket,
            &"--from-peer",
            &CLIENT_PEER,
            &"--sequence",
            &sequence,
            &"--peer-bootstrap",
            &receiver.peer_ref,
            &"--authority",
            &receiver.authority_ref,
            &"--policy",
            &receiver.peer_ref,
            &"--resource",
            &receiver.authority_ref,
            &"--join-timeout-ms",
            &join_timeout,
            &"--max-attempts",
            &"1",
        ]),
    )
}

/// Reap a client send. A lost response is never read: its outcome stays unresolved.
pub(crate) fn finish_send(
    harness: &mut Harness,
    request: &Request,
    handle: ProcessHandle,
    fate: ResponseFate,
) -> StepResult<ClientAttempt> {
    let status = harness.wait(handle)?;
    let outcome = match fate {
        ResponseFate::Lost => ClientOutcome::Unresolved,
        ResponseFate::Observed if status.success() && send_decision(&harness.output(handle)) == Some("pass") => {
            ClientOutcome::Acknowledged
        }
        ResponseFate::Observed => ClientOutcome::Failed,
    };
    Ok(ClientAttempt {
        request_ref: request.label.clone(),
        fate,
        outcome,
    })
}

/// The decision of the send receipt the client printed. The client exits
/// successfully for a denied send, so the receipt decides the outcome.
fn send_decision(output: &str) -> Option<&str> {
    let receipt = &output[output.find(SEND_RECEIPT_RECORD)?..];
    let decision = &receipt[receipt.find(DECISION_FIELD)? + DECISION_FIELD.len()..];
    decision.split('"').next()
}

/// Read the receiver history through `node control-history`.
pub(crate) fn read_history(
    harness: &mut Harness,
    receiver: &Receiver,
    kind: CheckpointKind,
) -> StepResult<HistoryCheckpoint> {
    let output = match harness
        .run("control-history", &arguments(&[&"node", &"control-history", &"--state-root", &receiver.state_root]))
    {
        Ok(output) => Some(output),
        Err(ScenarioStop::StartupFailed(_)) => None,
        Err(stop) => return Err(stop),
    };
    Ok(HistoryCheckpoint {
        kind,
        receiver: output.and_then(|output| parse_history(&output, &receiver.labels)),
    })
}

fn parse_history(output: &str, labels: &std::collections::BTreeMap<String, String>) -> Option<ReceiverHistory> {
    let label = |request: &str| labels.get(request).cloned().unwrap_or_else(|| request.to_string());
    let mut history = ReceiverHistory {
        deliveries: Vec::new(),
        pending_requests: Vec::new(),
        processed: Vec::new(),
    };
    let mut lines = output.lines();
    let header = fields(lines.next()?.strip_prefix("node control-history ")?);
    for line in lines {
        let (kind, rest) = line.split_once(' ')?;
        let line_fields = fields(rest);
        match kind {
            "delivery" => history.deliveries.push(ObservedDelivery {
                envelope_ref: line_fields.get("envelope")?.clone(),
                request_ref: label(line_fields.get("request")?),
                is_accepted: line_fields.get("decision")? == "pass",
            }),
            "pending" => history.pending_requests.push(label(line_fields.get("request")?)),
            "processed" => history.processed.push(ObservedProcessed {
                request_ref: label(line_fields.get("request")?),
                is_passed: line_fields.get("decision")? == "pass",
            }),
            _ => return None,
        }
    }
    let counts = [
        (header.get("deliveries")?, history.deliveries.len()),
        (header.get("pending")?, history.pending_requests.len()),
        (header.get("processed")?, history.processed.len()),
    ];
    counts
        .iter()
        .all(|(declared, read)| declared.parse::<usize>().ok() == Some(*read))
        .then_some(history)
}

fn fields(text: &str) -> std::collections::BTreeMap<String, String> {
    text.split_whitespace()
        .filter_map(|field| field.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}
