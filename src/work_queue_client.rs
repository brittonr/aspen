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

use crate::{Job, JobStatus, QueueStats, Worker, WorkerRegistration, WorkerHeartbeat, WorkerStats};

/// HTTP client for Work Queue API over iroh+h3
#[derive(Clone)]
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
    /// # Arguments
    /// * `worker_id` - Optional worker ID to assign the job to
    /// * `worker_type` - Optional worker type for filtering compatible jobs
    ///
    /// Returns None if no work is available
    pub async fn claim_work(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<Job>> {
        tracing::info!(worker_id = ?worker_id, worker_type = ?worker_type, "claim_work() called - about to POST to /queue/claim");

        // Build query parameters
        let mut url = "/queue/claim".to_string();
        let mut query_parts = Vec::new();
        if let Some(wid) = worker_id {
            query_parts.push(format!("worker_id={}", urlencoding::encode(wid)));
        }
        if let Some(wt) = worker_type {
            query_parts.push(format!("worker_type={}", wt));
        }
        if !query_parts.is_empty() {
            url.push('?');
            url.push_str(&query_parts.join("&"));
        }

        let response = self.post(&url, &()).await?;
        tracing::info!(status = response.status, body_len = response.body.len(), "POST response received");

        if response.status == 204 {
            // No content = no work available
            tracing::info!("No work available (204 status)");
            return Ok(None);
        }

        if response.status != 200 {
            tracing::error!(status = response.status, "Non-200 status from claim");
            return Err(anyhow!("Failed to claim work: HTTP {}", response.status));
        }

        let job: Job = serde_json::from_slice(&response.body)?;
        tracing::info!(job_id = %job.id, "Work item parsed successfully");
        Ok(Some(job))
    }

    /// Update the status of a work item
    pub async fn update_status(&self, job_id: &str, status: JobStatus, error_message: Option<String>) -> Result<()> {
        #[derive(Serialize)]
        struct StatusUpdate {
            status: JobStatus,
            error_message: Option<String>,
        }

        let path = format!("/queue/status/{}", job_id);
        let response = self.post(&path, &StatusUpdate { status, error_message }).await?;

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
    pub async fn list_work(&self) -> Result<Vec<Job>> {
        let response = self.get("/queue/list").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to list work: HTTP {}", response.status));
        }

        let jobs: Vec<Job> = serde_json::from_slice(&response.body)?;
        Ok(jobs)
    }

    /// Get queue statistics
    pub async fn stats(&self) -> Result<QueueStats> {
        let response = self.get("/queue/stats").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to get stats: HTTP {}", response.status));
        }

        let stats: QueueStats = serde_json::from_slice(&response.body)?;
        Ok(stats)
    }

    // =========================================================================
    // WORKER MANAGEMENT API
    // =========================================================================

    /// Register a worker with the control plane
    ///
    /// Called by worker binaries on startup to register with the orchestrator.
    /// Returns the assigned worker ID.
    pub async fn register_worker(&self, registration: WorkerRegistration) -> Result<Worker> {
        #[derive(serde::Deserialize)]
        struct RegisterResponse {
            worker_id: String,
        }

        let response = self.post("/workers/register", &registration).await?;

        if response.status != 201 {
            return Err(anyhow!("Failed to register worker: HTTP {}", response.status));
        }

        // Parse the full worker object from the response
        let worker: Worker = serde_json::from_slice(&response.body)
            .or_else(|_: serde_json::Error| -> std::result::Result<Worker, serde_json::Error> {
                // Fallback: If the response is just {worker_id, message}, construct a Worker
                let register_resp: RegisterResponse = serde_json::from_slice(&response.body)?;
                Ok(Worker {
                    id: register_resp.worker_id.clone(),
                    worker_type: registration.worker_type,
                    status: crate::WorkerStatus::Online,
                    endpoint_id: registration.endpoint_id,
                    registered_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to get timestamp: {}", e);
                            0 // Use epoch as fallback
                        }),
                    last_heartbeat: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to get timestamp: {}", e);
                            0 // Use epoch as fallback
                        }),
                    cpu_cores: registration.cpu_cores,
                    memory_mb: registration.memory_mb,
                    active_jobs: 0,
                    total_jobs_completed: 0,
                    metadata: registration.metadata,
                })
            })?;

        Ok(worker)
    }

    /// Send worker heartbeat
    ///
    /// Called periodically by workers to indicate they're alive and update their status.
    pub async fn send_heartbeat(&self, worker_id: &str, heartbeat: WorkerHeartbeat) -> Result<()> {
        let path = format!("/workers/{}/heartbeat", worker_id);
        let response = self.post(&path, &heartbeat).await?;

        if response.status != 200 {
            return Err(anyhow!(
                "Failed to send heartbeat for worker {}: HTTP {}",
                worker_id,
                response.status
            ));
        }

        Ok(())
    }

    /// List all workers
    pub async fn list_workers(&self) -> Result<Vec<Worker>> {
        let response = self.get("/workers").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to list workers: HTTP {}", response.status));
        }

        let workers: Vec<Worker> = serde_json::from_slice(&response.body)?;
        Ok(workers)
    }

    /// Get worker details
    pub async fn get_worker(&self, worker_id: &str) -> Result<Option<Worker>> {
        let path = format!("/workers/{}", worker_id);
        let response = self.get(&path).await?;

        if response.status == 404 {
            return Ok(None);
        }

        if response.status != 200 {
            return Err(anyhow!(
                "Failed to get worker {}: HTTP {}",
                worker_id,
                response.status
            ));
        }

        let worker: Worker = serde_json::from_slice(&response.body)?;
        Ok(Some(worker))
    }

    /// Mark worker as draining (graceful shutdown)
    pub async fn drain_worker(&self, worker_id: &str) -> Result<()> {
        let path = format!("/workers/{}/drain", worker_id);
        let response = self.post(&path, &()).await?;

        if response.status != 200 {
            return Err(anyhow!(
                "Failed to drain worker {}: HTTP {}",
                worker_id,
                response.status
            ));
        }

        Ok(())
    }

    /// Get worker pool statistics
    pub async fn worker_stats(&self) -> Result<WorkerStats> {
        let response = self.get("/workers/stats").await?;

        if response.status != 200 {
            return Err(anyhow!("Failed to get worker stats: HTTP {}", response.status));
        }

        let stats: WorkerStats = serde_json::from_slice(&response.body)?;
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
        tracing::debug!(method = method, path = path, "Starting HTTP request");

        // Connect to control plane via iroh P2P
        tracing::debug!("Connecting to control plane via iroh");
        let conn = self.endpoint
            .connect(self.control_plane_addr.clone(), b"iroh+h3")
            .await
            .map_err(|e| anyhow!("Failed to connect to control plane: {}", e))?;
        tracing::debug!("iroh connection established");

        // Create HTTP/3 connection
        tracing::debug!("Creating h3 connection");
        let conn = h3_iroh::Connection::new(conn);
        let (mut driver, mut send_request) = h3::client::new(conn)
            .await
            .map_err(|e| anyhow!("Failed to create h3 client: {}", e))?;
        tracing::debug!("h3 client created successfully");

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

        // Spawn driver task in background - it will keep connection alive
        // We don't wait for it to complete, just let it drive the connection
        tracing::debug!("Spawning driver task in background");
        tokio::spawn(async move {
            tracing::debug!("drive_task: Starting to drive connection");
            let err = future::poll_fn(|cx| driver.poll_close(cx)).await;
            tracing::debug!("drive_task: poll_close completed");
            match err {
                h3::error::ConnectionError::Local { ref error, .. } => {
                    if matches!(error, h3::error::LocalError::Closing { .. }) {
                        tracing::debug!("drive_task: Connection closed normally");
                    } else {
                        tracing::warn!("drive_task: Local error - {:?}", error);
                    }
                }
                _ => {
                    tracing::warn!("drive_task: Connection error - {:?}", err);
                }
            }
            tracing::debug!("drive_task: Exiting");
        });

        // Wait for response (driver runs independently)
        tracing::debug!("Waiting for response");
        response_fut.await
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}
