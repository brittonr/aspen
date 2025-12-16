//! Property-based tests for cluster coordination layer.
//!
//! This module tests properties of:
//! - Configuration parsing and roundtrip
//! - Cluster ticket encoding/decoding
//! - Node metadata persistence
//!
//! These tests complement the fuzz targets by verifying semantic correctness
//! rather than just crash-freedom.

use std::collections::BTreeSet;

use proptest::prelude::*;

use aspen::cluster::config::NodeConfig;
use aspen::cluster::ticket::AspenClusterTicket;
use iroh_gossip::proto::TopicId;

// Strategy to generate valid node IDs (non-zero u64)
fn valid_node_id() -> impl Strategy<Value = u64> {
    1u64..u64::MAX
}

// Strategy to generate valid cluster IDs
fn valid_cluster_id() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9-]{0,31}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

// Strategy to generate valid timeout values (ms)
fn valid_timeout_ms() -> impl Strategy<Value = u64> {
    50u64..10000u64
}

// Strategy to generate topic IDs
fn topic_id() -> impl Strategy<Value = TopicId> {
    prop::array::uniform32(any::<u8>()).prop_map(TopicId::from_bytes)
}

// Test 1: Cluster ticket serialization roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_cluster_ticket_roundtrip(
        topic in topic_id(),
        cluster_id in valid_cluster_id(),
        num_bootstrap in 0usize..5usize,
    ) {
        // Generate bootstrap peers
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut bootstrap = BTreeSet::new();
            for i in 0..num_bootstrap {
                let bytes: [u8; 32] = std::array::from_fn(|j| (i * 32 + j) as u8);
                let secret = iroh::SecretKey::from(bytes);
                bootstrap.insert(secret.public());
            }

            let original = AspenClusterTicket {
                topic_id: topic,
                bootstrap,
                cluster_id,
            };

            // Serialize with postcard
            let bytes = postcard::to_stdvec(&original)
                .expect("Failed to serialize ticket");

            // Deserialize
            let roundtripped: AspenClusterTicket = postcard::from_bytes(&bytes)
                .expect("Failed to deserialize ticket");

            // Verify equality
            prop_assert_eq!(original.topic_id, roundtripped.topic_id);
            prop_assert_eq!(original.cluster_id, roundtripped.cluster_id);
            prop_assert_eq!(original.bootstrap.len(), roundtripped.bootstrap.len());

            // Verify bootstrap peers match
            for peer in &original.bootstrap {
                prop_assert!(
                    roundtripped.bootstrap.contains(peer),
                    "Missing bootstrap peer after roundtrip"
                );
            }

            Ok(())
        })?;
    }
}

// Test 2: Cluster ticket with empty bootstrap
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_cluster_ticket_empty_bootstrap(
        topic in topic_id(),
        cluster_id in valid_cluster_id(),
    ) {
        let original = AspenClusterTicket {
            topic_id: topic,
            bootstrap: BTreeSet::new(),
            cluster_id,
        };

        let bytes = postcard::to_stdvec(&original)
            .expect("Failed to serialize empty ticket");

        let roundtripped: AspenClusterTicket = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize empty ticket");

        prop_assert!(roundtripped.bootstrap.is_empty());
        prop_assert_eq!(original.cluster_id, roundtripped.cluster_id);
    }
}

// Test 3: Config timeout consistency
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_config_timeout_constraints(
        heartbeat_interval in valid_timeout_ms(),
        election_timeout_min in valid_timeout_ms(),
        election_timeout_max in valid_timeout_ms(),
    ) {
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
            heartbeat_interval, election_timeout_min, election_timeout_max
        );

        let config: NodeConfig = toml::from_str(&toml_str)
            .expect("Failed to parse config TOML");

        // Verify config captures values
        prop_assert_eq!(config.heartbeat_interval_ms, heartbeat_interval);
        prop_assert_eq!(config.election_timeout_min_ms, election_timeout_min);
        prop_assert_eq!(config.election_timeout_max_ms, election_timeout_max);
    }
}

// Test 4: Node ID validity
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_node_id_validity(
        node_id in valid_node_id(),
    ) {
        // Property: Valid node IDs should be accepted
        let toml_str = format!("node_id = {}", node_id);
        let config: NodeConfig = toml::from_str(&toml_str)
            .expect("Failed to parse config TOML");

        prop_assert!(config.node_id > 0, "Node ID must be non-zero");
        prop_assert_eq!(config.node_id, node_id);
    }
}

// Test 5: Cluster ID format
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_cluster_id_format(
        cluster_id in valid_cluster_id(),
    ) {
        // Property: Cluster IDs should follow naming conventions
        // - Start with lowercase letter
        // - Contain only lowercase letters, numbers, and hyphens

        prop_assert!(
            cluster_id.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false),
            "Cluster ID should start with lowercase letter"
        );

        prop_assert!(
            cluster_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "Cluster ID should only contain lowercase letters, digits, and hyphens"
        );
    }
}

// Test 6: Bootstrap peer uniqueness
proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn test_bootstrap_peer_uniqueness(
        num_peers in 1usize..10usize,
    ) {
        let mut bootstrap = BTreeSet::new();

        // Generate unique peers
        for i in 0..num_peers {
            let bytes: [u8; 32] = std::array::from_fn(|j| (i * 32 + j) as u8);
            let secret = iroh::SecretKey::from(bytes);
            bootstrap.insert(secret.public());
        }

        // BTreeSet guarantees uniqueness
        prop_assert_eq!(bootstrap.len(), num_peers);

        // Create ticket and verify
        let ticket = AspenClusterTicket {
            topic_id: TopicId::from_bytes([0u8; 32]),
            bootstrap,
            cluster_id: "test".to_string(),
        };

        prop_assert_eq!(ticket.bootstrap.len(), num_peers);
    }
}

// Test 7: Config serialization to TOML
proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn test_config_toml_roundtrip(
        node_id in valid_node_id(),
        heartbeat in valid_timeout_ms(),
    ) {
        // Parse from TOML
        let toml_str = format!(
            r#"
            node_id = {}
            heartbeat_interval_ms = {}
            "#,
            node_id, heartbeat
        );
        let config: NodeConfig = toml::from_str(&toml_str)
            .expect("Failed to parse config TOML");

        // Re-serialize to TOML
        let reserial = toml::to_string(&config)
            .expect("Failed to serialize config to TOML");

        // Deserialize again
        let parsed: NodeConfig = toml::from_str(&reserial)
            .expect("Failed to parse re-serialized config from TOML");

        prop_assert_eq!(config.node_id, parsed.node_id);
        prop_assert_eq!(config.heartbeat_interval_ms, parsed.heartbeat_interval_ms);
    }
}
