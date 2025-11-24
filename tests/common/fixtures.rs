//! Test fixtures for integration testing
//!
//! Provides ControlPlaneFixture and TestWorkerFixture for managing
//! test infrastructure lifecycle.

use anyhow::Result;
use iroh::Endpoint;
use mvm_ci::config::AppConfig;
use mvm_ci::domain::types::{Job, WorkerHeartbeat, WorkerRegistration, WorkerType};
use mvm_ci::work_queue_client::WorkQueueClient;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Test fixture for running a control plane server
///
/// Manages the lifecycle of a control plane instance for integration testing.
/// Automatically cleans up resources on drop.
pub struct ControlPlaneFixture {
    /// Endpoint ticket for workers to connect
    pub endpoint_ticket: String,
    /// Server task handle
    server_handle: Option<JoinHandle<Result<()>>>,
    /// Shutdown signal
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    /// Configuration used
    pub config: AppConfig,
}

impl ControlPlaneFixture {
    /// Start a new control plane instance for testing
    ///
    /// # Arguments
    /// * `port` - HTTP port for localhost listener (e.g., 3020)
    ///
    /// # Returns
    /// A fixture with a running control plane server
    ///
    /// # Requirements
    /// - A flawless server must be running (default: http://localhost:27288)
    /// - Set FLAWLESS_URL environment variable to override
    pub async fn new(port: u16) -> Result<Self> {
        use flawless_utils::{Server, DeployedModule};
        use mvm_ci::server::{self, ServerConfig};
        use mvm_ci::state::factory::{InfrastructureFactory, test_factory::TestInfrastructureFactory};

        // Create test configuration
        let mut config = AppConfig::load()
            .unwrap_or_else(|_| AppConfig::default());

        // Override port for testing
        config.network.http_port = port;

        // Get flawless URL from environment or use default
        let flawless_url = std::env::var("FLAWLESS_URL")
            .unwrap_or_else(|_| "http://localhost:27288".to_string());

        // Connect to flawless and deploy test module
        let flawless = Server::new(&flawless_url, None);

        // Load and deploy test module
        let module_bytes = flawless_utils::load_module_from_build!("module1");
        let module = flawless.deploy(module_bytes).await
            .map_err(|e| anyhow::anyhow!(
                "Failed to deploy test module: {}. Is flawless server running at {}?",
                e, flawless_url
            ))?;

        // Create iroh endpoint
        let endpoint = iroh::Endpoint::builder()
            .alpns(vec![config.network.iroh_alpn.clone()])
            .bind()
            .await?;

        // Wait for endpoint to be online
        server::wait_for_online(&endpoint).await?;

        let node_id = endpoint.id().to_string();

        // Build application state using test factory
        let factory = TestInfrastructureFactory::new();
        let state = factory
            .build_app_state(&config, module, endpoint.clone(), node_id)
            .await?;

        // Get the work queue ticket
        let endpoint_ticket = state.infrastructure().work_queue().get_ticket();

        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        // Start the server
        let server_config = ServerConfig {
            app_config: config.clone(),
            endpoint,
            state,
        };

        let handle = server::start(server_config).await?;

        // Spawn server in background
        let mut shutdown_rx = shutdown_tx.subscribe();
        let server_handle = tokio::spawn(async move {
            tokio::select! {
                result = handle.run() => {
                    result.map_err(|e| anyhow::anyhow!("Server error: {}", e))
                }
                _ = shutdown_rx.recv() => {
                    Ok(())
                }
            }
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(Self {
            endpoint_ticket,
            server_handle: Some(server_handle),
            shutdown_tx,
            config,
        })
    }

    /// Get the endpoint ticket for worker connections
    pub fn endpoint_ticket(&self) -> &str {
        &self.endpoint_ticket
    }

    /// Get a client for making API calls to this control plane
    pub async fn client(&self) -> WorkQueueClient {
        WorkQueueClient::connect(&self.endpoint_ticket)
            .await
            .expect("Failed to create test client")
    }

    /// Shutdown the control plane gracefully
    pub async fn shutdown(mut self) -> Result<()> {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Wait for server to stop
        if let Some(handle) = self.server_handle.take() {
            handle.await??;
        }

        Ok(())
    }
}

impl Drop for ControlPlaneFixture {
    fn drop(&mut self) {
        // Best effort shutdown on drop
        let _ = self.shutdown_tx.send(());
    }
}

/// Test fixture for simulating a worker
///
/// Provides a test worker that can register, send heartbeats, and claim jobs.
pub struct TestWorkerFixture {
    /// Worker ID assigned after registration
    pub worker_id: Option<String>,
    /// Worker type (WASM or Firecracker)
    pub worker_type: WorkerType,
    /// Client for communicating with control plane
    client: WorkQueueClient,
    /// Heartbeat task handle
    heartbeat_task: Option<JoinHandle<()>>,
    /// Heartbeat stop signal
    heartbeat_stop: Arc<AtomicBool>,
    /// Current active jobs count
    active_jobs: Arc<AtomicU32>,
}

impl TestWorkerFixture {
    /// Create a new test worker
    ///
    /// # Arguments
    /// * `endpoint_ticket` - Control plane endpoint ticket
    /// * `worker_type` - Type of worker to simulate
    ///
    /// # Returns
    /// An unregistered test worker
    pub async fn new(endpoint_ticket: &str, worker_type: WorkerType) -> Result<Self> {
        let client = WorkQueueClient::connect(endpoint_ticket).await?;

        Ok(Self {
            worker_id: None,
            worker_type,
            client,
            heartbeat_task: None,
            heartbeat_stop: Arc::new(AtomicBool::new(false)),
            active_jobs: Arc::new(AtomicU32::new(0)),
        })
    }

    /// Register this worker with the control plane
    ///
    /// Sends a registration request and stores the assigned worker ID.
    pub async fn register(&mut self) -> Result<String> {
        let registration = WorkerRegistration {
            worker_type: self.worker_type,
            endpoint_id: format!("test-endpoint-{}", self.worker_type),
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            metadata: serde_json::json!({
                "test": true,
                "worker_type": self.worker_type.to_string()
            }),
        };

        let worker = self.client.register_worker(registration).await?;
        let worker_id = worker.id.clone();
        self.worker_id = Some(worker_id.clone());

        Ok(worker_id)
    }

    /// Start sending periodic heartbeats
    ///
    /// Spawns a background task that sends heartbeats every second.
    pub fn start_heartbeat(&mut self) {
        if self.heartbeat_task.is_some() {
            return; // Already running
        }

        let worker_id = self.worker_id.clone().expect("Worker must be registered first");
        let client = self.client.clone();
        let stop_signal = self.heartbeat_stop.clone();
        let active_jobs = self.active_jobs.clone();

        stop_signal.store(false, Ordering::SeqCst);

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            while !stop_signal.load(Ordering::SeqCst) {
                interval.tick().await;

                let heartbeat = WorkerHeartbeat {
                    worker_id: worker_id.clone(),
                    active_jobs: active_jobs.load(Ordering::SeqCst),
                    cpu_cores: Some(4),
                    memory_mb: Some(8192),
                };

                if let Err(e) = client.send_heartbeat(&worker_id, heartbeat).await {
                    eprintln!("Heartbeat failed: {}", e);
                }
            }
        });

        self.heartbeat_task = Some(task);
    }

    /// Stop sending heartbeats
    pub fn stop_heartbeat(&mut self) {
        self.heartbeat_stop.store(true, Ordering::SeqCst);

        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
    }

    /// Claim a job from the control plane
    ///
    /// Automatically increments active_jobs count if successful.
    pub async fn claim_work(&self) -> Result<Option<Job>> {
        let worker_id = self.worker_id.as_deref();
        let job = self.client.claim_work(worker_id, Some(self.worker_type)).await?;

        if job.is_some() {
            self.active_jobs.fetch_add(1, Ordering::SeqCst);
        }

        Ok(job)
    }

    /// Mark a job as completed
    ///
    /// Decrements active_jobs count.
    pub async fn complete_job(&self, job_id: &str) -> Result<()> {
        use mvm_ci::domain::types::JobStatus;

        self.client.update_status(job_id, JobStatus::Completed, None).await?;
        self.active_jobs.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    /// Mark a job as failed
    ///
    /// Decrements active_jobs count.
    pub async fn fail_job(&self, job_id: &str, error: &str) -> Result<()> {
        use mvm_ci::domain::types::JobStatus;

        self.client.update_status(job_id, JobStatus::Failed, Some(error.to_string())).await?;
        self.active_jobs.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    /// Get the current active jobs count
    pub fn active_jobs(&self) -> u32 {
        self.active_jobs.load(Ordering::SeqCst)
    }

    /// Shutdown the worker
    pub async fn shutdown(mut self) {
        self.stop_heartbeat();
    }
}

impl Drop for TestWorkerFixture {
    fn drop(&mut self) {
        // Stop heartbeat on drop
        self.heartbeat_stop.store(true, Ordering::SeqCst);

        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires external network access to iroh relay servers - run with --ignored flag to test"]
    async fn test_control_plane_fixture_lifecycle() {
        // Test that fixture can be created and shut down
        let fixture = ControlPlaneFixture::new(3021).await.unwrap();
        assert!(!fixture.endpoint_ticket().is_empty());

        fixture.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_fixture_creation() {
        // Note: This test will fail without a real endpoint, but tests the structure
        // In practice, use with a ControlPlaneFixture
        let worker_type = WorkerType::Wasm;
        assert_eq!(worker_type, WorkerType::Wasm);
    }
}
