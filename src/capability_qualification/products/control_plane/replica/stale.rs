//! The stale frames the crashed leader's recovered process presents to the
//! re-elected leader, how that leader handles each, and the answers the stale
//! process receives.

use crate::fabric_consistency::raft::ReplicaTransportEffects as _;

pub(super) async fn send_probes(node: &mut super::Node, stale_term: u64) -> crate::error::Result<()> {
    let state = node.service.state();
    let append = crate::fabric_consistency::raft::Message::AppendEntries {
        term: stale_term,
        leader_id: super::super::fixture::NODE_A.to_string(),
        prev_log_index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        prev_log_term: stale_term,
        entries: Vec::new(),
        leader_commit: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    };
    let vote = crate::fabric_consistency::raft::Message::RequestVote {
        term: stale_term,
        candidate_id: super::super::fixture::NODE_A.to_string(),
        last_log_index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        last_log_term: stale_term,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    };
    let stale_fencing = crate::fabric_consistency::raft::Message::AppendEntries {
        term: stale_term,
        leader_id: super::super::fixture::NODE_A.to_string(),
        prev_log_index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        prev_log_term: stale_term,
        entries: Vec::new(),
        leader_commit: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch - 1,
    };
    let group_binding_ref = state.profile.group_binding_ref.clone();
    let generation = state.profile.service_generation;
    let envelopes = [
        (generation, append.clone()),
        (generation, vote),
        (generation - 1, append),
        (generation, stale_fencing),
    ];
    for (service_generation, message) in envelopes {
        let envelope = crate::fabric_consistency::raft::ReplicaMessageEnvelope {
            group_binding_ref: group_binding_ref.clone(),
            service_generation,
            from: super::super::fixture::NODE_A.to_string(),
            to: super::super::fixture::NODE_B.to_string(),
            message,
        };
        node.service.ports_mut().transport.send(&envelope).await?;
    }
    Ok(())
}

/// Classify a frame from the crashed leader's process at the re-elected leader.
/// The recovered node A never campaigns nor leads, so every append or vote
/// request it sends to node B is one of the stale probes.
pub(in crate::capability_qualification::products::control_plane) fn classify(
    event: &crate::fabric_consistency::raft::ReplicaEvent,
) -> Option<super::super::receipt::Probe> {
    let crate::fabric_consistency::raft::ReplicaEvent::Message { envelope } = event else {
        return None;
    };
    if envelope.from != super::super::fixture::NODE_A {
        return None;
    }
    let is_append = matches!(envelope.message, crate::fabric_consistency::raft::Message::AppendEntries { .. });
    let is_vote = matches!(envelope.message, crate::fabric_consistency::raft::Message::RequestVote { .. });
    if !is_append && !is_vote {
        return None;
    }
    Some(if envelope.service_generation != super::super::fixture::SERVICE_GENERATION {
        super::super::receipt::Probe::ServiceGeneration
    } else if envelope.message.fencing_epoch() != super::super::fixture::FENCING_EPOCH {
        super::super::receipt::Probe::FencingEpoch
    } else if is_vote {
        super::super::receipt::Probe::TermVote
    } else {
        super::super::receipt::Probe::TermAppend
    })
}

pub(super) async fn handle_probe(
    node: &mut super::Node,
    run_directory: &std::path::Path,
    probe: super::super::receipt::Probe,
    event: crate::fabric_consistency::raft::ReplicaEvent,
) -> crate::error::Result<()> {
    let before = node.service.state().clone();
    let outcome = node.service.handle_event(event).await;
    let after = node.service.state();
    let class = match outcome {
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => "applied",
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { .. } => "denied",
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(_) => "failed",
    };
    let recorded = super::super::receipt::ProbeOutcome {
        probe: probe.as_str().to_string(),
        outcome: class.to_string(),
        leader_retained: before.role == crate::fabric_consistency::raft::ReplicaRole::Leader
            && after.role == crate::fabric_consistency::raft::ReplicaRole::Leader,
        term_retained: after.current_term == before.current_term,
        commit_retained: after.commit_index == before.commit_index,
        log_retained: after.log == before.log,
    };
    super::super::super::peer::write_value(&super::super::receipt::probe_file(run_directory, probe), &recorded.value())
}

/// At the stale leader's process: the re-elected leader's answer to a stale probe.
pub(super) fn record_rejection(
    run_directory: &std::path::Path,
    event: &crate::fabric_consistency::raft::ReplicaEvent,
    stale_term: u64,
) -> crate::error::Result<()> {
    let crate::fabric_consistency::raft::ReplicaEvent::Message { envelope } = event else {
        return Ok(());
    };
    if envelope.from != super::super::fixture::NODE_B {
        return Ok(());
    }
    let (probe, is_rejected) = match &envelope.message {
        crate::fabric_consistency::raft::Message::AppendResponse { success, .. } => {
            (super::super::receipt::Probe::TermAppend, !success)
        }
        crate::fabric_consistency::raft::Message::VoteResponse { granted, .. } => {
            (super::super::receipt::Probe::TermVote, !granted)
        }
        _ => return Ok(()),
    };
    let output = super::super::receipt::rejection_file(run_directory, probe);
    if output.is_file() {
        return Ok(());
    }
    let rejection = super::super::receipt::Rejection {
        probe: probe.as_str().to_string(),
        is_rejected,
        is_responder_term_newer: envelope.message.term() > stale_term,
    };
    super::super::super::peer::write_value(&output, &rejection.value())
}
