//! Application shell for system-extension time, scheduling, and entropy admission.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the shell composes explicit application-owned time and extension domain types"
)]

use super::*;
use crate::error::Failure;
use crate::error::Result;
use crate::fabric::FabricPortKey;
use crate::system_extension::Dispatcher;
use crate::system_extension::SystemExtensionExecutor;

mod owned;
mod reservation;

pub use owned::SchedulerChoice;
pub use owned::SchedulerRuntime;
pub use owned::SchedulerRuntimeError;
pub use reservation::*;

// r[impl molten.modularity.fabric_boundary.shell]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTimeContext {
    service_id: String,
    generation: u64,
    max_timers: u64,
    max_runnables: u64,
    max_concurrency: u64,
    max_entropy_request_bytes: u64,
    capability_refs: Vec<String>,
    timer_profile: Option<String>,
    scheduler_profile: Option<String>,
    entropy_profile: Option<String>,
}

impl ExtensionTimeContext {
    /// Application-owned admitted service, without an extension manifest envelope.
    /// System extensions must use `from_host` instead.
    pub fn for_application(service_id: String, generation: u64, profile: &AdmittedTimeProfile) -> Self {
        Self {
            service_id,
            generation,
            max_timers: profile.max_timers,
            max_runnables: profile.max_runnables,
            max_concurrency: profile.max_scheduler_concurrency,
            max_entropy_request_bytes: profile.max_entropy_request_bytes,
            capability_refs: Vec::new(),
            timer_profile: Some(profile.profile_id.clone()),
            scheduler_profile: Some(profile.profile_id.clone()),
            entropy_profile: None,
        }
    }

    pub fn from_host<E: SystemExtensionExecutor>(host: &Dispatcher<E>) -> Self {
        let manifest = host.manifest().manifest();
        Self {
            service_id: manifest.service_id.clone(),
            generation: host.state().generation,
            max_timers: manifest.resources.max_timers,
            max_runnables: manifest.resources.max_queued_events,
            max_concurrency: manifest.resources.max_concurrent_callbacks,
            max_entropy_request_bytes: manifest.resources.max_inflight_bytes,
            capability_refs: manifest.capability_refs.clone(),
            timer_profile: bound_port_profile(host, FABRIC_TIMER_PORT_ID),
            scheduler_profile: bound_port_profile(host, FABRIC_SCHEDULER_PORT_ID),
            entropy_profile: bound_port_profile(host, FABRIC_ENTROPY_PORT_ID),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &AdmittedTimeProfile,
        capability_refs: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            max_timers: profile.max_timers,
            max_runnables: profile.max_runnables,
            max_concurrency: profile.max_scheduler_concurrency,
            max_entropy_request_bytes: profile.max_entropy_request_bytes,
            capability_refs,
            timer_profile: Some(profile.profile_id.clone()),
            scheduler_profile: Some(profile.profile_id.clone()),
            entropy_profile: Some(profile.profile_id.clone()),
        }
    }

    // r[impl molten.fabric_time.live_sim_parity]
    pub fn schedule_timer(
        &self,
        profile: &AdmittedTimeProfile,
        active_timer_count: u64,
        request: &TimerScheduleRequest,
    ) -> Result<TimerState> {
        ensure_bound_profile(&self.timer_profile, profile, "timer")?;
        if request.key.service_id != self.service_id {
            return Err(Failure::invalid_harness("timer service identity mismatch"));
        }
        if active_timer_count >= self.max_timers {
            return Err(Failure::invalid_harness(format!(
                "system-extension timer limit {} exhausted",
                self.max_timers
            )));
        }
        super::schedule_timer(profile, self.generation, active_timer_count, request)
            .map_err(|error| core_error("system-extension timer admission", error))
    }

    pub fn apply_scheduler_command(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        command: &SchedulerCommand,
    ) -> Result<SchedulerTransition> {
        let decision = self.preview_command(profile, policy, state, command).map_err(Failure::from)?;
        let mut next = state.clone();
        let action = decision.apply(&mut next, command.clone());
        Ok(SchedulerTransition {
            next,
            action,
            runnable: scheduler_command_key(command).clone(),
        })
    }

    pub fn choose_runnable(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        recorded_choice: Option<&RunnableKey>,
    ) -> Result<SchedulerSelection> {
        let decision = self.preview_choice(profile, policy, state, recorded_choice, None).map_err(Failure::from)?;
        let selected = state.runnables[decision.index]
            .as_ref()
            .ok_or(SchedulerRuntimeError::Core(SchedulerError::NoReadyRunnable))
            .map_err(Failure::from)?
            .key
            .clone();
        let choice_sequence = decision.choice_sequence;
        let eligible_count = decision.eligible_count;
        let mut next = state.clone();
        decision.apply(&mut next);
        Ok(SchedulerSelection {
            next,
            selected,
            choice_sequence,
            eligible_count,
        })
    }

    fn preview_command(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        command: &SchedulerCommand,
    ) -> std::result::Result<SchedulerCommandDecision, SchedulerRuntimeError> {
        ensure_bound_profile(&self.scheduler_profile, profile, "scheduler")
            .map_err(SchedulerRuntimeError::Admission)?;
        if scheduler_command_key(command).service_id != self.service_id {
            return Err(SchedulerRuntimeError::Admission(Failure::invalid_harness(
                "runnable service identity mismatch",
            )));
        }
        let decision = preview_scheduler_command(profile, policy, state, self.generation, command)
            .map_err(SchedulerRuntimeError::Core)?;
        if decision.usage_change(state).0 > 0 {
            let active = state
                .runnables
                .iter()
                .flatten()
                .filter(|runnable| {
                    matches!(runnable.phase, RunnablePhase::Ready | RunnablePhase::Running | RunnablePhase::Blocked)
                })
                .count();
            if u64::try_from(active).map_err(|_| SchedulerRuntimeError::Core(SchedulerError::Overflow))?
                >= self.max_runnables
            {
                return Err(SchedulerRuntimeError::Admission(Failure::invalid_harness(
                    "system-extension runnable limit exhausted",
                )));
            }
        }
        Ok(decision)
    }

    fn preview_choice(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        recorded: Option<&RunnableKey>,
        ready: Option<&[usize]>,
    ) -> std::result::Result<SchedulerChoiceDecision, SchedulerRuntimeError> {
        ensure_bound_profile(&self.scheduler_profile, profile, "scheduler")
            .map_err(SchedulerRuntimeError::Admission)?;
        let decision = preview_runnable_choice(profile, policy, state, self.generation, recorded, ready)
            .map_err(SchedulerRuntimeError::Core)?;
        let running =
            state.runnables.iter().flatten().filter(|runnable| runnable.phase == RunnablePhase::Running).count();
        if u64::try_from(running).map_err(|_| SchedulerRuntimeError::Core(SchedulerError::Overflow))?
            >= self.max_concurrency
        {
            return Err(SchedulerRuntimeError::Admission(Failure::invalid_harness(
                "system-extension concurrency limit exhausted",
            )));
        }
        Ok(decision)
    }

    pub fn open_entropy_stream(
        &self,
        profile: &AdmittedTimeProfile,
        request: &EntropyStreamRequest,
    ) -> Result<EntropyStreamState> {
        ensure_bound_profile(&self.entropy_profile, profile, "entropy")?;
        if !self.capability_refs.contains(&request.capability_ref) {
            return Err(Failure::invalid_harness("entropy request lacks an admitted system-extension capability"));
        }
        super::open_entropy_stream(profile, self.generation, request)
            .map_err(|error| core_error("system-extension entropy stream", error))
    }

    pub fn admit_entropy_request(&self, request: EntropyRequest) -> Result<()> {
        let requested = request.requested_bytes();
        if requested > self.max_entropy_request_bytes {
            return Err(Failure::invalid_harness(format!(
                "entropy request {requested} exceeds system-extension byte envelope {}",
                self.max_entropy_request_bytes
            )));
        }
        Ok(())
    }

    pub fn cleanup_retired_timers(&self, states: &[TimerState], retired_generation: u64) -> Vec<TimerState> {
        cleanup_generation(states, retired_generation)
    }
}

fn bound_port_profile<E: SystemExtensionExecutor>(host: &Dispatcher<E>, port_id: &str) -> Option<String> {
    host.manifest()
        .binding_for(&FabricPortKey {
            port_id: port_id.to_string(),
            version: if port_id == FABRIC_SCHEDULER_PORT_ID {
                FABRIC_SCHEDULER_PORT_VERSION
            } else {
                FABRIC_TIME_PORT_VERSION
            }
            .to_string(),
        })
        .map(|binding| binding.binding.implementation_profile.clone())
}

fn ensure_bound_profile(bound_profile: &Option<String>, profile: &AdmittedTimeProfile, port: &str) -> Result<()> {
    let Some(bound_profile) = bound_profile else {
        return Err(Failure::invalid_harness(format!("system extension has no admitted {port} fabric port binding")));
    };
    if bound_profile != &profile.profile_id {
        return Err(Failure::invalid_harness(format!(
            "system extension {port} profile {bound_profile} does not admit requested profile {}",
            profile.profile_id
        )));
    }
    Ok(())
}

fn scheduler_command_key(command: &SchedulerCommand) -> &RunnableKey {
    match command {
        SchedulerCommand::Wake { key, .. }
        | SchedulerCommand::Cancel { key }
        | SchedulerCommand::Block { key }
        | SchedulerCommand::Yield { key }
        | SchedulerCommand::Complete { key } => key,
    }
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> Failure {
    Failure::invalid_harness(format!("{label} failed: {error:?}"))
}
