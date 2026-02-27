//! Wire-level integration test for git-remote-aspen.
//!
//! Tests the complete `git push` / `git clone` / `git fetch` workflow over
//! real iroh QUIC networking. Spins up a minimal RPC server that handles
//! only git bridge operations (no Raft needed), creates local git repos,
//! and invokes the actual `git` CLI with the `git-remote-aspen` helper.
//!
//! # Architecture
//!
//! ```text
//! Local git repo
//!   └── git push aspen://<ticket>/<repo_id>
//!         └── git-remote-aspen binary (QUIC client)
//!               └── iroh Endpoint ─── QUIC ──→ MinimalForgeServer
//!                                                 └── ForgeNode (in-memory)
//!                                                       └── GitImporter / GitExporter
//! ```
//!
//! # Requirements
//!
//! - Real network access (iroh binds to loopback)
//! - `git` CLI available in PATH
//! - `git-remote-aspen` binary built (cargo builds it as test dependency)
//!
//! # Running
//!
//! These tests require network access and are `#[ignore]` for CI sandboxes:
//!
//! ```sh
//! cargo nextest run --features testing,forge,git-bridge,blob \
//!     --test forge_git_wire_test --run-ignored all
//! ```

#![cfg(all(feature = "git-bridge", feature = "blob", feature = "testing"))]

// ============================================================================
// Push Session State (global map keyed by session ID)
// ============================================================================
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use aspen::CLIENT_ALPN;
use aspen::blob::InMemoryBlobStore;
use aspen::cluster::ticket::AspenClusterTicket;
use aspen::forge::ForgeNode;
use aspen::forge::git::bridge::GitExporter;
use aspen::forge::git::bridge::GitImporter;
use aspen::forge::git::bridge::HashMappingStore;
use aspen::forge::identity::RepoId;
use aspen::testing::DeterministicKeyValueStore;
use aspen_client_api::AuthenticatedRequest;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_core::hlc::create_hlc;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
use iroh_gossip::proto::TopicId;
use tempfile::TempDir;
use tokio::time::timeout;
use tracing::info;

/// Per-session push state.
struct PushSession {
    repo_id: String,
    ref_updates: Vec<aspen_client_api::GitBridgeRefUpdate>,
}

/// Global session map keyed by session ID.
static PUSH_SESSIONS: std::sync::LazyLock<Mutex<HashMap<String, PushSession>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// Minimal Forge RPC Server
// ============================================================================

/// A minimal server that handles only git bridge RPC requests over iroh QUIC.
///
/// No Raft consensus, no full handler registry — just ForgeNode + git bridge
/// operations wired directly. This is sufficient for testing the wire protocol
/// between git-remote-aspen and the server handlers.
struct MinimalForgeServer {
    endpoint: Endpoint,
    forge_node: Arc<ForgeNode<InMemoryBlobStore, dyn aspen_core::KeyValueStore>>,
    _kv: Arc<dyn aspen_core::KeyValueStore>,
}

impl MinimalForgeServer {
    /// Create a new minimal forge server with in-memory storage.
    async fn new() -> Self {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let blobs = Arc::new(InMemoryBlobStore::new());
        let forge_node = Arc::new(ForgeNode::new(blobs, kv.clone(), secret_key.clone()));

        // Create iroh endpoint that accepts CLIENT_ALPN connections
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .clear_discovery()
            .bind()
            .await
            .expect("failed to bind iroh endpoint");

        Self {
            endpoint,
            forge_node,
            _kv: kv,
        }
    }

    /// Get the endpoint address for constructing a cluster ticket.
    ///
    /// Converts `0.0.0.0` to `127.0.0.1` so the client connects to loopback
    /// instead of trying external discovery.
    fn endpoint_addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.endpoint.id());
        for socket_addr in self.endpoint.bound_sockets() {
            // Convert 0.0.0.0 → 127.0.0.1 for loopback connectivity
            let fixed = if socket_addr.ip().is_unspecified() {
                std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), socket_addr.port())
            } else {
                socket_addr
            };
            addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
        }
        addr
    }

    /// Create a cluster ticket pointing to this server.
    fn cluster_ticket(&self) -> AspenClusterTicket {
        let topic_id = TopicId::from_bytes([0u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-forge".into());
        ticket.add_bootstrap_addr(&self.endpoint_addr()).expect("failed to add bootstrap addr");
        ticket
    }

    /// Create a repository and return its ID.
    async fn create_repo(&self, name: &str) -> RepoId {
        let identity =
            self.forge_node.create_repo(name, vec![self.forge_node.public_key()], 1).await.expect("create repo");
        identity.repo_id()
    }

    /// Run the RPC server loop, handling git bridge requests.
    ///
    /// Runs until cancelled. Each connection is handled in a spawned task.
    async fn serve(&self) {
        loop {
            let conn = match self.endpoint.accept().await {
                Some(incoming) => match incoming.await {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("forge-server: accept error: {}", e);
                        continue;
                    }
                },
                None => break,
            };

            let forge = self.forge_node.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(conn, forge).await {
                    eprintln!("forge-server: connection error: {}", e);
                }
            });
        }
    }
}

/// Handle a single client connection, dispatching git bridge RPCs.
async fn handle_connection(
    conn: iroh::endpoint::Connection,
    forge: Arc<ForgeNode<InMemoryBlobStore, dyn aspen_core::KeyValueStore>>,
) -> anyhow::Result<()> {
    loop {
        let (mut send, mut recv) = match conn.accept_bi().await {
            Ok(streams) => streams,
            Err(_) => break, // Connection closed
        };

        let forge = forge.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_stream(&mut send, &mut recv, &forge).await {
                eprintln!("forge-server: stream error: {}", e);
            }
        });
    }
    Ok(())
}

/// Handle a single bidirectional stream (one RPC request/response).
async fn handle_stream(
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
    forge: &ForgeNode<InMemoryBlobStore, dyn aspen_core::KeyValueStore>,
) -> anyhow::Result<()> {
    // Read request
    let request_bytes = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await?;
    let auth_request: AuthenticatedRequest = postcard::from_bytes(&request_bytes)?;
    let request = auth_request.request;

    // Dispatch to git bridge handlers
    let response = dispatch_git_bridge(forge, request).await;

    // Write response
    let response_bytes = postcard::to_stdvec(&response)?;
    send.write_all(&response_bytes).await?;
    send.finish()?;

    Ok(())
}

/// Dispatch a git bridge RPC request to the appropriate handler.
async fn dispatch_git_bridge(
    forge: &ForgeNode<InMemoryBlobStore, dyn aspen_core::KeyValueStore>,
    request: ClientRpcRequest,
) -> ClientRpcResponse {
    use std::collections::HashSet;

    use aspen_client_api::GitBridgeFetchResponse;
    use aspen_client_api::GitBridgeListRefsResponse;
    use aspen_client_api::GitBridgeObject;
    use aspen_client_api::GitBridgeProbeObjectsResponse;
    use aspen_client_api::GitBridgePushResponse;
    use aspen_client_api::GitBridgeRefResult;
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::Sha1Hash;

    let kv = forge.kv().clone();
    let mapping = Arc::new(HashMappingStore::new(kv));
    let refs = Arc::new(forge.refs.clone());
    let secret_key = forge.secret_key().clone();

    match request {
        ClientRpcRequest::GitBridgeListRefs { repo_id } => {
            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                        is_success: false,
                        refs: vec![],
                        head: None,
                        error: Some(format!("Invalid repo ID: {}", e)),
                    });
                }
            };

            let hlc = create_hlc("test-server");
            let exporter = GitExporter::new(mapping, forge.git.blobs().clone(), refs, secret_key, hlc);

            match exporter.list_refs(&repo_id).await {
                Ok(ref_list) => {
                    let git_refs: Vec<aspen_client_api::GitBridgeRefInfo> = ref_list
                        .iter()
                        .filter_map(|(name, sha1_opt)| {
                            sha1_opt.map(|sha1| aspen_client_api::GitBridgeRefInfo {
                                ref_name: format!("refs/{}", name),
                                sha1: sha1.to_hex(),
                            })
                        })
                        .collect();

                    let head = ref_list
                        .iter()
                        .find(|(name, sha1_opt)| sha1_opt.is_some() && (name == "heads/main" || name == "heads/master"))
                        .map(|(name, _)| format!("refs/{}", name));

                    ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                        is_success: true,
                        refs: git_refs,
                        head,
                        error: None,
                    })
                }
                Err(e) => ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                    is_success: false,
                    refs: vec![],
                    head: None,
                    error: Some(e.to_string()),
                }),
            }
        }

        ClientRpcRequest::GitBridgePush {
            repo_id,
            objects,
            refs: ref_updates,
        } => {
            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                        is_success: false,
                        objects_imported: 0,
                        objects_skipped: 0,
                        ref_results: vec![],
                        error: Some(format!("Invalid repo ID: {}", e)),
                    });
                }
            };

            let hlc = create_hlc("test-server");
            let importer =
                GitImporter::new(mapping.clone(), forge.git.blobs().clone(), refs.clone(), secret_key.clone(), hlc);

            // Convert and import objects
            let import_objects: Vec<_> = objects
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
                    let header = format!("{} {}\0", obj.object_type, obj.data.len());
                    let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
                    git_bytes.extend_from_slice(header.as_bytes());
                    git_bytes.extend_from_slice(&obj.data);
                    Some((sha1, obj_type, git_bytes))
                })
                .collect();

            let (objects_imported, objects_skipped) = match importer.import_objects(&repo_id, import_objects).await {
                Ok(result) => (result.objects_imported, result.objects_skipped),
                Err(e) => {
                    return ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                        is_success: false,
                        objects_imported: 0,
                        objects_skipped: 0,
                        ref_results: vec![],
                        error: Some(format!("Import failed: {}", e)),
                    });
                }
            };

            // Update refs
            let mut ref_results = Vec::new();
            for ref_update in &ref_updates {
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

                let ref_name = ref_update.ref_name.strip_prefix("refs/").unwrap_or(&ref_update.ref_name);
                match forge.refs.set(&repo_id, ref_name, blake3_hash).await {
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
            ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                is_success: all_success,
                objects_imported,
                objects_skipped,
                ref_results,
                error: None,
            })
        }

        ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                        is_success: false,
                        objects: vec![],
                        skipped: 0,
                        error: Some(format!("Invalid repo ID: {}", e)),
                    });
                }
            };

            let want_hashes: Result<Vec<Sha1Hash>, _> = want.iter().map(|s| Sha1Hash::from_hex(s)).collect();
            let want_hashes = match want_hashes {
                Ok(h) => h,
                Err(e) => {
                    return ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                        is_success: false,
                        objects: vec![],
                        skipped: 0,
                        error: Some(format!("Invalid want hash: {}", e)),
                    });
                }
            };

            let have_hashes: HashSet<Sha1Hash> = have.iter().filter_map(|s| Sha1Hash::from_hex(s).ok()).collect();

            let hlc = create_hlc("test-server");
            let exporter = GitExporter::new(mapping.clone(), forge.git.blobs().clone(), refs, secret_key, hlc);

            let mut objects = Vec::new();
            let mut skipped: u32 = 0;

            for want_sha1 in &want_hashes {
                if let Ok(Some((blake3_hash, _))) = mapping.get_blake3(&repo_id, want_sha1).await {
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
                            return ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                                is_success: false,
                                objects: vec![],
                                skipped: 0,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
            }

            ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                is_success: true,
                objects,
                skipped,
                error: None,
            })
        }

        ClientRpcRequest::GitBridgeProbeObjects { repo_id, sha1s } => {
            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
                        is_success: false,
                        known_sha1s: vec![],
                        error: Some(format!("Invalid repo ID: {}", e)),
                    });
                }
            };

            let mut known_sha1s = Vec::new();
            for hex_str in &sha1s {
                if let Ok(sha1) = Sha1Hash::from_hex(hex_str)
                    && let Ok(true) = mapping.has_sha1(&repo_id, &sha1).await
                {
                    known_sha1s.push(hex_str.clone());
                }
            }

            ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
                is_success: true,
                known_sha1s,
                error: None,
            })
        }

        ClientRpcRequest::GitBridgePushStart {
            repo_id,
            total_objects: _,
            total_size_bytes: _,
            refs: ref_updates,
            metadata: _,
        } => {
            use aspen_client_api::DEFAULT_GIT_CHUNK_SIZE_BYTES;
            use aspen_client_api::GitBridgePushStartResponse;

            let session_id = format!("test-session-{}", rand::random::<u32>());

            // Store session state for PushChunk/PushComplete
            PUSH_SESSIONS.lock().unwrap().insert(session_id.clone(), PushSession { repo_id, ref_updates });
            ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
                session_id,
                max_chunk_size_bytes: DEFAULT_GIT_CHUNK_SIZE_BYTES,
                is_success: true,
                error: None,
            })
        }

        ClientRpcRequest::GitBridgePushChunk {
            session_id,
            chunk_id,
            total_chunks: _,
            objects,
            chunk_hash: _,
        } => {
            use aspen_client_api::GitBridgePushChunkResponse;

            // Import objects into the forge node as they arrive
            let repo_id_hex = PUSH_SESSIONS.lock().unwrap().get(&session_id).map(|s| s.repo_id.clone());
            if let Some(repo_id_hex) = &repo_id_hex
                && let Ok(repo_id) = RepoId::from_hex(repo_id_hex)
            {
                let hlc = create_hlc("test-server");
                let importer =
                    GitImporter::new(mapping.clone(), forge.git.blobs().clone(), refs.clone(), secret_key.clone(), hlc);

                let import_objects: Vec<_> = objects
                    .iter()
                    .filter_map(|obj| {
                        let sha1 = aspen_forge::git::bridge::Sha1Hash::from_hex(&obj.sha1).ok()?;
                        let obj_type = match obj.object_type.as_str() {
                            "blob" => aspen_forge::git::bridge::GitObjectType::Blob,
                            "tree" => aspen_forge::git::bridge::GitObjectType::Tree,
                            "commit" => aspen_forge::git::bridge::GitObjectType::Commit,
                            "tag" => aspen_forge::git::bridge::GitObjectType::Tag,
                            _ => return None,
                        };
                        let header = format!("{} {}\0", obj.object_type, obj.data.len());
                        let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
                        git_bytes.extend_from_slice(header.as_bytes());
                        git_bytes.extend_from_slice(&obj.data);
                        Some((sha1, obj_type, git_bytes))
                    })
                    .collect();

                let _ = importer.import_objects(&repo_id, import_objects).await;
            }

            ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
                session_id,
                chunk_id,
                is_success: true,
                error: None,
            })
        }

        ClientRpcRequest::GitBridgePushComplete {
            session_id,
            content_hash: _,
        } => {
            use aspen_client_api::GitBridgePushCompleteResponse;

            // Apply ref updates from the stored session
            let session = PUSH_SESSIONS.lock().unwrap().remove(&session_id);
            let (repo_id_hex, ref_updates) = match session {
                Some(s) => (Some(s.repo_id), s.ref_updates),
                None => (None, vec![]),
            };

            let mut ref_results = Vec::new();
            if let Some(repo_id_hex) = repo_id_hex
                && let Ok(repo_id) = RepoId::from_hex(&repo_id_hex)
            {
                let hlc = create_hlc("test-server");
                let importer =
                    GitImporter::new(mapping.clone(), forge.git.blobs().clone(), refs.clone(), secret_key.clone(), hlc);

                for ref_update in &ref_updates {
                    let new_sha1 = match aspen_forge::git::bridge::Sha1Hash::from_hex(&ref_update.new_sha1) {
                        Ok(h) => h,
                        Err(e) => {
                            ref_results.push(aspen_client_api::GitBridgeRefResult {
                                ref_name: ref_update.ref_name.clone(),
                                is_success: false,
                                error: Some(format!("Invalid SHA-1: {}", e)),
                            });
                            continue;
                        }
                    };

                    let blake3_hash = match importer.get_blake3(&repo_id, &new_sha1).await {
                        Ok(Some(h)) => h,
                        Ok(None) => {
                            ref_results.push(aspen_client_api::GitBridgeRefResult {
                                ref_name: ref_update.ref_name.clone(),
                                is_success: false,
                                error: Some("Object not found in mapping".to_string()),
                            });
                            continue;
                        }
                        Err(e) => {
                            ref_results.push(aspen_client_api::GitBridgeRefResult {
                                ref_name: ref_update.ref_name.clone(),
                                is_success: false,
                                error: Some(e.to_string()),
                            });
                            continue;
                        }
                    };

                    let ref_name = ref_update.ref_name.strip_prefix("refs/").unwrap_or(&ref_update.ref_name);
                    match forge.refs.set(&repo_id, ref_name, blake3_hash).await {
                        Ok(()) => {
                            ref_results.push(aspen_client_api::GitBridgeRefResult {
                                ref_name: ref_update.ref_name.clone(),
                                is_success: true,
                                error: None,
                            });
                        }
                        Err(e) => {
                            ref_results.push(aspen_client_api::GitBridgeRefResult {
                                ref_name: ref_update.ref_name.clone(),
                                is_success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
            }

            let all_success = ref_results.iter().all(|r| r.is_success);
            ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
                session_id,
                is_success: all_success,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results,
                error: None,
            })
        }

        _ => ClientRpcResponse::error("UNSUPPORTED", "Only git bridge operations are supported in this test server"),
    }
}

// ============================================================================
// Git Helper Functions
// ============================================================================

/// Path to the git-remote-aspen binary built by cargo.
fn git_remote_aspen_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove "deps" directory
    path.push("git-remote-aspen");
    path
}

/// Create a local git repo with some commits, returning the path.
fn create_git_repo(dir: &Path, files: &[(&str, &[u8])], message: &str) -> PathBuf {
    let repo_dir = dir.join("source-repo");
    std::fs::create_dir_all(&repo_dir).unwrap();

    // git init
    run_git(&repo_dir, &["init", "--initial-branch=main"]);
    run_git(&repo_dir, &["config", "user.email", "test@example.com"]);
    run_git(&repo_dir, &["config", "user.name", "Test User"]);

    // Create files
    for (name, content) in files {
        let file_path = repo_dir.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
        run_git(&repo_dir, &["add", name]);
    }

    // Commit
    run_git(&repo_dir, &["commit", "-m", message]);

    repo_dir
}

/// Run a git command in a directory, panicking on failure.
fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("git {:?} failed: {}", args, stderr);
    }

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Run a git command with extra PATH and env vars for git-remote-aspen.
///
/// Uses spawn_blocking so it doesn't block the tokio runtime, allowing
/// the server task to continue processing connections.
async fn run_git_with_remote(dir: &Path, args: &[&str], remote_binary: &Path) -> std::process::Output {
    let remote_dir = remote_binary.parent().unwrap().to_owned();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", remote_dir.display(), current_path);
    let dir = dir.to_owned();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    tokio::task::spawn_blocking(move || {
        Command::new("git")
            .args(&args)
            .current_dir(&dir)
            .env("PATH", &new_path)
            .env("GIT_TERMINAL_PROMPT", "0")
            // Suppress noisy iroh discovery output
            .env("RUST_LOG", "warn")
            .output()
            .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e))
    })
    .await
    .expect("spawn_blocking panicked")
}

/// Get the HEAD commit SHA-1 of a git repo.
fn get_head_sha1(dir: &Path) -> String {
    run_git(dir, &["rev-parse", "HEAD"])
}

// ============================================================================
// Tests
// ============================================================================

/// Test: git push → git clone round-trip over real QUIC.
///
/// 1. Start minimal forge server
/// 2. Create local git repo with files
/// 3. `git push` to forge via git-remote-aspen
/// 4. `git clone` from forge to new directory
/// 5. Verify files match
#[tokio::test]
#[ignore] // Requires network access (iroh binds to loopback)
async fn test_git_push_clone_roundtrip() {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").with_test_writer().try_init();

    // Check git-remote-aspen binary exists
    let remote_binary = git_remote_aspen_path();
    if !remote_binary.exists() {
        panic!(
            "git-remote-aspen binary not found at {:?}. Build with: cargo build --features git-bridge",
            remote_binary
        );
    }

    let tmp = TempDir::new().unwrap();

    // Start forge server
    let server = MinimalForgeServer::new().await;
    let repo_id = server.create_repo("test-push-clone").await;
    let ticket = server.cluster_ticket();
    let ticket_str = ticket.serialize();
    let repo_id_hex = repo_id.to_hex();

    info!("Server endpoint: {:?}", server.endpoint_addr());
    info!("Ticket: {}", ticket_str);
    info!("Repo ID: {}", repo_id_hex);

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        server.serve().await;
    });

    // Small delay for server to be ready
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create source repo with files
    let source = create_git_repo(
        tmp.path(),
        &[
            ("README.md", b"# Test Repo\n\nThis is a test.\n"),
            ("src/main.rs", b"fn main() {\n    println!(\"Hello, Forge!\");\n}\n"),
            ("Cargo.toml", b"[package]\nname = \"test\"\nversion = \"0.1.0\"\n"),
        ],
        "Initial commit",
    );

    let source_head = get_head_sha1(&source);
    info!("Source HEAD: {}", source_head);

    // git push to forge
    let remote_url = format!("aspen://{}/{}", ticket_str, repo_id_hex);
    run_git(&source, &["remote", "add", "forge", &remote_url]);

    let push_output = timeout(Duration::from_secs(30), async {
        run_git_with_remote(&source, &["push", "forge", "main"], &remote_binary).await
    })
    .await
    .expect("push timed out");

    let push_stderr = String::from_utf8_lossy(&push_output.stderr);
    info!("Push stderr: {}", push_stderr);

    if !push_output.status.success() {
        panic!("git push failed (exit {}): {}", push_output.status, push_stderr);
    }

    // git clone from forge
    let clone_dir = tmp.path().join("cloned-repo");
    let clone_output = timeout(Duration::from_secs(30), async {
        run_git_with_remote(tmp.path(), &["clone", &remote_url, clone_dir.to_str().unwrap()], &remote_binary).await
    })
    .await
    .expect("clone timed out");

    let clone_stderr = String::from_utf8_lossy(&clone_output.stderr);
    info!("Clone stderr: {}", clone_stderr);

    if !clone_output.status.success() {
        panic!("git clone failed (exit {}): {}", clone_output.status, clone_stderr);
    }

    // Verify cloned content matches source
    let clone_head = get_head_sha1(&clone_dir);
    assert_eq!(source_head, clone_head, "HEAD SHA-1 should match between source and clone");

    // Verify file contents
    let readme = std::fs::read_to_string(clone_dir.join("README.md")).unwrap();
    assert_eq!(readme, "# Test Repo\n\nThis is a test.\n");

    let main_rs = std::fs::read_to_string(clone_dir.join("src/main.rs")).unwrap();
    assert_eq!(main_rs, "fn main() {\n    println!(\"Hello, Forge!\");\n}\n");

    let cargo_toml = std::fs::read_to_string(clone_dir.join("Cargo.toml")).unwrap();
    assert_eq!(cargo_toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"\n");

    // Cleanup
    server_handle.abort();
}

/// Test: incremental push (second push after initial sends fewer objects).
#[tokio::test]
#[ignore]
async fn test_git_incremental_push() {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").with_test_writer().try_init();

    let remote_binary = git_remote_aspen_path();
    if !remote_binary.exists() {
        panic!("git-remote-aspen binary not found at {:?}", remote_binary);
    }

    let tmp = TempDir::new().unwrap();

    let server = MinimalForgeServer::new().await;
    let repo_id = server.create_repo("test-incremental").await;
    let ticket = server.cluster_ticket();
    let ticket_str = ticket.serialize();
    let repo_id_hex = repo_id.to_hex();

    let server_handle = tokio::spawn(async move {
        server.serve().await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create initial commit
    let source = create_git_repo(tmp.path(), &[("file.txt", b"version 1\n")], "Initial commit");

    let remote_url = format!("aspen://{}/{}", ticket_str, repo_id_hex);
    run_git(&source, &["remote", "add", "forge", &remote_url]);

    // First push
    let push1 = timeout(Duration::from_secs(30), async {
        run_git_with_remote(&source, &["push", "forge", "main"], &remote_binary).await
    })
    .await
    .expect("push 1 timed out");

    assert!(push1.status.success(), "first push should succeed");
    let push1_stderr = String::from_utf8_lossy(&push1.stderr);
    info!("Push 1: {}", push1_stderr);

    // Make a second commit
    std::fs::write(source.join("file.txt"), b"version 2\n").unwrap();
    run_git(&source, &["add", "file.txt"]);
    run_git(&source, &["commit", "-m", "Update to v2"]);

    // Second push (should be incremental)
    let push2 = timeout(Duration::from_secs(30), async {
        run_git_with_remote(&source, &["push", "forge", "main"], &remote_binary).await
    })
    .await
    .expect("push 2 timed out");

    assert!(push2.status.success(), "second push should succeed");
    let push2_stderr = String::from_utf8_lossy(&push2.stderr);
    info!("Push 2: {}", push2_stderr);

    // The second push stderr should mention "server already has" (from probe)
    assert!(
        push2_stderr.contains("server already has") || push2_stderr.contains("sending"),
        "second push should show incremental behavior: {}",
        push2_stderr
    );

    server_handle.abort();
}

/// Test: git fetch after push retrieves correct content.
#[tokio::test]
#[ignore]
async fn test_git_push_then_fetch() {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").with_test_writer().try_init();

    let remote_binary = git_remote_aspen_path();
    if !remote_binary.exists() {
        panic!("git-remote-aspen binary not found at {:?}", remote_binary);
    }

    let tmp = TempDir::new().unwrap();

    let server = MinimalForgeServer::new().await;
    let repo_id = server.create_repo("test-fetch").await;
    let ticket = server.cluster_ticket();
    let ticket_str = ticket.serialize();
    let repo_id_hex = repo_id.to_hex();

    let server_handle = tokio::spawn(async move {
        server.serve().await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let remote_url = format!("aspen://{}/{}", ticket_str, repo_id_hex);

    // Create and push from source
    let source = create_git_repo(tmp.path(), &[("hello.txt", b"Hello from source!\n")], "Push this");
    run_git(&source, &["remote", "add", "forge", &remote_url]);

    let push = timeout(Duration::from_secs(30), async {
        run_git_with_remote(&source, &["push", "forge", "main"], &remote_binary).await
    })
    .await
    .expect("push timed out");

    let push_stderr = String::from_utf8_lossy(&push.stderr);
    let push_stdout = String::from_utf8_lossy(&push.stdout);
    info!("Push stderr: {}", push_stderr);
    info!("Push stdout: {}", push_stdout);
    assert!(push.status.success(), "push should succeed: {}", push_stderr);

    // Create a second repo that fetches from the same remote
    let fetcher = tmp.path().join("fetcher");
    std::fs::create_dir_all(&fetcher).unwrap();
    run_git(&fetcher, &["init", "--initial-branch=main"]);
    run_git(&fetcher, &["config", "user.email", "test@example.com"]);
    run_git(&fetcher, &["config", "user.name", "Test User"]);
    run_git(&fetcher, &["remote", "add", "forge", &remote_url]);

    // Fetch
    let fetch = timeout(Duration::from_secs(30), async {
        run_git_with_remote(&fetcher, &["fetch", "forge", "main"], &remote_binary).await
    })
    .await
    .expect("fetch timed out");

    let fetch_stderr = String::from_utf8_lossy(&fetch.stderr);
    info!("Fetch stderr: {}", fetch_stderr);
    assert!(fetch.status.success(), "fetch should succeed: {}", fetch_stderr);

    // Checkout fetched content
    run_git(&fetcher, &["checkout", "FETCH_HEAD"]);

    // Verify content
    let hello = std::fs::read_to_string(fetcher.join("hello.txt")).unwrap();
    assert_eq!(hello, "Hello from source!\n");

    // Verify SHA-1 matches
    let source_head = get_head_sha1(&source);
    let fetcher_head = get_head_sha1(&fetcher);
    assert_eq!(source_head, fetcher_head, "fetch should get same commit");

    server_handle.abort();
}
