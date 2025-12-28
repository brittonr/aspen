//! Git object handling for Forge.
//!
//! This module provides storage and manipulation of Git objects (blobs, trees,
//! commits, tags) using iroh-blobs as the backing store. All objects are
//! content-addressed by their BLAKE3 hash.
//!
//! ## Object Types
//!
//! - **Blob**: Raw file content
//! - **Tree**: Directory listing with mode, name, and hash for each entry
//! - **Commit**: Points to a tree with parent refs, author, and message
//! - **Tag**: Named reference to another object with optional message
//!
//! ## Storage Model
//!
//! Unlike traditional Git which uses SHA-1, Forge uses BLAKE3 for all hashing.
//! Objects are stored as postcard-serialized `SignedObject<GitObject>` in iroh-blobs.

mod object;
mod store;

pub use object::{GitObject, TreeEntry};
pub use store::GitBlobStore;
