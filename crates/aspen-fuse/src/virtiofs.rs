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
use vhost::vhost_user::Listener;
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
/// The daemon will serve filesystem requests until shutdown is signaled.
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

    // Create the backend handler
    let handler = AspenVirtioFsHandler::new(fs)?;

    // Wrap in Arc<RwLock> for the daemon
    let handler = Arc::new(RwLock::new(handler));

    // Create the vhost-user daemon
    let mut daemon =
        VhostUserDaemon::new(String::from("aspen-virtiofs"), handler, GuestMemoryAtomic::new(GuestMemoryMmap::new()))
            .map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

    // Create listener on the socket
    let listener = Listener::new(socket_path, true).map_err(|e| VirtioFsError::ListenerError(format!("{e:?}")))?;

    info!("VirtioFS daemon listening on {}", socket_path.display());

    // Start serving requests
    daemon.start(listener).map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

    info!("VirtioFS daemon started, waiting for VMM connection");

    // Wait for daemon to finish
    daemon.wait().map_err(|e| VirtioFsError::DaemonError(format!("{:?}", e)))?;

    info!("VirtioFS daemon stopped");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
