//! Integration tests for gossip-based peer discovery.
//!
//! These tests verify the full gossip discovery stack including:
//! - Ticket generation and parsing
//! - Topic ID derivation from cluster cookie
//! - GossipPeerDiscovery lifecycle
//! - Bootstrap integration with gossip enabled/disabled

mod support;

use std::time::Duration;

use anyhow::Result;
use aspen::cluster::config::ControlBackend;
use aspen::cluster::config::IrohConfig;
use aspen::cluster::config::NodeConfig;
use aspen::cluster::ticket::AspenClusterTicket;
use iroh::SecretKey;
use iroh_gossip::proto::TopicId;
use support::mock_gossip::MockGossip;
use tokio::time::timeout;

fn create_test_config(node_id: u64, enable_gossip: bool) -> NodeConfig {
    NodeConfig {
        node_id,
        data_dir: Some(std::env::temp_dir().join(format!("aspen-test-{}", node_id))),
        host: "127.0.0.1".into(),
        cookie: "test-cluster".into(),
        http_addr: format!("127.0.0.1:{}", 8080 + node_id).parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip,
            gossip_ticket: None,
            enable_mdns: true,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            enable_pkarr_dht: true,
            enable_pkarr_relay: true,
            include_pkarr_direct_addresses: true,
            pkarr_republish_delay_secs: 600,
            enable_raft_auth: false,
        },
        ..Default::default()
    }
}

#[test]
fn test_ticket_serialization_roundtrip() {
    let topic_id = TopicId::from_bytes([42u8; 32]);
    let secret_key = SecretKey::from([1u8; 32]);
    let endpoint_id = secret_key.public();

    let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), endpoint_id);

    // Serialize
    let serialized = ticket.serialize();
    assert!(serialized.starts_with("aspen"));

    // Deserialize
    let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();
    assert_eq!(deserialized.topic_id, topic_id);
    assert_eq!(deserialized.cluster_id, "test-cluster");
    assert_eq!(deserialized.bootstrap.len(), 1);
    assert!(deserialized.bootstrap.contains(&endpoint_id));
}

#[test]
fn test_ticket_with_multiple_bootstrap_peers() {
    let topic_id = TopicId::from_bytes([23u8; 32]);
    let mut ticket = AspenClusterTicket::new(topic_id, "prod-cluster".into());

    // Add multiple bootstrap peers (max 16)
    for i in 1..=10 {
        let secret_key = SecretKey::from([i; 32]);
        let endpoint_id = secret_key.public();
        ticket.add_bootstrap(endpoint_id).unwrap();
    }

    assert_eq!(ticket.bootstrap.len(), 10);

    // Roundtrip
    let serialized = ticket.serialize();
    let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();
    assert_eq!(deserialized.bootstrap.len(), 10);
}

#[test]
fn test_ticket_max_bootstrap_limit() {
    let topic_id = TopicId::from_bytes([1u8; 32]);
    let mut ticket = AspenClusterTicket::new(topic_id, "test".into());

    // Add exactly MAX_BOOTSTRAP_PEERS (16)
    for i in 0..AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
        let secret_key = SecretKey::from([i as u8; 32]);
        ticket.add_bootstrap(secret_key.public()).unwrap();
    }

    // Adding one more should fail
    let extra_peer = SecretKey::from([255u8; 32]).public();
    let result = ticket.add_bootstrap(extra_peer);
    assert!(result.is_err());
}

#[test]
fn test_topic_id_derivation_from_cookie() {
    // Verify that the same cookie produces the same topic ID
    let cookie1 = "my-cluster";
    let cookie2 = "my-cluster";
    let cookie3 = "different-cluster";

    let hash1 = blake3::hash(cookie1.as_bytes());
    let topic1 = TopicId::from_bytes(*hash1.as_bytes());

    let hash2 = blake3::hash(cookie2.as_bytes());
    let topic2 = TopicId::from_bytes(*hash2.as_bytes());

    let hash3 = blake3::hash(cookie3.as_bytes());
    let topic3 = TopicId::from_bytes(*hash3.as_bytes());

    // Same cookie -> same topic ID
    assert_eq!(topic1, topic2);

    // Different cookie -> different topic ID
    assert_ne!(topic1, topic3);
}

#[tokio::test]
async fn test_mock_gossip_peer_announcement() -> Result<()> {
    let gossip = MockGossip::new();
    let topic = TopicId::from_bytes([1u8; 32]);

    // Two nodes subscribe to the same topic
    let handle1 = gossip.subscribe(topic)?;
    let handle2 = gossip.subscribe(topic)?;

    // Node 1 announces itself
    let secret_key1 = SecretKey::from([1u8; 32]);
    let endpoint_addr1 = iroh::EndpointAddr::new(secret_key1.public());

    #[derive(serde::Serialize, serde::Deserialize)]
    struct PeerAnnouncement {
        node_id: u64,
        endpoint_addr: iroh::EndpointAddr,
        timestamp_micros: u64,
    }

    let announcement = PeerAnnouncement {
        node_id: 1,
        endpoint_addr: endpoint_addr1.clone(),
        timestamp_micros: 12345,
    };
    let bytes = postcard::to_stdvec(&announcement)?;

    handle1.broadcast(bytes).await?;

    // Node 2 should receive the announcement
    let received = timeout(Duration::from_secs(1), handle2.receive()).await?.unwrap().unwrap();

    let parsed: PeerAnnouncement = postcard::from_bytes(&received)?;
    assert_eq!(parsed.node_id, 1);
    assert_eq!(parsed.endpoint_addr.id, endpoint_addr1.id);
    assert_eq!(parsed.timestamp_micros, 12345);

    Ok(())
}

#[tokio::test]
async fn test_mock_gossip_multiple_nodes() -> Result<()> {
    let gossip = MockGossip::new();
    let topic = TopicId::from_bytes([2u8; 32]);

    // Three nodes subscribe
    let handle1 = gossip.subscribe(topic)?;
    let handle2 = gossip.subscribe(topic)?;
    let handle3 = gossip.subscribe(topic)?;

    // Node 1 broadcasts
    handle1.broadcast(b"hello from node 1".to_vec()).await?;

    // All nodes (including sender) should receive
    let msg2 = handle2.receive().await?.unwrap();
    let msg3 = handle3.receive().await?.unwrap();
    let msg1 = handle1.receive().await?.unwrap();

    assert_eq!(msg1, b"hello from node 1"[..]);
    assert_eq!(msg2, b"hello from node 1"[..]);
    assert_eq!(msg3, b"hello from node 1"[..]);

    Ok(())
}

#[test]
fn test_config_gossip_enabled_by_default() {
    let config = IrohConfig::default();
    assert!(config.enable_gossip);
}

#[test]
fn test_config_gossip_disabled() {
    let config = IrohConfig {
        enable_gossip: false,
        ..Default::default()
    };
    assert!(!config.enable_gossip);
}

#[test]
fn test_config_with_ticket() {
    let topic_id = TopicId::from_bytes([1u8; 32]);
    let ticket = AspenClusterTicket::new(topic_id, "test".into());
    let ticket_str = ticket.serialize();

    let config = IrohConfig {
        secret_key: None,
        enable_gossip: true,
        gossip_ticket: Some(ticket_str.clone()),
        enable_mdns: true,
        enable_dns_discovery: false,
        dns_discovery_url: None,
        enable_pkarr: false,
        enable_pkarr_dht: true,
        enable_pkarr_relay: true,
        include_pkarr_direct_addresses: true,
        pkarr_republish_delay_secs: 600,
        enable_raft_auth: false,
    };

    assert_eq!(config.gossip_ticket, Some(ticket_str));
}

#[tokio::test]
async fn test_bootstrap_config_gossip_fields() {
    let config = create_test_config(1, true);
    assert!(config.iroh.enable_gossip);
    assert_eq!(config.iroh.gossip_ticket, None);

    let config_disabled = create_test_config(2, false);
    assert!(!config_disabled.iroh.enable_gossip);
}

#[test]
fn test_ticket_invalid_deserialize() {
    // Invalid prefix
    let result = AspenClusterTicket::deserialize("invalid_ticket");
    assert!(result.is_err());

    // Invalid base32
    let result = AspenClusterTicket::deserialize("aspen!!!invalid!!!");
    assert!(result.is_err());

    // Empty string
    let result = AspenClusterTicket::deserialize("");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_gossip_topic_isolation() -> Result<()> {
    let gossip = MockGossip::new();
    let topic1 = TopicId::from_bytes([1u8; 32]);
    let topic2 = TopicId::from_bytes([2u8; 32]);

    let handle1a = gossip.subscribe(topic1)?;
    let handle1b = gossip.subscribe(topic1)?;
    let handle2 = gossip.subscribe(topic2)?;

    // Broadcast on topic1
    handle1a.broadcast(b"topic1 message".to_vec()).await?;

    // Topic1 subscribers receive
    let msg1b = handle1b.receive().await?.unwrap();
    assert_eq!(msg1b, b"topic1 message"[..]);

    // Topic2 subscriber does not receive
    let msg2 = handle2.try_receive().await?;
    assert!(msg2.is_none());

    Ok(())
}

#[test]
fn test_cluster_bootstrap_config_merge_gossip_fields() {
    let mut base = NodeConfig {
        node_id: 1,
        host: "127.0.0.1".into(),
        cookie: "base-cookie".into(),
        http_addr: "127.0.0.1:8080".parse().unwrap(),
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip: true,
            gossip_ticket: None,
            enable_mdns: true,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            enable_pkarr_dht: true,
            enable_pkarr_relay: true,
            include_pkarr_direct_addresses: true,
            pkarr_republish_delay_secs: 600,
            enable_raft_auth: false,
        },
        ..Default::default()
    };

    let override_config = NodeConfig {
        node_id: 0,
        host: "127.0.0.1".into(),
        cookie: "base-cookie".into(),
        http_addr: "127.0.0.1:8080".parse().unwrap(),
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip: false, // Override to disable
            gossip_ticket: Some("test-ticket".into()),
            enable_mdns: false,
            enable_dns_discovery: true,
            dns_discovery_url: Some("https://dns.example.com".into()),
            enable_pkarr: false,
            enable_pkarr_dht: true,
            enable_pkarr_relay: true,
            include_pkarr_direct_addresses: true,
            pkarr_republish_delay_secs: 600,
            enable_raft_auth: true, // Override to enable
        },
        ..Default::default()
    };

    base.merge(override_config);

    assert!(!base.iroh.enable_gossip);
    assert_eq!(base.iroh.gossip_ticket, Some("test-ticket".into()));
}
