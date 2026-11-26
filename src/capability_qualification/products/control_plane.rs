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
mod receipt;
mod replica;
#[cfg(test)]
mod tests;

use self::receipt::Probe;
use crate::fabric_consistency::raft::INITIAL_LOG_INDEX;
use crate::fabric_consistency::raft::ReplicaQuorum;
use crate::fabric_consistency::raft::ReplicaQuorumBoundary;
use crate::fabric_consistency::raft::ReplicatedEntry;
use crate::fabric_consistency::raft::STATIC_QUORUM_COUNT;
use crate::fabric_consistency::raft::STATIC_VOTER_COUNT;
use crate::fabric_consistency::raft::ValidatedReplicaQuorum;

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
    launcher: &super::Launcher,
) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::RaftFailoverCommitSurvives => {
            with_run_directory("raft-failover", |directory| failover_commit_survives(launcher, directory))
        }
        super::super::Scenario::RaftStaleLeaderFenced => {
            with_run_directory("raft-stale-leader", |directory| stale_leader_fenced(launcher, directory))
        }
        super::super::Scenario::RaftFinalQuorumEvidence => {
            with_run_directory("raft-final-quorum", |directory| final_quorum_evidence(launcher, directory))
        }
        super::super::Scenario::RaftFalseQuorumDenied => {
            with_run_directory("raft-false-quorum", |directory| false_quorum_denied(launcher, directory))
        }
        _ => Err(super::super::invalid("scenario is not a control-plane failover scenario")),
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

/// The fresh phase as the three replica processes reported it at the checkpoint.
struct Fresh {
    peers: Vec<super::peer::Peer>,
    process_ids: std::collections::BTreeSet<u64>,
    replicas: Vec<receipt::Replica>,
}

/// The recovered phase: per-replica startup recovery and terminal receipts.
struct Recovered {
    process_ids: std::collections::BTreeSet<u64>,
    started: Vec<receipt::Replica>,
    terminal: Vec<receipt::Replica>,
    probes: Vec<receipt::ProbeOutcome>,
    rejections: Vec<receipt::Rejection>,
}

// r[impl molten.fabric_consistency.live_raft]
fn fresh_phase(launcher: &super::Launcher, directory: &std::path::Path) -> crate::error::Result<Fresh> {
    let mut peers = Vec::with_capacity(STATIC_VOTER_COUNT);
    for node_id in fixture::NODES {
        peers.push(launcher.spawn(&format!("{ROLE_PREFIX}{FRESH_ROLE}{node_id}"), directory)?);
    }
    for node_id in fixture::NODES {
        super::peer::wait_value(&receipt::node_file(directory, node_id, "ready"), PHASE_TIMEOUT)?;
    }
    signal(directory, receipt::START_FILE, "start")?;
    super::peer::wait_value(&directory.join(receipt::LEADER_DONE_FILE), PHASE_TIMEOUT)?;
    signal(directory, receipt::CHECKPOINT_FILE, "checkpoint")?;
    let replicas = read_replicas(directory, "checkpoint")?;
    let process_ids = peers.iter().filter_map(super::peer::Peer::id).map(u64::from).collect();
    Ok(Fresh {
        peers,
        process_ids,
        replicas,
    })
}

/// Crash every replica process abruptly, then restart all three from their
/// durable stores and drive reconciliation, re-election and the stale probes.
// r[impl molten.fabric_consistency.live_raft]
fn crash_and_recover(
    launcher: &super::Launcher,
    directory: &std::path::Path,
    fresh: &mut Fresh,
    stale_term: u64,
) -> crate::error::Result<Recovered> {
    for peer in &mut fresh.peers {
        peer.kill()?;
    }
    clear_fresh_signals(directory)?;
    super::peer::write_value(&directory.join(receipt::STALE_LEADER_FILE), &receipt::term_value(stale_term))?;
    let mut peers = Vec::with_capacity(STATIC_VOTER_COUNT);
    for node_id in fixture::NODES {
        peers.push(launcher.spawn(&format!("{ROLE_PREFIX}{RECOVER_ROLE}{node_id}"), directory)?);
    }
    let started = read_replicas(directory, "recovered")?;
    for node_id in fixture::NODES {
        super::peer::wait_value(&receipt::node_file(directory, node_id, "ready"), PHASE_TIMEOUT)?;
    }
    signal(directory, receipt::RESTART_FILE, "restart")?;
    let mut probes = Vec::with_capacity(Probe::ALL.len());
    for probe in Probe::ALL {
        probes.push(receipt::ProbeOutcome::parse(&super::peer::wait_value(
            &receipt::probe_file(directory, probe),
            PHASE_TIMEOUT,
        )?)?);
    }
    let mut rejections = Vec::with_capacity(2);
    for probe in [Probe::TermAppend, Probe::TermVote] {
        rejections.push(receipt::Rejection::parse(&super::peer::wait_value(
            &receipt::rejection_file(directory, probe),
            PHASE_TIMEOUT,
        )?)?);
    }
    signal(directory, receipt::STOP_FILE, "stop")?;
    let terminal = read_replicas(directory, "terminal")?;
    let process_ids = peers.iter().filter_map(super::peer::Peer::id).map(u64::from).collect();
    for peer in &mut peers {
        peer.wait_success(EXIT_TIMEOUT)?;
    }
    Ok(Recovered {
        process_ids,
        started,
        terminal,
        probes,
        rejections,
    })
}

// r[impl molten.fabric_consistency.live_raft]
fn failover_commit_survives(
    launcher: &super::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut fresh = fresh_phase(launcher, directory)?;
    let quorum = check_fresh(&mut evidence, &fresh)?;
    let expected_log_ref = crate::preserves_rail::canonical_hash(&committed_log(quorum.term))?;
    let expected_state_ref =
        crate::preserves_rail::canonical_hash(&replica::application_state_value(&[replica::request_ref()]))?;
    evidence.check(
        "commit-replicated-to-every-process",
        fresh
            .replicas
            .iter()
            .all(|replica| replica.commit_index == INITIAL_LOG_INDEX && replica.committed_log_ref == expected_log_ref),
    );
    evidence.check(
        "commit-applied-by-every-process",
        fresh.replicas.iter().all(|replica| replica.application_state_ref == expected_state_ref),
    );
    let recovered = crash_and_recover(launcher, directory, &mut fresh, quorum.term)?;
    evidence.check("every-replica-process-crashed-and-reaped", fresh.peers.iter().all(|peer| peer.id().is_none()));
    check_recovered_processes(&mut evidence, &fresh, &recovered);
    evidence.check(
        "every-replica-recovered-durable-commit",
        recovered.started.iter().all(|replica| {
            replica.recovery.as_ref().is_some_and(|recovery| {
                recovery.durable_commit_index == INITIAL_LOG_INDEX && recovery.replay_entry_count == 1
            }) && replica.commit_index == INITIAL_LOG_INDEX
                && replica.last_applied == INITIAL_LOG_INDEX
                && replica.committed_log_ref == expected_log_ref
        }),
    );
    evidence.check(
        "recovered-replicas-reapplied-committed-state",
        recovered.started.iter().all(|replica| replica.application_state_ref == expected_state_ref),
    );
    evidence.check(
        "committed-entry-survives-reelection",
        recovered.terminal.iter().all(|replica| {
            replica.commit_index == INITIAL_LOG_INDEX
                && replica.last_applied == INITIAL_LOG_INDEX
                && replica.committed_log_ref == expected_log_ref
                && replica.application_state_ref == expected_state_ref
        }),
    );
    check_reelection(&mut evidence, &recovered, quorum.term);
    evidence.runtime(committed_log(quorum.term));
    evidence.runtime(replica::application_state_value(&[replica::request_ref()]));
    evidence.runtime(crate::preserves_rail::record(
        "qualification-live-raft-recovery-v1",
        recovered
            .started
            .iter()
            .map(|replica| {
                let recovery = replica.recovery.clone().unwrap_or(receipt::Recovery {
                    recovery_ref: String::new(),
                    durable_commit_index: 0,
                    replay_entry_count: 0,
                });
                crate::preserves_rail::record("replica-recovery", vec![
                    crate::preserves_rail::string(&replica.node_id),
                    crate::preserves_rail::u64_value(recovery.durable_commit_index),
                    crate::preserves_rail::u64_value(recovery.replay_entry_count),
                    crate::preserves_rail::string(&recovery.recovery_ref),
                ])
            })
            .collect(),
    ));
    Ok(evidence)
}

// r[impl molten.fabric_consistency.live_raft]
fn stale_leader_fenced(
    launcher: &super::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut fresh = fresh_phase(launcher, directory)?;
    let quorum = check_fresh(&mut evidence, &fresh)?;
    let recovered = crash_and_recover(launcher, directory, &mut fresh, quorum.term)?;
    check_recovered_processes(&mut evidence, &fresh, &recovered);
    check_reelection(&mut evidence, &recovered, quorum.term);
    let expected_log_ref = crate::preserves_rail::canonical_hash(&committed_log(quorum.term))?;
    for probe in &recovered.probes {
        evidence.check(probe_check(&probe.probe)?, probe_fenced(probe));
    }
    let rejected = |name: Probe| {
        recovered.rejections.iter().any(|rejection| {
            rejection.probe == name.as_str() && rejection.is_rejected && rejection.is_responder_term_newer
        })
    };
    evidence.check("stale-leader-process-received-append-rejection", rejected(Probe::TermAppend));
    evidence.check("stale-leader-process-received-vote-refusal", rejected(Probe::TermVote));
    evidence.check(
        "committed-state-unchanged-by-stale-leader",
        recovered.terminal.iter().all(|replica| replica.committed_log_ref == expected_log_ref),
    );
    for probe in &recovered.probes {
        evidence.runtime(probe.value());
    }
    for rejection in &recovered.rejections {
        evidence.runtime(rejection.value());
    }
    Ok(evidence)
}

/// The fail-closed fencing each stale probe must meet at the re-elected leader:
/// a stale term is handled by the protocol and refused, a stale service
/// generation or fencing epoch is denied before any transition; either way the
/// leader keeps its role, term, commit index and log.
fn probe_fenced(probe: &receipt::ProbeOutcome) -> bool {
    let expected_outcome =
        if probe.probe == Probe::ServiceGeneration.as_str() || probe.probe == Probe::FencingEpoch.as_str() {
            "denied"
        } else {
            "applied"
        };
    probe.outcome == expected_outcome
        && probe.leader_retained
        && probe.term_retained
        && probe.commit_retained
        && probe.log_retained
}

fn probe_check(probe: &str) -> crate::error::Result<&'static str> {
    Probe::ALL
        .into_iter()
        .find(|candidate| candidate.as_str() == probe)
        .map(|probe| match probe {
            Probe::TermAppend => "stale-term-append-refused-by-new-leader",
            Probe::TermVote => "stale-term-vote-refused-by-new-leader",
            Probe::ServiceGeneration => "stale-service-generation-denied",
            Probe::FencingEpoch => "stale-fencing-epoch-denied",
        })
        .ok_or_else(|| super::super::invalid("unknown live Raft stale probe"))
}

// r[impl molten.fabric_consistency.final_validation]
fn final_quorum_evidence(
    launcher: &super::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let fresh = fresh_phase(launcher, directory)?;
    let recorded = check_fresh(&mut evidence, &fresh)?;
    let leader = single_leader(&fresh.replicas)?;
    let observed = observed_quorum(leader, &recorded);
    let validated = crate::fabric_consistency::raft::validate_replica_quorum(&observed)?;
    evidence.check("validated-ref-matches-leader-ledger", validated.evidence_ref == recorded.evidence_ref);
    evidence.check(
        "acknowledgements-are-distinct-admitted-majority",
        validated.acknowledgement_members.len() >= STATIC_QUORUM_COUNT
            && validated.acknowledgement_members.iter().all(|member| fixture::NODES.contains(&member.as_str())),
    );
    let expected_log_ref = crate::preserves_rail::canonical_hash(&committed_log(recorded.term))?;
    let acknowledgers = fresh
        .replicas
        .iter()
        .filter(|replica| validated.acknowledgement_members.contains(&replica.node_id))
        .collect::<Vec<_>>();
    evidence.check(
        "acknowledgements-come-from-distinct-processes",
        acknowledgers.len() == validated.acknowledgement_members.len()
            && acknowledgers
                .iter()
                .map(|replica| replica.process_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == acknowledgers.len()
            && acknowledgers.iter().all(|replica| fresh.process_ids.contains(&replica.process_id)),
    );
    evidence.check(
        "acknowledgers-hold-the-committed-entry",
        acknowledgers.iter().all(|replica| replica.committed_log_ref == expected_log_ref),
    );
    evidence.runtime(quorum_value(&observed, &validated));
    Ok(evidence)
}

// r[impl molten.fabric_consistency.final_validation]
fn false_quorum_denied(
    launcher: &super::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let fresh = fresh_phase(launcher, directory)?;
    let recorded = check_fresh(&mut evidence, &fresh)?;
    let leader = single_leader(&fresh.replicas)?;
    let observed = observed_quorum(leader, &recorded);
    evidence.check(
        "observed-quorum-validates",
        crate::fabric_consistency::raft::validate_replica_quorum(&observed).is_ok(),
    );
    for (name, tampered) in false_quorums(&observed) {
        let result = crate::fabric_consistency::raft::validate_replica_quorum(&tampered);
        evidence.check(name, result.is_err());
        if let Err(error) = &result {
            evidence.runtime(crate::preserves_rail::record("qualification-live-raft-false-quorum-v1", vec![
                crate::preserves_rail::string(name),
                super::error_class(error),
            ]));
        }
    }
    Ok(evidence)
}

/// False quorum evidence built from the actually observed commit quorum: a
/// duplicated acknowledgement, an acknowledgement outside the admitted static
/// membership, and a minority of one.
pub(super) fn false_quorums(observed: &ReplicaQuorum) -> [(&'static str, ReplicaQuorum); 3] {
    let first = observed.acknowledgement_members.first().cloned().unwrap_or_default();
    let mut duplicate = observed.clone();
    duplicate.acknowledgement_members = vec![first.clone(), first.clone()];
    let mut outsider = observed.clone();
    outsider.acknowledgement_members.push(OUTSIDER.to_string());
    let mut minority = observed.clone();
    minority.acknowledgement_members = vec![first];
    [
        ("duplicate-acknowledgement-quorum-denied", duplicate),
        ("outsider-acknowledgement-quorum-denied", outsider),
        ("minority-acknowledgement-quorum-denied", minority),
    ]
}

/// Invariants of the fresh phase shared by every scenario; returns the commit
/// quorum the leader's own selected evidence ledger recorded.
fn check_fresh(evidence: &mut super::Evidence, fresh: &Fresh) -> crate::error::Result<receipt::CommitQuorum> {
    let reported = fresh.replicas.iter().map(|replica| replica.process_id).collect::<std::collections::BTreeSet<_>>();
    evidence.check(
        "three-distinct-replica-processes",
        fresh.process_ids.len() == STATIC_VOTER_COUNT
            && reported == fresh.process_ids
            && !fresh.process_ids.contains(&u64::from(std::process::id())),
    );
    evidence.check(
        "distinct-replica-endpoint-identities",
        fresh
            .replicas
            .iter()
            .map(|replica| replica.endpoint_identity.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == STATIC_VOTER_COUNT,
    );
    evidence.check(
        "replicas-bind-one-static-membership",
        fresh.replicas.iter().all(|replica| {
            replica.voters.len() == STATIC_VOTER_COUNT
                && replica.voters.iter().all(|voter| fixture::NODES.contains(&voter.as_str()))
                && replica.membership_ref == fresh.replicas[0].membership_ref
                && replica.group_binding_ref == fresh.replicas[0].group_binding_ref
        }),
    );
    let leader = single_leader(&fresh.replicas);
    evidence.check("single-leader-elected", leader.is_ok());
    let quorum = leader?
        .commit_quorum
        .clone()
        .ok_or_else(|| super::super::invalid("live Raft leader recorded no commit quorum evidence"))?;
    evidence.check(
        "entry-committed-with-quorum",
        quorum.index == INITIAL_LOG_INDEX && quorum.members.len() >= STATIC_QUORUM_COUNT,
    );
    evidence.check(
        "durable-log-written-by-every-process",
        fresh.replicas.iter().all(|replica| replica.durable_record_count > 0),
    );
    Ok(quorum)
}

fn check_recovered_processes(evidence: &mut super::Evidence, fresh: &Fresh, recovered: &Recovered) {
    let reported = recovered
        .terminal
        .iter()
        .map(|replica| replica.process_id)
        .collect::<std::collections::BTreeSet<_>>();
    evidence.check(
        "three-distinct-recovered-processes",
        recovered.process_ids.len() == STATIC_VOTER_COUNT
            && reported == recovered.process_ids
            && recovered.started.iter().all(|replica| recovered.process_ids.contains(&replica.process_id)),
    );
    evidence.check(
        "recovered-endpoint-identities-stable",
        fresh.replicas.iter().zip(&recovered.terminal).all(|(before, after)| {
            before.node_id == after.node_id && before.endpoint_identity == after.endpoint_identity
        }),
    );
    evidence.check(
        "every-replica-restarted-from-durable-state",
        recovered.started.iter().all(|replica| {
            replica
                .recovery
                .as_ref()
                .is_some_and(|recovery| crate::preserves_rail::validate_content_ref(&recovery.recovery_ref).is_ok())
        }),
    );
}

fn check_reelection(evidence: &mut super::Evidence, recovered: &Recovered, stale_term: u64) {
    let leader = single_leader(&recovered.terminal);
    evidence.check("single-leader-reelected-after-recovery", leader.is_ok());
    evidence.check(
        "reelection-advanced-past-crashed-leader-term",
        leader.is_ok_and(|leader| {
            leader.term > stale_term && recovered.terminal.iter().all(|replica| replica.term == leader.term)
        }),
    );
    evidence.check(
        "every-recovered-replica-regained-voting-admission",
        recovered.terminal.iter().all(|replica| replica.is_voter),
    );
}

fn single_leader(replicas: &[receipt::Replica]) -> crate::error::Result<&receipt::Replica> {
    let mut leaders = replicas.iter().filter(|replica| replica.role == LEADER_ROLE);
    match (leaders.next(), leaders.next()) {
        (Some(leader), None) => Ok(leader),
        _ => Err(super::super::invalid("live Raft replicas did not report exactly one leader")),
    }
}

fn observed_quorum(leader: &receipt::Replica, recorded: &receipt::CommitQuorum) -> ReplicaQuorum {
    ReplicaQuorum {
        boundary: ReplicaQuorumBoundary::Commit,
        group_binding_ref: leader.group_binding_ref.clone(),
        membership_ref: leader.membership_ref.clone(),
        config_epoch: leader.config_epoch,
        term: recorded.term,
        index: recorded.index,
        admitted_voters: leader.voters.clone(),
        acknowledgement_members: recorded.members.clone(),
        source_ref: recorded.source_ref.clone(),
    }
}

fn quorum_value(observed: &ReplicaQuorum, validated: &ValidatedReplicaQuorum) -> preserves::IOValue {
    let members = |values: &[String]| {
        let mut sorted = values.to_vec();
        sorted.sort();
        crate::preserves_rail::sequence(sorted.iter().map(crate::preserves_rail::string).collect())
    };
    crate::preserves_rail::record("qualification-live-raft-commit-quorum-v1", vec![
        crate::preserves_rail::string(observed.boundary.as_str()),
        crate::preserves_rail::string(&observed.group_binding_ref),
        crate::preserves_rail::string(&observed.membership_ref),
        crate::preserves_rail::u64_value(observed.config_epoch),
        crate::preserves_rail::u64_value(observed.term),
        crate::preserves_rail::u64_value(observed.index),
        members(&observed.admitted_voters),
        members(&validated.acknowledgement_members),
        crate::preserves_rail::string(&observed.source_ref),
        crate::preserves_rail::string(&validated.evidence_ref),
    ])
}

/// The committed log every replica must hold: the fixed request at the first
/// index, in the term the fresh leader committed it.
fn committed_log(term: u64) -> preserves::IOValue {
    replica::committed_log_value(&[ReplicatedEntry {
        index: INITIAL_LOG_INDEX,
        term,
        request_ref: replica::request_ref(),
        command_ref: replica::command_ref(),
        command_schema_ref: fixture::command_schema_ref(),
    }])
}

fn read_replicas(directory: &std::path::Path, phase: &str) -> crate::error::Result<Vec<receipt::Replica>> {
    fixture::NODES
        .iter()
        .map(|node_id| {
            let replica = receipt::Replica::parse(&super::peer::wait_value(
                &receipt::node_file(directory, node_id, phase),
                PHASE_TIMEOUT,
            )?)?;
            if replica.node_id != *node_id {
                return Err(super::super::invalid("live Raft replica receipt names another replica"));
            }
            Ok(replica)
        })
        .collect()
}

fn signal(directory: &std::path::Path, file: &str, label: &str) -> crate::error::Result<()> {
    super::peer::write_value(&directory.join(file), &receipt::signal(label))
}

/// Remove the crashed generation's signals and endpoints so the recovered
/// processes exchange only their own fresh endpoints and readiness.
fn clear_fresh_signals(directory: &std::path::Path) -> crate::error::Result<()> {
    let mut paths = vec![
        directory.join(receipt::START_FILE),
        directory.join(receipt::CATCH_UP_FILE),
        directory.join(receipt::LEADER_DONE_FILE),
        directory.join(receipt::CHECKPOINT_FILE),
    ];
    for node_id in fixture::NODES {
        for phase in ["endpoint", "ready", "checkpoint"] {
            paths.push(receipt::node_file(directory, node_id, phase));
        }
    }
    for path in paths {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(crate::error::Failure::from(error)),
        }
    }
    Ok(())
}
