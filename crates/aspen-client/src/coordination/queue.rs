//! Distributed queue client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for distributed queue operations.
///
/// Provides a high-level async API for queue operations including enqueue,
/// dequeue with visibility timeout, acknowledgment, and dead letter queue.
///
/// # Example
///
/// ```ignore
/// use aspen::client::coordination::QueueClient;
///
/// let queue = QueueClient::new(rpc_client, "my-queue");
///
/// // Enqueue an item
/// let item_id = queue.enqueue(b"hello".to_vec()).await?;
///
/// // Dequeue with visibility timeout
/// let items = queue.dequeue("consumer-1", 10, Duration::from_secs(30)).await?;
///
/// // Process and acknowledge
/// for item in items {
///     // Process item...
///     queue.ack(&item.receipt_handle).await?;
/// }
/// ```
pub struct QueueClient<C: CoordinationRpc> {
    client: Arc<C>,
    queue_name: String,
}

impl<C: CoordinationRpc> QueueClient<C> {
    /// Create a new queue client.
    pub fn new(client: Arc<C>, queue_name: impl Into<String>) -> Self {
        Self {
            client,
            queue_name: queue_name.into(),
        }
    }

    /// Create or configure a queue.
    ///
    /// Returns (created, success) where created is true if the queue was newly created.
    pub async fn create(&self, config: QueueCreateConfig) -> Result<bool> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueCreate {
                queue_name: self.queue_name.clone(),
                default_visibility_timeout_ms: config.default_visibility_timeout.map(|d| d.as_millis() as u64),
                default_ttl_ms: config.default_ttl.map(|d| d.as_millis() as u64),
                max_delivery_attempts: config.max_delivery_attempts,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueCreateResult(result) => {
                if result.is_success {
                    Ok(result.was_created)
                } else {
                    bail!("queue create failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueCreate"),
        }
    }

    /// Delete the queue and all its items.
    ///
    /// Returns the number of items deleted.
    pub async fn delete(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDelete {
                queue_name: self.queue_name.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDeleteResult(result) => {
                if result.is_success {
                    Ok(result.items_deleted.unwrap_or(0))
                } else {
                    bail!("queue delete failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDelete"),
        }
    }

    /// Enqueue an item.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue(&self, payload: Vec<u8>) -> Result<u64> {
        self.enqueue_with_options(payload, QueueEnqueueOptions::default()).await
    }

    /// Enqueue an item with options.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue_with_options(&self, payload: Vec<u8>, options: QueueEnqueueOptions) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueEnqueue {
                queue_name: self.queue_name.clone(),
                payload,
                ttl_ms: options.ttl.map(|d| d.as_millis() as u64),
                message_group_id: options.message_group_id,
                deduplication_id: options.deduplication_id,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueEnqueueResult(result) => {
                if result.is_success {
                    Ok(result.item_id.unwrap_or(0))
                } else {
                    bail!("enqueue failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueEnqueue"),
        }
    }

    /// Enqueue multiple items in a batch.
    ///
    /// Returns the list of item IDs on success.
    pub async fn enqueue_batch(&self, items: Vec<QueueEnqueueBatchItem>) -> Result<Vec<u64>> {
        use aspen_client_api::QueueEnqueueItem;

        let rpc_items: Vec<QueueEnqueueItem> = items
            .into_iter()
            .map(|item| QueueEnqueueItem {
                payload: item.payload,
                ttl_ms: item.ttl.map(|d| d.as_millis() as u64),
                message_group_id: item.message_group_id,
                deduplication_id: item.deduplication_id,
            })
            .collect();

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueEnqueueBatch {
                queue_name: self.queue_name.clone(),
                items: rpc_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueEnqueueBatchResult(result) => {
                if result.is_success {
                    Ok(result.item_ids)
                } else {
                    bail!("enqueue batch failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueEnqueueBatch"),
        }
    }

    /// Dequeue items with visibility timeout (non-blocking).
    ///
    /// Returns up to `max_items` items. Each item is locked for `visibility_timeout`.
    /// Returns empty vec if queue is empty.
    pub async fn dequeue(
        &self,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout: Duration,
    ) -> Result<Vec<QueueDequeuedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDequeue {
                queue_name: self.queue_name.clone(),
                consumer_id: consumer_id.to_string(),
                max_items,
                visibility_timeout_ms: visibility_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDequeueResult(result) => {
                if result.is_success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDequeuedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            visibility_deadline: Duration::from_millis(item.visibility_deadline_ms),
                        })
                        .collect())
                } else {
                    bail!("dequeue failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDequeue"),
        }
    }

    /// Dequeue items with blocking wait.
    ///
    /// Polls until items are available or `wait_timeout` expires.
    pub async fn dequeue_wait(
        &self,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout: Duration,
        wait_timeout: Duration,
    ) -> Result<Vec<QueueDequeuedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDequeueWait {
                queue_name: self.queue_name.clone(),
                consumer_id: consumer_id.to_string(),
                max_items,
                visibility_timeout_ms: visibility_timeout.as_millis() as u64,
                wait_timeout_ms: wait_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDequeueResult(result) => {
                if result.is_success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDequeuedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            visibility_deadline: Duration::from_millis(item.visibility_deadline_ms),
                        })
                        .collect())
                } else {
                    bail!("dequeue_wait failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDequeueWait"),
        }
    }

    /// Peek at items without removing them.
    ///
    /// Returns up to `max_items` items from the front of the queue.
    pub async fn peek(&self, max_items: u32) -> Result<Vec<QueuePeekedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueuePeek {
                queue_name: self.queue_name.clone(),
                max_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueuePeekResult(result) => {
                if result.is_success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueuePeekedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            expires_at: if item.expires_at_ms > 0 {
                                Some(Duration::from_millis(item.expires_at_ms))
                            } else {
                                None
                            },
                            delivery_attempts: item.delivery_attempts,
                        })
                        .collect())
                } else {
                    bail!("peek failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueuePeek"),
        }
    }

    /// Acknowledge successful processing of an item.
    pub async fn ack(&self, receipt_handle: &str) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueAck {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueAckResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    bail!("ack failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueAck"),
        }
    }

    /// Negative acknowledge - return item to queue.
    pub async fn nack(&self, receipt_handle: &str) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueNack {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                move_to_dlq: false,
                error_message: None,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueNackResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    bail!("nack failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueNack"),
        }
    }

    /// Negative acknowledge and move to dead letter queue.
    pub async fn nack_to_dlq(&self, receipt_handle: &str, error_message: Option<String>) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueNack {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                move_to_dlq: true,
                error_message,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueNackResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    bail!("nack_to_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueNack"),
        }
    }

    /// Extend the visibility timeout for a pending item.
    ///
    /// Returns the new visibility deadline (Unix epoch millis).
    pub async fn extend_visibility(&self, receipt_handle: &str, additional_timeout: Duration) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueExtendVisibility {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                additional_timeout_ms: additional_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueExtendVisibilityResult(result) => {
                if result.is_success {
                    Ok(result.new_deadline_ms.unwrap_or(0))
                } else {
                    bail!("extend_visibility failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueExtendVisibility"),
        }
    }

    /// Get queue status.
    pub async fn status(&self) -> Result<QueueStatusInfo> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueStatus {
                queue_name: self.queue_name.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueStatusResult(result) => {
                if result.is_success {
                    Ok(QueueStatusInfo {
                        does_exist: result.does_exist,
                        visible_count: result.visible_count.unwrap_or(0),
                        pending_count: result.pending_count.unwrap_or(0),
                        dlq_count: result.dlq_count.unwrap_or(0),
                        total_enqueued: result.total_enqueued.unwrap_or(0),
                        total_acked: result.total_acked.unwrap_or(0),
                    })
                } else {
                    bail!("status failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueStatus"),
        }
    }

    /// Get items from the dead letter queue.
    pub async fn get_dlq(&self, max_items: u32) -> Result<Vec<QueueDLQItemInfo>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueGetDLQ {
                queue_name: self.queue_name.clone(),
                max_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueGetDLQResult(result) => {
                if result.is_success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDLQItemInfo {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            delivery_attempts: item.delivery_attempts,
                            reason: item.reason,
                            moved_at: Duration::from_millis(item.moved_at_ms),
                            last_error: item.last_error,
                        })
                        .collect())
                } else {
                    bail!("get_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueGetDLQ"),
        }
    }

    /// Move a DLQ item back to the main queue.
    pub async fn redrive_dlq(&self, item_id: u64) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueRedriveDLQ {
                queue_name: self.queue_name.clone(),
                item_id,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueRedriveDLQResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    bail!("redrive_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueRedriveDLQ"),
        }
    }
}

/// Configuration for creating a queue.
#[derive(Debug, Clone, Default)]
pub struct QueueCreateConfig {
    /// Default visibility timeout.
    pub default_visibility_timeout: Option<Duration>,
    /// Default item TTL.
    pub default_ttl: Option<Duration>,
    /// Maximum delivery attempts before moving to DLQ.
    pub max_delivery_attempts: Option<u32>,
}

/// Options for enqueuing an item.
#[derive(Debug, Clone, Default)]
pub struct QueueEnqueueOptions {
    /// Item TTL (overrides queue default).
    pub ttl: Option<Duration>,
    /// Message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Item to enqueue in a batch operation.
#[derive(Debug, Clone)]
pub struct QueueEnqueueBatchItem {
    /// Item payload.
    pub payload: Vec<u8>,
    /// Item TTL.
    pub ttl: Option<Duration>,
    /// Message group ID.
    pub message_group_id: Option<String>,
    /// Deduplication ID.
    pub deduplication_id: Option<String>,
}

/// A dequeued item with receipt handle for acknowledgment.
#[derive(Debug, Clone)]
pub struct QueueDequeuedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Visibility deadline (as Duration from Unix epoch).
    pub visibility_deadline: Duration,
}

/// A peeked item (not dequeued).
#[derive(Debug, Clone)]
pub struct QueuePeekedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Expiration time (as Duration from Unix epoch), None if no expiration.
    pub expires_at: Option<Duration>,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
}

/// Queue status information.
#[derive(Debug, Clone)]
pub struct QueueStatusInfo {
    /// Whether the queue exists.
    pub does_exist: bool,
    /// Approximate number of visible items.
    pub visible_count: u64,
    /// Approximate number of pending items.
    pub pending_count: u64,
    /// Approximate number of DLQ items.
    pub dlq_count: u64,
    /// Total items enqueued.
    pub total_enqueued: u64,
    /// Total items acked.
    pub total_acked: u64,
}

/// A dead letter queue item.
#[derive(Debug, Clone)]
pub struct QueueDLQItemInfo {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: String,
    /// Time moved to DLQ (as Duration from Unix epoch).
    pub moved_at: Duration,
    /// Last error message.
    pub last_error: Option<String>,
}
