//! Test-owned DoltLite semantic-state oracle shell.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the shell keeps bounded-exec, DoltLite, and oracle protocol names explicit"
)]

mod ports;
mod process;
mod records;

pub use ports::*;
pub use process::*;
pub use records::*;

#[cfg(test)]
mod tests;
