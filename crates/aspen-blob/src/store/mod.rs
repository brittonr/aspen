//! Blob storage implementations.
//!
//! Provides `IrohBlobStore` and `InMemoryBlobStore` implementations
//! of the blob storage traits defined in `super::traits`.

mod async_read_seek;
mod errors;
mod iroh_store;
mod memory_store;

pub use async_read_seek::AsyncReadSeek;
pub use errors::*;
pub use iroh_store::IrohBlobStore;
pub use memory_store::InMemoryBlobStore;

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    // Integration tests require an actual iroh endpoint, so they're in tests/
}
