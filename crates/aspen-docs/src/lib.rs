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
pub mod events;
pub mod exporter;
pub mod importer;
pub mod origin;
pub mod peer_manager;
pub mod store;
pub mod ticket;

pub use constants::BACKGROUND_SYNC_INTERVAL;
pub use constants::DOCS_SYNC_ALPN;
pub use constants::DOCS_SYNC_TIMEOUT;
pub use constants::EXPORT_BATCH_SIZE;
pub use constants::MAX_DOC_KEY_SIZE;
pub use constants::MAX_DOC_VALUE_SIZE;
pub use constants::MAX_DOCS_CONNECTIONS;
pub use events::DOCS_EVENT_BUFFER_SIZE;
// Event types
pub use events::DocsEvent;
pub use events::DocsEventBroadcaster;
pub use events::DocsEventType;
pub use events::ImportResultType;
pub use events::create_docs_event_channel;
pub use exporter::BlobBackedDocsWriter;
pub use exporter::DocsExporter;
pub use exporter::DocsWriter;
pub use exporter::InMemoryDocsWriter;
pub use exporter::IrohDocsWriter;
pub use exporter::SyncHandleDocsWriter;
pub use importer::DocsImporter;
pub use importer::ImportResult;
pub use importer::MAX_PEER_SUBSCRIPTIONS;
pub use importer::PeerStatus;
pub use importer::PeerSubscription;
pub use origin::KeyOrigin;
pub use origin::ORIGIN_KEY_PREFIX;
pub use peer_manager::MAX_PEER_CONNECTIONS;
pub use peer_manager::PeerConnectionState;
pub use peer_manager::PeerInfo;
pub use peer_manager::PeerManager;
pub use peer_manager::SyncStatus;
pub use store::BACKGROUND_SYNC_INTERVAL_SECS;
pub use store::DocsProtocolHandler;
pub use store::DocsResources;
pub use store::DocsSyncResources;
pub use store::DocsSyncService;
pub use store::MAX_OUTBOUND_SYNCS;
pub use store::init_docs_resources;
pub use ticket::AspenDocsTicket;
pub use ticket::TICKET_PREFIX;
