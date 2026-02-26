//! Error types for Cloud Hypervisor worker.

use std::path::PathBuf;

use snafu::Snafu;

/// Errors from Cloud Hypervisor worker operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[allow(missing_docs)] // Snafu errors are documented via display attributes
pub enum CloudHypervisorError {
    /// Failed to connect to VM API socket.
    #[snafu(display("failed to connect to VM API socket at {}: {source}", path.display()))]
    ConnectSocket { path: PathBuf, source: std::io::Error },

    /// API socket not found after timeout.
    #[snafu(display("VM API socket not found at {} after {timeout_ms}ms", path.display()))]
    SocketTimeout { path: PathBuf, timeout_ms: u64 },

    /// HTTP request to VM API failed.
    #[snafu(display("HTTP request to VM API failed: {source}"))]
    HttpRequest { source: hyper::Error },

    /// HTTP response body read failed.
    #[snafu(display("failed to read HTTP response body: {source}"))]
    ReadBody { source: hyper::Error },

    /// VM API returned error status.
    #[snafu(display("VM API returned error {status}: {body}"))]
    ApiError { status: u16, body: String },

    /// Failed to serialize request body.
    #[snafu(display("failed to serialize request: {source}"))]
    SerializeRequest { source: serde_json::Error },

    /// Failed to deserialize response body.
    #[snafu(display("failed to deserialize response: {source}"))]
    DeserializeResponse { source: serde_json::Error },

    /// VM is in invalid state for operation.
    #[snafu(display("VM {vm_id} is in invalid state {state} for operation {operation}"))]
    InvalidState {
        vm_id: String,
        state: String,
        operation: String,
    },

    /// Failed to start virtiofsd.
    #[snafu(display("failed to start virtiofsd: {source}"))]
    StartVirtiofsd { source: std::io::Error },

    /// Virtiofsd socket not ready after timeout.
    #[snafu(display(
        "virtiofsd socket for '{tag}' not ready at {} after {timeout_ms}ms on VM {vm_id}",
        path.display()
    ))]
    VirtiofsdSocketNotReady {
        vm_id: String,
        tag: String,
        path: PathBuf,
        timeout_ms: u64,
    },

    /// Failed to start cloud-hypervisor.
    #[snafu(display("failed to start cloud-hypervisor: {source}"))]
    StartCloudHypervisor { source: std::io::Error },

    /// VM boot timed out.
    #[snafu(display("VM {vm_id} boot timed out after {timeout_ms}ms"))]
    BootTimeout { vm_id: String, timeout_ms: u64 },

    /// Guest agent not responding.
    #[snafu(display("guest agent on VM {vm_id} not responding after {timeout_ms}ms"))]
    GuestAgentTimeout { vm_id: String, timeout_ms: u64 },

    /// Failed to connect to guest agent via vsock.
    #[snafu(display("failed to connect to guest agent on VM {vm_id}: {source}"))]
    VsockConnect { vm_id: String, source: std::io::Error },

    /// Failed to send message to guest agent.
    #[snafu(display("failed to send message to guest agent: {source}"))]
    VsockSend { source: std::io::Error },

    /// Failed to receive message from guest agent.
    #[snafu(display("failed to receive message from guest agent: {source}"))]
    VsockRecv { source: std::io::Error },

    /// Guest agent returned error.
    #[snafu(display("guest agent error: {message}"))]
    GuestAgentError { message: String },

    /// No VMs available in pool.
    #[snafu(display("no VMs available in pool after {timeout_ms}ms"))]
    NoVmsAvailable { timeout_ms: u64 },

    /// Pool is at capacity.
    #[snafu(display("VM pool is at capacity ({max_vms} VMs)"))]
    PoolAtCapacity { max_vms: u32 },

    /// Failed to create VM.
    #[snafu(display("failed to create VM: {reason}"))]
    CreateVmFailed { reason: String },

    /// Failed to snapshot VM.
    #[snafu(display("failed to snapshot VM {vm_id}: {reason}"))]
    SnapshotFailed { vm_id: String, reason: String },

    /// Failed to restore VM from snapshot.
    #[snafu(display("failed to restore VM from snapshot at {}: {reason}", path.display()))]
    RestoreFailed { path: PathBuf, reason: String },

    /// Workspace setup failed.
    #[snafu(display("failed to setup workspace: {source}"))]
    WorkspaceSetup { source: std::io::Error },

    /// Job execution failed.
    #[snafu(display("job execution failed: {reason}"))]
    ExecutionFailed { reason: String },

    /// Invalid configuration.
    #[snafu(display("invalid configuration: {message}"))]
    InvalidConfig { message: String },

    /// Glob pattern error.
    #[snafu(display("invalid glob pattern '{pattern}': {source}"))]
    GlobPattern {
        pattern: String,
        source: glob::PatternError,
    },

    /// Failed to read artifact file.
    #[snafu(display("failed to read artifact {}: {source}", path.display()))]
    ReadArtifact { path: PathBuf, source: std::io::Error },

    /// Artifact path escapes workspace.
    #[snafu(display("artifact path {} escapes workspace", path.display()))]
    ArtifactEscapesWorkspace { path: PathBuf },

    /// Failed to seed workspace with source.
    #[snafu(display("failed to seed workspace: {reason}"))]
    WorkspaceSeed { reason: String },

    /// Failed to create source archive.
    #[snafu(display("failed to create source archive: {reason}"))]
    SourceArchive { reason: String },

    /// Failed to start in-process VirtioFS daemon.
    #[snafu(display("failed to start VirtioFS daemon: {reason}"))]
    StartVirtioFsDaemon { reason: String },

    /// Failed to provision workspace via AspenFs.
    #[snafu(display("workspace provision failed: {reason}"))]
    WorkspaceProvision { reason: String },
}

/// Result type for Cloud Hypervisor worker operations.
pub type Result<T> = std::result::Result<T, CloudHypervisorError>;
