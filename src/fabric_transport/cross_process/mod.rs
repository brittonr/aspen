//! Capability-scoped cross-process transport artifacts and Iroh shell.

mod canonical;
mod effect_port;
mod iroh_shell;

pub use canonical::*;
pub use effect_port::*;
pub use iroh_shell::*;

#[cfg(test)]
mod tests;
