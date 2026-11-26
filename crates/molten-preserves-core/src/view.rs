//! Record readers over `PackedValue` that mirror the `molten` rail's `IOValue` parsers.
//!
//! Each reader accepts exactly the values the rail parser of the same name accepts, returns the
//! same result, and denies with a [`ViewError`] whose `Display` is the rail's `Failure` message and
//! whose class ([`crate::fault::Classified`]) is the rail's `Failure` class. The shell converts it
//! with `From<ViewError> for Failure` in `molten-node-host`; see [`crate::fault`] for how kernel
//! error enums wrap it.
//!
//! The rail keeps its own `IOValue` traversal rather than converting to `PackedValue` and calling
//! these readers. Rail parsers run on every record a shell decodes, and today borrow or shallowly
//! clone `IOValue` children; delegating would deep-copy the whole value, strings included, before
//! each read. Instead the rail builds its denials from [`ViewError`] and re-exports the typed ids,
//! so messages and id rules have one source, and a bounded differential property in the rail's
//! tests holds the two traversals to identical accept/deny decisions, results and messages.

pub use crate::shape::*;
