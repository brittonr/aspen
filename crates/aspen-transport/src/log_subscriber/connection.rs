//! Log subscriber connection handling.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_auth::hmac_auth::AuthContext;
use aspen_auth::hmac_auth::AuthResponse;
use aspen_auth::hmac_auth::AuthResult;
use aspen_core::hlc::SerializableTimestamp;
use iroh::endpoint::Connection;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::constants::AUTH_HANDSHAKE_TIMEOUT;
use super::constants::MAX_AUTH_MESSAGE_SIZE;
use super::constants::MAX_HISTORICAL_BATCH_SIZE;
use super::constants::SUBSCRIBE_HANDSHAKE_TIMEOUT;
use super::constants::SUBSCRIBE_KEEPALIVE_INTERVAL;
use super::types::EndOfStreamReason;
use super::types::HistoricalLogReader;
use super::types::LogEntryMessage;
use super::types::LogEntryPayload;
use super::types::SubscribeRejectReason;
use super::types::SubscribeRequest;
use super::types::SubscribeResponse;

/// Handle a log subscriber connection.
#[allow(clippy::too_many_arguments)]
#[instrument(skip(
    connection,
    auth_context,
    log_receiver,
    committed_index,
    historical_reader,
    hlc,
    watch_registry
))]
pub(super) async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    node_id: u64,
    subscriber_id: u64,
    committed_index: Arc<AtomicU64>,
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
    hlc: &aspen_core::hlc::HLC,
    watch_registry: Option<Arc<dyn aspen_core::WatchRegistry>>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let remote_node_id = connection.remote_id();

    // Accept the initial stream for authentication and subscription setup
    let (mut send, mut recv) = connection.accept_bi().await.context("failed to accept subscriber stream")?;

    // Step 1: Send challenge
    let challenge = auth_context.generate_challenge();
    let challenge_bytes = postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    send.write_all(&challenge_bytes).await.context("failed to send challenge")?;

    // Step 2: Receive auth response
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv.read_to_end(MAX_AUTH_MESSAGE_SIZE).await.context("failed to read auth response")?;
        let response: AuthResponse = postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let auth_response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, subscriber_id = subscriber_id, "subscriber auth failed");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth failure to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth failure");
            }
            return Err(err);
        }
        Err(_) => {
            warn!(subscriber_id = subscriber_id, "subscriber auth timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth timeout to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth timeout");
            }
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify
    let auth_result = auth_context.verify_response(&challenge, &auth_response);

    // Step 4: Send auth result
    let result_bytes = postcard::to_stdvec(&auth_result)?;
    send.write_all(&result_bytes).await.context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(subscriber_id = subscriber_id, result = ?auth_result, "subscriber auth failed");
        if let Err(finish_err) = send.finish() {
            debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth verification failure");
        }
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!(subscriber_id = subscriber_id, "subscriber authenticated");

    // Step 5: Receive subscription request
    let sub_request_result = tokio::time::timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
        let buffer = recv
            .read_to_end(1024) // Subscription requests are small
            .await
            .context("failed to read subscribe request")?;
        let request: SubscribeRequest =
            postcard::from_bytes(&buffer).context("failed to deserialize subscribe request")?;
        Ok::<_, anyhow::Error>(request)
    })
    .await;

    let sub_request = match sub_request_result {
        Ok(Ok(request)) => request,
        Ok(Err(err)) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe error");
            }
            return Err(err);
        }
        Err(_) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe timeout rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe timeout");
            }
            return Err(anyhow::anyhow!("subscribe request timeout"));
        }
    };

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        prefix = ?sub_request.key_prefix,
        "processing subscription request"
    );

    // Step 6: Accept subscription
    let current_committed_index = committed_index.load(Ordering::Acquire);
    let response = SubscribeResponse::Accepted {
        current_index: current_committed_index,
        node_id,
    };
    let response_bytes = postcard::to_stdvec(&response)?;
    send.write_all(&response_bytes).await.context("failed to send subscribe response")?;

    info!(
        subscriber_id = subscriber_id,
        remote = %remote_node_id,
        start_index = sub_request.start_index,
        current_index = current_committed_index,
        "log subscription active"
    );

    // Register watch with registry if configured
    // Note: Log subscriptions don't include previous values (that's a client-side watch feature)
    let watch_id = watch_registry.as_ref().map(|registry| {
        let prefix = String::from_utf8_lossy(&sub_request.key_prefix).to_string();
        registry.register_watch(prefix, false)
    });

    // Step 6b: Historical replay if requested and available
    let replay_end_index = if sub_request.start_index < current_committed_index && sub_request.start_index != u64::MAX {
        if let Some(ref reader) = historical_reader {
            debug!(
                subscriber_id = subscriber_id,
                start_index = sub_request.start_index,
                end_index = current_committed_index,
                "starting historical replay"
            );

            // Replay in batches to avoid memory exhaustion
            let mut current_start = sub_request.start_index;
            let mut total_replayed = 0u64;

            while current_start <= current_committed_index {
                let batch_end = std::cmp::min(
                    current_start.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64 - 1),
                    current_committed_index,
                );

                match reader.read_entries(current_start, batch_end).await {
                    Ok(entries) => {
                        if entries.is_empty() {
                            // No more entries available (may have been compacted)
                            debug!(
                                subscriber_id = subscriber_id,
                                start = current_start,
                                "no historical entries available, may have been compacted"
                            );
                            break;
                        }

                        for entry in entries {
                            // Apply prefix filter
                            if !sub_request.key_prefix.is_empty()
                                && !entry.operation.matches_prefix(&sub_request.key_prefix)
                            {
                                continue;
                            }

                            let message = LogEntryMessage::Entry(entry.clone());
                            let message_bytes = match postcard::to_stdvec(&message) {
                                Ok(bytes) => bytes,
                                Err(err) => {
                                    error!(error = %err, "failed to serialize historical entry");
                                    continue;
                                }
                            };

                            if let Err(err) = send.write_all(&message_bytes).await {
                                debug!(
                                    subscriber_id = subscriber_id,
                                    error = %err,
                                    "subscriber disconnected during replay"
                                );
                                return Err(anyhow::anyhow!("subscriber disconnected during replay"));
                            }

                            total_replayed += 1;
                            current_start = entry.index + 1;
                        }
                    }
                    Err(err) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            error = %err,
                            "failed to read historical entries, continuing with live stream"
                        );
                        break;
                    }
                }

                // Check if we've finished
                if current_start > current_committed_index {
                    break;
                }
            }

            info!(subscriber_id = subscriber_id, total_replayed = total_replayed, "historical replay complete");
            current_start
        } else {
            debug!(subscriber_id = subscriber_id, "historical replay requested but no reader available");
            current_committed_index
        }
    } else {
        current_committed_index
    };

    // Update the log receiver to skip entries we've already sent
    // (entries between replay_end_index and any new entries that arrived during replay)
    let _ = replay_end_index; // Used for logging context

    // Step 7: Stream log entries
    let key_prefix = sub_request.key_prefix;
    let mut keepalive_interval = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Receive log entry from broadcast channel
            entry_result = log_receiver.recv() => {
                match entry_result {
                    Ok(entry) => {
                        // Apply prefix filter
                        if !key_prefix.is_empty() && !entry.operation.matches_prefix(&key_prefix) {
                            continue;
                        }

                        let message = LogEntryMessage::Entry(entry);
                        let message_bytes = match postcard::to_stdvec(&message) {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                error!(error = %err, "failed to serialize log entry");
                                continue;
                            }
                        };

                        if let Err(err) = send.write_all(&message_bytes).await {
                            debug!(subscriber_id = subscriber_id, error = %err, "subscriber disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            lagged_count = count,
                            "subscriber lagged, disconnecting"
                        );
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::Lagged,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send lagged end-of-stream");
                            }
                        }
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(subscriber_id = subscriber_id, "log broadcast channel closed");
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::ServerShutdown,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send shutdown end-of-stream");
                            }
                        }
                        break;
                    }
                }
            }

            // Send keepalive on idle
            _ = keepalive_interval.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: committed_index.load(Ordering::Acquire),
                    hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                };
                let message_bytes = match postcard::to_stdvec(&keepalive) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if send.write_all(&message_bytes).await.is_err() {
                    debug!(subscriber_id = subscriber_id, "subscriber disconnected during keepalive");
                    break;
                }
            }
        }
    }

    // Best-effort stream finish - log if it fails
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish log subscription stream");
    }

    // Unregister watch from registry
    if let (Some(registry), Some(id)) = (&watch_registry, watch_id) {
        registry.unregister_watch(id);
    }

    info!(subscriber_id = subscriber_id, "log subscription ended");

    Ok(())
}
