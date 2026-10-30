//! Pure collaboration membership and resource ownership, separate from node membership.
mod admission;
mod identity;
mod model;
mod transition;

pub use admission::*;
pub use identity::*;
pub use model::*;
pub use transition::*;

#[cfg(test)]
mod tests;
