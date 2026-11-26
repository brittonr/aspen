//! Control-plane failover over three distinct live Raft replica processes.
//!
//! Every scenario starts three peer processes of the executing binary. Each
//! peer assembles the production scoped live replica over the real Iroh
//! transport and a capability-rooted Redb durable log under one fixed group.
//! The fixed leader commits one request with a quorum acknowledgement, every
//! process is crashed (SIGKILL) and restarted from the same durable state, the
//! recovered replicas reconcile and re-elect, and the crashed leader's
//! recovered process presents stale frames to the new leader. Scratch paths,
//! ports, process ids, endpoint identities and post-recovery terms are reduced
//! to invariants; the observation records only deterministic canonical values
//! (the committed log, application state, recovery receipts, stale-frame
//! outcomes and the production-validated commit quorum evidence).

mod fixture;
mod invariants;
mod lifecycle;
mod receipt;
mod replica;
mod scenarios;
#[cfg(test)]
mod tests;

const ROLE_PREFIX: &str = "raft-";
const FRESH_ROLE: &str = "fresh-";
const RECOVER_ROLE: &str = "recover-";
const LEADER_ROLE: &str = "leader";
/// Bound of every producer wait on a replica phase.
const PHASE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);
const EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// An acknowledgement member outside the admitted static membership.
const OUTSIDER: &str = "node-outsider";

pub(super) fn run(
    scenario: super::super::Scenario,
    launcher: &super::peer::Launcher,
) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::RaftFailoverCommitSurvives => {
            with_run_directory("raft-failover", |directory| scenarios::failover_commit_survives(launcher, directory))
        }
        super::super::Scenario::RaftStaleLeaderFenced => {
            with_run_directory("raft-stale-leader", |directory| scenarios::stale_leader_fenced(launcher, directory))
        }
        super::super::Scenario::RaftFinalQuorumEvidence => {
            with_run_directory("raft-final-quorum", |directory| scenarios::final_quorum_evidence(launcher, directory))
        }
        super::super::Scenario::RaftFalseQuorumDenied => {
            with_run_directory("raft-false-quorum", |directory| scenarios::false_quorum_denied(launcher, directory))
        }
        super::super::Scenario::GrantedSend
        | super::super::Scenario::DeniedSend
        | super::super::Scenario::DurableRestartRecovery
        | super::super::Scenario::DurableCorruptSnapshotDenied
        | super::super::Scenario::ExecutionUnknownAfterStart
        | super::super::Scenario::ExecutionPreStartDefinite
        | super::super::Scenario::NativeWorkUnknownAfterEffect
        | super::super::Scenario::NativeUncertainWorkNotReadmitted
        | super::super::Scenario::TransportEndpointImport
        | super::super::Scenario::TransportEndpointTamperDenied
        | super::super::Scenario::TransportSessionAcknowledged
        | super::super::Scenario::TransportSessionLostAcknowledgement
        | super::super::Scenario::WorldReplayAdmitted
        | super::super::Scenario::WorldReplayAuthorityDenied
        | super::super::Scenario::WorldRestoreCurrentAdmission
        | super::super::Scenario::WorldRestoreStaleAdmissionDenied
        | super::super::Scenario::WorldActivationCurrentFacts
        | super::super::Scenario::WorldActivationRevokedDenied => {
            Err(super::super::invalid("scenario is not a control-plane failover scenario"))
        }
    }
}

/// Peer entry for `raft-fresh-<node>` and `raft-recover-<node>` roles.
pub(super) fn peer(role: &str, run_directory: &std::path::Path) -> Option<crate::error::Result<()>> {
    let role = role.strip_prefix(ROLE_PREFIX)?;
    let (mode, node) = if let Some(node) = role.strip_prefix(FRESH_ROLE) {
        (replica::Mode::Fresh, node)
    } else if let Some(node) = role.strip_prefix(RECOVER_ROLE) {
        (replica::Mode::Recover, node)
    } else {
        return Some(Err(super::super::invalid("unsupported live Raft replica peer mode")));
    };
    let Some(node_id) = fixture::NODES.into_iter().find(|candidate| *candidate == node) else {
        return Some(Err(super::super::invalid("live Raft replica peer is outside static membership")));
    };
    Some(replica::run(node_id, mode, run_directory))
}

fn with_run_directory(
    label: &str,
    scenario: impl FnOnce(&std::path::Path) -> crate::error::Result<super::Evidence>,
) -> crate::error::Result<super::Evidence> {
    let scratch = super::scratch(label)?;
    scenario(scratch.path()).map_err(|error| diagnose(error, scratch.path()))
}

#[cfg(test)]
fn diagnose(error: crate::error::Failure, directory: &std::path::Path) -> crate::error::Failure {
    tests::with_peer_logs(error, directory)
}

#[cfg(not(test))]
const fn diagnose(error: crate::error::Failure, _directory: &std::path::Path) -> crate::error::Failure {
    error
}
