// Work Queue HTTP Client
//
// Connects to control plane via iroh+h3 and provides the same API as WorkQueue
// but over the network instead of embedded hiqlite.

use anyhow::{anyhow, Result};
use bytes::Buf;
use iroh::{Endpoint, EndpointAddr};
use iroh_tickets::endpoint::EndpointTicket;
use serde::Serialize;
use std::{future, str::FromStr};

use crate::{WorkItem, WorkStatus, WorkQueueStats};

/// HTTP client for Work Queue API over iroh+h3
pub struct WorkQueueClient {
    endpoint: Endpoint,
    control_plane_addr: EndpointAddr,
    node_id: String,
}

impl WorkQueueClient {
    /// Connect to control plane using an iroh+h3://endpoint{...} ticket
    ///
    /// # Example
    /// ```no_run
    /// let ticket = "iroh+h3://endpoint{base32...}/";
    /// let client = WorkQueueClient::connect(ticket).await?;
    /// ```
    pub async fn connect(ticket: &str) -> Result<Self> {
        // Parse the iroh+h3 endpoint ticket
        // Strip the iroh+h3:// prefix if present
        let ticket_str = ticket.strip_prefix("iroh+h3://").unwrap_or(ticket);
        // Strip trailing slash if present
        let ticket_str = ticket_str.trim_end_matches('/');

        let ticket = EndpointTicket::from_str(ticket_str)
            .map_err(|e| anyhow!("Failed to parse endpoint ticket: {}", e))?;

        let control_plane_addr: EndpointAddr = ticket.into();

        // Create local iroh endpoint
        let endpoint = Endpoint::builder()
            .alpns(vec![b"iroh+h3".to_vec()])
            .bind()
            .await?;

        let node_id = endpoint.id().to_string();

        tracing::info!(
            node_id = %node_id,
            "WorkQueueClient connected to control plane"
        );

        Ok(Self {
            endpoint,
            control_plane_addr,
            node_id,
        })
    }

    /// Claim an available work item from the queue
    ///
    /// Returns None if no work is available
    pub async fn claim_work(&self) -> Result<Option<WorkItem>> {
        let response = self.post("/queue/claim", &()).await?;

        if response.status == 204 {
            // No content = no work available
            return Ok(None);
        }

        if response.status != 200 {
            return Err(anyhow!("Failed to claim work: HTTP {}", response.status));
        }

        let work_item: WorkItem = serde_json::from_slice(&response.body)?;
        Ok(Some(work_item))
    }

    /// Update the status of a work item
    pub async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()> {
        #[derive(Serialize)]
        struct StatusUpdate {
            status: WorkStatus,
        }

        let path = format!("/queue/status/{}", job_id);
        let response = self.post(&path, &StatusUpdate { status }).await?;

        if response.status != 200 {
            return Err(anyhow!(
                "Failed to update status for job {}: HTTP {}",
                job_id,
                response.status
            ));
        }

        Ok(())
    }

    /// List all work items in the queue
    pub async fn list_work(&self) -> Result<Vec<WorkItem>> {
        let response = self.get("/queue/list").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to list work: HTTP {}", response.status));
        }

        let items: Vec<WorkItem> = serde_json::from_slice(&response.body)?;
        Ok(items)
    }

    /// Get queue statistics
    pub async fn stats(&self) -> Result<WorkQueueStats> {
        let response = self.get("/queue/stats").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to get stats: HTTP {}", response.status));
        }

        let stats: WorkQueueStats = serde_json::from_slice(&response.body)?;
        Ok(stats)
    }

    /// Get the node ID of this client
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    // Internal helper: Make HTTP GET request over iroh+h3
    async fn get(&self, path: &str) -> Result<HttpResponse> {
        self.request("GET", path, None).await
    }

    // Internal helper: Make HTTP POST request over iroh+h3
    async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<HttpResponse> {
        let json_body = serde_json::to_vec(body)?;
        self.request("POST", path, Some(json_body)).await
    }

    // Internal: Generic HTTP request over iroh+h3
    async fn request(&self, method: &str, path: &str, body: Option<Vec<u8>>) -> Result<HttpResponse> {
        // Connect to control plane via iroh P2P
        let conn = self.endpoint
            .connect(self.control_plane_addr.clone(), b"iroh+h3")
            .await
            .map_err(|e| anyhow!("Failed to connect to control plane: {}", e))?;

        // Create HTTP/3 connection
        let conn = h3_iroh::Connection::new(conn);
        let (mut driver, mut send_request) = h3::client::new(conn)
            .await
            .map_err(|e| anyhow!("Failed to create h3 client: {}", e))?;

        // Build HTTP request
        let req = http::Request::builder()
            .method(method)
            .uri(format!("http://control-plane{}", path))
            .header("content-type", "application/json")
            .body(())
            .map_err(|e| anyhow!("Failed to build request: {}", e))?;

        // Send request and drive connection concurrently
        let response_fut = async {
            let mut stream = send_request.send_request(req).await?;

            // Send body if present
            if let Some(body_bytes) = body {
                stream.send_data(bytes::Bytes::from(body_bytes)).await?;
            }
            stream.finish().await?;

            // Receive response
            let response = stream.recv_response().await?;
            let status = response.status().as_u16();

            // Read response body
            let mut body = Vec::new();
            while let Some(mut chunk) = stream.recv_data().await? {
                while chunk.has_remaining() {
                    let bytes = chunk.chunk();
                    body.extend_from_slice(bytes);
                    let len = bytes.len();
                    chunk.advance(len);
                }
            }

            Ok::<HttpResponse, anyhow::Error>(HttpResponse { status, body })
        };

        let drive_fut = async move {
            let err = future::poll_fn(|cx| driver.poll_close(cx)).await;
            match err {
                h3::error::ConnectionError::Local { ref error, .. } => {
                    if matches!(error, h3::error::LocalError::Closing { .. }) {
                        Ok(())
                    } else {
                        Err(err)
                    }
                }
                _ => Err(err),
            }
        };

        // Run both concurrently
        let (response_result, drive_result) = tokio::join!(response_fut, drive_fut);
        drive_result.map_err(|e| anyhow!("h3 connection error: {}", e))?;
        response_result
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}
