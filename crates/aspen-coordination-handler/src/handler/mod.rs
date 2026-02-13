//! Coordination primitives request handler.
//!
//! Handles: Lock operations, Counter operations, Sequence operations,
//! RateLimiter operations, Barrier operations, Semaphore operations,
//! RWLock operations, Queue operations.

mod handlers;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use handlers::counter::*;
use handlers::lock::*;
use handlers::primitives::*;
use handlers::queue::*;
use handlers::rwlock::*;

/// Normalize a timeout value where 0 means no timeout.
#[inline]
const fn normalize_timeout_ms(timeout_ms: u64) -> Option<u64> {
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

/// Handler for coordination primitive operations.
pub struct CoordinationHandler;

#[async_trait::async_trait]
impl RequestHandler for CoordinationHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            // Lock operations
            ClientRpcRequest::LockAcquire { .. }
                | ClientRpcRequest::LockTryAcquire { .. }
                | ClientRpcRequest::LockRelease { .. }
                | ClientRpcRequest::LockRenew { .. }
                // Counter operations
                | ClientRpcRequest::CounterGet { .. }
                | ClientRpcRequest::CounterIncrement { .. }
                | ClientRpcRequest::CounterDecrement { .. }
                | ClientRpcRequest::CounterAdd { .. }
                | ClientRpcRequest::CounterSubtract { .. }
                | ClientRpcRequest::CounterSet { .. }
                | ClientRpcRequest::CounterCompareAndSet { .. }
                // Signed counter operations
                | ClientRpcRequest::SignedCounterGet { .. }
                | ClientRpcRequest::SignedCounterAdd { .. }
                // Sequence operations
                | ClientRpcRequest::SequenceNext { .. }
                | ClientRpcRequest::SequenceReserve { .. }
                | ClientRpcRequest::SequenceCurrent { .. }
                // Rate limiter operations
                | ClientRpcRequest::RateLimiterTryAcquire { .. }
                | ClientRpcRequest::RateLimiterAcquire { .. }
                | ClientRpcRequest::RateLimiterAvailable { .. }
                | ClientRpcRequest::RateLimiterReset { .. }
                // Barrier operations
                | ClientRpcRequest::BarrierEnter { .. }
                | ClientRpcRequest::BarrierLeave { .. }
                | ClientRpcRequest::BarrierStatus { .. }
                // Semaphore operations
                | ClientRpcRequest::SemaphoreAcquire { .. }
                | ClientRpcRequest::SemaphoreTryAcquire { .. }
                | ClientRpcRequest::SemaphoreRelease { .. }
                | ClientRpcRequest::SemaphoreStatus { .. }
                // RWLock operations
                | ClientRpcRequest::RWLockAcquireRead { .. }
                | ClientRpcRequest::RWLockTryAcquireRead { .. }
                | ClientRpcRequest::RWLockAcquireWrite { .. }
                | ClientRpcRequest::RWLockTryAcquireWrite { .. }
                | ClientRpcRequest::RWLockReleaseRead { .. }
                | ClientRpcRequest::RWLockReleaseWrite { .. }
                | ClientRpcRequest::RWLockDowngrade { .. }
                | ClientRpcRequest::RWLockStatus { .. }
                // Queue operations
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
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            // =====================================================================
            // Lock Operations
            // =====================================================================
            ClientRpcRequest::LockAcquire {
                key,
                holder_id,
                ttl_ms,
                timeout_ms,
            } => handle_lock_acquire(ctx, key, holder_id, ttl_ms, Some(timeout_ms)).await,

            ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } => {
                handle_lock_try_acquire(ctx, key, holder_id, ttl_ms).await
            }

            ClientRpcRequest::LockRelease {
                key,
                holder_id,
                fencing_token,
            } => handle_lock_release(ctx, key, holder_id, fencing_token).await,

            ClientRpcRequest::LockRenew {
                key,
                holder_id,
                fencing_token,
                ttl_ms,
            } => handle_lock_renew(ctx, key, holder_id, fencing_token, ttl_ms).await,

            // =====================================================================
            // Counter Operations
            // =====================================================================
            ClientRpcRequest::CounterGet { key } => handle_counter_get(ctx, key).await,
            ClientRpcRequest::CounterIncrement { key } => handle_counter_increment(ctx, key).await,
            ClientRpcRequest::CounterDecrement { key } => handle_counter_decrement(ctx, key).await,
            ClientRpcRequest::CounterAdd { key, amount } => handle_counter_add(ctx, key, amount).await,
            ClientRpcRequest::CounterSubtract { key, amount } => handle_counter_subtract(ctx, key, amount).await,
            ClientRpcRequest::CounterSet { key, value } => handle_counter_set(ctx, key, value).await,
            ClientRpcRequest::CounterCompareAndSet {
                key,
                expected,
                new_value,
            } => handle_counter_compare_and_set(ctx, key, expected, new_value).await,

            // =====================================================================
            // Signed Counter Operations
            // =====================================================================
            ClientRpcRequest::SignedCounterGet { key } => handle_signed_counter_get(ctx, key).await,
            ClientRpcRequest::SignedCounterAdd { key, amount } => handle_signed_counter_add(ctx, key, amount).await,

            // =====================================================================
            // Sequence Operations
            // =====================================================================
            ClientRpcRequest::SequenceNext { key } => handle_sequence_next(ctx, key).await,
            ClientRpcRequest::SequenceReserve { key, count } => handle_sequence_reserve(ctx, key, count).await,
            ClientRpcRequest::SequenceCurrent { key } => handle_sequence_current(ctx, key).await,

            // =====================================================================
            // Rate Limiter Operations
            // =====================================================================
            ClientRpcRequest::RateLimiterTryAcquire {
                key,
                tokens,
                capacity,
                refill_rate,
            } => handle_rate_limiter_try_acquire(ctx, key, tokens, capacity, refill_rate).await,

            ClientRpcRequest::RateLimiterAcquire {
                key,
                tokens,
                capacity,
                refill_rate,
                timeout_ms,
            } => handle_rate_limiter_acquire(ctx, key, tokens, capacity, refill_rate, timeout_ms).await,

            ClientRpcRequest::RateLimiterAvailable {
                key,
                capacity,
                refill_rate,
            } => handle_rate_limiter_available(ctx, key, capacity, refill_rate).await,

            ClientRpcRequest::RateLimiterReset {
                key,
                capacity,
                refill_rate,
            } => handle_rate_limiter_reset(ctx, key, capacity, refill_rate).await,

            // =====================================================================
            // Barrier Operations
            // =====================================================================
            ClientRpcRequest::BarrierEnter {
                name,
                participant_id,
                required_count,
                timeout_ms,
            } => {
                handle_barrier_enter(ctx, name, participant_id, required_count, normalize_timeout_ms(timeout_ms)).await
            }

            ClientRpcRequest::BarrierLeave {
                name,
                participant_id,
                timeout_ms,
            } => handle_barrier_leave(ctx, name, participant_id, normalize_timeout_ms(timeout_ms)).await,

            ClientRpcRequest::BarrierStatus { name } => handle_barrier_status(ctx, name).await,

            // =====================================================================
            // Semaphore Operations
            // =====================================================================
            ClientRpcRequest::SemaphoreAcquire {
                name,
                holder_id,
                permits,
                capacity,
                ttl_ms,
                timeout_ms,
            } => {
                handle_semaphore_acquire(
                    ctx,
                    name,
                    holder_id,
                    permits,
                    capacity,
                    ttl_ms,
                    normalize_timeout_ms(timeout_ms),
                )
                .await
            }

            ClientRpcRequest::SemaphoreTryAcquire {
                name,
                holder_id,
                permits,
                capacity,
                ttl_ms,
            } => handle_semaphore_try_acquire(ctx, name, holder_id, permits, capacity, ttl_ms).await,

            ClientRpcRequest::SemaphoreRelease {
                name,
                holder_id,
                permits,
            } => handle_semaphore_release(ctx, name, holder_id, permits).await,

            ClientRpcRequest::SemaphoreStatus { name } => handle_semaphore_status(ctx, name).await,

            // =====================================================================
            // RWLock Operations
            // =====================================================================
            ClientRpcRequest::RWLockAcquireRead {
                name,
                holder_id,
                ttl_ms,
                timeout_ms,
            } => handle_rwlock_acquire_read(ctx, name, holder_id, ttl_ms, normalize_timeout_ms(timeout_ms)).await,

            ClientRpcRequest::RWLockTryAcquireRead {
                name,
                holder_id,
                ttl_ms,
            } => handle_rwlock_try_acquire_read(ctx, name, holder_id, ttl_ms).await,

            ClientRpcRequest::RWLockAcquireWrite {
                name,
                holder_id,
                ttl_ms,
                timeout_ms,
            } => handle_rwlock_acquire_write(ctx, name, holder_id, ttl_ms, normalize_timeout_ms(timeout_ms)).await,

            ClientRpcRequest::RWLockTryAcquireWrite {
                name,
                holder_id,
                ttl_ms,
            } => handle_rwlock_try_acquire_write(ctx, name, holder_id, ttl_ms).await,

            ClientRpcRequest::RWLockReleaseRead { name, holder_id } => {
                handle_rwlock_release_read(ctx, name, holder_id).await
            }

            ClientRpcRequest::RWLockReleaseWrite {
                name,
                holder_id,
                fencing_token,
            } => handle_rwlock_release_write(ctx, name, holder_id, fencing_token).await,

            ClientRpcRequest::RWLockDowngrade {
                name,
                holder_id,
                fencing_token,
                ttl_ms,
            } => handle_rwlock_downgrade(ctx, name, holder_id, fencing_token, ttl_ms).await,

            ClientRpcRequest::RWLockStatus { name } => handle_rwlock_status(ctx, name).await,

            // =====================================================================
            // Queue Operations
            // =====================================================================
            ClientRpcRequest::QueueCreate {
                queue_name,
                default_visibility_timeout_ms,
                default_ttl_ms,
                max_delivery_attempts,
            } => {
                handle_queue_create(
                    ctx,
                    queue_name,
                    default_visibility_timeout_ms,
                    default_ttl_ms,
                    max_delivery_attempts,
                )
                .await
            }

            ClientRpcRequest::QueueDelete { queue_name } => handle_queue_delete(ctx, queue_name).await,

            ClientRpcRequest::QueueEnqueue {
                queue_name,
                payload,
                ttl_ms,
                message_group_id,
                deduplication_id,
            } => handle_queue_enqueue(ctx, queue_name, payload, ttl_ms, message_group_id, deduplication_id).await,

            ClientRpcRequest::QueueEnqueueBatch { queue_name, items } => {
                handle_queue_enqueue_batch(ctx, queue_name, items).await
            }

            ClientRpcRequest::QueueDequeue {
                queue_name,
                consumer_id,
                max_items,
                visibility_timeout_ms,
            } => handle_queue_dequeue(ctx, queue_name, consumer_id, max_items, visibility_timeout_ms).await,

            ClientRpcRequest::QueueDequeueWait {
                queue_name,
                consumer_id,
                max_items,
                visibility_timeout_ms,
                wait_timeout_ms,
            } => {
                handle_queue_dequeue_wait(
                    ctx,
                    queue_name,
                    consumer_id,
                    max_items,
                    visibility_timeout_ms,
                    wait_timeout_ms,
                )
                .await
            }

            ClientRpcRequest::QueuePeek { queue_name, max_items } => {
                handle_queue_peek(ctx, queue_name, max_items).await
            }

            ClientRpcRequest::QueueAck {
                queue_name,
                receipt_handle,
            } => handle_queue_ack(ctx, queue_name, receipt_handle).await,

            ClientRpcRequest::QueueNack {
                queue_name,
                receipt_handle,
                move_to_dlq,
                error_message,
            } => handle_queue_nack(ctx, queue_name, receipt_handle, move_to_dlq, error_message).await,

            ClientRpcRequest::QueueExtendVisibility {
                queue_name,
                receipt_handle,
                additional_timeout_ms,
            } => handle_queue_extend_visibility(ctx, queue_name, receipt_handle, additional_timeout_ms).await,

            ClientRpcRequest::QueueStatus { queue_name } => handle_queue_status(ctx, queue_name).await,

            ClientRpcRequest::QueueGetDLQ { queue_name, max_items } => {
                handle_queue_get_dlq(ctx, queue_name, max_items).await
            }

            ClientRpcRequest::QueueRedriveDLQ { queue_name, item_id } => {
                handle_queue_redrive_dlq(ctx, queue_name, item_id).await
            }

            _ => Err(anyhow::anyhow!("request not handled by CoordinationHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CoordinationHandler"
    }
}
