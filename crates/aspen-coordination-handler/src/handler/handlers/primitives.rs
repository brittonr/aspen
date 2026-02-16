//! Handlers for Sequence, RateLimiter, Barrier, and Semaphore primitives.

use aspen_client_api::BarrierResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::RateLimiterResultResponse;
use aspen_client_api::SemaphoreResultResponse;
use aspen_client_api::SequenceResultResponse;
use aspen_coordination::BarrierManager;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SemaphoreManager;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

// ============================================================================
// Sequence Operation Handlers
// ============================================================================

pub(crate) async fn handle_sequence_next(
    ctx: &ClientProtocolContext,
    key: String,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            is_success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
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

pub(crate) async fn handle_sequence_reserve(
    ctx: &ClientProtocolContext,
    key: String,
    count: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            is_success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
    match seq.reserve(count).await {
        // Reserve returns the start of the reserved range
        // The client can compute end as start + count - 1
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

pub(crate) async fn handle_sequence_current(
    ctx: &ClientProtocolContext,
    key: String,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
            is_success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
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

// ============================================================================
// Rate Limiter Operation Handlers
// ============================================================================

pub(crate) async fn handle_rate_limiter_try_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    tokens: u64,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

    match limiter.try_acquire_n(tokens).await {
        Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: true,
            tokens_remaining: Some(remaining),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: None,
            retry_after_ms: e.retry_after_ms(),
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rate_limiter_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    tokens: u64,
    capacity: u64,
    refill_rate: f64,
    timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
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
            is_success: true,
            tokens_remaining: Some(remaining),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: None,
            retry_after_ms: e.retry_after_ms(),
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rate_limiter_available(
    ctx: &ClientProtocolContext,
    key: String,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

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

pub(crate) async fn handle_rate_limiter_reset(
    ctx: &ClientProtocolContext,
    key: String,
    capacity: u64,
    refill_rate: f64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = RateLimiterConfig::new(refill_rate, capacity);
    let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);

    match limiter.reset().await {
        Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            is_success: true,
            tokens_remaining: Some(capacity),
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

// ============================================================================
// Barrier Operation Handlers
// ============================================================================

pub(crate) async fn handle_barrier_enter(
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
            is_success: true,
            current_count: Some(count),
            required_count: Some(required_count),
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
            is_success: false,
            current_count: None,
            required_count: Some(required_count),
            phase: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_barrier_leave(
    ctx: &ClientProtocolContext,
    name: String,
    participant_id: String,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = BarrierManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.leave(&name, &participant_id, timeout).await {
        Ok((remaining, phase)) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
            is_success: true,
            current_count: Some(remaining),
            required_count: None,
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
            is_success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_barrier_status(
    ctx: &ClientProtocolContext,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = BarrierManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((current, required, phase)) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            is_success: true,
            current_count: Some(current),
            required_count: Some(required),
            phase: Some(phase),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            is_success: false,
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

pub(crate) async fn handle_semaphore_acquire(
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
            is_success: true,
            permits_acquired: Some(acquired),
            available: Some(available),
            capacity: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: Some(100),
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_semaphore_try_acquire(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    permits: u32,
    capacity: u32,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.try_acquire(&name, &holder_id, permits, capacity, ttl_ms).await {
        Ok(Some((acquired, available))) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
            is_success: true,
            permits_acquired: Some(acquired),
            available: Some(available),
            capacity: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: Some(100),
            error: Some("no permits available".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity: Some(capacity),
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_semaphore_release(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    permits: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.release(&name, &holder_id, permits).await {
        Ok(available) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
            is_success: true,
            permits_acquired: None,
            available: Some(available),
            capacity: None,
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_semaphore_status(
    ctx: &ClientProtocolContext,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = SemaphoreManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((available, capacity)) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
            is_success: true,
            permits_acquired: None,
            available: Some(available),
            capacity: Some(capacity),
            retry_after_ms: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity: None,
            retry_after_ms: None,
            error: Some(e.to_string()),
        })),
    }
}
