//! Native coordination primitive handler.
//!
//! Provides native implementations for the most commonly used coordination
//! primitives (counter, sequence, lock) so they work without WASM plugins.
//! Complex operations (queue, barrier, semaphore, rwlock, service registry)
//! return NOT_IMPLEMENTED errors directing users to load the WASM plugin.
//!
//! ## Why native?
//!
//! Without this handler, coordination requests hit the "no handler found"
//! path in the registry, which either returns `CapabilityUnavailable` (if
//! the request has a `required_app`) or a generic error. Since coordination
//! primitives return `required_app() = None` (they're core), the error path
//! is opaque. This handler gives clean, actionable error messages.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::coordination::CounterResultResponse;
use aspen_client_api::coordination::LockResultResponse;
use aspen_client_api::coordination::SequenceResultResponse;
use aspen_client_api::coordination::SignedCounterResultResponse;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedLock;
use aspen_coordination::LockConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_coordination::SignedAtomicCounter;
use aspen_core::KeyValueStore;
use async_trait::async_trait;

use crate::ClientProtocolContext;
use crate::RequestHandler;

/// Native handler for distributed coordination primitives.
///
/// Handles counter, sequence, and lock operations natively using the
/// `aspen-coordination` library. Other coordination operations (queue,
/// barrier, semaphore, rwlock, service registry) return NOT_IMPLEMENTED.
pub struct CoordinationHandler;

impl CoordinationHandler {
    /// KV key prefix for coordination counters.
    const COUNTER_PREFIX: &'static str = "__coord:counter:";
    /// KV key prefix for coordination sequences.
    const SEQUENCE_PREFIX: &'static str = "__coord:seq:";
    /// KV key prefix for coordination locks.
    const LOCK_PREFIX: &'static str = "__coord:lock:";

    fn not_implemented(op: &str) -> ClientRpcResponse {
        ClientRpcResponse::error(
            "NOT_IMPLEMENTED",
            format!("{op} requires the WASM coordination plugin; load it with `aspen-cli plugin reload`"),
        )
    }
}

#[async_trait]
impl RequestHandler for CoordinationHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            // Lock
            ClientRpcRequest::LockAcquire { .. }
                | ClientRpcRequest::LockTryAcquire { .. }
                | ClientRpcRequest::LockRelease { .. }
                | ClientRpcRequest::LockRenew { .. }
                // Counter
                | ClientRpcRequest::CounterGet { .. }
                | ClientRpcRequest::CounterIncrement { .. }
                | ClientRpcRequest::CounterDecrement { .. }
                | ClientRpcRequest::CounterAdd { .. }
                | ClientRpcRequest::CounterSubtract { .. }
                | ClientRpcRequest::CounterSet { .. }
                | ClientRpcRequest::CounterCompareAndSet { .. }
                // Signed counter
                | ClientRpcRequest::SignedCounterGet { .. }
                | ClientRpcRequest::SignedCounterAdd { .. }
                // Sequence
                | ClientRpcRequest::SequenceNext { .. }
                | ClientRpcRequest::SequenceReserve { .. }
                | ClientRpcRequest::SequenceCurrent { .. }
                // Rate limiter
                | ClientRpcRequest::RateLimiterTryAcquire { .. }
                | ClientRpcRequest::RateLimiterAcquire { .. }
                | ClientRpcRequest::RateLimiterAvailable { .. }
                | ClientRpcRequest::RateLimiterReset { .. }
                // Barrier
                | ClientRpcRequest::BarrierEnter { .. }
                | ClientRpcRequest::BarrierLeave { .. }
                | ClientRpcRequest::BarrierStatus { .. }
                // Semaphore
                | ClientRpcRequest::SemaphoreAcquire { .. }
                | ClientRpcRequest::SemaphoreTryAcquire { .. }
                | ClientRpcRequest::SemaphoreRelease { .. }
                | ClientRpcRequest::SemaphoreStatus { .. }
                // RWLock
                | ClientRpcRequest::RWLockAcquireRead { .. }
                | ClientRpcRequest::RWLockTryAcquireRead { .. }
                | ClientRpcRequest::RWLockAcquireWrite { .. }
                | ClientRpcRequest::RWLockTryAcquireWrite { .. }
                | ClientRpcRequest::RWLockReleaseRead { .. }
                | ClientRpcRequest::RWLockReleaseWrite { .. }
                | ClientRpcRequest::RWLockDowngrade { .. }
                | ClientRpcRequest::RWLockStatus { .. }
                // Queue
                | ClientRpcRequest::QueueCreate { .. }
                | ClientRpcRequest::QueueDelete { .. }
                | ClientRpcRequest::QueueEnqueue { .. }
                | ClientRpcRequest::QueueEnqueueBatch { .. }
                | ClientRpcRequest::QueueDequeue { .. }
                | ClientRpcRequest::QueueDequeueWait { .. }
                | ClientRpcRequest::QueuePeek { .. }
                | ClientRpcRequest::QueueAck { .. }
                | ClientRpcRequest::QueueNack { .. }
                | ClientRpcRequest::QueueExtendVisibility { .. }
                | ClientRpcRequest::QueueStatus { .. }
                | ClientRpcRequest::QueueGetDLQ { .. }
                | ClientRpcRequest::QueueRedriveDLQ { .. }
                // Service registry
                | ClientRpcRequest::ServiceRegister { .. }
                | ClientRpcRequest::ServiceDeregister { .. }
                | ClientRpcRequest::ServiceDiscover { .. }
                | ClientRpcRequest::ServiceList { .. }
                | ClientRpcRequest::ServiceGetInstance { .. }
                | ClientRpcRequest::ServiceHeartbeat { .. }
                | ClientRpcRequest::ServiceUpdateHealth { .. }
                | ClientRpcRequest::ServiceUpdateMetadata { .. }
        )
    }

    async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        match request {
            // =================================================================
            // Counter operations (native)
            // =================================================================
            ClientRpcRequest::CounterGet { key } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.get().await {
                    Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterIncrement { key } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.increment().await {
                    Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterDecrement { key } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.decrement().await {
                    Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterAdd { key, amount } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.add(amount).await {
                    Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterSubtract { key, amount } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.subtract(amount).await {
                    Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterSet { key, value } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.set(value).await {
                    Ok(()) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::CounterCompareAndSet {
                key,
                expected,
                new_value,
            } => {
                let counter = self.make_counter(&ctx.kv_store, &key);
                match counter.compare_and_set(expected, new_value).await {
                    Ok(swapped) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: swapped,
                        value: Some(if swapped { new_value } else { expected }),
                        error: if swapped {
                            None
                        } else {
                            Some("compare-and-set failed: value has changed".into())
                        },
                    })),
                    Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

            // =================================================================
            // Signed counter operations (native)
            // =================================================================
            ClientRpcRequest::SignedCounterGet { key } => {
                let counter = self.make_signed_counter(&ctx.kv_store, &key);
                match counter.get().await {
                    Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::SignedCounterAdd { key, amount } => {
                let counter = self.make_signed_counter(&ctx.kv_store, &key);
                match counter.add(amount).await {
                    Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

            // =================================================================
            // Sequence operations (native)
            // =================================================================
            ClientRpcRequest::SequenceNext { key } => {
                let seq = self.make_sequence(&ctx.kv_store, &key);
                match seq.next().await {
                    Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::SequenceReserve { key, count } => {
                let seq = self.make_sequence(&ctx.kv_store, &key);
                match seq.reserve(count).await {
                    Ok(start) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: true,
                        value: Some(start),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::SequenceCurrent { key } => {
                let seq = self.make_sequence(&ctx.kv_store, &key);
                match seq.current().await {
                    Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: true,
                        value: Some(value),
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                        is_success: false,
                        value: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

            // =================================================================
            // Lock operations (partial native: try_acquire + release)
            // =================================================================
            ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } => {
                let lock = self.make_lock(&ctx.kv_store, &key, &holder_id, ttl_ms);
                match lock.try_acquire().await {
                    Ok(guard) => {
                        let token = guard.fencing_token();
                        // Intentionally forget the guard so the lock persists beyond this request.
                        // The lock is released via LockRelease or TTL expiration.
                        std::mem::forget(guard);
                        Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            is_success: true,
                            fencing_token: Some(token.value()),
                            holder_id: Some(holder_id),
                            deadline_ms: None,
                            error: None,
                        }))
                    }
                    Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        is_success: false,
                        fencing_token: None,
                        holder_id: None,
                        deadline_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::LockRelease {
                key,
                holder_id,
                fencing_token: _,
            } => {
                // Release by writing a "released" lock entry via CAS
                // The simplest approach: delete the lock key
                let lock_key = format!("{}{key}", Self::LOCK_PREFIX);
                match ctx.kv_store.delete(aspen_core::DeleteRequest { key: lock_key }).await {
                    Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        is_success: true,
                        fencing_token: None,
                        holder_id: Some(holder_id),
                        deadline_ms: None,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        is_success: false,
                        fencing_token: None,
                        holder_id: Some(holder_id),
                        deadline_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

            // Lock acquire (blocking) and renew — need WASM for timeout handling
            ClientRpcRequest::LockAcquire { .. } => Ok(Self::not_implemented("lock acquire (blocking)")),
            ClientRpcRequest::LockRenew { .. } => Ok(Self::not_implemented("lock renew")),

            // =================================================================
            // NOT_IMPLEMENTED: complex operations require WASM plugin
            // =================================================================
            ClientRpcRequest::RateLimiterTryAcquire { .. }
            | ClientRpcRequest::RateLimiterAcquire { .. }
            | ClientRpcRequest::RateLimiterAvailable { .. }
            | ClientRpcRequest::RateLimiterReset { .. } => Ok(Self::not_implemented("rate limiter")),

            ClientRpcRequest::BarrierEnter { .. }
            | ClientRpcRequest::BarrierLeave { .. }
            | ClientRpcRequest::BarrierStatus { .. } => Ok(Self::not_implemented("barrier")),

            ClientRpcRequest::SemaphoreAcquire { .. }
            | ClientRpcRequest::SemaphoreTryAcquire { .. }
            | ClientRpcRequest::SemaphoreRelease { .. }
            | ClientRpcRequest::SemaphoreStatus { .. } => Ok(Self::not_implemented("semaphore")),

            ClientRpcRequest::RWLockAcquireRead { .. }
            | ClientRpcRequest::RWLockTryAcquireRead { .. }
            | ClientRpcRequest::RWLockAcquireWrite { .. }
            | ClientRpcRequest::RWLockTryAcquireWrite { .. }
            | ClientRpcRequest::RWLockReleaseRead { .. }
            | ClientRpcRequest::RWLockReleaseWrite { .. }
            | ClientRpcRequest::RWLockDowngrade { .. }
            | ClientRpcRequest::RWLockStatus { .. } => Ok(Self::not_implemented("rwlock")),

            ClientRpcRequest::QueueCreate { .. }
            | ClientRpcRequest::QueueDelete { .. }
            | ClientRpcRequest::QueueEnqueue { .. }
            | ClientRpcRequest::QueueEnqueueBatch { .. }
            | ClientRpcRequest::QueueDequeue { .. }
            | ClientRpcRequest::QueueDequeueWait { .. }
            | ClientRpcRequest::QueuePeek { .. }
            | ClientRpcRequest::QueueAck { .. }
            | ClientRpcRequest::QueueNack { .. }
            | ClientRpcRequest::QueueExtendVisibility { .. }
            | ClientRpcRequest::QueueStatus { .. }
            | ClientRpcRequest::QueueGetDLQ { .. }
            | ClientRpcRequest::QueueRedriveDLQ { .. } => Ok(Self::not_implemented("queue")),

            ClientRpcRequest::ServiceRegister { .. }
            | ClientRpcRequest::ServiceDeregister { .. }
            | ClientRpcRequest::ServiceDiscover { .. }
            | ClientRpcRequest::ServiceList { .. }
            | ClientRpcRequest::ServiceGetInstance { .. }
            | ClientRpcRequest::ServiceHeartbeat { .. }
            | ClientRpcRequest::ServiceUpdateHealth { .. }
            | ClientRpcRequest::ServiceUpdateMetadata { .. } => Ok(Self::not_implemented("service registry")),

            _ => Err(anyhow::anyhow!("unexpected request in CoordinationHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CoordinationHandler"
    }
}

// =============================================================================
// Helper methods for creating coordination primitives
// =============================================================================

impl CoordinationHandler {
    fn make_counter(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> AtomicCounter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::COUNTER_PREFIX);
        AtomicCounter::new(Arc::clone(store), prefixed, CounterConfig::default())
    }

    fn make_signed_counter(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> SignedAtomicCounter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::COUNTER_PREFIX);
        SignedAtomicCounter::new(Arc::clone(store), prefixed, CounterConfig::default())
    }

    fn make_sequence(&self, store: &Arc<dyn KeyValueStore>, key: &str) -> SequenceGenerator<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::SEQUENCE_PREFIX);
        SequenceGenerator::new(Arc::clone(store), prefixed, SequenceConfig::default())
    }

    fn make_lock(
        &self,
        store: &Arc<dyn KeyValueStore>,
        key: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> DistributedLock<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::LOCK_PREFIX);
        let config = LockConfig {
            ttl_ms,
            ..LockConfig::default()
        };
        DistributedLock::new(Arc::clone(store), prefixed, holder_id, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> CoordinationHandler {
        CoordinationHandler
    }

    // =========================================================================
    // can_handle coverage
    // =========================================================================

    #[test]
    fn test_can_handle_counter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::CounterGet { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterIncrement { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterDecrement { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::CounterAdd {
            key: "k".into(),
            amount: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterSubtract {
            key: "k".into(),
            amount: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterSet {
            key: "k".into(),
            value: 0,
        }));
        assert!(h.can_handle(&ClientRpcRequest::CounterCompareAndSet {
            key: "k".into(),
            expected: 0,
            new_value: 1,
        }));
    }

    #[test]
    fn test_can_handle_signed_counter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::SignedCounterGet { key: "k".into() }));
        assert!(h.can_handle(&ClientRpcRequest::SignedCounterAdd {
            key: "k".into(),
            amount: -1,
        }));
    }

    #[test]
    fn test_can_handle_sequence_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::SequenceNext { key: "s".into() }));
        assert!(h.can_handle(&ClientRpcRequest::SequenceReserve {
            key: "s".into(),
            count: 10,
        }));
        assert!(h.can_handle(&ClientRpcRequest::SequenceCurrent { key: "s".into() }));
    }

    #[test]
    fn test_can_handle_lock_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::LockAcquire {
            key: "l".into(),
            holder_id: "h".into(),
            ttl_ms: 1000,
            timeout_ms: 5000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockTryAcquire {
            key: "l".into(),
            holder_id: "h".into(),
            ttl_ms: 1000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockRelease {
            key: "l".into(),
            holder_id: "h".into(),
            fencing_token: 1,
        }));
        assert!(h.can_handle(&ClientRpcRequest::LockRenew {
            key: "l".into(),
            holder_id: "h".into(),
            fencing_token: 1,
            ttl_ms: 1000,
        }));
    }

    #[test]
    fn test_can_handle_ratelimiter_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterTryAcquire {
            key: "r".into(),
            tokens: 1,
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
    }

    #[test]
    fn test_can_handle_queue_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::QueueCreate {
            queue_name: "q".into(),
            default_visibility_timeout_ms: None,
            default_ttl_ms: None,
            max_delivery_attempts: None,
        }));
        assert!(h.can_handle(&ClientRpcRequest::QueueStatus { queue_name: "q".into() }));
    }

    #[test]
    fn test_can_handle_service_registry_variants() {
        let h = handler();
        assert!(h.can_handle(&ClientRpcRequest::ServiceDiscover {
            service_name: "svc".into(),
            healthy_only: true,
            tags: String::new(),
            version_prefix: None,
            limit: None,
        }));
    }

    #[test]
    fn test_cannot_handle_non_coordination() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::Ping));
        assert!(!h.can_handle(&ClientRpcRequest::GetHealth));
        assert!(!h.can_handle(&ClientRpcRequest::ReadKey { key: "k".into() }));
    }
}
