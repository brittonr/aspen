//! NIP-01 Nostr relay engine for Aspen.
//!
//! Embeds a Nostr relay inside an Aspen cluster, exposing cluster state
//! (Forge repos, CI pipelines, issues) to standard Nostr clients over
//! WebSocket. Event storage uses the Raft-backed KV store with secondary
//! indexes for filter queries.
//!
//! Feature-gated behind `nostr-relay`, disabled by default.

pub mod auth;
pub mod config;
pub mod connection;
pub mod constants;
pub mod filters;
pub mod keys;
pub mod relay;
pub mod storage;
pub mod subscriptions;

pub use config::NostrRelayConfig;
pub use config::WritePolicy;
pub use keys::NostrIdentity;
pub use relay::NostrRelayService;
pub use relay::new_dyn_relay;
pub use storage::KvEventStore;
pub use storage::NostrEventStore;
pub use storage::StorageError;
pub use subscriptions::SubscriptionRegistry;
