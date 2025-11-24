//! Mock implementations of service traits for testing
//!
//! Provides in-memory mock implementations that follow the established
//! testing patterns (Arc<Mutex<T>> for state management).

#![allow(dead_code)] // Mocks are for future tests

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::traits::*;
use crate::domain::types::HealthStatus;

// =============================================================================
// IROH NETWORKING MOCKS
// =============================================================================

/// Mock implementation of EndpointInfo for testing
#[derive(Clone)]
pub struct MockEndpointInfo {
    endpoint_id: EndpointId,
    local_endpoints: Arc<Mutex<Vec<String>>>,
}

impl MockEndpointInfo {
    /// Create a new mock with a generated endpoint ID
    pub fn new() -> Self {
        // Generate a zero endpoint ID for testing
        // EndpointId::from_bytes returns Result, so we need to handle it
        let endpoint_id = EndpointId::from_bytes(&[0u8; 32])
            .expect("Valid zero endpoint ID");

        Self {
            endpoint_id,
            local_endpoints: Arc::new(Mutex::new(vec![
                "127.0.0.1:11204".to_string(),
                "[::1]:11204".to_string(),
            ])),
        }
    }

    /// Set local endpoints for testing
    pub async fn set_local_endpoints(&self, endpoints: Vec<String>) {
        *self.local_endpoints.lock().await = endpoints;
    }
}

impl EndpointInfo for MockEndpointInfo {
    fn endpoint_id(&self) -> EndpointId {
        self.endpoint_id
    }

    fn endpoint_addr(&self) -> EndpointAddr {
        // Create a minimal endpoint address for testing
        // Note: This creates a default address without relay info
        EndpointAddr::new(self.endpoint_id)
    }

    fn local_endpoints(&self) -> Vec<String> {
        // Block on async to satisfy trait signature
        // This is acceptable in tests
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.local_endpoints.lock().await.clone()
            })
        })
    }
}

/// Mock implementation of BlobStorage for testing
#[derive(Clone)]
pub struct MockBlobStorage {
    blobs: Arc<Mutex<HashMap<String, Bytes>>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockBlobStorage {
    pub fn new() -> Self {
        Self {
            blobs: Arc::new(Mutex::new(HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }

    /// Pre-populate a blob for testing retrieval
    pub async fn add_blob(&self, hash: String, data: Bytes) {
        self.blobs.lock().await.insert(hash, data);
    }

    /// Get all stored blobs (for test verification)
    pub async fn get_all_blobs(&self) -> HashMap<String, Bytes> {
        self.blobs.lock().await.clone()
    }
}

#[async_trait]
impl BlobStorage for MockBlobStorage {
    async fn store_blob(&self, data: Bytes) -> Result<String> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock blob storage failure");
        }

        // Simple hash: use length and first few bytes
        let hash = format!("mock_hash_{}", data.len());
        self.blobs.lock().await.insert(hash.clone(), data);
        Ok(hash)
    }

    async fn retrieve_blob(&self, hash: &str) -> Result<Bytes> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock blob retrieval failure");
        }

        self.blobs
            .lock()
            .await
            .get(hash)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Blob not found: {}", hash))
    }
}

/// Mock implementation of GossipNetwork for testing
#[derive(Clone)]
pub struct MockGossipNetwork {
    joined_topics: Arc<Mutex<Vec<String>>>,
    broadcast_messages: Arc<Mutex<Vec<(String, Bytes)>>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockGossipNetwork {
    pub fn new() -> Self {
        Self {
            joined_topics: Arc::new(Mutex::new(Vec::new())),
            broadcast_messages: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }

    /// Get all joined topics (for test verification)
    pub async fn get_joined_topics(&self) -> Vec<String> {
        self.joined_topics.lock().await.clone()
    }

    /// Get all broadcast messages (for test verification)
    pub async fn get_broadcast_messages(&self) -> Vec<(String, Bytes)> {
        self.broadcast_messages.lock().await.clone()
    }
}

#[async_trait]
impl GossipNetwork for MockGossipNetwork {
    async fn join_topic(&self, topic_id: &str) -> Result<()> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock gossip join failure");
        }

        self.joined_topics.lock().await.push(topic_id.to_string());
        Ok(())
    }

    async fn broadcast_message(&self, topic_id: &str, message: Bytes) -> Result<()> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock gossip broadcast failure");
        }

        self.broadcast_messages
            .lock()
            .await
            .push((topic_id.to_string(), message));
        Ok(())
    }
}

/// Mock implementation of PeerConnection for testing
#[derive(Clone)]
pub struct MockPeerConnection {
    connected_peers: Arc<Mutex<Vec<String>>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockPeerConnection {
    pub fn new() -> Self {
        Self {
            connected_peers: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }

    /// Get all connected peers (for test verification)
    pub async fn get_connected_peers(&self) -> Vec<String> {
        self.connected_peers.lock().await.clone()
    }
}

#[async_trait]
impl PeerConnection for MockPeerConnection {
    async fn connect_peer(&self, endpoint_addr_str: &str) -> Result<()> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock peer connection failure");
        }

        self.connected_peers
            .lock()
            .await
            .push(endpoint_addr_str.to_string());
        Ok(())
    }
}

// =============================================================================
// HIQLITE DATABASE MOCKS
// =============================================================================

/// Mock implementation of DatabaseQueries for testing
#[derive(Clone)]
pub struct MockDatabaseQueries {
    execute_count: Arc<Mutex<usize>>,
    query_results: Arc<Mutex<Vec<String>>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockDatabaseQueries {
    pub fn new() -> Self {
        Self {
            execute_count: Arc::new(Mutex::new(0)),
            query_results: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }

    /// Get execute call count (for test verification)
    pub async fn get_execute_count(&self) -> usize {
        *self.execute_count.lock().await
    }

    /// Set query results for testing
    pub async fn set_query_results(&self, results: Vec<String>) {
        *self.query_results.lock().await = results;
    }
}

#[async_trait]
impl DatabaseQueries for MockDatabaseQueries {
    async fn execute(
        &self,
        _query: impl Into<Cow<'static, str>> + Send,
        _params: Vec<Param>,
    ) -> Result<usize> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock database execute failure");
        }

        let mut count = self.execute_count.lock().await;
        *count += 1;
        Ok(1) // Return 1 row affected
    }

    async fn query_as<T>(
        &self,
        _query: impl Into<Cow<'static, str>> + Send,
        _params: Vec<Param>,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock database query failure");
        }

        // Return empty results for now
        // In real tests, you'd use set_query_results() and deserialize
        Ok(Vec::new())
    }

    async fn query_as_one<T>(
        &self,
        _query: impl Into<Cow<'static, str>> + Send,
        _params: Vec<Param>,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock database query_one failure");
        }

        anyhow::bail!("No mock result configured")
    }
}

/// Mock implementation of DatabaseHealth for testing
#[derive(Clone)]
pub struct MockDatabaseHealth {
    health: Arc<Mutex<HealthStatus>>,
}

impl MockDatabaseHealth {
    pub fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(HealthStatus {
                is_healthy: true,
                node_count: 1,
                has_leader: true,
            })),
        }
    }

    /// Set cluster health for testing
    pub async fn set_health(&self, health: HealthStatus) {
        *self.health.lock().await = health;
    }
}

#[async_trait]
impl DatabaseHealth for MockDatabaseHealth {
    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(self.health.lock().await.clone())
    }
}

/// Mock implementation of DatabaseSchema for testing
#[derive(Clone)]
pub struct MockDatabaseSchema {
    initialized: Arc<Mutex<bool>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockDatabaseSchema {
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(Mutex::new(false)),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if schema was initialized (for test verification)
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.lock().await
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }
}

#[async_trait]
impl DatabaseSchema for MockDatabaseSchema {
    async fn initialize_schema(&self) -> Result<()> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock schema initialization failure");
        }

        *self.initialized.lock().await = true;
        Ok(())
    }
}

/// Mock implementation of DatabaseLifecycle for testing
#[derive(Clone)]
pub struct MockDatabaseLifecycle {
    shutdown_called: Arc<Mutex<bool>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockDatabaseLifecycle {
    pub fn new() -> Self {
        Self {
            shutdown_called: Arc::new(Mutex::new(false)),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if shutdown was called (for test verification)
    pub async fn was_shutdown_called(&self) -> bool {
        *self.shutdown_called.lock().await
    }

    /// Set whether operations should fail (for error testing)
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }
}

#[async_trait]
impl DatabaseLifecycle for MockDatabaseLifecycle {
    async fn shutdown(&self) -> Result<()> {
        if *self.should_fail.lock().await {
            anyhow::bail!("Mock shutdown failure");
        }

        *self.shutdown_called.lock().await = true;
        Ok(())
    }
}
