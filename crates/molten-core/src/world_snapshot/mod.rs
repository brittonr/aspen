//! Pure execution-snapshot profile, compatibility, restore, and clone planning.

mod model;
mod validation;

pub use model::*;
pub use validation::*;

#[cfg(test)]
mod tests;
