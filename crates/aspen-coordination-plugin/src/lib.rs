//! WASM guest plugin for distributed coordination primitives.
//!
//! Handles locks, counters, sequences, rate limiters, barriers,
//! semaphores, RW locks, and queues — all implemented as KV state machines
//! using host-provided CAS operations.
//!
//! All state is stored under the `__coord:` KV prefix.

mod barrier;
mod counter;
mod kv;
mod lock;
mod queue;
mod ratelimit;
mod rwlock;
mod semaphore;
mod sequence;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct CoordinationPlugin;

impl AspenPlugin for CoordinationPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "coordination".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "LockAcquire".to_string(),
                "LockTryAcquire".to_string(),
                "LockRelease".to_string(),
                "LockRenew".to_string(),
                "CounterGet".to_string(),
                "CounterIncrement".to_string(),
                "CounterDecrement".to_string(),
                "CounterAdd".to_string(),
                "CounterSubtract".to_string(),
                "CounterSet".to_string(),
                "CounterCompareAndSet".to_string(),
                "SignedCounterGet".to_string(),
                "SignedCounterAdd".to_string(),
                "SequenceNext".to_string(),
                "SequenceReserve".to_string(),
                "SequenceCurrent".to_string(),
                "RateLimiterTryAcquire".to_string(),
                "RateLimiterAcquire".to_string(),
                "RateLimiterAvailable".to_string(),
                "RateLimiterReset".to_string(),
                "BarrierEnter".to_string(),
                "BarrierLeave".to_string(),
                "BarrierStatus".to_string(),
                "SemaphoreAcquire".to_string(),
                "SemaphoreTryAcquire".to_string(),
                "SemaphoreRelease".to_string(),
                "SemaphoreStatus".to_string(),
                "RWLockAcquireRead".to_string(),
                "RWLockTryAcquireRead".to_string(),
                "RWLockAcquireWrite".to_string(),
                "RWLockTryAcquireWrite".to_string(),
                "RWLockReleaseRead".to_string(),
                "RWLockReleaseWrite".to_string(),
                "RWLockDowngrade".to_string(),
                "RWLockStatus".to_string(),
                "QueueCreate".to_string(),
                "QueueDelete".to_string(),
                "QueueEnqueue".to_string(),
                "QueueEnqueueBatch".to_string(),
                "QueueDequeue".to_string(),
                "QueueDequeueWait".to_string(),
                "QueuePeek".to_string(),
                "QueueAck".to_string(),
                "QueueNack".to_string(),
                "QueueExtendVisibility".to_string(),
                "QueueStatus".to_string(),
                "QueueGetDLQ".to_string(),
                "QueueRedriveDLQ".to_string(),
            ],
            priority: 930,
            app_id: Some("coordination".to_string()),
            kv_prefixes: vec!["__coord:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                randomness: true,
                timers: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            // === Lock ===
            ClientRpcRequest::LockAcquire {
                key,
                holder_id,
                ttl_ms,
                timeout_ms,
            } => lock::handle_acquire(key, holder_id, ttl_ms, Some(timeout_ms)),

            ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } => {
                lock::handle_try_acquire(key, holder_id, ttl_ms)
            }

            ClientRpcRequest::LockRelease {
                key,
                holder_id,
                fencing_token,
            } => lock::handle_release(key, holder_id, fencing_token),

            ClientRpcRequest::LockRenew {
                key,
                holder_id,
                fencing_token,
                ttl_ms,
            } => lock::handle_renew(key, holder_id, ttl_ms, fencing_token),

            // === Unsigned counters ===
            ClientRpcRequest::CounterGet { key } => counter::handle_get(key),
            ClientRpcRequest::CounterIncrement { key } => counter::handle_increment(key),
            ClientRpcRequest::CounterDecrement { key } => counter::handle_decrement(key),
            ClientRpcRequest::CounterAdd { key, amount } => counter::handle_add(key, amount),
            ClientRpcRequest::CounterSubtract { key, amount } => counter::handle_subtract(key, amount),
            ClientRpcRequest::CounterSet { key, value } => counter::handle_set(key, value),
            ClientRpcRequest::CounterCompareAndSet {
                key,
                expected,
                new_value,
            } => counter::handle_compare_and_set(key, expected, new_value),

            // === Signed counters ===
            ClientRpcRequest::SignedCounterGet { key } => counter::handle_signed_get(key),
            ClientRpcRequest::SignedCounterAdd { key, amount } => counter::handle_signed_add(key, amount),

            // === Sequences ===
            ClientRpcRequest::SequenceNext { key } => sequence::handle_next(key),
            ClientRpcRequest::SequenceReserve { key, count } => sequence::handle_reserve(key, count),
            ClientRpcRequest::SequenceCurrent { key } => sequence::handle_current(key),

            // === Rate limiters ===
            ClientRpcRequest::RateLimiterTryAcquire {
                key,
                tokens,
                capacity_tokens,
                refill_rate,
            } => ratelimit::handle_try_acquire(key, tokens, capacity_tokens, refill_rate),

            ClientRpcRequest::RateLimiterAcquire {
                key,
                tokens,
                capacity_tokens,
                refill_rate,
                timeout_ms,
            } => ratelimit::handle_acquire(key, tokens, capacity_tokens, refill_rate, timeout_ms),

            ClientRpcRequest::RateLimiterAvailable {
                key,
                capacity_tokens,
                refill_rate,
            } => ratelimit::handle_available(key, capacity_tokens, refill_rate),

            ClientRpcRequest::RateLimiterReset {
                key,
                capacity_tokens,
                refill_rate,
            } => ratelimit::handle_reset(key, capacity_tokens, refill_rate),

            // === Barriers ===
            ClientRpcRequest::BarrierEnter {
                name,
                participant_id,
                required_count,
                timeout_ms: _,
            } => barrier::handle_enter(name, participant_id, required_count),

            ClientRpcRequest::BarrierLeave {
                name,
                participant_id,
                timeout_ms: _,
            } => barrier::handle_leave(name, participant_id),

            ClientRpcRequest::BarrierStatus { name } => barrier::handle_status(name),

            // === Semaphores ===
            ClientRpcRequest::SemaphoreAcquire {
                name,
                holder_id,
                permits: _,
                capacity_permits,
                ttl_ms: _,
                timeout_ms,
            } => semaphore::handle_acquire(name, holder_id, capacity_permits as u64, timeout_ms),

            ClientRpcRequest::SemaphoreTryAcquire {
                name,
                holder_id,
                permits: _,
                capacity_permits,
                ttl_ms: _,
            } => semaphore::handle_try_acquire(name, holder_id, capacity_permits as u64),

            ClientRpcRequest::SemaphoreRelease {
                name,
                holder_id,
                permits: _,
            } => semaphore::handle_release(name, holder_id),

            ClientRpcRequest::SemaphoreStatus { name } => semaphore::handle_status(name),

            // === RW Locks ===
            ClientRpcRequest::RWLockAcquireRead {
                name,
                holder_id,
                ttl_ms: _,
                timeout_ms,
            } => rwlock::handle_acquire_read(name, holder_id, timeout_ms),

            ClientRpcRequest::RWLockTryAcquireRead {
                name,
                holder_id,
                ttl_ms: _,
            } => rwlock::handle_try_acquire_read(name, holder_id),

            ClientRpcRequest::RWLockAcquireWrite {
                name,
                holder_id,
                ttl_ms: _,
                timeout_ms,
            } => rwlock::handle_acquire_write(name, holder_id, timeout_ms),

            ClientRpcRequest::RWLockTryAcquireWrite {
                name,
                holder_id,
                ttl_ms: _,
            } => rwlock::handle_try_acquire_write(name, holder_id),

            ClientRpcRequest::RWLockReleaseRead { name, holder_id } => rwlock::handle_release_read(name, holder_id),

            ClientRpcRequest::RWLockReleaseWrite {
                name,
                holder_id,
                fencing_token: _,
            } => rwlock::handle_release_write(name, holder_id),

            ClientRpcRequest::RWLockDowngrade {
                name,
                holder_id,
                fencing_token: _,
                ttl_ms: _,
            } => rwlock::handle_downgrade(name, holder_id),

            ClientRpcRequest::RWLockStatus { name } => rwlock::handle_status(name),

            // === Queues ===
            ClientRpcRequest::QueueCreate {
                queue_name,
                default_visibility_timeout_ms,
                default_ttl_ms: _,
                max_delivery_attempts,
            } => queue::handle_create(queue_name, default_visibility_timeout_ms, max_delivery_attempts),

            ClientRpcRequest::QueueDelete { queue_name } => queue::handle_delete(queue_name),

            ClientRpcRequest::QueueEnqueue {
                queue_name,
                payload,
                ttl_ms: _,
                message_group_id,
                deduplication_id,
            } => queue::handle_enqueue(queue_name, payload, deduplication_id, message_group_id),

            ClientRpcRequest::QueueEnqueueBatch { queue_name, items } => {
                let payloads: Vec<Vec<u8>> = items.into_iter().map(|i| i.payload).collect();
                queue::handle_enqueue_batch(queue_name, payloads)
            }

            ClientRpcRequest::QueueDequeue {
                queue_name,
                consumer_id: _,
                max_items: _,
                visibility_timeout_ms,
            } => queue::handle_dequeue(queue_name, Some(visibility_timeout_ms)),

            ClientRpcRequest::QueueDequeueWait {
                queue_name,
                consumer_id: _,
                max_items: _,
                visibility_timeout_ms,
                wait_timeout_ms,
            } => queue::handle_dequeue_wait(queue_name, Some(visibility_timeout_ms), wait_timeout_ms),

            ClientRpcRequest::QueuePeek { queue_name, max_items } => queue::handle_peek(queue_name, Some(max_items)),

            ClientRpcRequest::QueueAck {
                queue_name,
                receipt_handle,
            } => queue::handle_ack(queue_name, receipt_handle),

            ClientRpcRequest::QueueNack {
                queue_name,
                receipt_handle,
                move_to_dlq,
                error_message: _,
            } => queue::handle_nack(queue_name, receipt_handle, move_to_dlq),

            ClientRpcRequest::QueueExtendVisibility {
                queue_name,
                receipt_handle,
                additional_timeout_ms,
            } => queue::handle_extend_visibility(queue_name, receipt_handle, additional_timeout_ms),

            ClientRpcRequest::QueueStatus { queue_name } => queue::handle_status(queue_name),

            ClientRpcRequest::QueueGetDLQ { queue_name, max_items } => {
                queue::handle_get_dlq(queue_name, Some(max_items))
            }

            ClientRpcRequest::QueueRedriveDLQ { queue_name, item_id } => {
                queue::handle_redrive_dlq(queue_name, Some(item_id.to_string()))
            }

            _ => ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "UNHANDLED_REQUEST".to_string(),
                message: "Request not handled by coordination plugin".to_string(),
            }),
        }
    }
}

register_plugin!(CoordinationPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let info = CoordinationPlugin::info();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("valid plugin.json");

        assert_eq!(info.name, manifest["name"].as_str().unwrap());
        assert_eq!(info.version, manifest["version"].as_str().unwrap());
        assert_eq!(info.priority, manifest["priority"].as_u64().unwrap() as u32);
        assert_eq!(info.app_id.as_deref(), manifest["app_id"].as_str());
        assert_eq!(info.handles.len(), manifest["handles"].as_array().unwrap().len(), "handle count mismatch");
    }
}
