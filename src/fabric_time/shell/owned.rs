use super::*;

#[derive(Debug)]
pub enum SchedulerRuntimeError {
    Activation(capacity::ActivationError),
    Admission(Failure),
    Core(SchedulerError),
    Capacity(molten_core::fabric_time::capacity::UseDecisionKind),
    Prepare(crate::fabric::FabricPortError),
    /// The transition is committed. The caller must not blindly retry it.
    CommittedEffectUncertain(crate::fabric::FabricPortError),
}

impl From<SchedulerRuntimeError> for Failure {
    fn from(error: SchedulerRuntimeError) -> Self {
        Failure::invalid_harness(format!("scheduler runtime: {error:?}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerChoice {
    pub index: usize,
    pub choice_sequence: u64,
    pub eligible_count: u64,
}

/// One admitted owner for state, bounded backing, accounting and effect order.
pub struct SchedulerRuntime<P: SchedulerWakePort> {
    context: ExtensionTimeContext,
    profile: AdmittedTimeProfile,
    capacity: capacity::Runtime,
    effects: P,
}

impl<P: SchedulerWakePort> SchedulerRuntime<P> {
    pub fn activate(
        context: ExtensionTimeContext,
        profile: AdmittedTimeProfile,
        effects: P,
    ) -> std::result::Result<Self, SchedulerRuntimeError> {
        ensure_bound_profile(&context.scheduler_profile, &profile, "scheduler")
            .map_err(SchedulerRuntimeError::Admission)?;
        let capacity =
            capacity::Runtime::activate(&profile, context.generation).map_err(SchedulerRuntimeError::Activation)?;
        Ok(Self {
            context,
            profile,
            capacity,
            effects,
        })
    }

    pub fn state(&self) -> &SchedulerState {
        &self.capacity.scheduler
    }
    pub fn capacity_state(&self) -> &molten_core::fabric_time::capacity::UseState {
        self.capacity.state()
    }
    pub fn next_occurrence_sequence(&self) -> std::result::Result<u64, SchedulerRuntimeError> {
        self.stage_capacity(0, 0)?;
        self.state().next_occurrence_sequence().map_err(SchedulerRuntimeError::Core)
    }

    pub fn resolve_wake(&self, token: SchedulerWakeToken) -> Option<&RunnableState> {
        let capacity = self.capacity_state();
        if capacity.is_released
            || capacity.generation != self.state().generation
            || capacity.profile_ref != self.state().profile_ref
            || capacity.runnable_usage == 0
            || capacity.queue_usage == 0
        {
            return None;
        }
        token.resolve(self.state())
    }

    pub fn effects_mut(&mut self) -> &mut P {
        &mut self.effects
    }
    #[cfg(test)]
    pub(crate) fn backing_capacity(&self) -> (usize, usize) {
        (self.state().runnables.capacity(), self.capacity.ready.capacity())
    }

    pub fn command(
        &mut self,
        command: SchedulerCommand,
    ) -> std::result::Result<SchedulerAction, SchedulerRuntimeError> {
        self.check_usage()?;
        let key = scheduler_command_key(&command);
        // The same context/core decision serves snapshot and owning consumers.
        let decision =
            self.context.preview_command(&self.profile, self.profile.scheduler_policy, self.state(), &command)?;
        let action = decision.action;
        let (runnable_delta, queue_delta) = decision.usage_change(self.state());
        let staged = self.stage_capacity(runnable_delta, queue_delta)?;
        let index = decision.occurrence_index(self.state());
        let prepared = if matches!(action, SchedulerAction::Woken | SchedulerAction::Yielded) {
            Some(self.effects.prepare(key, index).map_err(SchedulerRuntimeError::Prepare)?)
        } else {
            None
        };
        // No fallible operation remains between these publications. Exclusive &mut
        // access prevents a receiver observing half a scheduler/capacity commit.
        decision.apply(&mut self.capacity.scheduler, command);
        molten_core::fabric_time::capacity::publish_use(&mut self.capacity.state, staged);
        self.update_ready(index, queue_delta);
        if let Some(prepared) = prepared {
            let committed = SchedulerWakeCommit {
                scheduler: &self.capacity.scheduler,
                capacity: &self.capacity.state,
                occurrence: index,
            };
            self.effects.deliver(prepared, committed).map_err(SchedulerRuntimeError::CommittedEffectUncertain)?;
        }
        Ok(action)
    }

    pub fn choose(
        &mut self,
        recorded: Option<&RunnableKey>,
    ) -> std::result::Result<SchedulerChoice, SchedulerRuntimeError> {
        self.check_usage()?;
        let decision = self.context.preview_choice(
            &self.profile,
            self.profile.scheduler_policy,
            self.state(),
            recorded,
            Some(&self.capacity.ready),
        )?;
        let staged = self.stage_capacity(0, -1)?;
        let choice = SchedulerChoice {
            index: decision.index,
            choice_sequence: decision.choice_sequence,
            eligible_count: decision.eligible_count,
        };
        decision.apply(&mut self.capacity.scheduler);
        molten_core::fabric_time::capacity::publish_use(&mut self.capacity.state, staged);
        self.update_ready(choice.index, -1);
        Ok(choice)
    }

    pub fn cleanup_generation(&mut self, generation: u64) -> std::result::Result<(), SchedulerRuntimeError> {
        self.check_usage()?;
        let mut active = 0_i64;
        let mut ready = 0_i64;
        for runnable in self.state().runnables.iter().flatten() {
            if runnable.key.generation == generation
                && matches!(runnable.phase, RunnablePhase::Ready | RunnablePhase::Running | RunnablePhase::Blocked)
            {
                active += 1;
                ready += i64::from(runnable.phase == RunnablePhase::Ready);
            }
        }
        let staged = self.stage_capacity(-active, -ready)?;
        cleanup_scheduler_generation_in_place(&mut self.capacity.scheduler, generation)
            .map_err(SchedulerRuntimeError::Core)?;
        self.capacity.ready.retain(|index| {
            self.capacity.scheduler.runnables[*index]
                .as_ref()
                .is_some_and(|runnable| runnable.phase == RunnablePhase::Ready)
        });
        molten_core::fabric_time::capacity::publish_use(&mut self.capacity.state, staged);
        Ok(())
    }

    /// Release only after quiescence; retained history stays bounded until drop.
    pub fn release(&mut self) -> std::result::Result<(), SchedulerRuntimeError> {
        self.check_usage()?;
        if self.capacity_state().runnable_usage != 0 {
            return Err(SchedulerRuntimeError::Admission(Failure::invalid_harness(
                "scheduler release requires quiescence",
            )));
        }
        self.capacity.ready = Vec::new();
        self.capacity.state.is_released = true;
        Ok(())
    }

    fn update_ready(&mut self, index: usize, delta: i64) {
        if delta > 0 {
            self.capacity.ready.push(index);
        }
        if delta < 0 {
            self.capacity.ready.retain(|entry| *entry != index);
        }
    }

    fn stage_capacity(
        &self,
        runnable_delta: i64,
        queue_delta: i64,
    ) -> std::result::Result<(u64, u64), SchedulerRuntimeError> {
        molten_core::fabric_time::capacity::preview_use(
            self.capacity.plan(),
            self.capacity_state(),
            &self.profile.profile_ref,
            self.context.generation,
            molten_core::fabric_time::capacity::UseRequest {
                runnable_delta,
                queue_delta,
            },
        )
        .map_err(SchedulerRuntimeError::Capacity)
    }

    fn check_usage(&self) -> std::result::Result<(), SchedulerRuntimeError> {
        self.stage_capacity(0, 0)?;
        let (mut active, mut ready, mut retained) = (0_usize, 0_usize, 0_usize);
        for runnable in self.state().runnables.iter().flatten() {
            match runnable.phase {
                RunnablePhase::Ready => {
                    active += 1;
                    ready += 1;
                }
                RunnablePhase::Running | RunnablePhase::Blocked => active += 1,
                RunnablePhase::Completed | RunnablePhase::Cancelled => retained += 1,
            }
        }
        if u64::try_from(active).ok() != Some(self.capacity_state().runnable_usage)
            || u64::try_from(ready).ok() != Some(self.capacity_state().queue_usage)
            || ready != self.capacity.ready.len()
            || u64::try_from(retained)
                .ok()
                .is_none_or(|count| count > self.capacity.plan().terminal_retention_slots)
            || u64::try_from(self.state().runnables.len())
                .ok()
                .is_none_or(|count| count > self.capacity.plan().physical_runnable_slots)
            || self.state().terminal_retention_limit != self.capacity.plan().terminal_retention_slots
        {
            return Err(SchedulerRuntimeError::Admission(Failure::invalid_harness(
                "scheduler capacity disagrees with phases",
            )));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn state_mut_for_test(&mut self) -> &mut SchedulerState {
        &mut self.capacity.scheduler
    }

    #[cfg(test)]
    pub(crate) fn capacity_mut_for_test(&mut self) -> &mut molten_core::fabric_time::capacity::UseState {
        &mut self.capacity.state
    }

    #[cfg(test)]
    pub(crate) fn delivery_parts_for_test(&mut self, occurrence: usize) -> (SchedulerWakeCommit<'_>, &mut P) {
        (
            SchedulerWakeCommit {
                scheduler: &self.capacity.scheduler,
                capacity: &self.capacity.state,
                occurrence,
            },
            &mut self.effects,
        )
    }
}
