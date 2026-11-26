const LOG_TAIL_BYTES: usize = 4_096;

/// Failure diagnostics for unit tests only: the tail of every peer log.
pub(super) fn with_peer_logs(error: crate::error::Failure, directory: &std::path::Path) -> crate::error::Failure {
    let mut logs = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "log"))
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let tail = text.char_indices().rev().nth(LOG_TAIL_BYTES).map_or(text.as_str(), |(index, _)| &text[index..]);
            format!("{}:\n{tail}", path.file_name().unwrap_or_default().to_string_lossy())
        })
        .collect::<Vec<_>>();
    logs.sort();
    crate::error::Failure::invalid_harness(format!("{error}\n{}", logs.join("\n")))
}

/// One actual run must reproduce the runtime-artifact ref that earlier
/// independent executions checked into `tests/evidence-matrix.ncl`.
fn assert_deterministic_pass(scenario: crate::capability_qualification::Scenario) {
    crate::capability_qualification::matrix::assert_scenario_bound(scenario);
}

// r[verify molten.fabric_consistency.live_raft]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn failover_commit_survives_distinct_process_crash_and_reelection() {
    assert_deterministic_pass(crate::capability_qualification::Scenario::RaftFailoverCommitSurvives);
}

// r[verify molten.fabric_consistency.live_raft]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn stale_leader_is_fenced_after_recovery() {
    assert_deterministic_pass(crate::capability_qualification::Scenario::RaftStaleLeaderFenced);
}

// r[verify molten.fabric_consistency.final_validation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn observed_commit_quorum_validates_into_canonical_evidence() {
    assert_deterministic_pass(crate::capability_qualification::Scenario::RaftFinalQuorumEvidence);
}

// r[verify molten.fabric_consistency.final_validation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn false_commit_quorums_are_denied() {
    assert_deterministic_pass(crate::capability_qualification::Scenario::RaftFalseQuorumDenied);
}

fn quorum() -> crate::fabric_consistency::raft::ReplicaQuorum {
    let group = super::fixture::group().expect("qualification group");
    crate::fabric_consistency::raft::ReplicaQuorum {
        boundary: crate::fabric_consistency::raft::ReplicaQuorumBoundary::Commit,
        group_binding_ref: group.binding_ref,
        membership_ref: group.membership_ref,
        config_epoch: group.config_epoch,
        term: 1,
        index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
        admitted_voters: super::fixture::NODES.iter().map(ToString::to_string).collect(),
        acknowledgement_members: vec![super::fixture::NODE_A.to_string(), super::fixture::NODE_B.to_string()],
        source_ref: super::fixture::reference("negative-control-commit-source"),
    }
}

/// Negative control: each tamper alone flips the production validator from
/// admit to deny, so the denial checks would not hold without the tamper.
// r[verify molten.fabric_consistency.final_validation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn false_quorum_denial_is_caused_by_each_tamper() {
    let observed = quorum();
    crate::fabric_consistency::raft::validate_replica_quorum(&observed).expect("untampered quorum validates");
    for (name, tampered) in super::invariants::false_quorums(&observed) {
        assert!(crate::fabric_consistency::raft::validate_replica_quorum(&tampered).is_err(), "{name} must deny");
    }
    let mut outsider_majority = observed.clone();
    outsider_majority.acknowledgement_members = vec![super::fixture::NODE_A.to_string(), "node-outsider".to_string()];
    assert!(crate::fabric_consistency::raft::validate_replica_quorum(&outsider_majority).is_err());
}

fn probe_envelope(
    service_generation: u64,
    fencing_epoch: u64,
) -> crate::fabric_consistency::raft::ReplicaMessageEnvelope {
    let group = super::fixture::group().expect("qualification group");
    crate::fabric_consistency::raft::ReplicaMessageEnvelope {
        group_binding_ref: group.binding_ref,
        service_generation,
        from: super::fixture::NODE_A.to_string(),
        to: super::fixture::NODE_B.to_string(),
        message: crate::fabric_consistency::raft::Message::AppendEntries {
            term: 1,
            leader_id: super::fixture::NODE_A.to_string(),
            prev_log_index: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
            prev_log_term: 1,
            entries: Vec::new(),
            leader_commit: crate::fabric_consistency::raft::INITIAL_LOG_INDEX,
            config_epoch: group.config_epoch,
            fencing_epoch,
        },
    }
}

/// Negative control: the production transition denies the stale generation and
/// stale fencing epoch probes, and admits the same frame once the stale field is
/// restored, so the denial is attributable to the fencing field alone.
// r[verify molten.fabric_consistency.live_raft]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn stale_fencing_denial_is_caused_by_the_stale_field() {
    let group = super::fixture::group().expect("qualification group");
    let time = super::fixture::time_profile().expect("time profile");
    let state = super::fixture::start_plan(&group, super::fixture::NODE_B, &time.profile.profile_ref)
        .expect("start plan")
        .state;
    let current = probe_envelope(super::fixture::SERVICE_GENERATION, super::fixture::FENCING_EPOCH);
    let stale_generation = probe_envelope(super::fixture::SERVICE_GENERATION - 1, super::fixture::FENCING_EPOCH);
    let stale_fencing = probe_envelope(super::fixture::SERVICE_GENERATION, super::fixture::FENCING_EPOCH - 1);
    let admit = |envelope: &crate::fabric_consistency::raft::ReplicaMessageEnvelope| {
        crate::fabric_consistency::raft::apply_replica_event(
            &state,
            crate::fabric_consistency::raft::ReplicaEvent::Message {
                envelope: envelope.clone(),
            },
        )
    };
    admit(&current).expect("current-generation frame is admitted to the protocol");
    assert!(admit(&stale_generation).is_err());
    assert!(admit(&stale_fencing).is_err());
    let classify = |envelope: crate::fabric_consistency::raft::ReplicaMessageEnvelope| {
        super::replica::stale::classify(&crate::fabric_consistency::raft::ReplicaEvent::Message { envelope })
    };
    assert_eq!(classify(current), Some(super::receipt::Probe::TermAppend));
    assert_eq!(classify(stale_generation), Some(super::receipt::Probe::ServiceGeneration));
    assert_eq!(classify(stale_fencing), Some(super::receipt::Probe::FencingEpoch));
}

#[test]
fn peer_roles_are_owned_and_bounded() {
    let directory = super::super::scratch("raft-role").expect("scratch");
    assert!(super::peer("transport-listener", directory.path()).is_none());
    assert!(super::peer("raft-unknown-node-a", directory.path()).is_some_and(|result| result.is_err()));
    assert!(super::peer("raft-fresh-node-outsider", directory.path()).is_some_and(|result| result.is_err()));
}
