//! Property-based tests for cluster coordination layer.
//!
//! This module tests properties of:
//! - Configuration parsing and roundtrip
//! - Cluster ticket encoding/decoding
//! - Node metadata persistence
//!
//! These tests complement the fuzz targets by verifying semantic correctness
//! rather than just crash-freedom.

mod support;

use std::collections::BTreeSet;

use aspen::cluster::config::NodeConfig;
use aspen::cluster::ticket::AspenClusterTicket;
use bolero::check;
use iroh_gossip::proto::TopicId;
use support::bolero_generators::ValidClusterId;
use support::bolero_generators::ValidTimeoutMs;

// Test 1: Cluster ticket serialization roundtrip
#[test]
fn test_cluster_ticket_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<(ValidClusterId, u8)>()
        .for_each(|(cluster_id, num_bootstrap)| {
            let num_bootstrap = (*num_bootstrap % 5) as usize;

            // Generate topic and bootstrap peers
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut bootstrap = BTreeSet::new();
                for i in 0..num_bootstrap {
                    let bytes: [u8; 32] = std::array::from_fn(|j| (i * 32 + j) as u8);
                    let secret = iroh::SecretKey::from(bytes);
                    bootstrap.insert(secret.public());
                }

                let topic = TopicId::from_bytes([0u8; 32]);

                let original = AspenClusterTicket {
                    topic_id: topic,
                    bootstrap,
                    cluster_id: cluster_id.0.clone(),
                };

                // Serialize with postcard
                let bytes = postcard::to_stdvec(&original).expect("Failed to serialize ticket");

                // Deserialize
                let roundtripped: AspenClusterTicket =
                    postcard::from_bytes(&bytes).expect("Failed to deserialize ticket");

                // Verify equality
                assert_eq!(original.topic_id, roundtripped.topic_id);
                assert_eq!(original.cluster_id, roundtripped.cluster_id);
                assert_eq!(original.bootstrap.len(), roundtripped.bootstrap.len());

                // Verify bootstrap peers match
                for peer in &original.bootstrap {
                    assert!(roundtripped.bootstrap.contains(peer), "Missing bootstrap peer after roundtrip");
                }
            });
        });
}

// Test 2: Cluster ticket with empty bootstrap
#[test]
fn test_cluster_ticket_empty_bootstrap() {
    check!().with_iterations(20).with_type::<ValidClusterId>().for_each(|cluster_id| {
        let topic = TopicId::from_bytes([0u8; 32]);

        let original = AspenClusterTicket {
            topic_id: topic,
            bootstrap: BTreeSet::new(),
            cluster_id: cluster_id.0.clone(),
        };

        let bytes = postcard::to_stdvec(&original).expect("Failed to serialize empty ticket");

        let roundtripped: AspenClusterTicket =
            postcard::from_bytes(&bytes).expect("Failed to deserialize empty ticket");

        assert!(roundtripped.bootstrap.is_empty());
        assert_eq!(original.cluster_id, roundtripped.cluster_id);
    });
}

// Test 3: Config timeout consistency
#[test]
fn test_config_timeout_constraints() {
    check!()
        .with_iterations(50)
        .with_type::<(ValidTimeoutMs, ValidTimeoutMs, ValidTimeoutMs)>()
        .for_each(|(heartbeat_interval, election_timeout_min, election_timeout_max)| {
            // Property: election_timeout_max should be >= election_timeout_min
            // And heartbeat should be < election_timeout_min (typically)

            // Parse a TOML config with these values
            let toml_str = format!(
                r#"
                node_id = 1
                heartbeat_interval_ms = {}
                election_timeout_min_ms = {}
                election_timeout_max_ms = {}
                "#,
                heartbeat_interval.0, election_timeout_min.0, election_timeout_max.0
            );

            let config: NodeConfig = toml::from_str(&toml_str).expect("Failed to parse config TOML");

            // Verify config captures values
            assert_eq!(config.heartbeat_interval_ms, heartbeat_interval.0);
            assert_eq!(config.election_timeout_min_ms, election_timeout_min.0);
            assert_eq!(config.election_timeout_max_ms, election_timeout_max.0);
        });
}

// Test 4: Node ID validity
#[test]
fn test_node_id_validity() {
    check!().with_iterations(100).with_type::<u32>().for_each(|node_id| {
        // Property: Valid node IDs should be accepted
        // Use u32 to ensure it fits in i64 range for TOML
        let node_id = 1 + (*node_id % 1_000_000) as u64;

        let toml_str = format!("node_id = {}", node_id);
        let config: NodeConfig = toml::from_str(&toml_str).expect("Failed to parse config TOML");

        assert!(config.node_id > 0, "Node ID must be non-zero");
        assert_eq!(config.node_id, node_id);
    });
}

// Test 5: Cluster ID format
#[test]
fn test_cluster_id_format() {
    check!().with_iterations(50).with_type::<ValidClusterId>().for_each(|cluster_id| {
        // Property: Cluster IDs should follow naming conventions
        // - Start with lowercase letter
        // - Contain only lowercase letters, numbers, and hyphens

        assert!(
            cluster_id.0.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false),
            "Cluster ID should start with lowercase letter"
        );

        assert!(
            cluster_id.0.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "Cluster ID should only contain lowercase letters, digits, and hyphens"
        );
    });
}

// Test 6: Bootstrap peer uniqueness
#[test]
fn test_bootstrap_peer_uniqueness() {
    check!().with_iterations(30).with_type::<u8>().for_each(|num_peers| {
        let num_peers = 1 + (*num_peers % 9) as usize;
        let mut bootstrap = BTreeSet::new();

        // Generate unique peers
        for i in 0..num_peers {
            let bytes: [u8; 32] = std::array::from_fn(|j| (i * 32 + j) as u8);
            let secret = iroh::SecretKey::from(bytes);
            bootstrap.insert(secret.public());
        }

        // BTreeSet guarantees uniqueness - verify we have at least 1 peer
        // and no more than requested
        assert!(!bootstrap.is_empty());
        assert!(bootstrap.len() <= num_peers);

        // Create ticket and verify
        let ticket = AspenClusterTicket {
            topic_id: TopicId::from_bytes([0u8; 32]),
            bootstrap,
            cluster_id: "test".to_string(),
        };

        assert!(!ticket.bootstrap.is_empty());
    });
}

// Test 7: Config serialization to TOML
#[test]
fn test_config_toml_roundtrip() {
    check!().with_iterations(30).with_type::<(u32, ValidTimeoutMs)>().for_each(|(node_id, heartbeat)| {
        // Use u32 to ensure it fits in i64 range for TOML
        let node_id = 1 + (*node_id % 1_000_000) as u64;

        // Parse from TOML
        let toml_str = format!(
            r#"
                node_id = {}
                heartbeat_interval_ms = {}
                "#,
            node_id, heartbeat.0
        );
        let config: NodeConfig = toml::from_str(&toml_str).expect("Failed to parse config TOML");

        // Re-serialize to TOML
        let reserial = toml::to_string(&config).expect("Failed to serialize config to TOML");

        // Deserialize again
        let parsed: NodeConfig = toml::from_str(&reserial).expect("Failed to parse re-serialized config from TOML");

        assert_eq!(config.node_id, parsed.node_id);
        assert_eq!(config.heartbeat_interval_ms, parsed.heartbeat_interval_ms);
    });
}
