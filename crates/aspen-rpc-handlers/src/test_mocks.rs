//! Mock implementations for RPC handler testing.
//!
//! Provides mock implementations of the various traits required by `ClientProtocolContext`
//! for use in unit and integration tests. These mocks are designed to be simple, deterministic,
//! and easy to configure for specific test scenarios.
//!
//! # Tiger Style
//!
//! - All mocks are bounded and deterministic
//! - No real network or file I/O
//! - Thread-safe via interior mutability where needed

#![cfg(any(test, feature = "testing"))]

use std::sync::Arc;

use aspen_core::EndpointProvider;
use aspen_core::NetworkFactory;
use aspen_core::WatchInfo;
use aspen_core::WatchRegistry;
use async_trait::async_trait;
use iroh::Endpoint as IrohEndpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;

// =============================================================================
// MockEndpointProvider
// =============================================================================

/// Mock implementation of `EndpointProvider` for testing.
///
/// Provides deterministic responses without real Iroh networking.
/// Creates a real Iroh endpoint for compatibility with handler code that
/// needs to access the endpoint.
///
/// # Example
///
/// ```ignore
/// use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
///
/// #[tokio::test]
/// async fn test_handler() {
///     let provider = MockEndpointProvider::new().await;
///     let peer_id = provider.peer_id().await;
/// }
/// ```
pub struct MockEndpointProvider {
    /// Iroh endpoint for mock network operations.
    endpoint: IrohEndpoint,
    /// Node address for peer discovery.
    node_addr: EndpointAddr,
    /// Public key bytes.
    public_key: Vec<u8>,
    /// Peer ID string.
    peer_id: String,
}

impl MockEndpointProvider {
    /// Create a new mock endpoint provider with a random secret key.
    ///
    /// This creates an isolated Iroh endpoint that won't connect to any real network.
    pub async fn new() -> Self {
        Self::with_seed(0).await
    }

    /// Create a mock endpoint provider with a deterministic seed.
    ///
    /// Using the same seed will produce the same node identity, useful for
    /// reproducible tests.
    pub async fn with_seed(seed: u64) -> Self {
        // Generate deterministic secret key from seed
        let mut key_bytes = [0u8; 32];
        key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
        let secret_key = SecretKey::from_bytes(&key_bytes);

        // Build endpoint without discovery (isolated)
        let endpoint = iroh::Endpoint::builder()
            .secret_key(secret_key.clone())
            .bind_addr_v4("127.0.0.1:0".parse().unwrap())
            .bind()
            .await
            .expect("failed to create mock endpoint");

        let node_addr = endpoint.addr();
        let public_key = secret_key.public().as_bytes().to_vec();
        let peer_id = node_addr.id.fmt_short().to_string();

        Self {
            endpoint,
            node_addr,
            public_key,
            peer_id,
        }
    }

    /// Create a mock endpoint provider for a specific node ID.
    ///
    /// The seed is derived from the node ID for deterministic identity.
    pub async fn for_node(node_id: u64) -> Self {
        Self::with_seed(node_id * 1000).await
    }
}

#[async_trait]
impl EndpointProvider for MockEndpointProvider {
    async fn public_key(&self) -> Vec<u8> {
        self.public_key.clone()
    }

    async fn peer_id(&self) -> String {
        self.peer_id.clone()
    }

    async fn addresses(&self) -> Vec<String> {
        vec!["127.0.0.1:0".to_string()]
    }

    fn node_addr(&self) -> &EndpointAddr {
        &self.node_addr
    }

    fn endpoint(&self) -> &IrohEndpoint {
        &self.endpoint
    }
}

// =============================================================================
// MockNetworkFactory
// =============================================================================

/// Mock implementation of `NetworkFactory` for testing.
///
/// Tracks peer additions/removals without actual network operations.
pub struct MockNetworkFactory {
    /// Peers that have been added.
    peers: std::sync::Mutex<std::collections::HashMap<u64, String>>,
}

impl MockNetworkFactory {
    /// Create a new mock network factory.
    pub fn new() -> Self {
        Self {
            peers: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Get the current set of registered peers.
    pub fn peers(&self) -> std::collections::HashMap<u64, String> {
        self.peers.lock().unwrap().clone()
    }

    /// Check if a peer is registered.
    pub fn has_peer(&self, node_id: u64) -> bool {
        self.peers.lock().unwrap().contains_key(&node_id)
    }
}

impl Default for MockNetworkFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkFactory for MockNetworkFactory {
    async fn add_peer(&self, node_id: u64, address: String) -> Result<(), String> {
        self.peers.lock().unwrap().insert(node_id, address);
        Ok(())
    }

    async fn remove_peer(&self, node_id: u64) -> Result<(), String> {
        self.peers.lock().unwrap().remove(&node_id);
        Ok(())
    }
}

// =============================================================================
// MockWatchRegistry
// =============================================================================

/// Mock implementation of `WatchRegistry` for testing.
///
/// Tracks watch registrations in memory for verification.
pub struct MockWatchRegistry {
    /// Active watches indexed by ID.
    watches: std::sync::RwLock<std::collections::HashMap<u64, WatchInfo>>,
    /// Next watch ID to assign.
    next_id: std::sync::atomic::AtomicU64,
}

impl MockWatchRegistry {
    /// Create a new mock watch registry.
    pub fn new() -> Self {
        Self {
            watches: std::sync::RwLock::new(std::collections::HashMap::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Get the number of registered watches.
    pub fn len(&self) -> usize {
        self.watches.read().unwrap().len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.watches.read().unwrap().is_empty()
    }
}

impl Default for MockWatchRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WatchRegistry for MockWatchRegistry {
    async fn get_all_watches(&self) -> Vec<WatchInfo> {
        self.watches.read().unwrap().values().cloned().collect()
    }

    async fn get_watch(&self, watch_id: u64) -> Option<WatchInfo> {
        self.watches.read().unwrap().get(&watch_id).cloned()
    }

    async fn watch_count(&self) -> usize {
        self.watches.read().unwrap().len()
    }

    fn register_watch(&self, prefix: String, include_prev_value: bool) -> u64 {
        let watch_id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let watch_info = WatchInfo {
            watch_id,
            prefix,
            include_prev_value,
            last_sent_index: 0,
            events_sent: 0,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };
        self.watches.write().unwrap().insert(watch_id, watch_info);
        watch_id
    }

    fn update_watch(&self, watch_id: u64, last_sent_index: u64, events_sent: u64) {
        if let Some(watch) = self.watches.write().unwrap().get_mut(&watch_id) {
            watch.last_sent_index = last_sent_index;
            watch.events_sent = events_sent;
        }
    }

    fn unregister_watch(&self, watch_id: u64) {
        self.watches.write().unwrap().remove(&watch_id);
    }
}

// =============================================================================
// MockSqlExecutor
// =============================================================================

/// Mock implementation of `SqlQueryExecutor` for testing.
///
/// Returns empty results for all queries. Used when the sql feature is enabled
/// but no real SQL queries need to be executed.
#[cfg(feature = "sql")]
pub struct MockSqlExecutor;

#[cfg(feature = "sql")]
impl MockSqlExecutor {
    /// Create a new mock SQL executor.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "sql")]
impl Default for MockSqlExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "sql")]
#[async_trait]
impl aspen_sql::SqlQueryExecutor for MockSqlExecutor {
    async fn execute_sql(
        &self,
        _request: aspen_sql::SqlQueryRequest,
    ) -> Result<aspen_sql::SqlQueryResult, aspen_sql::SqlQueryError> {
        Ok(aspen_sql::SqlQueryResult {
            columns: vec![],
            rows: vec![],
            row_count: 0,
            is_truncated: false,
            execution_time_ms: 0,
        })
    }
}

/// Create a mock SQL executor wrapped in Arc.
#[cfg(feature = "sql")]
pub fn mock_sql_executor() -> Arc<MockSqlExecutor> {
    Arc::new(MockSqlExecutor::new())
}

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a mock endpoint provider wrapped in Arc for use in tests.
pub async fn mock_endpoint() -> Arc<MockEndpointProvider> {
    Arc::new(MockEndpointProvider::new().await)
}

/// Create a mock endpoint provider for a specific node ID.
pub async fn mock_endpoint_for_node(node_id: u64) -> Arc<MockEndpointProvider> {
    Arc::new(MockEndpointProvider::for_node(node_id).await)
}

/// Create a mock network factory wrapped in Arc.
pub fn mock_network_factory() -> Arc<MockNetworkFactory> {
    Arc::new(MockNetworkFactory::new())
}

/// Create a mock watch registry wrapped in Arc.
pub fn mock_watch_registry() -> Arc<MockWatchRegistry> {
    Arc::new(MockWatchRegistry::new())
}

// =============================================================================
// Tests for Mocks
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_endpoint_provider_peer_id() {
        let provider = MockEndpointProvider::new().await;
        let peer_id = provider.peer_id().await;
        assert!(!peer_id.is_empty(), "peer_id should not be empty");
    }

    #[tokio::test]
    async fn test_mock_endpoint_provider_deterministic() {
        let provider1 = MockEndpointProvider::with_seed(42).await;
        let provider2 = MockEndpointProvider::with_seed(42).await;

        assert_eq!(provider1.peer_id().await, provider2.peer_id().await, "same seed should produce same peer_id");
    }

    #[tokio::test]
    async fn test_mock_network_factory_add_remove() {
        let factory = MockNetworkFactory::new();

        factory.add_peer(1, "127.0.0.1:9000".to_string()).await.unwrap();
        assert!(factory.has_peer(1));

        factory.remove_peer(1).await.unwrap();
        assert!(!factory.has_peer(1));
    }

    #[tokio::test]
    async fn test_mock_watch_registry_lifecycle() {
        let registry = MockWatchRegistry::new();

        let watch_id = registry.register_watch("test:".to_string(), false);
        assert_eq!(registry.watch_count().await, 1);

        registry.update_watch(watch_id, 100, 10);
        let watch = registry.get_watch(watch_id).await.unwrap();
        assert_eq!(watch.last_sent_index, 100);
        assert_eq!(watch.events_sent, 10);

        registry.unregister_watch(watch_id);
        assert_eq!(registry.watch_count().await, 0);
    }
}
