// Control Protocol - Socket communication with VMs

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use super::vm_types::{VmControlMessage, VmMetrics};

/// Control protocol handler for VM communication
pub struct ControlProtocol {
    socket_path: PathBuf,
    vm_id: Uuid,
    metrics: Arc<RwLock<VmMetrics>>,
    command_tx: mpsc::Sender<VmCommand>,
    command_rx: Arc<RwLock<mpsc::Receiver<VmCommand>>>,
}

/// Commands that can be sent to VMs
#[derive(Debug, Clone)]
pub enum VmCommand {
    /// Execute a job
    ExecuteJob {
        job: serde_json::Value,
        response_tx: mpsc::Sender<Result<()>>,
    },
    /// Ping health check
    Ping {
        response_tx: mpsc::Sender<Result<VmPongResponse>>,
    },
    /// Get VM status
    GetStatus {
        response_tx: mpsc::Sender<Result<VmStatusResponse>>,
    },
    /// Shutdown VM
    Shutdown {
        timeout_secs: u32,
        response_tx: mpsc::Sender<Result<()>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmPongResponse {
    pub uptime_secs: i64,
    pub jobs_completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatusResponse {
    pub state: String,
    pub metrics: VmMetrics,
}

impl ControlProtocol {
    /// Create new control protocol handler
    pub fn new(vm_id: Uuid, socket_path: PathBuf) -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);

        Self {
            socket_path,
            vm_id,
            metrics: Arc::new(RwLock::new(VmMetrics::default())),
            command_tx,
            command_rx: Arc::new(RwLock::new(command_rx)),
        }
    }

    /// Start the control protocol server (VM side)
    pub async fn start_server(&self) -> Result<()> {
        // Remove old socket if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Create parent directory
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        tracing::info!(
            vm_id = %self.vm_id,
            socket = %self.socket_path.display(),
            "Control protocol server started"
        );

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    // Handle each connection in a separate task
                    let metrics = Arc::clone(&self.metrics);
                    let vm_id = self.vm_id;

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, vm_id, metrics).await {
                            tracing::error!(error = %e, "Connection handler error");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to accept connection");
                }
            }
        }
    }

    /// Handle a single connection
    async fn handle_connection(
        mut stream: UnixStream,
        vm_id: Uuid,
        metrics: Arc<RwLock<VmMetrics>>,
    ) -> Result<()> {
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();

        // Read command
        reader.read_line(&mut line).await?;

        // Parse command
        let message: VmControlMessage = serde_json::from_str(&line)?;

        tracing::debug!(vm_id = %vm_id, message = ?message, "Received control message");

        // Process command and send response
        let response = match message {
            VmControlMessage::Ping => {
                let metrics = metrics.read().await;
                VmControlMessage::Pong {
                    uptime_secs: chrono::Utc::now().timestamp(),
                    jobs_completed: metrics.jobs_completed,
                }
            }
            VmControlMessage::ExecuteJob { job: _ } => {
                // In a real implementation, this would queue the job
                tracing::info!(vm_id = %vm_id, "Executing job");

                // Update metrics
                {
                    let mut metrics = metrics.write().await;
                    metrics.jobs_completed += 1;
                }

                VmControlMessage::Ack
            }
            VmControlMessage::GetStatus => {
                let metrics = metrics.read().await;
                VmControlMessage::Status {
                    state: "Ready".to_string(),
                    metrics: metrics.clone(),
                }
            }
            VmControlMessage::Shutdown { timeout_secs } => {
                tracing::info!(
                    vm_id = %vm_id,
                    timeout_secs = timeout_secs,
                    "Shutdown requested"
                );
                VmControlMessage::Ack
            }
            _ => VmControlMessage::Error {
                message: "Unknown command".to_string(),
            },
        };

        // Send response
        let response_json = serde_json::to_string(&response)?;
        stream.write_all(response_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        Ok(())
    }

    /// Send a command to VM (client side)
    pub async fn send_command(&self, message: VmControlMessage) -> Result<VmControlMessage> {
        let socket_path = &self.socket_path;

        // Connect with timeout
        let stream = timeout(
            Duration::from_secs(5),
            UnixStream::connect(socket_path)
        ).await
            .map_err(|_| anyhow!("Connection timeout"))?
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        // Send message
        let mut stream = stream;
        let message_json = serde_json::to_string(&message)?;
        stream.write_all(message_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // Read response with timeout
        let response = timeout(
            Duration::from_secs(10),
            Self::read_response(&mut stream)
        ).await
            .map_err(|_| anyhow!("Response timeout"))??;

        Ok(response)
    }

    /// Read response from stream
    async fn read_response(stream: &mut UnixStream) -> Result<VmControlMessage> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: VmControlMessage = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// High-level API: Execute job
    pub async fn execute_job(&self, job: serde_json::Value) -> Result<()> {
        let message = VmControlMessage::ExecuteJob { job };
        let response = self.send_command(message).await?;

        match response {
            VmControlMessage::Ack => Ok(()),
            VmControlMessage::Error { message } => Err(anyhow!("VM error: {}", message)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// High-level API: Health check
    pub async fn ping(&self) -> Result<VmPongResponse> {
        let message = VmControlMessage::Ping;
        let response = self.send_command(message).await?;

        match response {
            VmControlMessage::Pong {
                uptime_secs,
                jobs_completed,
            } => Ok(VmPongResponse {
                uptime_secs,
                jobs_completed,
            }),
            VmControlMessage::Error { message } => Err(anyhow!("VM error: {}", message)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// High-level API: Get status
    pub async fn get_status(&self) -> Result<VmStatusResponse> {
        let message = VmControlMessage::GetStatus;
        let response = self.send_command(message).await?;

        match response {
            VmControlMessage::Status { state, metrics } => Ok(VmStatusResponse {
                state,
                metrics,
            }),
            VmControlMessage::Error { message } => Err(anyhow!("VM error: {}", message)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// High-level API: Shutdown VM
    pub async fn shutdown(&self, timeout_secs: u32) -> Result<()> {
        let message = VmControlMessage::Shutdown { timeout_secs };
        let response = self.send_command(message).await?;

        match response {
            VmControlMessage::Ack => Ok(()),
            VmControlMessage::Error { message } => Err(anyhow!("VM error: {}", message)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }

    /// Get command sender for async command processing
    pub fn get_command_sender(&self) -> mpsc::Sender<VmCommand> {
        self.command_tx.clone()
    }

    /// Process commands from queue (VM side)
    pub async fn process_commands(&self) {
        let mut rx = self.command_rx.write().await;

        while let Some(command) = rx.recv().await {
            match command {
                VmCommand::ExecuteJob { job, response_tx } => {
                    let result = self.execute_job(job).await;
                    if let Err(e) = response_tx.send(result).await {
                        tracing::warn!("Failed to send ExecuteJob response: {:?}", e);
                    }
                }
                VmCommand::Ping { response_tx } => {
                    let result = self.ping().await;
                    if let Err(e) = response_tx.send(result).await {
                        tracing::warn!("Failed to send Ping response: {:?}", e);
                    }
                }
                VmCommand::GetStatus { response_tx } => {
                    let result = self.get_status().await;
                    if let Err(e) = response_tx.send(result).await {
                        tracing::warn!("Failed to send GetStatus response: {:?}", e);
                    }
                }
                VmCommand::Shutdown {
                    timeout_secs,
                    response_tx,
                } => {
                    let result = self.shutdown(timeout_secs).await;
                    if let Err(e) = response_tx.send(result).await {
                        tracing::warn!("Failed to send Shutdown response: {:?}", e);
                    }
                }
            }
        }
    }
}

/// Control protocol client for easy VM communication
pub struct ControlClient {
    socket_path: PathBuf,
    vm_id: Uuid,
}

impl ControlClient {
    /// Create new control client
    pub fn new(vm_id: Uuid, socket_path: PathBuf) -> Self {
        Self { socket_path, vm_id }
    }

    /// Send command and get response
    pub async fn send(&self, message: VmControlMessage) -> Result<VmControlMessage> {
        // Connect to socket
        let mut stream = timeout(
            Duration::from_secs(5),
            UnixStream::connect(&self.socket_path)
        ).await
            .map_err(|_| anyhow!("Connection timeout"))?
            .map_err(|e| anyhow!("Failed to connect to VM {}: {}", self.vm_id, e))?;

        // Send message
        let message_json = serde_json::to_string(&message)?;
        stream.write_all(message_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: VmControlMessage = serde_json::from_str(&response_line)?;
        Ok(response)
    }

    /// Execute a job on the VM
    pub async fn execute_job(&self, job: serde_json::Value) -> Result<()> {
        let response = self.send(VmControlMessage::ExecuteJob { job }).await?;

        match response {
            VmControlMessage::Ack => Ok(()),
            VmControlMessage::Error { message } => {
                Err(anyhow!("VM {} rejected job: {}", self.vm_id, message))
            }
            _ => Err(anyhow!("Unexpected response from VM {}", self.vm_id)),
        }
    }

    /// Ping the VM
    pub async fn ping(&self) -> Result<(i64, u32)> {
        let response = self.send(VmControlMessage::Ping).await?;

        match response {
            VmControlMessage::Pong {
                uptime_secs,
                jobs_completed,
            } => Ok((uptime_secs, jobs_completed)),
            VmControlMessage::Error { message } => {
                Err(anyhow!("VM {} ping failed: {}", self.vm_id, message))
            }
            _ => Err(anyhow!("Unexpected response from VM {}", self.vm_id)),
        }
    }

    /// Request graceful shutdown
    pub async fn shutdown(&self, timeout_secs: u32) -> Result<()> {
        let response = self
            .send(VmControlMessage::Shutdown { timeout_secs })
            .await?;

        match response {
            VmControlMessage::Ack => Ok(()),
            VmControlMessage::Error { message } => {
                Err(anyhow!("VM {} shutdown failed: {}", self.vm_id, message))
            }
            _ => Err(anyhow!("Unexpected response from VM {}", self.vm_id)),
        }
    }
}
