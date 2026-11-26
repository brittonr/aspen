//! Operations on one local chunk root: content-addressed writes, manifest and object reads, range
//! reads, pins, listings, and availability decisions. Each operation records a pass or denial
//! receipt in the Redb index.

pub(super) mod census;
pub(super) mod hold;
pub(super) mod ingest;
pub(super) mod readout;
pub(super) mod slice;
mod writer;
