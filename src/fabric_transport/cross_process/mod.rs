//! Capability-scoped cross-process transport artifacts and Iroh shell.

mod canonical;
mod iroh_shell;

pub use canonical::*;
pub use iroh_shell::*;

#[cfg(test)]
mod tests;
