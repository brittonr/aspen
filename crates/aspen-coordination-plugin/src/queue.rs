//! Distributed queue operations.
//!
//! Queue state: `__coord:queue:{name}:config` — JSON QueueConfig
//! Queue items: `__coord:queue:{name}:item:{id}` — JSON QueueItem
//! DLQ items:   `__coord:queue:{name}:dlq:{id}` — JSON DLQItem

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::QueueAckResultResponse;
use aspen_client_api::QueueCreateResultResponse;
use aspen_client_api::QueueDeleteResultResponse;
use aspen_client_api::QueueDequeueResultResponse;
use aspen_client_api::QueueDequeuedItemResponse;
use aspen_client_api::QueueEnqueueBatchResultResponse;
use aspen_client_api::QueueEnqueueResultResponse;
use aspen_client_api::QueueExtendVisibilityResultResponse;
use aspen_client_api::QueueGetDLQResultResponse;
use aspen_client_api::QueueItemResponse;
use aspen_client_api::QueueNackResultResponse;
use aspen_client_api::QueuePeekResultResponse;
use aspen_client_api::QueueRedriveDLQResultResponse;
use aspen_client_api::QueueStatusResultResponse;
use aspen_wasm_guest_sdk::host::current_time_ms;
use aspen_wasm_guest_sdk::host::get_random_bytes;
use serde::Deserialize;
use serde::Serialize;

use crate::kv;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueConfig {
    visibility_timeout_ms: u64,
    max_delivery_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueItem {
    id: u64,
    payload: Vec<u8>,
    enqueued_at_ms: u64,
    delivery_count: u32,
    visible_after_ms: u64,
    receipt_handle: String,
    dedup_key: Option<String>,
    group_id: Option<String>,
}

fn config_key(name: &str) -> String {
    format!("__coord:queue:{name}:config")
}

fn item_key(name: &str, id: u64) -> String {
    format!("__coord:queue:{name}:item:{id:016x}")
}

fn item_prefix(name: &str) -> String {
    format!("__coord:queue:{name}:item:")
}

fn dlq_key(name: &str, id: u64) -> String {
    format!("__coord:queue:{name}:dlq:{id:016x}")
}

fn dlq_prefix(name: &str) -> String {
    format!("__coord:queue:{name}:dlq:")
}

fn gen_id() -> u64 {
    let bytes = get_random_bytes(8);
    u64::from_le_bytes(bytes[..8].try_into().unwrap_or([0u8; 8]))
}

fn gen_receipt() -> String {
    hex::encode(get_random_bytes(16))
}

pub fn handle_create(
    queue_name: String,
    default_visibility_timeout_ms: Option<u64>,
    max_delivery_attempts: Option<u32>,
) -> ClientRpcResponse {
    let k = config_key(&queue_name);
    // Check if already exists
    if kv::get_json::<QueueConfig>(&k).ok().flatten().is_some() {
        return ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            is_success: true,
            was_created: false,
            error: None,
        });
    }
    let config = QueueConfig {
        visibility_timeout_ms: default_visibility_timeout_ms.unwrap_or(30000),
        max_delivery_attempts: max_delivery_attempts.unwrap_or(5),
    };
    match kv::put_json(&k, &config) {
        Ok(()) => ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            is_success: true,
            was_created: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            is_success: false,
            was_created: false,
            error: Some(e),
        }),
    }
}

pub fn handle_delete(queue_name: String) -> ClientRpcResponse {
    let _ = kv::delete(&config_key(&queue_name));
    let items = kv::scan(&item_prefix(&queue_name), 1000).unwrap_or_default();
    let count = items.len() as u64;
    for (key, _) in &items {
        let _ = kv::delete(key);
    }
    for (key, _) in kv::scan(&dlq_prefix(&queue_name), 1000).unwrap_or_default() {
        let _ = kv::delete(&key);
    }
    ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
        is_success: true,
        items_deleted: Some(count),
        error: None,
    })
}

pub fn handle_enqueue(
    queue_name: String,
    payload: Vec<u8>,
    dedup_key: Option<String>,
    group_id: Option<String>,
) -> ClientRpcResponse {
    // Dedup check
    if let Some(ref dk) = dedup_key {
        if let Ok(items) = kv::scan(&item_prefix(&queue_name), 1000) {
            for (_, v) in &items {
                if let Ok(item) = serde_json::from_slice::<QueueItem>(v) {
                    if item.dedup_key.as_deref() == Some(dk.as_str()) {
                        return ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
                            is_success: true,
                            item_id: Some(item.id),
                            error: None,
                        });
                    }
                }
            }
        }
    }

    let id = gen_id();
    let item = QueueItem {
        id,
        payload,
        enqueued_at_ms: current_time_ms(),
        delivery_count: 0,
        visible_after_ms: 0,
        receipt_handle: String::new(),
        dedup_key,
        group_id,
    };
    match kv::put_json(&item_key(&queue_name, id), &item) {
        Ok(()) => ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
            is_success: true,
            item_id: Some(id),
            error: None,
        }),
        Err(e) => ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
            is_success: false,
            item_id: None,
            error: Some(e),
        }),
    }
}

pub fn handle_enqueue_batch(queue_name: String, payloads: Vec<Vec<u8>>) -> ClientRpcResponse {
    let mut ids = Vec::new();
    for payload in payloads {
        let id = gen_id();
        let item = QueueItem {
            id,
            payload,
            enqueued_at_ms: current_time_ms(),
            delivery_count: 0,
            visible_after_ms: 0,
            receipt_handle: String::new(),
            dedup_key: None,
            group_id: None,
        };
        if let Err(e) = kv::put_json(&item_key(&queue_name, id), &item) {
            return ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
                is_success: false,
                item_ids: ids,
                error: Some(e),
            });
        }
        ids.push(id);
    }
    ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
        is_success: true,
        item_ids: ids,
        error: None,
    })
}

pub fn handle_dequeue(queue_name: String, visibility_timeout_ms: Option<u64>) -> ClientRpcResponse {
    let config = match kv::get_json::<QueueConfig>(&config_key(&queue_name)) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: false,
                items: vec![],
                error: Some("queue not found".to_string()),
            });
        }
        Err(e) => {
            return ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: false,
                items: vec![],
                error: Some(e),
            });
        }
    };

    let timeout = visibility_timeout_ms.unwrap_or(config.visibility_timeout_ms);
    let now = current_time_ms();

    let scanned = match kv::scan(&item_prefix(&queue_name), 100) {
        Ok(items) => items,
        Err(e) => {
            return ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: false,
                items: vec![],
                error: Some(e),
            });
        }
    };

    for (key, value) in scanned {
        if let Ok(mut item) = serde_json::from_slice::<QueueItem>(&value) {
            if item.visible_after_ms <= now {
                let old_raw = match std::str::from_utf8(&value) {
                    Ok(s) => s.to_string(),
                    Err(_) => continue,
                };
                item.delivery_count += 1;
                item.visible_after_ms = now + timeout;
                item.receipt_handle = gen_receipt();
                if kv::cas_json(&key, Some(&old_raw), &item).unwrap_or(false) {
                    return ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                        is_success: true,
                        items: vec![QueueDequeuedItemResponse {
                            item_id: item.id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_count,
                            enqueued_at_ms: item.enqueued_at_ms,
                            visibility_deadline_ms: item.visible_after_ms,
                        }],
                        error: None,
                    });
                }
            }
        }
    }

    ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
        is_success: true,
        items: vec![],
        error: None,
    })
}

pub fn handle_dequeue_wait(
    queue_name: String,
    visibility_timeout_ms: Option<u64>,
    wait_timeout_ms: u64,
) -> ClientRpcResponse {
    let deadline = if wait_timeout_ms > 0 {
        current_time_ms() + wait_timeout_ms
    } else {
        current_time_ms() + 5000
    };

    loop {
        let result = handle_dequeue(queue_name.clone(), visibility_timeout_ms);
        if let ClientRpcResponse::QueueDequeueResult(ref r) = result {
            if !r.items.is_empty() || !r.is_success {
                return result;
            }
        }
        if current_time_ms() > deadline {
            return ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                is_success: true,
                items: vec![],
                error: None,
            });
        }
    }
}

pub fn handle_peek(queue_name: String, max_items: Option<u32>) -> ClientRpcResponse {
    let limit = max_items.unwrap_or(10).min(100);
    let now = current_time_ms();

    let scanned = match kv::scan(&item_prefix(&queue_name), limit) {
        Ok(items) => items,
        Err(e) => {
            return ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
                is_success: false,
                items: vec![],
                error: Some(e),
            });
        }
    };

    let items: Vec<QueueItemResponse> = scanned
        .iter()
        .filter_map(|(_, v)| serde_json::from_slice::<QueueItem>(v).ok())
        .filter(|item| item.visible_after_ms <= now)
        .map(|item| QueueItemResponse {
            item_id: item.id,
            payload: item.payload,
            enqueued_at_ms: item.enqueued_at_ms,
            expires_at_ms: 0,
            delivery_attempts: item.delivery_count,
        })
        .collect();

    ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
        is_success: true,
        items,
        error: None,
    })
}

pub fn handle_ack(queue_name: String, receipt_handle: String) -> ClientRpcResponse {
    // Find item by receipt handle
    if let Ok(items) = kv::scan(&item_prefix(&queue_name), 1000) {
        for (key, value) in items {
            if let Ok(item) = serde_json::from_slice::<QueueItem>(&value) {
                if item.receipt_handle == receipt_handle {
                    let _ = kv::delete(&key);
                    return ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
                        is_success: true,
                        error: None,
                    });
                }
            }
        }
    }
    ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
        is_success: false,
        error: Some("receipt handle not found".to_string()),
    })
}

pub fn handle_nack(queue_name: String, receipt_handle: String, move_to_dlq: bool) -> ClientRpcResponse {
    if let Ok(items) = kv::scan(&item_prefix(&queue_name), 1000) {
        for (key, value) in items {
            if let Ok(mut item) = serde_json::from_slice::<QueueItem>(&value) {
                if item.receipt_handle == receipt_handle {
                    if move_to_dlq {
                        let dk = dlq_key(&queue_name, item.id);
                        let _ = kv::put_json(&dk, &item);
                        let _ = kv::delete(&key);
                    } else {
                        item.visible_after_ms = 0;
                        item.receipt_handle = String::new();
                        let old_raw = std::str::from_utf8(&value).unwrap_or("").to_string();
                        let _ = kv::cas_json(&key, Some(&old_raw), &item);
                    }
                    return ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
                        is_success: true,
                        error: None,
                    });
                }
            }
        }
    }
    ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
        is_success: false,
        error: Some("receipt handle not found".to_string()),
    })
}

pub fn handle_extend_visibility(
    queue_name: String,
    receipt_handle: String,
    additional_timeout_ms: u64,
) -> ClientRpcResponse {
    if let Ok(items) = kv::scan(&item_prefix(&queue_name), 1000) {
        for (key, value) in items {
            if let Ok(mut item) = serde_json::from_slice::<QueueItem>(&value) {
                if item.receipt_handle == receipt_handle {
                    let new_deadline = current_time_ms() + additional_timeout_ms;
                    item.visible_after_ms = new_deadline;
                    let old_raw = std::str::from_utf8(&value).unwrap_or("").to_string();
                    let _ = kv::cas_json(&key, Some(&old_raw), &item);
                    return ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
                        is_success: true,
                        new_deadline_ms: Some(new_deadline),
                        error: None,
                    });
                }
            }
        }
    }
    ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
        is_success: false,
        new_deadline_ms: None,
        error: Some("receipt handle not found".to_string()),
    })
}

pub fn handle_status(queue_name: String) -> ClientRpcResponse {
    let exists = kv::get_json::<QueueConfig>(&config_key(&queue_name)).ok().flatten().is_some();

    let items = kv::scan(&item_prefix(&queue_name), 10000).unwrap_or_default();
    let dlq_count = kv::scan(&dlq_prefix(&queue_name), 10000).unwrap_or_default().len() as u64;
    let now = current_time_ms();

    let visible = items
        .iter()
        .filter(|(_, v)| serde_json::from_slice::<QueueItem>(v).map(|i| i.visible_after_ms <= now).unwrap_or(false))
        .count() as u64;

    let pending = (items.len() as u64).saturating_sub(visible);

    ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
        is_success: true,
        does_exist: exists,
        visible_count: Some(visible),
        pending_count: Some(pending),
        dlq_count: Some(dlq_count),
        total_enqueued: Some(items.len() as u64),
        total_acked: None,
        error: None,
    })
}

pub fn handle_get_dlq(queue_name: String, max_items: Option<u32>) -> ClientRpcResponse {
    let limit = max_items.unwrap_or(10).min(100);
    let items = match kv::scan(&dlq_prefix(&queue_name), limit) {
        Ok(items) => items,
        Err(e) => {
            return ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
                is_success: false,
                items: vec![],
                error: Some(e),
            });
        }
    };

    let dlq_items: Vec<aspen_client_api::QueueDLQItemResponse> = items
        .iter()
        .filter_map(|(_, v)| serde_json::from_slice::<QueueItem>(v).ok())
        .map(|item| aspen_client_api::QueueDLQItemResponse {
            item_id: item.id,
            payload: item.payload,
            enqueued_at_ms: item.enqueued_at_ms,
            delivery_attempts: item.delivery_count,
            reason: "max delivery attempts exceeded".to_string(),
            moved_at_ms: current_time_ms(),
            last_error: None,
        })
        .collect();

    ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
        is_success: true,
        items: dlq_items,
        error: None,
    })
}

pub fn handle_redrive_dlq(queue_name: String, item_id: Option<String>) -> ClientRpcResponse {
    let dlq_items = kv::scan(&dlq_prefix(&queue_name), 1000).unwrap_or_default();

    for (key, value) in &dlq_items {
        if let Ok(mut item) = serde_json::from_slice::<QueueItem>(value) {
            if let Some(ref target_id) = item_id {
                if item.id.to_string() != *target_id {
                    continue;
                }
            }
            item.delivery_count = 0;
            item.visible_after_ms = 0;
            item.receipt_handle = String::new();
            if kv::put_json(&item_key(&queue_name, item.id), &item).is_ok() {
                let _ = kv::delete(key);
            }
            if item_id.is_some() {
                break;
            }
        }
    }

    ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
        is_success: true,
        error: None,
    })
}
