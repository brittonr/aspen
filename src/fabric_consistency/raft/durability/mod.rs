use super::*;

#[derive(Debug)]
pub struct RedbReplicaPort {
    adapter: crate::fabric_durability::RedbDurableStateAdapter,
    durable_log_ref: String,
    snapshot_store_ref: String,
}

impl RedbReplicaPort {
    pub fn new(
        adapter: crate::fabric_durability::RedbDurableStateAdapter,
        durable_log_ref: String,
        snapshot_store_ref: String,
    ) -> crate::error::Result<Self> {
        crate::preserves_rail::validate_content_ref(&durable_log_ref)?;
        crate::preserves_rail::validate_content_ref(&snapshot_store_ref)?;
        Ok(Self {
            adapter,
            durable_log_ref,
            snapshot_store_ref,
        })
    }

    pub const fn adapter(&self) -> &crate::fabric_durability::RedbDurableStateAdapter {
        &self.adapter
    }

    pub fn durable_log_ref(&self) -> &str {
        &self.durable_log_ref
    }

    pub fn snapshot_store_ref(&self) -> &str {
        &self.snapshot_store_ref
    }

    /// The rejoin admission of this replica's storage, as a readback.
    ///
    /// Storage that serves admits the replica as a voter. Storage that is
    /// quarantined, repaired, or uncertain admits nothing on local facts alone:
    /// it needs protocol reconciliation, log or snapshot validation, current
    /// membership, and a resolved term and vote first. Startup recovery does
    /// not consult this yet: the peer-query recovery path it needs is open
    /// work in `recover-from-storage-faults`.
    // r[impl molten.consensus.repair_admission]
    pub fn storage_admission(&self) -> molten_core::consensus_recovery::RepairAdmission {
        let is_serving = self.adapter.health() == crate::fabric_durability::AdapterHealth::Serving;
        molten_core::consensus_recovery::admit_repaired_participant(
            &molten_core::consensus_recovery::RepairAdmissionInput {
                is_persistent_state_suspect: !is_serving || self.adapter.is_repair_observed(),
                is_storage_serving: is_serving,
                is_protocol_reconciled: false,
                is_log_or_snapshot_validated: false,
                is_membership_current: false,
                hard_state: molten_core::consensus_recovery::HardStateDecision::KeepLocal,
                current_term: 0,
            },
        )
    }

    /// Explicitly reconcile storage that a local repair reopened, by its own
    /// consistency, so startup recovery can read it. The repair stays
    /// recorded, and `storage_admission` keeps reporting the store as suspect.
    // r[impl molten.fabric_durability.adapter_quarantine]
    pub fn reconcile_repaired_storage(self) -> crate::error::Result<Self> {
        let repaired = crate::fabric_durability::AdapterHealth::Quarantined(
            crate::fabric_durability::QuarantineReason::RepairedOnOpen,
        );
        if self.adapter.health() != repaired {
            return Ok(self);
        }
        let Self {
            adapter,
            durable_log_ref,
            snapshot_store_ref,
        } = self;
        let adapter = adapter
            .reopen_and_reconcile(crate::fabric_durability::ReconciliationReference::SelfConsistent)
            .map_err(|failure| failure.error)?;
        if adapter.health() != crate::fabric_durability::AdapterHealth::Serving {
            return Err(crate::error::Failure::invalid_harness(format!(
                "repaired replica storage did not reconcile: {:?}",
                adapter.health()
            )));
        }
        Ok(Self {
            adapter,
            durable_log_ref,
            snapshot_store_ref,
        })
    }

    pub fn plan_recovery(&self, start_plan: ReplicaStartPlan) -> crate::error::Result<ReplicaPlan> {
        if !self.adapter.state().buffered_log.is_empty() {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft recovery denies while buffered durability records remain",
            ));
        }
        let latest_snapshot_ref = self
            .adapter
            .state()
            .snapshots
            .values()
            .max_by(|left, right| {
                (left.covered_log_sequence, &left.snapshot_ref).cmp(&(right.covered_log_sequence, &right.snapshot_ref))
            })
            .map(|snapshot| snapshot.snapshot_ref.clone());
        let snapshot_bytes = latest_snapshot_ref
            .as_deref()
            .map(|snapshot_ref| {
                self.adapter
                    .load_snapshot_bytes(snapshot_ref, self.adapter.state().descriptor.generation)
                    .map(|(_restore, bytes)| bytes)
            })
            .transpose()?;
        plan_replica(start_plan, &self.adapter.state().durable_log, snapshot_bytes.as_deref())
    }

    fn append_value(
        &mut self,
        value: preserves::IOValue,
        durability: crate::fabric_durability::DurabilityLevel,
    ) -> crate::error::Result<String> {
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let descriptor = &self.adapter.state().descriptor;
        let expected_sequence = self.adapter.state().next_log_sequence().map_err(|error| {
            crate::error::Failure::invalid_harness(format!("live Raft durable sequence denied: {error:?}"))
        })?;
        let request = crate::fabric_durability::AppendRequest {
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

impl ReplicaDurabilityEffects for RedbReplicaPort {
    fn persist_hard_state(&mut self, term: u64, voted_for: Option<&str>) -> crate::error::Result<String> {
        let vote = voted_for.map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |voter| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(voter)]),
        );
        self.append_value(
            crate::preserves_rail::record("raft-hard-state-v1", vec![crate::preserves_rail::u64_value(term), vote]),
            crate::fabric_durability::DurabilityLevel::MachineLoss,
        )
    }

    fn persist_entries(
        &mut self,
        truncate_from: Option<u64>,
        entries: &[ReplicatedEntry],
    ) -> crate::error::Result<String> {
        let truncate = truncate_from.map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |index| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(index)]),
        );
        self.append_value(
            crate::preserves_rail::record("raft-log-mutation-v1", vec![
                truncate,
                crate::preserves_rail::sequence(entries.iter().map(entry_value).collect()),
            ]),
            crate::fabric_durability::DurabilityLevel::Buffered,
        )
    }

    fn flush_log(&mut self, through_index: u64) -> crate::error::Result<String> {
        self.append_value(
            crate::preserves_rail::record("raft-log-flush-v1", vec![crate::preserves_rail::u64_value(through_index)]),
            crate::fabric_durability::DurabilityLevel::Buffered,
        )?;
        let generation = self.adapter.state().descriptor.generation;
        Ok(self
            .adapter
            .flush(generation, crate::fabric_durability::DurabilityLevel::MachineLoss)?
            .transition_ref)
    }

    fn persist_commit(&mut self, through_index: u64) -> crate::error::Result<String> {
        if through_index == INITIAL_COMMIT_INDEX {
            return Err(crate::error::Failure::invalid_harness("live Raft commit boundary must be positive"));
        }
        self.append_value(
            crate::preserves_rail::record("raft-commit-boundary-v1", vec![crate::preserves_rail::u64_value(
                through_index,
            )]),
            crate::fabric_durability::DurabilityLevel::MachineLoss,
        )
    }

    fn persist_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
        if snapshot.snapshot_ref != snapshot_ref(snapshot)? {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft snapshot identity mismatch before persistence",
            ));
        }
        let value = snapshot_value(snapshot);
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
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

pub(super) fn snapshot_value(snapshot: &ReplicaSnapshot) -> preserves::IOValue {
    crate::preserves_rail::record("raft-replica-snapshot-v1", vec![
        crate::preserves_rail::string(&snapshot.snapshot_ref),
        crate::preserves_rail::string(&snapshot.group_binding_ref),
        crate::preserves_rail::string(&snapshot.membership_ref),
        crate::preserves_rail::u64_value(snapshot.config_epoch),
        crate::preserves_rail::u64_value(snapshot.fencing_epoch),
        crate::preserves_rail::u64_value(snapshot.last_included_index),
        crate::preserves_rail::u64_value(snapshot.last_included_term),
        crate::preserves_rail::string(&snapshot.application_state_ref),
        crate::preserves_rail::sequence(
            snapshot
                .completed_requests
                .iter()
                .map(|(request_ref, index)| {
                    crate::preserves_rail::record("completed-request", vec![
                        crate::preserves_rail::string(request_ref),
                        crate::preserves_rail::u64_value(*index),
                    ])
                })
                .collect(),
        ),
    ])
}
