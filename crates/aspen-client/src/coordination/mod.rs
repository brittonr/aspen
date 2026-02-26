//! Typed client wrappers for coordination primitives.
//!
//! Provides ergonomic APIs for distributed locks, counters, sequences,
//! and rate limiters over the client RPC protocol.
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::client::coordination::{LockClient, CounterClient, SequenceClient, RateLimiterClient};
//!
//! // Get a distributed lock
//! let lock = LockClient::new(rpc_client.clone(), "my-resource");
//! let guard = lock.acquire("holder-1", Duration::from_secs(30)).await?;
//! // ... protected work ...
//! guard.release().await?;
//!
//! // Use an atomic counter
//! let counter = CounterClient::new(rpc_client.clone(), "request-count");
//! let value = counter.increment().await?;
//!
//! // Generate unique IDs
//! let seq = SequenceClient::new(rpc_client.clone(), "order-ids");
//! let order_id = seq.next().await?;
//!
//! // Rate limit operations
//! let limiter = RateLimiterClient::new(rpc_client.clone(), "api-calls", 100.0, 50);
//! if limiter.try_acquire().await? {
//!     // proceed with rate-limited operation
//! }
//! ```

mod barrier;
mod batch;
mod counter;
mod lease;
mod lock;
mod queue;
mod rate_limiter;
mod rwlock;
mod semaphore;
mod sequence;
mod service;
mod watch;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

/// Trait for RPC clients that can send coordination requests.
///
/// Implemented by both single-node and multi-node clients.
pub trait CoordinationRpc: Send + Sync {
    /// Send a coordination RPC request.
    fn send_coordination_request(
        &self,
        request: ClientRpcRequest,
    ) -> impl std::future::Future<Output = Result<ClientRpcResponse>> + Send;
}

// Re-export all public types
pub use barrier::BarrierClient;
pub use barrier::BarrierEnterResult;
pub use barrier::BarrierLeaveResult;
pub use barrier::BarrierStatusResult;
pub use batch::BatchClient;
pub use batch::BatchConditionOp;
pub use batch::BatchWriteOp;
pub use batch::ConditionalBatchResult;
pub use counter::CounterClient;
pub use counter::SignedCounterClient;
pub use lease::LeaseClient;
pub use lease::LeaseGrantResult;
pub use lease::LeaseInfoLocal;
pub use lease::LeaseKeepaliveHandle;
pub use lease::LeaseKeepaliveResult;
pub use lease::LeaseRevokeResult;
pub use lease::LeaseTimeToLiveResult;
pub use lock::LockClient;
pub use lock::RemoteLockGuard;
pub use queue::QueueClient;
pub use queue::QueueCreateConfig;
pub use queue::QueueDLQItemInfo;
pub use queue::QueueDequeuedItem;
pub use queue::QueueEnqueueBatchItem;
pub use queue::QueueEnqueueOptions;
pub use queue::QueuePeekedItem;
pub use queue::QueueStatusInfo;
pub use rate_limiter::RateLimitResult;
pub use rate_limiter::RateLimiterClient;
pub use rwlock::RWLockClient;
pub use rwlock::RWLockReadResult;
pub use rwlock::RWLockStatusResult;
pub use rwlock::RWLockWriteResult;
pub use semaphore::SemaphoreAcquireResult;
pub use semaphore::SemaphoreClient;
pub use semaphore::SemaphoreStatusResult;
pub use sequence::SequenceClient;
pub use service::ServiceClient;
pub use service::ServiceDiscoveryFilter;
pub use service::ServiceHeartbeatHandle;
pub use service::ServiceInstanceInfo;
pub use service::ServiceMetadataUpdate;
pub use service::ServiceRegisterOptions;
pub use service::ServiceRegistration;
pub use watch::WatchClient;
pub use watch::WatchHandle;
pub use watch::WatchInfoLocal;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    use anyhow::bail;
    use aspen_client_api::CounterResultResponse;
    use aspen_client_api::LeaseGrantResultResponse;
    use aspen_client_api::LeaseInfo;
    use aspen_client_api::LeaseKeepaliveResultResponse;
    use aspen_client_api::LeaseListResultResponse;
    use aspen_client_api::LeaseRevokeResultResponse;
    use aspen_client_api::LeaseTimeToLiveResultResponse;
    use aspen_client_api::LockResultResponse;
    // Queue response types
    use aspen_client_api::QueueAckResultResponse;
    use aspen_client_api::QueueCreateResultResponse;
    use aspen_client_api::QueueDLQItemResponse;
    use aspen_client_api::QueueDeleteResultResponse;
    use aspen_client_api::QueueDequeueResultResponse;
    use aspen_client_api::QueueDequeuedItemResponse;
    use aspen_client_api::QueueEnqueueBatchResultResponse;
    use aspen_client_api::QueueEnqueueResultResponse;
    use aspen_client_api::QueueExtendVisibilityResultResponse;
    use aspen_client_api::QueueGetDLQResultResponse;
    use aspen_client_api::QueueItemResponse;
    use aspen_client_api::QueueNackResultResponse;
    use aspen_client_api::QueuePeekResultResponse;
    use aspen_client_api::QueueRedriveDLQResultResponse;
    use aspen_client_api::QueueStatusResultResponse;
    // RWLock response types
    use aspen_client_api::RWLockResultResponse;
    use aspen_client_api::RateLimiterResultResponse;
    // Semaphore response types
    use aspen_client_api::SemaphoreResultResponse;
    use aspen_client_api::SequenceResultResponse;
    // Service registry response types
    use aspen_client_api::ServiceDiscoverResultResponse;
    use aspen_client_api::ServiceGetInstanceResultResponse;
    use aspen_client_api::ServiceInstanceResponse;
    use aspen_client_api::ServiceListResultResponse;
    use aspen_client_api::WriteResultResponse;

    use super::*;

    /// Mock RPC client for testing.
    pub(crate) struct MockRpcClient {
        responses: Mutex<Vec<ClientRpcResponse>>,
    }

    impl MockRpcClient {
        pub(crate) fn new(responses: Vec<ClientRpcResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl CoordinationRpc for MockRpcClient {
        async fn send_coordination_request(&self, _request: ClientRpcRequest) -> Result<ClientRpcResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                bail!("no more mock responses");
            }
            Ok(responses.remove(0))
        }
    }

    #[tokio::test]
    async fn test_counter_client_increment() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::CounterResult(CounterResultResponse {
            is_success: true,
            value: Some(42),
            error: None,
        })]));

        let counter = CounterClient::new(client, "test");
        let value = counter.increment().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_sequence_client_next() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SequenceResult(SequenceResultResponse {
            is_success: true,
            value: Some(1001),
            error: None,
        })]));

        let seq = SequenceClient::new(client, "test");
        let value = seq.next().await.unwrap();
        assert_eq!(value, 1001);
    }

    #[tokio::test]
    async fn test_rate_limiter_acquired() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                is_success: true,
                tokens_remaining: Some(99),
                retry_after_ms: None,
                error: None,
            })]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_acquired());
        assert_eq!(result, RateLimitResult::Acquired { tokens_remaining: 99 });
    }

    #[tokio::test]
    async fn test_rate_limiter_rate_limited() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                is_success: false,
                tokens_remaining: Some(0),
                retry_after_ms: Some(500),
                error: None,
            })]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_rate_limited());
        assert_eq!(result, RateLimitResult::RateLimited { retry_after_ms: 500 });
    }

    #[tokio::test]
    async fn test_lock_client_acquire() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockResult(LockResultResponse {
            is_success: true,
            fencing_token: Some(42),
            holder_id: Some("holder-1".to_string()),
            deadline_ms: Some(1234567890),
            error: None,
        })]));

        let lock = LockClient::new(client, "test");
        let guard = lock.acquire("holder-1", Duration::from_secs(30), Duration::from_secs(5)).await.unwrap();

        assert_eq!(guard.fencing_token(), 42);
        assert_eq!(guard.deadline_ms(), 1234567890);
    }

    // ================================
    // LeaseClient tests
    // ================================

    #[tokio::test]
    async fn test_lease_grant_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                is_success: true,
                lease_id: Some(12345),
                ttl_seconds: Some(60),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant(60).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.ttl_seconds, 60);
    }

    #[tokio::test]
    async fn test_lease_grant_with_id_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                is_success: true,
                lease_id: Some(99999),
                ttl_seconds: Some(300),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant_with_id(300, Some(99999)).await.unwrap();

        assert_eq!(result.lease_id, 99999);
        assert_eq!(result.ttl_seconds, 300);
    }

    #[tokio::test]
    async fn test_lease_grant_failure() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                is_success: false,
                lease_id: None,
                ttl_seconds: None,
                error: Some("Lease ID already exists".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant_with_id(60, Some(12345)).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease ID already exists"));
    }

    #[tokio::test]
    async fn test_lease_revoke_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                is_success: true,
                keys_deleted: Some(5),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.revoke(12345).await.unwrap();

        assert_eq!(result.keys_deleted, 5);
    }

    #[tokio::test]
    async fn test_lease_revoke_not_found() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                is_success: false,
                keys_deleted: None,
                error: Some("Lease not found".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.revoke(99999).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease not found"));
    }

    #[tokio::test]
    async fn test_lease_keepalive_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                is_success: true,
                lease_id: Some(12345),
                ttl_seconds: Some(60),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.keepalive(12345).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.ttl_seconds, 60);
    }

    #[tokio::test]
    async fn test_lease_keepalive_expired() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                is_success: false,
                lease_id: None,
                ttl_seconds: None,
                error: Some("Lease expired or not found".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.keepalive(12345).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease expired or not found"));
    }

    #[tokio::test]
    async fn test_lease_time_to_live_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                is_success: true,
                lease_id: Some(12345),
                granted_ttl_seconds: Some(60),
                remaining_ttl_seconds: Some(45),
                keys: Some(vec!["key1".to_string(), "key2".to_string()]),
                error: None,
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(12345, true).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.granted_ttl_seconds, 60);
        assert_eq!(result.remaining_ttl_seconds, 45);
        assert_eq!(result.keys, vec!["key1".to_string(), "key2".to_string()]);
    }

    #[tokio::test]
    async fn test_lease_time_to_live_without_keys() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                is_success: true,
                lease_id: Some(12345),
                granted_ttl_seconds: Some(60),
                remaining_ttl_seconds: Some(45),
                keys: None,
                error: None,
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(12345, false).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert!(result.keys.is_empty());
    }

    #[tokio::test]
    async fn test_lease_time_to_live_not_found() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                is_success: false,
                lease_id: None,
                granted_ttl_seconds: None,
                remaining_ttl_seconds: None,
                keys: None,
                error: Some("Lease not found".to_string()),
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(99999, false).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease not found"));
    }

    #[tokio::test]
    async fn test_lease_list_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
            is_success: true,
            leases: Some(vec![
                LeaseInfo {
                    lease_id: 100,
                    granted_ttl_seconds: 60,
                    remaining_ttl_seconds: 50,
                    attached_keys: 2,
                },
                LeaseInfo {
                    lease_id: 200,
                    granted_ttl_seconds: 120,
                    remaining_ttl_seconds: 100,
                    attached_keys: 5,
                },
            ]),
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.list().await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].lease_id, 100);
        assert_eq!(result[1].lease_id, 200);
    }

    #[tokio::test]
    async fn test_lease_list_empty() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
            is_success: true,
            leases: Some(vec![]),
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.list().await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_lease_put_with_lease_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: true,
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.put_with_lease("mykey", b"myvalue", 12345).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lease_put_with_lease_failure() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some("Lease not found".to_string()),
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.put_with_lease("mykey", b"myvalue", 12345).await;

        assert!(result.is_err());
    }

    // ================================
    // LeaseKeepaliveHandle tests
    // ================================

    #[tokio::test]
    async fn test_lease_keepalive_handle_stop() {
        // This test verifies that the handle can be stopped
        let client = Arc::new(MockRpcClient::new(vec![]));
        let lease_client = LeaseClient::new(client);

        let handle = lease_client.start_keepalive(12345, Duration::from_millis(100));
        assert!(handle.is_running());
        assert_eq!(handle.lease_id(), 12345);

        handle.stop();
        // After stop, the task should be cancelled
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Can't check is_running after stop since handle is consumed
    }

    #[tokio::test]
    async fn test_lease_keepalive_sends_requests() {
        use std::sync::atomic::AtomicU32;
        use std::sync::atomic::Ordering;

        // Create a mock that counts keepalive requests
        struct CountingMockClient {
            keepalive_count: AtomicU32,
        }

        impl CoordinationRpc for CountingMockClient {
            async fn send_coordination_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
                match request {
                    ClientRpcRequest::LeaseKeepalive { .. } => {
                        self.keepalive_count.fetch_add(1, Ordering::SeqCst);
                        Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                            is_success: true,
                            lease_id: Some(12345),
                            ttl_seconds: Some(60),
                            error: None,
                        }))
                    }
                    _ => bail!("unexpected request"),
                }
            }
        }

        let mock = Arc::new(CountingMockClient {
            keepalive_count: AtomicU32::new(0),
        });
        let lease_client = LeaseClient::new(mock.clone());

        // Start keepalive with 50ms interval
        let handle = lease_client.start_keepalive(12345, Duration::from_millis(50));

        // Wait for a few keepalives
        tokio::time::sleep(Duration::from_millis(180)).await;

        handle.stop();

        // Should have sent at least 2-3 keepalives (one immediately, plus 2-3 more)
        let count = mock.keepalive_count.load(Ordering::SeqCst);
        assert!(count >= 2, "expected at least 2 keepalives, got {}", count);
    }

    // ================================
    // QueueClient tests
    // ================================

    #[tokio::test]
    async fn test_queue_create_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
                is_success: true,
                was_created: true,
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let was_created = queue.create(QueueCreateConfig::default()).await.unwrap();
        assert!(was_created);
    }

    #[tokio::test]
    async fn test_queue_create_failure() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
                is_success: false,
                was_created: false,
                error: Some("queue already exists".to_string()),
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let result = queue.create(QueueCreateConfig::default()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("queue already exists"));
    }

    #[tokio::test]
    async fn test_queue_enqueue_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
                is_success: true,
                item_id: Some(42),
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let item_id = queue.enqueue(b"hello".to_vec()).await.unwrap();
        assert_eq!(item_id, 42);
    }

    #[tokio::test]
    async fn test_queue_enqueue_batch_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueEnqueueBatchResult(
            QueueEnqueueBatchResultResponse {
                is_success: true,
                item_ids: vec![1, 2, 3],
                error: None,
            },
        )]));

        let queue = QueueClient::new(client, "test-queue");
        let items = vec![
            QueueEnqueueBatchItem {
                payload: b"item1".to_vec(),
                ttl: None,
                message_group_id: None,
                deduplication_id: None,
            },
            QueueEnqueueBatchItem {
                payload: b"item2".to_vec(),
                ttl: None,
                message_group_id: None,
                deduplication_id: None,
            },
        ];
        let item_ids = queue.enqueue_batch(items).await.unwrap();
        assert_eq!(item_ids, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_queue_dequeue_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: true,
                items: vec![QueueDequeuedItemResponse {
                    item_id: 100,
                    payload: b"payload".to_vec(),
                    receipt_handle: "receipt-123".to_string(),
                    delivery_attempts: 1,
                    enqueued_at_ms: 1000,
                    visibility_deadline_ms: 2000,
                }],
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let items = queue.dequeue("consumer-1", 10, Duration::from_secs(30)).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, 100);
        assert_eq!(items[0].receipt_handle, "receipt-123");
    }

    #[tokio::test]
    async fn test_queue_dequeue_empty() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: true,
                items: vec![],
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let items = queue.dequeue("consumer-1", 10, Duration::from_secs(30)).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_queue_peek_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
            is_success: true,
            items: vec![QueueItemResponse {
                item_id: 50,
                payload: b"peeked".to_vec(),
                enqueued_at_ms: 1000,
                expires_at_ms: 2000,
                delivery_attempts: 0,
            }],
            error: None,
        })]));

        let queue = QueueClient::new(client, "test-queue");
        let items = queue.peek(10).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, 50);
        assert_eq!(items[0].delivery_attempts, 0);
    }

    #[tokio::test]
    async fn test_queue_ack_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
            is_success: true,
            error: None,
        })]));

        let queue = QueueClient::new(client, "test-queue");
        let result = queue.ack("receipt-123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_nack_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            is_success: true,
            error: None,
        })]));

        let queue = QueueClient::new(client, "test-queue");
        let result = queue.nack("receipt-123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_nack_to_dlq() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            is_success: true,
            error: None,
        })]));

        let queue = QueueClient::new(client, "test-queue");
        let result = queue.nack_to_dlq("receipt-123", Some("processing failed".to_string())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_status_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
                is_success: true,
                does_exist: true,
                visible_count: Some(10),
                pending_count: Some(5),
                dlq_count: Some(2),
                total_enqueued: Some(100),
                total_acked: Some(80),
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let status = queue.status().await.unwrap();
        assert!(status.does_exist);
        assert_eq!(status.visible_count, 10);
        assert_eq!(status.pending_count, 5);
        assert_eq!(status.dlq_count, 2);
        assert_eq!(status.total_enqueued, 100);
        assert_eq!(status.total_acked, 80);
    }

    #[tokio::test]
    async fn test_queue_delete_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
                is_success: true,
                items_deleted: Some(42),
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let deleted = queue.delete().await.unwrap();
        assert_eq!(deleted, 42);
    }

    #[tokio::test]
    async fn test_queue_get_dlq() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
                is_success: true,
                items: vec![QueueDLQItemResponse {
                    item_id: 99,
                    payload: b"failed".to_vec(),
                    enqueued_at_ms: 1000,
                    delivery_attempts: 5,
                    reason: "max retries exceeded".to_string(),
                    moved_at_ms: 2000,
                    last_error: Some("processing error".to_string()),
                }],
                error: None,
            })]));

        let queue = QueueClient::new(client, "test-queue");
        let items = queue.get_dlq(10).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, 99);
        assert_eq!(items[0].delivery_attempts, 5);
        assert_eq!(items[0].reason, "max retries exceeded");
    }

    #[tokio::test]
    async fn test_queue_redrive_dlq() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueRedriveDLQResult(
            QueueRedriveDLQResultResponse {
                is_success: true,
                error: None,
            },
        )]));

        let queue = QueueClient::new(client, "test-queue");
        let result = queue.redrive_dlq(99).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_extend_visibility() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::QueueExtendVisibilityResult(
            QueueExtendVisibilityResultResponse {
                is_success: true,
                new_deadline_ms: Some(5000),
                error: None,
            },
        )]));

        let queue = QueueClient::new(client, "test-queue");
        let new_deadline = queue.extend_visibility("receipt-123", Duration::from_secs(10)).await.unwrap();
        assert_eq!(new_deadline, 5000);
    }

    // ================================
    // SemaphoreClient tests
    // ================================

    #[tokio::test]
    async fn test_semaphore_acquire_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
                is_success: true,
                permits_acquired: Some(2),
                available: Some(3),
                capacity_permits: None,
                retry_after_ms: None,
                error: None,
            })]));

        let semaphore = SemaphoreClient::new(client);
        let result = semaphore.acquire("test-sem", "holder-1", 2, 5, Duration::from_secs(60), None).await.unwrap();
        assert_eq!(result.permits_acquired, 2);
        assert_eq!(result.available, 3);
    }

    #[tokio::test]
    async fn test_semaphore_acquire_failure() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
                is_success: false,
                permits_acquired: None,
                available: None,
                capacity_permits: None,
                retry_after_ms: None,
                error: Some("timeout waiting for permits".to_string()),
            })]));

        let semaphore = SemaphoreClient::new(client);
        let result = semaphore
            .acquire("test-sem", "holder-1", 2, 5, Duration::from_secs(60), Some(Duration::from_secs(1)))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout waiting for permits"));
    }

    #[tokio::test]
    async fn test_semaphore_try_acquire_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                is_success: true,
                permits_acquired: Some(1),
                available: Some(4),
                capacity_permits: None,
                retry_after_ms: None,
                error: None,
            })]));

        let semaphore = SemaphoreClient::new(client);
        let result = semaphore.try_acquire("test-sem", "holder-1", 1, 5, Duration::from_secs(60)).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().permits_acquired, 1);
    }

    #[tokio::test]
    async fn test_semaphore_try_acquire_not_available() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                is_success: false,
                permits_acquired: None,
                available: Some(0),
                capacity_permits: None,
                retry_after_ms: None,
                error: None,
            })]));

        let semaphore = SemaphoreClient::new(client);
        let result = semaphore.try_acquire("test-sem", "holder-1", 3, 5, Duration::from_secs(60)).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_semaphore_release_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
                is_success: true,
                permits_acquired: None,
                available: Some(5),
                capacity_permits: None,
                retry_after_ms: None,
                error: None,
            })]));

        let semaphore = SemaphoreClient::new(client);
        let available = semaphore.release("test-sem", "holder-1", 2).await.unwrap();
        assert_eq!(available, 5);
    }

    #[tokio::test]
    async fn test_semaphore_status_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
                is_success: true,
                permits_acquired: None,
                available: Some(3),
                capacity_permits: Some(5),
                retry_after_ms: None,
                error: None,
            })]));

        let semaphore = SemaphoreClient::new(client);
        let status = semaphore.status("test-sem").await.unwrap();
        assert_eq!(status.available, 3);
        assert_eq!(status.capacity_permits, 5);
    }

    // ================================
    // RWLockClient tests
    // ================================

    #[tokio::test]
    async fn test_rwlock_acquire_read_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                is_success: true,
                mode: Some("read".to_string()),
                fencing_token: Some(1),
                deadline_ms: Some(9999),
                reader_count: Some(1),
                writer_holder: None,
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.acquire_read("test-lock", "reader-1", Duration::from_secs(30), None).await.unwrap();
        assert_eq!(result.fencing_token, 1);
        assert_eq!(result.deadline_ms, 9999);
        assert_eq!(result.reader_count, 1);
    }

    #[tokio::test]
    async fn test_rwlock_acquire_read_failure() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                is_success: false,
                mode: None,
                fencing_token: None,
                deadline_ms: None,
                reader_count: None,
                writer_holder: None,
                error: Some("timeout acquiring read lock".to_string()),
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock
            .acquire_read("test-lock", "reader-1", Duration::from_secs(30), Some(Duration::from_secs(1)))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout acquiring read lock"));
    }

    #[tokio::test]
    async fn test_rwlock_try_acquire_read_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                is_success: true,
                mode: Some("read".to_string()),
                fencing_token: Some(2),
                deadline_ms: Some(8888),
                reader_count: Some(2),
                writer_holder: None,
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.try_acquire_read("test-lock", "reader-2", Duration::from_secs(30)).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().reader_count, 2);
    }

    #[tokio::test]
    async fn test_rwlock_try_acquire_read_not_available() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                is_success: false,
                mode: None,
                fencing_token: None,
                deadline_ms: None,
                reader_count: None,
                writer_holder: None,
                error: Some("lock not available".to_string()),
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.try_acquire_read("test-lock", "reader-1", Duration::from_secs(30)).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_acquire_write_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
                is_success: true,
                mode: Some("write".to_string()),
                fencing_token: Some(10),
                deadline_ms: Some(7777),
                reader_count: None,
                writer_holder: Some("writer-1".to_string()),
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.acquire_write("test-lock", "writer-1", Duration::from_secs(30), None).await.unwrap();
        assert_eq!(result.fencing_token, 10);
        assert_eq!(result.deadline_ms, 7777);
    }

    #[tokio::test]
    async fn test_rwlock_try_acquire_write_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                is_success: true,
                mode: Some("write".to_string()),
                fencing_token: Some(11),
                deadline_ms: Some(6666),
                reader_count: None,
                writer_holder: Some("writer-2".to_string()),
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.try_acquire_write("test-lock", "writer-2", Duration::from_secs(30)).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().fencing_token, 11);
    }

    #[tokio::test]
    async fn test_rwlock_try_acquire_write_not_available() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                is_success: false,
                mode: None,
                fencing_token: None,
                deadline_ms: None,
                reader_count: None,
                writer_holder: None,
                error: Some("lock not available".to_string()),
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.try_acquire_write("test-lock", "writer-1", Duration::from_secs(30)).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_release_read_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
                is_success: true,
                mode: None,
                fencing_token: None,
                deadline_ms: None,
                reader_count: None,
                writer_holder: None,
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.release_read("test-lock", "reader-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rwlock_release_write_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
                is_success: true,
                mode: None,
                fencing_token: None,
                deadline_ms: None,
                reader_count: None,
                writer_holder: None,
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.release_write("test-lock", "writer-1", 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rwlock_downgrade_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
                is_success: true,
                mode: Some("read".to_string()),
                fencing_token: Some(10),
                deadline_ms: Some(5555),
                reader_count: Some(1),
                writer_holder: None,
                error: None,
            })]));

        let rwlock = RWLockClient::new(client);
        let result = rwlock.downgrade("test-lock", "writer-1", 10, Duration::from_secs(30)).await.unwrap();
        assert_eq!(result.fencing_token, 10);
        assert_eq!(result.reader_count, 1);
    }

    #[tokio::test]
    async fn test_rwlock_status_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
            is_success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(5),
            deadline_ms: None,
            reader_count: Some(3),
            writer_holder: None,
            error: None,
        })]));

        let rwlock = RWLockClient::new(client);
        let status = rwlock.status("test-lock").await.unwrap();
        assert_eq!(status.mode, "read");
        assert_eq!(status.fencing_token, 5);
        assert_eq!(status.reader_count, 3);
        assert!(status.writer_holder.is_none());
    }

    // ================================
    // ServiceClient tests
    // ================================

    #[tokio::test]
    async fn test_service_discover_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::ServiceDiscoverResult(
            ServiceDiscoverResultResponse {
                is_success: true,
                instances: vec![
                    ServiceInstanceResponse {
                        instance_id: "inst-1".to_string(),
                        service_name: "test-svc".to_string(),
                        address: "10.0.0.1:8080".to_string(),
                        health_status: "healthy".to_string(),
                        version: "1.0.0".to_string(),
                        tags: vec![],
                        weight: 100,
                        custom_metadata: "{}".to_string(),
                        registered_at_ms: 1000,
                        last_heartbeat_ms: 2000,
                        deadline_ms: 3000,
                        lease_id: None,
                        fencing_token: 1,
                    },
                    ServiceInstanceResponse {
                        instance_id: "inst-2".to_string(),
                        service_name: "test-svc".to_string(),
                        address: "10.0.0.2:8080".to_string(),
                        health_status: "healthy".to_string(),
                        version: "1.0.0".to_string(),
                        tags: vec![],
                        weight: 100,
                        custom_metadata: "{}".to_string(),
                        registered_at_ms: 1500,
                        last_heartbeat_ms: 2500,
                        deadline_ms: 3500,
                        lease_id: None,
                        fencing_token: 2,
                    },
                ],
                count: 2,
                error: None,
            },
        )]));

        let service = ServiceClient::new(client);
        let instances = service.discover("test-svc", None).await.unwrap();
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].instance_id, "inst-1");
        assert_eq!(instances[1].instance_id, "inst-2");
    }

    #[tokio::test]
    async fn test_service_discover_empty() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::ServiceDiscoverResult(
            ServiceDiscoverResultResponse {
                is_success: true,
                instances: vec![],
                count: 0,
                error: None,
            },
        )]));

        let service = ServiceClient::new(client);
        let instances = service.discover("test-svc", None).await.unwrap();
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_service_list_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                is_success: true,
                services: vec![
                    "api-service".to_string(),
                    "db-service".to_string(),
                    "cache-service".to_string(),
                ],
                count: 3,
                error: None,
            })]));

        let service = ServiceClient::new(client);
        let services = service.list_services("", None).await.unwrap();
        assert_eq!(services.len(), 3);
        assert_eq!(services[0], "api-service");
        assert_eq!(services[1], "db-service");
        assert_eq!(services[2], "cache-service");
    }

    #[tokio::test]
    async fn test_service_get_instance_found() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::ServiceGetInstanceResult(
            ServiceGetInstanceResultResponse {
                is_success: true,
                was_found: true,
                instance: Some(ServiceInstanceResponse {
                    instance_id: "inst-1".to_string(),
                    service_name: "test-svc".to_string(),
                    address: "10.0.0.1:8080".to_string(),
                    health_status: "healthy".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec![],
                    weight: 100,
                    custom_metadata: "{}".to_string(),
                    registered_at_ms: 1000,
                    last_heartbeat_ms: 2000,
                    deadline_ms: 3000,
                    lease_id: Some(999),
                    fencing_token: 1,
                }),
                error: None,
            },
        )]));

        let service = ServiceClient::new(client);
        let instance = service.get_instance("test-svc", "inst-1").await.unwrap();
        assert!(instance.is_some());
        let inst = instance.unwrap();
        assert_eq!(inst.instance_id, "inst-1");
        assert_eq!(inst.service_name, "test-svc");
        assert_eq!(inst.address, "10.0.0.1:8080");
        assert_eq!(inst.lease_id, Some(999));
    }

    #[tokio::test]
    async fn test_service_get_instance_not_found() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::ServiceGetInstanceResult(
            ServiceGetInstanceResultResponse {
                is_success: true,
                was_found: false,
                instance: None,
                error: None,
            },
        )]));

        let service = ServiceClient::new(client);
        let instance = service.get_instance("test-svc", "inst-999").await.unwrap();
        assert!(instance.is_none());
    }
}
