use super::*;

impl RedbReplicaPort {
    pub(super) fn recovery_snapshot(&self) -> crate::error::Result<Option<(Vec<u8>, Option<u64>)>> {
        let Some(coverage) =
            self.adapter.state().snapshots.values().map(|snapshot| snapshot.covered_log_sequence).max()
        else {
            return Ok(None);
        };
        let mut selected: Option<(ReplicaSnapshot, Vec<u8>)> = None;
        for descriptor in
            self.adapter.state().snapshots.values().filter(|snapshot| snapshot.covered_log_sequence == coverage)
        {
            let (_, bytes) = self
                .adapter
                .load_snapshot_bytes(&descriptor.snapshot_ref, self.adapter.state().descriptor.generation)?;
            let snapshot = super::super::recovery::parse_snapshot(&bytes)?;
            if let Some((previous, _)) = &selected {
                if snapshot.last_included_index == previous.last_included_index
                    && snapshot.snapshot_ref != previous.snapshot_ref
                {
                    return Err(crate::error::Failure::invalid_harness("equal-boundary recovery snapshots conflict"));
                }
                if snapshot.last_included_index <= previous.last_included_index {
                    continue;
                }
            }
            selected = Some((snapshot, bytes));
        }
        Ok(selected.map(|(_, bytes)| (bytes, coverage)))
    }
}
