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

use std::io::{self, Write};
use std::time::Duration;

use aspen::client_rpc::{
    ClientRpcRequest, ClientRpcResponse, GitBridgeFetchResponse, GitBridgeListRefsResponse,
    GitBridgePushResponse, GitBridgeRefUpdate,
};
use aspen::cluster::ticket::AspenClusterTicket;
use protocol::{Command, ProtocolReader, ProtocolWriter};
use url::{AspenUrl, ConnectionTarget};

/// RPC timeout for git bridge operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(60);

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
            .alpns(vec![aspen::protocol_handlers::CLIENT_ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self { endpoint, ticket })
    }

    /// Send an RPC request.
    async fn send(&self, request: ClientRpcRequest) -> io::Result<ClientRpcResponse> {
        use aspen::client_rpc::{AuthenticatedRequest, MAX_CLIENT_MESSAGE_SIZE};
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
        let connection = timeout(RPC_TIMEOUT, async {
            self.endpoint
                .connect(target_addr, aspen::protocol_handlers::CLIENT_ALPN)
                .await
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connection timeout"))?
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;

        // Open stream
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Send request (unauthenticated for now)
        let auth_request = AuthenticatedRequest::unauthenticated(request);
        let request_bytes = postcard::to_stdvec(&auth_request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        send.write_all(&request_bytes)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        send.finish()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Read response
        let response_bytes = timeout(RPC_TIMEOUT, async {
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response timeout"))?
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Deserialize
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

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
        let url = AspenUrl::parse(url_str).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid URL: {e}"))
        })?;

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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "cluster ticket has no bootstrap peers",
                ));
            }

            let client = RpcClient::connect(ticket).await?;
            self.client = Some(client);
        }

        Ok(self.client.as_ref().unwrap())
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
    async fn handle_list<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        _for_push: bool,
    ) -> io::Result<()> {
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

        // Get client and send RPC
        let client = self.get_client().await?;
        let request = ClientRpcRequest::GitBridgeFetch {
            repo_id,
            want: vec![sha1.to_string()],
            have: vec![], // TODO: Get "have" list from git
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

                // Write objects to git (via stdout or sideband)
                // For now, we'd need to use git-fast-import or write a packfile
                // This is a placeholder - actual implementation would pipe objects to git
                if self.options.verbosity > 0 {
                    eprintln!(
                        "git-remote-aspen: received {} objects",
                        objects.len()
                    );
                }

                // TODO: Actually write objects to git's object store
                // This would use git-fast-import or direct object writing

                writer.write_fetch_done()
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_fetch_done()
            }
        }
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

        // TODO: Read objects from local git repo
        // This would involve:
        // 1. Resolve src to a commit SHA-1
        // 2. Walk the commit graph to find objects to send
        // 3. Read object data from local .git/objects

        // For now, send a placeholder push
        let client = self.get_client().await?;
        let request = ClientRpcRequest::GitBridgePush {
            repo_id,
            objects: vec![], // TODO: Populate with actual objects
            refs: vec![GitBridgeRefUpdate {
                ref_name: dst.to_string(),
                old_sha1: String::new(), // TODO: Get current remote ref value
                new_sha1: String::new(), // TODO: Resolve src to SHA-1
                force,
            }],
        };

        let response = client.send(request).await?;

        match response {
            ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                success,
                objects_imported: _,
                objects_skipped: _,
                ref_results,
                error,
            }) => {
                if !success {
                    let msg = error.unwrap_or_else(|| "unknown error".to_string());
                    return writer.write_push_error(dst, &msg);
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

                Ok(())
            }
            _ => {
                eprintln!("git-remote-aspen: unexpected response type");
                writer.write_push_error(dst, "unexpected response")
            }
        }
    }

    /// Handle the "option" command.
    fn handle_option<W: Write>(
        &mut self,
        writer: &mut ProtocolWriter<W>,
        name: &str,
        value: &str,
    ) -> io::Result<()> {
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
