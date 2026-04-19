//! Client-side federation sync functions.
//!
//! These functions connect to remote federated clusters and perform
//! protocol operations (handshake, resource listing, state queries, object sync).

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint;
use iroh::endpoint::Connection;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::info;
use tracing::warn;

use super::FEDERATION_ALPN;
use super::FEDERATION_PROTOCOL_VERSION;
use super::HANDSHAKE_TIMEOUT;
use super::REQUEST_TIMEOUT;
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

/// Result of a federation handshake, including the peer's advertised capabilities.
pub struct ConnectResult {
    /// The QUIC connection to the peer.
    pub connection: Connection,
    /// The peer's signed cluster identity.
    pub identity: SignedClusterIdentity,
    /// Capabilities the peer advertised in the handshake (e.g., "forge", "streaming-sync").
    pub capabilities: Vec<String>,
}

struct StageContext {
    timeout_context: &'static str,
    error_context: &'static str,
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "federation sync deadlines need an explicit monotonic boundary helper"
)]
fn monotonic_now() -> Instant {
    Instant::now()
}

fn request_deadline(timeout_window: Duration) -> Instant {
    monotonic_now() + timeout_window
}

fn remaining_timeout(deadline: Instant, timeout_context: &'static str) -> Result<Duration> {
    match deadline.checked_duration_since(monotonic_now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        None | Some(_) => Err(anyhow::anyhow!(timeout_context)),
    }
}

async fn run_timed_stage<F, T, E>(stage_timeout: Duration, future: F, context: StageContext) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    timeout(stage_timeout, future)
        .await
        .context(context.timeout_context)?
        .map_err(Into::into)
        .context(context.error_context)
}

async fn run_stage_with_deadline<F, T, E>(deadline: Instant, future: F, context: StageContext) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    run_timed_stage(remaining_timeout(deadline, context.timeout_context)?, future, context).await
}

impl ConnectResult {
    /// Check if the peer supports a specific capability.
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

/// Connect to a federated cluster and perform handshake.
///
/// Returns a [`ConnectResult`] with the QUIC connection, the peer's verified
/// identity, and the capabilities the peer advertised in the handshake
/// response.
///
/// If `credential` is provided, it is sent in the handshake request so the
/// remote handler can authorize subsequent sync operations based on the
/// token's capabilities (e.g., `FederationPull`, `FederationPush`).
async fn read_handshake_response(
    connection: &Connection,
    our_identity: &ClusterIdentity,
    credential: Option<aspen_auth::Credential>,
) -> Result<FederationResponse> {
    let deadline = request_deadline(HANDSHAKE_TIMEOUT);
    let (mut send, mut recv) = run_stage_with_deadline(deadline, connection.open_bi(), StageContext {
        timeout_context: "stream open timeout",
        error_context: "failed to open handshake stream",
    })
    .await?;
    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string(), "streaming-sync".to_string()],
        credential,
    };
    run_stage_with_deadline(deadline, write_message(&mut send, &request), StageContext {
        timeout_context: "request write timeout",
        error_context: "failed to send handshake request",
    })
    .await?;
    send.finish().context("failed to finish handshake send stream")?;
    run_stage_with_deadline(deadline, read_message(&mut recv), StageContext {
        timeout_context: "response timeout",
        error_context: "failed to read handshake response",
    })
    .await
}

fn finalize_handshake_response(connection: Connection, response: FederationResponse) -> Result<ConnectResult> {
    match response {
        FederationResponse::Handshake {
            identity,
            trusted,
            capabilities,
            ..
        } => {
            debug_assert!(!capabilities.is_empty());
            if !identity.verify() {
                connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
                anyhow::bail!("Peer identity verification failed");
            }
            info!(
                peer = %identity.public_key(),
                peer_name = %identity.name(),
                we_are_trusted = trusted,
                peer_capabilities = ?capabilities,
                "federation handshake complete"
            );
            Ok(ConnectResult {
                connection,
                identity,
                capabilities,
            })
        }
        FederationResponse::Error { code, message } => {
            connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
            anyhow::bail!("Handshake failed: {} - {}", code, message)
        }
        other => {
            connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
            anyhow::bail!("Unexpected handshake response: {:?}", other)
        }
    }
}

pub async fn connect_to_cluster(
    endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    peer_addr: impl Into<iroh::EndpointAddr>,
    credential: Option<aspen_auth::Credential>,
) -> Result<ConnectResult> {
    debug_assert!(!our_identity.name().is_empty());
    let connection = timeout(HANDSHAKE_TIMEOUT, endpoint.connect(peer_addr, FEDERATION_ALPN))
        .await
        .context("connection timeout")?
        .context("failed to connect to federated cluster")?;
    let response = match read_handshake_response(&connection, our_identity, credential).await {
        Ok(response) => response,
        Err(error) => {
            connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
            return Err(error);
        }
    };
    finalize_handshake_response(connection, response)
}

/// List resources available on a federated cluster.
pub async fn list_remote_resources(
    connection: &Connection,
    resource_type: Option<&str>,
    limit: u32,
) -> Result<Vec<ResourceInfo>> {
    debug_assert!(limit > 0);
    debug_assert!(resource_type.is_none_or(|value| !value.is_empty()));
    let deadline = request_deadline(REQUEST_TIMEOUT);
    let response: FederationResponse = async {
        let (mut send, mut recv) = run_stage_with_deadline(deadline, connection.open_bi(), StageContext {
            timeout_context: "stream open timeout",
            error_context: "failed to open stream",
        })
        .await?;

        let request = FederationRequest::ListResources {
            resource_type: resource_type.map(|s| s.to_string()),
            cursor: None,
            limit,
        };
        run_stage_with_deadline(deadline, write_message(&mut send, &request), StageContext {
            timeout_context: "request write timeout",
            error_context: "failed to send list resources request",
        })
        .await?;
        send.finish().context("failed to finish send stream")?;

        run_stage_with_deadline(deadline, read_message(&mut recv), StageContext {
            timeout_context: "response timeout",
            error_context: "failed to read list resources response",
        })
        .await
    }
    .await?;

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
    debug_assert!(!fed_id.local_id.is_empty());
    let deadline = request_deadline(REQUEST_TIMEOUT);
    let response: FederationResponse = async {
        let (mut send, mut recv) = run_stage_with_deadline(deadline, connection.open_bi(), StageContext {
            timeout_context: "stream open timeout",
            error_context: "failed to open stream",
        })
        .await?;

        let request = FederationRequest::GetResourceState { fed_id: *fed_id };
        run_stage_with_deadline(deadline, write_message(&mut send, &request), StageContext {
            timeout_context: "request write timeout",
            error_context: "failed to send get resource state request",
        })
        .await?;
        send.finish().context("failed to finish send stream")?;

        run_stage_with_deadline(deadline, read_message(&mut recv), StageContext {
            timeout_context: "response timeout",
            error_context: "failed to read get resource state response",
        })
        .await
    }
    .await?;

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
///
/// # Deprecation
///
/// Prefer [`SyncSession`] for multi-round sync. `SyncSession` reuses a
/// single QUIC bidirectional stream across rounds, avoiding stream
/// exhaustion on long-running syncs. This function opens a new stream
/// per call.
async fn request_sync_objects_response(
    connection: &Connection,
    request: FederationRequest,
) -> Result<FederationResponse> {
    let deadline = request_deadline(REQUEST_TIMEOUT);
    let (mut send, mut recv) = run_stage_with_deadline(deadline, connection.open_bi(), StageContext {
        timeout_context: "stream open timeout",
        error_context: "failed to open stream",
    })
    .await?;
    run_stage_with_deadline(deadline, write_message(&mut send, &request), StageContext {
        timeout_context: "request write timeout",
        error_context: "failed to send sync objects request",
    })
    .await?;
    send.finish().context("failed to finish send stream")?;
    run_stage_with_deadline(deadline, read_message(&mut recv), StageContext {
        timeout_context: "response timeout",
        error_context: "failed to read sync objects response",
    })
    .await
}

fn verify_sync_objects(
    fed_id: &FederatedId,
    objects: Vec<SyncObject>,
    delegates: Option<&[iroh::PublicKey]>,
) -> Vec<SyncObject> {
    let mut verified_objects = Vec::with_capacity(objects.len());
    for obj in objects {
        if !verify_content_hash(&obj.data, &obj.hash) {
            warn!(
                object_type = %obj.object_type,
                expected_hash = %hex::encode(obj.hash),
                "rejected sync object: content hash mismatch"
            );
            continue;
        }
        if let (Some(sig), Some(signer_bytes), Some(valid_delegates)) = (&obj.signature, &obj.signer, delegates) {
            if let Ok(signer_key) = iroh::PublicKey::from_bytes(signer_bytes) {
                if !verify_delegate_signature(fed_id, &obj.object_type, &obj.hash, 0, sig, &signer_key, valid_delegates)
                {
                    warn!(
                        object_type = %obj.object_type,
                        signer = %hex::encode(signer_bytes),
                        "rejected sync object: delegate signature verification failed"
                    );
                    continue;
                }
            } else {
                warn!(object_type = %obj.object_type, "rejected sync object: invalid signer public key");
                continue;
            }
        }
        verified_objects.push(obj);
    }
    verified_objects
}

#[deprecated(note = "use SyncSession::sync_objects() for multi-round sync")]
pub async fn sync_remote_objects(
    connection: &Connection,
    fed_id: &FederatedId,
    want_types: Vec<String>,
    have_hashes: Vec<[u8; 32]>,
    limit: u32,
    delegates: Option<&[iroh::PublicKey]>,
) -> Result<(Vec<SyncObject>, bool)> {
    debug_assert!(limit > 0);
    let request = FederationRequest::SyncObjects {
        fed_id: *fed_id,
        want_types,
        have_hashes,
        limit,
    };
    let response = request_sync_objects_response(connection, request).await?;
    match response {
        FederationResponse::Objects { objects, has_more } => {
            Ok((verify_sync_objects(fed_id, objects, delegates), has_more))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Sync objects failed: {} - {}", code, message)
        }
        other => anyhow::bail!("Unexpected sync response: {:?}", other),
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
    let object_count = objects.len();
    let ref_count = ref_updates.len();
    debug_assert!(object_count <= objects.capacity());
    debug_assert!(ref_count <= ref_updates.capacity());

    let deadline = request_deadline(REQUEST_TIMEOUT);
    let response: FederationResponse = async {
        let (mut send, mut recv) = run_stage_with_deadline(deadline, connection.open_bi(), StageContext {
            timeout_context: "stream open timeout",
            error_context: "failed to open stream for push",
        })
        .await?;

        let request = FederationRequest::PushObjects {
            fed_id: *fed_id,
            objects,
            ref_updates,
        };
        run_stage_with_deadline(deadline, write_message(&mut send, &request), StageContext {
            timeout_context: "request write timeout",
            error_context: "failed to send push request",
        })
        .await?;
        send.finish().context("failed to finish send stream")?;

        run_stage_with_deadline(deadline, read_message(&mut recv), StageContext {
            timeout_context: "response timeout",
            error_context: "failed to read push response",
        })
        .await
    }
    .await?;

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
        let (send, recv) = run_timed_stage(REQUEST_TIMEOUT, connection.open_bi(), StageContext {
            timeout_context: "stream open timeout",
            error_context: "failed to open sync stream",
        })
        .await?;
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
        let deadline = request_deadline(REQUEST_TIMEOUT);
        run_stage_with_deadline(deadline, write_message(&mut self.send, &request), StageContext {
            timeout_context: "request write timeout",
            error_context: "failed to send sync objects request",
        })
        .await?;

        let response: FederationResponse =
            run_stage_with_deadline(deadline, read_message(&mut self.recv), StageContext {
                timeout_context: "response timeout",
                error_context: "failed to read sync objects response",
            })
            .await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_timeout_helper_reports_response_timeout() {
        let deadline = request_deadline(Duration::from_millis(10));
        let result = run_stage_with_deadline(
            deadline,
            std::future::pending::<std::result::Result<(), std::io::Error>>(),
            StageContext {
                timeout_context: "response timeout",
                error_context: "failed to read response",
            },
        )
        .await;

        let error = result.expect_err("pending future must time out");
        assert!(error.to_string().contains("response timeout"));
    }
}
