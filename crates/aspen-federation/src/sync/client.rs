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
pub async fn connect_to_cluster(
    endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    peer_id: iroh::PublicKey,
) -> Result<(Connection, SignedClusterIdentity)> {
    // Connect to peer
    let connection =
        endpoint.connect(peer_id, FEDERATION_ALPN).await.context("failed to connect to federated cluster")?;

    // Open a stream for handshake
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open handshake stream")?;

    // Send handshake request
    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
    };
    write_message(&mut send, &request).await?;

    // Read handshake response
    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Handshake { identity, trusted, .. } => {
            // Verify peer's identity
            if !identity.verify() {
                anyhow::bail!("Peer identity verification failed");
            }

            info!(
                peer = %identity.public_key(),
                peer_name = %identity.name(),
                we_are_trusted = trusted,
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
