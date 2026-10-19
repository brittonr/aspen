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

/// The owner's actual published fields at the external delivery boundary. This is
/// a borrowed observation, not a projected future state or separate authority.
#[derive(Clone, Copy)]
pub struct SchedulerWakeCommit<'a> {
    pub scheduler: &'a super::SchedulerState,
    pub capacity: &'a molten_core::fabric_time::capacity::UseState,
    pub occurrence: usize,
}

/// Preparation is fallible and has no externally visible effect. Delivery happens
/// only after scheduler and capacity publication; its error is an uncertain outcome,
/// never a denied command and never an instruction to retry automatically.
pub trait SchedulerWakePort {
    type Prepared;

    fn prepare(&mut self, key: &super::RunnableKey, index: usize) -> FabricPortResult<Self::Prepared>;
    fn deliver(&mut self, prepared: Self::Prepared, committed: SchedulerWakeCommit<'_>) -> FabricPortResult<()>;
}
