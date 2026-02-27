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
//!
//! # Push Size Optimization
//!
//! ## Implemented: Incremental Push via Object Probing
//!
//! Push now uses a three-phase incremental protocol:
//!
//! 1. **Enumerate**: `git rev-list --objects` to get all reachable SHA-1s (fast, no data reading)
//! 2. **Probe**: `GitBridgeProbeObjects` RPC asks server which SHA-1s it already has
//! 3. **Send**: Only read and send objects the server doesn't have
//!
//! For a typical push after initial import, this reduces objects from ~14K to ~10-50,
//! cutting transfer size by **90-99%** (e.g., 131 MB → 1-5 MB for subsequent pushes).
//!
//! If the server doesn't support probing (old version), falls back to full push.
//! If all objects are already known, ref update happens without any object transfer.
//!
//! ## Remaining Optimization Opportunities
//!
//! 1. **Wire compression** - Wrap RPC messages with zstd/lz4 compression. Expected reduction:
//!    ~60-70% on remaining data. Location: `RpcClient::send_once()` before `write_all()`.
//!
//! 2. **Git pack files** - Generate delta-compressed packs via `gix-pack`. Expected reduction: ~50%
//!    additional after compression. Changes: Replace `GitBridgeObject` with pack file bytes.
//!
//! ## Related Code Locations
//!
//! - Object enumeration: `enumerate_push_sha1s()` in this file
//! - Object probing: `GitBridgeProbeObjects` RPC handler in aspen-forge-handler
//! - Object collection: `collect_objects_for_push()` in this file
//! - Server import: `crates/aspen-forge/src/git/bridge/importer.rs`
//! - Chunked transfer: `GitBridgePushStart/Chunk/Complete` protocol

mod protocol;
mod url;

use std::io::Write;
use std::io::{self};
use std::time::Duration;

use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::ErrorResponse;
use aspen::client_rpc::GitBridgeFetchResponse;
use aspen::client_rpc::GitBridgeListRefsResponse;
use aspen::client_rpc::GitBridgeRefUpdate;
use aspen_core::MAX_GIT_OBJECT_SIZE;
use aspen_core::MAX_GIT_OBJECT_TREE_DEPTH;
use aspen_core::MAX_GIT_OBJECTS_PER_PUSH;
use aspen_core::MAX_GIT_PACKED_REFS_SIZE;
use aspen_core::MAX_KEY_FILE_SIZE;
use protocol::Command;
use protocol::ProtocolReader;
use protocol::ProtocolWriter;
use tracing_subscriber::EnvFilter;
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
/// Server-side imports now run in parallel (up to 32 concurrent), so we can
/// increase this from 500 to 2000. Larger batches reduce RPC overhead and
/// keep more work server-side where parallelism is leveraged.
///
/// Note: Commits are small (~500 bytes), so thousands fit in one batch by bytes.
/// The bottleneck is now I/O parallelism, not Raft consensus per object.
const MAX_BATCH_OBJECTS: usize = 2000;

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
    /// Bootstrap peer addresses (with direct socket addresses for V2 tickets).
    bootstrap_addrs: Vec<iroh::EndpointAddr>,
}

impl RpcClient {
    /// Connect to an Aspen cluster using endpoint addresses.
    ///
    /// For V2 tickets, these addresses include direct socket addresses for
    /// relay-less connectivity. For V1 tickets, only the node IDs are available.
    async fn connect(bootstrap_addrs: Vec<iroh::EndpointAddr>) -> io::Result<Self> {
        use iroh::endpoint::TransportConfig;
        use iroh::endpoint::VarInt;

        if bootstrap_addrs.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "no bootstrap peers"));
        }

        // Check if we have direct addresses in the bootstrap peers
        let has_direct_addrs = bootstrap_addrs.iter().any(|addr| !addr.addrs.is_empty());

        if !has_direct_addrs {
            eprintln!(
                "git-remote-aspen: warning: no direct addresses in ticket, connection may fail without discovery"
            );
        }

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
        // SAFETY: RPC_TIMEOUT (600 seconds) is well within QUIC IdleTimeout max (2^62 microseconds).
        // The conversion from Duration to IdleTimeout only fails for durations > ~146 years.
        transport_config
            .max_idle_timeout(Some(RPC_TIMEOUT.try_into().expect("RPC_TIMEOUT of 600s is valid for QUIC IdleTimeout")));

        // Build endpoint with or without discovery based on ticket type.
        // V2 tickets have direct addresses - prefer direct connection but keep
        // discovery as fallback. V1 tickets require discovery to resolve addresses.
        let mut builder = iroh::Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![aspen::CLIENT_ALPN.to_vec()])
            .transport_config(transport_config);

        // Only clear discovery if we have direct addresses AND they're local/private.
        // For remote connections, keep discovery enabled for relay fallback.
        let all_local = bootstrap_addrs.iter().all(|addr| {
            addr.addrs.iter().all(|a| match a {
                iroh::TransportAddr::Ip(sock) => {
                    let ip = sock.ip();
                    ip.is_loopback()
                        || match ip {
                            std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
                            std::net::IpAddr::V6(v6) => v6.is_loopback(),
                        }
                }
                _ => false,
            })
        });

        if has_direct_addrs && all_local {
            // Local addresses: disable discovery to avoid DNS/relay overhead
            eprintln!("git-remote-aspen: using direct local connection (discovery disabled)");
            builder = builder.clear_discovery();
        } else if !has_direct_addrs {
            // No direct addresses: we NEED discovery, keep it enabled (default)
            eprintln!("git-remote-aspen: discovery enabled (no direct addresses in ticket)");
        } else {
            // Remote addresses with discovery: keep discovery for relay fallback
            eprintln!("git-remote-aspen: discovery enabled for relay fallback");
        }

        let endpoint = builder.bind().await.map_err(io::Error::other)?;

        Ok(Self {
            endpoint,
            bootstrap_addrs,
        })
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
        use tokio::time::timeout;

        // Get first bootstrap peer address
        let target_addr = self
            .bootstrap_addrs
            .first()
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no bootstrap peers"))?;

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
        use iroh::EndpointAddr;

        if self.client.is_none() {
            // Extract endpoint addresses from the ticket.
            // V2 tickets include direct socket addresses for relay-less connectivity.
            // V1 tickets only have node IDs (requires discovery/relay).
            let bootstrap_addrs: Vec<EndpointAddr> = match &self.url.target {
                ConnectionTarget::Ticket(t) => {
                    // Ticket includes direct socket addresses for relay-less connectivity
                    t.endpoint_addrs()
                }
                ConnectionTarget::SignedTicket(s) => {
                    // Signed ticket: use endpoint addresses from inner ticket
                    s.ticket.endpoint_addrs()
                }
                ConnectionTarget::NodeId(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "direct node ID connections not yet supported, use a cluster ticket",
                    ));
                }
            };

            if bootstrap_addrs.is_empty() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "cluster ticket has no bootstrap peers"));
            }

            let client = RpcClient::connect(bootstrap_addrs).await?;
            self.client = Some(client);
        }

        // Tiger Style: avoid unwrap even when logically safe after initialization above
        self.client.as_ref().ok_or_else(|| io::Error::other("internal error: client not initialized"))
    }

    /// Run the remote helper protocol loop.
    ///
    /// Per git-remote-helpers(7), fetch and push commands arrive in **batches**
    /// terminated by a blank line. The helper must process the entire batch
    /// and respond with a single blank line when done. Processing each command
    /// individually breaks multi-branch repos (e.g., fetching main + dev).
    async fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let mut reader = ProtocolReader::new(stdin.lock());
        let mut writer = ProtocolWriter::new(stdout.lock());

        // Batch accumulators for fetch/push commands.
        // Git sends these in groups terminated by a blank line.
        let mut fetch_batch: Vec<(String, String)> = Vec::new();
        let mut push_batch: Vec<(String, String, bool)> = Vec::new();

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
                    // Accumulate — don't process until blank line
                    fetch_batch.push((sha1, ref_name));
                }

                Command::Push { src, dst, force } => {
                    // Accumulate — don't process until blank line
                    push_batch.push((src, dst, force));
                }

                Command::Option { name, value } => {
                    self.handle_option(&mut writer, &name, &value)?;
                }

                Command::Empty => {
                    // Blank line terminates a batch or the session.
                    if !fetch_batch.is_empty() {
                        let batch = std::mem::take(&mut fetch_batch);
                        self.handle_fetch_batch(&mut writer, &batch).await?;
                    } else if !push_batch.is_empty() {
                        let batch = std::mem::take(&mut push_batch);
                        self.handle_push_batch(&mut writer, &batch).await?;
                    } else {
                        // No pending batch — end of session
                        break;
                    }
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
                is_success,
                refs,
                head,
                error,
            }) => {
                if !is_success {
                    let msg = error.unwrap_or_else(|| "unknown error".to_string());
                    eprintln!("git-remote-aspen: list failed: {}", msg);
                    return writer.write_end();
                }

                // Write HEAD symref if present
                #[allow(clippy::collapsible_if)]
                if let Some(head_target) = head {
                    if let Some(head_ref) = refs.iter().find(|r| r.ref_name == head_target) {
                        writer.write_head_symref(&head_ref.sha1, &head_target)?;
                    }
                }

                // Write all refs
                for ref_info in refs {
                    writer.write_ref(&ref_info.sha1, &ref_info.ref_name)?;
                }

                writer.write_end()
            }
            ClientRpcResponse::Error(ErrorResponse { code, message }) => {
                eprintln!("git-remote-aspen: server error [{}]: {}", code, message);
                writer.write_end()
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_end()
            }
        }
    }

    /// Handle a batch of fetch commands.
    ///
    /// Per git-remote-helpers(7), multiple `fetch` lines arrive in a batch
    /// terminated by a blank line. We combine all wanted SHA-1s into a single
    /// RPC request, write all returned objects, then emit one blank line.
    async fn handle_fetch_batch<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        batch: &[(String, String)],
    ) -> io::Result<()> {
        let repo_id = self.url.repo_id().to_hex();

        // Deduplicate wanted SHA-1s (Git may request the same SHA for different refs)
        let mut seen = std::collections::HashSet::new();
        let mut want: Vec<String> = Vec::new();
        for (sha1, ref_name) in batch {
            if self.options.verbosity > 0 {
                eprintln!("git-remote-aspen: fetching {} {}", sha1, ref_name);
            }
            if seen.insert(sha1.clone()) {
                want.push(sha1.clone());
            }
        }

        if self.options.verbosity > 0 && batch.len() > 1 {
            eprintln!("git-remote-aspen: batch fetch: {} refs, {} unique objects requested", batch.len(), want.len());
        }

        // Get list of commits we already have locally (must be done before mutable borrow of client)
        let have = self.get_local_commits(&repo_id)?;

        // Single RPC with all wanted SHA-1s
        let client = self.get_client().await?;
        let request = ClientRpcRequest::GitBridgeFetch { repo_id, want, have };

        let response = client.send(request).await?;

        match response {
            ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                is_success,
                objects,
                skipped: _,
                error,
            }) => {
                if !is_success {
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
            ClientRpcResponse::Error(ErrorResponse { code, message }) => {
                eprintln!("git-remote-aspen: server error [{}]: {}", code, message);
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

    /// Handle a batch of push commands.
    ///
    /// Per git-remote-helpers(7), multiple `push` lines arrive in a batch
    /// terminated by a blank line. We resolve all refs, enumerate the union of
    /// all reachable objects, probe once, send objects once, and report per-ref
    /// results followed by a single blank line.
    async fn handle_push_batch<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        batch: &[(String, String, bool)],
    ) -> io::Result<()> {
        let repo_id = self.url.repo_id().to_hex();
        let git_dir = std::env::var("GIT_DIR").unwrap_or_else(|_| ".git".to_string());
        let git_dir = std::path::Path::new(&git_dir);

        if self.options.verbosity > 0 && batch.len() > 1 {
            eprintln!("git-remote-aspen: batch push: {} refs", batch.len());
        }

        // Phase 1: Resolve all refs and build ref updates + collect commit SHA-1s
        let mut ref_updates: Vec<GitBridgeRefUpdate> = Vec::new();
        let mut commit_sha1s: Vec<String> = Vec::new();
        let mut failed_refs: Vec<(String, String)> = Vec::new(); // (dst, error_msg)

        for (src, dst, force) in batch {
            if self.options.verbosity > 0 {
                let force_str = if *force { " (force)" } else { "" };
                eprintln!("git-remote-aspen: pushing {}:{}{}", src, dst, force_str);
            }

            // Empty src = delete ref
            if src.is_empty() {
                let old_sha1 = self.get_remote_ref_value(&repo_id, dst).unwrap_or_default();
                ref_updates.push(GitBridgeRefUpdate {
                    ref_name: dst.to_string(),
                    old_sha1,
                    new_sha1: "0000000000000000000000000000000000000000".to_string(),
                    is_force: *force,
                });
                continue;
            }

            match self.resolve_ref(git_dir, src) {
                Ok(sha) => {
                    if self.options.verbosity > 0 {
                        eprintln!("git-remote-aspen: resolved {} to {}", src, sha);
                    }
                    let old_sha1 = self.get_remote_ref_value(&repo_id, dst).unwrap_or_default();
                    ref_updates.push(GitBridgeRefUpdate {
                        ref_name: dst.to_string(),
                        old_sha1,
                        new_sha1: sha.clone(),
                        is_force: *force,
                    });
                    commit_sha1s.push(sha);
                }
                Err(e) => {
                    eprintln!("git-remote-aspen: failed to resolve {}: {}", src, e);
                    failed_refs.push((dst.to_string(), format!("cannot resolve {}", src)));
                }
            }
        }

        // Report immediately-failed refs
        for (dst, msg) in &failed_refs {
            writer.write_push_error(dst, msg)?;
        }

        // If all refs failed to resolve, we're done
        if ref_updates.is_empty() {
            return writer.write_end();
        }

        // Phase 2: Enumerate union of all reachable SHA-1s across all push targets
        let mut all_sha1s_set = std::collections::HashSet::new();
        let mut all_sha1s = Vec::new();

        for commit_sha1 in &commit_sha1s {
            match self.enumerate_push_sha1s(commit_sha1) {
                Ok(shas) => {
                    for sha in shas {
                        if all_sha1s_set.insert(sha.clone()) {
                            all_sha1s.push(sha);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("git-remote-aspen: failed to enumerate objects for {}: {}", commit_sha1, e);
                    // Non-fatal: other refs can still proceed with their objects
                }
            }
        }

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: enumerated {} unique reachable objects", all_sha1s.len());
        }

        // Phase 3: Probe the server for which objects it already has
        let known_sha1s = if all_sha1s.is_empty() {
            std::collections::HashSet::new()
        } else {
            let client = self.get_client().await?;
            let probe_request = ClientRpcRequest::GitBridgeProbeObjects {
                repo_id: repo_id.clone(),
                sha1s: all_sha1s.clone(),
            };

            match client.send(probe_request).await {
                Ok(ClientRpcResponse::GitBridgeProbeObjects(resp)) if resp.is_success => {
                    let known: std::collections::HashSet<String> = resp.known_sha1s.into_iter().collect();
                    if self.options.verbosity > 0 {
                        eprintln!(
                            "git-remote-aspen: server already has {}/{} objects, sending {} new",
                            known.len(),
                            all_sha1s.len(),
                            all_sha1s.len() - known.len()
                        );
                    }
                    known
                }
                Ok(ClientRpcResponse::GitBridgeProbeObjects(resp)) => {
                    eprintln!(
                        "git-remote-aspen: probe failed ({}), falling back to full push",
                        resp.error.unwrap_or_default()
                    );
                    std::collections::HashSet::new()
                }
                Ok(_) | Err(_) => {
                    eprintln!("git-remote-aspen: probe unavailable, falling back to full push");
                    std::collections::HashSet::new()
                }
            }
        };

        // Phase 4: Read object data only for objects the server doesn't have
        let objects_dir = git_dir.join("objects");
        let objects = match self.collect_objects_for_push(&objects_dir, &all_sha1s, &known_sha1s) {
            Ok(objs) => objs,
            Err(e) => {
                eprintln!("git-remote-aspen: failed to collect objects: {}", e);
                for update in &ref_updates {
                    writer.write_push_error(&update.ref_name, &format!("failed to read objects: {}", e))?;
                }
                return writer.write_end();
            }
        };

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: collected {} objects to push", objects.len());
        }

        // Phase 5: Send objects and ref updates via chunked protocol
        // Fast path: all objects already on server — just update refs
        if objects.is_empty() {
            if self.options.verbosity > 0 {
                eprintln!("git-remote-aspen: all objects already on server, updating refs only");
            }

            let client = self.get_client().await?;
            let request = ClientRpcRequest::GitBridgePush {
                repo_id: repo_id.clone(),
                objects: vec![],
                refs: ref_updates.clone(),
            };

            let response = client.send(request).await?;
            match response {
                ClientRpcResponse::GitBridgePush(resp) => {
                    for result in &resp.ref_results {
                        if result.is_success {
                            writer.write_push_ok(&result.ref_name)?;
                        } else {
                            let msg = result.error.as_deref().unwrap_or("unknown error");
                            writer.write_push_error(&result.ref_name, msg)?;
                        }
                    }
                }
                ClientRpcResponse::Error(ErrorResponse { code, message }) => {
                    eprintln!("git-remote-aspen: server error [{}]: {}", code, message);
                    for update in &ref_updates {
                        writer.write_push_error(&update.ref_name, &format!("[{}] {}", code, message))?;
                    }
                }
                _ => {
                    for update in &ref_updates {
                        writer.write_push_error(&update.ref_name, "unexpected response")?;
                    }
                }
            }

            return writer.write_end();
        }

        // Full push path: batch objects and send via chunked protocol
        self.send_objects_chunked(writer, &repo_id, objects, ref_updates).await
    }

    /// Send objects via the chunked push protocol and report ref results.
    ///
    /// Extracted from handle_push_batch to keep that method under 70 lines per
    /// Tiger Style. Handles object batching, chunked transfer, and result reporting.
    async fn send_objects_chunked<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        repo_id: &str,
        objects: Vec<aspen::client_rpc::GitBridgeObject>,
        ref_updates: Vec<GitBridgeRefUpdate>,
    ) -> io::Result<()> {
        let total_objects = objects.len();
        let verbosity = self.options.verbosity;

        // Reorder objects: blobs → trees (all together) → commits → tags
        let batches = self.build_object_batches(objects);
        let num_batches = batches.len();

        if num_batches > 1 && verbosity > 0 {
            eprintln!("git-remote-aspen: sending {} objects in {} batches", total_objects, num_batches);
        }

        let client = self.get_client().await?;
        let total_size_bytes: u64 =
            batches.iter().flat_map(|batch| batch.iter()).map(|obj| obj.data.len() as u64).sum();

        if verbosity > 0 {
            eprintln!(
                "git-remote-aspen: starting chunked push session: {} objects, {:.2} MB",
                total_objects,
                total_size_bytes as f64 / (1024.0 * 1024.0)
            );
        }

        // Start session
        let start_request = ClientRpcRequest::GitBridgePushStart {
            repo_id: repo_id.to_string(),
            total_objects: total_objects as u64,
            total_size_bytes,
            refs: ref_updates.clone(),
            metadata: None,
        };

        let start_response = client.send(start_request).await?;
        let session_id = match start_response {
            ClientRpcResponse::GitBridgePushStart(ref resp) => {
                if !resp.is_success {
                    let msg = resp.error.as_deref().unwrap_or("failed to start session");
                    for update in &ref_updates {
                        writer.write_push_error(&update.ref_name, msg)?;
                    }
                    return writer.write_end();
                }
                if verbosity > 1 {
                    eprintln!(
                        "git-remote-aspen: session started: {}, max_chunk_size: {} MB",
                        resp.session_id,
                        resp.max_chunk_size_bytes / (1024 * 1024)
                    );
                }
                resp.session_id.clone()
            }
            _ => {
                for update in &ref_updates {
                    writer.write_push_error(&update.ref_name, "unexpected response to GitBridgePushStart")?;
                }
                return writer.write_end();
            }
        };

        // Send chunks
        let mut content_hasher = blake3::Hasher::new();
        for (batch_idx, batch) in batches.into_iter().enumerate() {
            if verbosity > 0 && num_batches > 1 {
                eprintln!(
                    "git-remote-aspen: sending chunk {}/{} ({} objects)",
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );
            }

            let mut chunk_hasher = blake3::Hasher::new();
            for obj in &batch {
                chunk_hasher.update(obj.sha1.as_bytes());
                chunk_hasher.update(obj.object_type.as_bytes());
                chunk_hasher.update(&obj.data);
                content_hasher.update(obj.sha1.as_bytes());
                content_hasher.update(obj.object_type.as_bytes());
                content_hasher.update(&obj.data);
            }
            let chunk_hash = *chunk_hasher.finalize().as_bytes();

            let chunk_request = ClientRpcRequest::GitBridgePushChunk {
                session_id: session_id.clone(),
                chunk_id: batch_idx as u64,
                total_chunks: num_batches as u64,
                objects: batch,
                chunk_hash,
            };

            let chunk_response = client.send(chunk_request).await?;
            match chunk_response {
                ClientRpcResponse::GitBridgePushChunk(ref resp) => {
                    if !resp.is_success {
                        let msg = resp.error.as_deref().unwrap_or("chunk upload failed");
                        for update in &ref_updates {
                            writer.write_push_error(&update.ref_name, msg)?;
                        }
                        return writer.write_end();
                    }
                }
                _ => {
                    for update in &ref_updates {
                        writer.write_push_error(&update.ref_name, "unexpected response to GitBridgePushChunk")?;
                    }
                    return writer.write_end();
                }
            }
        }

        // Complete session
        let content_hash = *content_hasher.finalize().as_bytes();
        let complete_request = ClientRpcRequest::GitBridgePushComplete {
            session_id,
            content_hash,
        };

        let response = client.send(complete_request).await?;
        match response {
            ClientRpcResponse::GitBridgePushComplete(ref resp) => {
                if !resp.is_success {
                    let msg = resp.error.as_deref().unwrap_or("chunked push failed");
                    for update in &ref_updates {
                        writer.write_push_error(&update.ref_name, msg)?;
                    }
                } else {
                    if verbosity > 0 {
                        eprintln!(
                            "git-remote-aspen: push complete: imported {} objects, skipped {}",
                            resp.objects_imported, resp.objects_skipped
                        );
                    }
                    for result in &resp.ref_results {
                        if result.is_success {
                            writer.write_push_ok(&result.ref_name)?;
                        } else {
                            let msg = result.error.as_deref().unwrap_or("unknown error");
                            writer.write_push_error(&result.ref_name, msg)?;
                        }
                    }
                }
            }
            ClientRpcResponse::Error(ErrorResponse { code, message }) => {
                eprintln!("git-remote-aspen: server error [{}]: {}", code, message);
                for update in &ref_updates {
                    writer.write_push_error(&update.ref_name, &format!("[{}] {}", code, message))?;
                }
            }
            _ => {
                for update in &ref_updates {
                    writer.write_push_error(&update.ref_name, "unexpected response")?;
                }
            }
        }

        writer.write_end()
    }

    /// Build ordered object batches: blobs → trees (single batch) → commits → tags.
    ///
    /// Trees are kept in a single batch because they have complex interdependencies
    /// (subdirectories) that the server resolves via topological sorting within a batch.
    fn build_object_batches(
        &self,
        objects: Vec<aspen::client_rpc::GitBridgeObject>,
    ) -> Vec<Vec<aspen::client_rpc::GitBridgeObject>> {
        build_object_batches_inner(objects, self.options.verbosity)
    }
}

/// Build ordered object batches: blobs → trees (single batch) → commits → tags.
///
/// Extracted as a free function for testability. `verbosity` controls stderr output.
fn build_object_batches_inner(
    objects: Vec<aspen::client_rpc::GitBridgeObject>,
    verbosity: u32,
) -> Vec<Vec<aspen::client_rpc::GitBridgeObject>> {
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

    let mut batches: Vec<Vec<aspen::client_rpc::GitBridgeObject>> = Vec::new();

    let add_batched = |objects: Vec<aspen::client_rpc::GitBridgeObject>,
                       batches: &mut Vec<Vec<aspen::client_rpc::GitBridgeObject>>| {
        let mut current_batch: Vec<aspen::client_rpc::GitBridgeObject> = Vec::new();
        let mut current_batch_bytes = 0usize;

        for obj in objects {
            let obj_size = obj.sha1.len() + obj.object_type.len() + obj.data.len() + 32;
            if (current_batch_bytes + obj_size > MAX_BATCH_BYTES || current_batch.len() >= MAX_BATCH_OBJECTS)
                && !current_batch.is_empty()
            {
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

    if verbosity > 0 && !blobs.is_empty() {
        eprintln!("git-remote-aspen: batching {} blobs", blobs.len());
    }
    add_batched(blobs, &mut batches);

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

    if verbosity > 0 && !commits.is_empty() {
        eprintln!("git-remote-aspen: batching {} commits", commits.len());
    }
    add_batched(commits, &mut batches);

    if !tags.is_empty() {
        add_batched(tags, &mut batches);
    }
    if !other.is_empty() {
        add_batched(other, &mut batches);
    }

    batches
}

impl RemoteHelper {
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

    /// Enumerate all SHA-1 hashes reachable from a commit using `git rev-list --objects`.
    ///
    /// Returns just the SHA-1 strings without reading object data. This is fast
    /// (~100ms for 14K objects) and used as input to the incremental push probe.
    ///
    /// Tiger Style: Bounds checking prevents memory exhaustion (CRIT-002)
    fn enumerate_push_sha1s(&self, commit_sha1: &str) -> io::Result<Vec<String>> {
        let output = std::process::Command::new("git")
            .args(["rev-list", "--objects", "--reverse", commit_sha1])
            .output()
            .map_err(|e| io::Error::other(format!("failed to run git rev-list: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::other(format!("git rev-list failed: {}", stderr.trim())));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut sha1s = Vec::new();
        let mut count = 0u32;

        for line in stdout.lines() {
            count += 1;
            if count > MAX_GIT_OBJECTS_PER_PUSH {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("exceeded maximum objects per push ({})", MAX_GIT_OBJECTS_PER_PUSH),
                ));
            }

            let sha1 = line.split_whitespace().next().unwrap_or("");
            if sha1.len() == 40 {
                sha1s.push(sha1.to_string());
            }
        }

        Ok(sha1s)
    }

    /// Collect git objects for push, reading only objects whose SHA-1 is NOT in `skip_sha1s`.
    ///
    /// This is the incremental push optimization: after probing the server for known
    /// objects, we only read data for the missing ones. For a typical push after
    /// initial import, this reduces from ~14K objects to ~10-50 objects.
    ///
    /// Tiger Style: Bounds checking prevents memory exhaustion (CRIT-002)
    fn collect_objects_for_push(
        &self,
        objects_dir: &std::path::Path,
        sha1s: &[String],
        skip_sha1s: &std::collections::HashSet<String>,
    ) -> io::Result<Vec<aspen::client_rpc::GitBridgeObject>> {
        let mut objects = Vec::new();

        for sha1 in sha1s {
            if skip_sha1s.contains(sha1) {
                continue;
            }

            match self.read_git_object(objects_dir, sha1) {
                Ok((object_type, data)) => {
                    objects.push(aspen::client_rpc::GitBridgeObject {
                        sha1: sha1.clone(),
                        object_type,
                        data,
                    });
                }
                Err(e) => {
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

        #[allow(clippy::collapsible_if)]
        if size_output.status.success() {
            if let Ok(size_str) = std::str::from_utf8(&size_output.stdout) {
                if let Ok(size) = size_str.trim().parse::<u64>() {
                    if size > MAX_GIT_OBJECT_SIZE {
                        return Err(io::Error::other(format!(
                            "git object {} exceeds maximum size ({} bytes)",
                            sha1, MAX_GIT_OBJECT_SIZE
                        )));
                    }
                }
            }
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

/// Initialize tracing subscriber with environment-based filtering.
///
/// Suppresses noisy warnings from network-related crates that produce
/// spurious warnings due to kernel/crate version mismatches.
fn init_tracing() {
    // Suppress noisy warnings from network-related crates:
    // - portmapper.service: kernel has newer IFLA_INET6_CONF NLA attributes (240 vs 236 bytes)
    // - netlink_packet_route: related netlink attribute size mismatches
    // - netlink_packet_core: related netlink core warnings
    // - quinn_udp: IPv6 unreachable errors when IPv6 is not available
    const NOISY_CRATES: &str = ",netlink_packet_route=error,quinn_udp=error,netlink_packet_core=error,portmapper=error";

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(format!("warn{NOISY_CRATES}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> io::Result<()> {
    init_tracing();

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a GitBridgeObject for testing.
    fn make_obj(sha1: &str, obj_type: &str, data_size: usize) -> aspen::client_rpc::GitBridgeObject {
        aspen::client_rpc::GitBridgeObject {
            sha1: sha1.to_string(),
            object_type: obj_type.to_string(),
            data: vec![0u8; data_size],
        }
    }

    // ========================================================================
    // Object batching tests
    // ========================================================================

    #[test]
    fn test_batch_type_ordering() {
        // Objects arrive in random order — batches should be: blobs, trees, commits, tags
        let objects = vec![
            make_obj("c1", "commit", 100),
            make_obj("b1", "blob", 100),
            make_obj("t1", "tag", 100),
            make_obj("tr1", "tree", 100),
        ];

        let batches = build_object_batches_inner(objects, 0);

        // Collect types per batch
        let batch_types: Vec<Vec<&str>> =
            batches.iter().map(|batch| batch.iter().map(|o| o.object_type.as_str()).collect()).collect();

        // Find first occurrence of each type
        let mut type_order = Vec::new();
        for types in &batch_types {
            for t in types {
                if !type_order.contains(t) {
                    type_order.push(*t);
                }
            }
        }

        assert_eq!(
            type_order,
            vec!["blob", "tree", "commit", "tag"],
            "type order should be blob → tree → commit → tag"
        );
    }

    #[test]
    fn test_batch_empty_objects() {
        let batches = build_object_batches_inner(vec![], 0);
        assert!(batches.is_empty(), "empty input should produce no batches");
    }

    #[test]
    fn test_batch_trees_single_batch() {
        // All trees should go in one batch regardless of count
        let objects: Vec<_> = (0..100).map(|i| make_obj(&format!("t{}", i), "tree", 100)).collect();
        let batches = build_object_batches_inner(objects, 0);

        // Find the tree batch
        let tree_batch = batches.iter().find(|b| b[0].object_type == "tree");
        assert!(tree_batch.is_some(), "should have a tree batch");
        assert_eq!(tree_batch.unwrap().len(), 100, "all trees should be in one batch");
    }

    #[test]
    fn test_batch_blobs_split_by_size() {
        // Create blobs that exceed MAX_BATCH_BYTES
        // MAX_BATCH_BYTES is 4MB. Each blob has ~1MB data + overhead.
        let objects: Vec<_> = (0..10).map(|i| make_obj(&format!("b{}", i), "blob", 1_000_000)).collect();

        let batches = build_object_batches_inner(objects, 0);

        // With 10 blobs of 1MB each, should need at least 3 batches (4MB limit)
        assert!(batches.len() >= 3, "10x1MB blobs should split into at least 3 batches, got {}", batches.len());

        // All batches should contain only blobs
        for batch in &batches {
            for obj in batch {
                assert_eq!(obj.object_type, "blob");
            }
        }
    }

    #[test]
    fn test_batch_blobs_split_by_count() {
        // MAX_BATCH_OBJECTS is 2000. Create more than that.
        let objects: Vec<_> = (0..2500).map(|i| make_obj(&format!("b{}", i), "blob", 10)).collect();

        let batches = build_object_batches_inner(objects, 0);

        // Should split into 2 batches (2000 + 500)
        assert_eq!(batches.len(), 2, "2500 small blobs should split into 2 batches");
        assert_eq!(batches[0].len(), 2000);
        assert_eq!(batches[1].len(), 500);
    }

    #[test]
    fn test_batch_mixed_types() {
        let objects = vec![
            make_obj("b1", "blob", 100),
            make_obj("b2", "blob", 100),
            make_obj("t1", "tree", 100),
            make_obj("t2", "tree", 100),
            make_obj("c1", "commit", 100),
            make_obj("tag1", "tag", 100),
        ];

        let batches = build_object_batches_inner(objects, 0);

        // Should have separate batches for blobs, trees, commits, tags
        // but small objects may combine within limits
        let total: usize = batches.iter().map(|b| b.len()).sum();
        assert_eq!(total, 6, "all 6 objects should be batched");

        // Verify ordering: all blobs before trees before commits before tags
        let mut all_types: Vec<&str> = Vec::new();
        for batch in &batches {
            for obj in batch {
                all_types.push(&obj.object_type);
            }
        }
        let blob_last = all_types.iter().rposition(|&t| t == "blob").unwrap_or(0);
        let tree_first = all_types.iter().position(|&t| t == "tree").unwrap_or(usize::MAX);
        let tree_last = all_types.iter().rposition(|&t| t == "tree").unwrap_or(0);
        let commit_first = all_types.iter().position(|&t| t == "commit").unwrap_or(usize::MAX);
        let commit_last = all_types.iter().rposition(|&t| t == "commit").unwrap_or(0);
        let tag_first = all_types.iter().position(|&t| t == "tag").unwrap_or(usize::MAX);

        assert!(blob_last < tree_first, "all blobs should come before all trees");
        assert!(tree_last < commit_first, "all trees should come before all commits");
        assert!(commit_last < tag_first, "all commits should come before all tags");
    }

    #[test]
    fn test_batch_only_commits() {
        let objects = vec![
            make_obj("c1", "commit", 500),
            make_obj("c2", "commit", 500),
            make_obj("c3", "commit", 500),
        ];

        let batches = build_object_batches_inner(objects, 0);
        assert_eq!(batches.len(), 1, "3 small commits should fit in 1 batch");
        assert_eq!(batches[0].len(), 3);
        for obj in &batches[0] {
            assert_eq!(obj.object_type, "commit");
        }
    }

    #[test]
    fn test_batch_only_tags() {
        let objects = vec![make_obj("tag1", "tag", 200), make_obj("tag2", "tag", 200)];

        let batches = build_object_batches_inner(objects, 0);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }

    #[test]
    fn test_batch_preserves_sha1s() {
        let objects = vec![
            make_obj("aaa", "blob", 50),
            make_obj("bbb", "tree", 50),
            make_obj("ccc", "commit", 50),
        ];

        let batches = build_object_batches_inner(objects, 0);
        let all_sha1s: Vec<&str> = batches.iter().flat_map(|b| b.iter().map(|o| o.sha1.as_str())).collect();

        assert!(all_sha1s.contains(&"aaa"));
        assert!(all_sha1s.contains(&"bbb"));
        assert!(all_sha1s.contains(&"ccc"));
    }
}
