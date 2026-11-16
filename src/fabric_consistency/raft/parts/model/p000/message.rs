#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    RecoveryQuery {
        term: u64,
        requester_id: String,
        round_ref: String,
        from_index: u64,
        is_barrier: bool,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    RecoveryReport {
        term: u64,
        peer_id: String,
        round_ref: String,
        observation_ref: String,
        from_index: u64,
        last_log_index: u64,
        commit_index: u64,
        entries: Vec<ReplicatedEntry>,
        snapshot: Option<Box<ReplicaSnapshot>>,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    RequestVote {
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    VoteResponse {
        term: u64,
        voter_id: String,
        granted: bool,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    AppendEntries {
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<ReplicatedEntry>,
        leader_commit: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    AppendResponse {
        term: u64,
        follower_id: String,
        success: bool,
        request_prev_log_index: u64,
        match_index: u64,
        conflict_index: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    ReadProbe {
        term: u64,
        leader_id: String,
        request_ref: String,
        required_index: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    ReadAcknowledgement {
        term: u64,
        follower_id: String,
        request_ref: String,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    InstallSnapshot {
        term: u64,
        leader_id: String,
        snapshot: Box<ReplicaSnapshot>,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    SnapshotResponse {
        term: u64,
        follower_id: String,
        snapshot_index: u64,
        accepted: bool,
        config_epoch: u64,
        fencing_epoch: u64,
    },
}

impl Message {
    pub const fn term(&self) -> u64 {
        match self {
            Self::RequestVote { term, .. }
            | Self::RecoveryQuery { term, .. }
            | Self::RecoveryReport { term, .. }
            | Self::VoteResponse { term, .. }
            | Self::AppendEntries { term, .. }
            | Self::AppendResponse { term, .. }
            | Self::ReadProbe { term, .. }
            | Self::ReadAcknowledgement { term, .. }
            | Self::InstallSnapshot { term, .. }
            | Self::SnapshotResponse { term, .. } => *term,
        }
    }

    pub const fn config_epoch(&self) -> u64 {
        match self {
            Self::RequestVote { config_epoch, .. }
            | Self::RecoveryQuery { config_epoch, .. }
            | Self::RecoveryReport { config_epoch, .. }
            | Self::VoteResponse { config_epoch, .. }
            | Self::AppendEntries { config_epoch, .. }
            | Self::AppendResponse { config_epoch, .. }
            | Self::ReadProbe { config_epoch, .. }
            | Self::ReadAcknowledgement { config_epoch, .. }
            | Self::InstallSnapshot { config_epoch, .. }
            | Self::SnapshotResponse { config_epoch, .. } => *config_epoch,
        }
    }

    pub const fn fencing_epoch(&self) -> u64 {
        match self {
            Self::RequestVote { fencing_epoch, .. }
            | Self::RecoveryQuery { fencing_epoch, .. }
            | Self::RecoveryReport { fencing_epoch, .. }
            | Self::VoteResponse { fencing_epoch, .. }
            | Self::AppendEntries { fencing_epoch, .. }
            | Self::AppendResponse { fencing_epoch, .. }
            | Self::ReadProbe { fencing_epoch, .. }
            | Self::ReadAcknowledgement { fencing_epoch, .. }
            | Self::InstallSnapshot { fencing_epoch, .. }
            | Self::SnapshotResponse { fencing_epoch, .. } => *fencing_epoch,
        }
    }
}