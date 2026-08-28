#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the pure adoption boundary keeps product and Basalt authority DTO names explicit for audit"
)]

//! Pure world-branch authority planning and activation admission.

mod activation;
mod mapping;
mod model;
mod promotion;

pub use activation::*;
pub use mapping::*;
pub use model::*;
pub use promotion::*;

#[cfg(test)]
mod tests;
