
// r[impl molten.fabric_simulation.protocol_projection]
// The protocol projections of one reference run.
//
// Every participant projects its initial state, each admitted workload
// transition, and each recovery from the durable image. Projections are
// canonical values from the reference extension's projector, never debug text.
// Work counters are recorded cumulatively at each workload step.

/// The per-participant progress a trace tracks.
#[derive(Debug, Clone, Copy, Default)]
struct ParticipantTrack {
    next_source_sequence: u64,
    applied_transitions: u64,
    generation: u64,
}

/// The projections and work counters of one reference run.
#[derive(Debug, Default)]
struct ProtocolTrace {
    projections: Vec<molten_core::protocol_oracle::ProtocolProjection>,
    tracks: std::collections::BTreeMap<String, ParticipantTrack>,
    counters: Vec<molten_core::protocol_oracle::CounterObservation>,
    next_run_sequence: u64,
    projected_bytes: u64,
}

/// One workload step a trace projects.
struct StepProjection<'a> {
    step: &'a SimulationWorkloadStep,
    choice: &'a EligibleChoice,
    operations: &'a [(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)],
    transition: &'a ReferenceServiceTransition,
    history_ref: &'a str,
    router: &'a DeterministicPortRouter,
}

/// One projection the trace records.
struct ProjectionRecord<'a> {
    participant: &'a str,
    generation: u64,
    choice_ref: String,
    class: molten_core::protocol_oracle::TransitionClass,
    state: &'a ReferenceServiceState,
    history_ref: String,
    durability: molten_core::protocol_oracle::DurabilityFact,
    reported: Vec<String>,
}

/// The cumulative port and storage work of a run at one step.
struct WorkObservation {
    port_events: u64,
    storage_submissions: u64,
}

impl ProtocolTrace {
    /// Project every participant's initial state.
    fn begin(prepared: &PreparedReferenceWorld) -> crate::error::Result<Self> {
        let mut trace = Self::default();
        for (node_id, host) in &prepared.hosts {
            trace.record(ProjectionRecord {
                participant: node_id,
                generation: 0,
                choice_ref: blake3_ref(format!("protocol-initial:{node_id}").as_bytes()),
                class: molten_core::protocol_oracle::TransitionClass::Initial,
                state: host.executor().state(),
                history_ref: blake3_ref(FIRST_HISTORY_MATERIAL.as_bytes()),
                durability: molten_core::protocol_oracle::DurabilityFact::Durable,
                reported: Vec::new(),
            })?;
        }
        Ok(trace)
    }

    /// Record one projection and return its canonical ref.
    fn record(&mut self, record: ProjectionRecord<'_>) -> crate::error::Result<String> {
        let track = self.tracks.entry(record.participant.to_string()).or_default();
        if record.class == molten_core::protocol_oracle::TransitionClass::Applied {
            track.applied_transitions = track.applied_transitions.saturating_add(1);
        }
        track.generation = track.generation.max(record.generation);
        let state = reference_state_value(record.state);
        let projection = molten_core::protocol_oracle::ProtocolProjection {
            protocol: REFERENCE_PROTOCOL.to_string(),
            service: record.state.kind().as_str().to_string(),
            participant: record.participant.to_string(),
            generation: track.generation,
            source_sequence: track.next_source_sequence,
            run_sequence: self.next_run_sequence,
            scheduler_choice_ref: record.choice_ref,
            transition_class: record.class,
            logical_position: reference_logical_position(record.state, track.applied_transitions),
            state_ref: molten_core::protocol_oracle::state_ref_of(&state),
            state,
            physical_ref: None,
            history_ref: record.history_ref,
            durability: record.durability,
            progress: track.applied_transitions,
            completeness: molten_core::protocol_oracle::ObservationCompleteness::Complete,
            reported_invariants: record.reported,
        };
        track.next_source_sequence = track.next_source_sequence.saturating_add(1);
        self.next_run_sequence = self.next_run_sequence.saturating_add(1);
        let encoded = molten_core::protocol_oracle::encode_canonical(&molten_core::protocol_oracle::projection_value(&projection));
        let encoded_bytes = u64::try_from(encoded.len())
            .map_err(|_| crate::error::Failure::invalid_harness("protocol projection size overflow"))?;
        self.projected_bytes = self.projected_bytes.saturating_add(encoded_bytes);
        let projection_ref = molten_core::protocol_oracle::projection_ref(&projection);
        self.projections.push(projection);
        Ok(projection_ref)
    }

    /// Project one admitted workload transition and record the run's cumulative work.
    fn record_step(&mut self, input: &StepProjection<'_>) -> crate::error::Result<String> {
        let operation = input
            .operations
            .iter()
            .find(|(kind, request_ref, _)| *kind == input.step.service && *request_ref == input.step.request_ref)
            .map(|(_, _, operation)| operation)
            .ok_or_else(|| crate::error::Failure::invalid_harness("reference step names no registered operation"))?;
        let projection_ref = self.record(ProjectionRecord {
            participant: &input.step.node_id,
            generation: input.choice.generation,
            choice_ref: blake3_ref(input.choice.choice_id.as_bytes()),
            class: reference_transition_class(operation, input.transition.decision),
            state: &input.transition.next,
            history_ref: input.history_ref.to_string(),
            durability: molten_core::protocol_oracle::DurabilityFact::Volatile,
            reported: input.transition.semantic_invariants.iter().map(|name| (*name).to_string()).collect(),
        })?;
        self.record_work(&WorkObservation {
            port_events: u64::try_from(input.router.events().len())
                .map_err(|_| crate::error::Failure::invalid_harness("port event count overflow"))?,
            storage_submissions: input.router.next_submission_ordinal,
        });
        Ok(projection_ref)
    }

    /// Project every rebuilt participant after a recovery from the durable image.
    fn record_recovery(
        &mut self,
        prepared: &PreparedReferenceWorld,
        fault_id: &str,
        replayed: &std::collections::BTreeMap<String, u64>,
    ) -> crate::error::Result<()> {
        for (node_id, host) in &prepared.hosts {
            let track = self.tracks.entry(node_id.clone()).or_default();
            track.applied_transitions = replayed.get(node_id).copied().ok_or_else(|| {
                crate::error::Failure::invalid_harness(format!("recovery replayed no count for {node_id}"))
            })?;
            let generation = track.generation;
            self.record(ProjectionRecord {
                participant: node_id,
                generation,
                choice_ref: blake3_ref(format!("protocol-recovery:{fault_id}").as_bytes()),
                class: molten_core::protocol_oracle::TransitionClass::Recovered,
                state: host.executor().state(),
                history_ref: blake3_ref(format!("protocol-recovery:{fault_id}:{node_id}").as_bytes()),
                durability: molten_core::protocol_oracle::DurabilityFact::Recovered,
                reported: Vec::new(),
            })?;
        }
        Ok(())
    }

    /// Record the cumulative work counters after one workload step.
    fn record_work(&mut self, work: &WorkObservation) {
        let generation = self.tracks.values().map(|track| track.generation).fold(0, u64::max);
        for (name, value) in [
            ("transitions", self.next_run_sequence),
            ("port-events", work.port_events),
            ("storage-operations", work.storage_submissions),
            ("copied-bytes", self.projected_bytes),
        ] {
            self.counters.push(molten_core::protocol_oracle::CounterObservation {
                name: name.to_string(),
                generation,
                value,
            });
        }
    }
}

/// The recovery bookkeeping of one run: fired faults and recoveries.
struct RecoveryProgress {
    fired_recovery_faults: std::collections::BTreeSet<String>,
    crash_recoveries: Vec<ReferenceCrashRecovery>,
}

/// The workload, storage, and transport choices eligible at the scheduler's tick.
fn eligible_choices(
    next_step: Option<&SimulationWorkloadStep>,
    router: &DeterministicPortRouter,
    scheduler: &SimulationSchedulerState,
) -> Vec<EligibleChoice> {
    let mut eligible = Vec::with_capacity(8);
    eligible.extend(next_step.map(workload_choice));
    eligible.extend(router.storage().eligible_completions(scheduler.virtual_tick));
    eligible.extend(router.transport().eligible_deliveries(scheduler.virtual_tick));
    eligible
}

/// Applies every crash or restart fault whose activation choice or tick has been reached,
/// rebuilding the hosts from the durable image and projecting each recovered participant.
fn apply_due_recovery_faults(
    prepared: &mut PreparedReferenceWorld,
    router: &mut DeterministicPortRouter,
    scheduler: &SimulationSchedulerState,
    progress: &mut RecoveryProgress,
    trace: &mut ProtocolTrace,
) -> crate::error::Result<()> {
    for fault in prepared.world.admitted.manifest.faults.clone() {
        if !matches!(fault.kind, SimulationFaultKind::Crash | SimulationFaultKind::Restart)
            || (scheduler.next_choice_position < fault.activate_at_choice
                && scheduler.virtual_tick < fault.activate_at_choice)
        {
            continue;
        }
        progress.fired_recovery_faults.insert(fault.fault_id.clone());
        let recovery = router.apply_crash(&fault).map_err(|error| {
            crate::error::Failure::invalid_harness(format!("reference crash recovery denied: {error:?}"))
        })?;
        let replayed = rebuild_hosts_from_image(prepared, router.storage().durable_image())?;
        trace.record_recovery(prepared, &fault.fault_id, &replayed)?;
        crate::bounded::VecSink::push_item(&mut progress.crash_recoveries, recovery);
    }
    Ok(())
}

fn workload_choice(step: &SimulationWorkloadStep) -> EligibleChoice {
    EligibleChoice {
        kind: SchedulerChoiceKind::Runnable,
        choice_id: workload_choice_id(step.sequence),
        node_id: step.node_id.clone(),
        generation: INITIAL_EXTENSION_GENERATION,
        ready_at_tick: FIRST_VIRTUAL_TICK,
    }
}
