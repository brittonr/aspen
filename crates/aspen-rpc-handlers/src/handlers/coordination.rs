//! Coordination primitives request handler.
//!
//! Handles: Lock operations, Counter operations, Sequence operations,
//! RateLimiter operations, Barrier operations, Semaphore operations,
//! RWLock operations, Queue operations.

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_core::validate_client_key;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_client_rpc::BarrierResultResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::CounterResultResponse;
use aspen_client_rpc::LockResultResponse;
use aspen_client_rpc::QueueAckResultResponse;
use aspen_client_rpc::QueueCreateResultResponse;
use aspen_client_rpc::QueueDeleteResultResponse;
use aspen_client_rpc::QueueDequeueResultResponse;
use aspen_client_rpc::QueueDequeuedItemResponse;
use aspen_client_rpc::QueueEnqueueBatchResultResponse;
use aspen_client_rpc::QueueEnqueueResultResponse;
use aspen_client_rpc::QueueExtendVisibilityResultResponse;
use aspen_client_rpc::QueueGetDLQResultResponse;
use aspen_client_rpc::QueueDLQItemResponse;
use aspen_client_rpc::QueueItemResponse;
use aspen_client_rpc::QueueNackResultResponse;
use aspen_client_rpc::QueuePeekResultResponse;
use aspen_client_rpc::QueueRedriveDLQResultResponse;
use aspen_client_rpc::QueueStatusResultResponse;
use aspen_client_rpc::RWLockResultResponse;
use aspen_client_rpc::RateLimiterResultResponse;
use aspen_client_rpc::SemaphoreResultResponse;
use aspen_client_rpc::SequenceResultResponse;
use aspen_client_rpc::SignedCounterResultResponse;
use aspen_coordination::EnqueueOptions;
use aspen_coordination::QueueConfig;
use aspen_coordination::AtomicCounter;
use aspen_coordination::BarrierManager;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedLock;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::LockConfig;
use aspen_coordination::QueueManager;
use aspen_coordination::RWLockManager;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SemaphoreManager;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_coordination::SignedAtomicCounter;

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
            ClientRpcRequest::CounterCompareAndSet { key, expected, new_value } => {
                handle_counter_compare_and_set(ctx, key, expected, new_value).await
            }

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
                let timeout = if timeout_ms == 0 { None } else { Some(timeout_ms) };
                handle_barrier_enter(ctx, name, participant_id, required_count, timeout).await
            }

            ClientRpcRequest::BarrierLeave {
                name,
                participant_id,
                timeout_ms,
            } => {
                let timeout = if timeout_ms == 0 { None } else { Some(timeout_ms) };
                handle_barrier_leave(ctx, name, participant_id, timeout).await
            }

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
                let timeout = if timeout_ms == 0 { None } else { Some(timeout_ms) };
                handle_semaphore_acquire(ctx, name, holder_id, permits, capacity, ttl_ms, timeout).await
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
            } => {
                let timeout = if timeout_ms == 0 { None } else { Some(timeout_ms) };
                handle_rwlock_acquire_read(ctx, name, holder_id, ttl_ms, timeout).await
            }

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
            } => {
                let timeout = if timeout_ms == 0 { None } else { Some(timeout_ms) };
                handle_rwlock_acquire_write(ctx, name, holder_id, ttl_ms, timeout).await
            }

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

            ClientRpcRequest::QueuePeek {
                queue_name,
                max_items,
            } => handle_queue_peek(ctx, queue_name, max_items).await,

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

            ClientRpcRequest::QueueGetDLQ {
                queue_name,
                max_items,
            } => handle_queue_get_dlq(ctx, queue_name, max_items).await,

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

// ============================================================================
// Lock Operation Handlers
// ============================================================================

async fn handle_lock_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = LockConfig {
        ttl_ms,
        acquire_timeout_ms: timeout_ms.unwrap_or(0), // 0 = no timeout
        ..Default::default()
    };
    let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

    match lock.acquire().await {
        Ok(guard) => {
            let token = guard.fencing_token().value();
            let deadline = guard.deadline_ms();
            std::mem::forget(guard);
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: true,
                fencing_token: Some(token),
                holder_id: Some(holder_id),
                deadline_ms: Some(deadline),
                error: None,
            }))
        }
        Err(e) => {
            use aspen_coordination::CoordinationError;
            let (holder, deadline) = match &e {
                CoordinationError::LockHeld { holder, deadline_ms } => (Some(holder.clone()), Some(*deadline_ms)),
                _ => (None, None),
            };
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: false,
                fencing_token: None,
                holder_id: holder,
                deadline_ms: deadline,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_lock_try_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = LockConfig {
        ttl_ms,
        ..Default::default()
    };
    let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

    match lock.try_acquire().await {
        Ok(guard) => {
            let token = guard.fencing_token().value();
            let deadline = guard.deadline_ms();
            std::mem::forget(guard);
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: true,
                fencing_token: Some(token),
                holder_id: Some(holder_id),
                deadline_ms: Some(deadline),
                error: None,
            }))
        }
        Err(e) => {
            use aspen_coordination::CoordinationError;
            let (holder, deadline) = match &e {
                CoordinationError::LockHeld { holder, deadline_ms } => (Some(holder.clone()), Some(*deadline_ms)),
                _ => (None, None),
            };
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: false,
                fencing_token: None,
                holder_id: holder,
                deadline_ms: deadline,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_lock_release(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_coordination::LockEntry;

    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

    match read_result {
        Ok(result) => {
            let value = result.kv.map(|kv| kv.value).unwrap_or_default();
            match serde_json::from_str::<LockEntry>(&value) {
                Ok(entry) => {
                    if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
                        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: Some(entry.fencing_token),
                            holder_id: Some(entry.holder_id),
                            deadline_ms: Some(entry.deadline_ms),
                            error: Some("lock not held by this holder".to_string()),
                        }));
                    }

                    let released = entry.released();
                    let released_json = serde_json::to_string(&released)?;

                    match ctx
                        .kv_store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(value),
                                new_value: released_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: true,
                            fencing_token: Some(fencing_token),
                            holder_id: Some(holder_id),
                            deadline_ms: None,
                            error: None,
                        })),
                        Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: None,
                            holder_id: None,
                            deadline_ms: None,
                            error: Some(format!("release failed: {}", e)),
                        })),
                    }
                }
                Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some("invalid lock entry format".to_string()),
                })),
            }
        }
        Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: true,
            fencing_token: Some(fencing_token),
            holder_id: Some(holder_id),
            deadline_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(format!("read failed: {}", e)),
        })),
    }
}

async fn handle_lock_renew(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    fencing_token: u64,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_coordination::LockEntry;

    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

    match read_result {
        Ok(result) => {
            let value = result.kv.map(|kv| kv.value).unwrap_or_default();
            match serde_json::from_str::<LockEntry>(&value) {
                Ok(entry) => {
                    if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
                        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: Some(entry.fencing_token),
                            holder_id: Some(entry.holder_id),
                            deadline_ms: Some(entry.deadline_ms),
                            error: Some("lock not held by this holder".to_string()),
                        }));
                    }

                    let renewed = LockEntry::new(holder_id.clone(), fencing_token, ttl_ms);
                    let renewed_json = serde_json::to_string(&renewed)?;

                    match ctx
                        .kv_store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(value),
                                new_value: renewed_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: true,
                            fencing_token: Some(fencing_token),
                            holder_id: Some(holder_id),
                            deadline_ms: Some(renewed.deadline_ms),
                            error: None,
                        })),
                        Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: None,
                            holder_id: None,
                            deadline_ms: None,
                            error: Some(format!("renew failed: {}", e)),
                        })),
                    }
                }
                Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some("invalid lock entry format".to_string()),
                })),
            }
        }
        Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(format!("read failed: {}", e)),
        })),
    }
}

// ============================================================================
// Counter Operation Handlers
// ============================================================================

async fn handle_counter_get(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.get().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_increment(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.increment().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_decrement(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.decrement().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_add(ctx: &ClientProtocolContext, key: String, amount: u64) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.add(amount).await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_subtract(
    ctx: &ClientProtocolContext,
    key: String,
    amount: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.subtract(amount).await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_set(ctx: &ClientProtocolContext, key: String, value: u64) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.set(value).await {
        Ok(()) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_counter_compare_and_set(
    ctx: &ClientProtocolContext,
    key: String,
    expected: u64,
    new_value: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.compare_and_set(expected, new_value).await {
        Ok(true) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(new_value),
            error: None,
        })),
        Ok(false) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some("compare-and-set condition not met".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Signed Counter Operation Handlers
// ============================================================================

async fn handle_signed_counter_get(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.get().await {
        Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_signed_counter_add(
    ctx: &ClientProtocolContext,
    key: String,
    amount: i64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.add(amount).await {
        Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Sequence Operation Handlers
// ============================================================================

async fn handle_sequence_next(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
    match seq.next().await {
        Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_sequence_reserve(
    ctx: &ClientProtocolContext,
    key: String,
    count: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
    match seq.reserve(count).await {
        // Reserve returns the start of the reserved range
        // The client can compute end as start + count - 1
        Ok(start) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: true,
            value: Some(start),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_sequence_current(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
    match seq.current().await {
        Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Rate Limiter Operation Handlers
// ============================================================================

async fn handle_rate_limiter_try_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    tokens: u64,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

    match limiter.try_acquire_n(tokens).await {
        Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: true,
            tokens_remaining: Some(remaining),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: e.retry_after_ms(),
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rate_limiter_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    tokens: u64,
    capacity: u64,
    refill_rate: f64,
    timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
    let timeout = std::time::Duration::from_millis(timeout_ms);

    match limiter.acquire_n(tokens, timeout).await {
        Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: true,
            tokens_remaining: Some(remaining),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: e.retry_after_ms(),
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rate_limiter_available(
    ctx: &ClientProtocolContext,
    key: String,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

    match limiter.available().await {
        Ok(available) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: true,
            tokens_remaining: Some(available),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rate_limiter_reset(
    ctx: &ClientProtocolContext,
    key: String,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

    match limiter.reset().await {
        Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: true,
            tokens_remaining: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Barrier Operation Handlers
// ============================================================================

async fn handle_barrier_enter(
    ctx: &ClientProtocolContext,
    name: String,
    participant_id: String,
    required_count: u32,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = BarrierManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.enter(&name, &participant_id, required_count, timeout).await {
        Ok((count, phase)) => Ok(ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
            success: true,
            current_count: Some(count),
            required_count: Some(required_count),
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
            success: false,
            current_count: None,
            required_count: Some(required_count),
            phase: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_barrier_leave(
    ctx: &ClientProtocolContext,
    name: String,
    participant_id: String,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = BarrierManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.leave(&name, &participant_id, timeout).await {
        Ok((remaining, phase)) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
            success: true,
            current_count: Some(remaining),
            required_count: None,
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
            success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_barrier_status(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    let manager = BarrierManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((current, required, phase)) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            success: true,
            current_count: Some(current),
            required_count: Some(required),
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Semaphore Operation Handlers
// ============================================================================

async fn handle_semaphore_acquire(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    permits: u32,
    capacity: u32,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.acquire(&name, &holder_id, permits, capacity, ttl_ms, timeout).await {
        Ok((acquired, available)) => Ok(ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
            success: true,
            permits_acquired: Some(acquired),
            available: Some(available),
            capacity: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
            success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: Some(100),
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_semaphore_try_acquire(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    permits: u32,
    capacity: u32,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.try_acquire(&name, &holder_id, permits, capacity, ttl_ms).await {
        Ok(Some((acquired, available))) => {
            Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                success: true,
                permits_acquired: Some(acquired),
                available: Some(available),
                capacity: Some(capacity),
                retry_after_ms: None,
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
            success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: Some(100),
            error: Some("no permits available".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
            success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_semaphore_release(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    permits: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.release(&name, &holder_id, permits).await {
        Ok(available) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
            success: true,
            permits_acquired: None,
            available: Some(available),
            capacity: None,
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
            success: false,
            permits_acquired: None,
            available: None,
            capacity: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_semaphore_status(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((available, capacity)) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
            success: true,
            permits_acquired: None,
            available: Some(available),
            capacity: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
            success: false,
            permits_acquired: None,
            available: None,
            capacity: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// RWLock Operation Handlers
// ============================================================================

async fn handle_rwlock_acquire_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.acquire_read(&name, &holder_id, ttl_ms, timeout).await {
        Ok((token, deadline, count)) => Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
            success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(count),
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_try_acquire_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.try_acquire_read(&name, &holder_id, ttl_ms).await {
        Ok(Some((token, deadline, count))) => {
            Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                success: true,
                mode: Some("read".to_string()),
                fencing_token: Some(token),
                deadline_ms: Some(deadline),
                reader_count: Some(count),
                writer_holder: None,
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some("lock unavailable".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_acquire_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.acquire_write(&name, &holder_id, ttl_ms, timeout).await {
        Ok((token, deadline)) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("write".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(0),
            writer_holder: Some(holder_id),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_try_acquire_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.try_acquire_write(&name, &holder_id, ttl_ms).await {
        Ok(Some((token, deadline))) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("write".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(0),
            writer_holder: Some(holder_id),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some("lock unavailable".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_release_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.release_read(&name, &holder_id).await {
        Ok(()) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
            success: true,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_release_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.release_write(&name, &holder_id, fencing_token).await {
        Ok(()) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("free".to_string()),
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_downgrade(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    fencing_token: u64,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.downgrade(&name, &holder_id, fencing_token, ttl_ms).await {
        Ok((token, deadline, count)) => Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
            success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(count),
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_rwlock_status(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((mode, reader_count, writer_holder, token)) => {
            Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                success: true,
                mode: Some(mode),
                fencing_token: Some(token),
                deadline_ms: None,
                reader_count: Some(reader_count),
                writer_holder,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Queue Operation Handlers
// ============================================================================

async fn handle_queue_create(
    ctx: &ClientProtocolContext,
    queue_name: String,
    default_visibility_timeout_ms: Option<u64>,
    default_ttl_ms: Option<u64>,
    max_delivery_attempts: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());
    let config = QueueConfig {
        default_visibility_timeout_ms,
        default_ttl_ms,
        max_delivery_attempts,
    };

    match manager.create(&queue_name, config).await {
        Ok((created, _)) => Ok(ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            success: true,
            created,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            success: false,
            created: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_delete(ctx: &ClientProtocolContext, queue_name: String) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.delete(&queue_name).await {
        Ok(deleted) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
            success: true,
            items_deleted: Some(deleted),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
            success: false,
            items_deleted: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_enqueue(
    ctx: &ClientProtocolContext,
    queue_name: String,
    payload: Vec<u8>,
    ttl_ms: Option<u64>,
    message_group_id: Option<String>,
    deduplication_id: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());
    let options = EnqueueOptions {
        ttl_ms,
        message_group_id,
        deduplication_id,
    };

    match manager.enqueue(&queue_name, payload, options).await {
        Ok(item_id) => Ok(ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
            success: true,
            item_id: Some(item_id),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
            success: false,
            item_id: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_enqueue_batch(
    ctx: &ClientProtocolContext,
    queue_name: String,
    items: Vec<aspen_client_rpc::QueueEnqueueItem>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    let batch: Vec<(Vec<u8>, EnqueueOptions)> = items
        .into_iter()
        .map(|item| {
            (
                item.payload,
                EnqueueOptions {
                    ttl_ms: item.ttl_ms,
                    message_group_id: item.message_group_id,
                    deduplication_id: item.deduplication_id,
                },
            )
        })
        .collect();

    match manager.enqueue_batch(&queue_name, batch).await {
        Ok(item_ids) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
            success: true,
            item_ids,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
            success: false,
            item_ids: vec![],
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_dequeue(
    ctx: &ClientProtocolContext,
    queue_name: String,
    consumer_id: String,
    max_items: u32,
    visibility_timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.dequeue(&queue_name, &consumer_id, max_items, visibility_timeout_ms).await {
        Ok(items) => {
            let response_items: Vec<QueueDequeuedItemResponse> = items
                .into_iter()
                .map(|item| QueueDequeuedItemResponse {
                    item_id: item.item_id,
                    payload: item.payload,
                    receipt_handle: item.receipt_handle,
                    delivery_attempts: item.delivery_attempts,
                    enqueued_at_ms: item.enqueued_at_ms,
                    visibility_deadline_ms: item.visibility_deadline_ms,
                })
                .collect();
            Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
            success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_dequeue_wait(
    ctx: &ClientProtocolContext,
    queue_name: String,
    consumer_id: String,
    max_items: u32,
    visibility_timeout_ms: u64,
    wait_timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager
        .dequeue_wait(&queue_name, &consumer_id, max_items, visibility_timeout_ms, wait_timeout_ms)
        .await
    {
        Ok(items) => {
            let response_items: Vec<QueueDequeuedItemResponse> = items
                .into_iter()
                .map(|item| QueueDequeuedItemResponse {
                    item_id: item.item_id,
                    payload: item.payload,
                    receipt_handle: item.receipt_handle,
                    delivery_attempts: item.delivery_attempts,
                    enqueued_at_ms: item.enqueued_at_ms,
                    visibility_deadline_ms: item.visibility_deadline_ms,
                })
                .collect();
            Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
            success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_peek(
    ctx: &ClientProtocolContext,
    queue_name: String,
    max_items: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.peek(&queue_name, max_items).await {
        Ok(items) => {
            let response_items: Vec<QueueItemResponse> = items
                .into_iter()
                .map(|item| QueueItemResponse {
                    item_id: item.item_id,
                    payload: item.payload,
                    enqueued_at_ms: item.enqueued_at_ms,
                    expires_at_ms: item.expires_at_ms,
                    delivery_attempts: item.delivery_attempts,
                })
                .collect();
            Ok(ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
                success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
            success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_ack(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.ack(&queue_name, &receipt_handle).await {
        Ok(()) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_nack(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
    move_to_dlq: bool,
    error_message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.nack(&queue_name, &receipt_handle, move_to_dlq, error_message).await {
        Ok(()) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_extend_visibility(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
    additional_timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.extend_visibility(&queue_name, &receipt_handle, additional_timeout_ms).await {
        Ok(new_deadline) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(
            QueueExtendVisibilityResultResponse {
                success: true,
                new_deadline_ms: Some(new_deadline),
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(
            QueueExtendVisibilityResultResponse {
                success: false,
                new_deadline_ms: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_queue_status(ctx: &ClientProtocolContext, queue_name: String) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.status(&queue_name).await {
        Ok(status) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
            success: true,
            exists: status.exists,
            visible_count: Some(status.visible_count),
            pending_count: Some(status.pending_count),
            dlq_count: Some(status.dlq_count),
            total_enqueued: Some(status.total_enqueued),
            total_acked: Some(status.total_acked),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
            success: false,
            exists: false,
            visible_count: None,
            pending_count: None,
            dlq_count: None,
            total_enqueued: None,
            total_acked: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_get_dlq(
    ctx: &ClientProtocolContext,
    queue_name: String,
    max_items: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.get_dlq(&queue_name, max_items).await {
        Ok(items) => {
            let response_items: Vec<QueueDLQItemResponse> = items
                .into_iter()
                .map(|item| {
                    let reason = match item.reason {
                        aspen_coordination::DLQReason::MaxDeliveryAttemptsExceeded => "max_delivery_attempts",
                        aspen_coordination::DLQReason::ExplicitlyRejected => "explicitly_rejected",
                        aspen_coordination::DLQReason::ExpiredWhilePending => "expired_while_pending",
                    };
                    QueueDLQItemResponse {
                        item_id: item.item_id,
                        payload: item.payload,
                        enqueued_at_ms: item.enqueued_at_ms,
                        delivery_attempts: item.delivery_attempts,
                        reason: reason.to_string(),
                        moved_at_ms: item.moved_at_ms,
                        last_error: item.last_error,
                    }
                })
                .collect();
            Ok(ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
                success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
            success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_queue_redrive_dlq(
    ctx: &ClientProtocolContext,
    queue_name: String,
    item_id: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.redrive_dlq(&queue_name, item_id).await {
        Ok(()) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}
