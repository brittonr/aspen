use aspen_auth::Operation;

use super::super::ClientRpcRequest;
use crate::coordination::LockSetMemberTokenWire;

fn canonical_lockset_member(members: &[String]) -> Option<&str> {
    members.iter().map(String::as_str).min()
}

fn canonical_lockset_token_member(member_tokens: &[LockSetMemberTokenWire]) -> Option<&str> {
    member_tokens.iter().map(|token| token.member.as_str()).min()
}

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
        ClientRpcRequest::LockSetAcquire { members, .. } | ClientRpcRequest::LockSetTryAcquire { members, .. } => {
            canonical_lockset_member(members).map(|member| {
                Some(Operation::Write {
                    key: format!("_lock:{member}"),
                    value: vec![],
                })
            })
        }
        ClientRpcRequest::LockSetRelease { member_tokens, .. }
        | ClientRpcRequest::LockSetRenew { member_tokens, .. } => {
            canonical_lockset_token_member(member_tokens).map(|member| {
                Some(Operation::Write {
                    key: format!("_lock:{member}"),
                    value: vec![],
                })
            })
        }

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

#[cfg(test)]
mod tests {
    use aspen_auth::Operation;

    use super::*;

    fn assert_canonical_lock_operation(operation_a: Operation, operation_b: Operation) {
        match (operation_a, operation_b) {
            (Operation::Write { key: key_a, .. }, Operation::Write { key: key_b, .. }) => {
                assert_eq!(key_a, "_lock:pipeline:42");
                assert_eq!(key_a, key_b);
            }
            _ => panic!("expected write operations"),
        }
    }

    #[test]
    fn test_client_rpc_lockset_requests_use_canonical_operation_member() {
        let request_a = ClientRpcRequest::LockSetTryAcquire {
            members: vec!["pipeline:42".to_string(), "repo:a".to_string()],
            holder_id: "holder-a".to_string(),
            ttl_ms: 1_000,
        };
        let request_b = ClientRpcRequest::LockSetTryAcquire {
            members: vec!["repo:a".to_string(), "pipeline:42".to_string()],
            holder_id: "holder-a".to_string(),
            ttl_ms: 1_000,
        };

        let operation_a = to_operation(&request_a).flatten().unwrap();
        let operation_b = to_operation(&request_b).flatten().unwrap();
        assert_canonical_lock_operation(operation_a, operation_b);
    }

    #[test]
    fn test_client_rpc_lockset_token_requests_use_canonical_operation_member() {
        let tokens_a = vec![
            LockSetMemberTokenWire {
                member: "repo:a".to_string(),
                fencing_token: 7,
            },
            LockSetMemberTokenWire {
                member: "pipeline:42".to_string(),
                fencing_token: 3,
            },
        ];
        let tokens_b = vec![
            LockSetMemberTokenWire {
                member: "pipeline:42".to_string(),
                fencing_token: 3,
            },
            LockSetMemberTokenWire {
                member: "repo:a".to_string(),
                fencing_token: 7,
            },
        ];
        let release_a = ClientRpcRequest::LockSetRelease {
            holder_id: "holder-a".to_string(),
            member_tokens: tokens_a.clone(),
        };
        let release_b = ClientRpcRequest::LockSetRelease {
            holder_id: "holder-a".to_string(),
            member_tokens: tokens_b.clone(),
        };
        let renew_a = ClientRpcRequest::LockSetRenew {
            holder_id: "holder-a".to_string(),
            member_tokens: tokens_a,
            ttl_ms: 1_000,
        };
        let renew_b = ClientRpcRequest::LockSetRenew {
            holder_id: "holder-a".to_string(),
            member_tokens: tokens_b,
            ttl_ms: 1_000,
        };

        assert_canonical_lock_operation(
            to_operation(&release_a).flatten().unwrap(),
            to_operation(&release_b).flatten().unwrap(),
        );
        assert_canonical_lock_operation(
            to_operation(&renew_a).flatten().unwrap(),
            to_operation(&renew_b).flatten().unwrap(),
        );
    }
}
