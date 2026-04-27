//! Unit tests for the NodeBuilder.

#[cfg(all(test, feature = "node-runtime"))]
use tempfile::TempDir;

#[cfg(test)]
use super::*;
#[cfg(test)]
use crate::raft::BatchConfig;
#[cfg(test)]
use crate::raft::storage::StorageBackend;

#[test]
fn test_nodebuilder_new_creates_with_defaults() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    let builder = NodeBuilder::new(node_id, data_dir);

    // Verify defaults
    assert_eq!(builder.config.node_id, 1);
    assert_eq!(builder.config.data_dir, Some(PathBuf::from(data_dir)));
    // Default storage backend should be Redb (the current default)
    assert_eq!(builder.config.storage_backend, StorageBackend::Redb);
    // Default cookie should be "aspen-cookie-UNSAFE-CHANGE-ME"
    assert_eq!(builder.config.cookie, "aspen-cookie-UNSAFE-CHANGE-ME");
}

#[test]
fn test_nodebuilder_with_storage_backends() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    // Test InMemory backend
    let builder = NodeBuilder::new(node_id, data_dir).with_storage(StorageBackend::InMemory);
    assert_eq!(builder.config.storage_backend, StorageBackend::InMemory);

    // Test Redb backend (the only persistent backend)
    let builder = NodeBuilder::new(node_id, data_dir).with_storage(StorageBackend::Redb);
    assert_eq!(builder.config.storage_backend, StorageBackend::Redb);

    // Test InMemory with different builder instance
    let builder2 = NodeBuilder::new(node_id, data_dir).with_storage(StorageBackend::InMemory);
    assert_eq!(builder2.config.storage_backend, StorageBackend::InMemory);
}

#[test]
fn test_nodebuilder_with_peers() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let peers = vec![
        "2@endpoint123:relay.example.com:127.0.0.1:8081".to_string(),
        "3@endpoint456:relay.example.com:127.0.0.1:8082".to_string(),
    ];

    let builder = NodeBuilder::new(node_id, data_dir).with_peers(peers.clone());

    assert_eq!(builder.config.peers, peers);
}

#[test]
fn test_nodebuilder_with_gossip() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    // Test enabling gossip
    let builder = NodeBuilder::new(node_id, data_dir).with_gossip(true);
    assert!(builder.config.iroh.enable_gossip);

    // Test disabling gossip
    let builder = NodeBuilder::new(node_id, data_dir).with_gossip(false);
    assert!(!builder.config.iroh.enable_gossip);
}

#[test]
fn test_nodebuilder_with_mdns() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    // Test enabling mDNS
    let builder = NodeBuilder::new(node_id, data_dir).with_mdns(true);
    assert!(builder.config.iroh.enable_mdns);

    // Test disabling mDNS
    let builder = NodeBuilder::new(node_id, data_dir).with_mdns(false);
    assert!(!builder.config.iroh.enable_mdns);
}

#[test]
fn test_nodebuilder_with_heartbeat_interval() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let interval_ms = 500u64;

    let builder = NodeBuilder::new(node_id, data_dir).with_heartbeat_interval_ms(interval_ms);

    assert_eq!(builder.config.heartbeat_interval_ms, interval_ms);
}

#[test]
fn test_nodebuilder_with_election_timeout() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let min_ms = 1000u64;
    let max_ms = 2000u64;

    let builder = NodeBuilder::new(node_id, data_dir).with_election_timeout_ms(min_ms, max_ms);

    assert_eq!(builder.config.election_timeout_min_ms, min_ms);
    assert_eq!(builder.config.election_timeout_max_ms, max_ms);
}

#[test]
fn test_nodebuilder_with_cookie() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let cookie = "test-cookie-123";

    let builder = NodeBuilder::new(node_id, data_dir).with_cookie(cookie);

    assert_eq!(builder.config.cookie, cookie);
}

#[test]
fn test_nodebuilder_with_iroh_secret_key() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    // 64 hex characters = 32 bytes
    let secret_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    let builder = NodeBuilder::new(node_id, data_dir).with_iroh_secret_key(secret_key);

    assert_eq!(builder.config.iroh.secret_key, Some(secret_key.to_string()));
}

#[test]
fn test_nodebuilder_with_gossip_ticket() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let ticket = "aspentktu4u5ars6t7sumdgqcxojisnnmaxr35r532tiyo63gbqibq7h7dmzacaifadiambqaaaaaaaaaaaaai";

    let builder = NodeBuilder::new(node_id, data_dir).with_gossip_ticket(ticket);

    assert_eq!(builder.config.iroh.gossip_ticket, Some(ticket.to_string()));
}

#[test]
fn test_nodebuilder_with_dns_discovery() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    let builder = NodeBuilder::new(node_id, data_dir).with_dns_discovery(true);

    assert!(builder.config.iroh.enable_dns_discovery);
}

#[test]
fn test_nodebuilder_with_pkarr() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    let builder = NodeBuilder::new(node_id, data_dir).with_pkarr(true);

    assert!(builder.config.iroh.enable_pkarr);
}

#[test]
fn test_nodebuilder_with_write_batching() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let batch_config = BatchConfig {
        max_entries: 100,
        max_bytes: 1024 * 1024, // 1MB
        max_wait_ms: 100,
        max_wait: std::time::Duration::from_millis(100),
    };

    let builder = NodeBuilder::new(node_id, data_dir).with_write_batching(batch_config.clone());

    assert!(builder.config.batch_config.is_some());
    let config = builder.config.batch_config.unwrap();
    assert_eq!(config.max_entries, batch_config.max_entries);
    assert_eq!(config.max_bytes, batch_config.max_bytes);
    assert_eq!(config.max_wait_ms, batch_config.max_wait_ms);
}

#[test]
fn test_nodebuilder_without_write_batching() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";

    // Start with batching enabled
    let batch_config = BatchConfig::default();
    let builder = NodeBuilder::new(node_id, data_dir).with_write_batching(batch_config).without_write_batching();

    assert!(builder.config.batch_config.is_none());
}

#[test]
fn test_nodebuilder_fluent_api_chaining() {
    let node_id = NodeId::from(1);
    let data_dir = "/tmp/test-node";
    let cookie = "test-cookie";
    let builder = NodeBuilder::new(node_id, data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_gossip(true)
        .with_mdns(false)
        .with_cookie(cookie)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1000, 2000);

    // Verify all settings were applied
    assert_eq!(builder.config.storage_backend, StorageBackend::InMemory);
    assert!(builder.config.iroh.enable_gossip);
    assert!(!builder.config.iroh.enable_mdns);
    assert_eq!(builder.config.cookie, cookie);
    assert_eq!(builder.config.heartbeat_interval_ms, 500);
    assert_eq!(builder.config.election_timeout_min_ms, 1000);
    assert_eq!(builder.config.election_timeout_max_ms, 2000);
}

#[cfg(feature = "node-runtime")]
#[tokio::test]
async fn test_nodebuilder_start_creates_node() {
    let node_id = NodeId::from(1);
    let temp_dir = TempDir::new().unwrap();

    let node = NodeBuilder::new(node_id, temp_dir.path())
        .with_storage(StorageBackend::InMemory)
        .with_gossip(false)
        .with_mdns(false)
        .with_cookie("test-cookie-unique-1")
        .start()
        .await;

    assert!(node.is_ok(), "Failed to start node: {:?}", node.err());

    let node = node.unwrap();
    assert_eq!(node.node_id(), node_id);

    // Verify we can get the store interface
    let _store = node.kv_store();

    // The node provides access to the underlying RaftNode

    // Clean shutdown
    assert!(node.shutdown().await.is_ok());
}

#[cfg(all(
    feature = "node-runtime",
    feature = "trust",
    feature = "secrets",
    feature = "jobs",
    feature = "docs",
    feature = "hooks",
    feature = "federation"
))]
#[tokio::test]
async fn test_finish_secrets_service_setup_keeps_service_without_runtime_trust() {
    use std::collections::HashMap;
    use std::sync::Arc;

    use tokio::sync::RwLock;

    struct InMemoryKvStore {
        data: Arc<RwLock<HashMap<String, String>>>,
    }

    #[async_trait::async_trait]
    impl aspen_core::KvRead for InMemoryKvStore {
        async fn read(
            &self,
            request: aspen_core::ReadRequest,
        ) -> std::result::Result<aspen_core::ReadResult, aspen_core::KeyValueStoreError> {
            let data = self.data.read().await;
            let kv = data.get(&request.key).map(|value| aspen_core::KeyValueWithRevision {
                key: request.key,
                value: value.clone(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
            });
            Ok(aspen_core::ReadResult { kv })
        }
    }

    #[async_trait::async_trait]
    impl aspen_core::KvWrite for InMemoryKvStore {
        async fn write(
            &self,
            request: aspen_core::WriteRequest,
        ) -> std::result::Result<aspen_core::WriteResult, aspen_core::KeyValueStoreError> {
            if let aspen_core::WriteCommand::Set { key, value } = &request.command {
                self.data.write().await.insert(key.clone(), value.clone());
            }
            Ok(aspen_core::WriteResult {
                command: Some(request.command),
                batch_applied: None,
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: Some(true),
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            })
        }
    }

    #[async_trait::async_trait]
    impl aspen_core::KvDelete for InMemoryKvStore {
        async fn delete(
            &self,
            request: aspen_core::DeleteRequest,
        ) -> std::result::Result<aspen_core::DeleteResult, aspen_core::KeyValueStoreError> {
            let existed = self.data.write().await.remove(&request.key).is_some();
            Ok(aspen_core::DeleteResult {
                key: request.key,
                is_deleted: existed,
            })
        }
    }

    #[async_trait::async_trait]
    impl aspen_core::KvScan for InMemoryKvStore {
        async fn scan(
            &self,
            request: aspen_core::ScanRequest,
        ) -> std::result::Result<aspen_core::ScanResult, aspen_core::KeyValueStoreError> {
            let data = self.data.read().await;
            let entries = data
                .iter()
                .filter(|(key, _)| key.starts_with(&request.prefix))
                .map(|(key, value)| aspen_core::KeyValueWithRevision {
                    key: key.clone(),
                    value: value.clone(),
                    version: 1,
                    create_revision: 1,
                    mod_revision: 1,
                })
                .collect::<Vec<_>>();
            Ok(aspen_core::ScanResult {
                result_count: entries.len() as u32,
                entries,
                is_truncated: false,
                continuation_token: None,
            })
        }
    }

    impl aspen_core::KeyValueStore for InMemoryKvStore {}

    let kv = Arc::new(InMemoryKvStore {
        data: Arc::new(RwLock::new(HashMap::new())),
    });
    let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv));

    let secrets_service = Node::finish_secrets_service_setup(mount_registry, Ok(None));
    assert!(
        secrets_service.is_some(),
        "secrets service should stay available when trust feature is compiled but runtime trust is not configured"
    );
}

#[cfg(feature = "node-runtime")]
#[tokio::test]
async fn test_nodebuilder_node_handle_accessors() {
    let node_id = NodeId::from(1);
    let temp_dir = TempDir::new().unwrap();

    let node = NodeBuilder::new(node_id, temp_dir.path())
        .with_storage(StorageBackend::InMemory)
        .with_gossip(false)
        .with_mdns(false)
        .with_cookie("test-cookie-unique-2")
        .start()
        .await
        .unwrap();

    // Test accessors
    assert_eq!(node.node_id(), node_id);

    // endpoint_addr returns EndpointAddr directly, not Option
    let addr = node.endpoint_addr();
    assert!(!addr.id.to_string().is_empty());

    // Clean shutdown
    node.shutdown().await.unwrap();
}

#[test]
fn test_nodebuilder_pathbuf_conversion() {
    let node_id = NodeId::from(1);

    // Test with &str
    let builder = NodeBuilder::new(node_id, "/tmp/test");
    assert_eq!(builder.config.data_dir, Some(PathBuf::from("/tmp/test")));

    // Test with String
    let path_string = String::from("/tmp/test2");
    let builder = NodeBuilder::new(node_id, path_string);
    assert_eq!(builder.config.data_dir, Some(PathBuf::from("/tmp/test2")));

    // Test with PathBuf
    let path_buf = PathBuf::from("/tmp/test3");
    let builder = NodeBuilder::new(node_id, path_buf.clone());
    assert_eq!(builder.config.data_dir, Some(path_buf));
}
