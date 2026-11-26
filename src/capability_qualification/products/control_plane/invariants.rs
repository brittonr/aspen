//! Invariants shared by the failover scenarios over the replica receipts, and
//! the canonical committed-log and commit-quorum values they record.

/// Invariants of the fresh phase shared by every scenario; returns the commit
/// quorum the leader's own selected evidence ledger recorded.
pub(super) fn check_fresh(
    evidence: &mut super::super::Evidence,
    fresh: &super::lifecycle::Fresh,
) -> crate::error::Result<super::receipt::CommitQuorum> {
    let reported = fresh.replicas.iter().map(|replica| replica.process_id).collect::<std::collections::BTreeSet<_>>();
    evidence.check(
        "three-distinct-replica-processes",
        fresh.process_ids.len() == crate::fabric_consistency::raft::STATIC_VOTER_COUNT
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
            == crate::fabric_consistency::raft::STATIC_VOTER_COUNT,
    );
    evidence.check(
        "replicas-bind-one-static-membership",
        fresh.replicas.iter().all(|replica| {
            replica.voters.len() == crate::fabric_consistency::raft::STATIC_VOTER_COUNT
                && replica.voters.iter().all(|voter| super::fixture::NODES.contains(&voter.as_str()))
                && replica.membership_ref == fresh.replicas[0].membership_ref
                && replica.group_binding_ref == fresh.replicas[0].group_binding_ref
        }),
    );
    let leader = single_leader(&fresh.replicas);
    evidence.check("single-leader-elected", leader.is_ok());
    let quorum = leader?
        .commit_quorum
        .clone()
        .ok_or_else(|| super::super::super::invalid("live Raft leader recorded no commit quorum evidence"))?;
    evidence.check(
        "entry-committed-with-quorum",
        quorum.index == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
            && quorum.members.len() >= crate::fabric_consistency::raft::STATIC_QUORUM_COUNT,
    );
    evidence.check(
        "durable-log-written-by-every-process",
        fresh.replicas.iter().all(|replica| replica.durable_record_count > 0),
    );
    Ok(quorum)
}

pub(super) fn check_recovered_processes(
    evidence: &mut super::super::Evidence,
    fresh: &super::lifecycle::Fresh,
    recovered: &super::lifecycle::Recovered,
) {
    let reported = recovered
        .terminal
        .iter()
        .map(|replica| replica.process_id)
        .collect::<std::collections::BTreeSet<_>>();
    evidence.check(
        "three-distinct-recovered-processes",
        recovered.process_ids.len() == crate::fabric_consistency::raft::STATIC_VOTER_COUNT
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

pub(super) fn check_reelection(
    evidence: &mut super::super::Evidence,
    recovered: &super::lifecycle::Recovered,
    stale_term: u64,
) {
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

pub(super) fn single_leader(replicas: &[super::receipt::Replica]) -> crate::error::Result<&super::receipt::Replica> {
    let mut leaders = replicas.iter().filter(|replica| replica.role == super::LEADER_ROLE);
    match (leaders.next(), leaders.next()) {
        (Some(leader), None) => Ok(leader),
        _ => Err(super::super::super::invalid("live Raft replicas did not report exactly one leader")),
    }
}

pub(super) fn observed_quorum(
    leader: &super::receipt::Replica,
    recorded: &super::receipt::CommitQuorum,
) -> crate::fabric_consistency::raft::ReplicaQuorum {
    crate::fabric_consistency::raft::ReplicaQuorum {
        boundary: crate::fabric_consistency::raft::ReplicaQuorumBoundary::Commit,
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

pub(super) fn quorum_value(
    observed: &crate::fabric_consistency::raft::ReplicaQuorum,
    validated: &crate::fabric_consistency::raft::ValidatedReplicaQuorum,
) -> preserves::IOValue {
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

/// False quorum evidence built from the actually observed commit quorum: a
/// duplicated acknowledgement, an acknowledgement outside the admitted static
/// membership, and a minority of one.
pub(super) fn false_quorums(
    observed: &crate::fabric_consistency::raft::ReplicaQuorum,
) -> [(&'static str, crate::fabric_consistency::raft::ReplicaQuorum); 3] {
    let first = observed.acknowledgement_members.first().cloned().unwrap_or_default();
    let mut duplicate = observed.clone();
    duplicate.acknowledgement_members = vec![first.clone(), first.clone()];
    let mut outsider = observed.clone();
    outsider.acknowledgement_members.push(super::OUTSIDER.to_string());
    let mut minority = observed.clone();
    minority.acknowledgement_members = vec![first];
    [
        ("duplicate-acknowledgement-quorum-denied", duplicate),
        ("outsider-acknowledgement-quorum-denied", outsider),
        ("minority-acknowledgement-quorum-denied", minority),
    ]
}

/// The committed log every replica must hold: the fixed request at the first
/// index, in the term the fresh leader committed it.
pub(super) fn committed_log(term: u64) -> preserves::IOValue {
    super::replica::committed_log_value(&[crate::fabric_consistency::raft::ReplicatedEntry {
        index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        term,
        request_ref: super::replica::request_ref(),
        command_ref: super::replica::command_ref(),
        command_schema_ref: super::fixture::command_schema_ref(),
    }])
}
