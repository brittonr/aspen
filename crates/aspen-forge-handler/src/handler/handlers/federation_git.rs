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
use aspen_cluster::federation::sync::RefEntry;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::ForgeNodeRef;
use super::federation::MirrorMetadata;
// collect_local_sha1_hashes is called via super::federation:: prefix at call sites
use super::federation::federation_import_objects;
use super::federation::get_or_create_mirror;
use super::federation::update_mirror_refs;
use super::federation::update_mirror_sync_timestamp;

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

/// Build a FederatedId and endpoint address from origin parameters.
fn build_origin_params(
    origin_key: &str,
    upstream_repo_hex: &str,
    origin_addr_hint: Option<&str>,
) -> Result<(aspen_cluster::federation::FederatedId, iroh::EndpointAddr), String> {
    use aspen_cluster::federation::FederatedId;

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

    Ok((fed_id, endpoint_addr))
}

/// Fetch only refs from the origin cluster (fast — no object transfer).
///
/// Returns the fetched refs with their SHA1 commit hashes.
async fn fetch_refs_from_origin(
    origin_key: &str,
    upstream_repo_hex: &str,
    origin_addr_hint: Option<&str>,
    cluster_identity: &Arc<aspen_cluster::federation::ClusterIdentity>,
    iroh_endpoint: &Arc<iroh::Endpoint>,
) -> Result<Vec<FetchedRef>, String> {
    let (fed_id, endpoint_addr) = build_origin_params(origin_key, upstream_repo_hex, origin_addr_hint)?;

    let (connection, _remote_identity) =
        aspen_cluster::federation::sync::connect_to_cluster(iroh_endpoint, cluster_identity, endpoint_addr, None)
            .await
            .map_err(|e| format!("connection to origin failed: {e}"))?;

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

    let mut fetched_refs = Vec::new();
    for obj in &ref_objects {
        if obj.object_type != "ref" {
            continue;
        }
        if let Ok(entry) = postcard::from_bytes::<RefEntry>(&obj.data) {
            fetched_refs.push((entry.ref_name, entry.head_hash, entry.commit_sha1));
        }
    }

    Ok(fetched_refs)
}

/// Perform a full federation sync (refs + git objects) to populate a mirror repo.
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
    let fed_id_str = build_fed_id_str(origin_key, upstream_repo_hex);
    let (fed_id, endpoint_addr) = build_origin_params(origin_key, upstream_repo_hex, origin_addr_hint)?;

    let (connection, _remote_identity) =
        aspen_cluster::federation::sync::connect_to_cluster(iroh_endpoint, cluster_identity, endpoint_addr, None)
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

    let mut fetched_refs: Vec<FetchedRef> = Vec::new();
    for obj in &ref_objects {
        if obj.object_type != "ref" {
            continue;
        }
        match postcard::from_bytes::<RefEntry>(&obj.data) {
            Ok(entry) => fetched_refs.push((entry.ref_name, entry.head_hash, entry.commit_sha1)),
            Err(e) => errors.push(format!("ref deserialize: {e}")),
        }
    }

    let refs_fetched = fetched_refs.len() as u32;

    // Phase 2: Fetch and import git objects incrementally.
    //
    // Each round fetches a batch from the origin, imports it into the local
    // mirror, then tells the origin what we have for the next round. This
    // builds up SHA1→BLAKE3 mappings incrementally so trees can resolve
    // references to blobs imported in earlier rounds.
    let have_hashes = super::federation::collect_local_sha1_hashes(forge_node, mirror_repo_id, 50_000).await;

    let mut all_git_objects = Vec::new();
    let mut current_have = have_hashes;
    let max_rounds = 100u32;
    let batch_size = 5000u32;
    let mut total_imported = 0u32;
    let mut combined_stats = super::federation::FederationImportStats::default();

    for round in 0..max_rounds {
        let (objects, has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
            &connection,
            &fed_id,
            vec!["commit".to_string(), "tree".to_string(), "blob".to_string()],
            current_have.clone(),
            batch_size,
            None,
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(origin = %origin_key, error = %e, round = round, "git object sync_remote_objects failed");
                errors.push(format!("git object fetch round {round}: {e}"));
                break;
            }
        };

        if objects.is_empty() {
            debug!(
                origin = %origin_key,
                round = round,
                total = total_imported,
                "sync returned empty git objects batch — stopping"
            );
            break;
        }

        let batch_len = objects.len();
        debug!(
            origin = %origin_key,
            round = round,
            batch = batch_len,
            has_more = has_more,
            "fetched git object batch, importing..."
        );

        // Add SHA-1 of each received object to have_set for next round.
        // SHA-1 is computed from the reconstructed full git bytes (header + content).
        for obj in &objects {
            use sha1::Digest as _;
            let header = format!("{} {}\0", obj.object_type, obj.data.len());
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(&obj.data);
            let sha1: [u8; 20] = hasher.finalize().into();
            current_have.push(aspen_forge::resolver::sha1_to_have_hash(&sha1));
        }

        // Import this batch immediately so mappings are available for next round
        let batch_stats = federation_import_objects(forge_node, mirror_repo_id, &objects).await;
        let batch_imported = batch_stats.commits + batch_stats.trees + batch_stats.blobs;
        total_imported += batch_imported;

        // Merge stats
        combined_stats.blobs += batch_stats.blobs;
        combined_stats.trees += batch_stats.trees;
        combined_stats.commits += batch_stats.commits;
        combined_stats.skipped += batch_stats.skipped;
        combined_stats.total_bytes += batch_stats.total_bytes;
        for (k, v) in batch_stats.sha1_to_local_blake3 {
            combined_stats.sha1_to_local_blake3.insert(k, v);
        }
        // Only log import errors, don't propagate — partial progress is OK
        for err in &batch_stats.errors {
            debug!(origin = %origin_key, round = round, error = %err, "batch import error (non-fatal)");
        }

        all_git_objects.extend(objects);

        if !has_more {
            info!(
                origin = %origin_key,
                round = round,
                total = total_imported,
                "sync stopping: remote reported no more objects"
            );
            break;
        }
    }

    // Retry pass: re-import ALL accumulated objects. Trees/commits that failed
    // in earlier rounds (due to missing blob deps) may now succeed because later
    // rounds imported the blobs. import_objects() skips already-imported objects
    // via has_sha1, so this is cheap when everything already succeeded.
    if !all_git_objects.is_empty() && total_imported > 0 {
        let retry_stats = federation_import_objects(forge_node, mirror_repo_id, &all_git_objects).await;
        let retry_new = retry_stats.commits + retry_stats.trees + retry_stats.blobs;
        if retry_new > 0 {
            total_imported += retry_new;
            combined_stats.commits += retry_stats.commits;
            combined_stats.trees += retry_stats.trees;
            combined_stats.blobs += retry_stats.blobs;
            for (k, v) in retry_stats.sha1_to_local_blake3 {
                combined_stats.sha1_to_local_blake3.insert(k, v);
            }
            info!(
                origin = %origin_key,
                recovered_commits = retry_stats.commits,
                recovered_trees = retry_stats.trees,
                recovered_blobs = retry_stats.blobs,
                "post-sync retry recovered objects"
            );
        }
    }

    let objects_imported = total_imported;
    // Use combined_stats for ref translation
    let stats = combined_stats;

    // Translate remote ref hashes to local BLAKE3 hashes.
    //
    // Each ref carries commit_sha1 (deterministic git SHA1). We compute
    // SHA1 from each commit SyncObject's content, match by SHA1, then
    // look up the locally imported BLAKE3 via content_to_local_blake3.
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

/// A fetched ref entry: (ref_name, head_hash, commit_sha1).
type FetchedRef = (String, [u8; 32], Option<[u8; 20]>);

/// Translate remote ref hashes to locally imported BLAKE3 hashes.
///
/// Each ref carries a `commit_sha1` (git SHA1 of the commit it points to).
/// Since SHA1 is deterministic from the raw git content, it's identical on
/// source and destination. We look up the SHA1 directly in `sha1_to_local_blake3`
/// which was built during import.
///
/// For refs without `commit_sha1` (older protocol), falls back to computing
/// SHA1 from commit SyncObjects and trying all of them (single-branch only).
///
/// For refs that can't be translated (no matching commit found), falls back
/// to the original hash (which won't resolve locally, but at least the ref
/// exists for later re-sync).
fn translate_ref_hashes(
    fetched_refs: &[FetchedRef],
    git_objects: &[aspen_cluster::federation::sync::SyncObject],
    stats: &super::federation::FederationImportStats,
) -> Vec<(String, [u8; 32])> {
    // Build a fallback SHA1 set from commit SyncObjects (for older protocol
    // without commit_sha1 in ref entries).
    let commit_sha1s: Vec<[u8; 20]> = git_objects
        .iter()
        .filter(|o| o.object_type == "commit")
        .map(|obj| {
            use sha1::Digest as _;
            let header = format!("commit {}\0", obj.data.len());
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(&obj.data);
            hasher.finalize().into()
        })
        .collect();

    let mut translated = Vec::with_capacity(fetched_refs.len());

    for (ref_name, remote_hash, commit_sha1) in fetched_refs {
        let local_blake3 = if let Some(sha1) = commit_sha1 {
            // Direct SHA-1 lookup (correct for multi-branch repos).
            stats.sha1_to_local_blake3.get(sha1)
        } else {
            // Fallback for older protocol without commit_sha1:
            // try all commit SHA1s (works for single-branch only).
            commit_sha1s.iter().find_map(|sha1| stats.sha1_to_local_blake3.get(sha1))
        };

        if let Some(blake3) = local_blake3 {
            debug!(
                ref_name = %ref_name,
                local_blake3 = %hex::encode(blake3.as_bytes()),
                "translated ref to local BLAKE3"
            );
            translated.push((ref_name.clone(), *blake3.as_bytes()));
        } else {
            tracing::warn!(
                ref_name = %ref_name,
                "could not translate ref to local BLAKE3, using remote hash"
            );
            translated.push((ref_name.clone(), *remote_hash));
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

    // Fetch refs directly from the origin — fast, no object transfer needed.
    // Object transfer happens in FederationGitFetch when git actually requests them.
    let fetched_refs = match fetch_refs_from_origin(origin_key, upstream_repo_hex, origin_addr_hint, identity, endpoint)
        .await
    {
        Ok(refs) => refs,
        Err(e) => {
            // Try stale mirror cache
            let fed_id_str = build_fed_id_str(origin_key, upstream_repo_hex);
            let mirror_bytes = derive_mirror_repo_id(origin_key, upstream_repo_hex);
            let mirror_repo_id = aspen_forge::identity::RepoId(mirror_bytes);
            if find_mirror_metadata(forge_node, &fed_id_str).await.is_some() {
                warn!(error = %e, "origin unreachable, serving stale mirror refs");
                let mirror_hex = hex::encode(mirror_repo_id.0);
                let result = super::git_bridge::handle_git_bridge_list_refs(forge_node, mirror_hex).await?;
                return match result {
                    ClientRpcResponse::GitBridgeListRefs(resp) => Ok(ClientRpcResponse::FederationGitListRefs(resp)),
                    other => Ok(other),
                };
            }
            return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
                is_success: false,
                refs: vec![],
                head: None,
                error: Some(format!("origin unreachable: {e}")),
            }));
        }
    };

    if fetched_refs.is_empty() {
        return Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
            is_success: true,
            refs: vec![],
            head: None,
            error: None,
        }));
    }

    // Convert fetched refs to git bridge format using their commit SHA1 hashes
    use aspen_client_api::GitBridgeRefInfo;
    let mut git_refs = Vec::with_capacity(fetched_refs.len());
    let mut head_sha1 = None;
    let mut head_ref: Option<String> = None;

    for (ref_name, _head_hash, commit_sha1) in &fetched_refs {
        if let Some(sha1_bytes) = commit_sha1 {
            let sha1_hex = hex::encode(sha1_bytes);
            // Forge stores refs without "refs/" prefix (e.g., "heads/main").
            // Git protocol expects the full ref path ("refs/heads/main").
            let full_ref = if ref_name.starts_with("refs/") {
                ref_name.clone()
            } else {
                format!("refs/{}", ref_name)
            };
            git_refs.push(GitBridgeRefInfo {
                ref_name: full_ref.clone(),
                sha1: sha1_hex.clone(),
            });
            // Use heads/main or heads/master as HEAD
            if head_sha1.is_none() && (ref_name == "heads/main" || ref_name == "heads/master") {
                head_sha1 = Some(sha1_hex);
                head_ref = Some(full_ref);
            }
        }
    }

    info!(
        origin = %origin_key,
        refs = git_refs.len(),
        "federation list-refs served directly from origin (no object sync)"
    );

    Ok(ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
        is_success: true,
        refs: git_refs,
        head: head_ref,
        error: None,
    }))
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

    let fed_id_str = build_fed_id_str(origin_key, upstream_repo_hex);
    let mirror_bytes = derive_mirror_repo_id(origin_key, upstream_repo_hex);
    let mirror_repo_id = aspen_forge::identity::RepoId(mirror_bytes);
    let mirror_hex = hex::encode(mirror_repo_id.0);

    // Ensure mirror repo exists
    let _ = get_or_create_mirror(forge_node, &fed_id_str, origin_key, None, None).await;

    // Try serving from mirror first
    let result =
        super::git_bridge::handle_git_bridge_fetch(forge_node, mirror_hex.clone(), want.clone(), have.clone()).await?;

    // Check if all objects were found
    let needs_sync = match &result {
        ClientRpcResponse::GitBridgeFetch(resp) => !resp.is_success || resp.objects.is_empty(),
        _ => true,
    };

    if needs_sync {
        info!(
            origin = %origin_key,
            want_count = want.len(),
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

    /// Two refs pointing to different commits must produce different local
    /// BLAKE3 hashes after translation. Verifies SHA1-based matching works
    /// correctly for multi-branch repos.
    #[test]
    fn test_translate_ref_hashes_multi_branch() {
        use sha1::Digest;

        // Build two distinct commit git objects (different tree SHA1s).
        let commit_data_a = b"tree 1111111111111111111111111111111111111111\n\
            author A <a@test> 1700000000 +0000\n\
            committer A <a@test> 1700000000 +0000\n\n\
            commit A\n";
        let commit_data_b = b"tree 2222222222222222222222222222222222222222\n\
            author B <b@test> 1700000000 +0000\n\
            committer B <b@test> 1700000000 +0000\n\n\
            commit B\n";

        // Compute SHA1 for each commit (header + content).
        let sha1_a: [u8; 20] = {
            let header = format!("commit {}\0", commit_data_a.len());
            let mut h = sha1::Sha1::new();
            h.update(header.as_bytes());
            h.update(commit_data_a.as_slice());
            h.finalize().into()
        };
        let sha1_b: [u8; 20] = {
            let header = format!("commit {}\0", commit_data_b.len());
            let mut h = sha1::Sha1::new();
            h.update(header.as_bytes());
            h.update(commit_data_b.as_slice());
            h.finalize().into()
        };

        assert_ne!(sha1_a, sha1_b, "commits must have different SHA1s");

        // Content hashes (blake3 of raw content).
        let ch_a: [u8; 32] = blake3::hash(commit_data_a).into();
        let ch_b: [u8; 32] = blake3::hash(commit_data_b).into();

        // Simulate SyncObjects for both commits.
        let sync_objects = vec![
            aspen_cluster::federation::sync::SyncObject {
                object_type: "commit".to_string(),
                hash: ch_a,
                data: commit_data_a.to_vec(),
                signature: None,
                signer: None,
            },
            aspen_cluster::federation::sync::SyncObject {
                object_type: "commit".to_string(),
                hash: ch_b,
                data: commit_data_b.to_vec(),
                signature: None,
                signer: None,
            },
        ];

        // Simulate import stats: each SHA-1 maps to a different local BLAKE3.
        let local_b3_a = blake3::Hash::from_bytes([0xAA; 32]);
        let local_b3_b = blake3::Hash::from_bytes([0xBB; 32]);
        let mut stats = super::super::federation::FederationImportStats::default();
        stats.sha1_to_local_blake3.insert(sha1_a, local_b3_a);
        stats.sha1_to_local_blake3.insert(sha1_b, local_b3_b);

        // Refs with commit_sha1 set (new protocol).
        let fetched_refs = vec![
            ("heads/main".to_string(), [0x11; 32], Some(sha1_a)),
            ("heads/dev".to_string(), [0x22; 32], Some(sha1_b)),
        ];

        let translated = translate_ref_hashes(&fetched_refs, &sync_objects, &stats);

        assert_eq!(translated.len(), 2);

        let main_hash = translated.iter().find(|(n, _)| n == "heads/main").map(|(_, h)| *h);
        let dev_hash = translated.iter().find(|(n, _)| n == "heads/dev").map(|(_, h)| *h);

        assert_eq!(main_hash, Some(*local_b3_a.as_bytes()), "heads/main should map to local_b3_a");
        assert_eq!(dev_hash, Some(*local_b3_b.as_bytes()), "heads/dev should map to local_b3_b");
        assert_ne!(main_hash, dev_hash, "different refs must get different local hashes");
    }
}
