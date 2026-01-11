//! Pub/Sub layer for Aspen distributed systems.
//!
//! This crate provides a publish-subscribe messaging layer built on top of
//! Aspen's Raft consensus and log streaming infrastructure. It provides:
//!
//! - **Linearizable ordering**: All events across all topics have a global total order
//! - **Exactly-once delivery**: Events go through Raft consensus
//! - **Wildcard subscriptions**: NATS-style `*` and `>` patterns
//! - **Historical replay**: Resume from any point using cursors
//!
//! # Architecture
//!
//! ```text
//! Publisher                          Subscriber
//!     |                                  |
//!     v                                  v
//! publish(topic, payload)      subscribe(pattern, cursor)
//!     |                                  |
//!     v                                  v
//! +---------------+              +----------------+
//! | KeyValueStore | <-- Raft --> | LogSubscriber  |
//! +---------------+              +----------------+
//!     |                                  |
//!     v                                  v
//!   Raft Log (single source of truth)
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use aspen_pubsub::{Topic, TopicPattern, Cursor, RaftPublisher, Publisher};
//! use std::sync::Arc;
//!
//! // Publishing
//! let publisher = RaftPublisher::new(kv_store);
//! let topic = Topic::new("orders.created")?;
//! let cursor = publisher.publish(&topic, b"order data").await?;
//! println!("Published at cursor {}", cursor);
//!
//! // Subscribing (with event stream)
//! let pattern = TopicPattern::new("orders.*")?;
//! // ... set up log subscriber stream ...
//! let mut stream = EventStream::new(log_stream, pattern);
//!
//! while let Some(event) = stream.next().await {
//!     let event = event?;
//!     println!("Topic: {}, Cursor: {}", event.topic, event.cursor);
//! }
//! ```
//!
//! # Wildcards
//!
//! Following NATS conventions:
//! - `*` matches exactly one segment: `"orders.*"` matches `"orders.created"`, not
//!   `"orders.us.created"`
//! - `>` matches zero or more: `"orders.>"` matches `"orders"`, `"orders.created"`,
//!   `"orders.us.created"`
//!
//! # Cursor Semantics
//!
//! Cursors are Raft log indices, providing:
//! - Resume from any historical point
//! - Global ordering across all topics
//! - Correlation with KV operations

#![warn(missing_docs)]

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

// Re-export commonly used types at crate root

// Constants
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
// Cursor
pub use cursor::Cursor;
// Error
pub use error::{PubSubError, Result};
// Event
pub use event::{Event, EventBuilder, validate_headers, validate_payload};
// Keys
pub use keys::{is_pubsub_key, key_to_string, key_to_topic, pattern_to_prefix, string_to_key, topic_to_key};
// Publisher
pub use publisher::{Publisher, RaftPublisher, publish};
// Subscriber
pub use subscriber::{EventStream, SubscriptionBuilder, SubscriptionConfig, filter_event};
// Topic
pub use topic::{PatternSegment, Topic, TopicPattern};
