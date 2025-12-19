//! Iroh-docs integration for real-time KV synchronization.
//!
//! This module provides real-time synchronization of KV store data
//! to clients via iroh-docs CRDT-based document replication.
//!
//! ## Architecture
//!
//! ```text
//! Raft Log Subscriber (broadcasts committed entries)
//!         |
//!         v
//! DocsExporter (converts KV ops to docs entries)
//!         |
//!         v
//! iroh-docs Namespace (CRDT replica)
//!         |
//!         v
//! P2P Sync (range-based set reconciliation)
//!         |
//!         v
//! Client (local replica with real-time updates)
//! ```
//!
//! ## Features
//!
//! - **Real-time export**: KV changes are exported to docs as they commit
//! - **CRDT-based**: Uses iroh-docs range-based set reconciliation
//! - **Access control**: Read-only vs read-write via ticket capability
//! - **Priority-based**: Clients can subscribe to multiple clusters with priority
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::docs::{DocsExporter, AspenDocsTicket};
//!
//! // Create docs protocol
//! let docs = Docs::memory()
//!     .spawn(endpoint.clone(), blobs.clone(), gossip.clone())
//!     .await?;
//!
//! // Create namespace and author
//! let namespace_id = docs.create_namespace().await?;
//! let author_id = docs.author_default().await?;
//!
//! // Create exporter
//! let exporter = Arc::new(DocsExporter::new(
//!     Arc::new(docs),
//!     namespace_id,
//!     author_id,
//! ));
//!
//! // Start exporting from log subscriber
//! exporter.clone().spawn(log_receiver);
//! ```

pub mod constants;
pub mod exporter;
pub mod importer;
pub mod origin;
pub mod peer_manager;
pub mod store;
pub mod ticket;

pub use constants::{
    BACKGROUND_SYNC_INTERVAL, DOCS_SYNC_ALPN, DOCS_SYNC_TIMEOUT, EXPORT_BATCH_SIZE,
    MAX_DOC_KEY_SIZE, MAX_DOC_VALUE_SIZE, MAX_DOCS_CONNECTIONS,
};

pub use exporter::{
    BlobBackedDocsWriter, DocsExporter, DocsWriter, InMemoryDocsWriter, IrohDocsWriter,
    SyncHandleDocsWriter,
};

pub use store::{
    BACKGROUND_SYNC_INTERVAL_SECS, DocsProtocolHandler, DocsResources, DocsSyncResources,
    DocsSyncService, MAX_OUTBOUND_SYNCS, init_docs_resources,
};

pub use importer::{
    DocsImporter, ImportResult, MAX_PEER_SUBSCRIPTIONS, PeerStatus, PeerSubscription,
};

pub use origin::{KeyOrigin, ORIGIN_KEY_PREFIX};

pub use peer_manager::{
    MAX_PEER_CONNECTIONS, PeerConnectionState, PeerInfo, PeerManager, SyncStatus,
};

pub use ticket::{AspenDocsTicket, TICKET_PREFIX};
