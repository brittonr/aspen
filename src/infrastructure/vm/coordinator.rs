// VmCoordinator - Message-based orchestrator for VM management components
//
// The coordinator is responsible for:
// - Receiving commands from the VmManager facade
// - Routing commands to the appropriate component
// - Handling events from components
// - Managing component lifecycle (startup/shutdown)
//
// All components communicate via channels to maintain loose coupling.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::health_checker::{HealthChecker, HealthCheckConfig};
use super::job_router::JobRouter;
use super::messages::{
    HealthCommand, HealthEvent, MonitorCommand, MonitoringEvent, RegistryEvent, RouterRequest,
    VmCommand, VmEvent,
};
use super::resource_monitor::ResourceMonitor;
use super::vm_controller::VmController;
use super::vm_registry::VmRegistry;
use super::VmManagerConfig;
use crate::hiqlite::HiqliteService;

/// Channel buffer sizes for message passing
const COMMAND_BUFFER_SIZE: usize = 100;
const EVENT_BUFFER_SIZE: usize = 1000;

/// VmCoordinator orchestrates all VM management components via message passing
pub struct VmCoordinator {
    // Component instances (private - not exposed externally)
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
    router: Arc<JobRouter>,
    monitor: Arc<ResourceMonitor>,
    health_checker: Arc<HealthChecker>,

    // Command channels (for sending commands to components)
    vm_cmd_tx: mpsc::Sender<VmCommand>,
    router_cmd_tx: mpsc::Sender<RouterRequest>,
    monitor_cmd_tx: mpsc::Sender<MonitorCommand>,
    health_cmd_tx: mpsc::Sender<HealthCommand>,

    // Event channels (for receiving events from components)
    vm_event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<VmEvent>>>,
    registry_event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<RegistryEvent>>>,
    monitoring_event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MonitoringEvent>>>,
    health_event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<HealthEvent>>>,

    // Background task handles for graceful shutdown
    task_handles: Arc<tokio::sync::Mutex<Vec<JoinHandle<()>>>>,

    config: VmManagerConfig,
}

impl VmCoordinator {
    /// Create a new VmCoordinator with all components
    pub async fn new(config: VmManagerConfig, hiqlite: Arc<HiqliteService>) -> Result<Self> {
        // Initialize registry with Hiqlite persistence
        let registry = Arc::new(VmRegistry::new(hiqlite, &config.state_dir).await?);

        // Create controller for VM lifecycle operations
        let controller = Arc::new(VmController::new(config.clone(), Arc::clone(&registry))?);

        // Initialize job router
        let router = Arc::new(JobRouter::new(
            Arc::clone(&registry),
            Arc::clone(&controller),
        ));

        // Set up resource monitor
        let monitor = Arc::new(ResourceMonitor::new(
            Arc::clone(&registry),
            Arc::clone(&controller),
        ));

        // Initialize health checker
        let health_config = HealthCheckConfig::default();
        let health_checker = Arc::new(HealthChecker::new(Arc::clone(&registry), health_config));

        // Create command channels
        // Note: Command receivers are unused for now - they will be used when
        // components are fully migrated to message-based communication
        let (vm_cmd_tx, _vm_cmd_rx) = mpsc::channel(COMMAND_BUFFER_SIZE);
        let (router_cmd_tx, _router_cmd_rx) = mpsc::channel(COMMAND_BUFFER_SIZE);
        let (monitor_cmd_tx, _monitor_cmd_rx) = mpsc::channel(COMMAND_BUFFER_SIZE);
        let (health_cmd_tx, _health_cmd_rx) = mpsc::channel(COMMAND_BUFFER_SIZE);

        // Create event channels
        // Note: Event senders are unused for now - they will be used when
        // components are fully migrated to emit events
        let (_vm_event_tx, vm_event_rx) = mpsc::channel(EVENT_BUFFER_SIZE);
        let (_registry_event_tx, registry_event_rx) = mpsc::channel(EVENT_BUFFER_SIZE);
        let (_monitoring_event_tx, monitoring_event_rx) = mpsc::channel(EVENT_BUFFER_SIZE);
        let (_health_event_tx, health_event_rx) = mpsc::channel(EVENT_BUFFER_SIZE);

        Ok(Self {
            registry,
            controller,
            router,
            monitor,
            health_checker,
            vm_cmd_tx,
            router_cmd_tx,
            monitor_cmd_tx,
            health_cmd_tx,
            vm_event_rx: Arc::new(tokio::sync::Mutex::new(vm_event_rx)),
            registry_event_rx: Arc::new(tokio::sync::Mutex::new(registry_event_rx)),
            monitoring_event_rx: Arc::new(tokio::sync::Mutex::new(monitoring_event_rx)),
            health_event_rx: Arc::new(tokio::sync::Mutex::new(health_event_rx)),
            task_handles: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            config,
        })
    }

    /// Start all background tasks
    pub async fn start_background_tasks(&self) -> Result<()> {
        tracing::info!("Starting VmCoordinator background tasks");

        let mut handles = self.task_handles.lock().await;

        // Start VM event processor
        let vm_event_rx = Arc::clone(&self.vm_event_rx);
        let handle = tokio::spawn(async move {
            Self::process_vm_events(vm_event_rx).await;
        });
        handles.push(handle);

        // Start registry event processor
        let registry_event_rx = Arc::clone(&self.registry_event_rx);
        let handle = tokio::spawn(async move {
            Self::process_registry_events(registry_event_rx).await;
        });
        handles.push(handle);

        // Start monitoring event processor
        let monitoring_event_rx = Arc::clone(&self.monitoring_event_rx);
        let controller = Arc::clone(&self.controller);
        let handle = tokio::spawn(async move {
            Self::process_monitoring_events(monitoring_event_rx, controller).await;
        });
        handles.push(handle);

        // Start health event processor
        let health_event_rx = Arc::clone(&self.health_event_rx);
        let registry = Arc::clone(&self.registry);
        let handle = tokio::spawn(async move {
            Self::process_health_events(health_event_rx, registry).await;
        });
        handles.push(handle);

        // Start health checking loop
        let health_checker = Arc::clone(&self.health_checker);
        let handle = tokio::spawn(async move {
            health_checker.health_check_loop().await;
        });
        handles.push(handle);

        // Start resource monitoring loop
        let monitor = Arc::clone(&self.monitor);
        let handle = tokio::spawn(async move {
            monitor.monitoring_loop().await;
        });
        handles.push(handle);

        // Pre-warm VMs if configured
        if self.config.auto_scaling && self.config.pre_warm_count > 0 {
            self.pre_warm_vms().await?;
        }

        // Recover any VMs from previous run
        self.recover_existing_vms().await?;

        tracing::info!("VmCoordinator background tasks started");
        Ok(())
    }

    /// Send a VM command and wait for response
    pub async fn send_vm_command(&self, cmd: VmCommand) -> Result<()> {
        self.vm_cmd_tx
            .send(cmd)
            .await
            .map_err(|e| anyhow!("Failed to send VM command: {}", e))
    }

    /// Send a router request and wait for response
    pub async fn send_router_request(&self, req: RouterRequest) -> Result<()> {
        self.router_cmd_tx
            .send(req)
            .await
            .map_err(|e| anyhow!("Failed to send router request: {}", e))
    }

    /// Send a monitor command
    pub async fn send_monitor_command(&self, cmd: MonitorCommand) -> Result<()> {
        self.monitor_cmd_tx
            .send(cmd)
            .await
            .map_err(|e| anyhow!("Failed to send monitor command: {}", e))
    }

    /// Send a health command
    pub async fn send_health_command(&self, cmd: HealthCommand) -> Result<()> {
        self.health_cmd_tx
            .send(cmd)
            .await
            .map_err(|e| anyhow!("Failed to send health command: {}", e))
    }

    /// Process VM lifecycle events
    async fn process_vm_events(
        event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<VmEvent>>>,
    ) {
        let mut rx = event_rx.lock().await;

        while let Some(event) = rx.recv().await {
            match event {
                VmEvent::Started { vm_id, config } => {
                    tracing::info!(
                        vm_id = %vm_id,
                        mode = ?config.mode,
                        "VM started event received"
                    );
                }
                VmEvent::Stopped { vm_id, reason } => {
                    tracing::info!(vm_id = %vm_id, reason = %reason, "VM stopped event received");
                }
                VmEvent::Failed { vm_id, error } => {
                    tracing::error!(vm_id = %vm_id, error = %error, "VM failed event received");
                }
                VmEvent::JobAssigned { vm_id, job_id } => {
                    tracing::debug!(
                        vm_id = %vm_id,
                        job_id = %job_id,
                        "Job assigned event received"
                    );
                }
                VmEvent::JobCompleted {
                    vm_id,
                    job_id,
                    success,
                } => {
                    tracing::info!(
                        vm_id = %vm_id,
                        job_id = %job_id,
                        success = success,
                        "Job completed event received"
                    );
                }
                VmEvent::StateChanged {
                    vm_id,
                    old_state,
                    new_state,
                } => {
                    tracing::debug!(
                        vm_id = %vm_id,
                        old_state = ?old_state,
                        new_state = ?new_state,
                        "VM state changed event received"
                    );
                }
            }
        }
    }

    /// Process registry events
    async fn process_registry_events(
        event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<RegistryEvent>>>,
    ) {
        let mut rx = event_rx.lock().await;

        while let Some(event) = rx.recv().await {
            match event {
                RegistryEvent::VmRegistered { vm_id, config } => {
                    tracing::debug!(
                        vm_id = %vm_id,
                        mode = ?config.mode,
                        "VM registered in registry"
                    );
                }
                RegistryEvent::VmStateUpdated {
                    vm_id,
                    old_state,
                    new_state,
                } => {
                    tracing::debug!(
                        vm_id = %vm_id,
                        old_state = ?old_state,
                        new_state = ?new_state,
                        "VM state updated in registry"
                    );
                }
                RegistryEvent::VmMetricsUpdated { vm_id, .. } => {
                    tracing::trace!(vm_id = %vm_id, "VM metrics updated in registry");
                }
                RegistryEvent::VmRemoved { vm_id } => {
                    tracing::debug!(vm_id = %vm_id, "VM removed from registry");
                }
            }
        }
    }

    /// Process monitoring events and take action
    async fn process_monitoring_events(
        event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MonitoringEvent>>>,
        controller: Arc<VmController>,
    ) {
        let mut rx = event_rx.lock().await;

        while let Some(event) = rx.recv().await {
            match event {
                MonitoringEvent::ResourceThreshold {
                    vm_id,
                    resource,
                    current,
                    threshold,
                } => {
                    tracing::warn!(
                        vm_id = %vm_id,
                        resource = ?resource,
                        current = current,
                        threshold = threshold,
                        "Resource threshold exceeded"
                    );
                }
                MonitoringEvent::VmShouldRecycle { vm_id, reason } => {
                    tracing::info!(vm_id = %vm_id, reason = ?reason, "VM should be recycled");

                    // Shutdown the VM
                    if let Err(e) = controller.shutdown_vm(vm_id, true).await {
                        tracing::error!(
                            vm_id = %vm_id,
                            error = %e,
                            "Failed to shutdown VM for recycling"
                        );
                    }
                }
                MonitoringEvent::IdleVmDetected {
                    vm_id,
                    idle_duration_secs,
                } => {
                    tracing::info!(
                        vm_id = %vm_id,
                        idle_secs = idle_duration_secs,
                        "Idle VM detected"
                    );
                }
                MonitoringEvent::AutoScalingTriggered { action, count } => {
                    tracing::info!(
                        action = ?action,
                        count = count,
                        "Auto-scaling triggered"
                    );
                }
            }
        }
    }

    /// Process health events and take action
    async fn process_health_events(
        event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<HealthEvent>>>,
        registry: Arc<VmRegistry>,
    ) {
        let mut rx = event_rx.lock().await;

        while let Some(event) = rx.recv().await {
            match event {
                HealthEvent::HealthCheckPassed {
                    vm_id,
                    response_time_ms,
                } => {
                    tracing::trace!(
                        vm_id = %vm_id,
                        response_ms = response_time_ms,
                        "Health check passed"
                    );
                }
                HealthEvent::HealthCheckFailed {
                    vm_id,
                    error,
                    failures,
                } => {
                    tracing::warn!(
                        vm_id = %vm_id,
                        error = %error,
                        failures = failures,
                        "Health check failed"
                    );
                }
                HealthEvent::VmUnhealthy {
                    vm_id,
                    failures,
                    error,
                } => {
                    tracing::error!(
                        vm_id = %vm_id,
                        failures = failures,
                        error = %error,
                        "VM marked as unhealthy"
                    );

                    // Update VM state in registry
                    if let Err(e) = registry
                        .update_state(
                            vm_id,
                            super::vm_types::VmState::Failed {
                                error: format!("Health check failures: {}", error),
                            },
                        )
                        .await
                    {
                        tracing::error!(
                            vm_id = %vm_id,
                            error = %e,
                            "Failed to update VM state to failed"
                        );
                    }
                }
                HealthEvent::VmRecovered { vm_id } => {
                    tracing::info!(vm_id = %vm_id, "VM recovered to healthy state");
                }
                HealthEvent::CircuitBreakerOpened {
                    vm_id,
                    duration_secs,
                } => {
                    tracing::warn!(
                        vm_id = %vm_id,
                        duration_secs = duration_secs,
                        "Circuit breaker opened for VM"
                    );
                }
            }
        }
    }

    /// Pre-warm idle VMs for faster job execution
    async fn pre_warm_vms(&self) -> Result<()> {
        tracing::info!(count = self.config.pre_warm_count, "Pre-warming service VMs");

        for i in 0..self.config.pre_warm_count {
            let config = super::vm_types::VmConfig::default_service();

            match self.controller.start_vm(config).await {
                Ok(vm) => {
                    tracing::info!(
                        vm_id = %vm.config.id,
                        index = i + 1,
                        "Pre-warmed service VM started"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        index = i + 1,
                        "Failed to pre-warm service VM"
                    );
                }
            }
        }

        Ok(())
    }

    /// Recover VMs from previous run (on restart)
    async fn recover_existing_vms(&self) -> Result<()> {
        let recovered = self.registry.recover_from_persistence().await?;

        if recovered > 0 {
            tracing::info!(count = recovered, "Recovered VMs from previous run");

            // Mark stale VMs as terminated
            self.registry.cleanup_stale_vms().await?;
        }

        Ok(())
    }

    /// Gracefully shutdown the coordinator and all components
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down VmCoordinator");

        // Stop accepting new jobs
        self.router.stop_routing().await;

        // Stop monitoring and health checking
        self.health_checker.stop().await;

        // Gracefully shutdown all VMs
        let all_vms = self.registry.list_all_vms().await?;
        for vm in all_vms {
            if !matches!(vm.state, super::vm_types::VmState::Terminated { .. }) {
                self.controller.shutdown_vm(vm.config.id, true).await?;
            }
        }

        // Cancel all background tasks
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        tracing::info!("VmCoordinator shutdown complete");
        Ok(())
    }

    /// Get direct access to registry (for facade compatibility)
    pub fn registry(&self) -> Arc<VmRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get direct access to controller (for facade compatibility)
    pub fn controller(&self) -> Arc<VmController> {
        Arc::clone(&self.controller)
    }

    /// Get direct access to router (for facade compatibility)
    pub fn router(&self) -> Arc<JobRouter> {
        Arc::clone(&self.router)
    }

    /// Get direct access to monitor (for facade compatibility)
    pub fn monitor(&self) -> Arc<ResourceMonitor> {
        Arc::clone(&self.monitor)
    }

    /// Get direct access to health checker (for facade compatibility)
    pub fn health_checker(&self) -> Arc<HealthChecker> {
        Arc::clone(&self.health_checker)
    }

    /// Get config reference
    pub fn config(&self) -> &VmManagerConfig {
        &self.config
    }
}
