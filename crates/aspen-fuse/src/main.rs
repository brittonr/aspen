//! Aspen FUSE filesystem daemon.
//!
//! Mounts an Aspen KV cluster as a POSIX filesystem.
//!
//! # Usage
//!
//! ```bash
//! # Mount Aspen cluster at /mnt/aspen
//! aspen-fuse --mount-point /mnt/aspen --ticket <cluster-ticket>
//!
//! # Run in foreground (for debugging)
//! aspen-fuse --mount-point /mnt/aspen --ticket <cluster-ticket> -f
//!
//! # VirtioFS mode for VM guests
//! aspen-fuse --virtiofs --socket /tmp/aspen.sock --ticket <cluster-ticket>
//! ```
//!
//! # Key Mapping
//!
//! Filesystem paths map directly to KV keys:
//! - `/myapp/config/db` -> KV key `myapp/config/db`
//! - Directories are virtual (derived from key prefixes)
//!
//! # Tiger Style
//!
//! - Explicit resource bounds (see constants.rs)
//! - Fail-fast on configuration errors
//! - Structured error handling

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use aspen_fuse::AspenFs;
use aspen_fuse::FuseSyncClient;
use clap::Parser;
use fuse_backend_rs::api::server::Server;
use fuse_backend_rs::transport::FuseChannel;
use fuse_backend_rs::transport::FuseSession;
use fuse_backend_rs::transport::Writer;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;

/// Number of threads for FUSE request handling.
const FUSE_THREADS: usize = 4;

#[derive(Parser, Debug)]
#[command(name = "aspen-fuse")]
#[command(about = "Mount Aspen KV cluster as a POSIX filesystem")]
struct Args {
    /// Mount point path.
    #[arg(long)]
    mount_point: Option<PathBuf>,

    /// Cluster ticket for Aspen connection.
    #[arg(long)]
    ticket: String,

    /// Run in foreground (don't daemonize).
    #[arg(long, short)]
    foreground: bool,

    /// Enable VirtioFS mode (for VM guests).
    #[cfg(feature = "virtiofs")]
    #[arg(long)]
    virtiofs: bool,

    /// VHost-user socket path (for VirtioFS mode).
    #[cfg(feature = "virtiofs")]
    #[arg(long)]
    socket: Option<PathBuf>,

    /// Filesystem name for mount.
    #[arg(long, default_value = "aspen")]
    fsname: String,

    /// Allow other users to access the mount.
    #[arg(long)]
    allow_other: bool,

    /// Number of threads for handling FUSE requests.
    #[arg(long, default_value_t = FUSE_THREADS)]
    threads: usize,
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

fn main() {
    init_tracing();

    let args = Args::parse();

    // Validate arguments
    #[cfg(feature = "virtiofs")]
    {
        if args.virtiofs {
            if args.socket.is_none() {
                error!("--socket is required for VirtioFS mode");
                std::process::exit(1);
            }
            if args.mount_point.is_some() {
                error!("--mount-point is not used in VirtioFS mode");
                std::process::exit(1);
            }
        } else if args.mount_point.is_none() {
            error!("--mount-point is required for FUSE mode");
            std::process::exit(1);
        }
    }

    #[cfg(not(feature = "virtiofs"))]
    if args.mount_point.is_none() {
        error!("--mount-point is required");
        std::process::exit(1);
    }

    info!(
        ticket = %args.ticket,
        "connecting to Aspen cluster"
    );

    // Connect to Aspen cluster using ticket
    let client = match FuseSyncClient::from_ticket(&args.ticket) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "failed to connect to Aspen cluster");
            std::process::exit(1);
        }
    };

    info!("connected to Aspen cluster");

    // Get current user info
    // SAFETY: getuid() and getgid() are POSIX syscalls that return the real
    // user/group ID. They have no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    // Create filesystem with connected client
    let fs = AspenFs::new(uid, gid, client);

    #[cfg(feature = "virtiofs")]
    if args.virtiofs {
        run_virtiofs(args, fs);
        return;
    }

    run_fuse(args, fs);
}

/// FUSE server wrapper for multi-threaded request handling.
struct FuseServer {
    server: Arc<Server<AspenFs>>,
    channel: FuseChannel,
}

impl FuseServer {
    /// Create a new FUSE server from session.
    fn new(server: Arc<Server<AspenFs>>, session: &mut FuseSession) -> Result<Self, fuse_backend_rs::transport::Error> {
        let channel = session.new_channel()?;
        Ok(Self { server, channel })
    }

    /// Run the FUSE service loop.
    ///
    /// Processes incoming FUSE requests until the kernel shuts down the session.
    fn service_loop(&mut self) {
        loop {
            // Get the next request from the kernel
            match self.channel.get_request() {
                Ok(Some((reader, fuse_writer))) => {
                    // Wrap FuseDevWriter in Writer enum for the server
                    let writer = Writer::FuseDev(fuse_writer);

                    // Handle the request
                    if let Err(e) = self.server.handle_message(reader, writer, None, None) {
                        // Check if kernel shut down the session
                        if let fuse_backend_rs::Error::EncodeMessage(ref io_err) = e
                            && io_err.raw_os_error() == Some(libc::EBADF)
                        {
                            info!("FUSE session closed by kernel");
                            break;
                        }
                        warn!(error = ?e, "error handling FUSE message");
                    }
                }
                Ok(None) => {
                    // No more requests, session closed
                    info!("FUSE session ended");
                    break;
                }
                Err(e) => {
                    // Check for expected shutdown errors (session closed)
                    let is_shutdown = matches!(
                        &e,
                        fuse_backend_rs::transport::Error::SessionFailure(msg)
                            if msg.contains("closed") || msg.contains("shutdown")
                    );
                    if is_shutdown {
                        info!("FUSE session closed");
                        break;
                    }
                    warn!(error = ?e, "error getting FUSE request");
                }
            }
        }
    }
}

fn run_fuse(args: Args, fs: AspenFs) {
    let mount_point = args.mount_point.expect("mount_point required for FUSE mode");

    info!(
        mount_point = %mount_point.display(),
        fsname = %args.fsname,
        threads = args.threads,
        "starting FUSE server"
    );

    // Create FUSE server
    let server = Arc::new(Server::new(fs));

    // Create FUSE session
    let mut session = match FuseSession::new(&mount_point, &args.fsname, "", false) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to create FUSE session");
            std::process::exit(1);
        }
    };

    // Mount the filesystem
    if let Err(e) = session.mount() {
        error!(error = %e, "failed to mount filesystem");
        std::process::exit(1);
    }

    info!("filesystem mounted at {}", mount_point.display());

    // Spawn worker threads for handling FUSE requests
    let mut handles = Vec::with_capacity(args.threads);

    for i in 0..args.threads {
        let fuse_server = match FuseServer::new(server.clone(), &mut session) {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, thread = i, "failed to create FUSE channel");
                continue;
            }
        };

        let handle = thread::Builder::new()
            .name(format!("fuse-worker-{}", i))
            .spawn(move || {
                info!(thread = i, "FUSE worker started");
                let mut server = fuse_server;
                server.service_loop();
                info!(thread = i, "FUSE worker stopped");
            })
            .expect("failed to spawn FUSE worker thread");

        handles.push(handle);
    }

    info!(threads = handles.len(), "FUSE workers started, press Ctrl-C to unmount");

    // Wait for shutdown signal
    let (tx, rx) = std::sync::mpsc::channel();
    if let Err(err) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        error!("failed to set Ctrl-C handler: {}", err);
    }

    let _ = rx.recv();

    info!("shutdown signal received, unmounting filesystem");

    // Unmount will cause workers to exit
    if let Err(e) = session.umount() {
        error!(error = %e, "failed to unmount");
    }

    // Wait for all workers to finish
    for handle in handles {
        let _ = handle.join();
    }

    info!("shutdown complete");
}

#[cfg(feature = "virtiofs")]
fn run_virtiofs(args: Args, fs: AspenFs) {
    use std::os::unix::net::UnixListener;
    use std::sync::Arc;

    use fuse_backend_rs::api::server::Server;

    let socket_path = args.socket.expect("socket required for VirtioFS mode");

    info!(
        socket = %socket_path.display(),
        "starting VirtioFS vhost-user server"
    );

    // Remove existing socket if it exists
    let _ = std::fs::remove_file(&socket_path);

    // Create the FUSE server with our filesystem
    let server = Server::new(Arc::new(fs));

    // Create a VirtioFS vhost-user backend
    // Note: Full vhost-user backend implementation for VirtioFS is complex and requires:
    // 1. Implementing the vhost-user protocol handshake
    // 2. Setting up virtio queues for request/response
    // 3. Bridging FUSE operations to virtio-fs protocol
    // 4. Memory mapping for zero-copy operations
    // 5. Thread management for queue processing
    //
    // This is a substantial implementation that would benefit from using
    // existing solutions like virtiofsd or cloud-hypervisor's vhost-user-fs
    // backend as a reference.

    // Create Unix socket listener
    let listener = UnixListener::bind(&socket_path).expect("failed to bind vhost-user socket");

    info!("VirtioFS vhost-user socket created, waiting for connection");
    info!("Note: Full vhost-user backend implementation pending");

    // Accept connection
    let (stream, _) = listener.accept().expect("failed to accept vhost-user connection");

    info!("vhost-user client connected");

    // Minimal vhost-user handler that responds to basic protocol messages
    // but doesn't fully implement VirtioFS functionality yet
    let mut handler = VhostUserHandler::new(server);

    // This would need a full event loop implementation to handle
    // vhost-user protocol messages and virtio-fs requests
    handler.handle_connection(stream);
}

#[cfg(feature = "virtiofs")]
struct VhostUserHandler {
    server: Server<Arc<AspenFs>>,
}

#[cfg(feature = "virtiofs")]
impl VhostUserHandler {
    fn new(server: Server<Arc<AspenFs>>) -> Self {
        Self { server }
    }

    fn handle_connection(&mut self, stream: std::os::unix::net::UnixStream) {
        // Basic vhost-user protocol handling
        // This is a placeholder - full implementation would:
        // 1. Handle GET_FEATURES, SET_FEATURES
        // 2. Setup memory regions with SET_MEM_TABLE
        // 3. Configure virtio queues with SET_VRING_*
        // 4. Process VirtioFS requests from queues
        // 5. Send responses back through virtio queues

        info!("VirtioFS vhost-user handler started (minimal implementation)");
        info!("Full VirtioFS backend implementation is a TODO");

        // For now, just log that we're connected
        // A full implementation would require significant additional code
        // to properly handle the vhost-user protocol and virtio-fs operations

        drop(stream);
        error!("VirtioFS vhost-user backend not fully implemented");
        error!("This requires substantial additional work to bridge FUSE and virtio-fs");
        std::process::exit(1);
    }
}
