//! Consumer groups for competing consumers in pub/sub.
//!
//! This module provides consumer group functionality that enables multiple consumers
//! to share a subscription, with each message processed by exactly one consumer
//! within the group.
//!
//! # Architecture
//!
//! Consumer groups support two assignment modes:
//!
//! - **Competing** (default): Any consumer can pull any message. Similar to NATS JetStream.
//!   Simpler, no rebalancing needed, but no ordering guarantees.
//!
//! - **Partitioned**: Messages assigned to partitions, partitions assigned to consumers. Similar to
//!   Kafka. Preserves per-partition ordering but requires rebalancing.
//!
//! # Acknowledgment Policies
//!
//! - **Explicit** (default): Consumer must acknowledge each message. Provides at-least-once
//!   delivery semantics. Messages not acknowledged within the visibility timeout are redelivered to
//!   another consumer.
//!
//! - **AutoAck**: Messages acknowledged automatically on receive. Provides at-most-once delivery
//!   semantics. Simpler but may lose messages on consumer failure.
//!
//! # Example
//!
//! ```ignore
//! use aspen_pubsub::consumer_group::{
//!     ConsumerGroupManager, ConsumerGroupConfig, GroupConsumer,
//!     AssignmentMode, AckPolicy,
//! };
//!
//! // Create a consumer group
//! let config = ConsumerGroupConfig::builder()
//!     .group_id("order-processors")
//!     .pattern(TopicPattern::new("orders.*")?)
//!     .assignment_mode(AssignmentMode::Competing)
//!     .ack_policy(AckPolicy::Explicit)
//!     .visibility_timeout_ms(30_000)
//!     .build()?;
//!
//! let manager = ConsumerGroupManager::new(kv_store);
//! let group_state = manager.create(config).await?;
//!
//! // Join as a consumer
//! let consumer = manager.join("order-processors", "consumer-1", Default::default()).await?;
//!
//! // Receive and process messages
//! loop {
//!     let messages = consumer.receive(10, Duration::from_secs(5)).await?;
//!     for msg in messages {
//!         process(&msg).await?;
//!         consumer.ack(&msg.receipt_handle).await?;
//!     }
//! }
//! ```
//!
//! # Key Storage Layout
//!
//! Consumer group state is stored in the KV store using FoundationDB-style tuple encoding:
//!
//! | Key Pattern | Purpose |
//! |-------------|---------|
//! | `__pubsub/groups/{group_id}/state` | Group metadata |
//! | `__pubsub/groups/{group_id}/consumers/{consumer_id}` | Consumer state |
//! | `__pubsub/groups/{group_id}/offsets/{partition_id}` | Committed offset |
//! | `__pubsub/groups/{group_id}/pending/{cursor}` | Pending entry |
//! | `__pubsub/groups/{group_id}/dlq/{cursor}` | Dead letter entry |

pub mod background;
pub mod constants;
pub mod consumer;
pub mod error;
pub mod fencing;
pub mod keys;
pub mod manager;
pub mod pending;
pub mod receipt;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use constants::*;
pub use error::ConsumerGroupError;
pub use error::Result;
pub use keys::ConsumerGroupKeys;
pub use types::AckPolicy;
pub use types::AckResult;
pub use types::AssignmentMode;
pub use types::BatchAckRequest;
pub use types::BatchAckResult;
pub use types::CommittedOffset;
pub use types::ConsumerGroupConfig;
pub use types::ConsumerGroupId;
pub use types::ConsumerId;
pub use types::ConsumerState;
pub use types::DeadLetterEntry;
pub use types::GroupMessage;
pub use types::GroupState;
pub use types::GroupStateType;
pub use types::HeartbeatResponse;
pub use types::JoinOptions;
pub use types::MemberInfo;
pub use types::NackResult;
pub use types::PartitionId;
pub use types::PendingEntry;

// Re-export core traits and implementations
pub use background::BackgroundTasksConfig;
pub use background::BackgroundTasksHandle;
pub use consumer::GroupConsumer;
pub use consumer::RaftGroupConsumer;
pub use fencing::generate_fencing_token;
pub use fencing::next_session_id;
pub use fencing::validate_fencing;
pub use manager::ConsumerGroupManager;
pub use manager::DefaultConsumerGroupManager;
pub use pending::DeliveryParams;
pub use pending::PendingEntriesManager;
pub use pending::RaftPendingEntriesList;
pub use pending::RedeliveryParams;
pub use receipt::generate_receipt_handle;
pub use receipt::parse_receipt_handle;
pub use receipt::ReceiptHandleComponents;
