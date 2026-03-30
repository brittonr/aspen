//! Federation protocol handler and connection management.
//!
//! Handles incoming federation connections via the Iroh QUIC protocol,
//! including stream management, request dispatch, and response serialization.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::FEDERATION_ALPN;
use super::FEDERATION_PROTOCOL_VERSION;
use super::MAX_FEDERATION_CONNECTIONS;
use super::MAX_OBJECTS_PER_SYNC;
use super::MAX_RESOURCES_PER_LIST;
use super::MAX_STREAMS_PER_CONNECTION;
use super::MESSAGE_READ_TIMEOUT;
use super::REQUEST_TIMEOUT;
use super::SYNC_PROCESSING_TIMEOUT;
use super::types::FederationRequest;
use super::types::FederationResponse;
use super::types::ResourceInfo;
use super::wire::read_message;
use super::wire::write_message;
use crate::identity::ClusterIdentity;
use crate::resolver::FederationResourceError;
use crate::resolver::FederationResourceResolver;
use crate::trust::TrustManager;
use crate::types::FederatedId;
use crate::types::FederationSettings;

// ============================================================================
// Protocol Handler
// ============================================================================

/// Context for federation protocol handling.
pub struct FederationProtocolContext {
    /// Our cluster identity.
    pub cluster_identity: ClusterIdentity,
    /// Trust manager.
    pub trust_manager: Arc<TrustManager>,
    /// Federation settings per resource.
    pub resource_settings: Arc<RwLock<HashMap<FederatedId, FederationSettings>>>,
    /// Iroh endpoint.
    pub endpoint: Arc<Endpoint>,
    /// HLC for timestamping.
    pub hlc: Arc<HLC>,
    /// Resource resolver for storage access.
    ///
    /// When set, enables actual data fetching for GetResourceState and SyncObjects.
    /// If None, these handlers return stub responses.
    pub resource_resolver: Option<Arc<dyn FederationResourceResolver>>,
    /// Verified credential from the federation handshake.
    ///
    /// Set during handshake processing when the remote peer presents a valid credential.
    /// Used to authorize subsequent sync requests on the same connection.
    pub session_credential: std::sync::Mutex<Option<aspen_auth::Credential>>,
}

/// Protocol handler for federation sync.
#[derive(Clone)]
pub struct FederationProtocolHandler {
    context: Arc<FederationProtocolContext>,
    connection_semaphore: Arc<Semaphore>,
}

impl std::fmt::Debug for FederationProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FederationProtocolHandler")
            .field("cluster_name", &self.context.cluster_identity.name())
            .finish()
    }
}

impl FederationProtocolHandler {
    /// Create a new federation protocol handler.
    pub fn new(context: FederationProtocolContext) -> Self {
        Self {
            context: Arc::new(context),
            connection_semaphore: Arc::new(Semaphore::new(MAX_FEDERATION_CONNECTIONS as usize)),
        }
    }

    /// Get our cluster identity.
    pub fn cluster_identity(&self) -> &ClusterIdentity {
        &self.context.cluster_identity
    }

    /// Get the trust manager.
    pub fn trust_manager(&self) -> &TrustManager {
        &self.context.trust_manager
    }
}

impl ProtocolHandler for FederationProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!("Federation connection limit reached ({}), rejecting {}", MAX_FEDERATION_CONNECTIONS, remote_id);
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
            }
        };

        debug!(remote = %remote_id, "accepted federation connection");

        // Handle the connection
        let result = handle_federation_connection(connection, self.context.clone()).await;

        // Release permit
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Federation protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a federation connection.
#[instrument(skip(connection, context))]
async fn handle_federation_connection(connection: Connection, context: Arc<FederationProtocolContext>) -> Result<()> {
    let remote_id = connection.remote_id();
    let mut stream_count = 0u32;

    loop {
        // Accept incoming streams
        let stream = match tokio::time::timeout(REQUEST_TIMEOUT, connection.accept_bi()).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                // Connection closed or error
                debug!(remote = %remote_id, error = %e, "federation connection ended");
                break;
            }
            Err(_) => {
                // Timeout - check if connection is still alive
                continue;
            }
        };

        // Check stream limit
        stream_count += 1;
        if stream_count > MAX_STREAMS_PER_CONNECTION {
            warn!(
                remote = %remote_id,
                "Too many streams ({}), closing connection",
                stream_count
            );
            break;
        }

        let (mut send, mut recv) = stream;
        let ctx = context.clone();
        let peer_key = remote_id;

        // Handle stream in background
        tokio::spawn(async move {
            if let Err(e) = handle_federation_stream(&mut send, &mut recv, &ctx, peer_key).await {
                warn!(remote = %remote_id, error = %e, "federation stream error");
            }
        });
    }

    Ok(())
}

/// Handle a single federation stream (request/response).
async fn handle_federation_stream(
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
    context: &FederationProtocolContext,
    remote_peer: PublicKey,
) -> Result<()> {
    // Read request with size limit and per-message timeout
    // Tiger Style: Prevents CPU exhaustion from slow/malicious senders
    let request = tokio::time::timeout(MESSAGE_READ_TIMEOUT, read_message::<FederationRequest>(recv))
        .await
        .context("message read timeout")?
        .context("failed to read federation request")?;

    // Process request with longer timeout — git DAG walks for large repos
    // involve many KV reads and can take tens of seconds for 30K+ object repos
    let response =
        tokio::time::timeout(SYNC_PROCESSING_TIMEOUT, process_federation_request(request, context, remote_peer))
            .await
            .context("sync processing timeout")?
            .context("failed to process federation request")?;

    // Write response
    write_message(send, &response).await?;

    Ok(())
}

/// Process a federation request.
async fn process_federation_request(
    request: FederationRequest,
    context: &FederationProtocolContext,
    remote_peer: PublicKey,
) -> Result<FederationResponse> {
    match request {
        FederationRequest::Handshake {
            identity,
            protocol_version: _,
            capabilities: _,
            credential,
        } => handle_handshake(identity, credential, context),

        FederationRequest::ListResources {
            resource_type,
            cursor: _,
            limit,
        } => handle_list_resources(resource_type, limit, context).await,

        FederationRequest::GetResourceState { fed_id } => {
            if let Some(denied) = check_resource_access(&fed_id, &remote_peer, context).await {
                return Ok(denied);
            }
            handle_get_resource_state(fed_id, context).await
        }

        FederationRequest::SyncObjects {
            fed_id,
            want_types,
            have_hashes,
            limit,
        } => {
            if let Some(denied) = check_resource_access(&fed_id, &remote_peer, context).await {
                return Ok(denied);
            }
            handle_sync_objects(fed_id, want_types, have_hashes, limit, context).await
        }

        FederationRequest::RefreshToken { credential } => handle_refresh_token(credential, context, remote_peer),

        FederationRequest::VerifyRefUpdate {
            fed_id,
            ref_name,
            new_hash,
            signature,
            signer,
        } => handle_verify_ref_update(fed_id, ref_name, new_hash, signature, signer),

        FederationRequest::PushObjects {
            fed_id,
            objects,
            ref_updates,
        } => handle_push_objects(fed_id, objects, ref_updates, context, remote_peer).await,

        FederationRequest::VerifyUpdate {
            fed_id,
            update_type: _,
            key,
            new_value,
            signature,
            signer,
        } => {
            // Generic verify delegates to the same signature check.
            // The `key` field serves the same role as `ref_name`.
            handle_verify_ref_update(fed_id, key, new_value, signature, signer)
        }
    }
}

/// Check if the remote peer is allowed to access a federated resource.
///
/// Returns `Some(FederationResponse::Error)` if denied, `None` if allowed.
///
/// Authorization flow:
/// 1. Blocked peers are always denied (regardless of credential)
/// 2. If session has a credential with `FederationPull` matching the resource, allow
/// 3. Fall back to legacy TrustManager / per-resource settings check
/// 4. Public resources allow everyone; AllowList requires explicit trust or credential
async fn check_resource_access(
    fed_id: &FederatedId,
    remote_peer: &PublicKey,
    context: &FederationProtocolContext,
) -> Option<FederationResponse> {
    // Blocked peers are always denied, regardless of credential
    if context.trust_manager.is_blocked(remote_peer) {
        warn!(
            fed_id = %fed_id.short(),
            remote_peer = %remote_peer,
            "access denied: peer is blocked"
        );
        return Some(FederationResponse::Error {
            code: "ACCESS_DENIED".to_string(),
            message: format!("Access denied for resource: {}", fed_id.short()),
        });
    }

    // Check session credential for FederationPull capability
    let session_cred = context.session_credential.lock().ok().and_then(|guard| guard.clone());

    if let Some(ref cred) = session_cred {
        let fed_id_short = fed_id.short();
        let pull_authorized = cred.token.capabilities.iter().any(|cap| {
            cap.authorizes(&aspen_auth::Operation::FederationPull {
                fed_id: fed_id_short.clone(),
            })
        });
        if pull_authorized {
            return None; // Allowed by FederationPull credential
        }

        // Also check legacy Read-based authorization for backward compatibility
        let settings = context.resource_settings.read().await;
        if let Some(resource_settings) = settings.get(fed_id) {
            if let Some(ref rt) = resource_settings.resource_type {
                let read_authorized = cred
                    .token
                    .capabilities
                    .iter()
                    .any(|cap| cap.authorizes(&aspen_auth::Operation::Read { key: rt.clone() }));
                if read_authorized {
                    return None; // Allowed by legacy Read credential
                }
            }
        } else {
            // Resource not in settings — don't block, let the handler return NotFound
            return None;
        }
    }

    // Fall back to legacy resource settings check
    let settings = context.resource_settings.read().await;
    let Some(resource_settings) = settings.get(fed_id) else {
        // Resource not in settings — don't block, let the handler return NotFound
        return None;
    };

    if !resource_settings.is_cluster_allowed(remote_peer) {
        warn!(
            fed_id = %fed_id.short(),
            remote_peer = %remote_peer,
            mode = ?resource_settings.mode,
            "access denied: peer not allowed for this resource"
        );
        return Some(FederationResponse::Error {
            code: "ACCESS_DENIED".to_string(),
            message: format!("Access denied for resource: {}", fed_id.short()),
        });
    }

    None
}

fn handle_handshake(
    identity: crate::identity::SignedClusterIdentity,
    credential: Option<aspen_auth::Credential>,
    context: &FederationProtocolContext,
) -> Result<FederationResponse> {
    // Verify the peer's identity signature
    if !identity.verify() {
        return Ok(FederationResponse::Error {
            code: "INVALID_SIGNATURE".to_string(),
            message: "Cluster identity signature verification failed".to_string(),
        });
    }

    let peer_key = identity.public_key();

    // If credential is present, verify it with our cluster key as trusted root
    if let Some(ref cred) = credential {
        let our_key = context.cluster_identity.public_key();
        match cred.verify(&[our_key], Some(&peer_key)) {
            Ok(()) => {
                debug!(
                    peer = %peer_key,
                    peer_name = %identity.name(),
                    "federation handshake: credential verified"
                );
                // Store verified credential in session
                if let Ok(mut session) = context.session_credential.lock() {
                    *session = Some(cred.clone());
                }
                // Update trust manager from credential
                context.trust_manager.update_from_credential(peer_key, cred);
            }
            Err(e) => {
                warn!(
                    peer = %peer_key,
                    error = %e,
                    "federation handshake: credential verification failed"
                );
                return Ok(FederationResponse::Error {
                    code: "INVALID_CREDENTIAL".to_string(),
                    message: format!("Credential verification failed: {e}"),
                });
            }
        }
    } else {
        // Legacy handshake — no credential, fall back to TrustManager
        warn!(
            peer = %peer_key,
            peer_name = %identity.name(),
            "federation handshake: no credential (legacy mode, deprecated)"
        );
    }

    let trusted = context.trust_manager.is_trusted(&peer_key);

    debug!(
        peer = %peer_key,
        peer_name = %identity.name(),
        trusted = trusted,
        "federation handshake complete"
    );

    // Return our identity
    Ok(FederationResponse::Handshake {
        identity: context.cluster_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
        trusted,
    })
}

async fn handle_list_resources(
    resource_type: Option<String>,
    limit: u32,
    context: &FederationProtocolContext,
) -> Result<FederationResponse> {
    let limit = limit.min(MAX_RESOURCES_PER_LIST);

    // Try resource resolver first (reads from KV, authoritative source)
    let mut resources: Vec<ResourceInfo> = if let Some(ref resolver) = context.resource_resolver {
        resolver
            .list_resources(limit)
            .await
            .into_iter()
            .map(|(fed_id, rt)| ResourceInfo {
                fed_id,
                resource_type: rt,
                name: fed_id.short(),
                mode: "public".to_string(),
                updated_at_hlc: SerializableTimestamp::from(context.hlc.new_timestamp()),
            })
            .collect()
    } else {
        // Fall back to in-memory settings
        let session_cred = context.session_credential.lock().ok().and_then(|guard| guard.clone());
        let settings = context.resource_settings.read().await;
        settings
            .iter()
            .filter(|(_, s)| {
                if matches!(s.mode, crate::types::FederationMode::Public) {
                    return true;
                }
                if let Some(ref cred) = session_cred {
                    if let Some(ref rt) = s.resource_type {
                        return cred
                            .token
                            .capabilities
                            .iter()
                            .any(|cap| cap.authorizes(&aspen_auth::Operation::Read { key: rt.clone() }));
                    }
                }
                false
            })
            .map(|(fed_id, s)| ResourceInfo {
                fed_id: *fed_id,
                resource_type: s.resource_type.clone().unwrap_or_else(|| "unknown".to_string()),
                name: fed_id.short(),
                mode: "public".to_string(),
                updated_at_hlc: SerializableTimestamp::from(context.hlc.new_timestamp()),
            })
            .take(limit as usize)
            .collect()
    };

    // Filter by resource type if specified
    if let Some(ref rt) = resource_type {
        resources.retain(|r| r.resource_type == *rt);
    }

    Ok(FederationResponse::ResourceList {
        total: Some(resources.len() as u32),
        resources,
        next_cursor: None,
    })
}

async fn handle_get_resource_state(
    fed_id: FederatedId,
    context: &FederationProtocolContext,
) -> Result<FederationResponse> {
    // Use resource resolver if available
    if let Some(ref resolver) = context.resource_resolver {
        return handle_get_resource_state_resolved(&fed_id, resolver.as_ref()).await;
    }

    // Fallback: Check settings only (no actual data fetch)
    let settings = context.resource_settings.read().await;
    if !settings.contains_key(&fed_id) {
        return Ok(FederationResponse::ResourceState {
            was_found: false,
            heads: HashMap::new(),
            metadata: None,
        });
    }

    // No resolver available, return stub response
    Ok(FederationResponse::ResourceState {
        was_found: true,
        heads: HashMap::new(),
        metadata: None,
    })
}

async fn handle_get_resource_state_resolved(
    fed_id: &FederatedId,
    resolver: &dyn FederationResourceResolver,
) -> Result<FederationResponse> {
    match resolver.get_resource_state(fed_id).await {
        Ok(state) => Ok(FederationResponse::ResourceState {
            was_found: state.was_found,
            heads: state.heads,
            metadata: state.metadata,
        }),
        Err(FederationResourceError::NotFound { .. }) => Ok(FederationResponse::ResourceState {
            was_found: false,
            heads: HashMap::new(),
            metadata: None,
        }),
        Err(FederationResourceError::FederationDisabled { fed_id }) => Ok(FederationResponse::Error {
            code: "FEDERATION_DISABLED".to_string(),
            message: format!("Federation is disabled for resource: {}", fed_id),
        }),
        Err(FederationResourceError::ShardNotReady { shard_id, state }) => Ok(FederationResponse::Error {
            code: "SHARD_NOT_READY".to_string(),
            message: format!("Shard {} is {}, retry later", shard_id, state),
        }),
        Err(FederationResourceError::Internal { message }) => {
            warn!(fed_id = %fed_id.short(), error = %message, "internal error getting resource state");
            Ok(FederationResponse::Error {
                code: "INTERNAL_ERROR".to_string(),
                message,
            })
        }
    }
}

async fn handle_sync_objects(
    fed_id: FederatedId,
    want_types: Vec<String>,
    have_hashes: Vec<[u8; 32]>,
    limit: u32,
    context: &FederationProtocolContext,
) -> Result<FederationResponse> {
    let limit = limit.min(MAX_OBJECTS_PER_SYNC);

    // Use resource resolver if available
    if let Some(ref resolver) = context.resource_resolver {
        return handle_sync_objects_resolved(&fed_id, &want_types, &have_hashes, limit, resolver.as_ref()).await;
    }

    // Fallback: Check settings only
    let settings = context.resource_settings.read().await;
    if !settings.contains_key(&fed_id) {
        return Ok(FederationResponse::Error {
            code: "NOT_FOUND".to_string(),
            message: format!("Resource not found: {}", fed_id.short()),
        });
    }

    // No resolver available, return empty response
    Ok(FederationResponse::Objects {
        objects: vec![],
        has_more: false,
    })
}

async fn handle_sync_objects_resolved(
    fed_id: &FederatedId,
    want_types: &[String],
    have_hashes: &[[u8; 32]],
    limit: u32,
    resolver: &dyn FederationResourceResolver,
) -> Result<FederationResponse> {
    match resolver.sync_objects(fed_id, want_types, have_hashes, limit).await {
        Ok(objects) => {
            // For git object sync, the DAG walk may be truncated even when
            // returning fewer objects than the limit. Use a conservative
            // heuristic: has_more is true if we got any objects at all AND
            // at least one non-ref type was requested. The client will send
            // another round; if the server returns empty, the loop stops.
            let wants_git = want_types.iter().any(|t| t == "commit" || t == "tree" || t == "blob");
            let has_more = if wants_git {
                !objects.is_empty()
            } else {
                objects.len() >= limit as usize
            };
            Ok(FederationResponse::Objects { objects, has_more })
        }
        Err(FederationResourceError::NotFound { fed_id }) => Ok(FederationResponse::Error {
            code: "NOT_FOUND".to_string(),
            message: format!("Resource not found: {}", fed_id),
        }),
        Err(FederationResourceError::FederationDisabled { fed_id }) => Ok(FederationResponse::Error {
            code: "FEDERATION_DISABLED".to_string(),
            message: format!("Federation is disabled for resource: {}", fed_id),
        }),
        Err(FederationResourceError::ShardNotReady { shard_id, state }) => Ok(FederationResponse::Error {
            code: "SHARD_NOT_READY".to_string(),
            message: format!("Shard {} is {}, retry later", shard_id, state),
        }),
        Err(FederationResourceError::Internal { message }) => {
            warn!(fed_id = %fed_id.short(), error = %message, "internal error syncing objects");
            Ok(FederationResponse::Error {
                code: "INTERNAL_ERROR".to_string(),
                message,
            })
        }
    }
}

/// Handle a push of objects and ref updates from a remote cluster.
///
/// Authorization: requires the sender to be trusted (via TrustManager or session
/// credential). Public mode is not sufficient for push — only pull.
async fn handle_push_objects(
    fed_id: FederatedId,
    objects: Vec<super::types::SyncObject>,
    ref_updates: Vec<super::types::RefEntry>,
    context: &FederationProtocolContext,
    remote_peer: PublicKey,
) -> Result<FederationResponse> {
    // Blocked peers always denied
    if context.trust_manager.is_blocked(&remote_peer) {
        warn!(
            fed_id = %fed_id.short(),
            remote_peer = %remote_peer,
            "push rejected: peer is blocked"
        );
        return Ok(FederationResponse::Error {
            code: "unauthorized".to_string(),
            message: "Push requires trusted peer or valid credential".to_string(),
        });
    }

    // Check trust: push requires explicit trust or a FederationPush credential
    let trust_level = context.trust_manager.trust_level(&remote_peer);
    let mut push_authorized = trust_level == crate::trust::TrustLevel::Trusted;

    if !push_authorized {
        // Check session credential for FederationPush capability matching this resource
        let session_cred = context.session_credential.lock().ok().and_then(|guard| guard.clone());
        if let Some(ref cred) = session_cred {
            let fed_id_short = fed_id.short();
            push_authorized = cred.token.capabilities.iter().any(|cap| {
                cap.authorizes(&aspen_auth::Operation::FederationPush {
                    fed_id: fed_id_short.clone(),
                })
            });
        }
    }

    if !push_authorized {
        warn!(
            fed_id = %fed_id.short(),
            remote_peer = %remote_peer,
            "push rejected: peer not trusted and no FederationPush credential"
        );
        return Ok(FederationResponse::Error {
            code: "unauthorized".to_string(),
            message: "Push requires trusted peer or FederationPush credential".to_string(),
        });
    }

    // Enforce object limit
    let object_count = objects.len() as u32;
    if object_count > MAX_OBJECTS_PER_SYNC {
        return Ok(FederationResponse::Error {
            code: "limit_exceeded".to_string(),
            message: format!("Push contains {} objects, max is {}", object_count, MAX_OBJECTS_PER_SYNC),
        });
    }

    // Delegate to resource resolver
    let Some(ref resolver) = context.resource_resolver else {
        return Ok(FederationResponse::Error {
            code: "NOT_CONFIGURED".to_string(),
            message: "No resource resolver configured for push import".to_string(),
        });
    };

    match resolver.import_pushed_objects(&fed_id, objects, ref_updates).await {
        Ok((imported, skipped, refs_updated)) => {
            info!(
                fed_id = %fed_id.short(),
                imported,
                skipped,
                refs_updated,
                "push accepted"
            );
            Ok(FederationResponse::PushResult {
                accepted: true,
                imported,
                skipped,
                refs_updated,
                errors: vec![],
            })
        }
        Err(FederationResourceError::NotFound { fed_id }) => Ok(FederationResponse::Error {
            code: "NOT_FOUND".to_string(),
            message: format!("Resource not found: {}", fed_id),
        }),
        Err(FederationResourceError::FederationDisabled { fed_id }) => Ok(FederationResponse::Error {
            code: "FEDERATION_DISABLED".to_string(),
            message: format!("Federation disabled for resource: {}", fed_id),
        }),
        Err(e) => {
            warn!(fed_id = %fed_id.short(), error = %e, "push import failed");
            Ok(FederationResponse::PushResult {
                accepted: false,
                imported: 0,
                skipped: 0,
                refs_updated: 0,
                errors: vec![e.to_string()],
            })
        }
    }
}

fn handle_refresh_token(
    credential: aspen_auth::Credential,
    context: &FederationProtocolContext,
    remote_peer: PublicKey,
) -> Result<FederationResponse> {
    let our_key = context.cluster_identity.public_key();

    // Verify the presented credential
    match credential.verify(&[our_key], Some(&remote_peer)) {
        Ok(()) => {}
        Err(e) => {
            return Ok(FederationResponse::Error {
                code: "INVALID_CREDENTIAL".to_string(),
                message: format!("Credential verification failed: {e}"),
            });
        }
    }

    // Check that the root token was issued by us
    let root_issuer = if credential.proofs.is_empty() {
        credential.token.issuer
    } else {
        // Last proof in chain is the root
        credential.proofs.last().map(|t| t.issuer).unwrap_or(credential.token.issuer)
    };

    if root_issuer != our_key {
        return Ok(FederationResponse::Error {
            code: "WRONG_ISSUER".to_string(),
            message: "Token was not issued by this cluster".to_string(),
        });
    }

    // Issue fresh token with same capabilities and new expiry
    let original_lifetime_secs = credential.token.expires_at.saturating_sub(credential.token.issued_at);
    let lifetime = std::time::Duration::from_secs(if original_lifetime_secs > 0 {
        original_lifetime_secs
    } else {
        86400 // Default 24h
    });

    let fresh_token = aspen_auth::TokenBuilder::new(context.cluster_identity.secret_key().clone())
        .for_key(remote_peer)
        .with_capabilities(credential.token.capabilities.clone())
        .with_lifetime(lifetime)
        .with_random_nonce()
        .build();

    match fresh_token {
        Ok(token) => Ok(FederationResponse::TokenRefreshed { token }),
        Err(e) => Ok(FederationResponse::Error {
            code: "REFRESH_FAILED".to_string(),
            message: format!("Failed to issue fresh token: {e}"),
        }),
    }
}

fn handle_verify_ref_update(
    fed_id: FederatedId,
    ref_name: String,
    new_hash: [u8; 32],
    signature: aspen_core::Signature,
    signer: [u8; 32],
) -> Result<FederationResponse> {
    let signer_key = match PublicKey::from_bytes(&signer) {
        Ok(k) => k,
        Err(_) => {
            return Ok(FederationResponse::VerifyResult {
                is_valid: false,
                error: Some("Invalid signer public key".to_string()),
            });
        }
    };

    // Build the message that was signed
    let mut message = Vec::new();
    message.extend_from_slice(fed_id.origin().as_bytes());
    message.extend_from_slice(fed_id.local_id());
    message.extend_from_slice(ref_name.as_bytes());
    message.extend_from_slice(&new_hash);

    // Verify signature
    let sig_bytes: [u8; 64] = match signature.0.try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(FederationResponse::VerifyResult {
                is_valid: false,
                error: Some("Invalid signature format".to_string()),
            });
        }
    };
    let sig = iroh::Signature::from_bytes(&sig_bytes);

    match signer_key.verify(&message, &sig) {
        Ok(()) => Ok(FederationResponse::VerifyResult {
            is_valid: true,
            error: None,
        }),
        Err(e) => Ok(FederationResponse::VerifyResult {
            is_valid: false,
            error: Some(format!("Signature verification failed: {}", e)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::TrustManager;

    async fn make_context_with_trust(trusted_peer: Option<PublicKey>) -> FederationProtocolContext {
        let identity = crate::identity::ClusterIdentity::generate("test-cluster".to_string());
        let trust_manager = Arc::new(TrustManager::new());
        if let Some(pk) = trusted_peer {
            trust_manager.add_trusted(pk, "trusted-peer".to_string(), None);
        }
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0).bind().await.expect("bind endpoint");
        FederationProtocolContext {
            cluster_identity: identity,
            trust_manager,
            resource_settings: Arc::new(RwLock::new(HashMap::new())),
            endpoint: Arc::new(endpoint),
            hlc: Arc::new(aspen_core::hlc::create_hlc("test")),
            resource_resolver: None,
            session_credential: std::sync::Mutex::new(None),
        }
    }

    #[tokio::test]
    async fn test_push_rejected_from_untrusted_peer() {
        let remote_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let context = make_context_with_trust(None).await; // no trusted peers

        let fed_id = crate::types::FederatedId::new(remote_key, [0xaa; 32]);
        let result = handle_push_objects(fed_id, vec![], vec![], &context, remote_key).await.unwrap();

        match result {
            super::super::types::FederationResponse::Error { code, .. } => {
                assert_eq!(code, "unauthorized");
            }
            other => panic!("expected unauthorized error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_push_limit_exceeded() {
        let remote_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let context = make_context_with_trust(Some(remote_key)).await;

        let fed_id = crate::types::FederatedId::new(remote_key, [0xaa; 32]);
        // Create more objects than MAX_OBJECTS_PER_SYNC
        let objects: Vec<_> = (0..super::MAX_OBJECTS_PER_SYNC + 1)
            .map(|i| super::super::types::SyncObject {
                object_type: "blob".to_string(),
                hash: [i as u8; 32],
                data: vec![i as u8],
                signature: None,
                signer: None,
                envelope_hash: None,

                origin_sha1: None,
            })
            .collect();

        let result = handle_push_objects(fed_id, objects, vec![], &context, remote_key).await.unwrap();

        match result {
            super::super::types::FederationResponse::Error { code, .. } => {
                assert_eq!(code, "limit_exceeded");
            }
            other => panic!("expected limit_exceeded error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_push_no_resolver_returns_not_configured() {
        let remote_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let context = make_context_with_trust(Some(remote_key)).await;

        let fed_id = crate::types::FederatedId::new(remote_key, [0xaa; 32]);
        let result = handle_push_objects(fed_id, vec![], vec![], &context, remote_key).await.unwrap();

        match result {
            super::super::types::FederationResponse::Error { code, .. } => {
                assert_eq!(code, "NOT_CONFIGURED");
            }
            other => panic!("expected NOT_CONFIGURED error, got: {:?}", other),
        }
    }
}
