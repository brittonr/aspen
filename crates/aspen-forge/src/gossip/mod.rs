//! Gossip integration for Forge.
//!
//! This module provides real-time announcement broadcasting and reception
//! for repository events over iroh-gossip.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ForgeGossipService                        │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │ Announcer   │  │ Receiver    │  │ Rate Limiter        │  │
//! │  │ Task        │  │ Tasks       │  │ (per-peer + global) │  │
//! │  └──────┬──────┘  └──────┬──────┘  └─────────────────────┘  │
//! │         │                │                                    │
//! │         ▼                ▼                                    │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │              iroh-gossip Topics                          │ │
//! │  │  • Global: forge:global (RepoCreated, Seeding)          │ │
//! │  │  • Per-repo: forge:repo:{id} (RefUpdate, CobChange)     │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//!          ▲                              │
//!          │ Events                       │ Announcements
//!          │                              ▼
//! ┌────────┴────────┐          ┌─────────────────────┐
//! │    RefStore     │          │ AnnouncementHandler │
//! │    CobStore     │          │ → SyncService       │
//! └─────────────────┘          │ → add_seeding_peer  │
//!                              └─────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::forge::gossip::{ForgeGossipService, ForgeAnnouncementHandler};
//!
//! // Create handler for incoming announcements
//! let handler = Arc::new(ForgeAnnouncementHandler::new(forge_node.clone()));
//!
//! // Spawn gossip service
//! let service = ForgeGossipService::spawn(
//!     gossip,
//!     secret_key,
//!     ref_store.subscribe(),
//!     cob_store.subscribe(),
//!     Some(handler),
//! ).await?;
//!
//! // Subscribe to repos we're seeding
//! service.subscribe_repo(&repo_id).await?;
//!
//! // Announce we're seeding
//! service.announce_seeding(&repo_id).await?;
//! ```

mod handler;
mod rate_limiter;
mod service;
mod types;

pub use handler::ForgeAnnouncementHandler;
pub use rate_limiter::ForgeGossipRateLimiter;
pub use rate_limiter::RateLimitReason;
pub use service::AnnouncementCallback;
pub use service::ForgeGossipService;
pub use types::Announcement;
pub use types::AnnouncementHandler;
pub use types::ForgeTopic;
pub use types::SignedAnnouncement;
