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
    use aspen_client_api::RateLimiterResultResponse;
    use aspen_client_api::SequenceResultResponse;
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
}
