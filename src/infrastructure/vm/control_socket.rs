//! VM Control Socket Communication
//!
//! Handles Unix socket communication with VMs

use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};

use super::vm_types::VmControlMessage;

/// Handles control socket communication with VMs
pub struct VmControlSocket;

impl VmControlSocket {
    /// Create a new control socket handler
    pub fn new() -> Self {
        Self
    }

    /// Wait for VM to become ready by pinging control socket
    pub async fn wait_for_vm_ready(&self, control_socket: &Path) -> Result<()> {
        let timeout_duration = Duration::from_secs(30);

        timeout(timeout_duration, async {
            loop {
                // Try to connect to control socket
                match UnixStream::connect(control_socket).await {
                    Ok(mut stream) => {
                        // Send ping
                        let ping = VmControlMessage::Ping;
                        let msg = serde_json::to_string(&ping)?;
                        stream.write_all(msg.as_bytes()).await?;
                        stream.write_all(b"\n").await?;

                        // Wait for pong
                        let mut reader = BufReader::new(stream);
                        let mut response = String::new();
                        reader.read_line(&mut response).await?;

                        if let Ok(msg) = serde_json::from_str::<VmControlMessage>(&response) {
                            if matches!(msg, VmControlMessage::Pong { .. }) {
                                return Ok(());
                            }
                        }
                    }
                    Err(_) => {
                        // Socket not ready yet
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        })
        .await
        .map_err(|_| anyhow!("Timeout waiting for VM to be ready"))?
    }

    /// Send job to VM via control socket
    pub async fn send_job(
        &self,
        control_socket: &Path,
        job: &crate::Job,
    ) -> Result<()> {
        // Connect to control socket
        let mut stream = UnixStream::connect(control_socket).await?;

        // Send job execution command
        let message = VmControlMessage::ExecuteJob {
            job: serde_json::to_value(job)?,
        };

        let msg = serde_json::to_string(&message)?;
        stream.write_all(msg.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // Wait for acknowledgment
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        if let Ok(msg) = serde_json::from_str::<VmControlMessage>(&response) {
            match msg {
                VmControlMessage::Ack => Ok(()),
                VmControlMessage::Error { message } => {
                    Err(anyhow!("VM rejected job: {}", message))
                }
                _ => Err(anyhow!("Unexpected response from VM")),
            }
        } else {
            Err(anyhow!("Invalid response from VM"))
        }
    }

    /// Send graceful shutdown command to VM
    pub async fn send_shutdown(&self, control_socket: &Path, timeout_secs: u32) -> Result<()> {
        match UnixStream::connect(control_socket).await {
            Ok(mut stream) => {
                let message = VmControlMessage::Shutdown { timeout_secs };
                let msg = serde_json::to_string(&message)?;
                stream.write_all(msg.as_bytes()).await?;
                stream.write_all(b"\n").await?;
                Ok(())
            }
            Err(e) => Err(anyhow!("Failed to connect to control socket: {}", e)),
        }
    }
}

impl Default for VmControlSocket {
    fn default() -> Self {
        Self::new()
    }
}
