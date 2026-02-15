// Queue operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueItem {
    pub payload: Vec<u8>,
    pub ttl_ms: Option<u64>,
    pub message_group_id: Option<String>,
    pub deduplication_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueCreateResultResponse {
    pub success: bool,
    pub created: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDeleteResultResponse {
    pub success: bool,
    pub items_deleted: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueResultResponse {
    pub success: bool,
    pub item_id: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueBatchResultResponse {
    pub success: bool,
    pub item_ids: Vec<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeuedItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub receipt_handle: String,
    pub delivery_attempts: u32,
    pub enqueued_at_ms: u64,
    pub visibility_deadline_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeueResultResponse {
    pub success: bool,
    pub items: Vec<QueueDequeuedItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub enqueued_at_ms: u64,
    pub expires_at_ms: u64,
    pub delivery_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePeekResultResponse {
    pub success: bool,
    pub items: Vec<QueueItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAckResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNackResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExtendVisibilityResultResponse {
    pub success: bool,
    pub new_deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusResultResponse {
    pub success: bool,
    pub exists: bool,
    pub visible_count: Option<u64>,
    pub pending_count: Option<u64>,
    pub dlq_count: Option<u64>,
    pub total_enqueued: Option<u64>,
    pub total_acked: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDLQItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub enqueued_at_ms: u64,
    pub delivery_attempts: u32,
    pub reason: String,
    pub moved_at_ms: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueGetDLQResultResponse {
    pub success: bool,
    pub items: Vec<QueueDLQItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRedriveDLQResultResponse {
    pub success: bool,
    pub error: Option<String>,
}
