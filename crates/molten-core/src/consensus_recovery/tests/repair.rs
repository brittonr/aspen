use super::*;

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.storage_fault_recovery_progress]
#[test]
fn admitted_local_state_needs_no_peer() {
    let local = LocalEntry::Present(entry(3, "committed"));
    assert_eq!(decide_entry_repair(&input(local, INDEX, Vec::new())), EntryRepairDecision::KeepLocal);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn a_missing_or_faulty_committed_entry_is_fetched_and_a_divergent_one_replaced() {
    let committed = entry(3, "committed");
    let peers = || {
        vec![
            report("node-b", Some(committed.clone()), INDEX),
            report("node-c", None, INDEX - 1),
        ]
    };
    for local in [LocalEntry::Missing, LocalEntry::Faulty] {
        assert_eq!(decide_entry_repair(&input(local, 0, peers())), EntryRepairDecision::FetchCommitted {
            from_peer: "node-b".to_string(),
            entry: committed.clone(),
        });
    }
    let divergent = decide_entry_repair(&input(LocalEntry::Present(entry(2, "stale")), 0, peers()));
    assert_eq!(divergent, EntryRepairDecision::ReplaceWithCommitted {
        from_peer: "node-b".to_string(),
        entry: committed,
    });
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn only_a_quorum_lacking_the_entry_removes_it_and_a_committed_entry_is_never_removed() {
    let local = entry(4, "local-only");
    let lacking = vec![
        report("node-b", None, INDEX - 1),
        report("node-c", Some(entry(4, "other")), INDEX - 1),
    ];
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Present(local.clone()), 0, lacking)),
        EntryRepairDecision::RemoveUncommitted,
        "a quorum lacks the local entry, so it was never committed"
    );
    let holding = vec![
        report("node-b", Some(local.clone()), INDEX - 1),
        report("node-c", None, INDEX - 1),
    ];
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Present(local.clone()), 0, holding)),
        EntryRepairDecision::Wait,
        "a quorum may hold it: commitment is unknown"
    );
    let committed = vec![
        report("node-b", Some(local.clone()), INDEX),
        report("node-c", None, INDEX - 1),
    ];
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Present(local), 0, committed)),
        EntryRepairDecision::KeepLocal
    );
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn unknown_commitment_and_an_unanswered_quorum_wait() {
    let one_report = vec![report("node-b", None, INDEX - 1)];
    assert_eq!(decide_entry_repair(&input(LocalEntry::Faulty, 0, Vec::new())), EntryRepairDecision::Wait);
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Present(entry(4, "local")), 0, one_report)),
        EntryRepairDecision::Wait,
        "one lacking peer is not a quorum"
    );
    let peer_holds = vec![report("node-b", Some(entry(4, "maybe")), INDEX - 1)];
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Faulty, 0, peer_holds)),
        EntryRepairDecision::Wait,
        "a peer holds an entry that a quorum may hold"
    );
    let all_lack = vec![report("node-b", None, INDEX - 1), report("node-c", None, INDEX - 1)];
    assert_eq!(decide_entry_repair(&input(LocalEntry::Faulty, 0, all_lack)), EntryRepairDecision::RemoveUncommitted);
}

// r[verify molten.consensus.storage_fault_validation]
#[test]
fn conflicting_committed_history_plans_nothing() {
    let conflicting = vec![
        report("node-b", Some(entry(3, "one")), INDEX),
        report("node-c", Some(entry(3, "two")), INDEX),
    ];
    let decision = decide_entry_repair(&input(LocalEntry::Missing, 0, conflicting));
    assert_eq!(decision, EntryRepairDecision::ConflictingHistory);
    assert!(plan_recovery_effects(&[(INDEX, decision)], &["node-b".to_string()]).is_empty());
    let outsider = vec![report("node-z", Some(entry(3, "outsider")), INDEX)];
    assert_eq!(
        decide_entry_repair(&input(LocalEntry::Missing, 0, outsider)),
        EntryRepairDecision::Wait,
        "a report from outside the admitted voters carries no weight"
    );
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn effect_plans_query_waiting_indices_and_remove_only_from_the_first_uncommitted_index() {
    let committed = entry(3, "committed");
    let decisions = vec![
        (9, EntryRepairDecision::RemoveUncommitted),
        (5, EntryRepairDecision::FetchCommitted {
            from_peer: "node-b".to_string(),
            entry: committed.clone(),
        }),
        (8, EntryRepairDecision::Wait),
        (6, EntryRepairDecision::KeepLocal),
    ];
    let peers = vec!["node-b".to_string(), "node-c".to_string()];
    assert_eq!(plan_recovery_effects(&decisions, &peers), vec![
        RecoveryEffect::QueryPeers {
            peers: peers.clone(),
            indices: vec![8],
        },
        RecoveryEffect::RemoveSuffixFrom { index: 9 },
        RecoveryEffect::FetchEntry {
            from_peer: "node-b".to_string(),
            index: 5,
            entry: committed,
        },
    ]);
}

// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_lost_term_or_vote_is_fenced_never_copied() {
    let present = LocalHardState::Present {
        term: 5,
        voted_for: Some("node-b".to_string()),
    };
    assert_eq!(decide_hard_state(&present, Some(9)), HardStateDecision::KeepLocal);
    assert_eq!(decide_hard_state(&LocalHardState::Lost, Some(9)), HardStateDecision::VoteFenced { through_term: 9 });
    assert_eq!(decide_hard_state(&LocalHardState::Lost, None), HardStateDecision::Wait);
}

fn snapshot(index: u64, label: &str) -> SnapshotFact {
    SnapshotFact {
        last_included_index: index,
        last_included_term: 3,
        content_ref: format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex()),
    }
}

fn snapshot_report(peer: &str, fact: SnapshotFact, commit_index: u64) -> PeerSnapshotReport {
    PeerSnapshotReport {
        peer_id: peer.to_string(),
        snapshot: fact,
        commit_index,
    }
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_snapshot_repair_needs_one_content_identity_at_a_committed_index() {
    let good = snapshot(20, "good");
    let committed = vec![
        snapshot_report("node-b", good.clone(), 20),
        snapshot_report("node-c", good.clone(), 25),
    ];
    assert_eq!(decide_snapshot_repair(None, &committed), SnapshotRepairDecision::InstallFrom {
        peer_id: "node-b".to_string(),
        snapshot: good.clone(),
    });
    assert_eq!(decide_snapshot_repair(Some(&good), &committed), SnapshotRepairDecision::KeepLocal);
    let bad = vec![
        snapshot_report("node-b", good, 20),
        snapshot_report("node-c", snapshot(20, "bad"), 20),
    ];
    assert_eq!(decide_snapshot_repair(None, &bad), SnapshotRepairDecision::ConflictingContent);
    let uncommitted = vec![snapshot_report("node-b", snapshot(30, "ahead"), 10)];
    assert_eq!(decide_snapshot_repair(None, &uncommitted), SnapshotRepairDecision::Wait);
}
