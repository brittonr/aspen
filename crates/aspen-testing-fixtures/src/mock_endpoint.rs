//! Mock endpoint provider for testing Iroh-based networking.
//!
//! This module provides a mock implementation of endpoint providers
//! for testing code that depends on Iroh networking without requiring
//! actual network connections.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

/// A mock endpoint provider for testing.
///
/// Simulates Iroh endpoint behavior without actual network connections.
/// Useful for testing code that needs to work with endpoint addresses
/// but doesn't need real networking.
///
/// # Example
///
/// ```ignore
/// let provider = MockEndpointProvider::new();
///
/// // Register a mock node
/// provider.register_node(1, "mock-endpoint-1").await;
///
/// // Get the endpoint address
/// let addr = provider.get_endpoint(1).await.unwrap();
/// assert_eq!(addr, "mock-endpoint-1");
/// ```
#[derive(Clone, Default)]
pub struct MockEndpointProvider {
    /// Registered node endpoints.
    endpoints: Arc<RwLock<HashMap<u64, String>>>,
    /// Whether the provider is in a "connected" state.
    connected: Arc<RwLock<bool>>,
}

impl MockEndpointProvider {
    /// Create a new mock endpoint provider.
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(true)),
        }
    }

    /// Register a node with a mock endpoint address.
    pub async fn register_node(&self, node_id: u64, endpoint: impl Into<String>) {
        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(node_id, endpoint.into());
    }

    /// Unregister a node.
    pub async fn unregister_node(&self, node_id: u64) -> Option<String> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.remove(&node_id)
    }

    /// Get the endpoint address for a node.
    pub async fn get_endpoint(&self, node_id: u64) -> Option<String> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(&node_id).cloned()
    }

    /// Get all registered endpoints.
    pub async fn all_endpoints(&self) -> HashMap<u64, String> {
        let endpoints = self.endpoints.read().await;
        endpoints.clone()
    }

    /// Check if a node is registered.
    pub async fn is_registered(&self, node_id: u64) -> bool {
        let endpoints = self.endpoints.read().await;
        endpoints.contains_key(&node_id)
    }

    /// Get the number of registered nodes.
    pub async fn node_count(&self) -> usize {
        let endpoints = self.endpoints.read().await;
        endpoints.len()
    }

    /// Set the connected state.
    ///
    /// When disconnected, operations that require connectivity may fail.
    pub async fn set_connected(&self, connected: bool) {
        let mut state = self.connected.write().await;
        *state = connected;
    }

    /// Check if the provider is connected.
    pub async fn is_connected(&self) -> bool {
        let state = self.connected.read().await;
        *state
    }

    /// Clear all registered endpoints.
    pub async fn clear(&self) {
        let mut endpoints = self.endpoints.write().await;
        endpoints.clear();
    }

    /// Register multiple nodes at once.
    pub async fn register_nodes(&self, nodes: impl IntoIterator<Item = (u64, String)>) {
        let mut endpoints = self.endpoints.write().await;
        for (id, endpoint) in nodes {
            endpoints.insert(id, endpoint);
        }
    }

    /// Generate a deterministic mock endpoint address for a node ID.
    ///
    /// This is useful when you need consistent mock addresses across tests.
    pub fn generate_mock_address(node_id: u64) -> String {
        format!("mock://node-{}/endpoint", node_id)
    }

    /// Register a node with a deterministically generated address.
    pub async fn register_with_generated_address(&self, node_id: u64) -> String {
        let address = Self::generate_mock_address(node_id);
        self.register_node(node_id, &address).await;
        address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_creation() {
        let provider = MockEndpointProvider::new();
        assert_eq!(provider.node_count().await, 0);
        assert!(provider.is_connected().await);
    }

    #[tokio::test]
    async fn test_register_and_get_endpoint() {
        let provider = MockEndpointProvider::new();

        provider.register_node(1, "endpoint-1").await;
        provider.register_node(2, "endpoint-2").await;

        assert_eq!(provider.get_endpoint(1).await, Some("endpoint-1".to_string()));
        assert_eq!(provider.get_endpoint(2).await, Some("endpoint-2".to_string()));
        assert_eq!(provider.get_endpoint(3).await, None);
    }

    #[tokio::test]
    async fn test_unregister_node() {
        let provider = MockEndpointProvider::new();

        provider.register_node(1, "endpoint-1").await;
        assert!(provider.is_registered(1).await);

        let removed = provider.unregister_node(1).await;
        assert_eq!(removed, Some("endpoint-1".to_string()));
        assert!(!provider.is_registered(1).await);

        // Unregistering again returns None
        let removed_again = provider.unregister_node(1).await;
        assert_eq!(removed_again, None);
    }

    #[tokio::test]
    async fn test_all_endpoints() {
        let provider = MockEndpointProvider::new();

        provider.register_node(1, "endpoint-1").await;
        provider.register_node(2, "endpoint-2").await;

        let all = provider.all_endpoints().await;
        assert_eq!(all.len(), 2);
        assert_eq!(all.get(&1), Some(&"endpoint-1".to_string()));
        assert_eq!(all.get(&2), Some(&"endpoint-2".to_string()));
    }

    #[tokio::test]
    async fn test_connected_state() {
        let provider = MockEndpointProvider::new();

        assert!(provider.is_connected().await);

        provider.set_connected(false).await;
        assert!(!provider.is_connected().await);

        provider.set_connected(true).await;
        assert!(provider.is_connected().await);
    }

    #[tokio::test]
    async fn test_clear() {
        let provider = MockEndpointProvider::new();

        provider.register_node(1, "endpoint-1").await;
        provider.register_node(2, "endpoint-2").await;
        assert_eq!(provider.node_count().await, 2);

        provider.clear().await;
        assert_eq!(provider.node_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_nodes_bulk() {
        let provider = MockEndpointProvider::new();

        let nodes = vec![
            (1, "endpoint-1".to_string()),
            (2, "endpoint-2".to_string()),
            (3, "endpoint-3".to_string()),
        ];

        provider.register_nodes(nodes).await;
        assert_eq!(provider.node_count().await, 3);
    }

    #[test]
    fn test_generate_mock_address() {
        let addr1 = MockEndpointProvider::generate_mock_address(1);
        let addr2 = MockEndpointProvider::generate_mock_address(2);

        assert_eq!(addr1, "mock://node-1/endpoint");
        assert_eq!(addr2, "mock://node-2/endpoint");

        // Same ID generates same address (deterministic)
        let addr1_again = MockEndpointProvider::generate_mock_address(1);
        assert_eq!(addr1, addr1_again);
    }

    #[tokio::test]
    async fn test_register_with_generated_address() {
        let provider = MockEndpointProvider::new();

        let addr = provider.register_with_generated_address(5).await;
        assert_eq!(addr, "mock://node-5/endpoint");
        assert_eq!(provider.get_endpoint(5).await, Some(addr));
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let provider1 = MockEndpointProvider::new();
        let provider2 = provider1.clone();

        provider1.register_node(1, "endpoint-1").await;

        // Cloned provider should see the same data
        assert_eq!(provider2.get_endpoint(1).await, Some("endpoint-1".to_string()));
        assert_eq!(provider2.node_count().await, 1);
    }
}
