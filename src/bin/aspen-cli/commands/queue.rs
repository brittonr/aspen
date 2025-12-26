//! Distributed queue commands.
//!
//! Commands for distributed message queues with visibility timeouts,
//! dead letter queues, FIFO ordering, and deduplication support.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::QueueEnqueueItem;

/// Distributed queue operations.
#[derive(Subcommand)]
pub enum QueueCommand {
    /// Create a new queue.
    Create(CreateArgs),

    /// Delete a queue and all its items.
    Delete(DeleteArgs),

    /// Enqueue an item to a queue.
    Enqueue(EnqueueArgs),

    /// Enqueue multiple items in a batch.
    EnqueueBatch(EnqueueBatchArgs),

    /// Dequeue items from a queue (non-blocking).
    Dequeue(DequeueArgs),

    /// Dequeue items with blocking wait.
    DequeueWait(DequeueWaitArgs),

    /// Peek at items without removing them.
    Peek(PeekArgs),

    /// Acknowledge successful processing of an item.
    Ack(AckArgs),

    /// Negative acknowledge - return to queue or move to DLQ.
    Nack(NackArgs),

    /// Extend visibility timeout for a pending item.
    Extend(ExtendArgs),

    /// Get queue status.
    Status(StatusArgs),

    /// Get items from dead letter queue.
    Dlq(DlqArgs),

    /// Move DLQ item back to main queue.
    Redrive(RedriveArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Queue name.
    pub queue_name: String,

    /// Default visibility timeout in milliseconds.
    #[arg(long, default_value = "30000")]
    pub visibility_timeout: u64,

    /// Default item TTL in milliseconds (0 = no expiration).
    #[arg(long, default_value = "0")]
    pub ttl: u64,

    /// Max delivery attempts before DLQ (0 = no limit).
    #[arg(long, default_value = "3")]
    pub max_attempts: u32,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Queue name.
    pub queue_name: String,
}

#[derive(Args)]
pub struct EnqueueArgs {
    /// Queue name.
    pub queue_name: String,

    /// Item payload (string).
    pub payload: String,

    /// Optional TTL in milliseconds.
    #[arg(long)]
    pub ttl: Option<u64>,

    /// Optional message group ID for FIFO ordering.
    #[arg(long)]
    pub group: Option<String>,

    /// Optional deduplication ID.
    #[arg(long)]
    pub dedup: Option<String>,
}

#[derive(Args)]
pub struct EnqueueBatchArgs {
    /// Queue name.
    pub queue_name: String,

    /// Items to enqueue (JSON array of objects with payload, ttl_ms, message_group_id, deduplication_id).
    #[arg(long)]
    pub items: String,
}

#[derive(Args)]
pub struct DequeueArgs {
    /// Queue name.
    pub queue_name: String,

    /// Consumer ID.
    #[arg(long)]
    pub consumer: String,

    /// Maximum items to return.
    #[arg(long, default_value = "1")]
    pub max: u32,

    /// Visibility timeout in milliseconds.
    #[arg(long)]
    pub visibility: u64,
}

#[derive(Args)]
pub struct DequeueWaitArgs {
    /// Queue name.
    pub queue_name: String,

    /// Consumer ID.
    #[arg(long)]
    pub consumer: String,

    /// Maximum items to return.
    #[arg(long, default_value = "1")]
    pub max: u32,

    /// Visibility timeout in milliseconds.
    #[arg(long)]
    pub visibility: u64,

    /// Wait timeout in milliseconds.
    #[arg(long)]
    pub wait: u64,
}

#[derive(Args)]
pub struct PeekArgs {
    /// Queue name.
    pub queue_name: String,

    /// Maximum items to return.
    #[arg(long, default_value = "10")]
    pub max: u32,
}

#[derive(Args)]
pub struct AckArgs {
    /// Queue name.
    pub queue_name: String,

    /// Receipt handle from dequeue.
    pub receipt_handle: String,
}

#[derive(Args)]
pub struct NackArgs {
    /// Queue name.
    pub queue_name: String,

    /// Receipt handle from dequeue.
    pub receipt_handle: String,

    /// Whether to move directly to DLQ.
    #[arg(long)]
    pub to_dlq: bool,

    /// Optional error message.
    #[arg(long)]
    pub error: Option<String>,
}

#[derive(Args)]
pub struct ExtendArgs {
    /// Queue name.
    pub queue_name: String,

    /// Receipt handle.
    pub receipt_handle: String,

    /// Additional timeout in milliseconds.
    #[arg(long)]
    pub add: u64,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Queue name.
    pub queue_name: String,
}

#[derive(Args)]
pub struct DlqArgs {
    /// Queue name.
    pub queue_name: String,

    /// Maximum items to return.
    #[arg(long, default_value = "10")]
    pub max: u32,
}

#[derive(Args)]
pub struct RedriveArgs {
    /// Queue name.
    pub queue_name: String,

    /// Item ID in DLQ.
    pub item_id: u64,
}

/// Queue create output.
pub struct QueueCreateOutput {
    pub success: bool,
    pub created: bool,
    pub error: Option<String>,
}

impl Outputable for QueueCreateOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "created": self.created,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            if self.created {
                "Queue created"
            } else {
                "Queue already exists"
            }
            .to_string()
        } else {
            format!("Create failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Queue delete output.
pub struct QueueDeleteOutput {
    pub success: bool,
    pub items_deleted: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for QueueDeleteOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "items_deleted": self.items_deleted,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Queue deleted ({} items)", self.items_deleted.unwrap_or(0))
        } else {
            format!("Delete failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Queue enqueue output.
pub struct QueueEnqueueOutput {
    pub success: bool,
    pub item_id: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for QueueEnqueueOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "item_id": self.item_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Enqueued. Item ID: {}", self.item_id.unwrap_or(0))
        } else {
            format!("Enqueue failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Queue enqueue batch output.
pub struct QueueEnqueueBatchOutput {
    pub success: bool,
    pub item_ids: Vec<u64>,
    pub error: Option<String>,
}

impl Outputable for QueueEnqueueBatchOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "item_ids": self.item_ids,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Enqueued {} items", self.item_ids.len())
        } else {
            format!("Enqueue batch failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Queue dequeue output.
pub struct QueueDequeueOutput {
    pub success: bool,
    pub items: Vec<QueueItemDisplay>,
    pub error: Option<String>,
}

pub struct QueueItemDisplay {
    pub item_id: u64,
    pub receipt_handle: String,
    pub payload: String,
    pub delivery_attempts: u32,
}

impl Outputable for QueueDequeueOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "items": self.items.iter().map(|i| {
                serde_json::json!({
                    "item_id": i.item_id,
                    "receipt_handle": i.receipt_handle,
                    "payload": i.payload,
                    "delivery_attempts": i.delivery_attempts
                })
            }).collect::<Vec<_>>(),
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.success {
            return format!("Dequeue failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        if self.items.is_empty() {
            return "No items available".to_string();
        }

        let mut output = format!("Dequeued {} item(s)\n", self.items.len());
        for item in &self.items {
            output.push_str(&format!(
                "\n[{}] (handle: {}, attempts: {})\n{}\n",
                item.item_id, item.receipt_handle, item.delivery_attempts, item.payload
            ));
        }
        output
    }
}

/// Queue simple success output.
pub struct QueueSuccessOutput {
    pub operation: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for QueueSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "OK".to_string()
        } else {
            format!("{} failed: {}", self.operation, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Queue status output.
pub struct QueueStatusOutput {
    pub success: bool,
    pub queue_name: String,
    pub visible_count: Option<u64>,
    pub pending_count: Option<u64>,
    pub dlq_count: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for QueueStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "queue_name": self.queue_name,
            "visible_count": self.visible_count,
            "pending_count": self.pending_count,
            "dlq_count": self.dlq_count,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.success {
            return format!("Status failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        format!(
            "Queue: {}\n  Visible: {}\n  Pending: {}\n  DLQ:     {}",
            self.queue_name,
            self.visible_count.unwrap_or(0),
            self.pending_count.unwrap_or(0),
            self.dlq_count.unwrap_or(0)
        )
    }
}

impl QueueCommand {
    /// Execute the queue command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            QueueCommand::Create(args) => queue_create(client, args, json).await,
            QueueCommand::Delete(args) => queue_delete(client, args, json).await,
            QueueCommand::Enqueue(args) => queue_enqueue(client, args, json).await,
            QueueCommand::EnqueueBatch(args) => queue_enqueue_batch(client, args, json).await,
            QueueCommand::Dequeue(args) => queue_dequeue(client, args, json).await,
            QueueCommand::DequeueWait(args) => queue_dequeue_wait(client, args, json).await,
            QueueCommand::Peek(args) => queue_peek(client, args, json).await,
            QueueCommand::Ack(args) => queue_ack(client, args, json).await,
            QueueCommand::Nack(args) => queue_nack(client, args, json).await,
            QueueCommand::Extend(args) => queue_extend(client, args, json).await,
            QueueCommand::Status(args) => queue_status(client, args, json).await,
            QueueCommand::Dlq(args) => queue_dlq(client, args, json).await,
            QueueCommand::Redrive(args) => queue_redrive(client, args, json).await,
        }
    }
}

async fn queue_create(client: &AspenClient, args: CreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueCreate {
            queue_name: args.queue_name,
            default_visibility_timeout_ms: Some(args.visibility_timeout),
            default_ttl_ms: Some(args.ttl),
            max_delivery_attempts: Some(args.max_attempts),
        })
        .await?;

    match response {
        ClientRpcResponse::QueueCreateResult(result) => {
            let output = QueueCreateOutput {
                success: result.success,
                created: result.created,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_delete(client: &AspenClient, args: DeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueDelete {
            queue_name: args.queue_name,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueDeleteResult(result) => {
            let output = QueueDeleteOutput {
                success: result.success,
                items_deleted: result.items_deleted,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_enqueue(client: &AspenClient, args: EnqueueArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueEnqueue {
            queue_name: args.queue_name,
            payload: args.payload.into_bytes(),
            ttl_ms: args.ttl,
            message_group_id: args.group,
            deduplication_id: args.dedup,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueEnqueueResult(result) => {
            let output = QueueEnqueueOutput {
                success: result.success,
                item_id: result.item_id,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_enqueue_batch(client: &AspenClient, args: EnqueueBatchArgs, json: bool) -> Result<()> {
    // Parse JSON items
    let items: Vec<serde_json::Value> =
        serde_json::from_str(&args.items).map_err(|e| anyhow::anyhow!("invalid items JSON: {}", e))?;

    let queue_items: Vec<QueueEnqueueItem> = items
        .into_iter()
        .map(|v| QueueEnqueueItem {
            payload: v["payload"].as_str().unwrap_or("").as_bytes().to_vec(),
            ttl_ms: v["ttl_ms"].as_u64(),
            message_group_id: v["message_group_id"].as_str().map(|s| s.to_string()),
            deduplication_id: v["deduplication_id"].as_str().map(|s| s.to_string()),
        })
        .collect();

    let response = client
        .send(ClientRpcRequest::QueueEnqueueBatch {
            queue_name: args.queue_name,
            items: queue_items,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueEnqueueBatchResult(result) => {
            let output = QueueEnqueueBatchOutput {
                success: result.success,
                item_ids: result.item_ids,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_dequeue(client: &AspenClient, args: DequeueArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueDequeue {
            queue_name: args.queue_name,
            consumer_id: args.consumer,
            max_items: args.max,
            visibility_timeout_ms: args.visibility,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueDequeueResult(result) => {
            let items = result
                .items
                .into_iter()
                .map(|i| QueueItemDisplay {
                    item_id: i.item_id,
                    receipt_handle: i.receipt_handle,
                    payload: String::from_utf8(i.payload).unwrap_or_else(|_| "<binary>".to_string()),
                    delivery_attempts: i.delivery_attempts,
                })
                .collect();

            let output = QueueDequeueOutput {
                success: result.success,
                items,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_dequeue_wait(client: &AspenClient, args: DequeueWaitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueDequeueWait {
            queue_name: args.queue_name,
            consumer_id: args.consumer,
            max_items: args.max,
            visibility_timeout_ms: args.visibility,
            wait_timeout_ms: args.wait,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueDequeueResult(result) => {
            let items = result
                .items
                .into_iter()
                .map(|i| QueueItemDisplay {
                    item_id: i.item_id,
                    receipt_handle: i.receipt_handle,
                    payload: String::from_utf8(i.payload).unwrap_or_else(|_| "<binary>".to_string()),
                    delivery_attempts: i.delivery_attempts,
                })
                .collect();

            let output = QueueDequeueOutput {
                success: result.success,
                items,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_peek(client: &AspenClient, args: PeekArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueuePeek {
            queue_name: args.queue_name,
            max_items: args.max,
        })
        .await?;

    match response {
        ClientRpcResponse::QueuePeekResult(result) => {
            let items = result
                .items
                .into_iter()
                .map(|i| QueueItemDisplay {
                    item_id: i.item_id,
                    receipt_handle: String::new(), // No receipt handle for peek
                    payload: String::from_utf8(i.payload).unwrap_or_else(|_| "<binary>".to_string()),
                    delivery_attempts: i.delivery_attempts,
                })
                .collect();

            let output = QueueDequeueOutput {
                success: result.success,
                items,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_ack(client: &AspenClient, args: AckArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueAck {
            queue_name: args.queue_name,
            receipt_handle: args.receipt_handle,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueAckResult(result) => {
            let output = QueueSuccessOutput {
                operation: "ack".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_nack(client: &AspenClient, args: NackArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueNack {
            queue_name: args.queue_name,
            receipt_handle: args.receipt_handle,
            move_to_dlq: args.to_dlq,
            error_message: args.error,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueNackResult(result) => {
            let output = QueueSuccessOutput {
                operation: "nack".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_extend(client: &AspenClient, args: ExtendArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueExtendVisibility {
            queue_name: args.queue_name,
            receipt_handle: args.receipt_handle,
            additional_timeout_ms: args.add,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueExtendVisibilityResult(result) => {
            let output = QueueSuccessOutput {
                operation: "extend".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_status(client: &AspenClient, args: StatusArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueStatus {
            queue_name: args.queue_name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::QueueStatusResult(result) => {
            let output = QueueStatusOutput {
                success: result.success,
                queue_name: args.queue_name,
                visible_count: result.visible_count,
                pending_count: result.pending_count,
                dlq_count: result.dlq_count,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_dlq(client: &AspenClient, args: DlqArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueGetDLQ {
            queue_name: args.queue_name,
            max_items: args.max,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueGetDLQResult(result) => {
            let items = result
                .items
                .into_iter()
                .map(|i| QueueItemDisplay {
                    item_id: i.item_id,
                    receipt_handle: String::new(),
                    payload: String::from_utf8(i.payload).unwrap_or_else(|_| "<binary>".to_string()),
                    delivery_attempts: i.delivery_attempts,
                })
                .collect();

            let output = QueueDequeueOutput {
                success: result.success,
                items,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn queue_redrive(client: &AspenClient, args: RedriveArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::QueueRedriveDLQ {
            queue_name: args.queue_name,
            item_id: args.item_id,
        })
        .await?;

    match response {
        ClientRpcResponse::QueueRedriveDLQResult(result) => {
            let output = QueueSuccessOutput {
                operation: "redrive".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
