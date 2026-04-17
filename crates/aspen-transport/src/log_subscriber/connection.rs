//! Log subscriber connection handling.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use anyhow::Context;
use aspen_auth::hmac_auth::AuthContext;
use aspen_auth::hmac_auth::AuthResponse;
use aspen_auth::hmac_auth::AuthResult;
use aspen_core::hlc::SerializableTimestamp;
use iroh::endpoint::Connection;
use iroh::endpoint::RecvStream;
use iroh::endpoint::SendStream;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::constants::AUTH_HANDSHAKE_TIMEOUT;
use super::constants::MAX_AUTH_MESSAGE_SIZE;
use super::constants::MAX_HISTORICAL_BATCH_SIZE;
use super::constants::MAX_LOG_ENTRY_MESSAGE_SIZE;
use super::constants::SUBSCRIBE_HANDSHAKE_TIMEOUT;
use super::constants::SUBSCRIBE_KEEPALIVE_INTERVAL;
use super::types::EndOfStreamReason;
use super::types::HistoricalLogReader;
use super::types::LogEntryMessage;
use super::types::LogEntryPayload;
use super::types::SubscribeRejectReason;
use super::types::SubscribeRequest;
use super::types::SubscribeResponse;
use super::wire::write_message;

struct SubscriptionAcceptContext {
    subscriber_id: u64,
    node_id: u64,
}

struct HistoricalReplayBounds {
    start_index: u64,
    committed_index: u64,
}

struct LiveStreamContext<'a> {
    committed_index: &'a Arc<AtomicU64>,
    hlc: &'a aspen_core::hlc::HLC,
    key_prefix: &'a [u8],
    subscriber_id: u64,
}

pub(super) struct LogSubscriberConnectionContext<'a> {
    pub auth_context: AuthContext,
    pub log_receiver: broadcast::Receiver<LogEntryPayload>,
    pub node_id: u64,
    pub subscriber_id: u64,
    pub committed_index: Arc<AtomicU64>,
    pub historical_reader: Option<Arc<dyn HistoricalLogReader>>,
    pub hlc: &'a aspen_core::hlc::HLC,
    pub watch_registry: Option<Arc<dyn aspen_core::WatchRegistry>>,
}

const _: () = {
    assert!(MAX_AUTH_MESSAGE_SIZE > 0);
    assert!(MAX_HISTORICAL_BATCH_SIZE > 0);
    assert!(MAX_LOG_ENTRY_MESSAGE_SIZE > 0);
    assert!(MAX_LOG_ENTRY_MESSAGE_SIZE >= MAX_AUTH_MESSAGE_SIZE);
};

fn historical_batch_span() -> u64 {
    u64::try_from(MAX_HISTORICAL_BATCH_SIZE).unwrap_or(u64::MAX).saturating_sub(1)
}

fn historical_replay_max_batches(bounds: HistoricalReplayBounds) -> u64 {
    let batch_width = historical_batch_span().saturating_add(1);
    let entry_count = bounds.committed_index.saturating_sub(bounds.start_index).saturating_add(1);
    debug_assert!(batch_width >= 1);
    entry_count.saturating_add(batch_width.saturating_sub(1)) / batch_width
}

fn next_log_index(index: u64) -> u64 {
    index.saturating_add(1)
}

async fn send_auth_failure(send: &mut SendStream, subscriber_id: u64) {
    if let Err(error) = write_message(send, &AuthResult::Failed, MAX_AUTH_MESSAGE_SIZE).await {
        debug!(subscriber_id = subscriber_id, error = %error, "failed to send auth failure");
    }
    if let Err(error) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %error, "failed to finish auth failure stream");
    }
}

async fn read_auth_response(recv: &mut RecvStream) -> anyhow::Result<AuthResponse> {
    tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv.read_to_end(MAX_AUTH_MESSAGE_SIZE).await.context("failed to read auth response")?;
        let response: AuthResponse = postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await
    .map_err(|_| anyhow::anyhow!("authentication timeout"))?
}

async fn authenticate_subscriber(
    connection: &Connection,
    auth_context: &AuthContext,
    subscriber_id: u64,
) -> anyhow::Result<()> {
    debug_assert!(AUTH_HANDSHAKE_TIMEOUT > std::time::Duration::ZERO);

    let (mut auth_send, mut auth_recv) = connection.accept_bi().await.context("failed to accept auth stream")?;
    let challenge = auth_context.generate_challenge();
    write_message(&mut auth_send, &challenge, MAX_AUTH_MESSAGE_SIZE)
        .await
        .context("failed to send challenge")?;

    let auth_response = match read_auth_response(&mut auth_recv).await {
        Ok(response) => response,
        Err(error) => {
            warn!(error = %error, subscriber_id = subscriber_id, "subscriber auth failed");
            send_auth_failure(&mut auth_send, subscriber_id).await;
            return Err(error);
        }
    };

    let auth_result = auth_context.verify_response(&challenge, &auth_response);
    write_message(&mut auth_send, &auth_result, MAX_AUTH_MESSAGE_SIZE)
        .await
        .context("failed to send auth result")?;
    if let Err(finish_err) = auth_send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish auth stream");
    }
    if !auth_result.is_ok() {
        warn!(subscriber_id = subscriber_id, result = ?auth_result, "subscriber auth failed");
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }
    debug!(subscriber_id = subscriber_id, "subscriber authenticated");
    Ok(())
}

async fn reject_subscription(send: &mut SendStream, subscriber_id: u64) {
    let response = SubscribeResponse::Rejected {
        reason: SubscribeRejectReason::InternalError,
    };
    if let Err(error) = write_message(send, &response, MAX_AUTH_MESSAGE_SIZE).await {
        debug!(subscriber_id = subscriber_id, error = %error, "failed to send subscription rejection");
    }
    if let Err(error) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %error, "failed to finish subscription rejection stream");
    }
}

async fn read_subscription_request(recv: &mut RecvStream) -> anyhow::Result<SubscribeRequest> {
    tokio::time::timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
        let buffer = recv.read_to_end(MAX_AUTH_MESSAGE_SIZE).await.context("failed to read subscribe request")?;
        let request: SubscribeRequest =
            postcard::from_bytes(&buffer).context("failed to deserialize subscribe request")?;
        Ok::<_, anyhow::Error>(request)
    })
    .await
    .map_err(|_| anyhow::anyhow!("subscribe request timeout"))?
}

async fn accept_subscription(
    connection: &Connection,
    context: SubscriptionAcceptContext,
    committed_index: &Arc<AtomicU64>,
) -> anyhow::Result<(SendStream, SubscribeRequest, u64)> {
    debug_assert!(SUBSCRIBE_HANDSHAKE_TIMEOUT > std::time::Duration::ZERO);

    let (mut send, mut recv) = connection.accept_bi().await.context("failed to accept subscribe stream")?;
    let sub_request = match read_subscription_request(&mut recv).await {
        Ok(request) => request,
        Err(error) => {
            reject_subscription(&mut send, context.subscriber_id).await;
            return Err(error);
        }
    };

    debug!(
        subscriber_id = context.subscriber_id,
        start_index = sub_request.start_index,
        prefix = ?sub_request.key_prefix,
        "processing subscription request"
    );
    let current_committed_index = committed_index.load(Ordering::Acquire);
    let response = SubscribeResponse::Accepted {
        current_index: current_committed_index,
        node_id: context.node_id,
    };
    write_message(&mut send, &response, MAX_AUTH_MESSAGE_SIZE)
        .await
        .context("failed to send subscribe response")?;
    Ok((send, sub_request, current_committed_index))
}

async fn replay_historical_entries(
    send: &mut SendStream,
    subscriber_id: u64,
    sub_request: &SubscribeRequest,
    current_committed_index: u64,
    historical_reader: &Option<Arc<dyn HistoricalLogReader>>,
) -> anyhow::Result<u64> {
    if sub_request.start_index >= current_committed_index || sub_request.start_index == u64::MAX {
        return Ok(current_committed_index);
    }
    let Some(reader) = historical_reader else {
        debug!(subscriber_id = subscriber_id, "historical replay requested but no reader available");
        return Ok(current_committed_index);
    };

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        end_index = current_committed_index,
        "starting historical replay"
    );
    let mut current_start = sub_request.start_index;
    let mut total_replayed = 0u64;
    let max_replay_batches = historical_replay_max_batches(HistoricalReplayBounds {
        start_index: sub_request.start_index,
        committed_index: current_committed_index,
    });
    debug_assert!(current_start <= current_committed_index);
    debug_assert!(max_replay_batches >= 1);

    for _batch_idx in 0..max_replay_batches {
        let batch_end = current_start.saturating_add(historical_batch_span()).min(current_committed_index);
        match reader.read_entries(current_start, batch_end).await {
            Ok(entries) => {
                if entries.is_empty() {
                    debug!(
                        subscriber_id = subscriber_id,
                        start = current_start,
                        "no historical entries available, may have been compacted"
                    );
                    break;
                }
                for entry in entries {
                    if !sub_request.key_prefix.is_empty() && !entry.operation.matches_prefix(&sub_request.key_prefix) {
                        continue;
                    }
                    let message = LogEntryMessage::Entry(entry.clone());
                    write_message(send, &message, MAX_LOG_ENTRY_MESSAGE_SIZE)
                        .await
                        .map_err(|_| anyhow::anyhow!("subscriber disconnected during replay"))?;
                    total_replayed = total_replayed.saturating_add(1);
                    current_start = next_log_index(entry.index);
                }
            }
            Err(error) => {
                warn!(subscriber_id = subscriber_id, error = %error, "failed to read historical entries, continuing with live stream");
                break;
            }
        }
        if current_start > current_committed_index {
            break;
        }
    }
    info!(subscriber_id = subscriber_id, total_replayed = total_replayed, "historical replay complete");
    Ok(current_start)
}

async fn stream_live_entries(
    send: &mut SendStream,
    log_receiver: &mut broadcast::Receiver<LogEntryPayload>,
    context: LiveStreamContext<'_>,
) {
    debug_assert!(SUBSCRIBE_KEEPALIVE_INTERVAL > std::time::Duration::ZERO);

    let mut keepalive_tick = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            entry_result = log_receiver.recv() => match entry_result {
                Ok(entry) => {
                    if !context.key_prefix.is_empty() && !entry.operation.matches_prefix(context.key_prefix) {
                        continue;
                    }
                    if let Err(error) = write_message(send, &LogEntryMessage::Entry(entry), MAX_LOG_ENTRY_MESSAGE_SIZE).await {
                        debug!(subscriber_id = context.subscriber_id, error = %error, "subscriber disconnected");
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!(subscriber_id = context.subscriber_id, lagged_count = count, "subscriber lagged, disconnecting");
                    if let Err(error) = write_message(
                        send,
                        &LogEntryMessage::EndOfStream { reason: EndOfStreamReason::Lagged },
                        MAX_AUTH_MESSAGE_SIZE,
                    ).await {
                        debug!(subscriber_id = context.subscriber_id, error = %error, "failed to send lagged end-of-stream");
                    }
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!(subscriber_id = context.subscriber_id, "log broadcast channel closed");
                    if let Err(error) = write_message(
                        send,
                        &LogEntryMessage::EndOfStream { reason: EndOfStreamReason::ServerShutdown },
                        MAX_AUTH_MESSAGE_SIZE,
                    ).await {
                        debug!(subscriber_id = context.subscriber_id, error = %error, "failed to send shutdown end-of-stream");
                    }
                    break;
                }
            },
            _ = keepalive_tick.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: context.committed_index.load(Ordering::Acquire),
                    hlc_timestamp: SerializableTimestamp::from(context.hlc.new_timestamp()),
                };
                if write_message(send, &keepalive, MAX_AUTH_MESSAGE_SIZE).await.is_err() {
                    debug!(subscriber_id = context.subscriber_id, "subscriber disconnected during keepalive");
                    break;
                }
            }
        }
    }
}

/// Handle a log subscriber connection.
#[instrument(skip(connection, context))]
pub(super) async fn handle_log_subscriber_connection(
    connection: Connection,
    mut context: LogSubscriberConnectionContext<'_>,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();
    authenticate_subscriber(&connection, &context.auth_context, context.subscriber_id).await?;
    let (mut send, sub_request, current_committed_index) = accept_subscription(
        &connection,
        SubscriptionAcceptContext {
            subscriber_id: context.subscriber_id,
            node_id: context.node_id,
        },
        &context.committed_index,
    )
    .await?;

    info!(
        subscriber_id = context.subscriber_id,
        remote = %remote_node_id,
        start_index = sub_request.start_index,
        current_index = current_committed_index,
        "log subscription active"
    );

    let watch_id = context.watch_registry.as_ref().map(|registry| {
        let prefix = String::from_utf8_lossy(&sub_request.key_prefix).to_string();
        registry.register_watch(prefix, false)
    });

    replay_historical_entries(
        &mut send,
        context.subscriber_id,
        &sub_request,
        current_committed_index,
        &context.historical_reader,
    )
    .await?;
    stream_live_entries(&mut send, &mut context.log_receiver, LiveStreamContext {
        committed_index: &context.committed_index,
        hlc: context.hlc,
        key_prefix: &sub_request.key_prefix,
        subscriber_id: context.subscriber_id,
    })
    .await;

    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = context.subscriber_id, error = %finish_err, "failed to finish log subscription stream");
    }
    if let (Some(registry), Some(id)) = (&context.watch_registry, watch_id) {
        registry.unregister_watch(id);
    }
    info!(subscriber_id = context.subscriber_id, "log subscription ended");
    Ok(())
}
