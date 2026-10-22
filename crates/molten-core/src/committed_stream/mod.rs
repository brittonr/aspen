//! The committed-change stream over committed service transitions.
//!
//! The stream is a durable, resumable observation surface. It is distinct from
//! the dataspace `Observe` surface, from durable delivery, and from the
//! world-promotion outbox: it records committed transitions and nothing else.
//!
//! Possessing an event, a cursor, or the stream authorizes nothing. Reading the
//! content an event references requires the existing current capability checks.

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public stream-prefixed types remain unambiguous when consumers import them outside this module"
)]
#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded stream decisions and local collections, never infrastructure adapters"
)]

mod identity;
mod issue;
mod model;
mod stream;

pub use identity::*;
pub use issue::*;
pub use model::*;
pub use stream::*;

#[cfg(test)]
mod tests;
