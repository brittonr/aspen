use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh_blobs::BlobsProtocol;
use molten_core::content_store_adapter::ContentReadGrant;
use tokio::sync::Semaphore;

#[derive(Debug)]
pub(super) struct ReadGatedBlobs {
    blobs: BlobsProtocol,
    grant: ContentReadGrant,
    manifest_ref: String,
    denied: Arc<AtomicU64>,
    connections: Arc<Semaphore>,
}

impl ReadGatedBlobs {
    pub(super) fn new(
        blobs: BlobsProtocol,
        grant: ContentReadGrant,
        manifest_ref: String,
        denied: Arc<AtomicU64>,
        maximum_connections: usize,
    ) -> Self {
        Self {
            blobs,
            grant,
            manifest_ref,
            denied,
            connections: Arc::new(Semaphore::new(maximum_connections)),
        }
    }
}

impl ProtocolHandler for ReadGatedBlobs {
    // r[impl molten.content_live.read_grant]
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        // The key is authenticated by Iroh. Permission comes from the separate
        // explicit operator grant, not from connection establishment or a pin.
        if !self.grant.allows(&self.manifest_ref, &connection.remote_id().to_string()) {
            self.denied.fetch_add(1, Ordering::Relaxed);
            connection.close(1_u32.into(), b"content-read-denied");
            return Ok(());
        }
        let Ok(_permit) = self.connections.clone().try_acquire_owned() else {
            connection.close(2_u32.into(), b"content-capacity-denied");
            return Ok(());
        };
        self.blobs.accept(connection).await
    }

    async fn shutdown(&self) {
        self.blobs.shutdown().await;
    }
}
