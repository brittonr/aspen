//! Vsock server for receiving execution requests from the host.
//!
//! The server listens on a vsock port and handles framed JSON messages
//! from the CloudHypervisorWorker on the host.

use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use snafu::ResultExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_vsock::VMADDR_CID_ANY;
use tokio_vsock::VsockAddr;
use tokio_vsock::VsockListener;
use tokio_vsock::VsockStream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::error::AgentError;
use crate::error::Result;
use crate::error::{self};
use crate::executor::Executor;
use crate::protocol::AgentMessage;
use crate::protocol::HostMessage;
use crate::protocol::LogMessage;
use crate::protocol::MAX_MESSAGE_SIZE;
use crate::protocol::vsock::DEFAULT_PORT;

/// Vsock server that accepts connections and processes requests.
pub struct VsockServer {
    port: u32,
    executor: Arc<Executor>,
}

impl VsockServer {
    /// Create a new vsock server.
    pub fn new(port: u32) -> Self {
        Self {
            port,
            executor: Arc::new(Executor::new()),
        }
    }

    /// Create a server with the default port.
    pub fn with_default_port() -> Self {
        Self::new(DEFAULT_PORT)
    }

    /// Run the server, accepting connections forever.
    pub async fn run(&self) -> Result<()> {
        let addr = VsockAddr::new(VMADDR_CID_ANY, self.port);

        let listener = VsockListener::bind(addr).context(error::BindVsockSnafu { port: self.port })?;

        info!(port = self.port, "vsock server listening");

        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    info!(cid = peer.cid(), port = peer.port(), "accepted connection");

                    let executor = self.executor.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, executor).await {
                            error!("connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("failed to accept connection: {}", e);
                    // Continue accepting other connections
                }
            }
        }
    }
}

/// Handle a single vsock connection.
async fn handle_connection(mut stream: VsockStream, executor: Arc<Executor>) -> Result<()> {
    // Send ready message
    send_message(&mut stream, &AgentMessage::Ready).await?;

    let mut read_buf = BytesMut::with_capacity(64 * 1024);

    loop {
        // Read messages
        let msg = match read_message(&mut stream, &mut read_buf).await {
            Ok(msg) => msg,
            Err(AgentError::ReadVsock { source }) if source.kind() == std::io::ErrorKind::UnexpectedEof => {
                info!("connection closed by peer");
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        debug!(?msg, "received message");

        match msg {
            HostMessage::Ping => {
                send_message(&mut stream, &AgentMessage::Pong).await?;
            }

            HostMessage::Execute(request) => {
                let job_id = request.id.clone();
                let executor = executor.clone();

                // Create channel for log streaming
                let (log_tx, mut log_rx) = mpsc::channel::<LogMessage>(1000);

                // Spawn execution task
                let exec_handle = tokio::spawn(async move { executor.execute(request, log_tx).await });

                // Stream logs to host
                while let Some(log_msg) = log_rx.recv().await {
                    let is_complete = matches!(log_msg, LogMessage::Complete(_));
                    // Convert LogMessage to flat AgentMessage variants
                    let agent_msg = match log_msg {
                        LogMessage::Stdout(data) => AgentMessage::Stdout { data },
                        LogMessage::Stderr(data) => AgentMessage::Stderr { data },
                        LogMessage::Complete(result) => AgentMessage::Complete { result },
                        LogMessage::Heartbeat { elapsed_secs } => AgentMessage::Heartbeat { elapsed_secs },
                    };
                    send_message(&mut stream, &agent_msg).await?;
                    if is_complete {
                        break;
                    }
                }

                // Wait for execution to finish and send final result
                match exec_handle.await {
                    Ok(Ok(result)) => {
                        // Send completion if not already sent via log channel
                        if result.error.is_none() || result.exit_code != -1 {
                            send_message(&mut stream, &AgentMessage::Complete { result }).await?;
                        }
                    }
                    Ok(Err(e)) => {
                        send_message(&mut stream, &AgentMessage::Error { message: e.to_string() }).await?;
                    }
                    Err(e) => {
                        error!(job_id = %job_id, "execution task panicked: {}", e);
                        send_message(&mut stream, &AgentMessage::Error {
                            message: format!("execution task panicked: {}", e),
                        })
                        .await?;
                    }
                }
            }

            HostMessage::Cancel { id } => match executor.cancel(&id).await {
                Ok(()) => {
                    info!(job_id = %id, "job cancelled");
                }
                Err(e) => {
                    warn!(job_id = %id, "failed to cancel job: {}", e);
                    send_message(&mut stream, &AgentMessage::Error { message: e.to_string() }).await?;
                }
            },

            HostMessage::Shutdown => {
                info!("shutdown requested, exiting");
                return Ok(());
            }
        }
    }
}

/// Read a length-prefixed JSON message from the stream.
async fn read_message(stream: &mut VsockStream, buf: &mut BytesMut) -> Result<HostMessage> {
    // Read length prefix (4 bytes, big-endian)
    while buf.len() < 4 {
        let n = stream.read_buf(buf).await.context(error::ReadVsockSnafu)?;
        if n == 0 {
            return Err(AgentError::ReadVsock {
                source: std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "connection closed"),
            });
        }
    }

    let len = buf.get_u32();

    // Validate message size
    if len > MAX_MESSAGE_SIZE {
        return error::MessageTooLargeSnafu {
            size: len,
            max: MAX_MESSAGE_SIZE,
        }
        .fail();
    }

    // Read message body
    while buf.len() < len as usize {
        let n = stream.read_buf(buf).await.context(error::ReadVsockSnafu)?;
        if n == 0 {
            return Err(AgentError::ReadVsock {
                source: std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "incomplete message"),
            });
        }
    }

    let msg_bytes = buf.split_to(len as usize);
    let msg: HostMessage = serde_json::from_slice(&msg_bytes).context(error::DeserializeMessageSnafu)?;

    Ok(msg)
}

/// Write a length-prefixed JSON message to the stream.
async fn send_message(stream: &mut VsockStream, msg: &AgentMessage) -> Result<()> {
    let json = serde_json::to_vec(msg).context(error::SerializeMessageSnafu)?;

    let mut frame = BytesMut::with_capacity(4 + json.len());
    frame.put_u32(json.len() as u32);
    frame.extend_from_slice(&json);

    stream.write_all(&frame).await.context(error::WriteVsockSnafu)?;

    stream.flush().await.context(error::WriteVsockSnafu)?;

    Ok(())
}
