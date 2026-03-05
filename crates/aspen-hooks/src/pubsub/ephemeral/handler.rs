//! Ephemeral pub/sub protocol handler for Iroh QUIC connections.
//!
//! Implements `iroh::protocol::ProtocolHandler` to accept remote subscribers.
//! Each connection registers a subscription with the local `EphemeralBroker`
//! and streams matching events over a QUIC send stream.

use std::sync::Arc;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::MAX_EPHEMERAL_SUBSCRIPTIONS;
use super::broker::EphemeralBroker;
use super::wire::EphemeralSubscribeRequest;
use super::wire::EphemeralWireEvent;
use super::wire::MAX_EPHEMERAL_WIRE_MESSAGE_SIZE;
use super::wire::read_frame;
use super::wire::write_frame;
use crate::pubsub::topic::TopicPattern;

/// Maximum concurrent ephemeral subscriber connections.
pub const MAX_EPHEMERAL_CONNECTIONS: u32 = 200;

/// ALPN protocol identifier for ephemeral pub/sub.
pub const EPHEMERAL_ALPN: &[u8] = b"aspen-ephemeral/0";

/// Protocol handler for ephemeral pub/sub over Iroh.
///
/// Accepts remote connections, reads a subscription request,
/// and streams matching events until the connection closes.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Bounded subscription count via broker
/// - Connection-scoped subscriptions (auto-cleanup on disconnect)
/// - Non-blocking publish (slow subscribers get dropped events)
#[derive(Debug)]
pub struct EphemeralProtocolHandler {
    broker: Arc<EphemeralBroker>,
    connection_semaphore: Arc<Semaphore>,
}

impl EphemeralProtocolHandler {
    /// Create a new ephemeral protocol handler.
    ///
    /// # Arguments
    /// * `broker` - The ephemeral broker to register subscriptions with
    pub fn new(broker: Arc<EphemeralBroker>) -> Self {
        Self {
            broker,
            connection_semaphore: Arc::new(Semaphore::new(MAX_EPHEMERAL_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for EphemeralProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();

        // Acquire connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote = %remote,
                    max = MAX_EPHEMERAL_CONNECTIONS,
                    "ephemeral connection limit reached, rejecting"
                );
                return Err(AcceptError::from_err(std::io::Error::other("ephemeral connection limit reached")));
            }
        };

        debug!(remote = %remote, "accepted ephemeral pub/sub connection");

        let broker = Arc::clone(&self.broker);
        let result = handle_ephemeral_connection(connection, broker).await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("ephemeral pub/sub protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a single ephemeral subscriber connection.
///
/// 1. Accept bi-stream
/// 2. Read subscribe request
/// 3. Validate topic pattern
/// 4. Register subscription with broker
/// 5. Stream events until disconnect
/// 6. Unsubscribe on cleanup
async fn handle_ephemeral_connection(connection: Connection, broker: Arc<EphemeralBroker>) -> anyhow::Result<()> {
    let remote = connection.remote_id();

    // Accept the first bi-stream (subscribe request + event stream)
    let (mut send, mut recv) = match connection.accept_bi().await {
        Ok(stream) => stream,
        Err(err) => {
            debug!(remote = %remote, error = %err, "ephemeral connection closed before stream");
            return Ok(());
        }
    };

    // Read subscription request
    let request: EphemeralSubscribeRequest = read_frame(&mut recv, MAX_EPHEMERAL_WIRE_MESSAGE_SIZE).await?;

    // Validate topic pattern
    let pattern = match TopicPattern::new(&request.pattern) {
        Ok(p) => p,
        Err(err) => {
            warn!(
                remote = %remote,
                pattern = %request.pattern,
                error = %err,
                "invalid topic pattern in ephemeral subscribe request"
            );
            return Ok(());
        }
    };

    // Clamp buffer size
    let buffer_size = (request.buffer_size as usize).clamp(1, MAX_EPHEMERAL_SUBSCRIPTIONS);

    // Register subscription
    let (sub_id, mut rx) = match broker.subscribe(pattern, buffer_size).await {
        Ok(result) => result,
        Err(err) => {
            warn!(
                remote = %remote,
                error = %err,
                "failed to register ephemeral subscription"
            );
            return Ok(());
        }
    };

    debug!(
        remote = %remote,
        sub_id = sub_id,
        pattern = %request.pattern,
        "ephemeral subscription registered"
    );

    // Stream events until disconnect or error
    let stream_result = stream_events_to_subscriber(&mut send, &mut recv, &mut rx).await;

    // Always unsubscribe on exit
    broker.unsubscribe(sub_id).await;

    debug!(
        remote = %remote,
        sub_id = sub_id,
        "ephemeral subscription removed"
    );

    match stream_result {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!(
                remote = %remote,
                sub_id = sub_id,
                error = %err,
                "ephemeral stream ended"
            );
            Ok(())
        }
    }
}

/// Stream events from the broker to a remote subscriber.
///
/// Loops until the mpsc receiver closes (broker shutdown / unsubscribe),
/// the QUIC send stream errors (subscriber disconnected), or the
/// recv stream is closed by the client (graceful disconnect signal).
async fn stream_events_to_subscriber(
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
    rx: &mut tokio::sync::mpsc::Receiver<crate::pubsub::event::Event>,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            // Wait for next event from broker
            event = rx.recv() => {
                match event {
                    Some(event) => {
                        let wire_event = EphemeralWireEvent {
                            topic: event.topic.as_str().to_string(),
                            timestamp_ms: event.timestamp_ms,
                            payload: event.payload,
                            headers: event.headers,
                        };
                        write_frame(send, &wire_event).await?;
                    }
                    None => break, // Broker shut down
                }
            }
            // Detect client disconnect via recv stream closing
            _ = recv.read_to_end(0) => {
                debug!("client closed recv stream, ending ephemeral subscription");
                break;
            }
        }
    }

    Ok(())
}
