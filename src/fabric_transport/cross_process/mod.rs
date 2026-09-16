//! Capability-scoped cross-process transport artifacts and Iroh shell.

mod canonical;
#[path = "effect/port.rs"]
mod effect_port;
#[path = "iroh/shell.rs"]
mod iroh_shell;

pub use canonical::*;
pub use effect_port::*;
pub use iroh_shell::*;

#[cfg(test)]
pub(crate) mod tests;
