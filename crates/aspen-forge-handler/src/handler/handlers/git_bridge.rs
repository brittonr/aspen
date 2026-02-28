//! Git bridge operations (git-remote-aspen interop).
//!
//! This module is feature-gated with `git-bridge`.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::GitBridgeObject;
use aspen_client_api::GitBridgeRefUpdate;
use aspen_forge::constants::MAX_CONCURRENT_PUSH_SESSIONS;
use aspen_forge::constants::PUSH_SESSION_TIMEOUT;

use super::ForgeNodeRef;

// =============================================================================
// Chunked Push Session State
// =============================================================================

/// State for an active chunked push session.
pub(crate) struct ChunkedPushSession {
    /// Repository ID for this push.
    pub(crate) repo_id: String,
    /// Ref updates to apply after all objects are received.
    pub(crate) refs: Vec<GitBridgeRefUpdate>,
    /// Expected total number of objects (stored for future progress tracking).
    #[allow(dead_code)]
    pub(crate) total_objects: u64,
    /// Expected total size in bytes (stored for future progress tracking).
    #[allow(dead_code)]
    pub(crate) total_size_bytes: u64,
    /// Set of chunk IDs that have been received.
    pub(crate) chunks_received: HashSet<u64>,
    /// Total number of chunks (set on first chunk).
    pub(crate) total_chunks: Option<u64>,
    /// Accumulated objects from all chunks.
    pub(crate) objects: Vec<GitBridgeObject>,
    /// Session creation time for timeout tracking.
    pub(crate) created_at: std::time::Instant,
}

/// Global session store for chunked push operations.
///
/// Tiger Style: Bounded to MAX_CONCURRENT_PUSH_SESSIONS with timeout-based cleanup.
pub(crate) static PUSH_SESSIONS: OnceLock<Arc<Mutex<HashMap<String, ChunkedPushSession>>>> = OnceLock::new();

/// Get the global session store, initializing if needed.
pub(crate) fn get_session_store() -> &'static Arc<Mutex<HashMap<String, ChunkedPushSession>>> {
    PUSH_SESSIONS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// Lock the session store, recovering from mutex poisoning.
///
/// Tiger Style: std::sync::Mutex can be poisoned if a thread panics while
/// holding the lock. Rather than propagating the panic via .unwrap(), we
/// recover the inner data. The HashMap state may be inconsistent after a
/// panic, but session timeouts provide natural cleanup.
fn lock_sessions(
    store: &Mutex<HashMap<String, ChunkedPushSession>>,
) -> MutexGuard<'_, HashMap<String, ChunkedPushSession>> {
    store.lock().unwrap_or_else(|poisoned| {
        tracing::warn!(
            target: "aspen_forge::git_bridge",
            "push sessions mutex was poisoned, recovering"
        );
        poisoned.into_inner()
    })
}

pub(crate) async fn handle_git_bridge_list_refs(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgeListRefsResponse;
    use aspen_client_api::GitBridgeRefInfo;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitExporter;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                is_success: false,
                refs: vec![],
                head: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Create exporter with hash mapping
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let exporter = GitExporter::new(
        mapping,
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    match exporter.list_refs(&repo_id).await {
        Ok(ref_list) => {
            let refs: Vec<GitBridgeRefInfo> = ref_list
                .iter()
                .filter_map(|(name, sha1_opt)| {
                    sha1_opt.map(|sha1| GitBridgeRefInfo {
                        ref_name: format!("refs/{}", name),
                        sha1: sha1.to_hex(),
                    })
                })
                .collect();

            // Determine HEAD - look for main or master branch
            let head = ref_list
                .iter()
                .find(|(name, sha1_opt)| sha1_opt.is_some() && (name == "heads/main" || name == "heads/master"))
                .map(|(name, _)| format!("refs/{}", name));

            Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                is_success: true,
                refs,
                head,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
            is_success: false,
            refs: vec![],
            head: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_git_bridge_fetch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    want: Vec<String>,
    have: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgeFetchResponse;
    use aspen_client_api::GitBridgeObject;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitExporter;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Parse want/have SHA-1 hashes
    let want_hashes: Result<Vec<Sha1Hash>, _> = want.iter().map(|s| Sha1Hash::from_hex(s)).collect();
    let want_hashes = match want_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid want hash: {}", e)),
            }));
        }
    };

    let have_hashes: Result<HashSet<Sha1Hash>, _> = have.iter().map(|s| Sha1Hash::from_hex(s)).collect();
    let have_hashes = match have_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                is_success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid have hash: {}", e)),
            }));
        }
    };

    // Create exporter with hash mapping
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let exporter = GitExporter::new(
        mapping.clone(),
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    // Export commits for all wanted refs
    let mut objects = Vec::new();
    let mut skipped: u32 = 0;

    for want_sha1 in &want_hashes {
        // Get the blake3 hash for this SHA-1 using the mapping
        match mapping.get_blake3(&repo_id, want_sha1).await {
            Ok(None) => {
                tracing::warn!(
                    target: "aspen_forge::git_bridge",
                    sha1 = %want_sha1,
                    "fetch: wanted SHA-1 has no BLAKE3 mapping, skipping"
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    target: "aspen_forge::git_bridge",
                    sha1 = %want_sha1,
                    error = %e,
                    "fetch: failed to look up SHA-1 mapping"
                );
                continue;
            }
            Ok(Some((blake3_hash, _))) => {
                // Found the mapping — export the commit DAG
                match exporter.export_commit_dag(&repo_id, blake3_hash, &have_hashes).await {
                    Ok(exported) => {
                        for obj in exported.objects {
                            objects.push(GitBridgeObject {
                                sha1: obj.sha1.to_hex(),
                                object_type: obj.object_type.as_str().to_string(),
                                data: obj.content,
                            });
                        }
                        skipped += exported.objects_skipped;
                    }
                    Err(e) => {
                        return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                            is_success: false,
                            objects: vec![],
                            skipped: 0,
                            error: Some(e.to_string()),
                        }));
                    }
                }
            }
        }
    }

    Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
        is_success: true,
        objects,
        skipped,
        error: None,
    }))
}

pub(crate) async fn handle_git_bridge_push(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    objects: Vec<aspen_client_api::GitBridgeObject>,
    refs: Vec<aspen_client_api::GitBridgeRefUpdate>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushResponse;
    use aspen_client_api::GitBridgeRefResult;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitImporter;
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                is_success: false,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results: vec![],
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Create importer
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let importer = GitImporter::new(
        mapping,
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    // Convert objects to import format (with git headers)
    let import_objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)> = objects
        .iter()
        .filter_map(|obj| {
            let sha1 = Sha1Hash::from_hex(&obj.sha1).ok()?;
            let obj_type = match obj.object_type.as_str() {
                "blob" => GitObjectType::Blob,
                "tree" => GitObjectType::Tree,
                "commit" => GitObjectType::Commit,
                "tag" => GitObjectType::Tag,
                _ => return None,
            };
            // Build full git object bytes with header: "type size\0content"
            let header = format!("{} {}\0", obj.object_type, obj.data.len());
            let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
            git_bytes.extend_from_slice(header.as_bytes());
            git_bytes.extend_from_slice(&obj.data);
            Some((sha1, obj_type, git_bytes))
        })
        .collect();

    // Import objects using batch import which handles topological ordering
    let (objects_imported, objects_skipped) = match importer.import_objects(&repo_id, import_objects).await {
        Ok(result) => (result.objects_imported, result.objects_skipped),
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                is_success: false,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results: vec![],
                error: Some(format!("Import failed: {}", e)),
            }));
        }
    };

    // Update refs
    let mut ref_results = Vec::new();
    for ref_update in &refs {
        // Get the blake3 hash for the new SHA-1
        let new_sha1 = match Sha1Hash::from_hex(&ref_update.new_sha1) {
            Ok(h) => h,
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    is_success: false,
                    error: Some(format!("Invalid SHA-1: {}", e)),
                });
                continue;
            }
        };

        // Get blake3 hash from mapping
        let blake3_hash = match importer.get_blake3(&repo_id, &new_sha1).await {
            Ok(Some(h)) => h,
            Ok(None) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    is_success: false,
                    error: Some("Object not found in mapping".to_string()),
                });
                continue;
            }
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    is_success: false,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        // Convert ref name (strip refs/ prefix)
        let ref_name = ref_update.ref_name.strip_prefix("refs/").unwrap_or(&ref_update.ref_name);

        match forge_node.refs.set(&repo_id, ref_name, blake3_hash).await {
            Ok(()) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    is_success: true,
                    error: None,
                });
            }
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    is_success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let all_success = ref_results.iter().all(|r| r.is_success);
    Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
        is_success: all_success,
        objects_imported,
        objects_skipped,
        ref_results,
        error: None,
    }))
}

/// Handle GitBridgePushStart - initiate chunked push session.
pub(crate) async fn handle_git_bridge_push_start(
    _forge_node: &ForgeNodeRef,
    repo_id: String,
    total_objects: u64,
    total_size_bytes: u64,
    refs: Vec<aspen_client_api::GitBridgeRefUpdate>,
    _metadata: Option<aspen_client_api::GitBridgePushMetadata>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DEFAULT_GIT_CHUNK_SIZE_BYTES;
    use aspen_client_api::GitBridgePushStartResponse;
    use uuid::Uuid;

    // Generate unique session ID
    let session_id = Uuid::new_v4().to_string();

    // Validate limits
    if total_objects > 100_000 {
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id: session_id.clone(),
            max_chunk_size_bytes: DEFAULT_GIT_CHUNK_SIZE_BYTES,
            is_success: false,
            error: Some("Too many objects - maximum 100,000 allowed".to_string()),
        }));
    }

    if total_size_bytes > 1024 * 1024 * 1024 {
        // 1GB limit
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id: session_id.clone(),
            max_chunk_size_bytes: DEFAULT_GIT_CHUNK_SIZE_BYTES,
            is_success: false,
            error: Some("Push too large - maximum 1GB allowed".to_string()),
        }));
    }

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        repo_id = repo_id,
        total_objects = total_objects,
        total_size_bytes = total_size_bytes,
        "Starting chunked git push session"
    );

    // Store session state for tracking chunks
    let session = ChunkedPushSession {
        repo_id: repo_id.clone(),
        refs,
        total_objects,
        total_size_bytes,
        chunks_received: HashSet::new(),
        total_chunks: None,
        objects: Vec::new(),
        created_at: std::time::Instant::now(),
    };

    let store = get_session_store();
    let mut sessions = lock_sessions(store);

    // Cleanup expired sessions first (Tiger Style: opportunistic cleanup)
    sessions.retain(|_, s| s.created_at.elapsed() < PUSH_SESSION_TIMEOUT);

    // Check capacity (Tiger Style bound)
    if sessions.len() >= MAX_CONCURRENT_PUSH_SESSIONS {
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id,
            max_chunk_size_bytes: DEFAULT_GIT_CHUNK_SIZE_BYTES,
            is_success: false,
            error: Some("Too many concurrent push sessions - try again later".to_string()),
        }));
    }

    sessions.insert(session_id.clone(), session);

    Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
        session_id,
        max_chunk_size_bytes: DEFAULT_GIT_CHUNK_SIZE_BYTES,
        is_success: true,
        error: None,
    }))
}

/// Handle GitBridgePushChunk - receive and validate chunk.
pub(crate) async fn handle_git_bridge_push_chunk(
    _forge_node: &ForgeNodeRef,
    session_id: String,
    chunk_id: u64,
    total_chunks: u64,
    objects: Vec<aspen_client_api::GitBridgeObject>,
    chunk_hash: [u8; 32],
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushChunkResponse;
    use blake3::Hasher;

    // Validate chunk hash for integrity
    let mut hasher = Hasher::new();
    for obj in &objects {
        hasher.update(obj.sha1.as_bytes());
        hasher.update(obj.object_type.as_bytes());
        hasher.update(&obj.data);
    }
    let computed_hash = *hasher.finalize().as_bytes();

    if computed_hash != chunk_hash {
        tracing::warn!(
            target: "aspen_forge::git_bridge",
            session_id = session_id,
            chunk_id = chunk_id,
            "Chunk hash mismatch - integrity check failed"
        );
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id,
            chunk_id,
            is_success: false,
            error: Some("Chunk integrity check failed - hash mismatch".to_string()),
        }));
    }

    tracing::debug!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        chunk_id = chunk_id,
        total_chunks = total_chunks,
        objects_count = objects.len(),
        "Received chunk with valid hash"
    );

    // Store chunk data and validate sequence
    let store = get_session_store();
    let mut sessions = lock_sessions(store);

    let session = match sessions.get_mut(&session_id) {
        Some(s) => s,
        None => {
            return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
                session_id,
                chunk_id,
                is_success: false,
                error: Some("Session not found or expired".to_string()),
            }));
        }
    };

    // Check session timeout
    if session.created_at.elapsed() > PUSH_SESSION_TIMEOUT {
        let sid = session_id.clone();
        sessions.remove(&session_id);
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id: sid,
            chunk_id,
            is_success: false,
            error: Some("Session expired".to_string()),
        }));
    }

    // Set or validate total_chunks consistency
    match session.total_chunks {
        Some(tc) if tc != total_chunks => {
            return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
                session_id,
                chunk_id,
                is_success: false,
                error: Some("Inconsistent total_chunks value".to_string()),
            }));
        }
        None => session.total_chunks = Some(total_chunks),
        _ => {}
    }

    // Check for duplicate chunk
    if session.chunks_received.contains(&chunk_id) {
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id,
            chunk_id,
            is_success: false,
            error: Some("Duplicate chunk received".to_string()),
        }));
    }

    // Store chunk data
    session.chunks_received.insert(chunk_id);
    session.objects.extend(objects);

    Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
        session_id,
        chunk_id,
        is_success: true,
        error: None,
    }))
}

/// Handle GitBridgePushComplete - finalize chunked push.
pub(crate) async fn handle_git_bridge_push_complete(
    forge_node: &ForgeNodeRef,
    session_id: String,
    content_hash: [u8; 32],
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushCompleteResponse;
    use blake3::Hasher;

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        "Completing chunked git push session"
    );

    // Retrieve and remove session from store
    let session = {
        let store = get_session_store();
        let mut sessions = lock_sessions(store);
        match sessions.remove(&session_id) {
            Some(s) => s,
            None => {
                return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
                    session_id,
                    is_success: false,
                    objects_imported: 0,
                    objects_skipped: 0,
                    ref_results: vec![],
                    error: Some("Session not found or expired".to_string()),
                }));
            }
        }
    };

    // Validate all chunks were received
    let total_chunks = session.total_chunks.unwrap_or(0);
    if session.chunks_received.len() as u64 != total_chunks {
        return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            is_success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some(format!("Missing chunks: received {}/{}", session.chunks_received.len(), total_chunks)),
        }));
    }

    // Validate content hash matches reassembled chunks
    let mut hasher = Hasher::new();
    for obj in &session.objects {
        hasher.update(obj.sha1.as_bytes());
        hasher.update(obj.object_type.as_bytes());
        hasher.update(&obj.data);
    }
    let computed_hash = *hasher.finalize().as_bytes();

    if computed_hash != content_hash {
        tracing::warn!(
            target: "aspen_forge::git_bridge",
            session_id = session_id,
            "Content hash mismatch on push complete"
        );
        return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            is_success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some("Content hash mismatch - push data corrupted".to_string()),
        }));
    }

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        objects_count = session.objects.len(),
        refs_count = session.refs.len(),
        "All chunks received, processing push"
    );

    // Reuse existing push logic
    let push_result = handle_git_bridge_push(forge_node, session.repo_id, session.objects, session.refs).await?;

    // Convert GitBridgePushResponse to GitBridgePushCompleteResponse
    match push_result {
        ClientRpcResponse::GitBridgePush(resp) => {
            Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
                session_id,
                is_success: resp.is_success,
                objects_imported: resp.objects_imported,
                objects_skipped: resp.objects_skipped,
                ref_results: resp.ref_results,
                error: resp.error,
            }))
        }
        _ => Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            is_success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some("Unexpected response from push handler".to_string()),
        })),
    }
}

/// Handle GitBridgeProbeObjects - check which SHA-1 hashes the server already has.
///
/// Used for incremental push: the client sends all SHA-1s from `git rev-list`,
/// the server reports which ones already have hash mappings, and the client
/// only reads/sends the missing objects.
///
/// This is a read-only operation (no Raft writes) and uses concurrent lookups
/// for performance.
pub(crate) async fn handle_git_bridge_probe_objects(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    sha1s: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgeProbeObjectsResponse;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
                is_success: false,
                known_sha1s: vec![],
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Tiger Style: bound input size to prevent resource exhaustion
    if sha1s.len() > 100_000 {
        return Ok(ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
            is_success: false,
            known_sha1s: vec![],
            error: Some("Too many SHA-1 hashes to probe (max 100,000)".to_string()),
        }));
    }

    // Parse SHA-1 hashes up front, reject invalid ones
    let parsed: Result<Vec<(String, Sha1Hash)>, _> =
        sha1s.iter().map(|s| Sha1Hash::from_hex(s).map(|h| (s.clone(), h))).collect();

    let parsed = match parsed {
        Ok(p) => p,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
                is_success: false,
                known_sha1s: vec![],
                error: Some(format!("Invalid SHA-1 hash: {}", e)),
            }));
        }
    };

    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));

    // Check all hashes sequentially. Each has_sha1() is a single KV read
    // (ReadIndex, no Raft write). Sequential is simple and predictable;
    // the KV reads are fast (~0.1ms each on local Raft) and this only
    // runs once per push before any data transfer.
    let mut known_sha1s = Vec::new();

    for (hex_str, sha1) in &parsed {
        match mapping.has_sha1(&repo_id, sha1).await {
            Ok(true) => known_sha1s.push(hex_str.clone()),
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(
                    target: "aspen_forge::git_bridge",
                    sha1 = %hex_str,
                    error = %e,
                    "probe lookup failed for object"
                );
            }
        }
    }

    tracing::debug!(
        target: "aspen_forge::git_bridge",
        total = sha1s.len(),
        known = known_sha1s.len(),
        missing = sha1s.len() - known_sha1s.len(),
        "probe objects complete"
    );

    Ok(ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
        is_success: true,
        known_sha1s,
        error: None,
    }))
}
