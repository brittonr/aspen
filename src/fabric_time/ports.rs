//! Application-owned time and entropy capability contracts.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "time ports name explicit domain observations and typed port results"
)]

use super::TimeDomain;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.ports]

pub trait TimerClockAdapter {
    fn profile_ref(&self) -> &str;
    fn timer_domain(&self) -> TimeDomain;
    fn now_ticks(&mut self) -> FabricPortResult<u64>;
    fn await_ticks(&mut self, target_ticks: u64) -> FabricPortResult<u64>;
}

pub trait CryptographicEntropySource {
    fn source_id(&self) -> &'static str;
    fn fill_secret(&mut self, output: &mut [u8]) -> FabricPortResult<()>;
}

/// A deferred wake locator scoped to its owning port, not transferable authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerWakeToken {
    pub occurrence: usize,
    pub generation: u64,
    pub occurrence_sequence: u64,
}

impl SchedulerWakeToken {
    // r[impl molten.audit_f11.freshness]
    pub fn resolve<'a>(&self, state: &'a super::SchedulerState) -> Option<&'a super::RunnableState> {
        let runnable = state.runnables.get(self.occurrence)?.as_ref()?;
        (self.occurrence_sequence != 0
            && state.generation == self.generation
            && runnable.key.generation == self.generation
            && runnable.key.occurrence_sequence == self.occurrence_sequence
            && runnable.phase == super::RunnablePhase::Ready)
            .then_some(runnable)
    }
}

/// The owner's actual published fields at the external delivery boundary. This is
/// a borrowed observation, not a projected future state or separate authority.
#[derive(Clone, Copy)]
pub struct SchedulerWakeCommit<'a> {
    pub scheduler: &'a super::SchedulerState,
    pub capacity: &'a molten_core::fabric_time::capacity::UseState,
    pub occurrence: usize,
}

impl SchedulerWakeCommit<'_> {
    pub fn accepts(&self, token: SchedulerWakeToken) -> bool {
        self.occurrence == token.occurrence
            && !self.capacity.is_released
            && self.capacity.generation == token.generation
            && self.capacity.profile_ref == self.scheduler.profile_ref
            && self.capacity.runnable_usage > 0
            && self.capacity.queue_usage > 0
            && token.resolve(self.scheduler).is_some()
    }
}

/// Preparation is fallible and has no externally visible effect. Delivery happens
/// only after scheduler and capacity publication; its error is an uncertain outcome,
/// never a denied command and never an instruction to retry automatically.
pub trait SchedulerWakePort {
    type Prepared;

    fn prepare(&mut self, key: &super::RunnableKey, index: usize) -> FabricPortResult<Self::Prepared>;
    fn deliver(&mut self, prepared: Self::Prepared, committed: SchedulerWakeCommit<'_>) -> FabricPortResult<()>;
}
