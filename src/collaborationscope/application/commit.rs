use super::*;

struct Batch {
    next: State,
    reference: StateRef,
    outcomes: Vec<preserves::IOValue>,
}

// r[impl molten.collaboration_scope.membership_currentness]
// r[impl molten.collaboration_scope.resource_binding]
impl crate::fabric_consistency::raft::CommittedBatchHandler for Application {
    fn restore_snapshot(
        &mut self,
        snapshot: &crate::fabric_consistency::raft::SnapshotRestore,
    ) -> crate::error::Result<String> {
        if self.reference.as_str() == snapshot.application_state_ref {
            return crate::preserves_rail::canonical_hash(&self.status());
        }
        let value = self
            .values
            .materialize(&snapshot.application_state_ref, MAX_RECORD_BYTES)
            .map_err(|_| denied(Denial::Uncertain))?;
        let loaded = parse_state(&crate::preserves_rail::parse_canonical_bytes(&value.bytes)?)?;
        let reference = validated_state_ref(&loaded)?;
        if reference.as_str() != snapshot.application_state_ref
            || loaded.currentness.service_generation != self.state.currentness.service_generation
        {
            return Err(denied(Denial::Stale));
        }
        let receipt = receipt_value(self, &reference, &loaded, &[], Vec::new());
        let receipt_ref = self.store.commit(&self.reference, &loaded, &receipt)?;
        self.state = loaded;
        self.reference = reference;
        Ok(receipt_ref)
    }

    fn apply_batch(&mut self, commands: &[crate::fabric_consistency::raft::Command]) -> crate::error::Result<String> {
        if commands.is_empty() || commands.len() > MAX_OPERATIONS {
            return Err(denied(Denial::Bound));
        }
        let batch = self.plan_committed(commands)?;
        if batch.next == self.state {
            if commands.len() == 1 {
                return self.store.result(&commands[0].request_ref);
            }
            let results = commands
                .iter()
                .map(|command| self.store.result(&command.request_ref).map(crate::preserves_rail::string))
                .collect::<crate::error::Result<Vec<_>>>()?;
            return crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "collaboration-idempotent-batch-v1",
                results,
            ));
        }
        let receipt = receipt_value(self, &batch.reference, &batch.next, commands, batch.outcomes);
        let receipt_ref = self.store.commit(&self.reference, &batch.next, &receipt)?;
        self.state = batch.next;
        self.reference = batch.reference;
        self.publish_value(&state_value(&self.state))?;
        self.publish_value(&receipt)?;
        Ok(receipt_ref)
    }
}

impl Application {
    fn plan_committed(&mut self, commands: &[crate::fabric_consistency::raft::Command]) -> crate::error::Result<Batch> {
        let mut batch = Batch {
            next: self.state.clone(),
            reference: self.reference.clone(),
            outcomes: Vec::with_capacity(commands.len()),
        };
        for command in commands {
            if command.command_schema_ref != command_schema_ref()? {
                return Err(denied(Denial::Domain));
            }
            let value = self.materialize_command(&command.command_ref)?;
            let (request, decisions) = command::parse_proposal(&value)?;
            if request.reference.as_str() != command.request_ref {
                return Err(denied(Denial::Domain));
            }
            let outcome = if self.store.has_result(&command.request_ref)? {
                "duplicate"
            } else {
                match preview_mutation(&batch.next, &batch.reference, &request, &decisions) {
                    Ok(MutationPlan::Apply(applied)) => {
                        batch.next = applied;
                        advance_index(&mut batch.next)?;
                        "applied"
                    }
                    Ok(MutationPlan::Duplicate(_)) => "duplicate",
                    Err(reason) => {
                        // An ordered but stale/conflicting command is a denied
                        // collaboration transition, not a failed Raft application.
                        // Only engine progress advances; scope/binding state stays put.
                        advance_index(&mut batch.next)?;
                        reason.as_str()
                    }
                }
            };
            batch.reference = validated_state_ref(&batch.next)?;
            batch.outcomes.push(crate::preserves_rail::record("collaboration-command-outcome-v1", vec![
                crate::preserves_rail::string(&command.request_ref),
                crate::preserves_rail::string(outcome),
            ]));
        }
        Ok(batch)
    }
}

fn advance_index(state: &mut State) -> crate::error::Result<()> {
    state.currentness.committed_index =
        state.currentness.committed_index.checked_add(1).ok_or_else(|| denied(Denial::Bound))?;
    Ok(())
}

fn receipt_value(
    application: &Application,
    after: &StateRef,
    state: &State,
    commands: &[crate::fabric_consistency::raft::Command],
    outcomes: Vec<preserves::IOValue>,
) -> preserves::IOValue {
    crate::preserves_rail::record("collaboration-commit-receipt-v1", vec![
        crate::preserves_rail::string(application.reference.as_str()),
        crate::preserves_rail::string(after.as_str()),
        currentness_value(&state.currentness),
        crate::preserves_rail::string("machine-loss-durable"),
        crate::preserves_rail::sequence(
            commands.iter().map(|command| crate::preserves_rail::string(&command.request_ref)).collect(),
        ),
        crate::preserves_rail::record("collaboration-durability-profile-v1", vec![
            crate::preserves_rail::string(application.store.profile_ref()),
            crate::preserves_rail::string(application.store.mode()),
            crate::preserves_rail::string("not-exactly-once-or-global-revocation-or-confidentiality-or-release-proof"),
        ]),
        crate::preserves_rail::sequence(outcomes),
    ])
}
