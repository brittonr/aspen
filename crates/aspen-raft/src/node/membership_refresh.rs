//! Membership address refresh: propagate gossip-discovered address changes into Raft membership.
//!
//! When a node restarts, it binds to a new port (same endpoint ID via `--iroh-secret-key`).
//! Gossip discovers the new address, but the Raft membership log retains the stale
//! `RaftMemberInfo`. This module updates the membership when fresher addresses arrive.
//!
//! Only the leader performs updates. A 60-second debounce prevents log spam.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::RaftNode;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "membership refresh debouncer timestamps recent updates with monotonic time"
)]
#[inline]
fn current_address_update_instant() -> Instant {
    Instant::now()
}

/// Debounce window for membership address updates.
const ADDRESS_UPDATE_DEBOUNCE: Duration = Duration::from_secs(60);

/// Tracks recently submitted address updates to prevent Raft log spam.
///
/// Key: (node_id, address_hash). Value: last update time.
/// Tiger Style: bounded by MAX_PEERS (addresses are only inserted for known members).
pub struct AddressUpdateDebouncer {
    recent: Mutex<HashMap<(NodeId, u64), Instant>>,
}

impl Default for AddressUpdateDebouncer {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressUpdateDebouncer {
    pub fn new() -> Self {
        Self {
            recent: Mutex::new(HashMap::new()),
        }
    }

    /// Check if this (node_id, address_hash) was recently updated.
    /// Returns true if debounced (should skip), false if the update should proceed.
    async fn is_debounced(&self, node_id: NodeId, addr_hash: u64) -> bool {
        let recent = self.recent.lock().await;
        if let Some(last_update) = recent.get(&(node_id, addr_hash)) {
            last_update.elapsed() < ADDRESS_UPDATE_DEBOUNCE
        } else {
            false
        }
    }

    /// Record that an address update was submitted for this (node_id, address_hash).
    async fn record_update(&self, node_id: NodeId, addr_hash: u64) {
        let mut recent = self.recent.lock().await;
        recent.insert((node_id, addr_hash), current_address_update_instant());

        // Evict stale entries to prevent unbounded growth.
        // Tiger Style: bounded cleanup — only evict if map exceeds 128 entries.
        if recent.len() > 128 {
            recent.retain(|_, instant| instant.elapsed() < ADDRESS_UPDATE_DEBOUNCE);
        }
    }
}

/// Hash the socket addresses of an EndpointAddr for debounce keying.
/// Uses a simple FNV-like hash over the debug representation.
fn hash_addrs(addr: &iroh::EndpointAddr) -> u64 {
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    addr.addrs.hash(&mut hasher);
    hasher.finish()
}

impl RaftNode {
    /// Get a member's current address from the Raft membership.
    ///
    /// Returns `None` if the node is not in the current membership.
    pub fn get_member_addr(&self, node_id: NodeId) -> Option<iroh::EndpointAddr> {
        let metrics = self.raft().metrics().borrow().clone();
        let membership = metrics.membership_config.membership();
        membership.get_node(&node_id).map(|info: &RaftMemberInfo| info.iroh_addr.clone())
    }

    /// Update a member's address in the Raft membership.
    ///
    /// Called when gossip discovers a peer with the same endpoint ID but
    /// different socket addresses. Only the leader performs the update.
    ///
    /// Returns `Ok(true)` if the membership was updated, `Ok(false)` if skipped.
    pub async fn try_update_member_address(
        &self,
        debouncer: &AddressUpdateDebouncer,
        target_node: NodeId,
        new_addr: iroh::EndpointAddr,
    ) -> anyhow::Result<bool> {
        // Only the leader can write to the Raft log.
        if !self.is_leader() {
            debug!(
                target_node = %target_node,
                "skipping membership address update (not leader)"
            );
            return Ok(false);
        }

        // Must be initialized.
        if !self.initialized_ref().load(Ordering::Acquire) {
            return Ok(false);
        }

        // Check current membership for this node.
        let current_addr = match self.get_member_addr(target_node) {
            Some(addr) => addr,
            None => {
                debug!(
                    target_node = %target_node,
                    "node not in membership, skipping address update"
                );
                return Ok(false);
            }
        };

        // Same endpoint ID required — different ID means different node, not a restart.
        if current_addr.id != new_addr.id {
            debug!(
                target_node = %target_node,
                current_id = %current_addr.id,
                new_id = %new_addr.id,
                "endpoint ID mismatch, skipping address update"
            );
            return Ok(false);
        }

        // Same socket addresses — nothing to update.
        if current_addr.addrs == new_addr.addrs {
            return Ok(false);
        }

        // Debounce: skip if the same address was recently submitted.
        let addr_hash = hash_addrs(&new_addr);
        if debouncer.is_debounced(target_node, addr_hash).await {
            debug!(
                target_node = %target_node,
                "debounced membership address update (submitted < 60s ago)"
            );
            return Ok(false);
        }

        // Build updated RaftMemberInfo preserving relay_url.
        let metrics = self.raft().metrics().borrow().clone();
        let membership = metrics.membership_config.membership();
        let existing_info = membership.get_node(&target_node);
        let mut updated_info = RaftMemberInfo::new(new_addr.clone());
        if let Some(existing) = existing_info {
            updated_info.relay_url = existing.relay_url.clone();
        }

        info!(
            target_node = %target_node,
            endpoint_id = %new_addr.id,
            old_addrs = current_addr.addrs.len(),
            new_addrs = new_addr.addrs.len(),
            "updating membership address via add_learner"
        );

        // Non-blocking add_learner: updates the Node metadata without
        // changing voter/learner sets. Timeout prevents hanging.
        let update_wait = Duration::from_secs(5);
        match tokio::time::timeout(update_wait, self.raft().add_learner(target_node, updated_info, false)).await {
            Ok(Ok(_)) => {
                debouncer.record_update(target_node, addr_hash).await;
                info!(
                    target_node = %target_node,
                    "membership address updated successfully"
                );
                Ok(true)
            }
            Ok(Err(err)) => {
                // Non-fatal: gossip fallback still works.
                warn!(
                    target_node = %target_node,
                    error = %err,
                    "failed to update membership address (gossip fallback still active)"
                );
                Ok(false)
            }
            Err(_) => {
                warn!(
                    target_node = %target_node,
                    "membership address update timed out after {}s",
                    update_wait.as_secs()
                );
                Ok(false)
            }
        }
    }

    /// Check if this node is the current Raft leader.
    fn is_leader(&self) -> bool {
        let metrics = self.raft().metrics().borrow().clone();
        metrics.current_leader == Some(self.node_id())
    }
}

/// Implement `MembershipAddressUpdater` for `RaftNode`.
///
/// Uses a per-instance debouncer since the trait is stateless — the debouncer
/// is stored alongside the `Arc<RaftNode>` in the gossip callback closure.
#[async_trait]
impl aspen_core::MembershipAddressUpdater for RaftNode {
    async fn update_member_address(&self, node_id: u64, new_endpoint_addr: &[u8]) -> anyhow::Result<bool> {
        let addr: iroh::EndpointAddr = postcard::from_bytes(new_endpoint_addr)
            .map_err(|e| anyhow::anyhow!("failed to deserialize EndpointAddr: {e}"))?;

        // This path is for the trait-based API. The direct method
        // `try_update_member_address` is preferred when the caller has
        // access to the debouncer.
        //
        // Since we can't store state in the trait, this always attempts
        // the update. The gossip_discovery.rs callback uses the direct
        // method with a shared debouncer instead.
        let debouncer = AddressUpdateDebouncer::new();
        self.try_update_member_address(&debouncer, node_id.into(), addr).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_addrs_deterministic() {
        let key = iroh::SecretKey::from_bytes(&[1u8; 32]);
        let addr = iroh::EndpointAddr::new(key.public());
        let h1 = hash_addrs(&addr);
        let h2 = hash_addrs(&addr);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_addrs_empty_vs_nonempty() {
        use std::collections::BTreeSet;

        let key = iroh::SecretKey::from_bytes(&[1u8; 32]);
        let addr1 = iroh::EndpointAddr::new(key.public());

        let mut addr2 = iroh::EndpointAddr::new(key.public());
        let socket_addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
        addr2.addrs = BTreeSet::from([iroh::TransportAddr::Ip(socket_addr)]);

        // Different addrs should produce different hashes
        assert_ne!(hash_addrs(&addr1), hash_addrs(&addr2));
    }

    #[tokio::test]
    async fn test_debouncer_allows_first_update() {
        let debouncer = AddressUpdateDebouncer::new();
        let node_id = NodeId::from(1u64);
        let addr_hash = 12345u64;

        assert!(!debouncer.is_debounced(node_id, addr_hash).await);
    }

    #[tokio::test]
    async fn test_debouncer_blocks_duplicate_within_window() {
        let debouncer = AddressUpdateDebouncer::new();
        let node_id = NodeId::from(1u64);
        let addr_hash = 12345u64;

        debouncer.record_update(node_id, addr_hash).await;
        assert!(debouncer.is_debounced(node_id, addr_hash).await);
    }

    #[tokio::test]
    async fn test_debouncer_allows_different_hash() {
        let debouncer = AddressUpdateDebouncer::new();
        let node_id = NodeId::from(1u64);

        debouncer.record_update(node_id, 111).await;
        // Different hash should not be debounced
        assert!(!debouncer.is_debounced(node_id, 222).await);
    }

    #[tokio::test]
    async fn test_debouncer_allows_different_node() {
        let debouncer = AddressUpdateDebouncer::new();
        let addr_hash = 12345u64;

        debouncer.record_update(NodeId::from(1u64), addr_hash).await;
        // Different node should not be debounced
        assert!(!debouncer.is_debounced(NodeId::from(2u64), addr_hash).await);
    }
}
