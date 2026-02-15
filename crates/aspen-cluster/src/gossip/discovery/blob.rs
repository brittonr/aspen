//! Blob announcement broadcasting for gossip peer discovery.

use anyhow::Context;
use anyhow::Result;
use aspen_raft_types::NodeId;
use iroh::EndpointAddr;
use iroh::SecretKey;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;

use crate::gossip::types::*;

/// Parameters for broadcasting a blob announcement.
///
/// Groups related parameters to avoid too many function arguments.
#[cfg(feature = "blob")]
pub struct BlobAnnouncementParams {
    /// Our node ID
    pub node_id: NodeId,
    /// Our endpoint address
    pub endpoint_addr: EndpointAddr,
    /// BLAKE3 hash of the blob
    pub blob_hash: iroh_blobs::Hash,
    /// Size of the blob in bytes
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq)
    pub blob_format: iroh_blobs::BlobFormat,
    /// Optional categorization tag
    pub tag: Option<String>,
}

/// Broadcast a blob announcement to the gossip network.
///
/// This announces that the local node has a blob available for P2P download.
/// Other nodes can use this information to fetch the blob for redundancy
/// or when they need the content.
///
/// # Arguments
/// * `gossip` - The gossip instance
/// * `topic_id` - The gossip topic to broadcast on
/// * `secret_key` - Secret key for signing
/// * `params` - Blob announcement parameters
///
/// # Returns
/// Ok(()) on success, Err if broadcast fails
#[cfg(feature = "blob")]
pub async fn broadcast_blob_announcement(
    gossip: &Gossip,
    topic_id: TopicId,
    secret_key: &SecretKey,
    params: BlobAnnouncementParams,
) -> Result<()> {
    // Create and sign the announcement
    let announcement = BlobAnnouncement::new(
        params.node_id,
        params.endpoint_addr,
        params.blob_hash,
        params.blob_size,
        params.blob_format,
        params.tag,
    )?;
    let signed = SignedBlobAnnouncement::sign(announcement, secret_key)?;
    let message = GossipMessage::BlobAnnouncement(signed);
    let bytes = message.to_bytes()?;

    // Subscribe briefly to get a sender, then broadcast
    // Note: This creates a new subscription which may not be ideal for frequent announcements.
    // For high-frequency blob announcements, consider caching the topic/sender.
    let mut topic = gossip.subscribe(topic_id, vec![]).await.context("failed to subscribe to gossip topic")?;
    topic.broadcast(bytes.into()).await.context("failed to broadcast blob announcement")?;

    tracing::debug!(
        hash = %params.blob_hash.fmt_short(),
        size = params.blob_size,
        format = ?params.blob_format,
        "broadcast blob announcement"
    );

    Ok(())
}
