#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-snapshot shell owns canonical records and later narrow effect adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-snapshot protocol names stay explicit at the product boundary"
)]

mod records;

pub use records::*;

#[cfg(test)]
mod tests;
