//! This module provides distributed primitives using `iroh` and `hiqlite`.
//! It aims to handle distributed state and communication for the Aspen project.

use anyhow::Result;
use iroh::Endpoint;
use hiqlite::{Client as Hiqlite, NodeConfig, Node, start_node_with_cache, LogSync, Lock};
use hiqlite_macros::params;
use strum::EnumIter;
use hiqlite::cache_idx::CacheIndex;
use tokio::fs;
use std::borrow::Cow;
use cryptr::EncKeys;
use uuid::Uuid;


/// A client for interacting with the distributed system.
pub struct DistributedClient {
    _iroh_node: Endpoint,
    hiqlite_client: Hiqlite,
}

#[derive(Debug, EnumIter)]
enum Cache {
    One,
}

impl CacheIndex for Cache {
    fn to_usize(self) -> usize {
        self as usize
    }
}

use anyhow::anyhow;
use portpicker;

// ... (other imports)

impl DistributedClient {
    /// Creates a new `DistributedClient`.
    pub async fn new() -> Result<Self> {
        // Placeholder for iroh node creation
        let _iroh_node = Endpoint::bind().await?;

        // Configure Hiqlite node
        let node_id = 1;

        let api_port = portpicker::pick_unused_port().ok_or_else(|| anyhow!("No unused API port found"))?;
        let raft_port = portpicker::pick_unused_port().ok_or_else(|| anyhow!("No unused RAFT port found"))?;

        let nodes = vec![
            Node {
                id: 1,
                addr_api: format!("127.0.0.1:{}", api_port),
                addr_raft: format!("127.0.0.1:{}", raft_port),
            },
        ];

        let enc_keys = EncKeys {
            enc_key_active: "default".to_string(),
            enc_keys: vec![("default".to_string(), "01234567890123456789012345678901".as_bytes().to_vec())], // 32-byte key
        };

        let unique_id = Uuid::new_v4().to_string();
        let data_dir = format!("/tmp/hiqlite_data_{}_{}", node_id, unique_id);

        let config = NodeConfig {
            node_id,
            nodes,
            listen_addr_api: Cow::Owned(format!("127.0.0.1:{}", api_port)),
            listen_addr_raft: Cow::Owned(format!("127.0.0.1:{}", raft_port)),
            data_dir: data_dir.into(),
            filename_db: Cow::Borrowed("hiqlite.db"),
            log_statements: false,
            prepared_statement_cache_capacity: 1024,
            read_pool_size: 4,
            wal_sync: LogSync::ImmediateAsync,
            wal_size: 2 * 1024 * 1024,
            cache_storage_disk: true,
            raft_config: NodeConfig::default_raft_config(10_000),
            tls_raft: None,
            tls_api: None,
            secret_raft: "SuperSecretRaftKey123".to_string(), // Must be at least 16 chars
            secret_api: "SuperSecretApiKey123".to_string(),   // Must be at least 16 chars
            enc_keys,
            shutdown_delay_millis: 5000,
            health_check_delay_secs: 30,
            ..Default::default() // Fill in remaining fields with default values
        };

        let _ = fs::remove_dir_all(&*config.data_dir).await; // Clean up previous data

        // Initialize the Hiqlite client
        let hiqlite_client = start_node_with_cache::<Cache>(config).await?;

        Ok(Self {
            _iroh_node,
            hiqlite_client,
        })
    }

    /// Placeholder for a distributed communication method.
    pub async fn send_message(&self, message: String) -> Result<()> {
        // In a real implementation, this would use iroh for communication
        println!("Sending distributed message: {}", message);
        Ok(())
    }

    /// Placeholder for a distributed state storage method.
    pub async fn store_state(&self, key: String, value: String) -> Result<()> {
        // In a real implementation, this would use hiqlite for state storage
        self.hiqlite_client.execute(
            "CREATE TABLE IF NOT EXISTS state (key TEXT PRIMARY KEY, value TEXT)",
            params!(),
        ).await?;
        self.hiqlite_client.execute(
            "INSERT OR REPLACE INTO state (key, value) VALUES (?, ?)",
            params!(key.clone(), value.clone()),
        ).await?;
        println!("Storing distributed state: {} = {}", key, value);
        Ok(())
    }

    /// Acquires a distributed lock.
    pub async fn get_distributed_lock<K>(&self, key: K) -> Result<Lock>
    where
        K: Into<Cow<'static, str>>,
    {
        self.hiqlite_client.lock(key).await.map_err(anyhow::Error::from)
    }

    /// Retrieves a distributed state.
    pub async fn get_state(&self, key: String) -> Result<Option<String>> {
        let rows = self.hiqlite_client.query_as::<String, _>(
            "SELECT value FROM state WHERE key = ?",
            params!(key.clone()),
        ).await?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_client_creation() {
        let client = DistributedClient::new().await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_send_message() {
        let client = DistributedClient::new().await.unwrap();
        let result = client.send_message("Hello, distributed world!".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_store_state() {
        let client = DistributedClient::new().await.unwrap();
        let result = client.store_state("test_key".to_string(), "test_value".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_distributed_lock() {
        let client = DistributedClient::new().await.unwrap();
        let lock_key = "my_test_lock".to_string();
        let lock = client.get_distributed_lock(lock_key.clone()).await;
        assert!(lock.is_ok());
        // The lock is automatically released when it goes out of scope
    }

    // Proptests
    use proptest::prelude::*;
    use proptest::test_runner::TestRunner;

    #[test]
    fn proptest_store_and_get_state_performance() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = rt.block_on(async {
            DistributedClient::new().await.unwrap()
        });

        let mut runner = TestRunner::default();
        let strategy = any::<(String, String)>();

        runner.run(&strategy, |(key, value)| {
            rt.block_on(async {
                client.store_state(key.clone(), value.clone()).await.unwrap();
                let retrieved_value = client.get_state(key.clone()).await.unwrap();
                prop_assert_eq!(retrieved_value, Some(value));
                Ok(())
            })
        }).unwrap();
    }
}