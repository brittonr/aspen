//! Peer exchange: local root-to-root sync and the Iroh blob publish/fetch adapter. Transport never
//! grants trust; every received manifest and chunk is re-verified against its content ref before it
//! is indexed.

pub(super) mod announce;
mod intake;
pub(super) mod landing;
pub(super) mod mirror;
mod token;
