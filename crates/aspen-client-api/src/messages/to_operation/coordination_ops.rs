use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    to_operation_lock_counter(request)
        .or_else(|| to_operation_ratelimiter_barrier(request))
        .or_else(|| to_operation_semaphore_rwlock(request))
        .or_else(|| to_operation_queue(request))
        .or_else(|| to_operation_service(request))
}

fn to_operation_lock_counter(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::LockAcquire { key, .. }
        | ClientRpcRequest::LockTryAcquire { key, .. }
        | ClientRpcRequest::LockRelease { key, .. }
        | ClientRpcRequest::LockRenew { key, .. } => Some(Some(Operation::Write {
            key: format!("_lock:{key}"),
            value: vec![],
        })),

        ClientRpcRequest::CounterGet { key }
        | ClientRpcRequest::SignedCounterGet { key }
        | ClientRpcRequest::SequenceCurrent { key } => Some(Some(Operation::Read {
            key: format!("_counter:{key}"),
        })),

        ClientRpcRequest::CounterIncrement { key }
        | ClientRpcRequest::CounterDecrement { key }
        | ClientRpcRequest::CounterAdd { key, .. }
        | ClientRpcRequest::CounterSubtract { key, .. }
        | ClientRpcRequest::CounterSet { key, .. }
        | ClientRpcRequest::CounterCompareAndSet { key, .. }
        | ClientRpcRequest::SignedCounterAdd { key, .. }
        | ClientRpcRequest::SequenceNext { key }
        | ClientRpcRequest::SequenceReserve { key, .. } => Some(Some(Operation::Write {
            key: format!("_counter:{key}"),
            value: vec![],
        })),

        _ => None,
    }
}

fn to_operation_ratelimiter_barrier(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::RateLimiterTryAcquire { key, .. }
        | ClientRpcRequest::RateLimiterAcquire { key, .. }
        | ClientRpcRequest::RateLimiterReset { key, .. } => Some(Some(Operation::Write {
            key: format!("_ratelimit:{key}"),
            value: vec![],
        })),
        ClientRpcRequest::RateLimiterAvailable { key, .. } => Some(Some(Operation::Read {
            key: format!("_ratelimit:{key}"),
        })),

        ClientRpcRequest::BarrierEnter { name, .. } | ClientRpcRequest::BarrierLeave { name, .. } => {
            Some(Some(Operation::Write {
                key: format!("_barrier:{name}"),
                value: vec![],
            }))
        }
        ClientRpcRequest::BarrierStatus { name } => Some(Some(Operation::Read {
            key: format!("_barrier:{name}"),
        })),

        _ => None,
    }
}

fn to_operation_semaphore_rwlock(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::SemaphoreAcquire { name, .. }
        | ClientRpcRequest::SemaphoreTryAcquire { name, .. }
        | ClientRpcRequest::SemaphoreRelease { name, .. } => Some(Some(Operation::Write {
            key: format!("_semaphore:{name}"),
            value: vec![],
        })),
        ClientRpcRequest::SemaphoreStatus { name } => Some(Some(Operation::Read {
            key: format!("_semaphore:{name}"),
        })),

        ClientRpcRequest::RWLockAcquireRead { name, .. }
        | ClientRpcRequest::RWLockTryAcquireRead { name, .. }
        | ClientRpcRequest::RWLockAcquireWrite { name, .. }
        | ClientRpcRequest::RWLockTryAcquireWrite { name, .. }
        | ClientRpcRequest::RWLockReleaseRead { name, .. }
        | ClientRpcRequest::RWLockReleaseWrite { name, .. }
        | ClientRpcRequest::RWLockDowngrade { name, .. } => Some(Some(Operation::Write {
            key: format!("_rwlock:{name}"),
            value: vec![],
        })),
        ClientRpcRequest::RWLockStatus { name } => Some(Some(Operation::Read {
            key: format!("_rwlock:{name}"),
        })),

        _ => None,
    }
}

fn to_operation_queue(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::QueueCreate { queue_name, .. }
        | ClientRpcRequest::QueueDelete { queue_name }
        | ClientRpcRequest::QueueEnqueue { queue_name, .. }
        | ClientRpcRequest::QueueEnqueueBatch { queue_name, .. }
        | ClientRpcRequest::QueueDequeue { queue_name, .. }
        | ClientRpcRequest::QueueDequeueWait { queue_name, .. }
        | ClientRpcRequest::QueueAck { queue_name, .. }
        | ClientRpcRequest::QueueNack { queue_name, .. }
        | ClientRpcRequest::QueueExtendVisibility { queue_name, .. }
        | ClientRpcRequest::QueueRedriveDLQ { queue_name, .. } => Some(Some(Operation::Write {
            key: format!("_queue:{queue_name}"),
            value: vec![],
        })),
        ClientRpcRequest::QueuePeek { queue_name, .. }
        | ClientRpcRequest::QueueStatus { queue_name }
        | ClientRpcRequest::QueueGetDLQ { queue_name, .. } => Some(Some(Operation::Read {
            key: format!("_queue:{queue_name}"),
        })),

        _ => None,
    }
}

fn to_operation_service(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::ServiceRegister { service_name, .. }
        | ClientRpcRequest::ServiceDeregister { service_name, .. }
        | ClientRpcRequest::ServiceHeartbeat { service_name, .. }
        | ClientRpcRequest::ServiceUpdateHealth { service_name, .. }
        | ClientRpcRequest::ServiceUpdateMetadata { service_name, .. } => Some(Some(Operation::Write {
            key: format!("_service:{service_name}"),
            value: vec![],
        })),
        ClientRpcRequest::ServiceDiscover { service_name, .. }
        | ClientRpcRequest::ServiceGetInstance { service_name, .. } => Some(Some(Operation::Read {
            key: format!("_service:{service_name}"),
        })),
        ClientRpcRequest::ServiceList { prefix, .. } => Some(Some(Operation::Read {
            key: format!("_service:{prefix}"),
        })),

        _ => None,
    }
}
