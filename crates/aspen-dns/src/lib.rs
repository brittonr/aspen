//! DNS record management layer for Aspen.
//!
//! This module provides DNS-specific functionality built on top of Aspen's
//! existing iroh-docs fan-out infrastructure. It enables:
//!
//! - **Server-side**: DNS record CRUD via Raft consensus
//! - **Client-side**: Real-time DNS record sync with local cache
//! - **DNS Protocol**: Embedded DNS server for local domain resolution
//!
//! # Architecture
//!
//! ```text
//! Server Side (Aspen Cluster)                    Client Side
//! ┌─────────────────────────────────────┐        ┌─────────────────────────────┐
//! │  DnsStore (API layer)               │        │  AspenDnsClient             │
//! │    ↓                                │        │    ↓                        │
//! │  KeyValueStore (set dns:* keys)     │        │  Local Cache                │
//! │    ↓                                │        │    ↑                        │
//! │  Raft Consensus                     │        │  iroh-docs sync             │
//! │    ↓                                │        │    ↑                        │
//! │  DocsExporter (existing)            │        │  P2P CRDT replication       │
//! │    ↓                                │        │    ↓                        │
//! │  iroh-docs namespace ─────── P2P CRDT sync ──┤  DnsProtocolServer          │
//! └─────────────────────────────────────┘        │  (UDP/TCP :5353)            │
//!                                                └─────────────────────────────┘
//! ```
//!
//! # Key Format
//!
//! DNS records are stored in the KV store with the following key convention:
//! - Records: `dns:{domain}:{record_type}` (e.g., `dns:api.example.com:A`)
//! - Zones: `dns:_zone:{zone_name}` (e.g., `dns:_zone:example.com`)
//!
//! # Record Types
//!
//! Supported DNS record types:
//! - `A` - IPv4 addresses
//! - `AAAA` - IPv6 addresses
//! - `CNAME` - Canonical name alias
//! - `MX` - Mail exchange servers
//! - `TXT` - Text records (SPF, DKIM, etc.)
//! - `SRV` - Service location
//! - `NS` - Nameservers
//! - `SOA` - Start of authority
//! - `PTR` - Pointer (reverse DNS)
//! - `CAA` - Certificate authority authorization
//!
//! # Server-Side Usage
//!
//! ```ignore
//! use aspen_dns::{DnsStore, AspenDnsStore, DnsRecord, DnsRecordData, RecordType};
//! use std::net::Ipv4Addr;
//!
//! // Create DNS store wrapping your KeyValueStore
//! let dns_store = AspenDnsStore::new(kv_store);
//!
//! // Create an A record
//! let record = DnsRecord {
//!     domain: "api.example.com".to_string(),
//!     record_type: RecordType::A,
//!     ttl_seconds: 300,
//!     data: DnsRecordData::A {
//!         addresses: vec![Ipv4Addr::new(192, 168, 1, 1)],
//!     },
//!     updated_at_ms: 0,
//! };
//!
//! // Store via Raft consensus
//! dns_store.set_record(record).await?;
//!
//! // Resolution with wildcard support
//! let records = dns_store.resolve("api.example.com", RecordType::A).await?;
//! ```
//!
//! # Client-Side Usage
//!
//! ```ignore
//! use aspen_dns::{AspenDnsClient, DnsClientTicket, RecordType};
//!
//! // Connect using a ticket
//! let ticket = DnsClientTicket::deserialize("aspendns...")?;
//! let client = AspenDnsClient::builder()
//!     .ticket(ticket)
//!     .build();
//!
//! // Fast local lookups (after sync)
//! if let Some(addrs) = client.lookup_a("api.example.com").await {
//!     println!("A records: {:?}", addrs);
//! }
//!
//! // Subscribe to record changes
//! let mut events = client.subscribe()?;
//! while let Ok(event) = events.recv().await {
//!     println!("DNS event: {:?}", event);
//! }
//! ```
//!
//! # Wildcard Records
//!
//! Wildcard records (`*.example.com`) are supported. Resolution order:
//! 1. Exact match (`api.example.com`)
//! 2. Wildcard match (`*.example.com`)
//!
//! # Zone Management
//!
//! Zones provide organizational grouping and metadata:
//!
//! ```ignore
//! use aspen_dns::{Zone, ZoneMetadata};
//!
//! // Create a zone
//! let zone = Zone {
//!     name: "example.com".to_string(),
//!     metadata: ZoneMetadata {
//!         created_at_ms: now(),
//!         enabled: true,
//!         description: Some("Production zone".to_string()),
//!     },
//! };
//!
//! dns_store.set_zone(zone).await?;
//! ```

pub mod authority;
pub mod client;
pub mod config;
pub mod constants;
pub mod error;
pub mod forwarding;
pub mod server;
pub mod store;
pub mod ticket;
pub mod types;
pub mod validation;

// Re-export main types for convenience
// DNS protocol server types
pub use authority::AspenDnsAuthority;
pub use client::AspenDnsClient;
pub use client::CacheStats;
pub use client::DnsClientBuilder;
pub use config::DnsServerConfig;
pub use constants::DEFAULT_TTL;
pub use constants::DNS_KEY_PREFIX;
pub use constants::DNS_TICKET_PREFIX;
pub use constants::DNS_ZONE_PREFIX;
pub use constants::MAX_BATCH_SIZE;
pub use constants::MAX_CACHED_RECORDS;
pub use constants::MAX_DOMAIN_LENGTH;
pub use constants::MAX_LABEL_LENGTH;
pub use constants::MAX_RECORDS_PER_DOMAIN;
pub use constants::MAX_SUBSCRIBERS;
pub use constants::MAX_TXT_LENGTH;
pub use constants::MAX_ZONES;
pub use constants::MIN_TTL;
pub use error::DnsClientError;
pub use error::DnsClientResult;
pub use error::DnsError;
pub use error::DnsResult;
pub use forwarding::ForwardingAuthority;
pub use server::DnsProtocolServer;
pub use server::DnsProtocolServerBuilder;
pub use store::AspenDnsStore;
pub use store::DnsStore;
pub use ticket::DnsClientTicket;
pub use types::CaaRecord;
pub use types::DnsEvent;
pub use types::DnsRecord;
pub use types::DnsRecordData;
pub use types::MxRecord;
pub use types::RecordType;
pub use types::SoaRecord;
pub use types::SrvRecord;
pub use types::SyncStatus;
pub use types::Zone;
pub use types::ZoneMetadata;
pub use validation::is_wildcard_domain;
pub use validation::validate_domain;
pub use validation::validate_record;
pub use validation::validate_ttl;
pub use validation::wildcard_parent;
