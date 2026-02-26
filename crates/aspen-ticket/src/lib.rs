//! Aspen cluster tickets for peer discovery bootstrap.
//!
//! Tickets provide a compact, URL-safe way to share cluster membership information.
//! They contain the gossip topic ID and bootstrap peer endpoint IDs with direct
//! socket addresses, encoded as a base32 string following the iroh ticket pattern.
//!
//! # Example
//!
//! ```
//! use aspen_ticket::AspenClusterTicket;
//! use iroh_gossip::proto::TopicId;
//!
//! // Create a new ticket
//! let topic_id = TopicId::from_bytes([23u8; 32]);
//! let ticket = AspenClusterTicket::new(topic_id, "my-cluster".to_string());
//!
//! // Serialize for sharing
//! let ticket_str = ticket.serialize();
//! println!("Join with: --ticket {}", ticket_str);
//!
//! // Deserialize on another node
//! let parsed = AspenClusterTicket::deserialize(&ticket_str)?;
//! assert_eq!(parsed.topic_id, topic_id);
//! # Ok::<(), anyhow::Error>(())
//! ```

mod constants;
mod parse;
mod signed;
mod v2;

pub use parse::parse_ticket_to_addrs;
pub use signed::SignedAspenClusterTicket;
pub use v2::AspenClusterTicket;
pub use v2::BootstrapPeer;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::SocketAddr;

    use constants::SIGNED_TICKET_VERSION;
    use iroh::EndpointAddr;
    use iroh::EndpointId;
    use iroh::SecretKey;
    use iroh::TransportAddr;
    use iroh_gossip::proto::TopicId;

    use super::*;

    fn create_test_endpoint_id(seed: u8) -> EndpointId {
        let secret_key = iroh::SecretKey::from([seed; 32]);
        secret_key.public()
    }

    fn create_test_socket_addr(port: u16) -> SocketAddr {
        use std::net::IpAddr;
        use std::net::Ipv4Addr;
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, port as u8)), port)
    }

    fn create_test_endpoint_addr(seed: u8, addrs: &[SocketAddr]) -> EndpointAddr {
        let id = create_test_endpoint_id(seed);
        let transport_addrs: BTreeSet<TransportAddr> = addrs.iter().map(|a| TransportAddr::Ip(*a)).collect();
        EndpointAddr {
            id,
            addrs: transport_addrs,
        }
    }

    // ========================================================================
    // Basic Ticket Tests
    // ========================================================================

    #[test]
    fn test_ticket_new() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        assert_eq!(ticket.topic_id, topic_id);
        assert_eq!(ticket.cluster_id, "test-cluster");
        assert!(ticket.bootstrap.is_empty());
    }

    #[test]
    fn test_ticket_with_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);

        assert_eq!(ticket.bootstrap.len(), 1);
        assert_eq!(ticket.bootstrap[0].endpoint_id, peer_id);
        assert!(ticket.bootstrap[0].direct_addrs.is_empty());
    }

    #[test]
    fn test_ticket_with_bootstrap_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        assert_eq!(ticket.bootstrap.len(), 1);
        assert_eq!(ticket.bootstrap[0].endpoint_id, endpoint_addr.id);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&addr));
    }

    #[test]
    fn test_add_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let peer1 = create_test_endpoint_id(1);
        let peer2 = create_test_endpoint_id(2);

        ticket.add_bootstrap(peer1).unwrap();
        ticket.add_bootstrap(peer2).unwrap();

        assert_eq!(ticket.bootstrap.len(), 2);
        let ids = ticket.endpoint_ids();
        assert!(ids.contains(&peer1));
        assert!(ids.contains(&peer2));
    }

    #[test]
    fn test_add_bootstrap_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        assert_eq!(ticket.bootstrap.len(), 2);
    }

    #[test]
    fn test_add_bootstrap_max_limit() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Add exactly MAX_BOOTSTRAP_PEERS
        for i in 0..AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
            let peer = create_test_endpoint_id(i as u8);
            ticket.add_bootstrap(peer).unwrap();
        }

        assert_eq!(ticket.bootstrap.len(), AspenClusterTicket::MAX_BOOTSTRAP_PEERS);

        // Adding one more should fail
        let extra_peer = create_test_endpoint_id(255);
        let result = ticket.add_bootstrap(extra_peer);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_bootstrap_truncates_addrs() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Create endpoint with more than MAX_DIRECT_ADDRS_PER_PEER addresses
        let addrs: Vec<SocketAddr> = (0..12).map(|i| create_test_socket_addr(7000 + i)).collect();
        let endpoint_addr = create_test_endpoint_addr(1, &addrs);

        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();

        // Should be truncated to MAX_DIRECT_ADDRS_PER_PEER
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), AspenClusterTicket::MAX_DIRECT_ADDRS_PER_PEER);
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_serialize_deserialize() {
        let topic_id = TopicId::from_bytes([42u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicket::new(topic_id, "production-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        // Serialize
        let serialized = ticket.serialize();
        assert!(serialized.starts_with("aspen"));

        // Deserialize
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();
        assert_eq!(ticket, deserialized);
    }

    #[test]
    fn test_deserialize_invalid() {
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

    #[test]
    fn test_serialize_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([1u8; 32]);
        let ticket = AspenClusterTicket::new(topic_id, "empty-bootstrap".into());

        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert!(deserialized.bootstrap.is_empty());
        assert_eq!(deserialized.cluster_id, "empty-bootstrap");
    }

    // ========================================================================
    // Endpoint Accessors Tests
    // ========================================================================

    #[test]
    fn test_endpoint_addrs() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        let addrs = ticket.endpoint_addrs();

        assert_eq!(addrs.len(), 2);
        assert!(addrs.iter().any(|a| a.id == endpoint_addr1.id));
        assert!(addrs.iter().any(|a| a.id == endpoint_addr2.id));
    }

    #[test]
    fn test_endpoint_ids() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let endpoint_id1 = create_test_endpoint_id(1);
        let endpoint_id2 = create_test_endpoint_id(2);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap(endpoint_id1).unwrap();
        ticket.add_bootstrap(endpoint_id2).unwrap();

        let ids = ticket.endpoint_ids();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&endpoint_id1));
        assert!(ids.contains(&endpoint_id2));
    }

    // ========================================================================
    // BootstrapPeer Tests
    // ========================================================================

    #[test]
    fn test_bootstrap_peer_new() {
        let endpoint_id = create_test_endpoint_id(1);
        let peer = BootstrapPeer::new(endpoint_id);

        assert_eq!(peer.endpoint_id, endpoint_id);
        assert!(peer.direct_addrs.is_empty());
    }

    #[test]
    fn test_bootstrap_peer_from_endpoint_addr() {
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr1, addr2]);

        let peer = BootstrapPeer::from_endpoint_addr(&endpoint_addr);

        assert_eq!(peer.endpoint_id, endpoint_addr.id);
        assert_eq!(peer.direct_addrs.len(), 2);
        assert!(peer.direct_addrs.contains(&addr1));
        assert!(peer.direct_addrs.contains(&addr2));
    }

    #[test]
    fn test_bootstrap_peer_to_endpoint_addr() {
        let endpoint_id = create_test_endpoint_id(1);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);

        let peer = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        let endpoint_addr = peer.to_endpoint_addr();

        assert_eq!(endpoint_addr.id, endpoint_id);
        assert_eq!(endpoint_addr.addrs.len(), 2);
        assert!(endpoint_addr.addrs.contains(&TransportAddr::Ip(addr1)));
        assert!(endpoint_addr.addrs.contains(&TransportAddr::Ip(addr2)));
    }

    #[test]
    fn test_bootstrap_peer_roundtrip_conversion() {
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let original = create_test_endpoint_addr(1, &[addr1, addr2]);

        let peer = BootstrapPeer::from_endpoint_addr(&original);
        let roundtripped = peer.to_endpoint_addr();

        assert_eq!(roundtripped.id, original.id);
        assert_eq!(roundtripped.addrs, original.addrs);
    }

    #[test]
    fn test_bootstrap_peer_from_trait_endpoint_addr() {
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let peer: BootstrapPeer = (&endpoint_addr).into();

        assert_eq!(peer.endpoint_id, endpoint_addr.id);
        assert!(peer.direct_addrs.contains(&addr));
    }

    #[test]
    fn test_bootstrap_peer_from_trait_endpoint_id() {
        let endpoint_id = create_test_endpoint_id(1);

        let peer: BootstrapPeer = endpoint_id.into();

        assert_eq!(peer.endpoint_id, endpoint_id);
        assert!(peer.direct_addrs.is_empty());
    }

    // ========================================================================
    // inject_direct_addr Tests
    // ========================================================================

    #[test]
    fn test_inject_direct_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Add two bootstrap peers with no addresses
        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(1)));
        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(2)));

        // Inject a direct address
        let injected_addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(injected_addr);

        // Both peers should have the injected address
        for peer in &ticket.bootstrap {
            assert!(peer.direct_addrs.contains(&injected_addr));
        }
    }

    #[test]
    fn test_inject_direct_addr_idempotent() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(1)));

        let addr = create_test_socket_addr(9999);

        // Inject the same address multiple times
        ticket.inject_direct_addr(addr);
        ticket.inject_direct_addr(addr);
        ticket.inject_direct_addr(addr);

        // Should only appear once
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), 1);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&addr));
    }

    #[test]
    fn test_inject_direct_addr_preserves_existing() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let existing_addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[existing_addr]);

        let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        // Inject a new address
        let new_addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(new_addr);

        // Both addresses should be present
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), 2);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&existing_addr));
        assert!(ticket.bootstrap[0].direct_addrs.contains(&new_addr));
    }

    #[test]
    fn test_inject_direct_addr_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Inject address with no bootstrap peers (should be a no-op)
        let addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(addr);

        // No crash, no peers
        assert!(ticket.bootstrap.is_empty());
    }

    // ========================================================================
    // parse_ticket_to_addrs Tests
    // ========================================================================

    #[test]
    fn test_parse_ticket_to_addrs() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let socket_addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[socket_addr]);

        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        let ticket_str = ticket.serialize();
        let (parsed_topic, parsed_cluster, addrs) = parse_ticket_to_addrs(&ticket_str).unwrap();

        assert_eq!(parsed_topic, topic_id);
        assert_eq!(parsed_cluster, "test-cluster");
        assert_eq!(addrs.len(), 1);
        assert!(addrs[0].addrs.contains(&TransportAddr::Ip(socket_addr)));
    }

    #[test]
    fn test_parse_ticket_to_addrs_multiple_peers() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        let ticket_str = ticket.serialize();
        let (_, _, addrs) = parse_ticket_to_addrs(&ticket_str).unwrap();

        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn test_parse_ticket_to_addrs_invalid_prefix() {
        let result = parse_ticket_to_addrs("invalid_prefix_ticket");
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid ticket format"));
    }

    #[test]
    fn test_parse_ticket_to_addrs_empty_string() {
        let result = parse_ticket_to_addrs("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ticket_to_addrs_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ticket = AspenClusterTicket::new(topic_id, "empty".into());
        let (_, _, addrs) = parse_ticket_to_addrs(&ticket.serialize()).unwrap();
        assert!(addrs.is_empty());
    }

    // ========================================================================
    // Signed Ticket Tests
    // ========================================================================

    fn create_test_secret_key(seed: u8) -> SecretKey {
        SecretKey::from([seed; 32])
    }

    #[test]
    fn test_signed_ticket_sign_and_verify() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);

        // Sign the ticket
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Verify should succeed
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &ticket);

        // Issuer should match signer's public key
        assert_eq!(*signed.issuer(), secret_key.public());
    }

    #[test]
    fn test_signed_ticket_roundtrip_serialization() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Serialize
        let serialized = signed.serialize();
        assert!(serialized.starts_with("aspensigned"));

        // Deserialize
        let deserialized = SignedAspenClusterTicket::deserialize(&serialized).unwrap();

        // Verify should still succeed after roundtrip
        let verified = deserialized.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &ticket);
    }

    #[test]
    fn test_signed_ticket_wrong_key_rejected() {
        let real_key = create_test_secret_key(42);
        let fake_key = create_test_secret_key(99);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Sign with real key
        let signed = SignedAspenClusterTicket::sign(ticket, &real_key).unwrap();

        // Tamper: replace issuer with fake key's public key
        let tampered = SignedAspenClusterTicket {
            issuer: fake_key.public(),
            ..signed
        };

        // Verification should fail (signature doesn't match fake issuer)
        assert!(tampered.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_tampered_data_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "original-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Tamper with the inner ticket data
        let mut tampered = signed.clone();
        tampered.ticket.cluster_id = "tampered-cluster".into();

        // Verification should fail (data doesn't match signature)
        assert!(tampered.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_expired_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Create a ticket that's already expired by tampering with timestamps
        let mut signed = SignedAspenClusterTicket::sign_with_validity(ticket, &secret_key, 3600).unwrap();

        // Set expires_at to the past (1 hour ago)
        signed.expires_at_secs = signed.issued_at_secs.saturating_sub(3600);

        // Verification should fail due to expiration
        assert!(signed.verify().is_none());
        assert!(signed.is_expired());
    }

    #[test]
    fn test_signed_ticket_future_issued_at_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let mut signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Tamper: set issued_at far in the future (beyond clock skew tolerance)
        signed.issued_at_secs += 3600; // 1 hour in future

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_nonce_is_random() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket1 = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let ticket2 = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let signed1 = SignedAspenClusterTicket::sign(ticket1, &secret_key).unwrap();
        let signed2 = SignedAspenClusterTicket::sign(ticket2, &secret_key).unwrap();

        // Nonces should be different (randomly generated)
        assert_ne!(signed1.nonce, signed2.nonce);
    }

    #[test]
    fn test_signed_ticket_verify_with_error() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Valid ticket should verify with detailed result
        let result = signed.verify_with_error();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &ticket);

        // Tampered ticket should give error message
        let mut tampered = signed;
        tampered.ticket.cluster_id = "tampered".into();
        let result = tampered.verify_with_error();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("signature"));
    }

    #[test]
    fn test_signed_ticket_version_field() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Version should be current
        assert_eq!(signed.version, SIGNED_TICKET_VERSION);
    }

    #[test]
    fn test_signed_ticket_future_version_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let mut signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Set version to a future version
        signed.version = SIGNED_TICKET_VERSION + 1;

        // Verification should fail (unknown version)
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_with_custom_validity() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Create ticket with 1 hour validity
        let one_hour_secs = 3600;
        let signed = SignedAspenClusterTicket::sign_with_validity(ticket.clone(), &secret_key, one_hour_secs).unwrap();

        // Should not be expired
        assert!(!signed.is_expired());

        // Expires at should be roughly 1 hour from issued_at
        let validity_duration = signed.expires_at_secs - signed.issued_at_secs;
        assert_eq!(validity_duration, one_hour_secs);

        // Verification should succeed
        assert!(signed.verify().is_some());
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_ticket_with_ipv6_address() {
        use std::net::IpAddr;
        use std::net::Ipv6Addr;

        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 7777);
        let endpoint_id = create_test_endpoint_id(1);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.bootstrap.push(BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![ipv6_addr],
        });

        // Serialize and deserialize
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv6_addr));
    }

    #[test]
    fn test_ticket_mixed_ipv4_ipv6() {
        use std::net::IpAddr;
        use std::net::Ipv4Addr;
        use std::net::Ipv6Addr;

        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 7777);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 7777);
        let endpoint_id = create_test_endpoint_id(1);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.bootstrap.push(BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![ipv4_addr, ipv6_addr],
        });

        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.bootstrap[0].direct_addrs.len(), 2);
        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv4_addr));
        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv6_addr));
    }

    #[test]
    fn test_ticket_long_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let long_cluster_id = "a".repeat(1000);

        let ticket = AspenClusterTicket::new(topic_id, long_cluster_id.clone());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, long_cluster_id);
    }

    #[test]
    fn test_ticket_unicode_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let unicode_cluster_id = "test-cluster-";

        let ticket = AspenClusterTicket::new(topic_id, unicode_cluster_id.into());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, unicode_cluster_id);
    }

    #[test]
    fn test_bootstrap_peer_equality() {
        let endpoint_id = create_test_endpoint_id(1);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);

        let peer1 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        let peer2 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        // Same content = equal
        assert_eq!(peer1, peer2);

        // Different order = not equal (Vec comparison)
        let peer3 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr2, addr1],
        };
        assert_ne!(peer1, peer3);
    }

    #[test]
    fn test_ticket_duplicate_bootstrap_peers() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        // Add the same peer twice
        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();

        // Both should be added (Vec allows duplicates)
        assert_eq!(ticket.bootstrap.len(), 2);

        // endpoint_ids() uses BTreeSet, so duplicates are merged
        let ids = ticket.endpoint_ids();
        assert_eq!(ids.len(), 1);
    }
}
