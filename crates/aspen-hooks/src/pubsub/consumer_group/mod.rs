//! Consumer groups for pub/sub message processing.
//!
//! Consumer groups enable competing consumers (like SQS/Kafka) where each
//! message is delivered to exactly one consumer in the group. Features include:
//!
//! - **Visibility timeout**: Messages become invisible to other consumers after delivery
//! - **Acknowledgment**: Explicit ack/nack with dead letter queue support
//! - **Fencing tokens**: Prevent zombie consumers from affecting the group
//! - **Partitioned mode**: Optional per-partition ordering with consumer assignment
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  Publisher      │
//! └────────┬────────┘
//!          │ publish
//!          ▼
//! ┌─────────────────┐
//! │  Topic Events   │  (Raft-backed KV store)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────────────────────────┐
//! │      Consumer Group Manager          │
//! │                                       │
//! │  ┌─────────────┐ ┌─────────────┐    │
//! │  │ Consumer A  │ │ Consumer B  │    │
//! │  │ pending: 5  │ │ pending: 3  │    │
//! │  └─────────────┘ └─────────────┘    │
//! │                                       │
//! │  ┌─────────────────────────────┐    │
//! │  │ Pending Entries List (PEL)  │    │
//! │  │ cursor -> consumer, deadline │    │
//! │  └─────────────────────────────┘    │
//! │                                       │
//! │  ┌─────────────────────────────┐    │
//! │  │   Dead Letter Queue (DLQ)    │    │
//! │  └─────────────────────────────┘    │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use aspen_hooks::pubsub::consumer_group::{
//!     ConsumerGroupManager, Consumer, ConsumerGroupConfig,
//!     ConsumerGroupId, ConsumerId, AssignmentMode, JoinOptions,
//! };
//!
//! // Create a group
//! let manager = ConsumerGroupManager::new(kv_store.clone());
//! let config = ConsumerGroupConfig::builder()
//!     .group_id_unchecked("my-group")
//!     .pattern_str("orders.*")
//!     .assignment_mode(AssignmentMode::Competing)
//!     .visibility_timeout_ms(30_000)
//!     .build()?;
//!
//! manager.create_group(config).await?;
//!
//! // Join as a consumer
//! let consumer = Consumer::join(
//!     kv_store.clone(),
//!     pending_manager.clone(),
//!     ConsumerGroupId::new("my-group")?,
//!     ConsumerId::new("consumer-1")?,
//!     JoinOptions::default(),
//! ).await?;
//!
//! // Process messages
//! loop {
//!     // Send heartbeat to stay in group
//!     consumer.heartbeat().await?;
//!
//!     // Fetch messages (implementation depends on message dispatcher)
//!     // ...
//!
//!     // Acknowledge processed message
//!     consumer.ack(&receipt_handle).await?;
//! }
//!
//! // Leave group
//! consumer.leave().await?;
//! ```

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

// Re-export main types
pub use constants::DEFAULT_MAX_DELIVERY_ATTEMPTS;
pub use constants::DEFAULT_VISIBILITY_TIMEOUT_MS;
pub use constants::MAX_BATCH_ACK_SIZE;
pub use constants::MAX_CONSUMER_GROUPS;
pub use constants::MAX_CONSUMERS_PER_GROUP;
pub use constants::MAX_FETCH_BATCH_SIZE;
pub use constants::MAX_PENDING_PER_CONSUMER;
pub use consumer::Consumer;
pub use error::ConsumerGroupError;
pub use error::Result;
pub use fencing::generate_fencing_token;
pub use fencing::validate_fencing;
pub use manager::ConsumerGroupManager;
pub use manager::GroupStats;
pub use pending::DeliveryParams;
pub use pending::PendingEntriesManager;
pub use pending::RaftPendingEntriesList;
pub use pending::RedeliveryParams;
pub use receipt::ReceiptHandleComponents;
pub use receipt::generate_receipt_handle;
pub use receipt::parse_receipt_handle;
pub use types::AckPolicy;
pub use types::AckResult;
pub use types::AssignmentMode;
pub use types::BatchAckRequest;
pub use types::BatchAckResult;
pub use types::CommittedOffset;
pub use types::ConsumerGroupConfig;
pub use types::ConsumerGroupConfigBuilder;
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
