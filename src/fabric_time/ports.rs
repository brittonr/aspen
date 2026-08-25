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
