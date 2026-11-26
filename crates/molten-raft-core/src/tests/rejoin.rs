mod fencing;

use super::*;

fn recovering() -> ReplicaState {
    let mut state = support::started_state(support::NODE_B);
    state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref: state.active_election_timer_ref.clone(),
        fence_term: None,
        peers: std::collections::BTreeMap::new(),
    });
    state
}

fn exchange(state: &ReplicaState, peer: &ReplicaState) -> ReplicaTransition {
    let round = apply_replica_event(state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("recovery timeout");
    assert_eq!(round.next.role, ReplicaRole::Follower);
    assert!(!round.effects.iter().any(|effect| matches!(effect, ReplicaEffect::PersistHardState { .. })));
    let observed = collect_reports(round, peer);
    if observed.effects.iter().any(|effect| {
        matches!(effect, ReplicaEffect::Send {
            envelope: ReplicaMessageEnvelope {
                message: Message::RecoveryQuery {
                    is_barrier: true,
                    from_index: 1,
                    ..
                },
                ..
            }
        })
    }) {
        assert!(matches!(
            observed.effects.first(),
            Some(ReplicaEffect::PersistHardState {
                is_recovery_required: true,
                voted_for: Some(_),
                ..
            })
        ));
        collect_reports(observed, peer)
    } else {
        observed
    }
}

fn collect_reports(round: ReplicaTransition, peer: &ReplicaState) -> ReplicaTransition {
    let lagging = support::started_state(support::NODE_C);
    let lagging_report = apply_replica_event(&lagging, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&round, support::NODE_C),
    })
    .expect("independent lagging peer observation");
    let observed = apply_replica_event(&round.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&lagging_report, &round.next.node_id),
    })
    .expect("lagging report alone cannot admit");
    assert!(matches!(observed.next.rejoin, ReplicaRejoin::Recovering(_)));
    let query = support::sent_envelope_to(&round, &peer.node_id);
    let report = apply_replica_event(peer, ReplicaEvent::Message { envelope: query }).expect("peer query");
    apply_replica_event(&observed.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&report, &round.next.node_id),
    })
    .expect("recovery report")
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.repair_admission]
#[test]
fn missing_committed_entry_is_durably_repaired_before_voting_admission() {
    let peer = support::committed_leader();
    let repaired = exchange(&recovering(), &peer);
    assert_eq!(repaired.next.log, peer.log);
    assert_eq!(repaired.next.commit_index, 0);
    assert!(matches!(repaired.next.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(matches!(repaired.effects.as_slice(), [
        ReplicaEffect::PersistEntries { .. },
        ReplicaEffect::FlushLog { .. },
        ..
    ]));
    let admitted = exchange(&repaired.next, &peer);
    assert_eq!(admitted.next.commit_index, peer.commit_index);
    assert_eq!(admitted.next.last_applied, peer.commit_index);
    assert!(
        matches!(admitted.next.rejoin, ReplicaRejoin::VoteFenced { through_term } if through_term == admitted.next.current_term)
    );
    assert!(
        matches!(admitted.effects.first(), Some(ReplicaEffect::PersistHardState { term, voted_for: Some(vote), is_recovery_required: false }) if vote == support::NODE_B && *term == admitted.next.current_term)
    );
    assert!(
        admitted
            .effects
            .iter()
            .any(|effect| matches!(effect, ReplicaEffect::ApplyCommitted { entries } if entries == &peer.log))
    );
    let election = apply_replica_event(&admitted.next, ReplicaEvent::ElectionTimeout {
        timer_ref: admitted.next.active_election_timer_ref.clone(),
    })
    .expect("fenced participant campaigns only in next term");
    assert_eq!(election.next.current_term, admitted.next.current_term + 1);
    assert_eq!(election.next.rejoin, ReplicaRejoin::Voter);
    assert_eq!(election.next.role, ReplicaRole::Candidate);
}

// r[verify molten.consensus.repair_admission]
#[test]
fn recovering_participant_denies_votes_and_never_acknowledges_replication() {
    let peer = support::committed_leader();
    let state = recovering();
    let envelope = |message| ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        from: support::NODE_A.to_string(),
        to: support::NODE_B.to_string(),
        message,
    };
    let vote = apply_replica_event(&state, ReplicaEvent::Message {
        envelope: envelope(Message::RequestVote {
            term: 1,
            candidate_id: support::NODE_A.to_string(),
            last_log_index: 1,
            last_log_term: 1,
            config_epoch: 1,
            fencing_epoch: 1,
        }),
    })
    .expect("denied vote response");
    assert!(matches!(support::sent_envelope_to(&vote, support::NODE_A).message, Message::VoteResponse {
        granted: false,
        ..
    }));
    let append = apply_replica_event(&vote.next, ReplicaEvent::Message {
        envelope: envelope(Message::AppendEntries {
            term: 1,
            leader_id: support::NODE_A.to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: peer.log,
            leader_commit: 1,
            config_epoch: 1,
            fencing_epoch: 1,
        }),
    })
    .expect("replication ignored until reconciliation");
    assert!(append.next.log.is_empty());
    assert!(!append.effects.iter().any(|effect| matches!(effect, ReplicaEffect::Send { .. })));
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn recovery_pages_cannot_admit_or_infer_absence_before_the_complete_observation() {
    let peer = paged_peer();
    let mut state = recovering();
    state.current_term = peer.current_term + 1;
    state.active_election_timer_ref = support::election_timer_ref(&state);
    let ReplicaRejoin::Recovering(round) = &mut state.rejoin else {
        panic!("recovery round")
    };
    round.fence_term = Some(state.current_term);
    let first = exchange(&state, &peer);
    assert!(first.next.log.is_empty());
    assert!(matches!(first.next.rejoin, ReplicaRejoin::Recovering(_)));
    let retry = apply_replica_event(&first.next, ReplicaEvent::ElectionTimeout {
        timer_ref: first.next.active_election_timer_ref.clone(),
    })
    .expect("slow pagination survives timer delivery");
    let continuation = support::sent_envelope_to(&retry, support::NODE_A);
    let Message::RecoveryQuery {
        round_ref: original_round,
        ..
    } = support::sent_envelope_to(&first, support::NODE_A).message
    else {
        panic!("original continuation query");
    };
    assert!(matches!(&continuation.message, Message::RecoveryQuery { round_ref, .. } if *round_ref == original_round));
    assert!(matches!(continuation.message, Message::RecoveryQuery { from_index: 129, .. }));
    let report = apply_replica_event(&peer, ReplicaEvent::Message { envelope: continuation }).expect("tail page");
    let repaired = apply_replica_event(&retry.next, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&report, support::NODE_B),
    })
    .expect("complete observation repairs bounded batch");
    assert_eq!(repaired.next.log, peer.log[..128]);
    assert_eq!(repaired.next.commit_index, 0);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn recovery_installs_a_peer_snapshot_before_admission() {
    let committed = support::committed_leader();
    let peer = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: support::test_ref("rejoin-application-state"),
    })
    .expect("peer snapshot")
    .next;
    let repaired = exchange(&recovering(), &peer);
    assert_eq!(repaired.next.snapshot, peer.snapshot);
    assert!(matches!(repaired.next.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(repaired.effects.iter().any(|effect| matches!(effect, ReplicaEffect::PersistSnapshot { .. })));
    assert!(
        repaired
            .effects
            .iter()
            .any(|effect| matches!(effect, ReplicaEffect::RestoreApplicationSnapshot { .. }))
    );
    let admitted = exchange(&repaired.next, &peer);
    assert!(matches!(admitted.next.rejoin, ReplicaRejoin::VoteFenced { .. }));
}

// r[verify molten.consensus.storage_fault_validation]
#[test]
fn recovery_rejects_a_complete_report_with_a_substituted_corpus_identity() {
    let peer = support::committed_leader();
    let state = recovering();
    let round = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    })
    .expect("recovery round");
    let report = apply_replica_event(&peer, ReplicaEvent::Message {
        envelope: support::sent_envelope_to(&round, support::NODE_A),
    })
    .expect("peer report");
    let mut envelope = support::sent_envelope_to(&report, support::NODE_B);
    let Message::RecoveryReport { observation_ref, .. } = &mut envelope.message else {
        panic!("expected recovery report");
    };
    *observation_ref = support::test_ref("substituted-corpus");
    assert!(apply_replica_event(&round.next, ReplicaEvent::Message { envelope }).is_err());
    assert!(round.next.log.is_empty());
}

// r[verify molten.consensus.repair_admission]
#[test]
fn recovering_state_cannot_return_cached_commits_or_local_reads() {
    let mut state = support::committed_leader();
    state.role = ReplicaRole::Follower;
    state.leader_id = None;
    state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref: state.active_election_timer_ref.clone(),
        fence_term: None,
        peers: std::collections::BTreeMap::new(),
    });
    let proposal = apply_replica_event(&state, ReplicaEvent::Propose {
        request_ref: state.log[0].request_ref.clone(),
        command_ref: state.log[0].command_ref.clone(),
        command_schema_ref: state.log[0].command_schema_ref.clone(),
    })
    .expect("explicit client denial");
    assert!(matches!(proposal.effects.as_slice(), [ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Denied,
        committed_index: None,
        ..
    }]));
    let read = apply_replica_event(&state, ReplicaEvent::Read {
        request_ref: support::test_ref("unreconciled-local-read"),
        mode: ReadMode::LocalStale,
    })
    .expect("explicit read denial");
    assert!(matches!(read.effects.as_slice(), [ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Denied,
        ..
    }]));
}

fn paged_peer() -> ReplicaState {
    let mut peer = support::committed_leader();
    peer.log = (1..=129)
        .map(|index| ReplicatedEntry {
            index,
            term: 1,
            request_ref: support::test_ref(&format!("paged-request-{index}")),
            command_ref: support::test_ref("paged-command"),
            command_schema_ref: support::test_ref("paged-schema"),
        })
        .collect();
    peer.commit_index = 129;
    peer.last_applied = 129;
    peer.completed_requests = peer.log.iter().map(|entry| (entry.request_ref.clone(), entry.index)).collect();
    peer
}
