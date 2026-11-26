use super::*;

// r[verify molten.consensus.repair_admission]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_higher_term_invalidates_recovery_barrier_observations_before_any_vote() {
    let peer = support::committed_leader();
    let repaired = exchange(&recovering(), &peer);
    let term = repaired.next.current_term + 1;
    let response = apply_replica_event(&repaired.next, ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            group_binding_ref: repaired.next.profile.group_binding_ref.clone(),
            service_generation: repaired.next.profile.service_generation,
            from: support::NODE_A.to_string(),
            to: support::NODE_B.to_string(),
            message: Message::RequestVote {
                term,
                candidate_id: support::NODE_A.to_string(),
                last_log_index: 1,
                last_log_term: 1,
                config_epoch: repaired.next.membership.config_epoch,
                fencing_epoch: repaired.next.profile.fencing_epoch,
            },
        },
    })
    .expect("higher-term request is fenced");
    let ReplicaRejoin::Recovering(round) = &response.next.rejoin else {
        panic!("recovery required")
    };
    assert_eq!(round.fence_term, None);
    assert!(round.peers.is_empty());
    assert!(matches!(support::sent_envelope_to(&response, support::NODE_A).message, Message::VoteResponse {
        granted: false,
        ..
    }));
    assert!(matches!(response.effects.first(), Some(ReplicaEffect::PersistHardState {
        term: observed, is_recovery_required: true, ..
    }) if *observed == term));
    let retry = apply_replica_event(&response.next, ReplicaEvent::ElectionTimeout {
        timer_ref: response.next.active_election_timer_ref.clone(),
    })
    .expect("rediscover before another barrier");
    assert_eq!(retry.next.role, ReplicaRole::Follower);
    assert!(matches!(support::sent_envelope_to(&retry, support::NODE_A).message, Message::RecoveryQuery {
        is_barrier: false,
        ..
    }));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn peer_snapshot_installation_discards_a_suffix_both_peers_lack_after_a_mismatched_boundary() {
    let committed = support::committed_leader();
    let mut local = recovering();
    local.log = committed.log.clone();
    local.log[0].term = 2;
    local.current_term = 2;
    local.active_election_timer_ref = support::election_timer_ref(&local);
    let mut suffix = committed.log[0].clone();
    suffix.index = 2;
    suffix.term = 2;
    suffix.request_ref = support::test_ref("retained-recovery-suffix");
    local.log.push(suffix.clone());
    let peer = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: support::test_ref("matching-boundary-snapshot"),
    })
    .expect("peer snapshot")
    .next;
    let installed = exchange(&local, &peer);
    assert_eq!(installed.next.snapshot, peer.snapshot);
    // The local term at the snapshot boundary differs from the snapshot's, and both other voters
    // report lacking the suffix entry, so it is established uncommitted and discarded exactly as a
    // leader's InstallSnapshot would. (With one report it stays unresolved and installation waits;
    // see `repair::tests`.)
    assert!(installed.next.log.is_empty(), "suffix {suffix:?} must not survive a mismatched boundary");
    assert!(installed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::PersistSnapshot {
        is_suffix_retained: false,
        ..
    })));
    assert_eq!(installed.next.commit_index, 1);
    assert!(matches!(installed.next.rejoin, ReplicaRejoin::Recovering(_)));
}

// r[verify molten.consensus.storage_fault_validation]
#[test]
fn regressed_log_terms_cannot_grant_a_higher_term_vote() {
    let mut state = support::committed_leader();
    state.log[0].term = 2;
    let mut suffix = state.log[0].clone();
    suffix.index = 2;
    suffix.term = 1;
    suffix.request_ref = support::test_ref("corrupted-term-order");
    state.log.push(suffix);
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        from: support::NODE_C.to_string(),
        to: state.node_id.clone(),
        message: Message::RequestVote {
            term: 3,
            candidate_id: support::NODE_C.to_string(),
            last_log_index: 2,
            last_log_term: 2,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    };
    assert!(
        apply_replica_event(&state, ReplicaEvent::Message {
            envelope: envelope.clone()
        })
        .is_err()
    );
    state.log[1].term = 2;
    let accepted = apply_replica_event(&state, ReplicaEvent::Message { envelope }).expect("ordered log may vote");
    assert!(matches!(support::sent_envelope_to(&accepted, support::NODE_C).message, Message::VoteResponse {
        granted: true,
        ..
    }));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.repair_admission]
#[test]
fn one_available_committed_source_repairs_data_but_cannot_admit_a_voter() {
    let peer = support::committed_leader();
    let state = recovering();
    let queries = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("initial recovery queries");
    let report = apply_replica_event(&peer, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&queries, support::NODE_A),
    })
    .expect("available peer report");
    let repaired = apply_replica_event(&queries.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&report, support::NODE_B),
    })
    .expect("available committed repair");
    assert_eq!(repaired.next.log, peer.log);
    assert_eq!(repaired.next.commit_index, 0);
    assert_eq!(repaired.next.role, ReplicaRole::Follower);
    assert!(matches!(repaired.next.rejoin, ReplicaRejoin::Recovering(RecoveryRound { fence_term: None, .. })));
    assert!(repaired.effects.iter().any(|effect| matches!(effect, ReplicaEffect::FlushLog { through_index: 1 })));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.repair_admission]
#[test]
fn cached_committed_pages_finish_data_repair_while_another_peer_remains_unavailable() {
    let peer = paged_peer();
    let state = recovering();
    let queries = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("initial queries");
    let first_report = apply_replica_event(&peer, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&queries, support::NODE_A),
    })
    .expect("first report page");
    let partial = apply_replica_event(&queries.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&first_report, support::NODE_B),
    })
    .expect("request tail page");
    let tail_report = apply_replica_event(&peer, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&partial, support::NODE_A),
    })
    .expect("tail report page");
    let first_batch = apply_replica_event(&partial.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&tail_report, support::NODE_B),
    })
    .expect("first durable repair batch");
    assert_eq!(first_batch.next.log, peer.log[..128]);
    let continued = apply_replica_event(&first_batch.next, ReplicaEvent::ElectionTimeout {
        timer_ref: first_batch.next.active_election_timer_ref.clone(),
    })
    .expect("continue repair from already verified committed corpus");
    assert_eq!(continued.next.log, peer.log);
    assert_eq!(continued.next.commit_index, 0);
    assert_eq!(continued.next.role, ReplicaRole::Follower);
    assert!(matches!(continued.next.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(matches!(
        support::sent_envelope_to(&continued, support::NODE_C).message,
        Message::RecoveryQuery { .. }
    ));
}
