//! Mock Iroh infrastructure for fast, deterministic unit tests.
//!
//! This module provides mock implementations of Iroh's P2P networking primitives,
//! enabling unit testing of code that depends on Iroh without requiring actual
//! network connections.
//!
//! # Components
//!
//! - [`MockIrohNetwork`]: Central network simulator managing virtual connections
//! - [`MockEndpoint`]: Mock Iroh endpoint that can accept and initiate connections
//! - [`MockConnection`]: Bidirectional connection between two mock endpoints
//! - [`MockSendStream`] / [`MockRecvStream`]: QUIC-like stream primitives
//!
//! # Tiger Style
//!
//! - Bounded channel sizes (256 messages per stream) to prevent unbounded memory
//! - Explicit size limits matching production (MAX_RPC_MESSAGE_SIZE, MAX_TUI_MESSAGE_SIZE)
//! - Deterministic execution - no real I/O, immediate message delivery
//! - Fail-fast behavior for configuration errors
//!
//! # Example
//!
//! ```ignore
//! use aspen::tests::support::mock_iroh::MockIrohNetwork;
//!
//! // Create a mock network
//! let network = MockIrohNetwork::new();
//!
//! // Create two endpoints
//! let ep1 = network.create_endpoint().await?;
//! let ep2 = network.create_endpoint().await?;
//!
//! // Connect ep1 to ep2
//! let conn = ep1.connect(ep2.id(), b"my-alpn").await?;
//!
//! // Open a bidirectional stream
//! let (mut send, mut recv) = conn.open_bi().await?;
//!
//! // Send data
//! send.write_all(b"hello").await?;
//! send.finish()?;
//!
//! // Accept the connection on ep2
//! let incoming = ep2.accept().await.unwrap();
//! let (mut recv2, mut send2) = incoming.accept_bi().await?;
//!
//! // Receive data
//! let data = recv2.read_to_end(1024).await?;
//! assert_eq!(data, b"hello");
//! ```

use std::collections::HashMap;
use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// Re-export ALPN constants from the main crate for test convenience
// (avoids duplication of constant definitions)
pub use aspen::{CLIENT_ALPN, RAFT_ALPN};
use bytes::Bytes;
use iroh::EndpointId;
use iroh::SecretKey;
use parking_lot::Mutex;
use rand::RngCore;
use tokio::sync::mpsc;

/// Default channel capacity for streams.
///
/// Tiger Style: Fixed limit to prevent unbounded memory growth.
const DEFAULT_STREAM_CAPACITY: usize = 256;

/// Maximum message size for mock streams (matches MAX_RPC_MESSAGE_SIZE).
const MAX_MOCK_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Central mock network managing all endpoints and connections.
///
/// Acts as a registry for mock endpoints and routes connections between them.
/// Supports deterministic failure injection for testing error handling.
#[derive(Clone)]
pub struct MockIrohNetwork {
    inner: Arc<MockNetworkInner>,
}

impl std::fmt::Debug for MockIrohNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockIrohNetwork")
            .field("endpoint_count", &self.inner.endpoints.lock().len())
            .field("connection_id_counter", &self.inner.connection_id_counter.load(Ordering::Relaxed))
            .finish()
    }
}

struct MockNetworkInner {
    /// Registered endpoints indexed by EndpointId.
    endpoints: Mutex<HashMap<EndpointId, EndpointEntry>>,
    /// Counter for generating unique connection IDs.
    connection_id_counter: AtomicU64,
    /// Counter for generating unique stream IDs.
    stream_id_counter: AtomicU64,
    /// Failure injection configuration.
    failure_injection: Mutex<FailureInjectionConfig>,
}

/// Entry for a registered endpoint.
struct EndpointEntry {
    /// Secret key for the endpoint.
    #[allow(dead_code)]
    secret_key: SecretKey,
    /// Channel for incoming connections.
    incoming_tx: mpsc::Sender<MockIncomingConnection>,
    /// ALPNs this endpoint accepts.
    alpns: Vec<Vec<u8>>,
}

/// Failure injection configuration for testing error paths.
#[derive(Default)]
pub struct FailureInjectionConfig {
    /// Endpoints that should refuse all connections.
    refused_endpoints: Vec<EndpointId>,
    /// Connections that should fail on next stream operation.
    #[allow(dead_code)]
    failing_connections: Vec<u64>,
    /// If true, simulate network partition (all connections fail).
    network_partition: bool,
}

impl MockIrohNetwork {
    /// Create a new mock network.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MockNetworkInner {
                endpoints: Mutex::new(HashMap::new()),
                connection_id_counter: AtomicU64::new(1),
                stream_id_counter: AtomicU64::new(1),
                failure_injection: Mutex::new(FailureInjectionConfig::default()),
            }),
        }
    }

    /// Create a new mock endpoint with a random identity.
    pub fn create_endpoint(&self) -> MockEndpoint {
        // Generate random bytes for secret key
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let secret_key = SecretKey::from(bytes);
        self.create_endpoint_with_key(secret_key)
    }

    /// Create a new mock endpoint with a specific secret key.
    pub fn create_endpoint_with_key(&self, secret_key: SecretKey) -> MockEndpoint {
        self.create_endpoint_with_key_and_alpns(secret_key, vec![])
    }

    /// Create a new mock endpoint with specific secret key and ALPNs.
    pub fn create_endpoint_with_key_and_alpns(&self, secret_key: SecretKey, alpns: Vec<Vec<u8>>) -> MockEndpoint {
        let endpoint_id = secret_key.public();
        let (incoming_tx, incoming_rx) = mpsc::channel(DEFAULT_STREAM_CAPACITY);

        {
            let mut endpoints = self.inner.endpoints.lock();
            endpoints.insert(
                endpoint_id,
                EndpointEntry {
                    secret_key: secret_key.clone(),
                    incoming_tx,
                    alpns,
                },
            );
        }

        MockEndpoint {
            network: self.clone(),
            endpoint_id,
            secret_key,
            incoming_rx: Arc::new(tokio::sync::Mutex::new(incoming_rx)),
        }
    }

    /// Connect from one endpoint to another.
    fn connect(&self, from: EndpointId, to: EndpointId, alpn: &[u8]) -> Result<MockConnection, MockConnectionError> {
        // Check for network partition
        {
            let config = self.inner.failure_injection.lock();
            if config.network_partition {
                return Err(MockConnectionError::NetworkPartition);
            }
            if config.refused_endpoints.contains(&to) {
                return Err(MockConnectionError::ConnectionRefused);
            }
        }

        // Look up target endpoint
        let incoming_tx = {
            let endpoints = self.inner.endpoints.lock();
            let entry = endpoints.get(&to).ok_or(MockConnectionError::EndpointNotFound)?;

            // Check ALPN support
            if !entry.alpns.is_empty() && !entry.alpns.contains(&alpn.to_vec()) {
                return Err(MockConnectionError::UnsupportedAlpn);
            }

            entry.incoming_tx.clone()
        };

        // Generate connection ID
        let connection_id = self.inner.connection_id_counter.fetch_add(1, Ordering::Relaxed);

        // Create bidirectional channels for the connection
        let (initiator_to_acceptor_tx, initiator_to_acceptor_rx) = mpsc::channel(DEFAULT_STREAM_CAPACITY);
        let (acceptor_to_initiator_tx, acceptor_to_initiator_rx) = mpsc::channel(DEFAULT_STREAM_CAPACITY);

        // Create connections for both sides
        let initiator_conn = MockConnection {
            id: connection_id,
            local_id: from,
            remote_id: to,
            network: self.clone(),
            outgoing_streams_tx: initiator_to_acceptor_tx,
            incoming_streams_rx: Arc::new(tokio::sync::Mutex::new(acceptor_to_initiator_rx)),
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let acceptor_conn = MockConnection {
            id: connection_id,
            local_id: to,
            remote_id: from,
            network: self.clone(),
            outgoing_streams_tx: acceptor_to_initiator_tx,
            incoming_streams_rx: Arc::new(tokio::sync::Mutex::new(initiator_to_acceptor_rx)),
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        // Create incoming connection for acceptor
        let incoming = MockIncomingConnection {
            connection: acceptor_conn,
            alpn: alpn.to_vec(),
        };

        // Send to acceptor's incoming queue
        incoming_tx.try_send(incoming).map_err(|_| MockConnectionError::EndpointBusy)?;

        Ok(initiator_conn)
    }

    /// Generate a unique stream ID.
    fn next_stream_id(&self) -> u64 {
        self.inner.stream_id_counter.fetch_add(1, Ordering::Relaxed)
    }

    // --- Failure Injection API ---

    /// Simulate a network partition where all connections fail.
    pub fn set_network_partition(&self, enabled: bool) {
        let mut config = self.inner.failure_injection.lock();
        config.network_partition = enabled;
    }

    /// Mark an endpoint as refusing all connections.
    pub fn refuse_connections_to(&self, endpoint: EndpointId) {
        let mut config = self.inner.failure_injection.lock();
        if !config.refused_endpoints.contains(&endpoint) {
            config.refused_endpoints.push(endpoint);
        }
    }

    /// Clear all failure injection configuration.
    #[allow(dead_code)]
    pub fn clear_failures(&self) {
        let mut config = self.inner.failure_injection.lock();
        *config = FailureInjectionConfig::default();
    }

    /// Allow connections to an endpoint that was previously refused.
    pub fn allow_connections_to(&self, endpoint: EndpointId) {
        let mut config = self.inner.failure_injection.lock();
        config.refused_endpoints.retain(|e| *e != endpoint);
    }
}

impl Default for MockIrohNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when establishing mock connections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockConnectionError {
    /// Target endpoint not found in the network.
    EndpointNotFound,
    /// Target endpoint refused the connection.
    ConnectionRefused,
    /// Target endpoint doesn't support the requested ALPN.
    UnsupportedAlpn,
    /// Target endpoint's incoming queue is full.
    EndpointBusy,
    /// Network partition is simulated.
    NetworkPartition,
    /// Connection has been closed.
    ConnectionClosed,
}

impl std::fmt::Display for MockConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndpointNotFound => write!(f, "endpoint not found"),
            Self::ConnectionRefused => write!(f, "connection refused"),
            Self::UnsupportedAlpn => write!(f, "unsupported ALPN protocol"),
            Self::EndpointBusy => write!(f, "endpoint busy"),
            Self::NetworkPartition => write!(f, "network partition"),
            Self::ConnectionClosed => write!(f, "connection closed"),
        }
    }
}

impl std::error::Error for MockConnectionError {}

impl From<MockConnectionError> for io::Error {
    fn from(err: MockConnectionError) -> Self {
        let kind = match err {
            MockConnectionError::EndpointNotFound => io::ErrorKind::NotFound,
            MockConnectionError::ConnectionRefused => io::ErrorKind::ConnectionRefused,
            MockConnectionError::UnsupportedAlpn => io::ErrorKind::InvalidInput,
            MockConnectionError::EndpointBusy => io::ErrorKind::WouldBlock,
            MockConnectionError::NetworkPartition => io::ErrorKind::NetworkUnreachable,
            MockConnectionError::ConnectionClosed => io::ErrorKind::ConnectionAborted,
        };
        io::Error::new(kind, err)
    }
}

/// Mock Iroh endpoint.
///
/// Simulates an Iroh endpoint for testing purposes. Supports:
/// - Initiating connections to other endpoints
/// - Accepting incoming connections
/// - ALPN-based protocol negotiation
pub struct MockEndpoint {
    network: MockIrohNetwork,
    endpoint_id: EndpointId,
    secret_key: SecretKey,
    incoming_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MockIncomingConnection>>>,
}

impl MockEndpoint {
    /// Get this endpoint's public identity.
    pub fn id(&self) -> EndpointId {
        self.endpoint_id
    }

    /// Get this endpoint's secret key.
    #[allow(dead_code)]
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Connect to another endpoint.
    ///
    /// # Arguments
    /// * `target` - The endpoint ID to connect to
    /// * `alpn` - The ALPN protocol to negotiate
    pub async fn connect(&self, target: EndpointId, alpn: &[u8]) -> Result<MockConnection, MockConnectionError> {
        self.network.connect(self.endpoint_id, target, alpn)
    }

    /// Accept the next incoming connection.
    ///
    /// Returns `None` if the endpoint has been closed.
    pub async fn accept(&self) -> Option<MockIncomingConnection> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await
    }

    /// Try to accept an incoming connection without blocking.
    #[allow(dead_code)]
    pub fn try_accept(&self) -> Option<MockIncomingConnection> {
        let mut rx = self.incoming_rx.try_lock().ok()?;
        rx.try_recv().ok()
    }
}

impl Clone for MockEndpoint {
    fn clone(&self) -> Self {
        Self {
            network: self.network.clone(),
            endpoint_id: self.endpoint_id,
            secret_key: self.secret_key.clone(),
            incoming_rx: Arc::clone(&self.incoming_rx),
        }
    }
}

/// Incoming connection waiting to be accepted.
pub struct MockIncomingConnection {
    connection: MockConnection,
    alpn: Vec<u8>,
}

impl MockIncomingConnection {
    /// Get the negotiated ALPN protocol.
    pub fn alpn(&self) -> &[u8] {
        &self.alpn
    }

    /// Get the remote endpoint's ID.
    pub fn remote_id(&self) -> EndpointId {
        self.connection.remote_id
    }

    /// Accept the connection and get the underlying MockConnection.
    pub fn accept(self) -> MockConnection {
        self.connection
    }

    /// Refuse the connection.
    #[allow(dead_code)]
    pub fn refuse(self) {
        // Just drop the connection
        drop(self.connection);
    }
}

/// Mock bidirectional connection.
///
/// Simulates a QUIC connection between two endpoints with support for
/// bidirectional streams.
#[derive(Debug)]
pub struct MockConnection {
    id: u64,
    local_id: EndpointId,
    remote_id: EndpointId,
    network: MockIrohNetwork,
    outgoing_streams_tx: mpsc::Sender<MockStreamPair>,
    incoming_streams_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MockStreamPair>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl MockConnection {
    /// Get the connection ID.
    #[allow(dead_code)]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the local endpoint ID.
    #[allow(dead_code)]
    pub fn local_id(&self) -> EndpointId {
        self.local_id
    }

    /// Get the remote endpoint ID (matches `iroh::endpoint::Connection::remote_id()`).
    pub fn remote_id(&self) -> EndpointId {
        self.remote_id
    }

    /// Check if the connection is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// Open a new bidirectional stream.
    ///
    /// Returns a (SendStream, RecvStream) pair for bidirectional communication.
    pub async fn open_bi(&self) -> Result<(MockSendStream, MockRecvStream), MockConnectionError> {
        if self.is_closed() {
            return Err(MockConnectionError::ConnectionClosed);
        }

        let stream_id = self.network.next_stream_id();

        // Create channels for bidirectional communication
        let (our_to_them_tx, our_to_them_rx) = mpsc::channel(DEFAULT_STREAM_CAPACITY);
        let (them_to_us_tx, them_to_us_rx) = mpsc::channel(DEFAULT_STREAM_CAPACITY);

        // Create stream pair to send to remote
        let remote_pair = MockStreamPair {
            stream_id,
            send_tx: them_to_us_tx,
            recv_rx: our_to_them_rx,
        };

        // Send to remote's incoming streams
        self.outgoing_streams_tx
            .send(remote_pair)
            .await
            .map_err(|_| MockConnectionError::ConnectionClosed)?;

        // Create our streams
        let send_stream = MockSendStream {
            stream_id,
            tx: Some(our_to_them_tx),
        };

        let recv_stream = MockRecvStream {
            stream_id,
            rx: them_to_us_rx,
            buffer: Vec::new(),
        };

        Ok((send_stream, recv_stream))
    }

    /// Accept an incoming bidirectional stream.
    ///
    /// Returns a (SendStream, RecvStream) pair, or an error if the connection is closed.
    pub async fn accept_bi(&self) -> Result<(MockSendStream, MockRecvStream), MockConnectionError> {
        if self.is_closed() {
            return Err(MockConnectionError::ConnectionClosed);
        }

        let mut rx = self.incoming_streams_rx.lock().await;
        let pair = rx.recv().await.ok_or(MockConnectionError::ConnectionClosed)?;

        let send_stream = MockSendStream {
            stream_id: pair.stream_id,
            tx: Some(pair.send_tx),
        };

        let recv_stream = MockRecvStream {
            stream_id: pair.stream_id,
            rx: pair.recv_rx,
            buffer: Vec::new(),
        };

        Ok((send_stream, recv_stream))
    }

    /// Close the connection.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

impl Clone for MockConnection {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            local_id: self.local_id,
            remote_id: self.remote_id,
            network: self.network.clone(),
            outgoing_streams_tx: self.outgoing_streams_tx.clone(),
            incoming_streams_rx: Arc::clone(&self.incoming_streams_rx),
            closed: Arc::clone(&self.closed),
        }
    }
}

/// Internal stream pair for sending to remote.
struct MockStreamPair {
    stream_id: u64,
    send_tx: mpsc::Sender<Bytes>,
    recv_rx: mpsc::Receiver<Bytes>,
}

/// Mock send stream for writing data.
///
/// Simulates `iroh::endpoint::SendStream` for testing.
pub struct MockSendStream {
    stream_id: u64,
    /// Option so we can drop the sender when finish() is called.
    tx: Option<mpsc::Sender<Bytes>>,
}

impl std::fmt::Debug for MockSendStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockSendStream")
            .field("stream_id", &self.stream_id)
            .field("finished", &self.tx.is_none())
            .finish()
    }
}

impl MockSendStream {
    /// Get the stream ID.
    #[allow(dead_code)]
    pub fn stream_id(&self) -> u64 {
        self.stream_id
    }

    /// Write all bytes to the stream.
    ///
    /// Matches `iroh::endpoint::SendStream::write_all()` signature.
    pub async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stream already finished"))?;

        if buf.len() > MAX_MOCK_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("message too large: {} bytes (max {})", buf.len(), MAX_MOCK_MESSAGE_SIZE),
            ));
        }

        tx.send(Bytes::copy_from_slice(buf))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "stream closed"))
    }

    /// Mark the stream as finished (no more data will be sent).
    ///
    /// Matches `iroh::endpoint::SendStream::finish()` signature.
    /// Dropping the sender signals EOF to the receiver.
    pub fn finish(&mut self) -> io::Result<()> {
        if self.tx.is_none() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "stream already finished"));
        }
        // Drop the sender to signal EOF to receiver
        self.tx = None;
        Ok(())
    }

    /// Check if the stream has been finished.
    #[allow(dead_code)]
    pub fn is_finished(&self) -> bool {
        self.tx.is_none()
    }
}

/// Mock receive stream for reading data.
///
/// Simulates `iroh::endpoint::RecvStream` for testing.
pub struct MockRecvStream {
    stream_id: u64,
    rx: mpsc::Receiver<Bytes>,
    buffer: Vec<u8>,
}

impl std::fmt::Debug for MockRecvStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockRecvStream")
            .field("stream_id", &self.stream_id)
            .field("buffer_len", &self.buffer.len())
            .finish()
    }
}

impl MockRecvStream {
    /// Get the stream ID.
    #[allow(dead_code)]
    pub fn stream_id(&self) -> u64 {
        self.stream_id
    }

    /// Read all data until the stream is finished, up to the specified limit.
    ///
    /// Matches `iroh::endpoint::RecvStream::read_to_end()` signature.
    pub async fn read_to_end(&mut self, limit: usize) -> io::Result<Vec<u8>> {
        loop {
            match self.rx.recv().await {
                Some(chunk) => {
                    if self.buffer.len() + chunk.len() > limit {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("data exceeds limit: {} + {} > {}", self.buffer.len(), chunk.len(), limit),
                        ));
                    }
                    self.buffer.extend_from_slice(&chunk);
                }
                None => {
                    // Stream finished (sender dropped or finished)
                    return Ok(std::mem::take(&mut self.buffer));
                }
            }
        }
    }

    /// Read exactly the specified number of bytes.
    #[allow(dead_code)]
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let needed = buf.len();

        // First use any buffered data
        let from_buffer = std::cmp::min(self.buffer.len(), needed);
        if from_buffer > 0 {
            buf[..from_buffer].copy_from_slice(&self.buffer[..from_buffer]);
            self.buffer.drain(..from_buffer);
            if from_buffer == needed {
                return Ok(());
            }
        }

        // Read more from channel
        let mut pos = from_buffer;
        while pos < needed {
            match self.rx.recv().await {
                Some(chunk) => {
                    let to_copy = std::cmp::min(chunk.len(), needed - pos);
                    buf[pos..pos + to_copy].copy_from_slice(&chunk[..to_copy]);
                    pos += to_copy;

                    // Buffer any remaining
                    if to_copy < chunk.len() {
                        self.buffer.extend_from_slice(&chunk[to_copy..]);
                    }
                }
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "stream ended before enough data was read",
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to generate a random secret key.
    fn random_secret_key() -> SecretKey {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        SecretKey::from(bytes)
    }

    #[tokio::test]
    async fn test_mock_network_basic_connection() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint_with_key_and_alpns(random_secret_key(), vec![RAFT_ALPN.to_vec()]);

        // Connect ep1 to ep2
        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
        assert_eq!(conn1.remote_id(), ep2.id());

        // Accept on ep2
        let incoming = ep2.accept().await.unwrap();
        assert_eq!(incoming.remote_id(), ep1.id());
        assert_eq!(incoming.alpn(), RAFT_ALPN);

        let conn2 = incoming.accept();
        assert_eq!(conn2.remote_id(), ep1.id());
    }

    #[tokio::test]
    async fn test_mock_stream_communication() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        // Connect
        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
        let incoming = ep2.accept().await.unwrap();
        let conn2 = incoming.accept();

        // Open stream from conn1
        let (mut send1, _recv1) = conn1.open_bi().await.unwrap();

        // Accept stream on conn2
        let (_send2, mut recv2) = conn2.accept_bi().await.unwrap();

        // Send data
        send1.write_all(b"hello world").await.unwrap();
        send1.finish().unwrap();

        // Receive data
        let data = recv2.read_to_end(1024).await.unwrap();
        assert_eq!(data, b"hello world");
    }

    #[tokio::test]
    async fn test_mock_bidirectional_stream() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        // Connect
        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
        let incoming = ep2.accept().await.unwrap();
        let conn2 = incoming.accept();

        // Open stream from conn1
        let (mut send1, mut recv1) = conn1.open_bi().await.unwrap();

        // Accept stream on conn2
        let (mut send2, mut recv2) = conn2.accept_bi().await.unwrap();

        // Send from conn1 to conn2
        send1.write_all(b"request").await.unwrap();
        send1.finish().unwrap();

        // Receive on conn2
        let request = recv2.read_to_end(1024).await.unwrap();
        assert_eq!(request, b"request");

        // Send response from conn2 to conn1
        send2.write_all(b"response").await.unwrap();
        send2.finish().unwrap();

        // Receive on conn1
        let response = recv1.read_to_end(1024).await.unwrap();
        assert_eq!(response, b"response");
    }

    #[tokio::test]
    async fn test_mock_network_partition() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        // Enable network partition
        network.set_network_partition(true);

        // Connection should fail
        let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
        assert_eq!(result.unwrap_err(), MockConnectionError::NetworkPartition);

        // Disable partition
        network.set_network_partition(false);

        // Connection should succeed now
        let conn = ep1.connect(ep2.id(), RAFT_ALPN).await;
        assert!(conn.is_ok());
    }

    #[tokio::test]
    async fn test_mock_connection_refused() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        // Refuse connections to ep2
        network.refuse_connections_to(ep2.id());

        // Connection should fail
        let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
        assert_eq!(result.unwrap_err(), MockConnectionError::ConnectionRefused);

        // Allow connections again
        network.allow_connections_to(ep2.id());

        // Connection should succeed
        let conn = ep1.connect(ep2.id(), RAFT_ALPN).await;
        assert!(conn.is_ok());
    }

    #[tokio::test]
    async fn test_mock_unsupported_alpn() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        // ep2 only accepts RAFT_ALPN
        let ep2 = network.create_endpoint_with_key_and_alpns(random_secret_key(), vec![RAFT_ALPN.to_vec()]);

        // Try to connect with CLIENT_ALPN (not supported)
        let result = ep1.connect(ep2.id(), CLIENT_ALPN).await;
        assert_eq!(result.unwrap_err(), MockConnectionError::UnsupportedAlpn);

        // Connect with supported ALPN
        let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_stream_size_limit() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
        let (mut send1, _recv1) = conn1.open_bi().await.unwrap();

        // Try to send data exceeding limit
        let large_data = vec![0u8; MAX_MOCK_MESSAGE_SIZE + 1];
        let result = send1.write_all(&large_data).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_connection_closed() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();

        // Close the connection
        conn1.close();

        // Should fail to open stream
        let result = conn1.open_bi().await;
        assert_eq!(result.unwrap_err(), MockConnectionError::ConnectionClosed);
    }

    #[tokio::test]
    async fn test_mock_endpoint_not_found() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let fake_id = random_secret_key().public();

        // Try to connect to non-existent endpoint
        let result = ep1.connect(fake_id, RAFT_ALPN).await;
        assert_eq!(result.unwrap_err(), MockConnectionError::EndpointNotFound);
    }

    #[tokio::test]
    async fn test_mock_multiple_streams() {
        let network = MockIrohNetwork::new();

        let ep1 = network.create_endpoint();
        let ep2 = network.create_endpoint();

        let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
        let incoming = ep2.accept().await.unwrap();
        let conn2 = incoming.accept();

        // Open multiple streams
        let (mut send1a, _) = conn1.open_bi().await.unwrap();
        let (mut send1b, _) = conn1.open_bi().await.unwrap();

        // Accept streams on conn2
        let (_, mut recv2a) = conn2.accept_bi().await.unwrap();
        let (_, mut recv2b) = conn2.accept_bi().await.unwrap();

        // Send different data on each stream
        send1a.write_all(b"stream A").await.unwrap();
        send1a.finish().unwrap();
        send1b.write_all(b"stream B").await.unwrap();
        send1b.finish().unwrap();

        // Receive on each stream
        let data_a = recv2a.read_to_end(1024).await.unwrap();
        let data_b = recv2b.read_to_end(1024).await.unwrap();

        assert_eq!(data_a, b"stream A");
        assert_eq!(data_b, b"stream B");
    }
}
