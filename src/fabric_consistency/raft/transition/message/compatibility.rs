use super::*;

#[test]
fn election_dispatch_rejects_other_families_without_mutation() {
    assert_rejections(
        super::super::super::compatibility::Family::Vote,
        "election dispatch admitted a non-election message",
    );
}

#[test]
fn replication_dispatch_rejects_other_families_without_mutation() {
    assert_rejections(
        super::super::super::compatibility::Family::Append,
        "replication dispatch admitted a non-replication message",
    );
}

#[test]
fn read_dispatch_rejects_other_families_without_mutation() {
    assert_rejections(super::super::super::compatibility::Family::Read, "read dispatch admitted a non-read message");
}

#[test]
fn snapshot_dispatch_rejects_other_families_without_mutation() {
    assert_rejections(
        super::super::super::compatibility::Family::Snapshot,
        "snapshot dispatch admitted a non-snapshot message",
    );
}

fn assert_rejections(family: super::super::super::compatibility::Family, diagnostic: &str) {
    let mut panic_count = 0;
    for case in super::super::super::compatibility::cases() {
        if case.family == family {
            continue;
        }
        for persist_hard_state in [false, true] {
            let before_effects = vec![ReplicaEffect::PersistHardState {
                term: case.receiver.current_term,
                voted_for: case.receiver.voted_for.clone(),
            }];
            let mut transition = MessageTransition {
                next: case.receiver.clone(),
                effects: before_effects.clone(),
                persist_hard_state,
            };
            let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                dispatch_family(family, &mut transition, case.envelope.from.clone(), case.envelope.message.clone())
            }));
            assert_eq!(transition.next, case.receiver, "{}", case.name);
            assert_eq!(transition.effects, before_effects, "{}", case.name);
            assert_eq!(transition.persist_hard_state, persist_hard_state, "{}", case.name);
            match observed {
                Ok(result) => {
                    assert_eq!(result, Err(crate::error::MoltenError::InvalidHarness(diagnostic.to_string())))
                }
                Err(_) => panic_count += 1,
            }
        }
    }
    assert_eq!(panic_count, 0, "wrong-family dispatch must return an error, not panic");
}

fn dispatch_family(
    family: super::super::super::compatibility::Family,
    transition: &mut MessageTransition,
    from: String,
    message: RaftMessage,
) -> Result<()> {
    match family {
        super::super::super::compatibility::Family::Vote => dispatch_election(transition, from, message),
        super::super::super::compatibility::Family::Append => dispatch_replication(transition, from, message),
        super::super::super::compatibility::Family::Read => dispatch_read(transition, from, message),
        super::super::super::compatibility::Family::Snapshot => dispatch_snapshot(transition, from, message),
    }
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn successful_dispatch_preserves_direct_handler_state_effect_order_and_persistence() {
    for case in super::super::super::compatibility::cases() {
        let before = case.receiver.clone();
        let mut routed = MessageTransition {
            next: before.clone(),
            effects: Vec::new(),
            persist_hard_state: false,
        };
        observe_higher_term(&mut routed, case.envelope.message.term());
        let mut direct = MessageTransition {
            next: routed.next.clone(),
            effects: routed.effects.clone(),
            persist_hard_state: routed.persist_hard_state,
        };
        dispatch(&mut routed, case.envelope.from.clone(), case.envelope.message.clone()).expect("valid dispatch");
        direct_handler(&mut direct, case.envelope.from.clone(), case.envelope.message.clone()).expect("direct handler");
        assert_eq!(routed.next, direct.next, "{}", case.name);
        assert_eq!(routed.effects, direct.effects, "{}", case.name);
        assert_eq!(routed.persist_hard_state, direct.persist_hard_state, "{}", case.name);
        let expected = finish_message_transition(direct).expect("finish direct transition");
        let actual = apply_replica_event(&case.receiver, ReplicaEvent::Message {
            envelope: case.envelope,
        })
        .expect("public message transition");
        assert_eq!(actual, expected, "{}", case.name);
        assert_eq!(case.receiver, before, "{}", case.name);
    }
}

fn direct_handler(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        message @ (RaftMessage::RequestVote { .. } | RaftMessage::VoteResponse { .. }) => {
            direct_election(transition, from, message)
        }
        message @ (RaftMessage::AppendEntries { .. } | RaftMessage::AppendResponse { .. }) => {
            direct_replication(transition, from, message)
        }
        message @ (RaftMessage::ReadProbe { .. } | RaftMessage::ReadAcknowledgement { .. }) => {
            direct_read(transition, from, message)
        }
        message @ (RaftMessage::InstallSnapshot { .. } | RaftMessage::SnapshotResponse { .. }) => {
            direct_snapshot(transition, from, message)
        }
    }
}

fn direct_election(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::RequestVote {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
            ..
        } => election::handle_request_vote(transition, election::VoteRequestInput {
            from,
            term,
            candidate_id,
            last_log_index,
            last_log_term,
        }),
        RaftMessage::VoteResponse {
            term,
            voter_id,
            granted,
            ..
        } => election::handle_vote_response(transition, election::VoteResponseInput {
            from,
            term,
            voter_id,
            is_granted: granted,
        }),
        _ => panic!("direct election fixture received the wrong family"),
    }
}

fn direct_replication(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::AppendEntries {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
            ..
        } => replication::handle_append_entries(transition, replication::AppendEntriesInput {
            from,
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }),
        RaftMessage::AppendResponse {
            term,
            follower_id,
            success,
            request_prev_log_index,
            match_index,
            conflict_index,
            ..
        } => replication::handle_append_response(transition, replication::AppendResponseInput {
            from,
            term,
            follower_id,
            is_success: success,
            request_prev_log_index,
            match_index,
            conflict_index,
        }),
        _ => panic!("direct replication fixture received the wrong family"),
    }
}

fn direct_read(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::ReadProbe {
            term,
            leader_id,
            request_ref,
            required_index,
            ..
        } => read::handle_read_probe(transition, read::ReadProbeInput {
            from,
            term,
            leader_id,
            request_ref,
            required_index,
        }),
        RaftMessage::ReadAcknowledgement {
            term,
            follower_id,
            request_ref,
            ..
        } => read::handle_read_acknowledgement(transition, read::ReadAcknowledgementInput {
            from,
            term,
            follower_id,
            request_ref,
        }),
        _ => panic!("direct read fixture received the wrong family"),
    }
}

fn direct_snapshot(transition: &mut MessageTransition, from: String, message: RaftMessage) -> Result<()> {
    match message {
        RaftMessage::InstallSnapshot {
            term,
            leader_id,
            snapshot: installed,
            ..
        } => snapshot::handle_install_snapshot(transition, snapshot::InstallSnapshotInput {
            from,
            term,
            leader_id,
            snapshot: *installed,
        }),
        RaftMessage::SnapshotResponse {
            term,
            follower_id,
            snapshot_index,
            accepted,
            ..
        } => snapshot::handle_snapshot_response(transition, snapshot::SnapshotResponseInput {
            from,
            term,
            follower_id,
            snapshot_index,
            is_accepted: accepted,
        }),
        _ => panic!("direct snapshot fixture received the wrong family"),
    }
}
