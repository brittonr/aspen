//! Coordination primitives request handler.
//!
//! Handles: Lock operations, Counter operations, Sequence operations,
//! RateLimiter operations, Barrier operations, Semaphore operations,
//! RWLock operations, Queue operations.

mod dispatch;
mod handlers;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

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
            // Lock operations
            req @ (ClientRpcRequest::LockAcquire { .. }
            | ClientRpcRequest::LockTryAcquire { .. }
            | ClientRpcRequest::LockRelease { .. }
            | ClientRpcRequest::LockRenew { .. }) => dispatch::dispatch_lock(req, ctx).await,

            // Counter operations
            req @ (ClientRpcRequest::CounterGet { .. }
            | ClientRpcRequest::CounterIncrement { .. }
            | ClientRpcRequest::CounterDecrement { .. }
            | ClientRpcRequest::CounterAdd { .. }
            | ClientRpcRequest::CounterSubtract { .. }
            | ClientRpcRequest::CounterSet { .. }
            | ClientRpcRequest::CounterCompareAndSet { .. }
            | ClientRpcRequest::SignedCounterGet { .. }
            | ClientRpcRequest::SignedCounterAdd { .. }) => dispatch::dispatch_counter(req, ctx).await,

            // Sequence, rate limiter, barrier, semaphore operations
            req @ (ClientRpcRequest::SequenceNext { .. }
            | ClientRpcRequest::SequenceReserve { .. }
            | ClientRpcRequest::SequenceCurrent { .. }
            | ClientRpcRequest::RateLimiterTryAcquire { .. }
            | ClientRpcRequest::RateLimiterAcquire { .. }
            | ClientRpcRequest::RateLimiterAvailable { .. }
            | ClientRpcRequest::RateLimiterReset { .. }
            | ClientRpcRequest::BarrierEnter { .. }
            | ClientRpcRequest::BarrierLeave { .. }
            | ClientRpcRequest::BarrierStatus { .. }
            | ClientRpcRequest::SemaphoreAcquire { .. }
            | ClientRpcRequest::SemaphoreTryAcquire { .. }
            | ClientRpcRequest::SemaphoreRelease { .. }
            | ClientRpcRequest::SemaphoreStatus { .. }) => dispatch::dispatch_primitives(req, ctx).await,

            // RWLock operations
            req @ (ClientRpcRequest::RWLockAcquireRead { .. }
            | ClientRpcRequest::RWLockTryAcquireRead { .. }
            | ClientRpcRequest::RWLockAcquireWrite { .. }
            | ClientRpcRequest::RWLockTryAcquireWrite { .. }
            | ClientRpcRequest::RWLockReleaseRead { .. }
            | ClientRpcRequest::RWLockReleaseWrite { .. }
            | ClientRpcRequest::RWLockDowngrade { .. }
            | ClientRpcRequest::RWLockStatus { .. }) => dispatch::dispatch_rwlock(req, ctx).await,

            // Queue operations
            req @ (ClientRpcRequest::QueueCreate { .. }
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
            | ClientRpcRequest::QueueRedriveDLQ { .. }) => dispatch::dispatch_queue(req, ctx).await,

            _ => Err(anyhow::anyhow!("request not handled by CoordinationHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CoordinationHandler"
    }
}
