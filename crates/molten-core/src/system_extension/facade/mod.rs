//! Typed service, state-machine, and task facades over the system-extension
//! interface.
//!
//! A facade is a typed lowering on top of the existing callback vocabulary. It
//! adds no runtime, no I/O route, and no state store: the facade core lowers
//! `current state plus admitted event` into `next state, replies, effect
//! intents, and timer intents`, and the shell lowers those into the canonical
//! envelopes the extension interface already defines.

mod admission;
mod lowering;
mod semantics;
#[cfg(test)]
mod tests;

pub use admission::*;
pub use lowering::*;
pub use semantics::*;
