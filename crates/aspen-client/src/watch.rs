//! Streaming watch client for real-time key change notifications.
//!
//! Provides a high-level API for subscribing to key changes via the log
//! subscription protocol (LOG_SUBSCRIBER_ALPN).
//!
//! ## Protocol Flow
//!
//! 1. Connect to node via `LOG_SUBSCRIBER_ALPN` ("aspen-logs")
//! 2. Complete cookie-based authentication (same as Raft auth)
//! 3. Send `SubscribeRequest` with optional key prefix filter
//! 4. Receive streaming `LogEntryMessage` events
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::client::watch::{WatchSession, WatchEvent};
//!
//! // Connect to a node
//! let session = WatchSession::connect(
//!     endpoint,
//!     node_addr,
//!     "cluster-cookie",
//! ).await?;
//!
//! // Subscribe to key prefix with historical replay
//! let mut subscription = session.subscribe("user:", 0).await?;
//!
//! // Receive events
//! while let Some(event) = subscription.next().await {
//!     match event {
//!         WatchEvent::Set { key, value, index } => {
//!             println!("Key {} set to {:?} at index {}", key, value, index);
//!         }
//!         WatchEvent::Delete { key, index } => {
//!             println!("Key {} deleted at index {}", key, index);
//!         }
//!         WatchEvent::Keepalive { committed_index } => {
//!             println!("Keepalive: committed index {}", committed_index);
//!         }
//!     }
//! }
//! ```

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use aspen_raft::auth::AuthChallenge;
use aspen_raft::auth::AuthContext;
use aspen_raft::auth::AuthResult;
use aspen_transport::log_subscriber::KvOperation;
use aspen_transport::log_subscriber::LOG_SUBSCRIBER_ALPN;
use aspen_transport::log_subscriber::LogEntryMessage;
use aspen_transport::log_subscriber::LogEntryPayload;
use aspen_transport::log_subscriber::MAX_LOG_ENTRY_MESSAGE_SIZE;
use aspen_transport::log_subscriber::SUBSCRIBE_HANDSHAKE_TIMEOUT;
use aspen_transport::log_subscriber::SubscribeRequest;
use aspen_transport::log_subscriber::SubscribeResponse;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::Connection;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Event emitted by a watch subscription.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A key was set.
    Set {
        /// Key that was modified.
        key: Vec<u8>,
        /// New value.
        value: Vec<u8>,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// A key was set with TTL.
    SetWithTTL {
        /// Key that was modified.
        key: Vec<u8>,
        /// New value.
        value: Vec<u8>,
        /// Expiration time (ms since epoch).
        expires_at_ms: u64,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// A key was deleted.
    Delete {
        /// Key that was deleted.
        key: Vec<u8>,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// Multiple keys were set atomically.
    SetMulti {
        /// Keys and values that were set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// Multiple keys were deleted atomically.
    DeleteMulti {
        /// Keys that were deleted.
        keys: Vec<Vec<u8>>,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// Compare-and-swap succeeded.
    CompareAndSwap {
        /// Key that was modified.
        key: Vec<u8>,
        /// New value.
        new_value: Vec<u8>,
        /// Log index of this change.
        index: u64,
        /// Raft term when change was committed.
        term: u64,
        /// Timestamp when committed (ms since epoch).
        committed_at_ms: u64,
    },
    /// Keepalive message from server.
    Keepalive {
        /// Current committed index.
        committed_index: u64,
        /// Server timestamp (ms since epoch).
        timestamp_ms: u64,
    },
    /// Cluster membership changed.
    MembershipChange {
        /// Description of the change.
        description: String,
        /// Log index of this change.
        index: u64,
    },
    /// Stream is ending.
    EndOfStream {
        /// Reason for termination.
        reason: String,
    },
}

impl WatchEvent {
    /// Convert a LogEntryPayload into a vector of WatchEvents.
    pub fn from_payload(payload: LogEntryPayload) -> Vec<WatchEvent> {
        let LogEntryPayload {
            index,
            term,
            committed_at_ms,
            operation,
        } = payload;

        match operation {
            KvOperation::Set { key, value } => vec![WatchEvent::Set {
                key,
                value,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => vec![WatchEvent::SetWithTTL {
                key,
                value,
                expires_at_ms,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::SetMulti { pairs } => vec![WatchEvent::SetMulti {
                pairs,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::SetMultiWithTTL {
                pairs,
                expires_at_ms: _,
            } => {
                // Treat SetMultiWithTTL as SetMulti for watch events
                vec![WatchEvent::SetMulti {
                    pairs,
                    index,
                    term,
                    committed_at_ms,
                }]
            }
            KvOperation::Delete { key } => vec![WatchEvent::Delete {
                key,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::DeleteMulti { keys } => vec![WatchEvent::DeleteMulti {
                keys,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::CompareAndSwap {
                key,
                expected: _,
                new_value,
            } => vec![WatchEvent::CompareAndSwap {
                key,
                new_value,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::CompareAndDelete { key, expected: _ } => vec![WatchEvent::Delete {
                key,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::Batch { operations } => {
                // Convert batch operations to individual events
                operations
                    .into_iter()
                    .map(|(is_set, key, value)| {
                        if is_set {
                            WatchEvent::Set {
                                key,
                                value,
                                index,
                                term,
                                committed_at_ms,
                            }
                        } else {
                            WatchEvent::Delete {
                                key,
                                index,
                                term,
                                committed_at_ms,
                            }
                        }
                    })
                    .collect()
            }
            KvOperation::ConditionalBatch {
                conditions: _,
                operations,
            } => {
                // Convert batch operations to individual events
                operations
                    .into_iter()
                    .map(|(is_set, key, value)| {
                        if is_set {
                            WatchEvent::Set {
                                key,
                                value,
                                index,
                                term,
                                committed_at_ms,
                            }
                        } else {
                            WatchEvent::Delete {
                                key,
                                index,
                                term,
                                committed_at_ms,
                            }
                        }
                    })
                    .collect()
            }
            KvOperation::Noop => vec![], // Skip no-ops
            KvOperation::MembershipChange { description } => {
                vec![WatchEvent::MembershipChange { description, index }]
            }
            KvOperation::SetWithLease {
                key,
                value,
                lease_id: _,
            } => vec![WatchEvent::Set {
                key,
                value,
                index,
                term,
                committed_at_ms,
            }],
            KvOperation::SetMultiWithLease { pairs, lease_id: _ } => vec![WatchEvent::SetMulti {
                pairs,
                index,
                term,
                committed_at_ms,
            }],
            // Lease operations are control-plane, not key-value operations
            KvOperation::LeaseGrant { .. } => vec![],
            KvOperation::LeaseRevoke { .. } => vec![],
            KvOperation::LeaseKeepalive { .. } => vec![],
            KvOperation::Transaction { success, failure, .. } => {
                // Convert transaction operations to watch events
                // Include both branches since we don't know which executed
                let mut events = Vec::new();
                for (op_type, key, value) in success.into_iter().chain(failure.into_iter()) {
                    match op_type {
                        0 => {
                            // Put
                            events.push(WatchEvent::Set {
                                key,
                                value,
                                index,
                                term,
                                committed_at_ms,
                            });
                        }
                        1 => {
                            // Delete
                            events.push(WatchEvent::Delete {
                                key,
                                index,
                                term,
                                committed_at_ms,
                            });
                        }
                        _ => {
                            // Get/Range - no watch event needed
                        }
                    }
                }
                events
            }
            KvOperation::OptimisticTransaction { read_set: _, write_set } => {
                // Convert write set to watch events (read set is for validation only)
                write_set
                    .into_iter()
                    .map(|(is_set, key, value)| {
                        if is_set {
                            WatchEvent::Set {
                                key,
                                value,
                                index,
                                term,
                                committed_at_ms,
                            }
                        } else {
                            WatchEvent::Delete {
                                key,
                                index,
                                term,
                                committed_at_ms,
                            }
                        }
                    })
                    .collect()
            }
        }
    }
}

/// A connected watch session to an Aspen node.
///
/// Maintains an authenticated connection to the node's log subscriber
/// protocol and can create multiple subscriptions.
pub struct WatchSession {
    /// The underlying QUIC connection.
    connection: Connection,
    /// Current committed index on the server.
    current_index: u64,
    /// Node ID of the connected server.
    node_id: u64,
}

impl WatchSession {
    /// Connect to a node's log subscriber endpoint.
    ///
    /// # Arguments
    /// * `endpoint` - Iroh endpoint to use for the connection
    /// * `node_addr` - Address of the target node
    /// * `cluster_cookie` - Shared secret for authentication
    ///
    /// # Returns
    /// A connected and authenticated watch session.
    pub async fn connect(endpoint: &Endpoint, target_addr: EndpointAddr, cluster_cookie: &str) -> Result<Self> {
        let connection = endpoint
            .connect(target_addr, LOG_SUBSCRIBER_ALPN)
            .await
            .context("failed to connect to log subscriber endpoint")?;

        debug!(
            remote = %connection.remote_id(),
            "connected to log subscriber endpoint"
        );

        // Perform authentication handshake
        let node_id = Self::authenticate(&connection, cluster_cookie, endpoint).await?;

        info!(node_id = node_id, "authenticated to log subscriber");

        Ok(Self {
            connection,
            current_index: 0,
            node_id,
        })
    }

    /// Perform the authentication handshake.
    ///
    /// The server sends a challenge on accepting our stream, we compute the
    /// response using our cookie, then receive the auth result on the same stream.
    async fn authenticate(connection: &Connection, cluster_cookie: &str, endpoint: &Endpoint) -> Result<u64> {
        // The server sends the challenge on the first stream it accepts from us
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open bidirectional stream")?;

        // Receive auth challenge (server writes it when it accepts our stream)
        let challenge_bytes = timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
            recv.read_to_end(1024).await.context("failed to read auth challenge")
        })
        .await
        .context("auth challenge timeout")??;

        let challenge: AuthChallenge =
            postcard::from_bytes(&challenge_bytes).context("failed to parse auth challenge")?;

        debug!(nonce = %hex::encode(challenge.nonce), "received auth challenge");

        // Create auth context and compute response
        let auth_context = AuthContext::new(cluster_cookie);
        let client_endpoint_id: [u8; 32] = *endpoint.id().as_bytes();
        let response = auth_context.compute_response(&challenge, &client_endpoint_id);

        let response_bytes = postcard::to_stdvec(&response).context("failed to serialize auth response")?;

        send.write_all(&response_bytes).await.context("failed to send auth response")?;
        send.finish().context("failed to finish auth response stream")?;

        // Read auth result on the receive side of the same stream
        // Wait for any remaining data
        let result_bytes = timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
            recv.read_to_end(1024).await.context("failed to read auth result")
        })
        .await
        .context("auth result timeout")??;

        let result: AuthResult = postcard::from_bytes(&result_bytes).context("failed to parse auth result")?;

        if !result.is_ok() {
            bail!("authentication failed: {:?}", result);
        }

        // We don't have access to the node_id from auth result (it's just an enum)
        // Return 0 for now - the actual node_id comes from subscribe response
        Ok(0)
    }

    /// Subscribe to key changes with an optional prefix filter.
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to watch (empty string for all keys)
    /// * `start_index` - Starting log index (0 = from beginning, u64::MAX = latest only)
    ///
    /// # Returns
    /// A subscription that can be used to receive watch events.
    pub async fn subscribe(&self, prefix: impl Into<Vec<u8>>, start_index: u64) -> Result<WatchSubscription> {
        let prefix = prefix.into();

        // Open stream for subscription
        let (mut send, mut recv) = self.connection.open_bi().await.context("failed to open subscription stream")?;

        // Send subscribe request
        let request = SubscribeRequest::with_prefix(start_index, prefix.clone());
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize subscribe request")?;

        send.write_all(&request_bytes).await.context("failed to send subscribe request")?;
        send.finish().context("failed to finish subscribe request stream")?;

        // Receive subscribe response
        let response_bytes = timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
            // Read response from receive stream
            let mut buf = vec![0u8; 1024];
            let n = recv
                .read(&mut buf)
                .await
                .context("failed to read subscribe response")?
                .ok_or_else(|| anyhow::anyhow!("stream closed before response"))?;
            buf.truncate(n);
            Ok::<Vec<u8>, anyhow::Error>(buf)
        })
        .await
        .context("subscribe response timeout")??;

        let response: SubscribeResponse =
            postcard::from_bytes(&response_bytes).context("failed to parse subscribe response")?;

        match response {
            SubscribeResponse::Accepted { current_index, node_id } => {
                debug!(
                    node_id = node_id,
                    current_index = current_index,
                    prefix = %String::from_utf8_lossy(&prefix),
                    "subscription accepted"
                );

                // Create channel for events
                let (event_tx, event_rx) = mpsc::channel(1000);

                // Spawn background task to read events
                let recv_handle = tokio::spawn(async move {
                    Self::event_reader(recv, event_tx).await;
                });

                Ok(WatchSubscription {
                    event_rx,
                    current_index,
                    prefix,
                    _recv_handle: recv_handle,
                })
            }
            SubscribeResponse::Rejected { reason } => {
                bail!("subscription rejected: {}", reason);
            }
        }
    }

    /// Background task that reads events from the stream.
    async fn event_reader(mut recv: iroh::endpoint::RecvStream, event_tx: mpsc::Sender<WatchEvent>) {
        loop {
            // Read next message
            let mut buf = vec![0u8; MAX_LOG_ENTRY_MESSAGE_SIZE];
            let n = match recv.read(&mut buf).await {
                Ok(Some(n)) => n,
                Ok(None) => {
                    debug!("log subscriber stream closed gracefully");
                    let _ = event_tx
                        .send(WatchEvent::EndOfStream {
                            reason: "stream closed".to_string(),
                        })
                        .await;
                    break;
                }
                Err(e) => {
                    error!(error = %e, "error reading from log subscriber stream");
                    let _ = event_tx
                        .send(WatchEvent::EndOfStream {
                            reason: format!("read error: {}", e),
                        })
                        .await;
                    break;
                }
            };
            buf.truncate(n);

            // Parse message
            let message: LogEntryMessage = match postcard::from_bytes(&buf) {
                Ok(m) => m,
                Err(e) => {
                    warn!(error = %e, "failed to parse log entry message");
                    continue;
                }
            };

            // Convert to events
            match message {
                LogEntryMessage::Entry(payload) => {
                    let events = WatchEvent::from_payload(payload);
                    for event in events {
                        if event_tx.send(event).await.is_err() {
                            debug!("event receiver dropped, stopping reader");
                            return;
                        }
                    }
                }
                LogEntryMessage::Keepalive {
                    committed_index,
                    timestamp_ms,
                } => {
                    if event_tx
                        .send(WatchEvent::Keepalive {
                            committed_index,
                            timestamp_ms,
                        })
                        .await
                        .is_err()
                    {
                        debug!("event receiver dropped, stopping reader");
                        return;
                    }
                }
                LogEntryMessage::EndOfStream { reason } => {
                    let _ = event_tx
                        .send(WatchEvent::EndOfStream {
                            reason: reason.to_string(),
                        })
                        .await;
                    break;
                }
            }
        }
    }

    /// Get the current committed index.
    pub fn current_index(&self) -> u64 {
        self.current_index
    }

    /// Get the connected node ID.
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Close the session.
    pub fn close(self) {
        self.connection.close(0u32.into(), b"session closed");
    }
}

/// An active subscription to key changes.
///
/// Receives events for keys matching the subscription prefix.
pub struct WatchSubscription {
    /// Channel for receiving events.
    event_rx: mpsc::Receiver<WatchEvent>,
    /// Current committed index at subscription time.
    current_index: u64,
    /// Key prefix being watched.
    prefix: Vec<u8>,
    /// Handle to the background reader task.
    _recv_handle: tokio::task::JoinHandle<()>,
}

impl WatchSubscription {
    /// Receive the next event.
    ///
    /// Returns `None` when the subscription ends (connection closed or error).
    pub async fn next(&mut self) -> Option<WatchEvent> {
        self.event_rx.recv().await
    }

    /// Receive the next event with a timeout.
    ///
    /// Returns `None` if no event is received within the timeout.
    pub async fn next_timeout(&mut self, duration: Duration) -> Option<WatchEvent> {
        timeout(duration, self.event_rx.recv()).await.ok().flatten()
    }

    /// Try to receive an event without blocking.
    ///
    /// Returns `None` if no event is immediately available.
    pub fn try_next(&mut self) -> Option<WatchEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Get the current committed index at subscription time.
    pub fn current_index(&self) -> u64 {
        self.current_index
    }

    /// Get the key prefix being watched.
    pub fn prefix(&self) -> &[u8] {
        &self.prefix
    }

    /// Check if more events are available without blocking.
    pub fn is_empty(&self) -> bool {
        self.event_rx.is_empty()
    }

    /// Get the number of pending events in the buffer.
    pub fn pending_count(&self) -> usize {
        self.event_rx.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_event_from_set_operation() {
        let payload = LogEntryPayload {
            index: 100,
            term: 5,
            committed_at_ms: 1700000000000,
            operation: KvOperation::Set {
                key: b"test_key".to_vec(),
                value: b"test_value".to_vec(),
            },
        };

        let events = WatchEvent::from_payload(payload);
        assert_eq!(events.len(), 1);

        match &events[0] {
            WatchEvent::Set {
                key,
                value,
                index,
                term,
                committed_at_ms,
            } => {
                assert_eq!(key, b"test_key");
                assert_eq!(value, b"test_value");
                assert_eq!(*index, 100);
                assert_eq!(*term, 5);
                assert_eq!(*committed_at_ms, 1700000000000);
            }
            _ => panic!("expected Set event"),
        }
    }

    #[test]
    fn test_watch_event_from_delete_operation() {
        let payload = LogEntryPayload {
            index: 101,
            term: 5,
            committed_at_ms: 1700000000100,
            operation: KvOperation::Delete {
                key: b"deleted_key".to_vec(),
            },
        };

        let events = WatchEvent::from_payload(payload);
        assert_eq!(events.len(), 1);

        match &events[0] {
            WatchEvent::Delete {
                key,
                index,
                term,
                committed_at_ms,
            } => {
                assert_eq!(key, b"deleted_key");
                assert_eq!(*index, 101);
                assert_eq!(*term, 5);
                assert_eq!(*committed_at_ms, 1700000000100);
            }
            _ => panic!("expected Delete event"),
        }
    }

    #[test]
    fn test_watch_event_from_batch_operation() {
        let payload = LogEntryPayload {
            index: 102,
            term: 5,
            committed_at_ms: 1700000000200,
            operation: KvOperation::Batch {
                operations: vec![
                    (true, b"set_key".to_vec(), b"set_value".to_vec()),
                    (false, b"del_key".to_vec(), vec![]),
                ],
            },
        };

        let events = WatchEvent::from_payload(payload);
        assert_eq!(events.len(), 2);

        match &events[0] {
            WatchEvent::Set { key, value, .. } => {
                assert_eq!(key, b"set_key");
                assert_eq!(value, b"set_value");
            }
            _ => panic!("expected Set event"),
        }

        match &events[1] {
            WatchEvent::Delete { key, .. } => {
                assert_eq!(key, b"del_key");
            }
            _ => panic!("expected Delete event"),
        }
    }

    #[test]
    fn test_watch_event_from_noop_is_empty() {
        let payload = LogEntryPayload {
            index: 103,
            term: 5,
            committed_at_ms: 1700000000300,
            operation: KvOperation::Noop,
        };

        let events = WatchEvent::from_payload(payload);
        assert!(events.is_empty());
    }

    #[test]
    fn test_watch_event_from_membership_change() {
        let payload = LogEntryPayload {
            index: 104,
            term: 5,
            committed_at_ms: 1700000000400,
            operation: KvOperation::MembershipChange {
                description: "added node 3".to_string(),
            },
        };

        let events = WatchEvent::from_payload(payload);
        assert_eq!(events.len(), 1);

        match &events[0] {
            WatchEvent::MembershipChange { description, index } => {
                assert_eq!(description, "added node 3");
                assert_eq!(*index, 104);
            }
            _ => panic!("expected MembershipChange event"),
        }
    }
}
