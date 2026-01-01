//! Unit tests for the NodeBuilder.

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tempfile::TempDir;

    use super::super::*;
    use crate::raft::storage::StorageBackend;
    use crate::raft::BatchConfig;

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
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_storage(StorageBackend::InMemory);
        assert_eq!(builder.config.storage_backend, StorageBackend::InMemory);

        // Test Redb backend (the only persistent backend)
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_storage(StorageBackend::Redb);
        assert_eq!(builder.config.storage_backend, StorageBackend::Redb);

        // Test InMemory with different builder instance
        let builder2 = NodeBuilder::new(node_id, data_dir)
            .with_storage(StorageBackend::InMemory);
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

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_peers(peers.clone());

        assert_eq!(builder.config.peers, peers);
    }

    #[test]
    fn test_nodebuilder_with_gossip() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";

        // Test enabling gossip
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_gossip(true);
        assert!(builder.config.iroh.enable_gossip);

        // Test disabling gossip
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_gossip(false);
        assert!(!builder.config.iroh.enable_gossip);
    }

    #[test]
    fn test_nodebuilder_with_mdns() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";

        // Test enabling mDNS
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_mdns(true);
        assert!(builder.config.iroh.enable_mdns);

        // Test disabling mDNS
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_mdns(false);
        assert!(!builder.config.iroh.enable_mdns);
    }

    #[test]
    fn test_nodebuilder_with_heartbeat_interval() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let interval_ms = 500u64;

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_heartbeat_interval_ms(interval_ms);

        assert_eq!(builder.config.heartbeat_interval_ms, interval_ms);
    }

    #[test]
    fn test_nodebuilder_with_election_timeout() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let min_ms = 1000u64;
        let max_ms = 2000u64;

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_election_timeout_ms(min_ms, max_ms);

        assert_eq!(builder.config.election_timeout_min_ms, min_ms);
        assert_eq!(builder.config.election_timeout_max_ms, max_ms);
    }

    #[test]
    fn test_nodebuilder_with_cookie() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let cookie = "test-cookie-123";

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_cookie(cookie);

        assert_eq!(builder.config.cookie, cookie);
    }

    #[test]
    fn test_nodebuilder_with_http_addr() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 9090);

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_http_addr(addr);

        assert_eq!(builder.config.http_addr, addr);
    }

    #[test]
    fn test_nodebuilder_with_iroh_secret_key() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        // 64 hex characters = 32 bytes
        let secret_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_iroh_secret_key(secret_key);

        assert_eq!(builder.config.iroh.secret_key, Some(secret_key.to_string()));
    }

    #[test]
    fn test_nodebuilder_with_gossip_ticket() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let ticket = "aspentktu4u5ars6t7sumdgqcxojisnnmaxr35r532tiyo63gbqibq7h7dmzacaifadiambqaaaaaaaaaaaaai";

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_gossip_ticket(ticket);

        assert_eq!(builder.config.iroh.gossip_ticket, Some(ticket.to_string()));
    }

    #[test]
    fn test_nodebuilder_with_dns_discovery() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_dns_discovery(true);

        assert!(builder.config.iroh.enable_dns_discovery);
    }

    #[test]
    fn test_nodebuilder_with_pkarr() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_pkarr(true);

        assert!(builder.config.iroh.enable_pkarr);
    }

    #[test]
    fn test_nodebuilder_with_write_batching() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let batch_config = BatchConfig {
            max_entries: 100,
            max_bytes: 1024 * 1024,  // 1MB
            max_wait_ms: 100,
            max_wait: std::time::Duration::from_millis(100),
        };

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_write_batching(batch_config.clone());

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
        let builder = NodeBuilder::new(node_id, data_dir)
            .with_write_batching(batch_config)
            .without_write_batching();

        assert!(builder.config.batch_config.is_none());
    }

    #[test]
    fn test_nodebuilder_fluent_api_chaining() {
        let node_id = NodeId::from(1);
        let data_dir = "/tmp/test-node";
        let cookie = "test-cookie";
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let builder = NodeBuilder::new(node_id, data_dir)
            .with_storage(StorageBackend::InMemory)
            .with_gossip(true)
            .with_mdns(false)
            .with_cookie(cookie)
            .with_http_addr(addr)
            .with_heartbeat_interval_ms(500)
            .with_election_timeout_ms(1000, 2000);

        // Verify all settings were applied
        assert_eq!(builder.config.storage_backend, StorageBackend::InMemory);
        assert!(builder.config.iroh.enable_gossip);
        assert!(!builder.config.iroh.enable_mdns);
        assert_eq!(builder.config.cookie, cookie);
        assert_eq!(builder.config.http_addr, addr);
        assert_eq!(builder.config.heartbeat_interval_ms, 500);
        assert_eq!(builder.config.election_timeout_min_ms, 1000);
        assert_eq!(builder.config.election_timeout_max_ms, 2000);
    }

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
}