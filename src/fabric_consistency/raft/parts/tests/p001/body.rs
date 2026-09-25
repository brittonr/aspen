
pub(super) fn elect_node_a() -> (ReplicaState, ReplicaState) {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let node_b = started_state(&group, NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("node A election");
    let vote_request = sent_envelope_to(&election, NODE_B);
    let vote =
        apply_replica_event(&node_b, ReplicaEvent::Message { envelope: vote_request }).expect("node B vote response");
    let vote_response = sent_envelope_to(&vote, NODE_A);
    let leader = apply_replica_event(&election.next, ReplicaEvent::Message {
        envelope: vote_response,
    })
    .expect("node A leadership");
    assert_eq!(leader.next.role, ReplicaRole::Leader);
    (leader.next, vote.next)
}

pub(super) fn committed_leader() -> ReplicaState {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("committed-helper-request"),
        command_ref: test_ref("committed-helper-command"),
        command_schema_ref: test_ref("committed-helper-schema"),
    })
    .expect("helper proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("helper follower append");
    let response = sent_envelope_to(&replicated, NODE_A);
    apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("helper majority commit")
        .next
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_majority_replication_commits_and_applies_in_effect_order() {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("proposal-request"),
        command_ref: test_ref("proposal-command"),
        command_schema_ref: test_ref("proposal-schema"),
    })
    .expect("leader proposal");
    assert_eq!(proposal.next.commit_index, INITIAL_COMMIT_INDEX);
    assert!(matches!(proposal.effects.first(), Some(ReplicaEffect::PersistEntries { .. })));
    assert!(matches!(proposal.effects.get(1), Some(ReplicaEffect::FlushLog { .. })));

    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("follower append");
    let append_response = sent_envelope_to(&replicated, NODE_A);
    let committed = apply_replica_event(&proposal.next, ReplicaEvent::Message {
        envelope: append_response,
    })
    .expect("leader majority commit");

    assert_eq!(committed.next.commit_index, INITIAL_LOG_INDEX);
    assert_eq!(committed.next.last_applied, INITIAL_LOG_INDEX);
    assert!(committed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ApplyCommitted { .. })));
    assert!(committed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Committed,
        committed_index: Some(INITIAL_LOG_INDEX),
        ..
    })));

    let heartbeat = apply_replica_event(&committed.next, ReplicaEvent::HeartbeatTimeout).expect("commit heartbeat");
    let commit_notice = sent_envelope_to(&heartbeat, NODE_B);
    let follower_commit = apply_replica_event(&replicated.next, ReplicaEvent::Message {
        envelope: commit_notice,
    })
    .expect("follower commit application");
    assert_eq!(follower_commit.next.commit_index, INITIAL_LOG_INDEX);
    assert!(follower_commit.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ApplyCommitted { .. })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_minority_and_duplicate_proposals_cannot_advance_commit() {
    let (leader, _follower) = elect_node_a();
    let request_ref = test_ref("minority-request");
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: request_ref.clone(),
        command_ref: test_ref("minority-command"),
        command_schema_ref: test_ref("minority-schema"),
    })
    .expect("minority proposal");
    assert_eq!(proposal.next.commit_index, INITIAL_COMMIT_INDEX);
    assert_eq!(proposal.next.log.len(), EXPECTED_SINGLE_LOG_ENTRY);

    let duplicate = apply_replica_event(&proposal.next, ReplicaEvent::Propose {
        request_ref,
        command_ref: test_ref("minority-command"),
        command_schema_ref: test_ref("minority-schema"),
    })
    .expect("duplicate proposal outcome");
    assert_eq!(duplicate.next.commit_index, INITIAL_COMMIT_INDEX);
    assert_eq!(duplicate.next.log.len(), EXPECTED_SINGLE_LOG_ENTRY);
    assert!(duplicate.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Retryable,
        committed_index: None,
        ..
    })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_snapshot_compacts_only_through_committed_application_state() {
    let committed = committed_leader();
    let snapshot = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: test_ref("snapshot-application-state"),
    })
    .expect("snapshot transition");
    let stored = snapshot.next.snapshot.as_ref().expect("snapshot state");

    assert_eq!(stored.last_included_index, committed.last_applied);
    assert_eq!(stored.snapshot_ref, snapshot_ref(stored).expect("snapshot identity"));
    assert!(snapshot.next.log.iter().all(|entry| entry.index > stored.last_included_index));
    assert!(matches!(snapshot.effects.as_slice(), [ReplicaEffect::PersistSnapshot { .. }]));
    let completed_request_ref = stored.completed_requests.keys().next().expect("completed request retained").clone();
    let duplicate = apply_replica_event(&snapshot.next, ReplicaEvent::Propose {
        request_ref: completed_request_ref,
        command_ref: test_ref("snapshot-duplicate-command"),
        command_schema_ref: test_ref("snapshot-duplicate-schema"),
    })
    .expect("compacted duplicate outcome");
    assert!(matches!(duplicate.effects.as_slice(), [ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Committed,
        committed_index: Some(INITIAL_LOG_INDEX),
        ..
    }]));

    let group = active_group();
    let empty = started_state(&group, NODE_A);
    let empty_error = apply_replica_event(&empty, ReplicaEvent::CreateSnapshot {
        application_state_ref: test_ref("empty-snapshot-application-state"),
    })
    .expect_err("uncommitted snapshot must deny");
    assert!(empty_error.to_string().contains("committed application boundary"));

    let mut tampered = snapshot.next;
    tampered.snapshot.as_mut().expect("snapshot").application_state_ref = test_ref("tampered-application-state");
    let tamper_error = apply_replica_event(&tampered, ReplicaEvent::Read {
        request_ref: test_ref("tampered-snapshot-read"),
        mode: crate::fabric_consistency::ConsistencyReadMode::LocalStale,
    })
    .expect_err("tampered snapshot must deny before read");
    assert!(tamper_error.to_string().contains("snapshot binding or identity mismatch"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_installs_bound_snapshots_before_acknowledging_catch_up() {
    let leader = committed_leader();
    let snapshot = apply_replica_event(&leader, ReplicaEvent::CreateSnapshot {
        application_state_ref: test_ref("catch-up-application-state"),
    })
    .expect("leader snapshot");
    let heartbeat = apply_replica_event(&snapshot.next, ReplicaEvent::HeartbeatTimeout).expect("snapshot heartbeat");
    let install = sent_envelope_to(&heartbeat, NODE_C);
    assert!(matches!(&install.message, RaftMessage::InstallSnapshot { .. }));

    let follower = started_state(&active_group(), NODE_C);
    let mut tampered = install.clone();
    match &mut tampered.message {
        RaftMessage::InstallSnapshot { snapshot, .. } => {
            snapshot.application_state_ref = test_ref("tampered-catch-up-state");
        }
        other => panic!("expected install snapshot, got {other:?}"),
    }
    let error = apply_replica_event(&follower, ReplicaEvent::Message { envelope: tampered })
        .expect_err("tampered snapshot must deny");
    assert!(error.to_string().contains("identity mismatch"));

    let installed =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: install }).expect("follower snapshot install");
    assert_eq!(installed.next.commit_index, INITIAL_LOG_INDEX);
    assert_eq!(installed.next.last_applied, INITIAL_LOG_INDEX);
    let persist_position = installed
        .effects
        .iter()
        .position(|effect| matches!(effect, ReplicaEffect::PersistSnapshot { .. }))
        .expect("snapshot persistence");
    let restore_position = installed
        .effects
        .iter()
        .position(|effect| matches!(effect, ReplicaEffect::RestoreApplicationSnapshot { .. }))
        .expect("application restore");
    let send_position = installed
        .effects
        .iter()
        .position(|effect| matches!(effect, ReplicaEffect::Send { .. }))
        .expect("snapshot acknowledgement");
    assert!(persist_position < restore_position);
    assert!(restore_position < send_position);

    let response = sent_envelope_to(&installed, NODE_A);
    let accepted = apply_replica_event(&snapshot.next, ReplicaEvent::Message { envelope: response })
        .expect("leader snapshot acknowledgement");
    assert_eq!(accepted.next.match_index.get(NODE_C), Some(&INITIAL_LOG_INDEX));
    assert_eq!(accepted.next.next_index.get(NODE_C), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_denies_superseded_election_timer_before_protocol_effects() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let superseded_timer_ref = node_a.active_election_timer_ref.clone();
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: superseded_timer_ref.clone(),
    })
    .expect("current election timer");

    let error = apply_replica_event(&election.next, ReplicaEvent::ElectionTimeout {
        timer_ref: superseded_timer_ref,
    })
    .expect_err("superseded timer must deny");
    assert!(error.to_string().contains("stale Raft election timer"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_denies_stale_epoch_messages_without_state_mutation() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let node_b = started_state(&group, NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("election request");
    let mut stale = sent_envelope_to(&election, NODE_B);
    match &mut stale.message {
        RaftMessage::RequestVote { config_epoch, .. } => {
            *config_epoch += STALE_EPOCH_STEP;
        }
        other => panic!("expected request vote, got {other:?}"),
    }
    let before = node_b.clone();
    let error = apply_replica_event(&node_b, ReplicaEvent::Message { envelope: stale })
        .expect_err("stale config epoch must deny");
    assert!(error.to_string().contains("stale configuration or fencing epoch"));
    assert_eq!(node_b, before);
}
