//! Pub/Sub layer for Aspen distributed systems.
//!
//! This module provides a pub/sub messaging layer built on top of Aspen's Raft-based
//! KV store. It offers:
//!
//! - **Topic-based messaging**: Hierarchical topics with NATS-style wildcards
//! - **Strong ordering**: Events are totally ordered by Raft log index
//! - **Resumable subscriptions**: Resume from any cursor position
//! - **Consumer groups**: Competing consumers with visibility timeout
//!
//! # Architecture
//!
//! Events are stored in the Raft-replicated KV store under a reserved prefix.
//! This provides:
//!
//! - **Durability**: Events survive node failures
//! - **Ordering**: Single source of truth via Raft log
//! - **Consistency**: Linearizable reads and writes
//! - **Simplicity**: Reuses existing infrastructure
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │  Publisher  │    │  Publisher  │    │  Publisher  │
//! └─────┬───────┘    └─────┬───────┘    └─────┬───────┘
//!       │                  │                  │
//!       │ publish          │ publish          │ publish
//!       ▼                  ▼                  ▼
//! ┌─────────────────────────────────────────────────┐
//! │                 Raft Consensus                   │
//! │           (ordering & durability)                │
//! └─────────────────────────────────────────────────┘
//!                        │
//!                        │ replicate
//!                        ▼
//! ┌─────────────────────────────────────────────────┐
//! │              KV Store (redb)                     │
//! │   __pubsub/events/topic/segment/cursor          │
//! └─────────────────────────────────────────────────┘
//!                        │
//!          ┌─────────────┼─────────────┐
//!          │             │             │
//!          ▼             ▼             ▼
//!    ┌───────────┐ ┌───────────┐ ┌───────────┐
//!    │Subscriber │ │Subscriber │ │ Consumer  │
//!    │ (direct)  │ │ (pattern) │ │  Group    │
//!    └───────────┘ └───────────┘ └───────────┘
//! ```
//!
//! # Topics
//!
//! Topics are hierarchical names separated by dots:
//! - `orders` - single segment
//! - `orders.created` - two segments
//! - `orders.us.east.created` - four segments
//!
//! Subscriptions can use NATS-style wildcards:
//! - `*` matches exactly one segment: `orders.*` matches `orders.created`
//! - `>` matches zero or more segments: `orders.>` matches `orders` and `orders.us.created`
//!
//! # Cursors
//!
//! Each event has a cursor (Raft log index) that provides:
//! - Global ordering across all topics
//! - Resume capability from any point
//! - Correlation with KV operations
//!
//! # Quick Start
//!
//! ```ignore
//! use aspen_hooks::pubsub::{RaftPublisher, Publisher, Topic, Cursor};
//! use std::sync::Arc;
//!
//! // Create a publisher
//! let publisher = RaftPublisher::new(kv_store.clone());
//!
//! // Publish an event
//! let topic = Topic::new("orders.created")?;
//! let cursor = publisher.publish(&topic, b"order data").await?;
//! println!("Published at cursor: {}", cursor);
//! ```
//!
//! # Consumer Groups
//!
//! For competing consumers (like SQS/Kafka), use consumer groups:
//!
//! ```ignore
//! use aspen_hooks::pubsub::consumer_group::{
//!     ConsumerGroupManager, Consumer, ConsumerGroupConfig,
//! };
//!
//! // Create a group
//! let manager = ConsumerGroupManager::new(kv_store.clone());
//! let config = ConsumerGroupConfig::builder()
//!     .group_id_unchecked("my-group")
//!     .pattern_str("orders.*")
//!     .build()?;
//!
//! manager.create_group(config).await?;
//! ```
//!
//! # Tiger Style
//!
//! All operations are bounded by explicit limits in `constants`:
//! - `MAX_TOPIC_SEGMENTS`: 16
//! - `MAX_PAYLOAD_SIZE`: 1 MB
//! - `MAX_PUBLISH_BATCH_SIZE`: 100

pub mod constants;
pub mod consumer_group;
pub mod cursor;
pub mod encoding;
pub mod error;
pub mod event;
pub mod keys;
pub mod publisher;
pub mod subscriber;
pub mod topic;

// Re-export main types for convenience
pub use constants::MAX_HEADER_KEY_SIZE;
pub use constants::MAX_HEADER_VALUE_SIZE;
pub use constants::MAX_HEADERS;
pub use constants::MAX_PAYLOAD_SIZE;
pub use constants::MAX_PUBLISH_BATCH_SIZE;
pub use constants::MAX_SEGMENT_LENGTH;
pub use constants::MAX_TOPIC_SEGMENTS;
pub use constants::PUBSUB_GROUPS_PREFIX;
pub use constants::PUBSUB_KEY_PREFIX;
pub use constants::TOPIC_SEGMENT_SEPARATOR;
pub use constants::WILDCARD_MULTI;
pub use constants::WILDCARD_SINGLE;
pub use cursor::Cursor;
pub use error::PubSubError;
pub use error::Result;
pub use event::Event;
pub use event::EventBuilder;
pub use keys::build_cursor_range;
pub use keys::build_event_key;
pub use keys::build_topic_prefix;
pub use keys::parse_event_key;
pub use publisher::Publisher;
pub use publisher::RaftPublisher;
pub use subscriber::Subscription;
pub use subscriber::SubscriptionOptions;
pub use topic::PatternSegment;
pub use topic::Topic;
pub use topic::TopicPattern;
