impl DurableCommandShell for RedbDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<TransitionEnvelope> {
        let result = match command {
            DurablePortCommand::Append(request) => self.append(request),
            DurablePortCommand::Flush { generation, durability } => self.flush(*generation, *durability),
            DurablePortCommand::Truncate {
                generation,
                retain_from_sequence,
                authority_ref,
            } => self.truncate(*generation, *retain_from_sequence, authority_ref.as_deref()),
            DurablePortCommand::AtomicBatch(request) => self.apply_batch(request),
            DurablePortCommand::Snapshot { request, bytes } => self.create_snapshot(request, bytes),
            DurablePortCommand::RetireSupersededSnapshots {
                generation,
                keep_snapshot_ref,
                authority_ref,
            } => self.retire_superseded_snapshots(*generation, keep_snapshot_ref, authority_ref.as_deref()),
            DurablePortCommand::TruncateSnapshotCovered {
                generation,
                snapshot_ref,
                retain_from_sequence,
                authority_ref,
            } => self.truncate_snapshot_covered(
                *generation,
                snapshot_ref,
                *retain_from_sequence,
                authority_ref.as_deref(),
            ),
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}

impl DurableCommandShell for SimulatedDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<TransitionEnvelope> {
        let result = match command {
            DurablePortCommand::Append(request) => self.append(request, None),
            DurablePortCommand::Flush { generation, durability } => self.flush(*generation, *durability),
            DurablePortCommand::Truncate {
                generation,
                retain_from_sequence,
                authority_ref,
            } => self.truncate(*generation, *retain_from_sequence, authority_ref.as_deref()),
            DurablePortCommand::AtomicBatch(request) => self.apply_batch(request, None),
            DurablePortCommand::Snapshot { request, .. } => self.create_snapshot(request),
            DurablePortCommand::RetireSupersededSnapshots {
                generation,
                keep_snapshot_ref,
                authority_ref,
            } => self.retire_superseded_snapshots(*generation, keep_snapshot_ref, authority_ref.as_deref()),
            DurablePortCommand::TruncateSnapshotCovered {
                generation,
                snapshot_ref,
                retain_from_sequence,
                authority_ref,
            } => self.truncate_snapshot_covered(
                *generation,
                snapshot_ref,
                *retain_from_sequence,
                authority_ref.as_deref(),
            ),
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}