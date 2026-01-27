//! Cloud Hypervisor REST API client over Unix socket.
//!
//! This client communicates with Cloud Hypervisor's HTTP API over a Unix socket.
//! API documentation: https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/vmm/src/api/openapi/cloud-hypervisor.yaml

use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio::net::UnixStream;
use tracing::{debug, trace};

use super::error::{self, CloudHypervisorError, Result};

/// Cloud Hypervisor REST API client.
///
/// Communicates with cloud-hypervisor via its Unix socket HTTP API.
#[derive(Clone)]
pub struct VmApiClient {
    socket_path: PathBuf,
}

impl VmApiClient {
    /// Create a new API client for a VM.
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Wait for the API socket to become available.
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(100);

        while tokio::time::Instant::now() < deadline {
            if self.socket_path.exists() {
                // Try to connect to verify it's actually ready
                if UnixStream::connect(&self.socket_path).await.is_ok() {
                    debug!(path = ?self.socket_path, "API socket is ready");
                    return Ok(());
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        error::SocketTimeoutSnafu {
            path: self.socket_path.clone(),
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Check if the VM API is responsive.
    pub async fn ping(&self) -> Result<()> {
        let _ = self.vmm_ping().await?;
        Ok(())
    }

    /// Get VMM information (ping endpoint).
    pub async fn vmm_ping(&self) -> Result<VmmPingResponse> {
        self.get("/api/v1/vmm.ping").await
    }

    /// Get VM information.
    pub async fn vm_info(&self) -> Result<VmInfo> {
        self.get("/api/v1/vm.info").await
    }

    /// Create a new VM with the given configuration.
    pub async fn create_vm(&self, config: &VmConfig) -> Result<()> {
        self.put_body("/api/v1/vm.create", config).await
    }

    /// Boot the VM.
    pub async fn boot(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.boot").await
    }

    /// Pause the VM.
    pub async fn pause(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.pause").await
    }

    /// Resume the VM.
    pub async fn resume(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.resume").await
    }

    /// Shutdown the VM gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.shutdown").await
    }

    /// Power off the VM (hard shutdown).
    pub async fn power_button(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.power-button").await
    }

    /// Delete the VM.
    pub async fn delete(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.delete").await
    }

    /// Reboot the VM.
    pub async fn reboot(&self) -> Result<()> {
        self.put_empty("/api/v1/vm.reboot").await
    }

    /// Create a snapshot of the VM.
    pub async fn snapshot(&self, dest_url: &str) -> Result<()> {
        let req = VmSnapshotRequest {
            destination_url: dest_url.to_string(),
        };
        self.put_body("/api/v1/vm.snapshot", &req).await
    }

    /// Restore a VM from a snapshot.
    pub async fn restore(&self, source_url: &str) -> Result<()> {
        let req = VmRestoreRequest {
            source_url: source_url.to_string(),
            prefault: Some(false),
        };
        self.put_body("/api/v1/vm.restore", &req).await
    }

    /// Add a vsock device to the VM.
    pub async fn add_vsock(&self, cid: u32, socket_path: &Path) -> Result<()> {
        let req = VsockConfig {
            cid,
            socket: socket_path.to_string_lossy().to_string(),
            id: None,
            pci_segment: None,
        };
        self.put_body("/api/v1/vm.add-vsock", &req).await
    }

    /// Add a virtio-fs device to the VM.
    pub async fn add_fs(&self, tag: &str, socket_path: &Path) -> Result<()> {
        let req = FsConfig {
            tag: tag.to_string(),
            socket: socket_path.to_string_lossy().to_string(),
            num_queues: 1,
            queue_size: 1024,
            id: None,
            pci_segment: None,
        };
        self.put_body("/api/v1/vm.add-fs", &req).await
    }

    /// Add a network device to the VM.
    pub async fn add_net(&self, tap: &str, mac: &str) -> Result<()> {
        let req = NetConfig {
            tap: Some(tap.to_string()),
            mac: Some(mac.to_string()),
            ip: None,
            mask: None,
            num_queues: 2,
            queue_size: 256,
            id: None,
            pci_segment: None,
            vhost_user: false,
            vhost_socket: None,
            vhost_mode: None,
        };
        self.put_body("/api/v1/vm.add-net", &req).await
    }

    /// Perform a GET request.
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let response = self.request(Method::GET, path, None).await?;
        self.parse_response(response).await
    }

    /// Perform a PUT request with JSON body, no response body expected.
    async fn put_body<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let body_json = serde_json::to_vec(body).context(error::SerializeRequestSnafu)?;
        let response = self.request(Method::PUT, path, Some(body_json)).await?;
        self.check_response(response).await
    }

    /// Perform a PUT request with empty body, expecting empty response.
    async fn put_empty(&self, path: &str) -> Result<()> {
        let response = self.request(Method::PUT, path, Some(vec![])).await?;
        self.check_response(response).await
    }

    /// Perform an HTTP request over Unix socket.
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<Response<Incoming>> {
        trace!(method = %method, path = %path, "API request");

        // Connect to Unix socket
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .context(error::ConnectSocketSnafu {
                path: self.socket_path.clone(),
            })?;

        let io = TokioIo::new(stream);

        // Create HTTP connection
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(|e| CloudHypervisorError::HttpRequest { source: e })?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                debug!("connection error: {}", e);
            }
        });

        // Build request
        let body_bytes = body
            .map(|b| Full::new(Bytes::from(b)))
            .unwrap_or_else(|| Full::new(Bytes::new()));

        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("Host", "localhost")
            .header("Content-Type", "application/json")
            .body(body_bytes)
            .expect("valid request");

        // Send request
        sender
            .send_request(req)
            .await
            .map_err(|e| CloudHypervisorError::HttpRequest { source: e })
    }

    /// Parse JSON response body.
    async fn parse_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: Response<Incoming>,
    ) -> Result<T> {
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|e| CloudHypervisorError::ReadBody { source: e })?
            .to_bytes();

        if !status.is_success() {
            let body_str = String::from_utf8_lossy(&body).to_string();
            return Err(CloudHypervisorError::ApiError {
                status: status.as_u16(),
                body: body_str,
            });
        }

        // Handle empty response
        if body.is_empty() {
            // Try to deserialize as unit type
            return serde_json::from_str("null").context(error::DeserializeResponseSnafu);
        }

        serde_json::from_slice(&body).context(error::DeserializeResponseSnafu)
    }

    /// Check response status without parsing body.
    async fn check_response(&self, response: Response<Incoming>) -> Result<()> {
        let status = response.status();

        if !status.is_success() {
            let body = response
                .into_body()
                .collect()
                .await
                .map_err(|e| CloudHypervisorError::ReadBody { source: e })?
                .to_bytes();
            let body_str = String::from_utf8_lossy(&body).to_string();
            return Err(CloudHypervisorError::ApiError {
                status: status.as_u16(),
                body: body_str,
            });
        }

        Ok(())
    }
}

// Cloud Hypervisor API types

/// VMM ping response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmmPingResponse {
    pub build_version: Option<String>,
    pub version: String,
    pub pid: Option<i64>,
    pub features: Option<Vec<String>>,
}

/// VM information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub state: String,
    pub config: VmConfig,
    pub memory_actual_size: Option<u64>,
    pub device_tree: Option<serde_json::Value>,
}

/// VM state enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    Created,
    Running,
    Shutdown,
    Paused,
}

impl VmState {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Created" => Some(Self::Created),
            "Running" => Some(Self::Running),
            "Shutdown" => Some(Self::Shutdown),
            "Paused" => Some(Self::Paused),
            _ => None,
        }
    }
}

/// VM configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<PayloadConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<CpusConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<ConsoleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console: Option<ConsoleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disks: Option<Vec<DiskConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<Vec<NetConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs: Option<Vec<FsConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsock: Option<VsockConfig>,
}

/// Kernel/initrd payload configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfig {
    pub kernel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initramfs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,
}

/// CPU configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpusConfig {
    pub boot_vcpus: u8,
    pub max_vcpus: u8,
}

/// Memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub size: u64, // bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,
}

/// Console configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Disk configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
    pub num_queues: u32,
    pub queue_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pci_segment: Option<u16>,
    #[serde(default)]
    pub vhost_user: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vhost_socket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vhost_mode: Option<String>,
}

/// Virtio-fs configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsConfig {
    pub tag: String,
    pub socket: String,
    pub num_queues: u32,
    pub queue_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pci_segment: Option<u16>,
}

/// Vsock configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsockConfig {
    pub cid: u32,
    pub socket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pci_segment: Option<u16>,
}

/// VM snapshot request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSnapshotRequest {
    pub destination_url: String,
}

/// VM restore request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRestoreRequest {
    pub source_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefault: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_serialization() {
        let config = VmConfig {
            payload: Some(PayloadConfig {
                kernel: "/path/to/kernel".to_string(),
                initramfs: Some("/path/to/initrd".to_string()),
                cmdline: Some("console=ttyS0".to_string()),
            }),
            cpus: Some(CpusConfig {
                boot_vcpus: 4,
                max_vcpus: 4,
            }),
            memory: Some(MemoryConfig {
                size: 8 * 1024 * 1024 * 1024,
                hugepages: None,
                shared: Some(true),
            }),
            serial: Some(ConsoleConfig {
                file: Some("/tmp/serial.log".to_string()),
                mode: None,
            }),
            console: None,
            disks: None,
            net: None,
            fs: None,
            vsock: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("kernel"));
        assert!(json.contains("boot_vcpus"));
    }

    #[test]
    fn test_vm_state_parsing() {
        assert_eq!(VmState::from_str("Running"), Some(VmState::Running));
        assert_eq!(VmState::from_str("Paused"), Some(VmState::Paused));
        assert_eq!(VmState::from_str("Unknown"), None);
    }
}
