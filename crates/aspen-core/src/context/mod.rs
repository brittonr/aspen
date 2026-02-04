//! Context traits for protocol handlers.
//!
//! Defines abstract interfaces for accessing node resources without
//! depending on concrete implementations.
//!
//! # Modules
//!
//! - **protocol**: Core protocol handler traits (EndpointProvider, StateMachineProvider,
//!   NetworkFactory)
//! - **peer**: Peer synchronization types (PeerManager, PeerImporter, PeerInfo)
//! - **docs**: Document synchronization (DocsSyncProvider, DocsEntry, AspenDocsTicket)
//! - **watch**: Watch registry for KV subscriptions (WatchRegistry, WatchInfo)
//! - **discovery**: Content discovery via DHT (feature-gated with `global-discovery`)

#[cfg(feature = "global-discovery")]
mod discovery;
mod docs;
mod peer;
mod protocol;
mod watch;

// Re-export all public types at module root for backwards compatibility

// Protocol types
// Discovery types (feature-gated)
#[cfg(feature = "global-discovery")]
pub use discovery::ContentDiscovery;
#[cfg(feature = "global-discovery")]
pub use discovery::ContentNodeAddr;
#[cfg(feature = "global-discovery")]
pub use discovery::ContentProviderInfo;
// Docs types
pub use docs::AspenDocsTicket;
pub use docs::DocsEntry;
pub use docs::DocsStatus;
pub use docs::DocsSyncProvider;
// Peer types
pub use peer::PeerConnectionState;
pub use peer::PeerImporter;
pub use peer::PeerInfo;
pub use peer::PeerManager;
pub use peer::SubscriptionFilter;
pub use peer::SyncStatus;
pub use protocol::EndpointAddr;
pub use protocol::EndpointProvider;
pub use protocol::IrohEndpoint;
pub use protocol::NetworkFactory;
// Shard topology
pub use protocol::ShardTopology;
pub use protocol::StateMachineProvider;
// Watch types
pub use watch::InMemoryWatchRegistry;
pub use watch::KeyOrigin;
pub use watch::WatchInfo;
pub use watch::WatchRegistry;
