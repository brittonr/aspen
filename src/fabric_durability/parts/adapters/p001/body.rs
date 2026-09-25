
impl SimulatedDurableStateAdapter {
    // r[impl molten.fabric_durability.live_sim_parity]
    pub fn new(profile: CanonicalDurableProfile, descriptor: DurableNamespaceDescriptor) -> crate::error::Result<Self> {
        if profile.profile.adapter_kind != DurableAdapterKind::DeterministicSimulation {
            return Err(crate::error::MoltenError::invalid_harness(
                "simulated durability adapter requires a deterministic-simulation profile",
            ));
        }
        validate_namespace_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| adapter_validation_error("simulated namespace", &issues))?;
        Ok(Self {
            profile,
            state: DurableState::empty(descriptor),
            simulated_ticks: 0,
        })
    }

    pub fn state(&self) -> &DurableState {
        &self.state
    }

    pub const fn simulated_ticks(&self) -> u64 {
        self.simulated_ticks
    }

    pub fn append(
        &mut self,
        request: &AppendRequest,
        fault: Option<&SimulatedDurabilityFault>,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        if matches!(
            fault,
            Some(SimulatedDurabilityFault::CrashBeforeMutation | SimulatedDurabilityFault::CapacityExhausted)
        ) {
            let operation = if matches!(fault, Some(SimulatedDurabilityFault::CapacityExhausted)) {
                "simulated-capacity-exhausted-before-append"
            } else {
                "simulated-crash-before-append"
            };
            return self.synthetic_transition(MutationOutcome::FailedBeforeMutation, operation, true, false);
        }
        let mut transition = append_log(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated append", &issues))?;
        if matches!(fault, Some(SimulatedDurabilityFault::ResponseLostAfterCommit)) {
            transition.outcome = MutationOutcome::Uncertain;
            transition.operation = "simulated-response-loss-after-append".to_string();
            transition.retry_safe = false;
            transition.reconciliation_required = true;
        }
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn flush(
        &mut self,
        generation: u64,
        durability: DurabilityLevel,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        let transition = flush_log(&self.profile.profile, &self.state, generation, durability)
            .map_err(|issues| adapter_validation_error("simulated flush", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn apply_batch(
        &mut self,
        request: &AtomicBatchRequest,
        fault: Option<&SimulatedDurabilityFault>,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        if matches!(
            fault,
            Some(SimulatedDurabilityFault::CrashBeforeMutation | SimulatedDurabilityFault::CapacityExhausted)
        ) {
            let operation = if matches!(fault, Some(SimulatedDurabilityFault::CapacityExhausted)) {
                "simulated-capacity-exhausted-before-batch"
            } else {
                "simulated-crash-before-batch"
            };
            return self.synthetic_transition(MutationOutcome::FailedBeforeMutation, operation, true, false);
        }
        let mut transition = apply_atomic_batch(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated batch", &issues))?;
        if matches!(fault, Some(SimulatedDurabilityFault::ResponseLostAfterCommit)) {
            transition.outcome = MutationOutcome::Uncertain;
            transition.operation = "simulated-response-loss-after-batch".to_string();
            transition.retry_safe = false;
            transition.reconciliation_required = true;
        }
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn truncate(
        &mut self,
        generation: u64,
        retain_from_sequence: u64,
        authority_ref: Option<&str>,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        let transition =
            truncate_log(&self.profile.profile, &self.state, generation, retain_from_sequence, authority_ref)
                .map_err(|issues| adapter_validation_error("simulated truncate", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn create_snapshot(&mut self, request: &SnapshotRequest) -> crate::error::Result<CanonicalDurableTransition> {
        let transition = create_snapshot(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated snapshot", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn apply_effect(
        &mut self,
        command: &EffectTransactionCommand,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        let transition = apply_effect_transaction(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("simulated effect", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn inject_fault(
        &mut self,
        fault: &SimulatedDurabilityFault,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        let transition = match fault {
            SimulatedDurabilityFault::DelayCompletion { ticks } => {
                self.simulated_ticks = self
                    .simulated_ticks
                    .checked_add(*ticks)
                    .ok_or_else(|| crate::error::MoltenError::invalid_harness("simulated durability time overflow"))?;
                DurableTransition {
                    next: self.state.clone(),
                    outcome: MutationOutcome::Validated,
                    operation: "simulated-latency".to_string(),
                    affected_items: 0,
                    affected_bytes: 0,
                    retry_safe: true,
                    reconciliation_required: false,
                }
            }
            SimulatedDurabilityFault::ProcessCrash => simulate_process_crash(&self.state),
            SimulatedDurabilityFault::CorruptSnapshot { snapshot_ref } => {
                let next = mark_snapshot_corrupt(&self.state, snapshot_ref)
                    .map_err(|issue| adapter_validation_error("corrupt snapshot", &[issue]))?;
                DurableTransition {
                    next,
                    outcome: MutationOutcome::FailedAfterPossibleMutation,
                    operation: "simulated-snapshot-corruption".to_string(),
                    affected_items: 1,
                    affected_bytes: 0,
                    retry_safe: false,
                    reconciliation_required: true,
                }
            }
            SimulatedDurabilityFault::CrashBeforeMutation
            | SimulatedDurabilityFault::ResponseLostAfterCommit
            | SimulatedDurabilityFault::CapacityExhausted => {
                return Err(crate::error::MoltenError::invalid_harness(
                    "operation-scoped durability fault requires append or batch execution",
                ));
            }
        };
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn recovery(&self, inventory: &RecoveryInventory) -> crate::error::Result<CanonicalRecoveryDecision> {
        canonical_recovery_decision(&self.profile, &self.state, evaluate_recovery(&self.state, inventory))
    }

    fn synthetic_transition(
        &self,
        outcome: MutationOutcome,
        operation: &str,
        retry_safe: bool,
        reconciliation_required: bool,
    ) -> crate::error::Result<CanonicalDurableTransition> {
        canonical_durable_transition(&self.profile, &DurableTransition {
            next: self.state.clone(),
            outcome,
            operation: operation.to_string(),
            affected_items: 0,
            affected_bytes: 0,
            retry_safe,
            reconciliation_required,
        })
    }
}

impl DurableCommandShell for RedbDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<CanonicalDurableTransition> {
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
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}

impl DurableCommandShell for SimulatedDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<CanonicalDurableTransition> {
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
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}

fn initialize_tables(database: &redb::Database) -> crate::error::Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        write.open_table(LOG_TABLE).map_err(adapter_error)?;
        write.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        write.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        write.open_table(EFFECT_TABLE).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}
