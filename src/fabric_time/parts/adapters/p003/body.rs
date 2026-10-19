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
    type Prepared = std::thread::Thread;

    fn prepare(&mut self, key: &super::RunnableKey, _index: usize) -> crate::fabric::FabricPortResult<Self::Prepared> {
        self.targets.get(key).cloned().ok_or_else(|| crate::fabric::FabricPortError::capability("no live scheduler wake target"))
    }

    fn deliver(&mut self, thread: Self::Prepared, _committed: super::SchedulerWakeCommit<'_>) -> crate::fabric::FabricPortResult<()> {
        thread.unpark();
        Ok(())
    }
}

/// Independent, explicitly drained deterministic effect transport. Occurrence
/// indices remain stable because the owner never purges or reuses terminal rows.
#[derive(Debug)]
pub struct SimulationSchedulerWakeAdapter {
    pending: std::collections::VecDeque<u64>,
    limit: usize,
}

impl SimulationSchedulerWakeAdapter {
    pub fn activate(profile: &super::AdmittedTimeProfile) -> crate::fabric::FabricPortResult<Self> {
        let limit = usize::try_from(profile.max_scheduler_queue_depth).map_err(|_| crate::fabric::FabricPortError::capability("simulation wake capacity is unrepresentable"))?;
        let mut pending = std::collections::VecDeque::new();
        pending.try_reserve_exact(limit).map_err(|_| crate::fabric::FabricPortError::capability("simulation wake allocation failed"))?;
        Ok(Self { pending, limit })
    }

    pub fn drain_next(&mut self) -> Option<u64> {
        self.pending.pop_front()
    }
}

impl super::SchedulerWakePort for SimulationSchedulerWakeAdapter {
    type Prepared = u64;

    fn prepare(&mut self, _key: &super::RunnableKey, index: usize) -> crate::fabric::FabricPortResult<Self::Prepared> {
        if self.pending.len() >= self.limit {
            return Err(crate::fabric::FabricPortError::capability("simulation wake delivery queue is full"));
        }
        u64::try_from(index)
            .map_err(|_| crate::fabric::FabricPortError::capability("simulation wake occurrence is unrepresentable"))
    }

    fn deliver(&mut self, index: Self::Prepared, _committed: super::SchedulerWakeCommit<'_>) -> crate::fabric::FabricPortResult<()> {
        self.pending.push_back(index);
        Ok(())
    }
}

fn run_scheduler_conformance(
    profile: &super::AdmittedTimeProfile,
    service_id: &str,
    generation: u64,
) -> crate::error::Result<(bool, bool)> {
    let runnable = super::RunnableKey { service_id: service_id.to_string(), generation, runnable_id: "adapter-conformance-runnable".to_string() };
    let context = super::ExtensionTimeContext::for_application(service_id.to_string(), generation, profile);
    match profile.kind {
        super::TimeProfileKind::Live => {
            let mut owner = super::SchedulerRuntime::activate(context, profile.clone(), ThreadSchedulerWakeAdapter::default())?;
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
            scheduler_owner_conformance(owner, &runnable, |owner| {
                let index = owner.effects_mut().drain_next().ok_or_else(|| crate::error::Failure::invalid_harness("simulation conformance delivery missing"))?;
                let index = usize::try_from(index)
                    .map_err(|_| crate::error::Failure::invalid_harness("simulation conformance index overflow"))?;
                if owner.state().runnables[index].phase != super::RunnablePhase::Ready || owner.capacity_state().queue_usage != 1 {
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
    let is_selected = &owner.state().runnables[selected.index].key == runnable;
    let cancelled = owner.command(super::SchedulerCommand::Cancel { key: runnable.clone() })?;
    Ok((is_selected, cancelled == super::SchedulerAction::Cancelled))
}