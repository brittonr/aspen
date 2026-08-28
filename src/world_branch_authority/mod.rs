#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the branch-authority shell keeps application port and protocol ownership visible at the effect boundary"
)]

mod ports;
mod promotion;
mod records;
mod service;

pub use ports::*;
pub use promotion::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
