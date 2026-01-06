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
use protocol::Command;
use protocol::ProtocolReader;
use protocol::ProtocolWriter;
use url::AspenUrl;
use url::ConnectionTarget;

/// RPC timeout for git bridge operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of retry attempts for RPC calls.
const MAX_RETRIES: u32 = 3;

/// Delay between retry attempts.
const RETRY_DELAY: Duration = Duration::from_millis(500);

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
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![aspen::CLIENT_ALPN.to_vec()])
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
        use aspen::client_rpc::GitBridgeObject;

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

        // Collect all objects reachable from the commit
        let objects_dir = git_dir.join("objects");
        let mut objects: Vec<GitBridgeObject> = Vec::new();
        let mut visited = std::collections::HashSet::new();

        if let Err(e) = self.collect_objects(&objects_dir, &commit_sha1, &mut objects, &mut visited) {
            eprintln!("git-remote-aspen: failed to collect objects: {}", e);
            return writer.write_push_error(dst, &format!("failed to read objects: {}", e));
        }

        if self.options.verbosity > 0 {
            eprintln!("git-remote-aspen: collected {} objects to push", objects.len());
        }

        // Get remote ref value before mutable borrow of client
        let old_sha1 = self.get_remote_ref_value(&repo_id, dst).unwrap_or_default();

        // Send the push request
        let client = self.get_client().await?;
        let request = ClientRpcRequest::GitBridgePush {
            repo_id,
            objects,
            refs: vec![GitBridgeRefUpdate {
                ref_name: dst.to_string(),
                old_sha1,
                new_sha1: commit_sha1,
                force,
            }],
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
                if !success {
                    let msg = error.unwrap_or_else(|| "unknown error".to_string());
                    return writer.write_push_error(dst, &msg);
                }

                if self.options.verbosity > 0 {
                    eprintln!("git-remote-aspen: imported {} objects, skipped {}", objects_imported, objects_skipped);
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

                // Terminal empty line to signal end of push response batch
                writer.write_end()
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_push_error(dst, "unexpected response")
            }
        }
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

    /// Collect all objects reachable from a commit.
    fn collect_objects(
        &self,
        objects_dir: &std::path::Path,
        sha1: &str,
        objects: &mut Vec<aspen::client_rpc::GitBridgeObject>,
        visited: &mut std::collections::HashSet<String>,
    ) -> io::Result<()> {
        if visited.contains(sha1) {
            return Ok(());
        }
        visited.insert(sha1.to_string());

        // Read the object
        let (object_type, data) = self.read_loose_object(objects_dir, sha1)?;

        // Recursively collect referenced objects
        match object_type.as_str() {
            "commit" => {
                // Parse commit to find tree and parents
                if let Ok(content) = std::str::from_utf8(&data) {
                    for line in content.lines() {
                        if let Some(tree_sha) = line.strip_prefix("tree ") {
                            self.collect_objects(objects_dir, tree_sha, objects, visited)?;
                        } else if let Some(parent_sha) = line.strip_prefix("parent ") {
                            self.collect_objects(objects_dir, parent_sha, objects, visited)?;
                        } else if line.is_empty() {
                            break; // End of headers
                        }
                    }
                }
            }
            "tree" => {
                // Parse tree entries
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
                    self.collect_objects(objects_dir, &entry_sha, objects, visited)?;
                    pos = null + 21;
                }
            }
            "tag" => {
                // Parse tag to find object
                if let Ok(content) = std::str::from_utf8(&data) {
                    for line in content.lines() {
                        if let Some(target_sha) = line.strip_prefix("object ") {
                            self.collect_objects(objects_dir, target_sha, objects, visited)?;
                            break;
                        }
                    }
                }
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

    /// Read a loose object from git's object store.
    fn read_loose_object(&self, objects_dir: &std::path::Path, sha1: &str) -> io::Result<(String, Vec<u8>)> {
        use std::io::Read;

        use flate2::read::ZlibDecoder;

        let path = objects_dir.join(&sha1[0..2]).join(&sha1[2..]);
        let compressed = std::fs::read(&path)?;

        // Decompress
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Parse header: "{type} {size}\0{content}"
        let null_pos = decompressed
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid object header"))?;

        let header = std::str::from_utf8(&decompressed[..null_pos])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid object header"))?;

        let space_pos = header
            .find(' ')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid object header"))?;

        let object_type = &header[..space_pos];
        let data = decompressed[null_pos + 1..].to_vec();

        Ok((object_type.to_string(), data))
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
