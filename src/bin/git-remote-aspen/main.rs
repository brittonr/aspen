//! Git remote helper for Aspen Forge.
//!
//! This binary implements the Git remote helper protocol, enabling standard
//! git commands to work with Aspen Forge repositories.
//!
//! ## Usage
//!
//! Git invokes this helper automatically for URLs with the `aspen://` scheme:
//!
//! ```bash
//! git clone aspen://<ticket>/<repo_id> my-repo
//! git fetch aspen
//! git push aspen main
//! ```
//!
//! The helper reads commands from stdin and writes responses to stdout,
//! following the protocol described in `git-remote-helpers(7)`.
//!
//! ## URL Formats
//!
//! - `aspen://<ticket>/<repo_id>` - Connect via cluster ticket
//! - `aspen://<node_id>/<repo_id>` - Direct connection to a node
//!
//! ## Capabilities
//!
//! - `fetch` - Pull refs and objects from Forge
//! - `push` - Push refs and objects to Forge
//! - `option` - Configure transport options

mod protocol;
mod url;

use std::io::Write;
use std::io::{self};
use std::time::Duration;

use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::GitBridgeFetchResponse;
use aspen::client_rpc::GitBridgeListRefsResponse;
use aspen::client_rpc::GitBridgePushResponse;
use aspen::client_rpc::GitBridgeRefUpdate;
use aspen::cluster::ticket::AspenClusterTicket;
use aspen_constants::MAX_GIT_OBJECT_SIZE;
use aspen_constants::MAX_GIT_OBJECT_TREE_DEPTH;
use aspen_constants::MAX_GIT_OBJECTS_PER_PUSH;
use aspen_constants::MAX_GIT_PACKED_REFS_SIZE;
use aspen_constants::MAX_KEY_FILE_SIZE;
use protocol::Command;
use protocol::ProtocolReader;
use protocol::ProtocolWriter;
use url::AspenUrl;
use url::ConnectionTarget;

/// RPC timeout for git bridge operations.
/// Large pushes with many objects may take a while to process, especially trees
/// which are processed sequentially on the server for topological ordering.
/// For repositories with 5000+ trees, processing can take 30-60 seconds.
const RPC_TIMEOUT: Duration = Duration::from_secs(600);

/// Maximum number of retry attempts for RPC calls.
const MAX_RETRIES: u32 = 3;

/// Delay between retry attempts.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Maximum bytes per push batch.
///
/// We've configured the QUIC transport to support 64MB stream receive windows,
/// so we can use larger batches. Larger batches mean fewer cross-batch
/// dependencies, which simplifies the topological ordering requirements.
/// Objects are sent in batches, with ref updates only on the final batch.
///
/// Note: 4MB batches balance cross-batch dependencies vs. per-batch processing time.
/// Too large and the server times out; too small and tree dependencies span batches.
const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024; // 4MB

/// Maximum objects per push batch.
///
/// Even if batch is under MAX_BATCH_BYTES, limit object count to prevent server timeout.
/// Commits are small (~500 bytes) so thousands fit in one batch by bytes alone,
/// but processing them through Raft takes time.
const MAX_BATCH_OBJECTS: usize = 500;

/// Supported options and their current values.
struct Options {
    /// Verbosity level (0 = quiet, 1 = normal, 2+ = verbose).
    verbosity: u32,
    /// Progress reporting enabled.
    #[allow(dead_code)]
    progress: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbosity: 1,
            progress: true,
        }
    }
}

/// Remote helper state.
struct RemoteHelper {
    /// Remote name (e.g., "origin").
    #[allow(dead_code)]
    remote_name: String,
    /// Parsed URL.
    url: AspenUrl,
    /// Current options.
    options: Options,
    /// RPC client (initialized on first use).
    client: Option<RpcClient>,
}

/// Simple RPC client for git bridge operations.
struct RpcClient {
    endpoint: iroh::Endpoint,
    ticket: AspenClusterTicket,
}

impl RpcClient {
    /// Connect to an Aspen cluster using a ticket.
    async fn connect(ticket: AspenClusterTicket) -> io::Result<Self> {
        use iroh::endpoint::TransportConfig;
        use iroh::endpoint::VarInt;

        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        // Configure transport for large git operations
        let mut transport_config = TransportConfig::default();
        // Set stream receive window to 64MB to handle large git objects
        transport_config.stream_receive_window(VarInt::from_u32(64 * 1024 * 1024));
        // Set connection receive window to 256MB
        transport_config.receive_window(VarInt::from_u32(256 * 1024 * 1024));
        // Set idle timeout high enough to handle server-side processing of large batches.
        // Server may take 30-60 seconds to process 5000+ tree objects sequentially.
        // Default QUIC idle timeout is often 30-60s, which is too short.
        transport_config.max_idle_timeout(Some(RPC_TIMEOUT.try_into().unwrap()));

        let endpoint = iroh::Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![aspen::CLIENT_ALPN.to_vec()])
            .transport_config(transport_config)
            .bind()
            .await
            .map_err(io::Error::other)?;

        Ok(Self { endpoint, ticket })
    }

    /// Send an RPC request with retry logic.
    async fn send(&self, request: ClientRpcRequest) -> io::Result<ClientRpcResponse> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                eprintln!("git-remote-aspen: retrying RPC (attempt {})", attempt + 1);
                tokio::time::sleep(RETRY_DELAY).await;
            }

            match self.send_once(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    eprintln!("git-remote-aspen: RPC attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| io::Error::other(format!("RPC failed after {} retries", MAX_RETRIES))))
    }

    /// Send a single RPC request without retry.
    async fn send_once(&self, request: ClientRpcRequest) -> io::Result<ClientRpcResponse> {
        use aspen::client_rpc::AuthenticatedRequest;
        use aspen::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
        use iroh::EndpointAddr;
        use tokio::time::timeout;

        // Get first bootstrap peer
        let peer_id = *self
            .ticket
            .bootstrap
            .iter()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no bootstrap peers"))?;

        let target_addr = EndpointAddr::new(peer_id);

        // Connect with timeout
        let connection = timeout(RPC_TIMEOUT, async { self.endpoint.connect(target_addr, aspen::CLIENT_ALPN).await })
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connection timeout"))?
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;

        // Open stream
        let (mut send, mut recv) = connection.open_bi().await.map_err(io::Error::other)?;

        // Send request (unauthenticated for now)
        let auth_request = AuthenticatedRequest::unauthenticated(request);
        let request_bytes =
            postcard::to_stdvec(&auth_request).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        eprintln!(
            "git-remote-aspen: sending {} bytes ({:.2} MB)",
            request_bytes.len(),
            request_bytes.len() as f64 / (1024.0 * 1024.0)
        );
        send.write_all(&request_bytes).await.map_err(io::Error::other)?;
        send.finish().map_err(io::Error::other)?;

        // Read response
        let response_bytes = timeout(RPC_TIMEOUT, async { recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await })
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response timeout"))?
            .map_err(io::Error::other)?;

        // Deserialize
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Close connection
        connection.close(iroh::endpoint::VarInt::from_u32(0), b"done");

        Ok(response)
    }

    /// Shutdown the client.
    async fn shutdown(self) {
        self.endpoint.close().await;
    }
}

impl RemoteHelper {
    /// Create a new remote helper.
    fn new(remote_name: String, url_str: &str) -> io::Result<Self> {
        let url = AspenUrl::parse(url_str)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid URL: {e}")))?;

        Ok(Self {
            remote_name,
            url,
            options: Options::default(),
            client: None,
        })
    }

    /// Get or create the RPC client.
    async fn get_client(&mut self) -> io::Result<&RpcClient> {
        if self.client.is_none() {
            let ticket = match &self.url.target {
                ConnectionTarget::Ticket(t) => t.clone(),
                ConnectionTarget::SignedTicket(s) => s.ticket.clone(),
                ConnectionTarget::NodeId(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "direct node ID connections not yet supported, use a cluster ticket",
                    ));
                }
            };

            if ticket.bootstrap.is_empty() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "cluster ticket has no bootstrap peers"));
            }

            let client = RpcClient::connect(ticket).await?;
            self.client = Some(client);
        }

        // Tiger Style: avoid unwrap even when logically safe after initialization above
        self.client.as_ref().ok_or_else(|| io::Error::other("internal error: client not initialized"))
    }

    /// Run the remote helper protocol loop.
    async fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let mut reader = ProtocolReader::new(stdin.lock());
        let mut writer = ProtocolWriter::new(stdout.lock());

        loop {
            let cmd = reader.read_command()?;

            match cmd {
                Command::Capabilities => {
                    writer.write_capabilities()?;
                }

                Command::List { for_push } => {
                    self.handle_list(&mut writer, for_push).await?;
                }

                Command::Fetch { sha1, ref_name } => {
                    self.handle_fetch(&mut writer, &sha1, &ref_name).await?;
                }

                Command::Push { src, dst, force } => {
                    self.handle_push(&mut writer, &src, &dst, force).await?;
                }

                Command::Option { name, value } => {
                    self.handle_option(&mut writer, &name, &value)?;
                }

                Command::Empty => {
                    // End of commands
                    break;
                }

                Command::Unknown(line) => {
                    writer.write_error(&format!("unknown command: {line}"))?;
                }
            }
        }

        // Cleanup
        if let Some(client) = self.client.take() {
            client.shutdown().await;
        }

        Ok(())
    }

    /// Handle the "list" command.
    async fn handle_list<W: Write>(&mut self, writer: &mut ProtocolWriter<W>, _for_push: bool) -> io::Result<()> {
        let repo_id = self.url.repo_id().to_hex();

        if self.options.verbosity > 1 {
            eprintln!("git-remote-aspen: listing refs for {}", repo_id);
        }

        // Get client and send RPC
        let client = self.get_client().await?;
        let request = ClientRpcRequest::GitBridgeListRefs { repo_id };

        let response = client.send(request).await?;

        match response {
            ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                success,
                refs,
                head,
                error,
            }) => {
                if !success {
                    let msg = error.unwrap_or_else(|| "unknown error".to_string());
                    eprintln!("git-remote-aspen: list failed: {}", msg);
                    return writer.write_end();
                }

                // Write HEAD symref if present
                if let Some(head_target) = head
                    && let Some(head_ref) = refs.iter().find(|r| r.ref_name == head_target)
                {
                    writer.write_head_symref(&head_ref.sha1, &head_target)?;
                }

                // Write all refs
                for ref_info in refs {
                    writer.write_ref(&ref_info.sha1, &ref_info.ref_name)?;
                }

                writer.write_end()
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_end()
            }
        }
    }

    /// Handle the "fetch" command.
    async fn handle_fetch<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        sha1: &str,
        ref_name: &str,
    ) -> io::Result<()> {
        let repo_id = self.url.repo_id().to_hex();

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: fetching {} {}", sha1, ref_name);
        }

        // Get list of commits we already have locally (must be done before mutable borrow of client)
        let have = self.get_local_commits(&repo_id)?;

        // Get client and send RPC
        let client = self.get_client().await?;

        let request = ClientRpcRequest::GitBridgeFetch {
            repo_id,
            want: vec![sha1.to_string()],
            have,
        };

        let response = client.send(request).await?;

        match response {
            ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                success,
                objects,
                skipped: _,
                error,
            }) => {
                if !success {
                    let msg = error.unwrap_or_else(|| "unknown error".to_string());
                    eprintln!("git-remote-aspen: fetch failed: {}", msg);
                    return writer.write_fetch_done();
                }

                if self.options.verbosity > 0 {
                    eprintln!("git-remote-aspen: received {} objects", objects.len());
                }

                // Write objects to git's object store as loose objects
                let git_dir = std::env::var("GIT_DIR").unwrap_or_else(|_| ".git".to_string());
                let objects_dir = std::path::Path::new(&git_dir).join("objects");

                for obj in &objects {
                    if let Err(e) = self.write_loose_object(&objects_dir, obj) {
                        eprintln!("git-remote-aspen: failed to write object {}: {}", obj.sha1, e);
                    } else if self.options.verbosity > 1 {
                        eprintln!("git-remote-aspen: wrote object {} ({})", obj.sha1, obj.object_type);
                    }
                }

                writer.write_fetch_done()
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_fetch_done()
            }
        }
    }

    /// Write a git object as a loose object file.
    fn write_loose_object(
        &self,
        objects_dir: &std::path::Path,
        obj: &aspen::client_rpc::GitBridgeObject,
    ) -> io::Result<()> {
        use std::io::Write as _;

        use flate2::Compression;
        use flate2::write::ZlibEncoder;

        // Build the full object content: "{type} {size}\0{data}"
        let header = format!("{} {}\0", obj.object_type, obj.data.len());
        let mut full_content = header.into_bytes();
        full_content.extend_from_slice(&obj.data);

        // Compress with zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&full_content)?;
        let compressed = encoder.finish()?;

        // Write to objects/{sha1[0..2]}/{sha1[2..]}
        let dir = objects_dir.join(&obj.sha1[0..2]);
        let file = dir.join(&obj.sha1[2..]);

        // Create directory if needed
        std::fs::create_dir_all(&dir)?;

        // Write file (skip if exists)
        if !file.exists() {
            std::fs::write(&file, &compressed)?;
        }

        Ok(())
    }

    /// Handle the "push" command.
    async fn handle_push<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        src: &str,
        dst: &str,
        force: bool,
    ) -> io::Result<()> {
        let repo_id = self.url.repo_id().to_hex();

        if self.options.verbosity > 0 {
            let force_str = if force { " (force)" } else { "" };
            eprintln!("git-remote-aspen: pushing {}:{}{}", src, dst, force_str);
        }

        let git_dir = std::env::var("GIT_DIR").unwrap_or_else(|_| ".git".to_string());
        let git_dir = std::path::Path::new(&git_dir);

        // Resolve src to a commit SHA-1
        let commit_sha1 = match self.resolve_ref(git_dir, src) {
            Ok(sha) => sha,
            Err(e) => {
                eprintln!("git-remote-aspen: failed to resolve {}: {}", src, e);
                return writer.write_push_error(dst, &format!("cannot resolve {}", src));
            }
        };

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: resolved {} to {}", src, commit_sha1);
        }

        // Collect all objects reachable from the commit using git rev-list --objects.
        // This is the correct way to enumerate objects - it only returns objects that
        // actually exist locally, handling shallow clones and missing history gracefully.
        let objects_dir = git_dir.join("objects");
        let objects = match self.collect_objects_for_push(&objects_dir, &commit_sha1) {
            Ok(objs) => objs,
            Err(e) => {
                eprintln!("git-remote-aspen: failed to collect objects: {}", e);
                return writer.write_push_error(dst, &format!("failed to read objects: {}", e));
            }
        };

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: collected {} objects to push", objects.len());
        }

        // Get remote ref value before mutable borrow of client
        let old_sha1 = self.get_remote_ref_value(&repo_id, dst).unwrap_or_default();

        // Send objects in byte-size-limited batches to avoid QUIC flow control issues.
        // The QUIC receive window is ~1MB by default, so we keep batches under MAX_BATCH_BYTES.
        // Each batch is sent with empty refs, and the final batch includes the ref update.
        let total_objects = objects.len();
        let mut total_imported = 0usize;
        let mut total_skipped = 0usize;
        let verbosity = self.options.verbosity;

        // Reorder objects to ensure dependencies are sent before dependents:
        // - Blobs first (no dependencies)
        // - Trees second (with --reverse flag on git rev-list, older trees come first)
        // - Commits/tags last (depend on trees and other commits)
        //
        // With `--reverse`, git rev-list outputs oldest objects first, which generally
        // means child trees appear before parent trees. We also reverse the tree group
        // to further ensure child-first ordering within the tree type.
        let (blobs, trees, commits, tags, other): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>) = {
            let mut blobs = Vec::new();
            let mut trees = Vec::new();
            let mut commits = Vec::new();
            let mut tags = Vec::new();
            let mut other = Vec::new();
            for obj in objects {
                match obj.object_type.as_str() {
                    "blob" => blobs.push(obj),
                    "tree" => trees.push(obj),
                    "commit" => commits.push(obj),
                    "tag" => tags.push(obj),
                    _ => other.push(obj),
                }
            }
            (blobs, trees, commits, tags, other)
        };

        // Note: With --reverse flag, git rev-list outputs older commits first, which should
        // give us better tree ordering (child trees before parent trees in most cases).
        // We keep trees in their original order from rev-list.

        // Build batches with special handling for trees.
        //
        // Trees must be kept together in a single batch because tree-to-tree dependencies
        // (subdirectories) can be complex and don't follow a simple ordering. The server
        // does topological sorting within a batch, but not across batches. Trees are
        // generally small (~100-300 bytes each), so even thousands fit in one batch.
        //
        // Strategy:
        // 1. Batch blobs (can be split, no dependencies)
        // 2. All trees in ONE batch (complex interdependencies)
        // 3. Batch commits (can be split, only depend on trees which are already sent)
        // 4. Tags and other at the end
        let mut batches: Vec<Vec<aspen::client_rpc::GitBridgeObject>> = Vec::new();

        // Helper to batch objects by byte/count limits
        let add_objects_batched =
            |objects: Vec<aspen::client_rpc::GitBridgeObject>,
             batches: &mut Vec<Vec<aspen::client_rpc::GitBridgeObject>>| {
                let mut current_batch: Vec<aspen::client_rpc::GitBridgeObject> = Vec::new();
                let mut current_batch_bytes = 0usize;

                for obj in objects {
                    let obj_size = obj.sha1.len() + obj.object_type.len() + obj.data.len() + 32;
                    let would_exceed_bytes = current_batch_bytes + obj_size > MAX_BATCH_BYTES;
                    let would_exceed_objects = current_batch.len() >= MAX_BATCH_OBJECTS;
                    if (would_exceed_bytes || would_exceed_objects) && !current_batch.is_empty() {
                        batches.push(std::mem::take(&mut current_batch));
                        current_batch_bytes = 0;
                    }
                    current_batch_bytes += obj_size;
                    current_batch.push(obj);
                }
                if !current_batch.is_empty() {
                    batches.push(current_batch);
                }
            };

        // 1. Batch blobs (no dependencies)
        if verbosity > 0 && !blobs.is_empty() {
            eprintln!("git-remote-aspen: batching {} blobs", blobs.len());
        }
        add_objects_batched(blobs, &mut batches);

        // 2. All trees in a single batch (complex interdependencies).
        //
        // Trees have complex interdependencies (subdirectories) that don't follow a
        // simple ordering pattern. The server does topological sorting within each batch,
        // but can't handle cross-batch dependencies where a tree in batch N references
        // a tree in batch N+1. By keeping all trees together, we ensure the server's
        // topological sort can correctly order them.
        //
        // Trees are generally small (~100-300 bytes each), so even thousands fit under
        // the MAX_BATCH_BYTES limit. The RPC_TIMEOUT is set high enough (600s) to handle
        // large tree batches on the server side.
        if !trees.is_empty() {
            if verbosity > 0 {
                let tree_bytes: usize = trees.iter().map(|t| t.data.len()).sum();
                eprintln!(
                    "git-remote-aspen: sending all {} trees together ({:.2} MB)",
                    trees.len(),
                    tree_bytes as f64 / (1024.0 * 1024.0)
                );
            }
            batches.push(trees);
        }

        // 3. Batch commits (depend on trees, which are now sent)
        if verbosity > 0 && !commits.is_empty() {
            eprintln!("git-remote-aspen: batching {} commits", commits.len());
        }
        add_objects_batched(commits, &mut batches);

        // 4. Tags and other
        if !tags.is_empty() {
            add_objects_batched(tags, &mut batches);
        }
        if !other.is_empty() {
            add_objects_batched(other, &mut batches);
        }

        let num_batches = batches.len();
        if num_batches > 1 && verbosity > 0 {
            eprintln!("git-remote-aspen: sending {} objects in {} batches", total_objects, num_batches);
        }

        let client = self.get_client().await?;

        for (batch_idx, batch) in batches.into_iter().enumerate() {
            let is_last_batch = batch_idx == num_batches - 1;

            // Only include ref update on the final batch
            let refs = if is_last_batch {
                vec![GitBridgeRefUpdate {
                    ref_name: dst.to_string(),
                    old_sha1: old_sha1.clone(),
                    new_sha1: commit_sha1.clone(),
                    force,
                }]
            } else {
                vec![]
            };

            if verbosity > 0 && num_batches > 1 {
                eprintln!(
                    "git-remote-aspen: sending batch {}/{} ({} objects)",
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );
            }

            let request = ClientRpcRequest::GitBridgePush {
                repo_id: repo_id.clone(),
                objects: batch,
                refs,
            };

            let response = client.send(request).await?;

            match response {
                ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                    success,
                    objects_imported,
                    objects_skipped,
                    ref_results,
                    error,
                }) => {
                    if verbosity > 1 {
                        eprintln!(
                            "git-remote-aspen: batch {}/{} response: success={}, imported={}, skipped={}",
                            batch_idx + 1,
                            num_batches,
                            success,
                            objects_imported,
                            objects_skipped
                        );
                    }

                    if !success {
                        let msg = error.unwrap_or_else(|| "unknown error".to_string());
                        return writer.write_push_error(dst, &msg);
                    }

                    total_imported += objects_imported;
                    total_skipped += objects_skipped;

                    // Only process ref results on the final batch
                    if is_last_batch {
                        if verbosity > 0 {
                            eprintln!(
                                "git-remote-aspen: push complete: imported {} objects, skipped {}",
                                total_imported, total_skipped
                            );
                        }

                        // Report results for each ref
                        for result in ref_results {
                            if result.success {
                                writer.write_push_ok(&result.ref_name)?;
                            } else {
                                let msg = result.error.unwrap_or_else(|| "unknown error".to_string());
                                writer.write_push_error(&result.ref_name, &msg)?;
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("git-remote-aspen: unexpected response type");
                    return writer.write_push_error(dst, "unexpected response");
                }
            }
        }

        // Terminal empty line to signal end of push response batch
        writer.write_end()
    }

    /// Resolve a git ref to a SHA-1 hash.
    fn resolve_ref(&self, git_dir: &std::path::Path, refspec: &str) -> io::Result<String> {
        // First, check if it's already a full SHA-1
        if refspec.len() == 40 && refspec.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(refspec.to_string());
        }

        // Try to resolve as a ref file
        let ref_paths = [
            git_dir.join(refspec),
            git_dir.join("refs/heads").join(refspec),
            git_dir.join("refs/tags").join(refspec),
        ];

        for path in &ref_paths {
            if path.exists() {
                // Tiger Style: Check file size before reading (ref files are small)
                let metadata = std::fs::metadata(path)?;
                if metadata.len() > MAX_KEY_FILE_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("ref file too large: {} bytes", metadata.len()),
                    ));
                }
                let content = std::fs::read_to_string(path)?;
                let sha = content.trim();
                // Handle symbolic refs
                if let Some(ref_target) = sha.strip_prefix("ref: ") {
                    return self.resolve_ref(git_dir, ref_target);
                }
                return Ok(sha.to_string());
            }
        }

        // Try packed-refs
        let packed_refs = git_dir.join("packed-refs");
        if packed_refs.exists() {
            // Tiger Style: Check file size before reading
            let metadata = std::fs::metadata(&packed_refs)?;
            if metadata.len() > MAX_GIT_PACKED_REFS_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("packed-refs too large: {} bytes", metadata.len()),
                ));
            }
            let content = std::fs::read_to_string(&packed_refs)?;
            for line in content.lines() {
                if line.starts_with('#') {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let sha = parts[0];
                    let ref_name = parts[1];
                    if ref_name.ends_with(refspec) || ref_name == refspec {
                        return Ok(sha.to_string());
                    }
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, format!("ref not found: {}", refspec)))
    }

    /// Collect all objects for a push using `git rev-list --objects`.
    ///
    /// This is the correct approach for push operations because:
    /// 1. It only returns objects that actually exist locally
    /// 2. It handles shallow clones and missing history gracefully
    /// 3. It's much faster than manual traversal for large repos
    ///
    /// Tiger Style: Bounds checking prevents memory exhaustion (CRIT-002)
    fn collect_objects_for_push(
        &self,
        objects_dir: &std::path::Path,
        commit_sha1: &str,
    ) -> io::Result<Vec<aspen::client_rpc::GitBridgeObject>> {
        // NOTE: For now, don't limit commit depth. The message size limit (64MB) handles
        // large pushes. Limiting commits causes issues with parent commit dependencies.
        // TODO: Implement proper incremental push that only sends commits the remote doesn't have.
        // const MAX_COMMIT_DEPTH: u32 = 100;

        // Use git rev-list --objects --reverse to enumerate all objects reachable from the commit.
        // --reverse outputs oldest commits first, which helps with tree dependency ordering.
        // This handles packed objects, shallow clones, and missing history gracefully.
        let output = std::process::Command::new("git")
            .args(["rev-list", "--objects", "--reverse", commit_sha1])
            .output()
            .map_err(|e| io::Error::other(format!("failed to run git rev-list: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::other(format!("git rev-list failed: {}", stderr.trim())));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut objects = Vec::new();
        let mut count = 0u32;

        for line in stdout.lines() {
            // Tiger Style CRIT-002: Prevent memory exhaustion
            count += 1;
            if count > MAX_GIT_OBJECTS_PER_PUSH {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("exceeded maximum objects per push ({})", MAX_GIT_OBJECTS_PER_PUSH),
                ));
            }

            // Each line is either "sha1" (commit) or "sha1 path" (tree entry/blob)
            let sha1 = line.split_whitespace().next().unwrap_or("");
            if sha1.len() != 40 {
                continue;
            }

            // Read the object content
            match self.read_git_object(objects_dir, sha1) {
                Ok((object_type, data)) => {
                    objects.push(aspen::client_rpc::GitBridgeObject {
                        sha1: sha1.to_string(),
                        object_type,
                        data,
                    });
                }
                Err(e) => {
                    // Log but skip objects we can't read (may be filtered out)
                    if self.options.verbosity > 1 {
                        eprintln!("git-remote-aspen: skipping object {}: {}", sha1, e);
                    }
                }
            }
        }

        Ok(objects)
    }

    /// Collect all objects reachable from a commit.
    ///
    /// NOTE: This function is kept for reference but `collect_objects_for_push` should
    /// be preferred for push operations as it uses `git rev-list` which handles
    /// shallow clones and missing objects gracefully.
    ///
    /// Tiger Style: depth parameter and bounds checking prevent:
    /// - Stack overflow from deeply nested structures (CRIT-001)
    /// - Memory exhaustion from repositories with millions of objects (CRIT-002)
    #[allow(dead_code)]
    fn collect_objects(
        &self,
        objects_dir: &std::path::Path,
        sha1: &str,
        objects: &mut Vec<aspen::client_rpc::GitBridgeObject>,
        visited: &mut std::collections::HashSet<String>,
        depth: u32,
    ) -> io::Result<()> {
        // Tiger Style CRIT-001: Prevent stack overflow from malicious deep nesting
        if depth > MAX_GIT_OBJECT_TREE_DEPTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("git object tree depth {} exceeds maximum ({})", depth, MAX_GIT_OBJECT_TREE_DEPTH),
            ));
        }

        // Tiger Style CRIT-002: Prevent memory exhaustion from huge repositories
        if visited.len() >= MAX_GIT_OBJECTS_PER_PUSH as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("exceeded maximum objects per push ({})", MAX_GIT_OBJECTS_PER_PUSH),
            ));
        }

        if visited.contains(sha1) {
            return Ok(());
        }
        visited.insert(sha1.to_string());

        // Read the object (uses git cat-file to handle both loose and packed objects)
        let (object_type, data) = self.read_git_object(objects_dir, sha1)?;

        // Recursively collect referenced objects (Tiger Style: pass depth + 1)
        match object_type.as_str() {
            "commit" => {
                self.collect_commit_refs(objects_dir, &data, objects, visited, depth)?;
            }
            "tree" => {
                self.collect_tree_refs(objects_dir, &data, objects, visited, depth)?;
            }
            "tag" => {
                self.collect_tag_refs(objects_dir, &data, objects, visited, depth)?;
            }
            "blob" => {
                // Blobs don't reference other objects
            }
            _ => {}
        }

        // Add this object to the list
        objects.push(aspen::client_rpc::GitBridgeObject {
            sha1: sha1.to_string(),
            object_type,
            data,
        });

        Ok(())
    }

    /// Parse commit object and collect referenced tree and parent objects.
    #[allow(dead_code)]
    fn collect_commit_refs(
        &self,
        objects_dir: &std::path::Path,
        data: &[u8],
        objects: &mut Vec<aspen::client_rpc::GitBridgeObject>,
        visited: &mut std::collections::HashSet<String>,
        depth: u32,
    ) -> io::Result<()> {
        let Ok(content) = std::str::from_utf8(data) else {
            return Ok(());
        };
        for line in content.lines() {
            if let Some(tree_sha) = line.strip_prefix("tree ") {
                self.collect_objects(objects_dir, tree_sha, objects, visited, depth + 1)?;
            } else if let Some(parent_sha) = line.strip_prefix("parent ") {
                self.collect_objects(objects_dir, parent_sha, objects, visited, depth + 1)?;
            } else if line.is_empty() {
                break; // End of headers
            }
        }
        Ok(())
    }

    /// Parse tree object and collect referenced entry objects.
    #[allow(dead_code)]
    fn collect_tree_refs(
        &self,
        objects_dir: &std::path::Path,
        data: &[u8],
        objects: &mut Vec<aspen::client_rpc::GitBridgeObject>,
        visited: &mut std::collections::HashSet<String>,
        depth: u32,
    ) -> io::Result<()> {
        let mut pos = 0;
        while pos < data.len() {
            // Find space after mode
            let space_pos = data[pos..].iter().position(|&b| b == b' ').map(|p| pos + p);
            let Some(space) = space_pos else { break };

            // Find null after filename
            let null_pos = data[space + 1..].iter().position(|&b| b == 0).map(|p| space + 1 + p);
            let Some(null) = null_pos else { break };

            // Extract the 20-byte SHA-1
            if null + 21 > data.len() {
                break;
            }
            let entry_sha = hex::encode(&data[null + 1..null + 21]);
            self.collect_objects(objects_dir, &entry_sha, objects, visited, depth + 1)?;
            pos = null + 21;
        }
        Ok(())
    }

    /// Parse tag object and collect referenced target object.
    #[allow(dead_code)]
    fn collect_tag_refs(
        &self,
        objects_dir: &std::path::Path,
        data: &[u8],
        objects: &mut Vec<aspen::client_rpc::GitBridgeObject>,
        visited: &mut std::collections::HashSet<String>,
        depth: u32,
    ) -> io::Result<()> {
        let Ok(content) = std::str::from_utf8(data) else {
            return Ok(());
        };
        for line in content.lines() {
            if let Some(target_sha) = line.strip_prefix("object ") {
                self.collect_objects(objects_dir, target_sha, objects, visited, depth + 1)?;
                break;
            }
        }
        Ok(())
    }

    /// Read a git object using git cat-file.
    ///
    /// This handles both loose objects and packed objects, unlike direct file reads.
    /// Uses bounded reads to prevent memory exhaustion.
    fn read_git_object(&self, _objects_dir: &std::path::Path, sha1: &str) -> io::Result<(String, Vec<u8>)> {
        // Get object type
        let type_output = std::process::Command::new("git")
            .args(["cat-file", "-t", sha1])
            .output()
            .map_err(|e| io::Error::other(format!("failed to run git cat-file -t: {}", e)))?;

        if !type_output.status.success() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("git object not found: {}", sha1)));
        }

        let object_type = String::from_utf8_lossy(&type_output.stdout).trim().to_string();

        // Get object size first to check bounds
        let size_output = std::process::Command::new("git")
            .args(["cat-file", "-s", sha1])
            .output()
            .map_err(|e| io::Error::other(format!("failed to run git cat-file -s: {}", e)))?;

        if size_output.status.success()
            && let Ok(size_str) = std::str::from_utf8(&size_output.stdout)
            && let Ok(size) = size_str.trim().parse::<u64>()
            && size > MAX_GIT_OBJECT_SIZE
        {
            return Err(io::Error::other(format!(
                "git object {} exceeds maximum size ({} bytes)",
                sha1, MAX_GIT_OBJECT_SIZE
            )));
        }

        // Get object content
        let content_output = std::process::Command::new("git")
            .args(["cat-file", &object_type, sha1])
            .output()
            .map_err(|e| io::Error::other(format!("failed to run git cat-file: {}", e)))?;

        if !content_output.status.success() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("failed to read git object: {}", sha1)));
        }

        Ok((object_type, content_output.stdout))
    }

    /// Handle the "option" command.
    fn handle_option<W: Write>(&mut self, writer: &mut ProtocolWriter<W>, name: &str, value: &str) -> io::Result<()> {
        let supported = match name {
            "verbosity" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.options.verbosity = v;
                    true
                } else {
                    false
                }
            }
            "progress" => {
                self.options.progress = value == "true";
                true
            }
            _ => false,
        };

        writer.write_option_response(supported)
    }

    /// Get list of commits that we already have locally for incremental fetch
    fn get_local_commits(&self, _repo_id: &str) -> io::Result<Vec<String>> {
        // Use git rev-list to get recent commits
        // For safety, limit to recent commits to avoid huge "have" lists
        let output = std::process::Command::new("git").args(["rev-list", "--max-count=1000", "--all"]).output();

        match output {
            Ok(output) if output.status.success() => {
                let commits = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .collect();
                Ok(commits)
            }
            _ => {
                // If we can't get local commits, return empty list (full fetch)
                Ok(Vec::new())
            }
        }
    }

    /// Get current value of a remote ref for compare-and-swap operations
    fn get_remote_ref_value(&self, _repo_id: &str, ref_name: &str) -> Option<String> {
        // Try to get the current remote ref value
        let output = std::process::Command::new("git")
            .args(["show-ref", &format!("refs/remotes/{}/{}", self.remote_name, ref_name)])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                // Parse "<sha> <refname>" format
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.split_whitespace().next().map(|s| s.to_string())
            }
            _ => None, // Ref doesn't exist yet
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: git-remote-aspen <remote-name> <url>");
        eprintln!();
        eprintln!("This is a Git remote helper for Aspen Forge.");
        eprintln!("It should be invoked by Git, not directly.");
        eprintln!();
        eprintln!("URL formats:");
        eprintln!("  aspen://<ticket>/<repo_id>  - Connect via cluster ticket");
        eprintln!("  aspen://<node_id>/<repo_id> - Direct node connection");
        std::process::exit(1);
    }

    let remote_name = &args[1];
    let url = &args[2];

    let mut helper = RemoteHelper::new(remote_name.clone(), url)?;
    helper.run().await
}
