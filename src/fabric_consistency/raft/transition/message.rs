use super::*;
use crate::error::Result;

pub(super) fn dispatch(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        message @ (RaftMessage::RequestVote { .. } | RaftMessage::VoteResponse { .. }) => {
            dispatch_election(transition, from, message)
        }
        message @ (RaftMessage::AppendEntries { .. } | RaftMessage::AppendResponse { .. }) => {
            dispatch_replication(transition, from, message)
        }
        message @ (RaftMessage::ReadProbe { .. } | RaftMessage::ReadAcknowledgement { .. }) => {
            dispatch_read(transition, from, message)
        }
        message @ (RaftMessage::InstallSnapshot { .. } | RaftMessage::SnapshotResponse { .. }) => {
            dispatch_snapshot(transition, from, message)
        }
    }
}

fn dispatch_election(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::RequestVote {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
            ..
        } => election::handle_request_vote(transition, election::VoteRequestInput {
            from,
            term,
            candidate_id,
            last_log_index,
            last_log_term,
        }),
        RaftMessage::VoteResponse {
            term,
            voter_id,
            granted,
            ..
        } => election::handle_vote_response(transition, election::VoteResponseInput {
            from,
            term,
            voter_id,
            is_granted: granted,
        }),
        _ => unreachable!("election dispatch admitted a non-election message"),
    }
}

fn dispatch_replication(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::AppendEntries {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
            ..
        } => replication::handle_append_entries(transition, replication::AppendEntriesInput {
            from,
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }),
        RaftMessage::AppendResponse {
            term,
            follower_id,
            success,
            request_prev_log_index,
            match_index,
            conflict_index,
            ..
        } => replication::handle_append_response(transition, replication::AppendResponseInput {
            from,
            term,
            follower_id,
            is_success: success,
            request_prev_log_index,
            match_index,
            conflict_index,
        }),
        _ => unreachable!("replication dispatch admitted a non-replication message"),
    }
}

fn dispatch_snapshot(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::InstallSnapshot {
            term,
            leader_id,
            snapshot: installed,
            ..
        } => snapshot::handle_install_snapshot(transition, snapshot::InstallSnapshotInput {
            from,
            term,
            leader_id,
            snapshot: *installed,
        }),
        RaftMessage::SnapshotResponse {
            term,
            follower_id,
            snapshot_index,
            accepted,
            ..
        } => snapshot::handle_snapshot_response(transition, snapshot::SnapshotResponseInput {
            from,
            term,
            follower_id,
            snapshot_index,
            is_accepted: accepted,
        }),
        _ => unreachable!("snapshot dispatch admitted a non-snapshot message"),
    }
}

fn dispatch_read(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::ReadProbe {
            term,
            leader_id,
            request_ref,
            required_index,
            ..
        } => read::handle_read_probe(transition, read::ReadProbeInput {
            from,
            term,
            leader_id,
            request_ref,
            required_index,
        }),
        RaftMessage::ReadAcknowledgement {
            term,
            follower_id,
            request_ref,
            ..
        } => read::handle_read_acknowledgement(transition, read::ReadAcknowledgementInput {
            from,
            term,
            follower_id,
            request_ref,
        }),
        _ => unreachable!("read dispatch admitted a non-read message"),
    }
}
