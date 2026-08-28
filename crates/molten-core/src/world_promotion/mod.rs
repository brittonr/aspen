//! Pure world promotion, release reservation, dispatch, and reconciliation policy.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "world promotion composes closed protocol and transactional reconciliation DTOs"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world promotion names remain explicit at the authority boundary"
)]

mod dispatch;
mod model;
mod observation;
mod planning;
mod reconciliation;

pub use dispatch::*;
pub use model::*;
pub use observation::*;
pub use planning::*;
pub use reconciliation::*;

#[cfg(test)]
mod tests;
