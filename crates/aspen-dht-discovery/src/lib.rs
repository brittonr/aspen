//! Global content discovery for Aspen using the BitTorrent Mainline DHT.
//!
//! This crate provides decentralized content discovery beyond cluster boundaries,
//! allowing nodes to find blob providers across the global network without relying
//! on centralized trackers.
//!
//! # Feature Flags
//!
//! - **`global-discovery`**: Enables actual DHT operations using the `mainline` crate. Without this
//!   feature, the service logs operations but doesn't perform real DHT queries. Enable with: `cargo
//!   build --features global-discovery`
//!
//! # Architecture
//!
//! The content discovery system uses a two-layer approach:
//!
//! 1. **Intra-cluster**: Gossip-based `BlobAnnouncement` for fast local discovery
//! 2. **Global**: Mainline DHT for cross-cluster and public content discovery
//!
//! # Design Decisions
//!
//! - Uses the `mainline` crate directly (v6.0) for DHT access when feature is enabled
//! - Maps BLAKE3 hashes to DHT infohashes via SHA-256 truncation (20 bytes)
//! - Uses BEP-44 mutable items to store full iroh NodeAddr (32-byte public key + relay URL)
//! - Each provider announces with their ed25519 key, using infohash as salt
//! - Discoverers query for mutable items to retrieve full NodeAddr for connection
//! - Rate-limited publishing to avoid DHT spam (1 announce per hash per 5 min)
//! - Background republishing every 30 minutes for active content
//!
//! # Security
//!
//! - All DHT announces are Ed25519 signed (BEP-44 mutable items)
//! - Provider identity is cryptographically verified
//! - DHT operations use standard BitTorrent mutable item protocol
//! - Provider verification can be done by attempting blob download via Iroh
//!
//! # References
//!
//! - [Iroh Content Discovery Blog](https://www.iroh.computer/blog/iroh-content-discovery)
//! - [mainline crate](https://crates.io/crates/mainline)
//! - [BEP-0005: DHT Protocol](http://bittorrent.org/beps/bep_0005.html)
//! - [BEP-0044: Storing arbitrary data in the DHT](http://bittorrent.org/beps/bep_0044.html)
//!
//! # Example
//!
//! ```ignore
//! use aspen_dht_discovery::{ContentDiscoveryService, ContentDiscoveryConfig};
//!
//! let config = ContentDiscoveryConfig::default();
//! let service = ContentDiscoveryService::spawn(
//!     endpoint.clone(),
//!     secret_key.clone(),
//!     config,
//!     cancellation_token.clone(),
//! ).await?;
//!
//! // Announce a blob to the global DHT
//! service.announce(blob_hash, blob_size, blob_format).await?;
//!
//! // Query for providers of a specific blob
//! let providers = service.find_providers(blob_hash, blob_format).await?;
//! ```

#![allow(dead_code, unused_imports)]

mod client;
pub mod config;
pub mod hash;
pub mod service;
pub mod types;

// Re-export main public types at crate root
pub use config::ContentDiscoveryConfig;
pub use hash::to_dht_infohash;
pub use service::ContentDiscoveryService;
pub use types::DhtNodeAddr;
pub use types::ProviderInfo;
