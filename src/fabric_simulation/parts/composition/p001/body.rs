
impl DeterministicSimulationPortRouter {
    fn new(world: &CanonicalSimulatedWorld) -> Self {
        let profiles = world
            .admitted
            .manifest
            .port_profiles
            .iter()
            .cloned()
            .map(|profile| (profile.port_id.clone(), profile))
            .collect();
        Self {
            profiles,
            faults: world.admitted.manifest.faults.clone(),
            current_choice_position: FIRST_CHOICE_POSITION,
            current_virtual_tick: FIRST_VIRTUAL_TICK,
            dispatching_node: String::new(),
            dispatching_request_ref: String::new(),
            next_submission_ordinal: FIRST_WORKLOAD_SEQUENCE,
            resource_units: 0,
            max_resource_units: world.admitted.manifest.bounds.max_resource_units,
            events: Vec::new(),
            transport: SimulatedTransportState::new(),
            storage: SimulatedStorageState::new(),
            opened_partition_faults: std::collections::BTreeSet::new(),
        }
    }

    fn begin_choice(&mut self, position: u64, virtual_tick: u64) {
        self.current_choice_position = position;
        self.current_virtual_tick = virtual_tick;
    }

    fn set_dispatching(&mut self, node_id: &str, request_ref: &str) {
        self.dispatching_node = node_id.to_string();
        self.dispatching_request_ref = request_ref.to_string();
    }

    fn events(&self) -> &[CanonicalSimulationPortEvent] {
        &self.events
    }

    fn resource_units(&self) -> u64 {
        self.resource_units
    }

    fn transport(&self) -> &SimulatedTransportState {
        &self.transport
    }

    fn storage(&self) -> &SimulatedStorageState {
        &self.storage
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    // r[impl molten.fabric_simulation.stateful_storage]
    fn step_boundary_faults(&mut self) {
        self.transport.heal_ready_partitions(self.current_virtual_tick);
        let position = self.current_choice_position;
        let tick = self.current_virtual_tick;
        let mut openings = Vec::with_capacity(self.faults.len());
        for fault in &self.faults {
            if fault.kind != SimulationFaultKind::Partition
                || self.opened_partition_faults.contains(&fault.fault_id)
                || !self.fault_is_active(fault, position)
            {
                continue;
            }
            let heals_at_tick = fault
                .duration_choices
                .map_or(NEVER_HEALS_TICK, |duration| tick.checked_add(duration).unwrap_or(NEVER_HEALS_TICK));
            openings.push((fault.fault_id.clone(), SimulatedPartition {
                fault_id: fault.fault_id.clone(),
                destination: fault.target.clone(),
                heals_at_tick,
            }));
        }
        for (fault_id, partition) in openings {
            if self.transport.open_partition(partition).is_ok() {
                self.opened_partition_faults.insert(fault_id);
            }
        }
    }

    fn fault_is_active(&self, fault: &SimulationFaultAction, position: u64) -> bool {
        if position < fault.activate_at_choice {
            return false;
        }
        match fault.duration_choices {
            None => true,
            Some(duration) => fault.activate_at_choice.checked_add(duration).is_some_and(|end| position < end),
        }
    }

    fn active_fault(&self, profile: &SimulatedPortProfile) -> Option<&SimulationFaultAction> {
        self.faults.iter().find(|fault| {
            fault.boundary == profile.class
                && fault.target == profile.port_id
                && self.fault_is_active(fault, self.current_choice_position)
        })
    }

    fn next_submission_ordinal(&mut self) -> Result<u64, FabricPortError> {
        let ordinal = self.next_submission_ordinal;
        self.next_submission_ordinal = self
            .next_submission_ordinal
            .checked_add(UNIT_RESOURCE_COST)
            .ok_or_else(|| FabricPortError::malformed("simulation submission ordinal overflow"))?;
        Ok(ordinal)
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    fn submit_transmission(
        &mut self,
        profile: &SimulatedPortProfile,
        effect: &crate::system_extension::TypedEffectRequest,
        fault: Option<&SimulationFaultAction>,
    ) -> Result<String, FabricPortError> {
        let transmission_id = format!("{}:{}:{}", profile.port_id, effect.request_ref, self.current_choice_position);
        let fault_delay = fault
            .filter(|fault| fault.kind == SimulationFaultKind::Delay)
            .and_then(|fault| fault.duration_choices)
            .unwrap_or(UNIT_RESOURCE_COST);
        let eligible_at_tick = self
            .current_virtual_tick
            .checked_add(fault_delay)
            .ok_or_else(|| FabricPortError::malformed("simulation transport delay tick overflow"))?;
        let transmission = SimulatedTransmission {
            transmission_id: transmission_id.clone(),
            destination: self.dispatching_node.clone(),
            port_id: profile.port_id.clone(),
            request_ref: effect.request_ref.clone(),
            generation: effect.generation,
            submitted_at_tick: self.current_virtual_tick,
            eligible_at_tick,
        };
        self.transport
            .submit(transmission)
            .map_err(|error| FabricPortError::malformed(format!("simulation transport submit denied: {error:?}")))?;
        if fault.is_some_and(|fault| fault.kind == SimulationFaultKind::Drop)
            && self.transport.drop_transmission(&transmission_id).is_err()
        {
            return Err(FabricPortError::malformed("simulation dropped transmission disappeared"));
        }
        let fault_kind = fault.map_or("none", |fault| fault.kind.as_str());
        Ok(blake3_ref(format!("{transmission_id}:{SUBMISSION_ACK_MATERIAL}:{fault_kind}").as_bytes()))
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn submit_storage_operation(
        &mut self,
        profile: &SimulatedPortProfile,
        effect: &crate::system_extension::TypedEffectRequest,
        fault: Option<&SimulationFaultAction>,
    ) -> Result<String, FabricPortError> {
        let operation_id = format!("{}:{}:{}", profile.port_id, effect.request_ref, self.current_choice_position);
        let ordinal = self.next_submission_ordinal()?;
        let operation = SimulatedStorageOperation {
            operation_id: operation_id.clone(),
            port_id: profile.port_id.clone(),
            owner_node_id: self.dispatching_node.clone(),
            request_ref: self.dispatching_request_ref.clone(),
            phase: crate::core_api::world_faults::FaultPhase::AfterPossibleSubmit,
            submission_ordinal: ordinal,
            submitted_at_tick: self.current_virtual_tick,
            eligible_at_tick: self.current_virtual_tick,
        };
        self.storage
            .submit(operation)
            .map_err(|error| FabricPortError::malformed(format!("simulation storage submit denied: {error:?}")))?;
        if let Some(fault) = fault.filter(|fault| fault.kind == SimulationFaultKind::Delay) {
            let delay = fault.duration_choices.unwrap_or(DEFAULT_COMPLETION_DELAY_TICKS);
            let eligible_at_tick = self
                .current_virtual_tick
                .checked_add(delay)
                .ok_or_else(|| FabricPortError::malformed("simulation storage delay tick overflow"))?;
            self.storage
                .hold_completion(&operation_id, eligible_at_tick)
                .map_err(|error| FabricPortError::malformed(format!("simulation storage hold denied: {error:?}")))?;
        }
        Ok(blake3_ref(format!("{operation_id}:{SUBMISSION_ACK_MATERIAL}").as_bytes()))
    }

    fn push_event(&mut self, input: SimulationPortEventInput<'_>) -> Result<(), FabricPortError> {
        let event = canonical_simulation_port_event(input).map_err(FabricPortError::from)?;
        self.events.push(event);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn complete_storage_head(&mut self, operation_id: &str) -> Result<String, FabricPortError> {
        let entry = self
            .storage
            .complete(operation_id, self.current_virtual_tick)
            .map_err(|error| FabricPortError::malformed(format!("simulation storage completion denied: {error:?}")))?;
        let output_ref = blake3_ref(format!("{}:{COMPLETION_ACK_MATERIAL}", entry.entry_id).as_bytes());
        let profile = self
            .profiles
            .values()
            .find(|profile| profile.class == crate::fabric::FabricPortClass::DurableState)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("simulation world lacks its durable-state profile"))?;
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &entry.request_ref,
            output_ref: &output_ref,
            fault: None,
        })?;
        Ok(output_ref)
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    fn deliver_transmission(&mut self, transmission_id: &str) -> Result<String, FabricPortError> {
        let transmission = self
            .transport
            .deliver(transmission_id, self.current_virtual_tick)
            .map_err(|error| FabricPortError::malformed(format!("simulation transport delivery denied: {error:?}")))?;
        let output_ref = blake3_ref(format!("{}:{DELIVERY_ACK_MATERIAL}", transmission.transmission_id).as_bytes());
        let profile = self
            .profiles
            .values()
            .find(|profile| profile.class == crate::fabric::FabricPortClass::Transport)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("simulation world lacks its transport profile"))?;
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &transmission.request_ref,
            output_ref: &output_ref,
            fault: None,
        })?;
        Ok(output_ref)
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn apply_crash(&mut self, fault: &SimulationFaultAction) -> Result<ReferenceCrashRecovery, FabricPortError> {
        let lost_operations = self.storage.crash_and_recover();
        let durable_entry_request_refs = self.storage().durable_image().request_refs();
        let recovery_ref = blake3_ref(format!("{}:{CRASH_ACK_MATERIAL}", fault.fault_id).as_bytes());
        let profile =
            self.profiles.values().find(|profile| profile.class == fault.boundary).cloned().ok_or_else(|| {
                FabricPortError::malformed("simulation world lacks the crashed fault boundary profile")
            })?;
        let request_ref = blake3_ref(fault.fault_id.as_bytes());
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &request_ref,
            output_ref: &recovery_ref,
            fault: Some(fault.kind),
        })?;
        Ok(ReferenceCrashRecovery {
            fault_id: fault.fault_id.clone(),
            lost_operations,
            durable_entry_request_refs,
        })
    }
}
