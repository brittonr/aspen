use super::*;

pub(super) fn dispatch(transition: &mut MessagePlan, from: String, message: Message) -> crate::error::Result<()> {
    match message {
        message @ (Message::RecoveryQuery { .. } | Message::RecoveryReport { .. }) => {
            rejoin::dispatch(transition, from, message)
        }
        message @ (Message::RequestVote { .. } | Message::VoteResponse { .. }) => {
            dispatch_election(transition, from, message)
        }
        message @ (Message::AppendEntries { .. } | Message::AppendResponse { .. }) => {
            dispatch_replication(transition, from, message)
        }
        message @ (Message::ReadProbe { .. } | Message::ReadAcknowledgement { .. }) => {
            dispatch_read(transition, from, message)
        }
        message @ (Message::InstallSnapshot { .. } | Message::SnapshotResponse { .. }) => {
            dispatch_snapshot(transition, from, message)
        }
    }
}

fn dispatch_election(transition: &mut MessagePlan, from: String, message: Message) -> crate::error::Result<()> {
    match message {
        Message::RequestVote {
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
        Message::VoteResponse {
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
        Message::AppendEntries { .. }
        | Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::AppendResponse { .. }
        | Message::ReadProbe { .. }
        | Message::ReadAcknowledgement { .. }
        | Message::InstallSnapshot { .. }
        | Message::SnapshotResponse { .. } => {
            Err(crate::error::Failure::invalid_harness("election dispatch admitted a non-election message"))
        }
    }
}

fn dispatch_replication(transition: &mut MessagePlan, from: String, message: Message) -> crate::error::Result<()> {
    match message {
        Message::AppendEntries {
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
        Message::AppendResponse {
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
        Message::RequestVote { .. }
        | Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::VoteResponse { .. }
        | Message::ReadProbe { .. }
        | Message::ReadAcknowledgement { .. }
        | Message::InstallSnapshot { .. }
        | Message::SnapshotResponse { .. } => {
            Err(crate::error::Failure::invalid_harness("replication dispatch admitted a non-replication message"))
        }
    }
}

fn dispatch_snapshot(transition: &mut MessagePlan, from: String, message: Message) -> crate::error::Result<()> {
    match message {
        Message::InstallSnapshot {
            term,
            leader_id,
            snapshot: installed,
            ..
        } => snapshot::handle_install(transition, snapshot::InstallInput {
            from,
            term,
            leader_id,
            snapshot: *installed,
        }),
        Message::SnapshotResponse {
            term,
            follower_id,
            snapshot_index,
            accepted,
            ..
        } => snapshot::handle_response(transition, snapshot::ResponseInput {
            from,
            term,
            follower_id,
            snapshot_index,
            is_accepted: accepted,
        }),
        Message::RequestVote { .. }
        | Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::VoteResponse { .. }
        | Message::AppendEntries { .. }
        | Message::AppendResponse { .. }
        | Message::ReadProbe { .. }
        | Message::ReadAcknowledgement { .. } => {
            Err(crate::error::Failure::invalid_harness("snapshot dispatch admitted a non-snapshot message"))
        }
    }
}

fn dispatch_read(transition: &mut MessagePlan, from: String, message: Message) -> crate::error::Result<()> {
    match message {
        Message::ReadProbe {
            term,
            leader_id,
            request_ref,
            required_index,
            ..
        } => read::handle_probe(transition, read::ProbeInput {
            from,
            term,
            leader_id,
            request_ref,
            required_index,
        }),
        Message::ReadAcknowledgement {
            term,
            follower_id,
            request_ref,
            ..
        } => read::handle_acknowledgement(transition, read::AcknowledgementInput {
            from,
            term,
            follower_id,
            request_ref,
        }),
        Message::RequestVote { .. }
        | Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::VoteResponse { .. }
        | Message::AppendEntries { .. }
        | Message::AppendResponse { .. }
        | Message::InstallSnapshot { .. }
        | Message::SnapshotResponse { .. } => {
            Err(crate::error::Failure::invalid_harness("read dispatch admitted a non-read message"))
        }
    }
}
