use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_durability::AppendRequest;
use crate::fabric_durability::DurabilityLevel;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::SnapshotKind;
use crate::fabric_durability::SnapshotRequest;

#[derive(Debug)]
pub struct RedbReplicaDurabilityPort {
    adapter: RedbDurableStateAdapter,
}

impl RedbReplicaDurabilityPort {
    pub const fn new(adapter: RedbDurableStateAdapter) -> Self {
        Self { adapter }
    }

    pub const fn adapter(&self) -> &RedbDurableStateAdapter {
        &self.adapter
    }

    fn append_value(&mut self, value: preserves::IOValue, durability: DurabilityLevel) -> Result<String> {
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let descriptor = &self.adapter.state().descriptor;
        let expected_sequence =
            self.adapter.state().next_log_sequence().map_err(|error| {
                MoltenError::invalid_harness(format!("live Raft durable sequence denied: {error:?}"))
            })?;
        let request = AppendRequest {
            adapter_id: descriptor.adapter_id.clone(),
            namespace_id: descriptor.namespace_id.clone(),
            generation: descriptor.generation,
            expected_sequence,
            value: bytes,
            value_ref,
            durability,
        };
        Ok(self.adapter.append(&request)?.transition_ref)
    }
}

impl ReplicaDurabilityEffects for RedbReplicaDurabilityPort {
    fn persist_hard_state(&mut self, term: u64, voted_for: Option<&str>) -> Result<String> {
        let vote = voted_for.map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |voter| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(voter)]),
        );
        self.append_value(
            crate::preserves_rail::record("raft-hard-state-v1", vec![crate::preserves_rail::u64_value(term), vote]),
            DurabilityLevel::MachineLoss,
        )
    }

    fn persist_entries(&mut self, truncate_from: Option<u64>, entries: &[ReplicatedEntry]) -> Result<String> {
        let truncate = truncate_from.map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |index| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(index)]),
        );
        self.append_value(
            crate::preserves_rail::record("raft-log-mutation-v1", vec![
                truncate,
                crate::preserves_rail::sequence(entries.iter().map(entry_value).collect()),
            ]),
            DurabilityLevel::Buffered,
        )
    }

    fn flush_log(&mut self, through_index: u64) -> Result<String> {
        self.append_value(
            crate::preserves_rail::record("raft-log-flush-v1", vec![crate::preserves_rail::u64_value(through_index)]),
            DurabilityLevel::Buffered,
        )?;
        let generation = self.adapter.state().descriptor.generation;
        Ok(self.adapter.flush(generation, DurabilityLevel::MachineLoss)?.transition_ref)
    }

    fn persist_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> Result<String> {
        let value = snapshot_value(snapshot);
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let generation = self.adapter.state().descriptor.generation;
        let covered_log_sequence = self.adapter.state().durable_log.last().map(|record| record.sequence);
        let request = SnapshotRequest {
            kind: SnapshotKind::Snapshot,
            generation,
            snapshot_ref: snapshot.snapshot_ref.clone(),
            content_ref,
            ordered_state_ref: snapshot.application_state_ref.clone(),
            covered_log_sequence,
            durability: DurabilityLevel::MachineLoss,
        };
        Ok(self.adapter.create_snapshot(&request, &bytes)?.transition_ref)
    }
}

fn entry_value(entry: &ReplicatedEntry) -> preserves::IOValue {
    crate::preserves_rail::record("raft-replicated-entry-v1", vec![
        crate::preserves_rail::string(RAFT_REPLICATED_ENTRY_SCHEMA),
        crate::preserves_rail::u64_value(entry.index),
        crate::preserves_rail::u64_value(entry.term),
        crate::preserves_rail::string(&entry.request_ref),
        crate::preserves_rail::string(&entry.command_ref),
        crate::preserves_rail::string(&entry.command_schema_ref),
    ])
}

fn snapshot_value(snapshot: &ReplicaSnapshot) -> preserves::IOValue {
    crate::preserves_rail::record("raft-replica-snapshot-v1", vec![
        crate::preserves_rail::string(&snapshot.snapshot_ref),
        crate::preserves_rail::string(&snapshot.group_binding_ref),
        crate::preserves_rail::string(&snapshot.membership_ref),
        crate::preserves_rail::u64_value(snapshot.config_epoch),
        crate::preserves_rail::u64_value(snapshot.fencing_epoch),
        crate::preserves_rail::u64_value(snapshot.last_included_index),
        crate::preserves_rail::u64_value(snapshot.last_included_term),
        crate::preserves_rail::string(&snapshot.application_state_ref),
    ])
}
