#![allow(
    tigerstyle::non_trait_imports,
    reason = "world distribution shell adapters keep generic mechanism boundaries and effect order explicit"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world distribution protocol names remain explicit at the product boundary"
)]

mod bridge;
mod catalog;
mod claims;
mod records;
mod retention;
mod service;
mod status;

pub use bridge::*;
pub use catalog::*;
pub use claims::*;
pub use records::*;
pub use retention::*;
pub use service::*;
pub use status::*;

#[cfg(test)]
mod tests;
