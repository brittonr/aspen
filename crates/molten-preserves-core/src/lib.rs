#![cfg_attr(not(test), no_std)]

//! Canonical packed Preserves: the byte-level identity kernel of Aspen.
//!
//! Aspen identity is BLAKE3 over canonical packed Preserves bytes. This crate owns that law without
//! `std` or the `preserves` crate:
//!
//! - `model`: the `PackedValue` tree, with sets and dictionaries keyed by canonical element bytes;
//! - `writer`: the canonical binary encoder (`0x80..0xB7`), byte-identical to
//!   `preserves::PackedWriter`;
//! - `reader`: the strict, bounded decoder that accepts exactly the canonical encoding of a value;
//! - `bounds`: the allocation-free shape preflight used before any decoding;
//! - `identity`: lowercase-only `blake3:` content references, domain-separated hashing, and the
//!   single `<none>` / `<some "blake3:...">` optional-reference encoding;
//! - `inspect`: `Embedded`-aware bounded structural traversal for marker and reference detection.
//!
//! Text syntax and `preserves::IOValue` interop stay in the `molten` shell (`src/preserves/`).
// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
// r[impl molten.project.molten_preserves_core_no_std_gate]

extern crate alloc;

pub mod bounds;
pub mod identity;
pub mod incremental;
pub mod inspect;
pub mod model;
pub mod reader;
pub mod tags;
pub mod writer;

#[cfg(test)]
mod tests;
