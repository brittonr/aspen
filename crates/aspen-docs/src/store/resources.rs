//! Core resource structs and their constructors.

use std::sync::Arc;

use iroh_docs::actor::SyncHandle;
use tracing::info;
use tracing::warn;

use super::DocsEventBroadcaster;
use super::DocsResources;
use super::DocsSyncResources;

impl DocsSyncResources {
    /// Create DocsSyncResources from DocsResources.
    ///
    /// This takes ownership of the Store and spawns a sync actor.
    /// After this, the Store cannot be accessed directly.
    ///
    /// **Note**: After creating DocsSyncResources, you must call `open_replica().await`
    /// before using `SyncHandleDocsWriter` to write entries.
    ///
    /// # Arguments
    /// * `resources` - DocsResources to convert (consumed)
    /// * `node_id` - Human-readable identifier for this node (used in logging)
    pub fn from_docs_resources(mut resources: DocsResources, node_id: &str) -> Self {
        // Import the author into the store before spawning the sync actor
        // This is required because SyncHandle::insert_local validates that authors exist
        if let Err(err) = resources.store.import_author(resources.author.clone()) {
            warn!(
                node_id,
                error = %err,
                "failed to import author into store (may already exist)"
            );
        }

        // Spawn the sync actor (takes ownership of store)
        let sync_handle = SyncHandle::spawn(
            resources.store,
            None, // No content status callback for now
            node_id.to_string(),
        );

        info!(
            node_id,
            namespace = %resources.namespace_id,
            "created DocsSyncResources with sync handle"
        );

        Self {
            sync_handle,
            namespace_id: resources.namespace_id,
            author: resources.author,
            docs_dir: resources.docs_dir,
            event_broadcaster: None,
        }
    }

    /// Set the event broadcaster for hook integration.
    ///
    /// When set, sync operations will emit `SyncStarted` and `SyncCompleted`
    /// events, enabling external hook programs to react to sync activity.
    #[must_use]
    pub fn with_event_broadcaster(mut self, broadcaster: Arc<DocsEventBroadcaster>) -> Self {
        self.event_broadcaster = Some(broadcaster);
        self
    }

    /// Get the event broadcaster, if any.
    pub fn event_broadcaster(&self) -> Option<&Arc<DocsEventBroadcaster>> {
        self.event_broadcaster.as_ref()
    }

    /// Create a protocol handler for accepting incoming sync connections.
    pub fn protocol_handler(&self) -> super::DocsProtocolHandler {
        super::DocsProtocolHandler::new(self.sync_handle.clone(), self.namespace_id, self.event_broadcaster.clone())
    }
}
