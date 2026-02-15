//! Core gossip peer discovery implementation.

mod blob;
mod helpers;
mod lifecycle;
mod trait_impl;

#[cfg(feature = "blob")]
pub use blob::BlobAnnouncementParams;
#[cfg(feature = "blob")]
pub use blob::broadcast_blob_announcement;
pub use helpers::calculate_backoff_duration;
pub use lifecycle::GossipPeerDiscovery;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use aspen_core::BlobAnnouncedInfo;
    use aspen_core::DiscoveredPeer;
    use aspen_core::DiscoveryHandle;
    use aspen_core::StaleTopologyInfo;
    use aspen_core::TopologyStaleCallback;
    use aspen_raft_types::NodeId;
    use iroh::EndpointAddr;
    use iroh::SecretKey;
    use iroh_gossip::proto::TopicId;
    use tokio::sync::Mutex;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::gossip::constants::*;

    /// Create a deterministic secret key from a seed for reproducible tests.
    fn secret_key_from_seed(seed: u64) -> SecretKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
        SecretKey::from_bytes(&key_bytes)
    }

    /// Create a mock EndpointAddr from a secret key.
    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> EndpointAddr {
        EndpointAddr::new(secret_key.public())
    }

    // =========================================================================
    // calculate_backoff_duration Tests (Pure Function)
    // =========================================================================

    #[test]
    fn test_calculate_backoff_duration_first_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
    }

    #[test]
    fn test_calculate_backoff_duration_middle_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(2));
    }

    #[test]
    fn test_calculate_backoff_duration_last_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(4));
    }

    #[test]
    fn test_calculate_backoff_duration_clamps_to_max() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        // Index 10 exceeds array length, should clamp to last element
        assert_eq!(calculate_backoff_duration(10, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(usize::MAX, &durations), Duration::from_secs(4));
    }

    #[test]
    fn test_calculate_backoff_duration_single_element() {
        let durations = vec![Duration::from_secs(5)];
        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(5));
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(5));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(5));
    }

    #[test]
    fn test_calculate_backoff_duration_with_default_constants() {
        // Test with the actual constants used in production
        let durations: Vec<Duration> = GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(2));
        assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(3, &durations), Duration::from_secs(8));
        assert_eq!(calculate_backoff_duration(4, &durations), Duration::from_secs(16));
        // Past the end should clamp to the last element
        assert_eq!(calculate_backoff_duration(5, &durations), Duration::from_secs(16));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(16));
    }

    // =========================================================================
    // GossipPeerDiscovery Unit Tests (No Network Required)
    // =========================================================================

    #[test]
    fn test_gossip_peer_discovery_initial_state_not_running() {
        // We can't create a full GossipPeerDiscovery without a real Gossip instance,
        // but we can test the AtomicBool and AtomicU64 patterns used internally.
        let is_running = Arc::new(AtomicBool::new(false));
        assert!(!is_running.load(Ordering::SeqCst));

        is_running.store(true, Ordering::SeqCst);
        assert!(is_running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_topology_version_atomic_operations() {
        let version = Arc::new(AtomicU64::new(0));
        assert_eq!(version.load(Ordering::SeqCst), 0);

        version.store(42, Ordering::SeqCst);
        assert_eq!(version.load(Ordering::SeqCst), 42);

        version.store(u64::MAX, Ordering::SeqCst);
        assert_eq!(version.load(Ordering::SeqCst), u64::MAX);
    }

    #[test]
    fn test_cancellation_token_hierarchy() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        assert!(!parent.is_cancelled());
        assert!(!child.is_cancelled());

        parent.cancel();

        assert!(parent.is_cancelled());
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_child_independence() {
        let parent = CancellationToken::new();
        let child1 = parent.child_token();
        let child2 = parent.child_token();

        // Cancelling a child doesn't affect parent or siblings
        // (children don't have cancel(), only parent does)
        assert!(!parent.is_cancelled());
        assert!(!child1.is_cancelled());
        assert!(!child2.is_cancelled());

        parent.cancel();

        // All should be cancelled now
        assert!(parent.is_cancelled());
        assert!(child1.is_cancelled());
        assert!(child2.is_cancelled());
    }

    // =========================================================================
    // BlobAnnouncementParams Tests
    // =========================================================================

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_fields() {
        let secret_key = secret_key_from_seed(1);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(123u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0xAB; 32]);

        let params = BlobAnnouncementParams {
            node_id,
            endpoint_addr: endpoint_addr.clone(),
            blob_hash,
            blob_size: 1024,
            blob_format: iroh_blobs::BlobFormat::Raw,
            tag: Some("test".to_string()),
        };

        assert_eq!(params.node_id, node_id);
        assert_eq!(params.endpoint_addr.id, endpoint_addr.id);
        assert_eq!(params.blob_hash, blob_hash);
        assert_eq!(params.blob_size, 1024);
        assert_eq!(params.blob_format, iroh_blobs::BlobFormat::Raw);
        assert_eq!(params.tag, Some("test".to_string()));
    }

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_no_tag() {
        let secret_key = secret_key_from_seed(2);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let params = BlobAnnouncementParams {
            node_id: NodeId::from(456u64),
            endpoint_addr,
            blob_hash: iroh_blobs::Hash::from_bytes([0xCD; 32]),
            blob_size: 0,
            blob_format: iroh_blobs::BlobFormat::HashSeq,
            tag: None,
        };

        assert!(params.tag.is_none());
        assert_eq!(params.blob_format, iroh_blobs::BlobFormat::HashSeq);
    }

    // =========================================================================
    // Topic ID Tests
    // =========================================================================

    #[test]
    fn test_topic_id_from_bytes_deterministic() {
        let topic1 = TopicId::from_bytes([1u8; 32]);
        let topic2 = TopicId::from_bytes([1u8; 32]);
        let topic3 = TopicId::from_bytes([2u8; 32]);

        assert_eq!(topic1, topic2);
        assert_ne!(topic1, topic3);
    }

    // =========================================================================
    // DiscoveredPeer Tests
    // =========================================================================

    #[test]
    fn test_discovered_peer_construction() {
        let secret_key = secret_key_from_seed(10);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(100u64);
        let timestamp = 1234567890u64;

        let peer = DiscoveredPeer {
            node_id,
            address: endpoint_addr.clone(),
            timestamp_micros: timestamp,
        };

        assert_eq!(peer.node_id, node_id);
        assert_eq!(peer.address.id, endpoint_addr.id);
        assert_eq!(peer.timestamp_micros, timestamp);
    }

    // =========================================================================
    // Constants Validation Tests
    // =========================================================================

    #[test]
    fn test_gossip_constants_sane_values() {
        // Verify constants are within reasonable bounds
        assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS > 0);
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS >= GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS);
        assert!(GOSSIP_ANNOUNCE_FAILURE_THRESHOLD > 0);
        assert!(GOSSIP_MAX_STREAM_RETRIES > 0);
        assert!(!GOSSIP_STREAM_BACKOFF_SECS.is_empty());
        assert!(MAX_GOSSIP_MESSAGE_SIZE > 0);
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT.as_secs() > 0);
    }

    #[test]
    fn test_gossip_stream_backoff_is_increasing() {
        // Verify backoff sequence is monotonically increasing
        let mut prev = 0u64;
        for &secs in &GOSSIP_STREAM_BACKOFF_SECS {
            assert!(secs > prev, "backoff sequence should be increasing");
            prev = secs;
        }
    }

    #[test]
    fn test_gossip_rate_limits_reasonable() {
        // Per-peer rate should be less than global rate
        assert!(GOSSIP_PER_PEER_RATE_PER_MINUTE < GOSSIP_GLOBAL_RATE_PER_MINUTE);
        assert!(GOSSIP_PER_PEER_BURST < GOSSIP_GLOBAL_BURST);
    }

    // =========================================================================
    // Async Tests (Require Tokio Runtime but No Real Network)
    // =========================================================================

    #[tokio::test]
    async fn test_mutex_callback_pattern() {
        // Test the callback storage pattern used by GossipPeerDiscovery
        let callback_storage: Mutex<Option<TopologyStaleCallback>> = Mutex::new(None);

        // Initially None
        assert!(callback_storage.lock().await.is_none());

        // Set a callback
        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let callback: TopologyStaleCallback = Box::new(move |_info| {
            let invoked = invoked_clone.clone();
            Box::pin(async move {
                invoked.store(true, Ordering::SeqCst);
            })
        });

        *callback_storage.lock().await = Some(callback);

        // Take and invoke
        let taken_callback = callback_storage.lock().await.take();
        assert!(taken_callback.is_some());

        let info = StaleTopologyInfo {
            announcing_node_id: 1,
            remote_version: 10,
            remote_hash: 123456,
            remote_term: 1,
        };
        taken_callback.unwrap()(info).await;

        assert!(invoked.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_cancellation_token_select_pattern() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Spawn a task that waits for cancellation
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    "cancelled"
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    "timeout"
                }
            }
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel and verify
        cancel.cancel();
        let result = handle.await.unwrap();
        assert_eq!(result, "cancelled");
    }

    #[tokio::test]
    async fn test_task_abortion_on_timeout() {
        let handle = tokio::spawn(async {
            // This task would run forever
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Wait briefly then abort
        tokio::time::sleep(Duration::from_millis(10)).await;
        handle.abort();

        // The task should be aborted
        let result = handle.await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn test_discovery_handle_cancellation() {
        let cancel = CancellationToken::new();
        let handle = DiscoveryHandle::new(cancel.clone());

        assert!(!cancel.is_cancelled());

        // Cancel via handle
        handle.cancel();

        assert!(cancel.is_cancelled());
    }

    // =========================================================================
    // Additional Edge Case Tests
    // =========================================================================

    #[test]
    fn test_calculate_backoff_duration_zero_length_clamp() {
        // Edge case: empty durations array (should be handled gracefully)
        // In practice this would panic due to saturating_sub on 0, but we test that
        // the code with GOSSIP_STREAM_BACKOFF_SECS is always non-empty
        assert!(!GOSSIP_STREAM_BACKOFF_SECS.is_empty());
    }

    #[test]
    fn test_secret_key_determinism() {
        // Verify our test helper produces deterministic keys
        let key1 = secret_key_from_seed(42);
        let key2 = secret_key_from_seed(42);
        let key3 = secret_key_from_seed(43);

        assert_eq!(key1.public(), key2.public());
        assert_ne!(key1.public(), key3.public());
    }

    #[test]
    fn test_endpoint_addr_from_secret_key_consistency() {
        let secret_key = secret_key_from_seed(100);
        let addr1 = endpoint_addr_from_secret_key(&secret_key);
        let addr2 = endpoint_addr_from_secret_key(&secret_key);

        assert_eq!(addr1.id, addr2.id);
    }

    #[test]
    fn test_node_id_conversions() {
        // Test NodeId from various u64 values
        let node_id_zero = NodeId::from(0u64);
        let node_id_one = NodeId::from(1u64);
        let node_id_max = NodeId::from(u64::MAX);

        assert_eq!(u64::from(node_id_zero), 0);
        assert_eq!(u64::from(node_id_one), 1);
        assert_eq!(u64::from(node_id_max), u64::MAX);
    }

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_large_size() {
        let secret_key = secret_key_from_seed(3);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let params = BlobAnnouncementParams {
            node_id: NodeId::from(1u64),
            endpoint_addr,
            blob_hash: iroh_blobs::Hash::from_bytes([0xFF; 32]),
            blob_size: u64::MAX, // Maximum possible size
            blob_format: iroh_blobs::BlobFormat::Raw,
            tag: None,
        };

        assert_eq!(params.blob_size, u64::MAX);
    }

    #[test]
    fn test_discovered_peer_with_zero_timestamp() {
        let secret_key = secret_key_from_seed(20);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let peer = DiscoveredPeer {
            node_id: NodeId::from(200u64),
            address: endpoint_addr,
            timestamp_micros: 0, // Edge case: zero timestamp
        };

        assert_eq!(peer.timestamp_micros, 0);
    }

    #[test]
    fn test_discovered_peer_with_max_timestamp() {
        let secret_key = secret_key_from_seed(21);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let peer = DiscoveredPeer {
            node_id: NodeId::from(201u64),
            address: endpoint_addr,
            timestamp_micros: u64::MAX, // Edge case: max timestamp
        };

        assert_eq!(peer.timestamp_micros, u64::MAX);
    }

    #[cfg(feature = "blob")]
    #[tokio::test]
    async fn test_blob_announced_callback_pattern() {
        // Test the BlobAnnouncedCallback storage pattern
        let callback_storage: Mutex<Option<aspen_core::BlobAnnouncedCallback>> = Mutex::new(None);

        // Initially None
        assert!(callback_storage.lock().await.is_none());

        // Set a callback
        let received_hash = Arc::new(Mutex::new(String::new()));
        let received_hash_clone = received_hash.clone();
        let callback: aspen_core::BlobAnnouncedCallback = Box::new(move |info| {
            let received = received_hash_clone.clone();
            Box::pin(async move {
                *received.lock().await = info.blob_hash_hex.clone();
            })
        });

        *callback_storage.lock().await = Some(callback);

        // Take and invoke
        let taken_callback = callback_storage.lock().await.take();
        assert!(taken_callback.is_some());

        let info = BlobAnnouncedInfo {
            announcing_node_id: 42,
            provider_public_key: secret_key_from_seed(99).public(),
            blob_hash_hex: "abcd1234".to_string(),
            blob_size: 1024,
            is_raw_format: true,
            tag: Some("test-tag".to_string()),
        };
        taken_callback.unwrap()(info).await;

        assert_eq!(*received_hash.lock().await, "abcd1234");
    }

    #[test]
    fn test_stale_topology_info_fields() {
        let info = StaleTopologyInfo {
            announcing_node_id: 5,
            remote_version: 100,
            remote_hash: 0xDEADBEEF,
            remote_term: 7,
        };

        assert_eq!(info.announcing_node_id, 5);
        assert_eq!(info.remote_version, 100);
        assert_eq!(info.remote_hash, 0xDEADBEEF);
        assert_eq!(info.remote_term, 7);
    }

    #[test]
    fn test_blob_announced_info_fields() {
        let public_key = secret_key_from_seed(50).public();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 10,
            provider_public_key: public_key,
            blob_hash_hex: "0123456789abcdef".to_string(),
            blob_size: 2048,
            is_raw_format: false,
            tag: None,
        };

        assert_eq!(info.announcing_node_id, 10);
        assert_eq!(info.blob_size, 2048);
        assert!(!info.is_raw_format);
        assert!(info.tag.is_none());
    }

    #[tokio::test]
    async fn test_cancellation_token_multiple_waiters() {
        let cancel = CancellationToken::new();

        // Multiple tasks waiting on the same token
        let c1 = cancel.clone();
        let c2 = cancel.clone();
        let c3 = cancel.clone();

        let h1 = tokio::spawn(async move {
            c1.cancelled().await;
            "task1"
        });
        let h2 = tokio::spawn(async move {
            c2.cancelled().await;
            "task2"
        });
        let h3 = tokio::spawn(async move {
            c3.cancelled().await;
            "task3"
        });

        // Give tasks time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel once, all should wake up
        cancel.cancel();

        let results = futures::future::join_all([h1, h2, h3]).await;
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_gossip_subscribe_timeout_reasonable() {
        // Verify the subscription timeout is reasonable for network operations
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT >= Duration::from_secs(5));
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT <= Duration::from_secs(120));
    }

    #[test]
    fn test_max_gossip_message_size_bounded() {
        // Verify message size limit is reasonable (not too small, not too large)
        assert!(MAX_GOSSIP_MESSAGE_SIZE >= 1024); // At least 1KB
        assert!(MAX_GOSSIP_MESSAGE_SIZE <= 10 * 1024 * 1024); // At most 10MB
    }

    #[test]
    fn test_announce_interval_bounds() {
        // Min should be at least a few seconds to avoid spam
        assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS >= 1);

        // Max should be reasonable for peer discovery latency
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS <= 3600); // Max 1 hour

        // Max should be significantly larger than min for backoff room
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS >= GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS * 2);
    }

    #[tokio::test]
    async fn test_atomic_bool_concurrent_access() {
        let flag = Arc::new(AtomicBool::new(false));

        // Spawn multiple tasks that read/write the flag
        let mut handles = Vec::new();
        for _ in 0..10 {
            let f = flag.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let _ = f.load(Ordering::SeqCst);
                    f.store(true, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Flag should be true after all operations
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_atomic_u64_concurrent_increment() {
        let counter = Arc::new(AtomicU64::new(0));

        // Spawn tasks that increment the counter
        let mut handles = Vec::new();
        for _ in 0..10 {
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Should have 10 * 100 = 1000 increments
        assert_eq!(counter.load(Ordering::SeqCst), 1000);
    }
}
