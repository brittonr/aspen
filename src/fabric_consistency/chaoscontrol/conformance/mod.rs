//! Offline ChaosControl contract interoperability and bounded instrumentation
//! evidence admission. These APIs validate supplied data, not guest execution.

/// Exact immutable source cohort admitted by this optional consumer.
pub const PRODUCER_REVISION: &str = "ec8aa799bad4735bf1028a092ca39feb08fb4bb7";

pub mod admission;
pub mod wire;
