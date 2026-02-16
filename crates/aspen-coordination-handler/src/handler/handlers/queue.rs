//! Queue and Dead Letter Queue operation handlers.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::QueueAckResultResponse;
use aspen_client_api::QueueCreateResultResponse;
use aspen_client_api::QueueDLQItemResponse;
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
use aspen_coordination::EnqueueOptions;
use aspen_coordination::QueueConfig;
use aspen_coordination::QueueManager;
use aspen_rpc_core::ClientProtocolContext;

pub(crate) async fn handle_queue_create(
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
            is_success: true,
            was_created: created,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            is_success: false,
            was_created: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_delete(
    ctx: &ClientProtocolContext,
    queue_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.delete(&queue_name).await {
        Ok(deleted) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
            is_success: true,
            items_deleted: Some(deleted),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
            is_success: false,
            items_deleted: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_enqueue(
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
            is_success: true,
            item_id: Some(item_id),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
            is_success: false,
            item_id: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_enqueue_batch(
    ctx: &ClientProtocolContext,
    queue_name: String,
    items: Vec<aspen_client_api::QueueEnqueueItem>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    let batch: Vec<(Vec<u8>, EnqueueOptions)> = items
        .into_iter()
        .map(|item| {
            (item.payload, EnqueueOptions {
                ttl_ms: item.ttl_ms,
                message_group_id: item.message_group_id,
                deduplication_id: item.deduplication_id,
            })
        })
        .collect();

    match manager.enqueue_batch(&queue_name, batch).await {
        Ok(item_ids) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
            is_success: true,
            item_ids,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
            is_success: false,
            item_ids: vec![],
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_dequeue(
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
                is_success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
            is_success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_dequeue_wait(
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
                is_success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
            is_success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_peek(
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
                is_success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
            is_success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_ack(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.ack(&queue_name, &receipt_handle).await {
        Ok(()) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
            is_success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_nack(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
    move_to_dlq: bool,
    error_message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.nack(&queue_name, &receipt_handle, move_to_dlq, error_message).await {
        Ok(()) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
            is_success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_extend_visibility(
    ctx: &ClientProtocolContext,
    queue_name: String,
    receipt_handle: String,
    additional_timeout_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.extend_visibility(&queue_name, &receipt_handle, additional_timeout_ms).await {
        Ok(new_deadline) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
            is_success: true,
            new_deadline_ms: Some(new_deadline),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
            is_success: false,
            new_deadline_ms: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_status(
    ctx: &ClientProtocolContext,
    queue_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.status(&queue_name).await {
        Ok(status) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
            is_success: true,
            does_exist: status.does_exist,
            visible_count: Some(status.visible_count),
            pending_count: Some(status.pending_count),
            dlq_count: Some(status.dlq_count),
            total_enqueued: Some(status.total_enqueued),
            total_acked: Some(status.total_acked),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
            is_success: false,
            does_exist: false,
            visible_count: None,
            pending_count: None,
            dlq_count: None,
            total_enqueued: None,
            total_acked: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_get_dlq(
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
                is_success: true,
                items: response_items,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
            is_success: false,
            items: vec![],
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_queue_redrive_dlq(
    ctx: &ClientProtocolContext,
    queue_name: String,
    item_id: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = QueueManager::new(ctx.kv_store.clone());

    match manager.redrive_dlq(&queue_name, item_id).await {
        Ok(()) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
            is_success: false,
            error: Some(e.to_string()),
        })),
    }
}
