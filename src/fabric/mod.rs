//! Canonical Preserves projection for the pure fabric contracts.
//!
//! The in-memory models and validation laws live in `molten-core`. This module
//! assigns canonical Preserves schemas and BLAKE3 refs without performing I/O;
//! adapter shells decide whether and where admitted artifacts are persisted.

#[cfg(test)]
mod audit;
mod port;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric/parts/mod/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric/parts/mod/p001/body.rs"));
