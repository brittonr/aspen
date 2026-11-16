use preserves::ValueImpl;

use super::*;

impl RedbReplicaPort {
    /// Persist the snapshot, then, under retention authority, compact in crash-safe order: write
    /// the compaction base after the synchronized snapshot, retire superseded snapshots, and
    /// truncate only records the new snapshot covers. Recovery after any prefix of these steps
    /// rebuilds the same state: the newest valid snapshot plus the hard state and entries above
    /// its boundary.
    // r[impl molten.consensus.durable_log_compaction]
    pub(in super::super) fn persist_snapshot_through(
        &mut self,
        snapshot: &ReplicaSnapshot,
        is_suffix_retained: bool,
        retained: &ReplicaRetainedState<'_>,
        last_step: Option<CompactionStep>,
    ) -> crate::error::Result<String> {
        if snapshot.snapshot_ref != snapshot_ref(snapshot)? {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft snapshot identity mismatch before persistence",
            ));
        }
        let value = snapshot_value(snapshot);
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        self.append_value(
            crate::preserves_rail::record("raft-snapshot-suffix-v1", vec![
                crate::preserves_rail::string(&snapshot.snapshot_ref),
                crate::preserves_rail::bool_value(is_suffix_retained),
            ]),
            crate::fabric_durability::DurabilityLevel::MachineLoss,
        )?;
        let generation = self.adapter.state().descriptor.generation;
        let covered_log_sequence = self.adapter.state().durable_log.last().map(|record| record.sequence);
        let request = crate::fabric_durability::SnapshotRequest {
            kind: crate::fabric_durability::SnapshotKind::Snapshot,
            generation,
            snapshot_ref: snapshot.snapshot_ref.clone(),
            content_ref,
            ordered_state_ref: snapshot.application_state_ref.clone(),
            covered_log_sequence,
            durability: crate::fabric_durability::DurabilityLevel::MachineLoss,
        };
        let snapshot_transition_ref = self.adapter.create_snapshot(&request, &bytes)?.transition_ref;
        let Some(authority_ref) = self.retention_authority_ref.clone() else {
            return Ok(snapshot_transition_ref);
        };
        if last_step < Some(CompactionStep::BaseStarted) {
            return Ok(snapshot_transition_ref);
        }
        validate_retained_suffix(snapshot, retained)?;
        let base_sequence = self.adapter.state().next_log_sequence().map_err(|issue| {
            crate::error::Failure::invalid_harness(format!("live Raft compaction base sequence denied: {issue:?}"))
        })?;
        // Only one entry is encoded at a time: neither an append nor its temporary canonical
        // value grows with the retained suffix. The footer is the completion boundary.
        let commit_index = retained_commit_index(&self.adapter.state().durable_log, snapshot.last_included_index)?;
        let base = super::super::recovery::compaction_base_value(
            snapshot,
            retained.term,
            retained.voted_for,
            retained.entries,
            commit_index,
        );
        let mut base_ref = self.append_value(base, crate::fabric_durability::DurabilityLevel::MachineLoss)?;
        let base_value_ref = self
            .adapter
            .state()
            .durable_log
            .last()
            .ok_or_else(|| {
                crate::error::Failure::invalid_harness("synchronized live Raft compaction base header missing")
            })?
            .value_ref
            .clone();
        if last_step == Some(CompactionStep::BaseStarted) {
            return Ok(base_ref);
        }
        for (offset, entry) in retained.entries.iter().enumerate() {
            self.append_value(
                crate::preserves_rail::record("raft-log-mutation-v1", vec![
                    crate::preserves_rail::record("none", Vec::new()),
                    crate::preserves_rail::sequence(vec![canonical::entry_value(entry)]),
                ]),
                crate::fabric_durability::DurabilityLevel::MachineLoss,
            )?;
            if last_step == Some(CompactionStep::BaseEntriesWritten(offset + 1)) {
                return Ok(base_ref);
            }
        }
        base_ref = self.append_value(
            crate::preserves_rail::record("raft-compaction-end-v1", vec![crate::preserves_rail::string(
                &base_value_ref,
            )]),
            crate::fabric_durability::DurabilityLevel::MachineLoss,
        )?;
        if last_step <= Some(CompactionStep::BaseWritten) {
            return Ok(base_ref);
        }
        let retire_ref = self
            .adapter
            .retire_superseded_snapshots(generation, &snapshot.snapshot_ref, Some(&authority_ref))?
            .transition_ref;
        if last_step <= Some(CompactionStep::SnapshotsRetired) {
            return Ok(retire_ref);
        }
        let truncate_ref = self
            .adapter
            .truncate_snapshot_covered(generation, &snapshot.snapshot_ref, base_sequence, Some(&authority_ref))?
            .transition_ref;
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-snapshot-compaction-v1", vec![
            crate::preserves_rail::string(&snapshot_transition_ref),
            crate::preserves_rail::string(&base_ref),
            crate::preserves_rail::string(&retire_ref),
            crate::preserves_rail::string(&truncate_ref),
        ]))
    }
}

/// The entries a compaction base keeps must continue the snapshot without a gap.
fn validate_retained_suffix(
    snapshot: &ReplicaSnapshot,
    retained: &ReplicaRetainedState<'_>,
) -> crate::error::Result<()> {
    let is_contiguous = retained
        .entries
        .iter()
        .zip(snapshot.last_included_index.saturating_add(1)..)
        .all(|(entry, expected)| entry.index == expected && entry.term >= snapshot.last_included_term);
    if !is_contiguous || retained.term < snapshot.last_included_term {
        return Err(crate::error::Failure::invalid_harness("live Raft compaction base does not continue its snapshot"));
    }
    Ok(())
}

/// Compaction also retains committed entries above an installed snapshot. Find their durable
/// boundary without constructing an encoded suffix or changing the executor's retained-state API.
fn retained_commit_index(
    records: &[crate::fabric_durability::LogRecord],
    snapshot_boundary: u64,
) -> crate::error::Result<u64> {
    let mut committed = snapshot_boundary;
    for record in records {
        let decoded = crate::preserves_rail::strict_canonical_decode(&record.value)?;
        if let Some(fields) = decoded.value.collect_simple_record("raft-commit-boundary-v1", Some(1)) {
            committed = committed.max(canonical::required_u64(&fields[0], "compaction commit boundary")?);
        }
        if let Some(fields) = decoded.value.collect_simple_record("raft-compaction-start-v1", Some(6)) {
            committed = committed.max(canonical::required_u64(&fields[5], "compaction retained commit boundary")?);
        }
    }
    Ok(committed)
}
