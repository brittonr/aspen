//! The replay observation boundary shell: the runtime source catalog and the
//! sealed observation adapter transitions read during replay.

mod sealed;
mod sources;

pub use sealed::*;
pub use sources::*;
