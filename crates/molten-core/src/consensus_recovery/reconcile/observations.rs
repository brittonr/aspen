use super::*;

pub(super) fn is_compacted(peer: &PeerRecovery, index: u64) -> bool {
    peer.snapshot.as_ref().is_some_and(|snapshot| index <= snapshot.last_included_index)
}

pub(super) fn recovery_base(input: &ReconcileInput) -> u64 {
    input
        .peers
        .iter()
        .filter_map(|peer| peer.snapshot.as_ref())
        .filter(|snapshot| {
            snapshot.last_included_index >= input.local_commit_index
                || matches!(input.local_entries.get(&snapshot.last_included_index),
                    Some(LocalEntry::Present(entry)) if entry.term == snapshot.last_included_term)
        })
        .map(|snapshot| snapshot.last_included_index)
        .fold(input.local_snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index), u64::max)
}

pub(super) fn reconcile_snapshot(input: &ReconcileInput, peers: &[PeerSnapshotReport]) -> SnapshotRepairDecision {
    let mut snapshots = std::collections::BTreeMap::new();
    for snapshot in input.local_snapshot.iter().chain(peers.iter().map(|peer| &peer.snapshot)) {
        if snapshots
            .insert(snapshot.last_included_index, snapshot)
            .is_some_and(|previous| previous != snapshot)
        {
            return SnapshotRepairDecision::ConflictingContent;
        }
        if snapshot.last_included_index <= input.local_commit_index
            && matches!(input.local_entries.get(&snapshot.last_included_index), Some(LocalEntry::Present(entry)) if entry.term != snapshot.last_included_term)
        {
            return SnapshotRepairDecision::ConflictingContent;
        }
        if input.peers.iter().any(|peer| {
            peer.commit_index >= snapshot.last_included_index
                && peer
                    .entries
                    .get(&snapshot.last_included_index)
                    .is_some_and(|entry| entry.term != snapshot.last_included_term)
        }) {
            return SnapshotRepairDecision::ConflictingContent;
        }
    }
    if peers.is_empty() {
        SnapshotRepairDecision::KeepLocal
    } else {
        // A validated committed log already covers an older snapshot. Never
        // install one over newer committed state when its boundary is unknown.
        match decide_snapshot_repair(input.local_snapshot.as_ref(), peers) {
            SnapshotRepairDecision::InstallFrom { snapshot, .. }
                if snapshot.last_included_index <= input.local_commit_index
                    && matches!(input.local_entries.get(&snapshot.last_included_index), Some(LocalEntry::Present(entry)) if entry.term == snapshot.last_included_term) =>
            {
                SnapshotRepairDecision::KeepLocal
            }
            SnapshotRepairDecision::InstallFrom { snapshot, .. }
                if snapshot.last_included_index < input.local_commit_index =>
            {
                SnapshotRepairDecision::Wait
            }
            decision => decision,
        }
    }
}

pub(super) fn validated_indices(input: &ReconcileInput) -> Option<std::collections::BTreeSet<u64>> {
    if input.peers.len() > MAX_RECOVERY_PEER_REPORTS
        || input.local_entries.len() > MAX_RECONCILED_INDICES
        || !input.voters.contains(&input.participant_id)
        || input.local_entries.contains_key(&0)
        || input
            .local_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.last_included_index > input.local_commit_index)
    {
        return None;
    }
    let mut identities = std::collections::BTreeSet::new();
    for peer in &input.peers {
        if !peer.is_complete
            || peer.entries.len() > MAX_RECONCILED_INDICES
            || !identities.insert(&peer.peer_id)
            || peer.peer_id == input.participant_id
            || !input.voters.contains(&peer.peer_id)
        {
            return None;
        }
        let snapshot_index = peer.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index);
        if snapshot_index > peer.commit_index || peer.entries.keys().any(|index| *index <= snapshot_index) {
            return None;
        }
        let last_index = peer.entries.last_key_value().map_or(snapshot_index, |(index, _)| *index);
        if last_index < peer.commit_index || u64::try_from(peer.entries.len()) != Ok(last_index - snapshot_index) {
            return None;
        }
    }
    let repair_base = recovery_base(input);
    // Superseded keys do not consume the repair bound, but their readable local
    // commitment must still veto conflicting committed peer history.
    for (index, local) in input.local_entries.range(..=repair_base.min(input.local_commit_index)) {
        if let LocalEntry::Present(local) = local
            && input
                .peers
                .iter()
                .any(|peer| peer.commit_index >= *index && peer.entries.get(index).is_some_and(|entry| entry != local))
        {
            return Some(std::collections::BTreeSet::from([*index]));
        }
    }
    let committed_through = input.peers.iter().map(|peer| peer.commit_index).fold(input.local_commit_index, u64::max);
    if !usize::try_from(committed_through.saturating_sub(repair_base)).is_ok_and(|span| span <= MAX_RECONCILED_INDICES)
    {
        return None;
    }
    let mut indices = std::collections::BTreeSet::new();
    for index in input.local_entries.keys().chain(input.peers.iter().flat_map(|peer| peer.entries.keys())) {
        if *index > repair_base {
            indices.insert(*index);
            if indices.len() > MAX_RECONCILED_INDICES {
                return None;
            }
        }
    }
    if repair_base < committed_through {
        for index in (repair_base + 1)..=committed_through {
            indices.insert(index);
            if indices.len() > MAX_RECONCILED_INDICES {
                return None;
            }
        }
    }
    (indices.len() <= MAX_RECONCILED_INDICES).then_some(indices)
}

pub(super) fn decide_unreadable(input: &ReconcileInput, index: u64, local: Option<&LocalEntry>) -> EntryRepairDecision {
    let mut lacking = 0;
    for peer in input.peers.iter().filter(|peer| !is_compacted(peer, index)) {
        if peer.entries.contains_key(&index) {
            return EntryRepairDecision::Wait;
        }
        lacking += 1;
    }
    if index <= input.local_commit_index || lacking < quorum(&input.voters) {
        return EntryRepairDecision::Wait;
    }
    match local {
        Some(LocalEntry::Faulty) => EntryRepairDecision::RemoveUncommitted,
        _ => EntryRepairDecision::KeepLocal,
    }
}

pub(super) fn unresolved(input: &ReconcileInput) -> Reconciliation {
    let hard_state = decide_hard_state(&input.local_hard_state, None);
    let admission = admit_repaired_participant(&RepairAdmissionInput {
        is_persistent_state_suspect: true,
        is_storage_serving: input.is_storage_serving,
        is_protocol_reconciled: false,
        is_log_or_snapshot_validated: input.is_log_or_snapshot_validated,
        is_membership_current: input.is_membership_current,
        hard_state: hard_state.clone(),
        current_term: input.current_term,
    });
    Reconciliation {
        decisions: Vec::new(),
        hard_state,
        snapshot: SnapshotRepairDecision::Wait,
        effects: Vec::new(),
        is_reconciled: false,
        admission,
    }
}
