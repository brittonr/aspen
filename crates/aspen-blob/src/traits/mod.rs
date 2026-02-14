//! Blob storage traits following the Interface Segregation Principle.
//!
//! This module provides focused sub-traits for blob storage operations:
//!
//! - [`BlobWrite`]: Write operations (add, protect, unprotect)
//! - [`BlobRead`]: Read operations (get, has, status, reader)
//! - [`BlobTransfer`]: Network/P2P operations (ticket, download)
//! - [`BlobQuery`]: Query operations (list, wait_available)
//!
//! The composite [`BlobStore`] trait requires all sub-traits for implementations
//! that provide full blob store functionality.

mod query;
mod read;
mod transfer;
mod write;

pub use query::BlobQuery;
pub use read::BlobRead;
pub use transfer::BlobTransfer;
pub use write::BlobWrite;

/// Composite trait for full blob store functionality.
///
/// This trait is automatically implemented for any type that implements
/// all four sub-traits: [`BlobWrite`], [`BlobRead`], [`BlobTransfer`], and [`BlobQuery`].
///
/// Use this trait when you need access to all blob store operations.
/// Use the individual sub-traits when you only need a subset of operations,
/// following the Interface Segregation Principle.
pub trait BlobStore: BlobWrite + BlobRead + BlobTransfer + BlobQuery {}

/// Blanket implementation for any type implementing all sub-traits.
impl<T: BlobWrite + BlobRead + BlobTransfer + BlobQuery> BlobStore for T {}
