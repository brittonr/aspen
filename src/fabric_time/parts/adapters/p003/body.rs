#[derive(Debug, Default)]
pub struct ThreadSchedulerWakeAdapter {
    targets: std::collections::BTreeMap<super::RunnableKey, std::thread::Thread>,
}

impl ThreadSchedulerWakeAdapter {
    pub fn register(&mut self, key: super::RunnableKey, thread: std::thread::Thread) -> crate::error::Result<()> {
        match self.targets.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => { entry.insert(thread); Ok(()) }
            std::collections::btree_map::Entry::Occupied(_) => Err(crate::error::Failure::invalid_harness("scheduler wake target already registered")),
        }
    }

    pub fn unregister(&mut self, key: &super::RunnableKey) -> bool {
        self.targets.remove(key).is_some()
    }

}

impl super::SchedulerWakePort for ThreadSchedulerWakeAdapter {
    type Prepared = (std::thread::Thread, super::SchedulerWakeToken);

    fn prepare(&mut self, key: &super::RunnableKey, index: usize) -> crate::fabric::FabricPortResult<Self::Prepared> {
        let thread = self.targets.get(key).cloned().ok_or_else(|| crate::fabric::FabricPortError::capability("no live scheduler wake target"))?;
        Ok((thread, super::SchedulerWakeToken { occurrence: index, generation: key.generation, occurrence_sequence: key.occurrence_sequence }))
    }

    fn deliver(&mut self, (thread, token): Self::Prepared, committed: super::SchedulerWakeCommit<'_>) -> crate::fabric::FabricPortResult<()> {
        if !committed.accepts(token) {
            return Err(crate::fabric::FabricPortError::capability("live scheduler wake token does not match committed occurrence"));
        }
        thread.unpark();
        Ok(())
    }
}

/// Independent, explicitly drained deterministic effect transport. Consumers
/// resolve occurrence-fenced tokens against their owning scheduler before use.
#[derive(Debug)]
pub struct SimulationSchedulerWakeAdapter {
    pending: std::collections::VecDeque<super::SchedulerWakeToken>,
    limit: usize,
}

impl SimulationSchedulerWakeAdapter {
    pub fn activate(profile: &super::AdmittedTimeProfile) -> crate::fabric::FabricPortResult<Self> {
        let limit = usize::try_from(profile.max_scheduler_queue_depth).map_err(|_| crate::fabric::FabricPortError::capability("simulation wake capacity is unrepresentable"))?;
        let mut pending = std::collections::VecDeque::new();
        pending.try_reserve_exact(limit).map_err(|_| crate::fabric::FabricPortError::capability("simulation wake allocation failed"))?;
        Ok(Self { pending, limit })
    }

    pub fn drain_next(&mut self) -> Option<super::SchedulerWakeToken> {
        self.pending.pop_front()
    }
}

impl super::SchedulerWakePort for SimulationSchedulerWakeAdapter {
    type Prepared = super::SchedulerWakeToken;

    fn prepare(&mut self, key: &super::RunnableKey, index: usize) -> crate::fabric::FabricPortResult<Self::Prepared> {
        if self.pending.len() >= self.limit {
            return Err(crate::fabric::FabricPortError::capability("simulation wake delivery queue is full"));
        }
        Ok(super::SchedulerWakeToken { occurrence: index, generation: key.generation, occurrence_sequence: key.occurrence_sequence })
    }

    fn deliver(&mut self, token: Self::Prepared, committed: super::SchedulerWakeCommit<'_>) -> crate::fabric::FabricPortResult<()> {
        if !committed.accepts(token) {
            return Err(crate::fabric::FabricPortError::capability("simulation scheduler wake token does not match committed occurrence"));
        }
        if self.pending.len() >= self.limit {
            return Err(crate::fabric::FabricPortError::capability("simulation wake delivery queue is full"));
        }
        self.pending.push_back(token);
        Ok(())
    }
}

fn run_scheduler_conformance(
    profile: &super::AdmittedTimeProfile,
    service_id: &str,
    generation: u64,
) -> crate::error::Result<(bool, bool)> {
    let runnable = |occurrence_sequence| super::RunnableKey { service_id: service_id.to_string(), generation, runnable_id: "adapter-conformance-runnable".to_string(), occurrence_sequence };
    let context = super::ExtensionTimeContext::for_application(service_id.to_string(), generation, profile);
    match profile.kind {
        super::TimeProfileKind::Live => {
            let mut owner = super::SchedulerRuntime::activate(context, profile.clone(), ThreadSchedulerWakeAdapter::default())?;
            let runnable = runnable(owner.next_occurrence_sequence().map_err(|_| crate::error::Failure::invalid_harness("scheduler conformance occurrence issuance exhausted"))?);
            let worker = std::thread::spawn(std::thread::park);
            let registration = owner.effects_mut().register(runnable.clone(), worker.thread().clone());
            let result = registration.and_then(|()| scheduler_owner_conformance(owner, &runnable, |_owner| Ok(())));
            if result.is_err() { worker.thread().unpark(); }
            worker.join().map_err(|_| crate::error::Failure::invalid_harness("scheduler conformance worker panicked"))?;
            result
        }
        super::TimeProfileKind::DeterministicSimulation => {
            let effects = SimulationSchedulerWakeAdapter::activate(profile)?;
            let owner = super::SchedulerRuntime::activate(context, profile.clone(), effects)?;
            let runnable = runnable(owner.next_occurrence_sequence().map_err(|_| crate::error::Failure::invalid_harness("scheduler conformance occurrence issuance exhausted"))?);
            scheduler_owner_conformance(owner, &runnable, |owner| {
                let token = owner.effects_mut().drain_next().ok_or_else(|| crate::error::Failure::invalid_harness("simulation conformance delivery missing"))?;
                if owner.resolve_wake(token).is_none() || owner.capacity_state().queue_usage != 1 {
                    return Err(crate::error::Failure::invalid_harness("simulation conformance delivery preceded commit"));
                }
                Ok(())
            })
        }
    }
}

fn scheduler_owner_conformance<P: super::SchedulerWakePort>(
    mut owner: super::SchedulerRuntime<P>,
    runnable: &super::RunnableKey,
    consume: impl FnOnce(&mut super::SchedulerRuntime<P>) -> crate::error::Result<()>,
) -> crate::error::Result<(bool, bool)> {
    owner.command(super::SchedulerCommand::Wake { key: runnable.clone(), priority: 0 })?;
    consume(&mut owner)?;
    let selected = owner.choose(Some(runnable))?;
    let is_selected = owner.state().runnables[selected.index].as_ref().is_some_and(|row| &row.key == runnable);
    let cancelled = owner.command(super::SchedulerCommand::Cancel { key: runnable.clone() })?;
    Ok((is_selected, cancelled == super::SchedulerAction::Cancelled))
}