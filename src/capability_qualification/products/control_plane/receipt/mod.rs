//! Canonical receipts a replica peer process publishes in its run directory,
//! and the phase-signal file names the producer and peers share. Receipts
//! carry process ids and endpoint identities only so the producer can prove
//! distinct-process execution; those never enter a product observation.

mod decode;

const REPLICA_SCHEMA: &str = "qualification-live-raft-replica-v1";
const REPLICA_FIELD_COUNT: usize = 24;
const PROBE_FIELD_COUNT: usize = 6;
const REJECTION_FIELD_COUNT: usize = 3;

pub(super) const START_FILE: &str = "start.preserves";
pub(super) const CATCH_UP_FILE: &str = "catch-up.preserves";
pub(super) const LEADER_DONE_FILE: &str = "leader-done.preserves";
pub(super) const CHECKPOINT_FILE: &str = "checkpoint.preserves";
pub(super) const STALE_LEADER_FILE: &str = "stale-leader.preserves";
pub(super) const RESTART_FILE: &str = "restart.preserves";
pub(super) const RECOVERED_LEADER_FILE: &str = "recovered-leader.preserves";
pub(super) const STOP_FILE: &str = "stop.preserves";

/// The stale frames the crashed leader's recovered process presents to the
/// re-elected leader: an old term (append and vote request), and an old term
/// additionally carrying a stale service generation or fencing epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Probe {
    TermAppend,
    TermVote,
    ServiceGeneration,
    FencingEpoch,
}

impl Probe {
    pub(super) const ALL: [Self; 4] = [
        Self::TermAppend,
        Self::TermVote,
        Self::ServiceGeneration,
        Self::FencingEpoch,
    ];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::TermAppend => "stale-term-append",
            Self::TermVote => "stale-term-vote",
            Self::ServiceGeneration => "stale-service-generation",
            Self::FencingEpoch => "stale-fencing-epoch",
        }
    }
}

pub(super) fn node_file(run_directory: &std::path::Path, node_id: &str, phase: &str) -> std::path::PathBuf {
    run_directory.join(format!("{node_id}-{phase}.preserves"))
}

pub(super) fn probe_file(run_directory: &std::path::Path, probe: Probe) -> std::path::PathBuf {
    run_directory.join(format!("probe-{}.preserves", probe.as_str()))
}

pub(super) fn rejection_file(run_directory: &std::path::Path, probe: Probe) -> std::path::PathBuf {
    run_directory.join(format!("rejection-{}.preserves", probe.as_str()))
}

pub(super) fn signal(label: &str) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-live-raft-signal-v1", vec![crate::preserves_rail::string(label)])
}

/// The commit acknowledgements the leader's own selected evidence ledger recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommitQuorum {
    pub(super) term: u64,
    pub(super) index: u64,
    pub(super) source_ref: String,
    pub(super) evidence_ref: String,
    pub(super) members: Vec<String>,
}

/// Startup recovery of one replica from its durable store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Recovery {
    pub(super) recovery_ref: String,
    pub(super) durable_commit_index: u64,
    pub(super) replay_entry_count: u64,
}

/// One replica's state as its own process observed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Replica {
    pub(super) node_id: String,
    pub(super) process_id: u64,
    pub(super) endpoint_identity: String,
    pub(super) role: String,
    pub(super) is_voter: bool,
    pub(super) term: u64,
    pub(super) commit_index: u64,
    pub(super) last_applied: u64,
    pub(super) committed_log_ref: String,
    pub(super) applied_request_refs: Vec<String>,
    pub(super) application_state_ref: String,
    pub(super) group_binding_ref: String,
    pub(super) membership_ref: String,
    pub(super) config_epoch: u64,
    pub(super) voters: Vec<String>,
    pub(super) durable_record_count: u64,
    pub(super) commit_quorum: Option<CommitQuorum>,
    pub(super) recovery: Option<Recovery>,
}

impl Replica {
    pub(super) fn value(&self) -> preserves::IOValue {
        let quorum = self.commit_quorum.clone().unwrap_or(CommitQuorum {
            term: 0,
            index: 0,
            source_ref: String::new(),
            evidence_ref: String::new(),
            members: Vec::new(),
        });
        let recovery = self.recovery.clone().unwrap_or(Recovery {
            recovery_ref: String::new(),
            durable_commit_index: 0,
            replay_entry_count: 0,
        });
        crate::preserves_rail::record(REPLICA_SCHEMA, vec![
            crate::preserves_rail::string(&self.node_id),
            crate::preserves_rail::u64_value(self.process_id),
            crate::preserves_rail::string(&self.endpoint_identity),
            crate::preserves_rail::string(&self.role),
            crate::preserves_rail::bool_value(self.is_voter),
            crate::preserves_rail::u64_value(self.term),
            crate::preserves_rail::u64_value(self.commit_index),
            crate::preserves_rail::u64_value(self.last_applied),
            crate::preserves_rail::string(&self.committed_log_ref),
            strings(&self.applied_request_refs),
            crate::preserves_rail::string(&self.application_state_ref),
            crate::preserves_rail::string(&self.group_binding_ref),
            crate::preserves_rail::string(&self.membership_ref),
            crate::preserves_rail::u64_value(self.config_epoch),
            strings(&self.voters),
            crate::preserves_rail::u64_value(self.durable_record_count),
            crate::preserves_rail::bool_value(self.commit_quorum.is_some()),
            crate::preserves_rail::u64_value(quorum.term),
            crate::preserves_rail::u64_value(quorum.index),
            crate::preserves_rail::string(&quorum.source_ref),
            crate::preserves_rail::string(&quorum.evidence_ref),
            strings(&quorum.members),
            crate::preserves_rail::bool_value(self.recovery.is_some()),
            crate::preserves_rail::record("recovery", vec![
                crate::preserves_rail::string(&recovery.recovery_ref),
                crate::preserves_rail::u64_value(recovery.durable_commit_index),
                crate::preserves_rail::u64_value(recovery.replay_entry_count),
            ]),
        ])
    }

    pub(super) fn parse(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let fields = decode::fields(value, REPLICA_SCHEMA, REPLICA_FIELD_COUNT)?;
        let has_quorum = decode::flag(&fields[16])?;
        let has_recovery = decode::flag(&fields[22])?;
        let recovery = decode::fields_of(&fields[23], "recovery", 3)?;
        Ok(Self {
            node_id: decode::text(&fields[0])?,
            process_id: decode::number(&fields[1])?,
            endpoint_identity: decode::text(&fields[2])?,
            role: decode::text(&fields[3])?,
            is_voter: decode::flag(&fields[4])?,
            term: decode::number(&fields[5])?,
            commit_index: decode::number(&fields[6])?,
            last_applied: decode::number(&fields[7])?,
            committed_log_ref: decode::text(&fields[8])?,
            applied_request_refs: decode::texts(&fields[9])?,
            application_state_ref: decode::text(&fields[10])?,
            group_binding_ref: decode::text(&fields[11])?,
            membership_ref: decode::text(&fields[12])?,
            config_epoch: decode::number(&fields[13])?,
            voters: decode::texts(&fields[14])?,
            durable_record_count: decode::number(&fields[15])?,
            commit_quorum: if has_quorum {
                Some(CommitQuorum {
                    term: decode::number(&fields[17])?,
                    index: decode::number(&fields[18])?,
                    source_ref: decode::text(&fields[19])?,
                    evidence_ref: decode::text(&fields[20])?,
                    members: decode::texts(&fields[21])?,
                })
            } else {
                None
            },
            recovery: if has_recovery {
                Some(Recovery {
                    recovery_ref: decode::text(&recovery[0])?,
                    durable_commit_index: decode::number(&recovery[1])?,
                    replay_entry_count: decode::number(&recovery[2])?,
                })
            } else {
                None
            },
        })
    }
}

pub(super) fn rejoin_is_voter(rejoin: &crate::fabric_consistency::raft::ReplicaRejoin) -> bool {
    matches!(rejoin, crate::fabric_consistency::raft::ReplicaRejoin::Voter)
}

/// How the re-elected leader's actual service handled one stale probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProbeOutcome {
    pub(super) probe: String,
    pub(super) outcome: String,
    pub(super) leader_retained: bool,
    pub(super) term_retained: bool,
    pub(super) commit_retained: bool,
    pub(super) log_retained: bool,
}

impl ProbeOutcome {
    pub(super) fn value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("qualification-live-raft-stale-probe-v1", vec![
            crate::preserves_rail::string(&self.probe),
            crate::preserves_rail::string(&self.outcome),
            crate::preserves_rail::bool_value(self.leader_retained),
            crate::preserves_rail::bool_value(self.term_retained),
            crate::preserves_rail::bool_value(self.commit_retained),
            crate::preserves_rail::bool_value(self.log_retained),
        ])
    }

    pub(super) fn parse(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let fields = decode::fields(value, "qualification-live-raft-stale-probe-v1", PROBE_FIELD_COUNT)?;
        Ok(Self {
            probe: decode::text(&fields[0])?,
            outcome: decode::text(&fields[1])?,
            leader_retained: decode::flag(&fields[2])?,
            term_retained: decode::flag(&fields[3])?,
            commit_retained: decode::flag(&fields[4])?,
            log_retained: decode::flag(&fields[5])?,
        })
    }
}

/// The response the stale leader's process received from the re-elected leader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Rejection {
    pub(super) probe: String,
    pub(super) is_rejected: bool,
    pub(super) is_responder_term_newer: bool,
}

impl Rejection {
    pub(super) fn value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("qualification-live-raft-stale-rejection-v1", vec![
            crate::preserves_rail::string(&self.probe),
            crate::preserves_rail::bool_value(self.is_rejected),
            crate::preserves_rail::bool_value(self.is_responder_term_newer),
        ])
    }

    pub(super) fn parse(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let fields = decode::fields(value, "qualification-live-raft-stale-rejection-v1", REJECTION_FIELD_COUNT)?;
        Ok(Self {
            probe: decode::text(&fields[0])?,
            is_rejected: decode::flag(&fields[1])?,
            is_responder_term_newer: decode::flag(&fields[2])?,
        })
    }
}

pub(super) fn term_value(term: u64) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-live-raft-stale-leader-v1", vec![crate::preserves_rail::u64_value(
        term,
    )])
}

pub(super) fn parse_term(value: &preserves::IOValue) -> crate::error::Result<u64> {
    let fields = decode::fields(value, "qualification-live-raft-stale-leader-v1", 1)?;
    decode::number(&fields[0])
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}
