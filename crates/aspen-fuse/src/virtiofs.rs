//! VirtioFS backend for Aspen FUSE filesystem.
//!
//! Implements the vhost-user-fs protocol to expose Aspen KV as a virtio-fs
//! device for cloud-hypervisor, QEMU, and other VMMs.
//!
//! # Architecture
//!
//! ```text
//! cloud-hypervisor / QEMU
//!        │
//!        │ vhost-user protocol (Unix socket)
//!        ▼
//! AspenVirtioFsBackend (VhostUserBackendMut)
//!        │
//!        │ virtio queues → descriptor chains
//!        ▼
//! fuse-backend-rs Server<AspenFs>
//!        │
//!        │ FUSE protocol
//!        ▼
//! AspenFs (FileSystem trait)
//!        │
//!        │ Iroh Client RPC
//!        ▼
//! Aspen Cluster (Raft consensus)
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Start VirtioFS daemon
//! aspen-fuse --virtiofs --socket /run/virtiofsd/aspen.sock --ticket <ticket>
//!
//! # cloud-hypervisor connects with:
//! cloud-hypervisor --memory shared=on --fs tag=aspen,socket=/run/virtiofsd/aspen.sock
//!
//! # Inside guest:
//! mount -t virtiofs aspen /mnt/aspen
//! ```
//!
//! # Tiger Style
//!
//! - Explicit resource bounds (queue sizes, thread counts)
//! - Fail-fast on protocol errors
//! - Graceful shutdown via kill eventfd

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use fuse_backend_rs::api::server::Server;
use fuse_backend_rs::transport::Reader;
use fuse_backend_rs::transport::VirtioFsWriter;
use tracing::debug;
use tracing::info;
use tracing::warn;
use vhost::vhost_user::Backend;
use vhost::vhost_user::message::VhostUserProtocolFeatures;
use vhost::vhost_user::message::VhostUserVirtioFeatures;
use vhost_user_backend::VhostUserBackendMut;
use vhost_user_backend::VhostUserDaemon;
use vhost_user_backend::VringMutex;
use vhost_user_backend::VringState;
use vhost_user_backend::VringT;
use virtio_queue::QueueOwnedT;
use vm_memory::GuestAddressSpace;
use vm_memory::GuestMemoryAtomic;
use vm_memory::GuestMemoryMmap;
use vmm_sys_util::epoll::EventSet;
use vmm_sys_util::eventfd::EventFd;

use crate::AspenFs;

// Virtio feature flags
const VIRTIO_F_VERSION_1: u32 = 32;
const VIRTIO_RING_F_INDIRECT_DESC: u32 = 28;
const VIRTIO_RING_F_EVENT_IDX: u32 = 29;

/// Maximum queue size (number of descriptors per queue).
/// Matches cloud-hypervisor default.
const QUEUE_SIZE: usize = 1024;

/// Number of virtio queues: hiprio (0) + request (1).
const NUM_QUEUES: usize = 2;

/// High-priority queue event index.
const HIPRIO_QUEUE_EVENT: u16 = 0;

/// Request queue event index.
const REQ_QUEUE_EVENT: u16 = 1;

/// VirtioFS backend error.
#[derive(Debug)]
pub enum VirtioFsError {
    /// Guest memory not configured.
    MemoryNotConfigured,
    /// Failed to iterate virtqueue.
    QueueIterError,
    /// Invalid descriptor chain.
    InvalidDescriptorChain(String),
    /// Failed to process FUSE message.
    ProcessMessageError(io::Error),
    /// Unknown device event.
    UnknownDeviceEvent(u16),
    /// Event is not EPOLLIN.
    NotEpollIn,
    /// Failed to create EventFd.
    EventFdError(io::Error),
    /// Failed to create listener.
    ListenerError(String),
    /// Failed to start daemon.
    DaemonError(String),
}

impl std::fmt::Display for VirtioFsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemoryNotConfigured => write!(f, "guest memory not configured"),
            Self::QueueIterError => write!(f, "failed to iterate virtqueue"),
            Self::InvalidDescriptorChain(e) => write!(f, "invalid descriptor chain: {e}"),
            Self::ProcessMessageError(e) => write!(f, "failed to process FUSE message: {e}"),
            Self::UnknownDeviceEvent(e) => write!(f, "unknown device event: {e}"),
            Self::NotEpollIn => write!(f, "event is not EPOLLIN"),
            Self::EventFdError(e) => write!(f, "eventfd error: {e}"),
            Self::ListenerError(e) => write!(f, "listener error: {e}"),
            Self::DaemonError(e) => write!(f, "daemon error: {e}"),
        }
    }
}

impl std::error::Error for VirtioFsError {}

impl From<VirtioFsError> for io::Error {
    fn from(e: VirtioFsError) -> Self {
        io::Error::other(e.to_string())
    }
}

/// VirtioFS backend state.
///
/// Holds the FUSE server, guest memory, and event handling state.
struct VirtioFsBackend {
    /// Whether EVENT_IDX is enabled for notification suppression.
    event_idx: bool,
    /// EventFd for daemon shutdown signaling.
    kill_evt: EventFd,
    /// Guest memory for reading/writing virtqueue descriptors.
    mem: Option<GuestMemoryAtomic<GuestMemoryMmap>>,
    /// FUSE protocol server wrapping AspenFs.
    server: Arc<Server<AspenFs>>,
    /// Optional backend request handler for DAX/cache operations.
    vu_req: Option<Backend>,
}

impl VirtioFsBackend {
    /// Create a new VirtioFS backend with the given filesystem.
    fn new(fs: AspenFs) -> Result<Self, VirtioFsError> {
        let kill_evt = EventFd::new(libc::EFD_NONBLOCK).map_err(VirtioFsError::EventFdError)?;

        Ok(Self {
            event_idx: false,
            kill_evt,
            mem: None,
            server: Arc::new(Server::new(fs)),
            vu_req: None,
        })
    }

    /// Process all available requests in the virtqueue.
    ///
    /// Reads descriptor chains, processes FUSE messages through the server,
    /// and returns results to the guest.
    fn process_queue(&mut self, vring_state: &mut std::sync::MutexGuard<'_, VringState>) -> io::Result<bool> {
        let mut used_any = false;

        let guest_mem = match &self.mem {
            Some(m) => m,
            None => return Err(VirtioFsError::MemoryNotConfigured.into()),
        };

        // Collect available descriptor chains
        let avail_chains: Vec<_> = vring_state
            .get_queue_mut()
            .iter(guest_mem.memory())
            .map_err(|e| {
                warn!("virtio queue iteration failed: {e:?}");
                VirtioFsError::QueueIterError
            })?
            .collect();

        for chain in avail_chains {
            used_any = true;

            let head_index = chain.head_index();
            let mem = guest_mem.memory();

            // Create reader/writer from descriptor chain
            let reader = Reader::from_descriptor_chain(&*mem, chain.clone())
                .map_err(|e| VirtioFsError::InvalidDescriptorChain(format!("{e:?}")))?;
            let writer = VirtioFsWriter::new(&*mem, chain.clone())
                .map(|w| w.into())
                .map_err(|e| VirtioFsError::InvalidDescriptorChain(format!("{e:?}")))?;

            // Process FUSE message through the server
            // This calls into AspenFs for the actual filesystem operations
            // Coerce Backend to dyn FsCacheReqHandler trait object
            let vu_req_handler =
                self.vu_req.as_mut().map(|b| b as &mut dyn fuse_backend_rs::transport::FsCacheReqHandler);
            if let Err(e) = self.server.handle_message(reader, writer, vu_req_handler, None) {
                // Log error but continue processing other requests
                warn!(error = %e, "failed to process FUSE message");
            }

            // Return descriptor to guest
            if self.event_idx {
                if vring_state.add_used(head_index, 0).is_err() {
                    warn!("failed to return used descriptor to ring");
                }

                // Only signal if guest needs notification
                match vring_state.needs_notification() {
                    Err(_) => {
                        warn!("failed to check notification need");
                        let _ = vring_state.signal_used_queue();
                    }
                    Ok(true) => {
                        let _ = vring_state.signal_used_queue();
                    }
                    Ok(false) => {}
                }
            } else {
                if vring_state.add_used(head_index, 0).is_err() {
                    warn!("failed to return used descriptor to ring");
                }
                let _ = vring_state.signal_used_queue();
            }
        }

        Ok(used_any)
    }
}

/// VirtioFS backend handler implementing VhostUserBackendMut.
///
/// Wraps VirtioFsBackend in a Mutex for thread-safe access.
pub struct AspenVirtioFsHandler {
    /// Tiger Style: Uses std::sync::Mutex (not tokio::sync::Mutex) because
    /// VhostUserBackendMut trait methods are synchronous.
    backend: Mutex<VirtioFsBackend>,
}

impl AspenVirtioFsHandler {
    /// Create a new handler wrapping the given filesystem.
    pub fn new(fs: AspenFs) -> Result<Self, VirtioFsError> {
        let backend = VirtioFsBackend::new(fs)?;
        Ok(Self {
            backend: Mutex::new(backend),
        })
    }

    /// Clone the internal kill EventFd for external shutdown signaling.
    pub fn kill_event(&self) -> Result<EventFd, VirtioFsError> {
        let backend =
            self.backend.lock().map_err(|_| VirtioFsError::DaemonError("backend lock poisoned".to_string()))?;
        backend.kill_evt.try_clone().map_err(VirtioFsError::EventFdError)
    }
}

impl VhostUserBackendMut for AspenVirtioFsHandler {
    type Bitmap = ();
    type Vring = VringMutex;

    fn num_queues(&self) -> usize {
        NUM_QUEUES
    }

    fn max_queue_size(&self) -> usize {
        QUEUE_SIZE
    }

    fn features(&self) -> u64 {
        1 << VIRTIO_F_VERSION_1
            | 1 << VIRTIO_RING_F_INDIRECT_DESC
            | 1 << VIRTIO_RING_F_EVENT_IDX
            | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits()
    }

    fn protocol_features(&self) -> VhostUserProtocolFeatures {
        VhostUserProtocolFeatures::MQ | VhostUserProtocolFeatures::BACKEND_REQ
    }

    fn set_event_idx(&mut self, enabled: bool) {
        if let Ok(mut backend) = self.backend.lock() {
            backend.event_idx = enabled;
        }
    }

    fn update_memory(&mut self, mem: GuestMemoryAtomic<GuestMemoryMmap>) -> io::Result<()> {
        let mut backend = self.backend.lock().map_err(|_| io::Error::other("backend lock poisoned"))?;
        backend.mem = Some(mem);
        Ok(())
    }

    fn set_backend_req_fd(&mut self, vu_req: Backend) {
        if let Ok(mut backend) = self.backend.lock() {
            backend.vu_req = Some(vu_req);
        }
    }

    fn exit_event(&self, _thread_index: usize) -> Option<EventFd> {
        self.backend.lock().ok().and_then(|b| b.kill_evt.try_clone().ok())
    }

    fn handle_event(
        &mut self,
        device_event: u16,
        evset: EventSet,
        vrings: &[VringMutex],
        _thread_id: usize,
    ) -> io::Result<()> {
        if evset != EventSet::IN {
            return Err(VirtioFsError::NotEpollIn.into());
        }

        let mut vring_state = match device_event {
            HIPRIO_QUEUE_EVENT => {
                debug!("processing hiprio queue");
                vrings[0].get_mut()
            }
            REQ_QUEUE_EVENT => {
                debug!("processing request queue");
                vrings[1].get_mut()
            }
            _ => return Err(VirtioFsError::UnknownDeviceEvent(device_event).into()),
        };

        let mut backend = self.backend.lock().map_err(|_| io::Error::other("backend lock poisoned"))?;

        if backend.event_idx {
            // Optimized path: disable notifications while processing
            loop {
                let _ = vring_state.disable_notification();
                backend.process_queue(&mut vring_state)?;
                if !vring_state.enable_notification().unwrap_or(false) {
                    break;
                }
            }
        } else {
            backend.process_queue(&mut vring_state)?;
        }

        Ok(())
    }
}

/// Run the VirtioFS daemon.
///
/// Creates a vhost-user-fs server listening on the specified Unix socket.
/// The daemon will serve filesystem requests until the VMM disconnects.
///
/// Uses `VhostUserDaemon::serve()` which handles VMM disconnection gracefully
/// (treats `Disconnected` and `PartialMessage` as normal shutdown).
///
/// # Arguments
///
/// * `socket_path` - Path to the Unix domain socket for vhost-user protocol
/// * `fs` - The Aspen filesystem to serve
///
/// # Example
///
/// ```ignore
/// let fs = AspenFs::new(uid, gid, client);
/// run_virtiofs_daemon(Path::new("/run/virtiofsd/aspen.sock"), fs)?;
/// ```
pub fn run_virtiofs_daemon(socket_path: &Path, fs: AspenFs) -> Result<(), VirtioFsError> {
    info!(socket = %socket_path.display(), "starting VirtioFS daemon");

    let handler = AspenVirtioFsHandler::new(fs)?;
    let handler = Arc::new(RwLock::new(handler));

    let mut daemon =
        VhostUserDaemon::new(String::from("aspen-virtiofs"), handler, GuestMemoryAtomic::new(GuestMemoryMmap::new()))
            .map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

    // serve() handles listener creation, accept, request loop, and graceful
    // shutdown on VMM disconnection (sends exit events to epoll handler threads)
    daemon.serve(socket_path).map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

    info!("VirtioFS daemon stopped");

    Ok(())
}

/// Handle for a spawned VirtioFS daemon running on a background OS thread.
///
/// Provides graceful shutdown via the kill EventFd. Dropping without calling
/// `shutdown()` will leave the daemon thread running until process exit.
pub struct VirtioFsDaemonHandle {
    thread: Option<std::thread::JoinHandle<Result<(), VirtioFsError>>>,
    kill_evt: EventFd,
}

impl VirtioFsDaemonHandle {
    /// Signal the daemon to stop and wait for the thread to join.
    ///
    /// TODO: If called before a VMM connects, the daemon thread may deadlock
    /// in VhostUserDaemon::start()'s blocking accept() loop, which does not
    /// monitor the kill EventFd. In practice cloud-hypervisor connects quickly
    /// after socket creation, but a robust fix would use a non-blocking accept
    /// with epoll that also watches the kill EventFd, or set SO_RCVTIMEO on
    /// the listener socket.
    pub fn shutdown(mut self) -> Result<(), VirtioFsError> {
        info!("shutting down VirtioFS daemon");

        // Write to kill_evt to signal the daemon's epoll loop to exit
        self.kill_evt.write(1).map_err(VirtioFsError::EventFdError)?;

        // Join the daemon thread
        if let Some(thread) = self.thread.take() {
            match thread.join() {
                Ok(result) => result?,
                Err(_) => {
                    warn!("VirtioFS daemon thread panicked");
                    return Err(VirtioFsError::DaemonError("daemon thread panicked".to_string()));
                }
            }
        }

        info!("VirtioFS daemon shut down");
        Ok(())
    }
}

/// Spawn a VirtioFS daemon on a background OS thread.
///
/// Creates an `AspenVirtioFsHandler` for the given filesystem, spawns a
/// vhost-user-fs daemon on a dedicated OS thread, and returns a handle
/// for shutdown.
///
/// The daemon listens on `socket_path` for vhost-user connections from
/// a VMM (cloud-hypervisor, QEMU, etc.).
pub fn spawn_virtiofs_daemon(socket_path: &Path, fs: AspenFs) -> Result<VirtioFsDaemonHandle, VirtioFsError> {
    info!(socket = %socket_path.display(), "spawning VirtioFS daemon thread");

    // Create handler and extract kill_evt before moving into thread
    let handler = AspenVirtioFsHandler::new(fs)?;
    let kill_evt = handler.kill_event()?;

    let socket_path = socket_path.to_path_buf();
    let thread = std::thread::Builder::new()
        .name("virtiofs-daemon".to_string())
        .spawn(move || {
            let handler = Arc::new(RwLock::new(handler));

            let mut daemon = VhostUserDaemon::new(
                String::from("aspen-virtiofs"),
                handler,
                GuestMemoryAtomic::new(GuestMemoryMmap::new()),
            )
            .map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

            info!("VirtioFS daemon listening on {}", socket_path.display());

            // serve() handles listener creation, accept, request loop, and graceful
            // shutdown on VMM disconnection (sends exit events to epoll handler threads)
            daemon.serve(&socket_path).map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

            info!("VirtioFS daemon stopped");

            Ok(())
        })
        .map_err(|e| VirtioFsError::DaemonError(format!("failed to spawn thread: {e}")))?;

    Ok(VirtioFsDaemonHandle {
        thread: Some(thread),
        kill_evt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: spawn the VirtioFS daemon, connect as a VMM frontend
    /// over the Unix socket, complete the full vhost-user protocol handshake,
    /// and verify correct feature negotiation before clean shutdown.
    ///
    /// This exercises the real vhost-user protocol path that cloud-hypervisor
    /// takes when connecting to our daemon: socket accept, GET_FEATURES,
    /// SET_FEATURES, GET_PROTOCOL_FEATURES, SET_PROTOCOL_FEATURES,
    /// GET_QUEUE_NUM, and graceful disconnection.
    #[test]
    fn test_daemon_vhost_user_handshake() {
        use vhost::VhostBackend;
        use vhost::vhost_user::Frontend;
        use vhost::vhost_user::VhostUserFrontend;

        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("virtiofs.sock");

        // Create in-memory filesystem (real KV operations backed by BTreeMap,
        // no Aspen cluster needed)
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Spawn the virtiofs daemon on a background OS thread.
        // The thread binds the socket, then blocks in accept() waiting for a VMM.
        let handle = spawn_virtiofs_daemon(&socket_path, fs).unwrap();

        // Wait for socket file to appear (daemon creates it in serve() -> Listener::new())
        for _ in 0..100 {
            if socket_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(socket_path.exists(), "daemon failed to create socket");

        // Connect as a VMM (frontend/master side).
        // Frontend::connect() has built-in retry (5x 100ms) for ConnectionRefused.
        // This connection unblocks the daemon's accept(), starting the request handler.
        let mut frontend = Frontend::connect(&socket_path, NUM_QUEUES as u64).unwrap();

        // --- Ownership ---
        frontend.set_owner().unwrap();

        // --- Virtio feature negotiation ---
        // GET_FEATURES travels over the Unix socket, through the vhost-user protocol
        // codec, into VhostUserDaemon's dispatcher, which calls handler.features().
        let features = frontend.get_features().unwrap();

        // Verify all expected feature bits
        assert!(features & (1 << VIRTIO_F_VERSION_1) != 0, "missing VIRTIO_F_VERSION_1");
        assert!(features & (1 << VIRTIO_RING_F_INDIRECT_DESC) != 0, "missing VIRTIO_RING_F_INDIRECT_DESC");
        assert!(features & (1 << VIRTIO_RING_F_EVENT_IDX) != 0, "missing VIRTIO_RING_F_EVENT_IDX");
        assert!(features & VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits() != 0, "missing PROTOCOL_FEATURES");

        // Acknowledge all features back to the backend
        frontend.set_features(features).unwrap();

        // --- Protocol feature negotiation ---
        let proto_features = frontend.get_protocol_features().unwrap();
        assert!(proto_features.contains(VhostUserProtocolFeatures::MQ), "missing MQ");
        assert!(proto_features.contains(VhostUserProtocolFeatures::BACKEND_REQ), "missing BACKEND_REQ");
        frontend.set_protocol_features(proto_features).unwrap();

        // --- Query queue count (requires MQ protocol feature) ---
        let num_queues = frontend.get_queue_num().unwrap();
        assert_eq!(num_queues, NUM_QUEUES as u64, "queue count mismatch");

        // --- Graceful disconnection ---
        // Drop the frontend to close the socket. The daemon's serve() method
        // treats Disconnected as normal shutdown and sends exit events to
        // epoll handler threads.
        drop(frontend);

        // Join the daemon thread. Since serve() handles disconnection gracefully,
        // this should return Ok.
        handle.shutdown().unwrap();
    }

    /// Integration test: spawn VirtioFS daemon, connect as frontend, negotiate
    /// features, configure shared memory regions and virtqueues (the full VMM
    /// transport setup path before FUSE I/O begins).
    #[test]
    fn test_daemon_memory_and_queue_setup() {
        use std::os::unix::io::AsRawFd;

        use vhost::VhostBackend;
        use vhost::VhostUserMemoryRegionInfo;
        use vhost::VringConfigData;
        use vhost::vhost_user::Frontend;
        use vhost::vhost_user::VhostUserFrontend;
        use vm_memory::MmapRegion;
        use vmm_sys_util::eventfd::EventFd;

        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("virtiofs.sock");

        let fs = AspenFs::new_in_memory(1000, 1000);
        let handle = spawn_virtiofs_daemon(&socket_path, fs).unwrap();

        // Wait for socket
        for _ in 0..100 {
            if socket_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(socket_path.exists(), "daemon failed to create socket");

        let mut frontend = Frontend::connect(&socket_path, NUM_QUEUES as u64).unwrap();

        // --- Ownership + feature negotiation (same as handshake test) ---
        frontend.set_owner().unwrap();
        let features = frontend.get_features().unwrap();
        frontend.set_features(features).unwrap();
        let proto_features = frontend.get_protocol_features().unwrap();
        frontend.set_protocol_features(proto_features).unwrap();

        // --- Configure shared memory region (4 MB) ---
        // Create a memfd/tempfile for the shared memory region
        let mem_file = tempfile::tempfile().unwrap();
        let region_size: u64 = 4 * 1024 * 1024; // 4 MB
        mem_file.set_len(region_size).unwrap();

        // mmap the region so we have a valid userspace address
        let mmap_region = unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                region_size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                mem_file.as_raw_fd(),
                0,
            );
            assert_ne!(ptr, libc::MAP_FAILED, "mmap failed");
            MmapRegion::<()>::build_raw(
                ptr as *mut u8,
                region_size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
            )
            .unwrap()
        };

        let region_info = VhostUserMemoryRegionInfo {
            guest_phys_addr: 0,
            memory_size: region_size,
            userspace_addr: mmap_region.as_ptr() as u64,
            mmap_offset: 0,
            mmap_handle: mem_file.as_raw_fd(),
        };

        frontend.set_mem_table(&[region_info]).unwrap();

        // --- Configure virtqueues ---
        // Queue descriptors table, available ring, and used ring must be within
        // the guest memory region. We place them at fixed offsets within our 4MB region.
        // Addresses passed to set_vring_addr are VMM virtual addresses (mmap ptr + offset),
        // which the backend translates back to GPA via the memory mapping table.
        let queue_size: u16 = 256;
        let region_base = mmap_region.as_ptr() as u64;

        for queue_index in 0..NUM_QUEUES {
            // Offset each queue's structures within the region (64KB per queue)
            let base_offset = (queue_index as u64) * 0x10000;

            frontend.set_vring_num(queue_index, queue_size).unwrap();

            let vring_config = VringConfigData {
                queue_max_size: queue_size,
                queue_size,
                flags: 0,
                desc_table_addr: region_base + base_offset,
                used_ring_addr: region_base + base_offset + 0x2000,
                avail_ring_addr: region_base + base_offset + 0x1000,
                log_addr: None,
            };
            frontend.set_vring_addr(queue_index, &vring_config).unwrap();

            frontend.set_vring_base(queue_index, 0).unwrap();

            // Create eventfds for kick (guest -> host notification) and
            // call (host -> guest notification)
            let kick_fd = EventFd::new(libc::EFD_NONBLOCK).unwrap();
            let call_fd = EventFd::new(libc::EFD_NONBLOCK).unwrap();

            frontend.set_vring_kick(queue_index, &kick_fd).unwrap();
            frontend.set_vring_call(queue_index, &call_fd).unwrap();
            frontend.set_vring_enable(queue_index, true).unwrap();
        }

        // --- Graceful disconnection ---
        drop(frontend);

        // Unmap the shared memory region
        unsafe {
            libc::munmap(mmap_region.as_ptr() as *mut libc::c_void, region_size as usize);
        }

        handle.shutdown().unwrap();
    }

    #[test]
    fn test_virtio_features() {
        // Verify feature flags are correctly set
        assert_eq!(VIRTIO_F_VERSION_1, 32);
        assert_eq!(VIRTIO_RING_F_INDIRECT_DESC, 28);
        assert_eq!(VIRTIO_RING_F_EVENT_IDX, 29);
    }

    #[test]
    fn test_queue_constants() {
        assert_eq!(NUM_QUEUES, 2);
        assert_eq!(QUEUE_SIZE, 1024);
        assert_eq!(HIPRIO_QUEUE_EVENT, 0);
        assert_eq!(REQ_QUEUE_EVENT, 1);
    }

    #[test]
    fn test_error_display() {
        let err = VirtioFsError::MemoryNotConfigured;
        assert_eq!(err.to_string(), "guest memory not configured");

        let err = VirtioFsError::UnknownDeviceEvent(42);
        assert_eq!(err.to_string(), "unknown device event: 42");
    }

    #[test]
    fn test_handler_creation() {
        // Create a mock filesystem
        let fs = AspenFs::new_mock(1000, 1000);

        // Handler should be created successfully
        let handler = AspenVirtioFsHandler::new(fs);
        assert!(handler.is_ok());

        let handler = handler.unwrap();
        assert_eq!(handler.num_queues(), 2);
        assert_eq!(handler.max_queue_size(), 1024);
    }

    #[test]
    fn test_handler_features() {
        let fs = AspenFs::new_mock(1000, 1000);
        let handler = AspenVirtioFsHandler::new(fs).unwrap();

        let features = handler.features();
        // Should have VERSION_1, INDIRECT_DESC, EVENT_IDX, and PROTOCOL_FEATURES
        assert!(features & (1 << VIRTIO_F_VERSION_1) != 0);
        assert!(features & (1 << VIRTIO_RING_F_INDIRECT_DESC) != 0);
        assert!(features & (1 << VIRTIO_RING_F_EVENT_IDX) != 0);
    }

    #[test]
    fn test_handler_protocol_features() {
        let fs = AspenFs::new_mock(1000, 1000);
        let handler = AspenVirtioFsHandler::new(fs).unwrap();

        let proto_features = handler.protocol_features();
        assert!(proto_features.contains(VhostUserProtocolFeatures::MQ));
        assert!(proto_features.contains(VhostUserProtocolFeatures::BACKEND_REQ));
    }
}
