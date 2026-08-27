#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    tigerstyle::renamed_imports,
    reason = "the optional adapter keeps protocol DTO names and visible ChaosControl and VM Cohort namespaces at the mechanism translation boundary"
)]

mod cohort;
mod helpers;

pub use cohort::*;
