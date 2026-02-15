//! PeerDiscovery trait implementation for GossipPeerDiscovery.

use std::sync::atomic::Ordering;

use anyhow::Result;
use aspen_core::DiscoveryHandle;
use aspen_core::PeerDiscoveredCallback;
use aspen_core::PeerDiscovery;
use async_trait::async_trait;
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;

use super::lifecycle::GossipPeerDiscovery;

// ============================================================================
// PeerDiscovery Trait Implementation
// ============================================================================

#[async_trait]
impl PeerDiscovery for GossipPeerDiscovery {
    type Address = EndpointAddr;
    type TopicId = TopicId;

    fn topic_id(&self) -> &Self::TopicId {
        &self.topic_id
    }

    async fn start(
        &self,
        on_peer_discovered: Option<PeerDiscoveredCallback<Self::Address>>,
    ) -> Result<DiscoveryHandle> {
        self.start_internal(on_peer_discovered).await?;
        Ok(DiscoveryHandle::new(self.cancel_token.clone()))
    }

    async fn announce(&self) -> Result<()> {
        self.broadcast_announcement().await
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}
