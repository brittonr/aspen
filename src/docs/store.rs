//! iroh-docs Store management.
//!
//! Handles initialization, persistence, and loading of iroh-docs Store,
//! namespace secrets, and author secrets for the DocsExporter.
//!
//! # Persistence Strategy
//!
//! When using persistent storage:
//! - Store: Uses redb-backed storage in `{data_dir}/docs/`
//! - Namespace secret: Persisted to `{data_dir}/docs/namespace_secret`
//! - Author secret: Persisted to `{data_dir}/docs/author_secret`
//!
//! On first start, new secrets are generated and persisted.
//! On subsequent starts, existing secrets are loaded.
//!
//! # Tiger Style
//!
//! - Explicit file paths for all persistence
//! - Fail-fast on file I/O errors
//! - Fixed 32-byte secrets (64 hex characters)

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, EndpointAddr};
use iroh_docs::actor::SyncHandle;
use iroh_docs::net::{self, AcceptOutcome, ConnectError, SyncFinished};
use iroh_docs::store::Store;
use iroh_docs::{Author, NamespaceId, NamespaceSecret};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::constants::MAX_DOCS_CONNECTIONS;
use crate::blob::store::BlobStore;

/// File name for the iroh-docs redb database.
const STORE_DB_FILE: &str = "docs.redb";

/// File name for persisted namespace secret.
const NAMESPACE_SECRET_FILE: &str = "namespace_secret";

/// File name for persisted author secret.
const AUTHOR_SECRET_FILE: &str = "author_secret";

/// Resources needed for iroh-docs export.
///
/// Contains the Store, NamespaceId, and Author needed by IrohDocsWriter.
pub struct DocsResources {
    /// The iroh-docs store.
    pub store: Store,
    /// The namespace ID for the docs namespace.
    pub namespace_id: NamespaceId,
    /// The author for signing entries.
    pub author: Author,
    /// Path to the docs directory (None for in-memory).
    pub docs_dir: Option<PathBuf>,
}

/// Resources needed for iroh-docs P2P sync.
///
/// Contains the SyncHandle and NamespaceId needed for accepting incoming
/// sync connections. This takes ownership of the Store.
///
/// Note: When using DocsSyncResources, the Store is consumed by the SyncHandle
/// actor. You cannot use DocsResources.store after converting to DocsSyncResources.
pub struct DocsSyncResources {
    /// Sync handle for P2P synchronization.
    pub sync_handle: SyncHandle,
    /// The namespace ID for the docs namespace.
    pub namespace_id: NamespaceId,
    /// The author for signing entries.
    pub author: Author,
    /// Path to the docs directory (None for in-memory).
    pub docs_dir: Option<PathBuf>,
}

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
        }
    }

    /// Open the replica for reading and writing.
    ///
    /// This must be called before writing entries via `SyncHandleDocsWriter`.
    /// The replica is opened with sync enabled to allow P2P synchronization.
    pub async fn open_replica(&self) -> Result<()> {
        use iroh_docs::actor::OpenOpts;

        self.sync_handle
            .open(self.namespace_id, OpenOpts::default().sync())
            .await
            .context("failed to open replica")?;

        debug!(
            namespace = %self.namespace_id,
            "replica opened for sync"
        );

        Ok(())
    }

    /// Create a protocol handler for accepting incoming sync connections.
    pub fn protocol_handler(&self) -> DocsProtocolHandler {
        DocsProtocolHandler::new(self.sync_handle.clone(), self.namespace_id)
    }

    /// Initiate outbound sync to a peer.
    ///
    /// Connects to the specified peer and performs range-based set reconciliation
    /// to sync the namespace. This is the "Alice" side of the sync protocol.
    ///
    /// # Arguments
    /// * `endpoint` - The Iroh endpoint for establishing connections
    /// * `peer` - The peer's endpoint address (node ID + optional direct addresses)
    ///
    /// # Returns
    /// * `Ok(SyncFinished)` - Sync completed successfully with details
    /// * `Err(ConnectError)` - Connection or sync protocol failed
    pub async fn sync_with_peer(
        &self,
        endpoint: &Endpoint,
        peer: EndpointAddr,
    ) -> Result<SyncFinished, ConnectError> {
        debug!(
            peer = %peer.id.fmt_short(),
            namespace = %self.namespace_id,
            "initiating outbound sync"
        );

        let result = net::connect_and_sync(
            endpoint,
            &self.sync_handle,
            self.namespace_id,
            peer.clone(),
            None, // No metrics for now
        )
        .await;

        match &result {
            Ok(finished) => {
                info!(
                    peer = %finished.peer.fmt_short(),
                    namespace = %finished.namespace,
                    sent = finished.outcome.num_sent,
                    recv = finished.outcome.num_recv,
                    connect_ms = ?finished.timings.connect.as_millis(),
                    process_ms = ?finished.timings.process.as_millis(),
                    "outbound sync completed"
                );
            }
            Err(err) => {
                warn!(
                    peer = %peer.id.fmt_short(),
                    namespace = %self.namespace_id,
                    error = %err,
                    "outbound sync failed"
                );
            }
        }

        result
    }

    /// Subscribe to sync events and forward RemoteInsert entries to the DocsImporter.
    ///
    /// This spawns a background task that:
    /// 1. Subscribes to sync events for this namespace
    /// 2. Filters for RemoteInsert events (entries received from peers)
    /// 3. Fetches content from the blob store using the content hash
    /// 4. Forwards each entry to the DocsImporter for priority-based import
    ///
    /// # Arguments
    /// * `importer` - The DocsImporter to forward entries to
    /// * `source_cluster_id` - Identifier for the source cluster (for origin tracking)
    /// * `blob_store` - The blob store for fetching content by hash
    ///
    /// # Returns
    /// A CancellationToken that can be used to stop the event listener.
    pub async fn spawn_sync_event_listener(
        &self,
        importer: std::sync::Arc<super::importer::DocsImporter>,
        source_cluster_id: String,
        blob_store: Arc<crate::blob::store::IrohBlobStore>,
    ) -> Result<CancellationToken> {
        use iroh_docs::sync::Event;

        // Create channel for sync events
        let (tx, rx) = async_channel::bounded::<Event>(1000);

        // Subscribe to sync events for our namespace
        self.sync_handle
            .subscribe(self.namespace_id, tx)
            .await
            .context("failed to subscribe to sync events")?;

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let namespace_id = self.namespace_id;

        tokio::spawn(async move {
            info!(
                namespace = %namespace_id,
                source = %source_cluster_id,
                "sync event listener started"
            );

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!(
                            namespace = %namespace_id,
                            "sync event listener shutting down"
                        );
                        break;
                    }
                    event = rx.recv() => {
                        match event {
                            Ok(Event::RemoteInsert {
                                namespace: _,
                                entry,
                                from,
                                should_download,
                                remote_content_status,
                            }) => {
                                // Extract key from the signed entry
                                let key = entry.key().to_vec();
                                let content_hash = entry.content_hash();
                                let content_len = entry.content_len();

                                debug!(
                                    namespace = %namespace_id,
                                    key_len = key.len(),
                                    from = %hex::encode(&from[..8]),
                                    hash = %content_hash.fmt_short(),
                                    len = content_len,
                                    should_download = should_download,
                                    remote_status = ?remote_content_status,
                                    "received remote entry"
                                );

                                // Skip tombstones (empty content)
                                if content_len == 0 {
                                    debug!(
                                        namespace = %namespace_id,
                                        key = %String::from_utf8_lossy(&key),
                                        "skipping tombstone entry"
                                    );
                                    continue;
                                }

                                // Fetch content from blob store using the content hash
                                let content = match blob_store.get_bytes(&content_hash).await {
                                    Ok(Some(bytes)) => {
                                        debug!(
                                            namespace = %namespace_id,
                                            hash = %content_hash.fmt_short(),
                                            size = bytes.len(),
                                            "fetched content from blob store"
                                        );
                                        bytes.to_vec()
                                    }
                                    Ok(None) => {
                                        // Content not yet available locally
                                        // This can happen if should_download is true and we need to
                                        // fetch from the remote peer. For now, we skip and log.
                                        // TODO: Trigger blob download from peer when should_download=true
                                        warn!(
                                            namespace = %namespace_id,
                                            hash = %content_hash.fmt_short(),
                                            should_download = should_download,
                                            key = %String::from_utf8_lossy(&key),
                                            "content not found in blob store, skipping"
                                        );
                                        continue;
                                    }
                                    Err(e) => {
                                        warn!(
                                            namespace = %namespace_id,
                                            hash = %content_hash.fmt_short(),
                                            error = %e,
                                            "failed to fetch content from blob store"
                                        );
                                        continue;
                                    }
                                };

                                // Check for tombstone marker (single null byte indicates deletion)
                                if content.len() == 1 && content[0] == 0x00 {
                                    debug!(
                                        namespace = %namespace_id,
                                        key = %String::from_utf8_lossy(&key),
                                        "skipping tombstone marker entry"
                                    );
                                    continue;
                                }

                                // Forward to importer for priority-based import
                                match importer.process_remote_entry(
                                    &source_cluster_id,
                                    &key,
                                    &content,
                                ).await {
                                    Ok(result) => {
                                        debug!(
                                            namespace = %namespace_id,
                                            key = %String::from_utf8_lossy(&key),
                                            result = ?result,
                                            "processed remote entry"
                                        );
                                    }
                                    Err(e) => {
                                        warn!(
                                            namespace = %namespace_id,
                                            key = %String::from_utf8_lossy(&key),
                                            error = %e,
                                            "failed to import remote entry"
                                        );
                                    }
                                }
                            }
                            Ok(Event::LocalInsert { .. }) => {
                                // Ignore local inserts - we only care about remote
                            }
                            Err(e) => {
                                warn!(
                                    namespace = %namespace_id,
                                    error = %e,
                                    "sync event channel error, stopping listener"
                                );
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(cancel)
    }
}

// ============================================================================
// Background Sync Service
// ============================================================================

/// Interval between background sync attempts.
pub const BACKGROUND_SYNC_INTERVAL_SECS: u64 = 60;

/// Maximum concurrent outbound sync connections.
pub const MAX_OUTBOUND_SYNCS: usize = 5;

/// Background sync service that periodically syncs with discovered peers.
///
/// This service runs in the background and initiates outbound sync to peers
/// discovered via gossip or other mechanisms. It complements the inbound
/// sync handled by DocsProtocolHandler.
///
/// # Tiger Style
///
/// - Bounded concurrent syncs (MAX_OUTBOUND_SYNCS)
/// - Configurable sync interval
/// - Graceful shutdown via CancellationToken
pub struct DocsSyncService {
    /// Docs sync resources (SyncHandle, namespace, etc.)
    docs_sync: Arc<DocsSyncResources>,
    /// Iroh endpoint for outbound connections
    endpoint: Endpoint,
    /// Cancellation token for shutdown
    cancel: CancellationToken,
    /// Semaphore for limiting concurrent outbound syncs
    sync_semaphore: Arc<Semaphore>,
}

impl DocsSyncService {
    /// Create a new background sync service.
    ///
    /// # Arguments
    /// * `docs_sync` - The DocsSyncResources to use for syncing
    /// * `endpoint` - The Iroh endpoint for connections
    pub fn new(docs_sync: Arc<DocsSyncResources>, endpoint: Endpoint) -> Self {
        Self {
            docs_sync,
            endpoint,
            cancel: CancellationToken::new(),
            sync_semaphore: Arc::new(Semaphore::new(MAX_OUTBOUND_SYNCS)),
        }
    }

    /// Spawn the background sync service.
    ///
    /// Returns a cancellation token that can be used to stop the service.
    /// The service will periodically sync with provided peers.
    ///
    /// # Arguments
    /// * `peer_provider` - A function that returns the current list of peers to sync with
    pub fn spawn<F>(self: Arc<Self>, peer_provider: F) -> CancellationToken
    where
        F: Fn() -> Vec<EndpointAddr> + Send + Sync + 'static,
    {
        let cancel = self.cancel.clone();
        let service = self.clone();

        tokio::spawn(async move {
            info!(
                namespace = %service.docs_sync.namespace_id,
                interval_secs = BACKGROUND_SYNC_INTERVAL_SECS,
                "docs sync service started"
            );

            let mut interval = tokio::time::interval(Duration::from_secs(BACKGROUND_SYNC_INTERVAL_SECS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = service.cancel.cancelled() => {
                        info!("docs sync service shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        let peers = peer_provider();
                        if peers.is_empty() {
                            debug!("no peers available for docs sync");
                            continue;
                        }

                        debug!(peer_count = peers.len(), "starting background sync round");
                        service.sync_with_peers(peers).await;
                    }
                }
            }
        });

        cancel
    }

    /// Sync with a list of peers concurrently (bounded by semaphore).
    async fn sync_with_peers(&self, peers: Vec<EndpointAddr>) {
        let mut handles = Vec::new();

        for peer in peers {
            let docs_sync = self.docs_sync.clone();
            let endpoint = self.endpoint.clone();
            let semaphore = self.sync_semaphore.clone();

            let handle = tokio::spawn(async move {
                // Acquire permit before starting sync
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("sync semaphore closed");
                        return;
                    }
                };

                // Perform sync (errors are logged inside sync_with_peer)
                let _ = docs_sync.sync_with_peer(&endpoint, peer).await;
            });

            handles.push(handle);
        }

        // Wait for all sync tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Trigger an immediate sync with a specific peer.
    ///
    /// This bypasses the background interval and syncs immediately.
    pub async fn sync_now(&self, peer: EndpointAddr) -> Result<SyncFinished, ConnectError> {
        self.docs_sync.sync_with_peer(&self.endpoint, peer).await
    }

    /// Shutdown the sync service.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

// ============================================================================
// Protocol Handler for Incoming Sync Connections
// ============================================================================

/// Protocol handler for iroh-docs P2P sync connections.
///
/// Implements `ProtocolHandler` to accept incoming sync connections from
/// remote peers. Uses `iroh_docs::net::handle_connection` for the actual
/// sync protocol implementation (range-based set reconciliation).
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore (MAX_DOCS_CONNECTIONS)
/// - Explicit access control via accept callback
/// - Clean shutdown via ProtocolHandler::shutdown()
#[derive(Debug)]
pub struct DocsProtocolHandler {
    /// Sync handle for coordinating with the replica store.
    sync_handle: SyncHandle,
    /// Our namespace ID (used for access control decisions).
    namespace_id: NamespaceId,
    /// Connection semaphore for bounded resources.
    connection_semaphore: Arc<Semaphore>,
}

impl DocsProtocolHandler {
    /// Create a new docs protocol handler.
    ///
    /// # Arguments
    /// * `sync_handle` - Handle to the sync actor for replica coordination
    /// * `namespace_id` - The namespace ID this node is serving
    pub fn new(sync_handle: SyncHandle, namespace_id: NamespaceId) -> Self {
        Self {
            sync_handle,
            namespace_id,
            connection_semaphore: Arc::new(Semaphore::new(MAX_DOCS_CONNECTIONS as usize)),
        }
    }
}

#[allow(refining_impl_trait)]
impl ProtocolHandler for DocsProtocolHandler {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl std::future::Future<Output = Result<(), AcceptError>> + Send + '_ {
        let sync_handle = self.sync_handle.clone();
        let namespace_id = self.namespace_id;
        let semaphore = self.connection_semaphore.clone();

        async move {
            let remote_peer = connection.remote_id();

            // Try to acquire a connection permit
            let permit = match semaphore.try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    warn!(
                        peer = %remote_peer.fmt_short(),
                        max = MAX_DOCS_CONNECTIONS,
                        "docs sync connection limit reached, rejecting"
                    );
                    return Err(AcceptError::from_err(std::io::Error::other(
                        "connection limit reached",
                    )));
                }
            };

            debug!(
                peer = %remote_peer.fmt_short(),
                namespace = %namespace_id,
                "accepting docs sync connection"
            );

            // Handle the connection using iroh-docs sync protocol
            let result = net::handle_connection(
                sync_handle,
                connection,
                |requested_namespace, peer| {
                    // Access control: only allow sync for our namespace
                    async move {
                        if requested_namespace == namespace_id {
                            debug!(
                                peer = %peer.fmt_short(),
                                namespace = %requested_namespace,
                                "accepting sync request"
                            );
                            AcceptOutcome::Allow
                        } else {
                            warn!(
                                peer = %peer.fmt_short(),
                                requested = %requested_namespace,
                                our_namespace = %namespace_id,
                                "rejecting sync request for unknown namespace"
                            );
                            AcceptOutcome::Reject(net::AbortReason::NotFound)
                        }
                    }
                },
                None, // No metrics for now
            )
            .await;

            // Release permit
            drop(permit);

            match result {
                Ok(finished) => {
                    info!(
                        peer = %finished.peer.fmt_short(),
                        namespace = %finished.namespace,
                        sent = finished.outcome.num_sent,
                        recv = finished.outcome.num_recv,
                        connect_ms = ?finished.timings.connect.as_millis(),
                        process_ms = ?finished.timings.process.as_millis(),
                        "docs sync completed"
                    );
                    Ok(())
                }
                Err(err) => {
                    warn!(
                        peer = ?err.peer(),
                        namespace = ?err.namespace(),
                        error = %err,
                        "docs sync failed"
                    );
                    Err(AcceptError::from_err(std::io::Error::other(err.to_string())))
                }
            }
        }
    }

    fn shutdown(&self) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            info!("docs protocol handler shutting down");
            // Close the semaphore to prevent new connections
            self.connection_semaphore.close();
        }
    }
}

/// Initialize iroh-docs resources for export.
///
/// This function handles all the complexity of:
/// 1. Creating or loading the Store (in-memory or persistent)
/// 2. Creating or loading the namespace secret
/// 3. Creating or loading the author secret
/// 4. Opening/creating the replica for the namespace
///
/// # Arguments
///
/// * `data_dir` - Base data directory for persistent storage
/// * `in_memory` - Use in-memory storage instead of persistent
/// * `namespace_secret_hex` - Optional hex-encoded namespace secret from config
/// * `author_secret_hex` - Optional hex-encoded author secret from config
///
/// # Returns
///
/// `DocsResources` containing the initialized Store, NamespaceId, and Author.
pub fn init_docs_resources(
    data_dir: &Path,
    in_memory: bool,
    namespace_secret_hex: Option<&str>,
    author_secret_hex: Option<&str>,
) -> Result<DocsResources> {
    let docs_dir = data_dir.join("docs");

    if in_memory {
        info!("initializing in-memory iroh-docs store");
        init_in_memory_resources(namespace_secret_hex, author_secret_hex)
    } else {
        info!(docs_dir = %docs_dir.display(), "initializing persistent iroh-docs store");
        init_persistent_resources(&docs_dir, namespace_secret_hex, author_secret_hex)
    }
}

/// Initialize in-memory docs resources.
fn init_in_memory_resources(
    namespace_secret_hex: Option<&str>,
    author_secret_hex: Option<&str>,
) -> Result<DocsResources> {
    // Create in-memory store
    let mut store = Store::memory();

    // Create or parse namespace secret
    let namespace_secret = match namespace_secret_hex {
        Some(hex) => parse_namespace_secret(hex)?,
        None => {
            let secret = NamespaceSecret::new(&mut rand::rng());
            debug!("generated new namespace secret for in-memory store");
            secret
        }
    };
    let namespace_id = namespace_secret.id();

    // Create or parse author secret
    let author = match author_secret_hex {
        Some(hex) => parse_author_secret(hex)?,
        None => {
            let author = Author::new(&mut rand::rng());
            debug!("generated new author for in-memory store");
            author
        }
    };

    // Create replica for the namespace
    store
        .new_replica(namespace_secret)
        .context("failed to create replica")?;

    info!(
        namespace_id = %namespace_id,
        author_id = %author.id(),
        "initialized in-memory iroh-docs resources"
    );

    Ok(DocsResources {
        store,
        namespace_id,
        author,
        docs_dir: None,
    })
}

/// Initialize persistent docs resources.
fn init_persistent_resources(
    docs_dir: &Path,
    namespace_secret_hex: Option<&str>,
    author_secret_hex: Option<&str>,
) -> Result<DocsResources> {
    // Ensure docs directory exists
    std::fs::create_dir_all(docs_dir)
        .with_context(|| format!("failed to create docs directory: {}", docs_dir.display()))?;

    // Create persistent store with path to the database file
    let db_path = docs_dir.join(STORE_DB_FILE);
    let mut store = Store::persistent(&db_path)
        .with_context(|| format!("failed to create persistent iroh-docs store at {}", db_path.display()))?;

    // Load or create namespace secret
    let namespace_secret_path = docs_dir.join(NAMESPACE_SECRET_FILE);
    let namespace_secret = load_or_create_namespace_secret(
        &namespace_secret_path,
        namespace_secret_hex,
    )?;
    let namespace_id = namespace_secret.id();

    // Load or create author secret
    let author_path = docs_dir.join(AUTHOR_SECRET_FILE);
    let author = load_or_create_author(&author_path, author_secret_hex)?;

    // Check if replica already exists, create if not
    if store.open_replica(&namespace_id).is_err() {
        store
            .new_replica(namespace_secret)
            .context("failed to create replica")?;
        debug!(namespace_id = %namespace_id, "created new replica");
    } else {
        debug!(namespace_id = %namespace_id, "opened existing replica");
    }

    info!(
        namespace_id = %namespace_id,
        author_id = %author.id(),
        docs_dir = %docs_dir.display(),
        "initialized persistent iroh-docs resources"
    );

    Ok(DocsResources {
        store,
        namespace_id,
        author,
        docs_dir: Some(docs_dir.to_path_buf()),
    })
}

/// Load or create a namespace secret.
///
/// Priority:
/// 1. Config value (hex string) if provided
/// 2. Existing file if present
/// 3. Generate new and persist
fn load_or_create_namespace_secret(
    path: &Path,
    config_hex: Option<&str>,
) -> Result<NamespaceSecret> {
    // Priority 1: Config value
    if let Some(hex) = config_hex {
        let secret = parse_namespace_secret(hex)?;
        info!("using namespace secret from configuration");
        return Ok(secret);
    }

    // Priority 2: Existing file
    if path.exists() {
        let hex = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read namespace secret from {}", path.display()))?;
        let secret = parse_namespace_secret(hex.trim())?;
        info!(path = %path.display(), "loaded namespace secret from file");
        return Ok(secret);
    }

    // Priority 3: Generate new
    let secret = NamespaceSecret::new(&mut rand::rng());
    let hex = hex::encode(secret.to_bytes());
    std::fs::write(path, &hex)
        .with_context(|| format!("failed to write namespace secret to {}", path.display()))?;
    info!(path = %path.display(), "generated and persisted new namespace secret");
    Ok(secret)
}

/// Load or create an author.
///
/// Priority:
/// 1. Config value (hex string) if provided
/// 2. Existing file if present
/// 3. Generate new and persist
fn load_or_create_author(path: &Path, config_hex: Option<&str>) -> Result<Author> {
    // Priority 1: Config value
    if let Some(hex) = config_hex {
        let author = parse_author_secret(hex)?;
        info!("using author from configuration");
        return Ok(author);
    }

    // Priority 2: Existing file
    if path.exists() {
        let hex = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read author secret from {}", path.display()))?;
        let author = parse_author_secret(hex.trim())?;
        info!(path = %path.display(), "loaded author from file");
        return Ok(author);
    }

    // Priority 3: Generate new
    let author = Author::new(&mut rand::rng());
    let hex = hex::encode(author.to_bytes());
    std::fs::write(path, &hex)
        .with_context(|| format!("failed to write author secret to {}", path.display()))?;
    info!(path = %path.display(), "generated and persisted new author");
    Ok(author)
}

/// Parse a namespace secret from hex string.
fn parse_namespace_secret(hex_str: &str) -> Result<NamespaceSecret> {
    let bytes = hex::decode(hex_str).context("invalid namespace secret hex")?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "invalid namespace secret length: expected 32 bytes, got {}",
            bytes.len()
        );
    }
    let array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("invalid length"))?;
    Ok(NamespaceSecret::from_bytes(&array))
}

/// Parse an author from hex-encoded secret.
fn parse_author_secret(hex_str: &str) -> Result<Author> {
    let bytes = hex::decode(hex_str).context("invalid author secret hex")?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "invalid author secret length: expected 32 bytes, got {}",
            bytes.len()
        );
    }
    let array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("invalid length"))?;
    Ok(Author::from_bytes(&array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_in_memory_resources() {
        let resources = init_in_memory_resources(None, None).unwrap();
        assert!(resources.docs_dir.is_none());
    }

    #[test]
    fn test_init_in_memory_with_config_secrets() {
        // Generate valid secrets
        let ns_secret = NamespaceSecret::new(&mut rand::rng());
        let author = Author::new(&mut rand::rng());
        let ns_hex = hex::encode(ns_secret.to_bytes());
        let author_hex = hex::encode(author.to_bytes());

        let resources = init_in_memory_resources(Some(&ns_hex), Some(&author_hex)).unwrap();

        assert_eq!(resources.namespace_id, ns_secret.id());
        assert_eq!(resources.author.id(), author.id());
    }

    #[test]
    fn test_init_persistent_resources() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");

        let resources = init_persistent_resources(&docs_dir, None, None).unwrap();

        assert!(resources.docs_dir.is_some());
        assert!(docs_dir.join(NAMESPACE_SECRET_FILE).exists());
        assert!(docs_dir.join(AUTHOR_SECRET_FILE).exists());
    }

    #[test]
    fn test_persistent_resources_reload() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");

        // First initialization
        let resources1 = init_persistent_resources(&docs_dir, None, None).unwrap();
        let ns_id1 = resources1.namespace_id;
        let author_id1 = resources1.author.id();

        // Drop and reload
        drop(resources1);
        let resources2 = init_persistent_resources(&docs_dir, None, None).unwrap();

        // Should have same IDs
        assert_eq!(resources2.namespace_id, ns_id1);
        assert_eq!(resources2.author.id(), author_id1);
    }

    #[test]
    fn test_parse_namespace_secret_valid() {
        let secret = NamespaceSecret::new(&mut rand::rng());
        let hex = hex::encode(secret.to_bytes());
        let parsed = parse_namespace_secret(&hex).unwrap();
        assert_eq!(parsed.id(), secret.id());
    }

    #[test]
    fn test_parse_namespace_secret_invalid_length() {
        let result = parse_namespace_secret("deadbeef"); // Too short
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_author_secret_valid() {
        let author = Author::new(&mut rand::rng());
        let hex = hex::encode(author.to_bytes());
        let parsed = parse_author_secret(&hex).unwrap();
        assert_eq!(parsed.id(), author.id());
    }
}
