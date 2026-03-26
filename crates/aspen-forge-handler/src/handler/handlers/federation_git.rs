//! Federated git operations (git-remote-aspen interop via federation).
//!
//! Handles `FederationGitListRefs` and `FederationGitFetch` RPCs by proxying
//! through the federation sync protocol to the origin cluster. Creates/reuses
//! a local mirror repo for caching.
//!
//! This module is feature-gated with `git-bridge`.

use std::sync::Arc;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::GitBridgeListRefsResponse;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::ForgeNodeRef;
use super::federation::MirrorMetadata;
use super::federation::collect_local_blake3_hashes;
use super::federation::federation_import_objects;
use super::federation::get_or_create_mirror;
use super::federation::update_mirror_refs;
use super::federation::update_mirror_sync_timestamp;

/// Staleness threshold: skip federation sync if mirror was synced within this window.
const MIRROR_STALENESS_SECS: u64 = 30;

/// Mirror metadata KV prefix (same as in federation.rs).
const MIRROR_PREFIX: &str = "_fed:mirror:";

/// Derive a deterministic mirror repo ID from origin key + upstream repo ID.
///
/// The mirror ID is `blake3(origin_key_bytes || upstream_repo_id_bytes)`.
/// This ensures the same federated repo always maps to the same local mirror.
fn derive_mirror_repo_id(origin_key: &str, upstream_repo_hex: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(origin_key.as_bytes());
    if let Ok(bytes) = hex::decode(upstream_repo_hex) {
        hasher.update(&bytes);
    } else {
        hasher.update(upstream_repo_hex.as_bytes());
    }
    *hasher.finalize().as_bytes()
}

/// Build a `FederatedId` string from origin key and repo ID for use with
/// existing mirror infrastructure.
fn build_fed_id_str(origin_key: &str, upstream_repo_hex: &str) -> String {
    format!("{}:{}", origin_key, upstream_repo_hex)
}

/// Check if the mirror is fresh enough to serve without re-syncing.
fn is_mirror_fresh(meta: &MirrorMetadata) -> bool {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    now.saturating_sub(meta.last_sync_timestamp) < MIRROR_STALENESS_SECS
}

/// Look up existing mirror metadata by fed_id_str.
async fn find_mirror_metadata(forge_node: &ForgeNodeRef, fed_id_str: &str) -> Option<MirrorMetadata> {
    let meta_key = format!("{}{}", MIRROR_PREFIX, fed_id_str);
    if let Ok(entries) = forge_node.scan_kv(&meta_key, Some(1)).await
        && let Some((_key, value)) = entries.first()
    {
        MirrorMetadata::from_json(value)
    } else {
        None
    }
}

/// Perform a federation sync (refs + git objects) to populate a mirror repo.
///
/// Connects to the origin cluster, fetches refs and git objects, imports
/// them into the local mirror repo, and updates the sync timestamp.
///
/// Returns `(refs_fetched, objects_imported, errors)`.
async fn sync_from_origin(
    origin_key: &str,
    upstream_repo_hex: &str,
    origin_addr_hint: Option<&str>,
    cluster_identity: &Arc<aspen_cluster::federation::ClusterIdentity>,
    iroh_endpoint: &Arc<iroh::Endpoint>,
    forge_node: &ForgeNodeRef,
    mirror_repo_id: &aspen_forge::identity::RepoId,
) -> Result<(u32, u32, Vec<String>), String> {
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::sync::RefEntry;

    let fed_id_str = build_fed_id_str(origin_key, upstream_repo_hex);

    // Parse the origin public key to build the FederatedId
    let origin_pubkey: iroh::PublicKey =
        origin_key.parse().map_err(|e| format!("invalid origin key '{}': {}", origin_key, e))?;

    let upstream_bytes = hex::decode(upstream_repo_hex)
        .map_err(|e| format!("invalid upstream repo ID '{}': {}", upstream_repo_hex, e))?;
    if upstream_bytes.len() != 32 {
        return Err(format!("upstream repo ID must be 32 bytes, got {}", upstream_bytes.len()));
    }
    let mut local_id = [0u8; 32];
    local_id.copy_from_slice(&upstream_bytes);

    let fed_id = FederatedId::new(origin_pubkey, local_id);

    // Build endpoint address with optional hint
    let endpoint_addr = if let Some(addr_str) = origin_addr_hint {
        if let Ok(socket_addr) = addr_str.parse::<std::net::SocketAddr>() {
            debug!(origin = %origin_key, addr = %socket_addr, "using address hint for origin");
            let mut addr = iroh::EndpointAddr::from(origin_pubkey);
            addr.addrs.insert(iroh::TransportAddr::Ip(socket_addr));
            addr
        } else {
            iroh::EndpointAddr::from(origin_pubkey)
        }
    } else {
        iroh::EndpointAddr::from(origin_pubkey)
    };

    // Connect and handshake
    let (connection, _remote_identity) =
        aspen_cluster::federation::sync::connect_to_cluster(iroh_endpoint, cluster_identity, endpoint_addr)
            .await
            .map_err(|e| format!("connection to origin failed: {e}"))?;

    let mut errors = Vec::new();

    // Phase 1: Fetch refs
    let (ref_objects, _has_more) = aspen_cluster::federation::sync::sync_remote_objects(
        &connection,
        &fed_id,
        vec!["refs".to_string()],
        Vec::new(),
        1000,
        None,
    )
    .await
    .map_err(|e| format!("ref fetch failed: {e}"))?;

    let mut fetched_refs: Vec<(String, [u8; 32])> = Vec::new();
    for obj in &ref_objects {
        if obj.object_type != "ref" {
            continue;
        }
        match postcard::from_bytes::<RefEntry>(&obj.data) {
            Ok(entry) => fetched_refs.push((entry.ref_name, entry.head_hash)),
            Err(e) => errors.push(format!("ref deserialize: {e}")),
        }
    }

    let refs_fetched = fetched_refs.len() as u32;

    // Phase 2: Fetch git objects (incremental)
    let have_hashes = collect_local_blake3_hashes(forge_node, mirror_repo_id, 10_000).await;

    let mut all_git_objects = Vec::new();
    let mut current_have = have_hashes;
    let max_rounds = 10u32;

    for _round in 0..max_rounds {
        let (objects, has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
            &connection,
            &fed_id,
            vec!["commit".to_string(), "tree".to_string(), "blob".to_string()],
            current_have.clone(),
            1000,
            None,
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(origin = %origin_key, error = %e, "git object sync_remote_objects failed");
                errors.push(format!("git object fetch: {e}"));
                break;
            }
        };

        if objects.is_empty() {
            tracing::info!(
                origin = %origin_key,
                round = _round,
                "sync returned empty git objects batch — stopping"
            );
            break;
        }

        for obj in &objects {
            current_have.push(obj.hash);
        }
        all_git_objects.extend(objects);

        if !has_more {
            break;
        }
    }

    // Import git objects into mirror
    let stats = federation_import_objects(forge_node, mirror_repo_id, &all_git_objects).await;
    let objects_imported = stats.commits + stats.trees + stats.blobs;
    errors.extend(stats.errors.iter().cloned());

    // Translate remote ref hashes to local BLAKE3 hashes.
    //
    // Remote refs point to Alice's envelope BLAKE3 (SignedObject::hash()),
    // but the local import creates new SignedObjects with different hashes.
    // Match by finding the commit SyncObject for each ref: compute its
    // content hash, look up the locally imported BLAKE3 in the mapping.
    let translated_refs = translate_ref_hashes(&fetched_refs, &all_git_objects, &stats);

    // Update mirror refs with locally imported hashes
    if let Err(e) = update_mirror_refs(forge_node, mirror_repo_id, &translated_refs).await {
        errors.push(format!("mirror ref update: {e}"));
    }

    // Update sync timestamp
    let _ = update_mirror_sync_timestamp(forge_node, &fed_id_str).await;

    info!(
        origin = %origin_key,
        refs = refs_fetched,
        objects = objects_imported,
        "federation git sync complete"
    );

    Ok((refs_fetched, objects_imported, errors))
}

/// Translate remote ref hashes to locally imported BLAKE3 hashes.
///
/// Remote refs point to the origin's envelope BLAKE3 hashes. After local
/// import, objects have different envelope hashes. This function finds the
/// commit SyncObject for each ref by matching content, then looks up the
/// locally imported BLAKE3 via the content hash mapping.
///
/// For refs that can't be translated (no matching commit found), falls back
/// to the original hash (which won't resolve locally, but at least the ref
/// exists for later re-sync).
fn translate_ref_hashes(
    fetched_refs: &[(String, [u8; 32])],
    git_objects: &[aspen_cluster::federation::sync::SyncObject],
    stats: &super::federation::FederationImportStats,
) -> Vec<(String, [u8; 32])> {
    // Build content_hash → SyncObject index for commits only
    let commit_content_hashes: Vec<[u8; 32]> = git_objects
        .iter()
        .filter(|o| o.object_type == "commit")
        .map(|o| {
            let h: [u8; 32] = blake3::hash(&o.data).into();
            h
        })
        .collect();

    let mut translated = Vec::with_capacity(fetched_refs.len());

    for (ref_name, _remote_hash) in fetched_refs {
        // Try each commit's content hash to find a local BLAKE3 match.
        // For single-branch repos, there's typically one commit.
        let mut found = false;
        for content_hash in &commit_content_hashes {
            if let Some(local_blake3) = stats.content_to_local_blake3.get(content_hash) {
                translated.push((ref_name.clone(), *local_blake3.as_bytes()));
                tracing::info!(
                    ref_name = %ref_name,
                    local_blake3 = %hex::encode(local_blake3.as_bytes()),
                    "translated ref to local BLAKE3"
                );
                found = true;
                break;
            }
        }
        if !found {
            // Fallback: use remote hash (won't resolve, but ref is recorded)
            tracing::warn!(
                ref_name = %ref_name,
                "could not translate ref to local BLAKE3, using remote hash"
            );
            translated.push((ref_name.clone(), *_remote_hash));
        }
    }

    translated
}

/// Handle `FederationGitListRefs` — list refs for a repo on a remote cluster.
///
/// 1. Derive deterministic mirror repo ID
/// 2. Check if mirror exists and is fresh (<30s old)
/// 3. If stale or missing, sync from origin via federation
/// 4. Return refs from the local mirror (delegating to the regular git bridge)
pub(crate) async fn handle_federation_git_list_refs(
    origin_key: &str,
    upstream_repo_hex: &str,
    origin_addr_hint: Option<&str>,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    let identity = match cluster_identity {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
                is_success: false,
                refs: vec![],
                head: None,
                error: Some("federation not configured on this cluster".to_string()),
            }));
        }
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => {
            return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
                is_success: false,
                refs: vec![],
                head: None,
                error: Some("iroh endpoint not available".to_string()),
            }));
        }
    };

    let fed_id_str = build_fed_id_str(origin_key, upstream_repo_hex);
    let mirror_bytes = derive_mirror_repo_id(origin_key, upstream_repo_hex);
    let mirror_repo_id = aspen_forge::identity::RepoId(mirror_bytes);

    // Check existing mirror
    let existing = find_mirror_metadata(forge_node, &fed_id_str).await;

    let needs_sync = match &existing {
        Some(meta) if is_mirror_fresh(meta) => {
            debug!(fed_id = %fed_id_str, "mirror is fresh, serving from cache");
            false
        }
        Some(_) => {
            debug!(fed_id = %fed_id_str, "mirror is stale, re-syncing");
            true
        }
        None => {
            debug!(fed_id = %fed_id_str, "no mirror exists, creating and syncing");
            // Create the mirror repo via the existing helper
            if let Err(e) = get_or_create_mirror(forge_node, &fed_id_str, origin_key).await {
                return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
                    is_success: false,
                    refs: vec![],
                    head: None,
                    error: Some(format!("failed to create mirror: {e}")),
                }));
            }
            true
        }
    };

    if needs_sync {
        match sync_from_origin(
            origin_key,
            upstream_repo_hex,
            origin_addr_hint,
            identity,
            endpoint,
            forge_node,
            &mirror_repo_id,
        )
        .await
        {
            Ok((refs, objects, errors)) => {
                if !errors.is_empty() {
                    warn!(errors = ?errors, "federation sync completed with errors");
                }
                debug!(refs = refs, objects = objects, "federation sync populated mirror");
            }
            Err(e) => {
                // If we have a stale mirror, serve it anyway with a warning
                if existing.is_some() {
                    warn!(error = %e, "federation sync failed, serving stale mirror");
                } else {
                    return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
                        is_success: false,
                        refs: vec![],
                        head: None,
                        error: Some(format!("origin unreachable: {e}")),
                    }));
                }
            }
        }
    }

    // Serve refs from the local mirror using the regular git bridge handler
    let mirror_hex = hex::encode(mirror_repo_id.0);
    let result = super::git_bridge::handle_git_bridge_list_refs(forge_node, mirror_hex).await?;

    // Re-wrap as FederationGitListRefs response
    match result {
        ClientRpcResponse::GitBridgeListRefs(resp) => Ok(ClientRpcResponse::FederationGitListRefs(resp)),
        other => Ok(other),
    }
}

/// Handle `FederationGitFetch` — fetch git objects for a federated repo.
///
/// Serves from the local mirror. If the mirror doesn't have the requested
/// objects, triggers a federation sync first.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_federation_git_fetch(
    origin_key: &str,
    upstream_repo_hex: &str,
    want: Vec<String>,
    have: Vec<String>,
    origin_addr_hint: Option<&str>,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgeFetchResponse;

    let identity = match cluster_identity {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederationGitFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some("federation not configured on this cluster".to_string()),
            }));
        }
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => {
            return Ok(ClientRpcResponse::FederationGitFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some("iroh endpoint not available".to_string()),
            }));
        }
    };

    let mirror_bytes = derive_mirror_repo_id(origin_key, upstream_repo_hex);
    let mirror_repo_id = aspen_forge::identity::RepoId(mirror_bytes);
    let mirror_hex = hex::encode(mirror_repo_id.0);

    // Try serving from mirror first
    let result =
        super::git_bridge::handle_git_bridge_fetch(forge_node, mirror_hex.clone(), want.clone(), have.clone()).await?;

    // Check if all objects were found
    let needs_sync = match &result {
        ClientRpcResponse::GitBridgeFetch(resp) => !resp.is_success || resp.objects.is_empty(),
        _ => true,
    };

    if needs_sync {
        debug!(
            origin = %origin_key,
            "mirror missing objects, syncing from origin"
        );

        // Sync from origin
        if let Err(e) = sync_from_origin(
            origin_key,
            upstream_repo_hex,
            origin_addr_hint,
            identity,
            endpoint,
            forge_node,
            &mirror_repo_id,
        )
        .await
        {
            return Ok(ClientRpcResponse::FederationGitFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("federation sync failed: {e}")),
            }));
        }

        // Retry from mirror after sync
        let retry = super::git_bridge::handle_git_bridge_fetch(forge_node, mirror_hex, want, have).await?;

        return match retry {
            ClientRpcResponse::GitBridgeFetch(resp) => Ok(ClientRpcResponse::FederationGitFetch(resp)),
            other => Ok(other),
        };
    }

    // Re-wrap as FederationGitFetch response
    match result {
        ClientRpcResponse::GitBridgeFetch(resp) => Ok(ClientRpcResponse::FederationGitFetch(resp)),
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_mirror_repo_id_deterministic() {
        let origin = "a".repeat(52);
        let repo = "bb".repeat(32);

        let id1 = derive_mirror_repo_id(&origin, &repo);
        let id2 = derive_mirror_repo_id(&origin, &repo);

        assert_eq!(id1, id2, "same inputs must produce same mirror ID");
    }

    #[test]
    fn test_derive_mirror_repo_id_different_origins() {
        let repo = "cc".repeat(32);

        let id1 = derive_mirror_repo_id(&"a".repeat(52), &repo);
        let id2 = derive_mirror_repo_id(&"b".repeat(52), &repo);

        assert_ne!(id1, id2, "different origin keys must produce different mirror IDs");
    }

    #[test]
    fn test_derive_mirror_repo_id_different_repos() {
        let origin = "a".repeat(52);

        let id1 = derive_mirror_repo_id(&origin, &"aa".repeat(32));
        let id2 = derive_mirror_repo_id(&origin, &"bb".repeat(32));

        assert_ne!(id1, id2, "different repo IDs must produce different mirror IDs");
    }

    #[test]
    fn test_build_fed_id_str() {
        let origin = "abc123";
        let repo = "def456";
        assert_eq!(build_fed_id_str(origin, repo), "abc123:def456");
    }
}
