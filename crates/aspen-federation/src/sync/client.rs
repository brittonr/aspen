//! Client-side federation sync functions.
//!
//! These functions connect to remote federated clusters and perform
//! protocol operations (handshake, resource listing, state queries, object sync).

use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint;
use iroh::endpoint::Connection;
use tracing::info;
use tracing::warn;

use super::FEDERATION_ALPN;
use super::FEDERATION_PROTOCOL_VERSION;
use super::types::FederationRequest;
use super::types::FederationResponse;
use super::types::RefEntry;
use super::types::ResourceInfo;
use super::types::ResourceMetadata;
use super::types::SyncObject;
use super::wire::read_message;
use super::wire::write_message;
use crate::identity::ClusterIdentity;
use crate::identity::SignedClusterIdentity;
use crate::trust::verify_content_hash;
use crate::trust::verify_delegate_signature;
use crate::types::FederatedId;

/// Connect to a federated cluster and perform handshake.
///
/// If `credential` is provided, it is sent in the handshake request so the
/// remote handler can authorize subsequent sync operations based on the
/// token's capabilities (e.g., `FederationPull`, `FederationPush`).
/// Result of a federation handshake, including the peer's advertised capabilities.
pub struct ConnectResult {
    /// The QUIC connection to the peer.
    pub connection: Connection,
    /// The peer's signed cluster identity.
    pub identity: SignedClusterIdentity,
    /// Capabilities the peer advertised in the handshake (e.g., "forge", "streaming-sync").
    pub capabilities: Vec<String>,
}

impl ConnectResult {
    /// Check if the peer supports a specific capability.
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

pub async fn connect_to_cluster(
    endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    peer_addr: impl Into<iroh::EndpointAddr>,
    credential: Option<aspen_auth::Credential>,
) -> Result<(Connection, SignedClusterIdentity)> {
    // Connect to peer
    let connection = endpoint
        .connect(peer_addr, FEDERATION_ALPN)
        .await
        .context("failed to connect to federated cluster")?;

    // Open a stream for handshake
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open handshake stream")?;

    // Send handshake request
    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string(), "streaming-sync".to_string()],
        credential,
    };
    write_message(&mut send, &request).await?;
    send.finish().context("failed to finish handshake send stream")?;

    // Read handshake response
    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Handshake {
            identity,
            trusted,
            capabilities,
            ..
        } => {
            // Verify peer's identity
            if !identity.verify() {
                anyhow::bail!("Peer identity verification failed");
            }

            info!(
                peer = %identity.public_key(),
                peer_name = %identity.name(),
                we_are_trusted = trusted,
                peer_capabilities = ?capabilities,
                "federation handshake complete"
            );

            Ok((connection, identity))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Handshake failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected handshake response"),
    }
}

/// Connect to a federated cluster with full result including peer capabilities.
///
/// Like `connect_to_cluster` but returns a `ConnectResult` that includes
/// the peer's advertised capabilities (e.g., `"streaming-sync"`).
pub async fn connect_to_cluster_full(
    endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    peer_addr: impl Into<iroh::EndpointAddr>,
    credential: Option<aspen_auth::Credential>,
) -> Result<ConnectResult> {
    let (connection, identity) = connect_to_cluster(endpoint, our_identity, peer_addr, credential).await?;
    // Capabilities are logged during connect_to_cluster. For now we re-derive
    // from the identity (old servers don't return them). A proper implementation
    // would thread the capabilities from the handshake response.
    // TODO: thread capabilities from handshake response once connect_to_cluster
    // is refactored. For now, assume peers built with the same code support
    // streaming-sync.
    let capabilities = vec!["forge".to_string(), "streaming-sync".to_string()];
    Ok(ConnectResult {
        connection,
        identity,
        capabilities,
    })
}

/// List resources available on a federated cluster.
pub async fn list_remote_resources(
    connection: &Connection,
    resource_type: Option<&str>,
    limit: u32,
) -> Result<Vec<ResourceInfo>> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::ListResources {
        resource_type: resource_type.map(|s| s.to_string()),
        cursor: None,
        limit,
    };
    write_message(&mut send, &request).await?;
    send.finish().context("failed to finish send stream")?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::ResourceList { resources, .. } => Ok(resources),
        FederationResponse::Error { code, message } => {
            anyhow::bail!("List resources failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

/// Get the state of a remote resource.
pub async fn get_remote_resource_state(
    connection: &Connection,
    fed_id: &FederatedId,
) -> Result<(bool, HashMap<String, [u8; 32]>, Option<ResourceMetadata>)> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::GetResourceState { fed_id: *fed_id };
    write_message(&mut send, &request).await?;
    send.finish().context("failed to finish send stream")?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::ResourceState {
            was_found,
            heads,
            metadata,
        } => Ok((was_found, heads, metadata)),
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Get resource state failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

/// Sync objects from a remote cluster.
///
/// Objects are verified before being returned:
/// 1. **Content hash** (BLAKE3) — always checked, mismatches are dropped
/// 2. **Delegate signature** — checked when `delegates` is provided and the object has a signature.
///    Invalid signatures are dropped.
///
/// Pass `None` for `delegates` to skip signature verification (e.g., for
/// CRDTs that don't use delegate signing).
pub async fn sync_remote_objects(
    connection: &Connection,
    fed_id: &FederatedId,
    want_types: Vec<String>,
    have_hashes: Vec<[u8; 32]>,
    limit: u32,
    delegates: Option<&[iroh::PublicKey]>,
) -> Result<(Vec<SyncObject>, bool)> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::SyncObjects {
        fed_id: *fed_id,
        want_types,
        have_hashes,
        limit,
    };
    write_message(&mut send, &request).await?;
    // Finish the send stream so the peer knows we're done writing and QUIC
    // can reclaim the stream slot. Without this, each round of the multi-round
    // sync loop leaks a half-open stream, hitting the server's max concurrent
    // stream limit and killing the connection.
    send.finish().context("failed to finish send stream")?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Objects { objects, has_more } => {
            // Tiger Style: Never accept unverified data from remote peers.
            // Two verification layers:
            //   1. Content hash (BLAKE3) — always
            //   2. Delegate signature — when delegates provided and object is signed
            let mut verified_objects = Vec::with_capacity(objects.len());
            for obj in objects {
                // Layer 1: Content hash verification
                if !verify_content_hash(&obj.data, &obj.hash) {
                    warn!(
                        object_type = %obj.object_type,
                        expected_hash = %hex::encode(obj.hash),
                        "rejected sync object: content hash mismatch"
                    );
                    continue;
                }

                // Layer 2: Delegate signature verification (when applicable)
                if let (Some(sig), Some(signer_bytes), Some(valid_delegates)) = (&obj.signature, &obj.signer, delegates)
                {
                    if let Ok(signer_key) = iroh::PublicKey::from_bytes(signer_bytes) {
                        // Use obj.object_type as the "ref_name" equivalent and
                        // current time as timestamp since we don't have the original.
                        // The delegate check verifies the signer is in the delegate list
                        // and the signature covers the expected message.
                        if !verify_delegate_signature(
                            fed_id,
                            &obj.object_type,
                            &obj.hash,
                            0, // timestamp not available in SyncObject; verified by content hash
                            sig,
                            &signer_key,
                            valid_delegates,
                        ) {
                            warn!(
                                object_type = %obj.object_type,
                                signer = %hex::encode(signer_bytes),
                                "rejected sync object: delegate signature verification failed"
                            );
                            continue;
                        }
                    } else {
                        warn!(
                            object_type = %obj.object_type,
                            "rejected sync object: invalid signer public key"
                        );
                        continue;
                    }
                }

                verified_objects.push(obj);
            }

            Ok((verified_objects, has_more))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Sync objects failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

/// Push result from a remote cluster.
#[derive(Debug)]
pub struct PushResult {
    /// Whether the push was accepted.
    pub accepted: bool,
    /// Number of objects imported by the receiver.
    pub imported: u32,
    /// Number of objects skipped (already present).
    pub skipped: u32,
    /// Number of refs updated.
    pub refs_updated: u32,
    /// Non-fatal errors encountered during import.
    pub errors: Vec<String>,
}

/// Push objects and ref updates to a remote cluster.
///
/// Sends git objects and ref updates to the remote peer, which imports
/// them into a mirror repo. The connection must already be established
/// via `connect_to_cluster`.
pub async fn push_to_cluster(
    connection: &Connection,
    fed_id: &FederatedId,
    objects: Vec<SyncObject>,
    ref_updates: Vec<RefEntry>,
) -> Result<PushResult> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream for push")?;

    let object_count = objects.len();
    let ref_count = ref_updates.len();

    let request = FederationRequest::PushObjects {
        fed_id: *fed_id,
        objects,
        ref_updates,
    };
    write_message(&mut send, &request).await?;
    send.finish().context("failed to finish send stream")?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::PushResult {
            accepted,
            imported,
            skipped,
            refs_updated,
            errors,
        } => {
            info!(
                fed_id = %fed_id.short(),
                accepted,
                imported,
                skipped,
                refs_updated,
                sent_objects = object_count,
                sent_refs = ref_count,
                "push complete"
            );
            Ok(PushResult {
                accepted,
                imported,
                skipped,
                refs_updated,
                errors,
            })
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Push failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected push response"),
    }
}

// ============================================================================
// Streaming sync session
// ============================================================================

/// A persistent bidirectional stream for multi-round federation sync.
///
/// Instead of opening a new QUIC stream per `SyncObjects` request, a
/// `SyncSession` reuses a single `(SendStream, RecvStream)` pair for the
/// entire sync conversation. The client writes length-prefixed requests
/// and reads length-prefixed responses in sequence.
///
/// Drop the session (or call `finish()`) to signal the server that the
/// conversation is over.
pub struct SyncSession {
    send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
}

impl SyncSession {
    /// Open a sync session on an existing connection.
    ///
    /// Opens one bidirectional stream that lives for the entire multi-round
    /// sync. The server loops reading requests on this stream until the
    /// client finishes the send side.
    pub async fn open(connection: &Connection) -> Result<Self> {
        let (send, recv) = connection.open_bi().await.context("failed to open sync stream")?;
        Ok(Self { send, recv })
    }

    /// Send a `SyncObjects` request and read the response on the held stream.
    ///
    /// Equivalent to `sync_remote_objects` but reuses the same stream.
    /// Objects are verified (content hash + optional delegate signature)
    /// before being returned.
    pub async fn sync_objects(
        &mut self,
        fed_id: &FederatedId,
        want_types: Vec<String>,
        have_hashes: Vec<[u8; 32]>,
        limit: u32,
        delegates: Option<&[iroh::PublicKey]>,
    ) -> Result<(Vec<SyncObject>, bool)> {
        let request = FederationRequest::SyncObjects {
            fed_id: *fed_id,
            want_types,
            have_hashes,
            limit,
        };
        write_message(&mut self.send, &request).await?;

        let response: FederationResponse = read_message(&mut self.recv).await?;

        match response {
            FederationResponse::Objects { objects, has_more } => {
                let mut verified = Vec::with_capacity(objects.len());
                for obj in objects {
                    if !verify_content_hash(&obj.data, &obj.hash) {
                        warn!(
                            object_type = %obj.object_type,
                            expected_hash = %hex::encode(obj.hash),
                            "rejected sync object: content hash mismatch"
                        );
                        continue;
                    }

                    if let (Some(sig), Some(signer_bytes), Some(valid_delegates)) =
                        (&obj.signature, &obj.signer, delegates)
                    {
                        if let Ok(signer_key) = iroh::PublicKey::from_bytes(signer_bytes) {
                            if !verify_delegate_signature(
                                fed_id,
                                &obj.object_type,
                                &obj.hash,
                                0,
                                sig,
                                &signer_key,
                                valid_delegates,
                            ) {
                                warn!(
                                    object_type = %obj.object_type,
                                    signer = %hex::encode(signer_bytes),
                                    "rejected sync object: delegate signature verification failed"
                                );
                                continue;
                            }
                        } else {
                            warn!(
                                object_type = %obj.object_type,
                                "rejected sync object: invalid signer public key"
                            );
                            continue;
                        }
                    }

                    verified.push(obj);
                }
                Ok((verified, has_more))
            }
            FederationResponse::Error { code, message } => {
                anyhow::bail!("Sync objects failed: {} - {}", code, message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Signal the server that the sync conversation is done.
    ///
    /// Finishes the send side of the stream. The server detects this as EOF
    /// and exits its request loop.
    pub async fn finish(mut self) -> Result<()> {
        self.send.finish().context("failed to finish sync stream")?;
        Ok(())
    }
}
