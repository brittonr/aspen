//! Test fixtures for aspen-cluster integration tests.
//!
//! Provides builders and utilities for creating test clusters.

use std::time::Duration;

use aspen_core::context::EndpointAddr;
use aspen_core::context::EndpointProvider;
use aspen_core::context::IrohEndpoint;
use async_trait::async_trait;
use iroh::SecretKey;
use tempfile::TempDir;

/// Mock endpoint provider for tests.
///
/// Creates an isolated Iroh endpoint that won't connect to any real network.
pub struct MockEndpointProvider {
    endpoint: IrohEndpoint,
    node_addr: EndpointAddr,
    public_key: Vec<u8>,
    peer_id: String,
}

impl MockEndpointProvider {
    /// Create a new mock endpoint provider.
    pub async fn new() -> Self {
        Self::with_seed(0).await
    }

    /// Create a mock endpoint provider with a deterministic seed.
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
    #[allow(dead_code)]
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

/// Create a temporary directory for test data.
#[allow(dead_code)]
pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

/// Wait for a condition with timeout.
pub async fn wait_for<F, Fut>(timeout: Duration, mut condition: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if condition().await {
            return true;
        }
        if tokio::time::Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
