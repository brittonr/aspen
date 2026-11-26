//! The four control-plane failover scenarios, each over one fresh replica
//! generation and, where the scenario needs it, its crashed and recovered
//! successor.

// r[impl molten.fabric_consistency.live_raft]
pub(super) fn failover_commit_survives(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::super::Evidence> {
    let mut evidence = super::super::Evidence::default();
    let mut fresh = super::lifecycle::fresh_phase(launcher, directory)?;
    let quorum = super::invariants::check_fresh(&mut evidence, &fresh)?;
    let expected_log_ref = crate::preserves_rail::canonical_hash(&super::invariants::committed_log(quorum.term))?;
    let expected_state_ref = crate::preserves_rail::canonical_hash(&super::replica::application_state_value(&[
        super::replica::request_ref(),
    ]))?;
    evidence.check(
        "commit-replicated-to-every-process",
        fresh.replicas.iter().all(|replica| {
            replica.commit_index == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
                && replica.committed_log_ref == expected_log_ref
        }),
    );
    evidence.check(
        "commit-applied-by-every-process",
        fresh.replicas.iter().all(|replica| replica.application_state_ref == expected_state_ref),
    );
    let recovered = super::lifecycle::crash_and_recover(launcher, directory, &mut fresh, quorum.term)?;
    evidence.check("every-replica-process-crashed-and-reaped", fresh.peers.iter().all(|peer| peer.id().is_none()));
    super::invariants::check_recovered_processes(&mut evidence, &fresh, &recovered);
    evidence.check(
        "every-replica-recovered-durable-commit",
        recovered.started.iter().all(|replica| {
            replica.recovery.as_ref().is_some_and(|recovery| {
                recovery.durable_commit_index == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
                    && recovery.replay_entry_count == 1
            }) && replica.commit_index == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
                && replica.last_applied == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
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
            replica.commit_index == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
                && replica.last_applied == crate::fabric_consistency::raft::INITIAL_LOG_INDEX
                && replica.committed_log_ref == expected_log_ref
                && replica.application_state_ref == expected_state_ref
        }),
    );
    super::invariants::check_reelection(&mut evidence, &recovered, quorum.term);
    evidence.runtime(super::invariants::committed_log(quorum.term));
    evidence.runtime(super::replica::application_state_value(&[super::replica::request_ref()]));
    evidence.runtime(recovery_value(&recovered));
    Ok(evidence)
}

/// The startup recovery every recovered replica reported, in node order.
fn recovery_value(recovered: &super::lifecycle::Recovered) -> preserves::IOValue {
    crate::preserves_rail::record(
        "qualification-live-raft-recovery-v1",
        recovered
            .started
            .iter()
            .map(|replica| {
                let recovery = replica.recovery.clone().unwrap_or(super::receipt::Recovery {
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
    )
}

// r[impl molten.fabric_consistency.live_raft]
pub(super) fn stale_leader_fenced(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::super::Evidence> {
    let mut evidence = super::super::Evidence::default();
    let mut fresh = super::lifecycle::fresh_phase(launcher, directory)?;
    let quorum = super::invariants::check_fresh(&mut evidence, &fresh)?;
    let recovered = super::lifecycle::crash_and_recover(launcher, directory, &mut fresh, quorum.term)?;
    super::invariants::check_recovered_processes(&mut evidence, &fresh, &recovered);
    super::invariants::check_reelection(&mut evidence, &recovered, quorum.term);
    let expected_log_ref = crate::preserves_rail::canonical_hash(&super::invariants::committed_log(quorum.term))?;
    for probe in &recovered.probes {
        evidence.check(probe_check(&probe.probe)?, probe_fenced(probe));
    }
    let rejected = |name: super::receipt::Probe| {
        recovered.rejections.iter().any(|rejection| {
            rejection.probe == name.as_str() && rejection.is_rejected && rejection.is_responder_term_newer
        })
    };
    evidence.check("stale-leader-process-received-append-rejection", rejected(super::receipt::Probe::TermAppend));
    evidence.check("stale-leader-process-received-vote-refusal", rejected(super::receipt::Probe::TermVote));
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
fn probe_fenced(probe: &super::receipt::ProbeOutcome) -> bool {
    let expected_outcome = if probe.probe == super::receipt::Probe::ServiceGeneration.as_str()
        || probe.probe == super::receipt::Probe::FencingEpoch.as_str()
    {
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
    super::receipt::Probe::ALL
        .into_iter()
        .find(|candidate| candidate.as_str() == probe)
        .map(|probe| match probe {
            super::receipt::Probe::TermAppend => "stale-term-append-refused-by-new-leader",
            super::receipt::Probe::TermVote => "stale-term-vote-refused-by-new-leader",
            super::receipt::Probe::ServiceGeneration => "stale-service-generation-denied",
            super::receipt::Probe::FencingEpoch => "stale-fencing-epoch-denied",
        })
        .ok_or_else(|| super::super::super::invalid("unknown live Raft stale probe"))
}

// r[impl molten.fabric_consistency.final_validation]
pub(super) fn final_quorum_evidence(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::super::Evidence> {
    let mut evidence = super::super::Evidence::default();
    let fresh = super::lifecycle::fresh_phase(launcher, directory)?;
    let recorded = super::invariants::check_fresh(&mut evidence, &fresh)?;
    let leader = super::invariants::single_leader(&fresh.replicas)?;
    let observed = super::invariants::observed_quorum(leader, &recorded);
    let validated = crate::fabric_consistency::raft::validate_replica_quorum(&observed)?;
    evidence.check("validated-ref-matches-leader-ledger", validated.evidence_ref == recorded.evidence_ref);
    evidence.check(
        "acknowledgements-are-distinct-admitted-majority",
        validated.acknowledgement_members.len() >= crate::fabric_consistency::raft::STATIC_QUORUM_COUNT
            && validated
                .acknowledgement_members
                .iter()
                .all(|member| super::fixture::NODES.contains(&member.as_str())),
    );
    let expected_log_ref = crate::preserves_rail::canonical_hash(&super::invariants::committed_log(recorded.term))?;
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
    evidence.runtime(super::invariants::quorum_value(&observed, &validated));
    Ok(evidence)
}

// r[impl molten.fabric_consistency.final_validation]
pub(super) fn false_quorum_denied(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<super::super::Evidence> {
    let mut evidence = super::super::Evidence::default();
    let fresh = super::lifecycle::fresh_phase(launcher, directory)?;
    let recorded = super::invariants::check_fresh(&mut evidence, &fresh)?;
    let leader = super::invariants::single_leader(&fresh.replicas)?;
    let observed = super::invariants::observed_quorum(leader, &recorded);
    evidence.check(
        "observed-quorum-validates",
        crate::fabric_consistency::raft::validate_replica_quorum(&observed).is_ok(),
    );
    for (name, tampered) in super::invariants::false_quorums(&observed) {
        let result = crate::fabric_consistency::raft::validate_replica_quorum(&tampered);
        evidence.check(name, result.is_err());
        if let Err(error) = &result {
            evidence.runtime(crate::preserves_rail::record("qualification-live-raft-false-quorum-v1", vec![
                crate::preserves_rail::string(name),
                super::super::error_class(error),
            ]));
        }
    }
    Ok(evidence)
}
