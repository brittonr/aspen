//! Coordination primitives request handler.
//!
//! Handles: Lock operations, Counter operations, Sequence operations,
//! RateLimiter operations, Barrier operations, Semaphore operations,
//! RWLock operations, Queue operations.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::api::validate_client_key;
use crate::api::ReadRequest;
use crate::api::WriteCommand;
use crate::api::WriteRequest;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;
use crate::client_rpc::CounterResultResponse;
use crate::client_rpc::LockResultResponse;
use crate::client_rpc::SequenceResultResponse;
use crate::client_rpc::SignedCounterResultResponse;
use crate::coordination::AtomicCounter;
use crate::coordination::CounterConfig;
use crate::coordination::DistributedLock;
use crate::coordination::LockConfig;
use crate::coordination::SequenceConfig;
use crate::coordination::SequenceGenerator;
use crate::coordination::SignedAtomicCounter;

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
            // Rate limiter handlers are placeholders - full implementation pending type alignment
            ClientRpcRequest::RateLimiterTryAcquire { .. }
            | ClientRpcRequest::RateLimiterAcquire { .. }
            | ClientRpcRequest::RateLimiterAvailable { .. }
            | ClientRpcRequest::RateLimiterReset { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "RateLimiter handler extraction in progress - operation handled by legacy path",
            )),

            // =====================================================================
            // Barrier Operations
            // =====================================================================
            // Barrier handlers are placeholders - full implementation pending type alignment
            ClientRpcRequest::BarrierEnter { .. }
            | ClientRpcRequest::BarrierLeave { .. }
            | ClientRpcRequest::BarrierStatus { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Barrier handler extraction in progress - operation handled by legacy path",
            )),

            // =====================================================================
            // Semaphore Operations
            // =====================================================================
            // Semaphore handlers are placeholders - full implementation pending type alignment
            ClientRpcRequest::SemaphoreAcquire { .. }
            | ClientRpcRequest::SemaphoreTryAcquire { .. }
            | ClientRpcRequest::SemaphoreRelease { .. }
            | ClientRpcRequest::SemaphoreStatus { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Semaphore handler extraction in progress - operation handled by legacy path",
            )),

            // =====================================================================
            // RWLock Operations
            // =====================================================================
            // RWLock handlers are placeholder - full implementation pending type alignment
            ClientRpcRequest::RWLockAcquireRead { .. }
            | ClientRpcRequest::RWLockTryAcquireRead { .. }
            | ClientRpcRequest::RWLockAcquireWrite { .. }
            | ClientRpcRequest::RWLockTryAcquireWrite { .. }
            | ClientRpcRequest::RWLockReleaseRead { .. }
            | ClientRpcRequest::RWLockReleaseWrite { .. }
            | ClientRpcRequest::RWLockDowngrade { .. }
            | ClientRpcRequest::RWLockStatus { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "RWLock handler extraction in progress - operation handled by legacy path",
            )),

            // =====================================================================
            // Queue Operations
            // =====================================================================
            // Queue handlers are placeholder - full implementation pending type alignment
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
            | ClientRpcRequest::QueueRedriveDLQ { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Queue handler extraction in progress - operation handled by legacy path",
            )),

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
            use crate::coordination::CoordinationError;
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
            use crate::coordination::CoordinationError;
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
    use crate::coordination::LockEntry;

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
        Err(crate::api::KeyValueStoreError::NotFound { .. }) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
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
    use crate::coordination::LockEntry;

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
// RateLimiter, Barrier, Semaphore, RWLock handlers are placeholders
// TODO: Extract these handlers once response type alignment is complete

// Queue handlers are placeholders - full implementation pending type alignment
// TODO: Extract Queue handler once response type alignment is complete
