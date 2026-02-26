//! Network/P2P transfer operations for blob storage.
//!
//! The `BlobTransfer` trait provides methods for sharing and downloading blobs.

use async_trait::async_trait;
use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_blobs::ticket::BlobTicket;

use crate::store::BlobStoreError;
use crate::types::BlobRef;

/// Trait for network/P2P transfer operations on blob storage.
///
/// Provides methods for creating tickets to share blobs and
/// downloading blobs from remote peers.
#[async_trait]
pub trait BlobTransfer: Send + Sync {
    /// Generate a BlobTicket for sharing.
    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError>;

    /// Download a blob from a remote peer using a ticket.
    async fn download(&self, ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError>;

    /// Download a blob directly from a specific peer by hash.
    ///
    /// This is used for P2P sync when we know which peer has the blob.
    /// Default implementation returns an error for stores that don't support P2P.
    async fn download_from_peer(&self, hash: &Hash, _provider: PublicKey) -> Result<BlobRef, BlobStoreError> {
        Err(BlobStoreError::Download {
            message: format!("P2P download not supported for hash {}", hash.to_hex()),
        })
    }
}
