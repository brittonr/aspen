#![allow(
    tigerstyle::non_trait_imports,
    reason = "the benchmark shell composes explicit ports around the pure core"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world benchmark protocol names remain explicit at the product boundary"
)]

mod chaoscontrol;
mod fixture;
mod instrumentation;
mod ports;
mod projection;
mod records;
mod service;

pub use chaoscontrol::*;
pub use fixture::*;
pub use instrumentation::*;
pub use ports::*;
pub use projection::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
