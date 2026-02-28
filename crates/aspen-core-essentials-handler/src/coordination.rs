//! Native coordination primitive handler.
//!
//! Provides native implementations for coordination primitives that have
//! building blocks in `aspen-coordination`: counters, sequences, locks,
//! and rate limiters.
//!
//! Operations that require the WASM coordination plugin (queue, barrier,
//! semaphore, rwlock, service registry) are NOT claimed by this handler,
//! allowing them to fall through to the WASM plugin handler in the
//! dispatch chain.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::RateLimiterResultResponse;
use aspen_client_api::coordination::CounterResultResponse;
use aspen_client_api::coordination::LockResultResponse;
use aspen_client_api::coordination::SequenceResultResponse;
use aspen_client_api::coordination::SignedCounterResultResponse;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedLock;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::LockConfig;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_coordination::SignedAtomicCounter;
use aspen_core::KeyValueStore;
use async_trait::async_trait;

use crate::ClientProtocolContext;
use crate::RequestHandler;

/// Native handler for distributed coordination primitives.
///
/// Handles counter, sequence, lock, and rate limiter operations natively
/// using the `aspen-coordination` library. Other coordination operations
/// (queue, barrier, semaphore, rwlock, service registry) are NOT claimed
/// here — they fall through to the WASM plugin handler.
pub struct CoordinationHandler;

impl CoordinationHandler {
    /// KV key prefix for coordination counters.
    const COUNTER_PREFIX: &'static str = "__coord:counter:";
    /// KV key prefix for coordination sequences.
    const SEQUENCE_PREFIX: &'static str = "__coord:seq:";
    /// KV key prefix for coordination locks.
    const LOCK_PREFIX: &'static str = "__coord:lock:";
    /// KV key prefix for rate limiters.
    const RATE_LIMITER_PREFIX: &'static str = "__coord:ratelimit:";
}

#[async_trait]
impl RequestHandler for CoordinationHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            // Lock (all four operations implemented natively)
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
                // Rate limiter (all four operations implemented natively)
                | ClientRpcRequest::RateLimiterTryAcquire { .. }
                | ClientRpcRequest::RateLimiterAcquire { .. }
                | ClientRpcRequest::RateLimiterAvailable { .. }
                | ClientRpcRequest::RateLimiterReset { .. }
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
            // Lock operations (all native)
            // =================================================================
            ClientRpcRequest::LockAcquire {
                key,
                holder_id,
                ttl_ms,
                timeout_ms,
            } => {
                let config = LockConfig {
                    ttl_ms,
                    acquire_timeout_ms: timeout_ms,
                    ..LockConfig::default()
                };
                let lock = DistributedLock::new(
                    Arc::clone(&ctx.kv_store),
                    format!("{}{key}", Self::LOCK_PREFIX),
                    &holder_id,
                    config,
                );
                match lock.acquire().await {
                    Ok(guard) => {
                        let token = guard.fencing_token();
                        let deadline = guard.deadline_ms();
                        // Intentionally forget the guard so the lock persists beyond this request.
                        // The lock is released via LockRelease or TTL expiration.
                        std::mem::forget(guard);
                        Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            is_success: true,
                            fencing_token: Some(token.value()),
                            holder_id: Some(holder_id),
                            deadline_ms: Some(deadline),
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
            ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } => {
                let lock = self.make_lock(&ctx.kv_store, &key, &holder_id, ttl_ms);
                match lock.try_acquire().await {
                    Ok(guard) => {
                        let token = guard.fencing_token();
                        let deadline = guard.deadline_ms();
                        std::mem::forget(guard);
                        Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            is_success: true,
                            fencing_token: Some(token.value()),
                            holder_id: Some(holder_id),
                            deadline_ms: Some(deadline),
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
            ClientRpcRequest::LockRenew {
                key,
                holder_id,
                fencing_token,
                ttl_ms,
            } => {
                // Renew by re-acquiring with same holder — if holder matches,
                // DistributedLock.try_acquire() re-ups the deadline for the same holder.
                let lock = self.make_lock(&ctx.kv_store, &key, &holder_id, ttl_ms);
                match lock.try_acquire().await {
                    Ok(guard) => {
                        let new_token = guard.fencing_token().value();
                        let deadline = guard.deadline_ms();
                        std::mem::forget(guard);
                        // Verify fencing token matches (holder hasn't lost and regained the lock)
                        if new_token != fencing_token {
                            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                is_success: false,
                                fencing_token: Some(new_token),
                                holder_id: Some(holder_id),
                                deadline_ms: Some(deadline),
                                error: Some(format!(
                                    "fencing token mismatch: expected {fencing_token}, got {new_token}"
                                )),
                            }))
                        } else {
                            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                is_success: true,
                                fencing_token: Some(new_token),
                                holder_id: Some(holder_id),
                                deadline_ms: Some(deadline),
                                error: None,
                            }))
                        }
                    }
                    Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        is_success: false,
                        fencing_token: None,
                        holder_id: Some(holder_id),
                        deadline_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

            // =================================================================
            // Rate limiter operations (native)
            // =================================================================
            ClientRpcRequest::RateLimiterTryAcquire {
                key,
                tokens,
                capacity_tokens,
                refill_rate,
            } => {
                let limiter = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
                match limiter.try_acquire_n(tokens).await {
                    Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    })),
                    Err(aspen_coordination::RateLimitError::TokensExhausted { retry_after_ms, .. }) => {
                        Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                            is_success: false,
                            tokens_remaining: None,
                            retry_after_ms: Some(retry_after_ms),
                            error: Some("rate limited".into()),
                        }))
                    }
                    Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::RateLimiterAcquire {
                key,
                tokens,
                capacity_tokens,
                refill_rate,
                timeout_ms,
            } => {
                let limiter = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
                match limiter.acquire_n(tokens, Duration::from_millis(timeout_ms)).await {
                    Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::RateLimiterAvailable {
                key,
                capacity_tokens,
                refill_rate,
            } => {
                let limiter = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
                match limiter.available().await {
                    Ok(available) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: true,
                        tokens_remaining: Some(available),
                        retry_after_ms: None,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }
            ClientRpcRequest::RateLimiterReset {
                key,
                capacity_tokens,
                refill_rate,
            } => {
                let limiter = self.make_rate_limiter(&ctx.kv_store, &key, refill_rate, capacity_tokens);
                match limiter.reset().await {
                    Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: true,
                        tokens_remaining: Some(capacity_tokens),
                        retry_after_ms: None,
                        error: None,
                    })),
                    Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        is_success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    })),
                }
            }

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

    fn make_rate_limiter(
        &self,
        store: &Arc<dyn KeyValueStore>,
        key: &str,
        refill_rate: f64,
        capacity_tokens: u64,
    ) -> DistributedRateLimiter<dyn KeyValueStore> {
        let prefixed = format!("{}{key}", Self::RATE_LIMITER_PREFIX);
        let config = RateLimiterConfig::new(refill_rate, capacity_tokens);
        DistributedRateLimiter::new(Arc::clone(store), prefixed, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> CoordinationHandler {
        CoordinationHandler
    }

    // =========================================================================
    // can_handle: operations we implement natively → true
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
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterAcquire {
            key: "r".into(),
            tokens: 1,
            capacity_tokens: 10,
            refill_rate: 1.0,
            timeout_ms: 5000,
        }));
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterAvailable {
            key: "r".into(),
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
        assert!(h.can_handle(&ClientRpcRequest::RateLimiterReset {
            key: "r".into(),
            capacity_tokens: 10,
            refill_rate: 1.0,
        }));
    }

    // =========================================================================
    // can_handle: WASM-plugin-only operations → false (fall through to plugin)
    // =========================================================================

    #[test]
    fn test_cannot_handle_barrier_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::BarrierEnter {
            name: "b".into(),
            participant_id: "p".into(),
            required_count: 3,
            timeout_ms: 5000,
        }));
    }

    #[test]
    fn test_cannot_handle_semaphore_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::SemaphoreStatus { name: "s".into() }));
    }

    #[test]
    fn test_cannot_handle_rwlock_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::RWLockStatus { name: "rw".into() }));
    }

    #[test]
    fn test_cannot_handle_queue_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::QueueCreate {
            queue_name: "q".into(),
            default_visibility_timeout_ms: None,
            default_ttl_ms: None,
            max_delivery_attempts: None,
        }));
        assert!(!h.can_handle(&ClientRpcRequest::QueueStatus { queue_name: "q".into() }));
    }

    #[test]
    fn test_cannot_handle_service_registry_variants() {
        let h = handler();
        assert!(!h.can_handle(&ClientRpcRequest::ServiceDiscover {
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
