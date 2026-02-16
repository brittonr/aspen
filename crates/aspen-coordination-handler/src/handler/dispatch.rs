//! Dispatch functions for coordination primitive operations.
//!
//! Each function handles a domain of coordination operations, destructuring
//! the request and delegating to the appropriate handler function.
//!
//! Tiger Style: Functions kept under 70 lines by separating dispatch from handling.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::counter::*;
use super::handlers::lock::*;
use super::handlers::primitives::*;
use super::handlers::queue::*;
use super::handlers::rwlock::*;
use super::normalize_timeout_ms;

/// Dispatch lock operations.
pub(super) async fn dispatch_lock(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
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

        _ => Err(anyhow::anyhow!("not a lock operation")),
    }
}

/// Dispatch counter operations (unsigned and signed).
pub(super) async fn dispatch_counter(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
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
        ClientRpcRequest::SignedCounterGet { key } => handle_signed_counter_get(ctx, key).await,
        ClientRpcRequest::SignedCounterAdd { key, amount } => handle_signed_counter_add(ctx, key, amount).await,
        _ => Err(anyhow::anyhow!("not a counter operation")),
    }
}

/// Dispatch sequence, rate limiter, barrier, and semaphore operations.
pub(super) async fn dispatch_primitives(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
        // Sequence operations
        ClientRpcRequest::SequenceNext { key } => handle_sequence_next(ctx, key).await,
        ClientRpcRequest::SequenceReserve { key, count } => handle_sequence_reserve(ctx, key, count).await,
        ClientRpcRequest::SequenceCurrent { key } => handle_sequence_current(ctx, key).await,

        // Rate limiter operations
        ClientRpcRequest::RateLimiterTryAcquire {
            key,
            tokens,
            capacity_tokens,
            refill_rate,
        } => handle_rate_limiter_try_acquire(ctx, key, tokens, capacity_tokens, refill_rate).await,

        ClientRpcRequest::RateLimiterAcquire {
            key,
            tokens,
            capacity_tokens,
            refill_rate,
            timeout_ms,
        } => handle_rate_limiter_acquire(ctx, key, tokens, capacity_tokens, refill_rate, timeout_ms).await,

        ClientRpcRequest::RateLimiterAvailable {
            key,
            capacity_tokens,
            refill_rate,
        } => handle_rate_limiter_available(ctx, key, capacity_tokens, refill_rate).await,

        ClientRpcRequest::RateLimiterReset {
            key,
            capacity_tokens,
            refill_rate,
        } => handle_rate_limiter_reset(ctx, key, capacity_tokens, refill_rate).await,

        // Barrier operations
        ClientRpcRequest::BarrierEnter {
            name,
            participant_id,
            required_count,
            timeout_ms,
        } => handle_barrier_enter(ctx, name, participant_id, required_count, normalize_timeout_ms(timeout_ms)).await,

        ClientRpcRequest::BarrierLeave {
            name,
            participant_id,
            timeout_ms,
        } => handle_barrier_leave(ctx, name, participant_id, normalize_timeout_ms(timeout_ms)).await,

        ClientRpcRequest::BarrierStatus { name } => handle_barrier_status(ctx, name).await,

        // Semaphore operations
        ClientRpcRequest::SemaphoreAcquire {
            name,
            holder_id,
            permits,
            capacity_permits,
            ttl_ms,
            timeout_ms,
        } => {
            handle_semaphore_acquire(
                ctx,
                name,
                holder_id,
                permits,
                capacity_permits,
                ttl_ms,
                normalize_timeout_ms(timeout_ms),
            )
            .await
        }

        ClientRpcRequest::SemaphoreTryAcquire {
            name,
            holder_id,
            permits,
            capacity_permits,
            ttl_ms,
        } => handle_semaphore_try_acquire(ctx, name, holder_id, permits, capacity_permits, ttl_ms).await,

        ClientRpcRequest::SemaphoreRelease {
            name,
            holder_id,
            permits,
        } => handle_semaphore_release(ctx, name, holder_id, permits).await,

        ClientRpcRequest::SemaphoreStatus { name } => handle_semaphore_status(ctx, name).await,

        _ => Err(anyhow::anyhow!("not a primitives operation")),
    }
}

/// Dispatch reader-writer lock operations.
pub(super) async fn dispatch_rwlock(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
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

        _ => Err(anyhow::anyhow!("not a rwlock operation")),
    }
}

/// Dispatch queue operations.
pub(super) async fn dispatch_queue(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
        ClientRpcRequest::QueueCreate {
            queue_name,
            default_visibility_timeout_ms,
            default_ttl_ms,
            max_delivery_attempts,
        } => {
            handle_queue_create(ctx, queue_name, default_visibility_timeout_ms, default_ttl_ms, max_delivery_attempts)
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
            handle_queue_dequeue_wait(ctx, queue_name, consumer_id, max_items, visibility_timeout_ms, wait_timeout_ms)
                .await
        }

        ClientRpcRequest::QueuePeek { queue_name, max_items } => handle_queue_peek(ctx, queue_name, max_items).await,

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

        _ => Err(anyhow::anyhow!("not a queue operation")),
    }
}
