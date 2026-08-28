//! Pure coordination-delivery policy and transition core.
//!
//! Base FIFO enqueue/dequeue semantics remain in `molten::coordination`. This
//! module owns only the separately versioned delivery extension. It consumes
//! supplied currentness, logical-time, policy, authority, resource, and
//! evidence facts. It performs no storage, clock, process, network, timer, or
//! observability effects.
//!
//! r[impl molten.coordination_delivery.versioned_extension]

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public delivery vocabulary stays explicit at the coordination boundary"
)]

mod identity;
mod model;
mod policy;
mod status;
mod transition;

pub use identity::*;
pub use model::*;
pub use policy::*;
pub use status::*;
pub use transition::*;

#[cfg(test)]
mod tests;
