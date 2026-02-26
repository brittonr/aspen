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

mod blob_download;
mod event_handling;
mod initialization;
mod protocol_handler;
mod resources;
mod sync_operations;
mod sync_service;

use std::path::PathBuf;
use std::sync::Arc;

pub use initialization::init_docs_resources;
use iroh_docs::Author;
use iroh_docs::NamespaceId;
use iroh_docs::actor::SyncHandle;
use iroh_docs::store::Store;
pub use protocol_handler::DocsProtocolHandler;
pub use sync_service::DocsSyncService;

use super::events::DocsEventBroadcaster;

/// File name for the iroh-docs redb database.
const STORE_DB_FILE: &str = "docs.redb";

/// File name for persisted namespace secret.
const NAMESPACE_SECRET_FILE: &str = "namespace_secret";

/// File name for persisted author secret.
const AUTHOR_SECRET_FILE: &str = "author_secret";

/// Interval between background sync attempts.
pub const BACKGROUND_SYNC_INTERVAL_SECS: u64 = 60;

/// Maximum concurrent outbound sync connections.
pub const MAX_OUTBOUND_SYNCS: usize = 5;

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
    /// Optional event broadcaster for hook integration.
    pub(crate) event_broadcaster: Option<Arc<DocsEventBroadcaster>>,
}

/// Check if content is a tombstone marker (single null byte).
#[inline]
fn is_tombstone_marker(content: &[u8]) -> bool {
    content.len() == 1 && content[0] == 0x00
}
