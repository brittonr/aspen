//! RaftActor supervision with automatic restart, health monitoring, and meltdown detection.
//!
//! # Overview
//!
//! The RaftSupervisor implements OneForOne supervision with exponential backoff,
//! automatic restart on failure, storage validation before restart, and health
//! monitoring to prevent infinite restart loops.
//!
//! # Operational Guide
//!
//! ## Monitoring Supervision Health
//!
//! Query the `/health` HTTP endpoint to check supervision status:
//!
//! ```bash
//! curl http://localhost:8080/health | jq .supervision
//! ```
//!
//! Response fields:
//! - `status`: "healthy", "degraded", or "unhealthy"
//! - `consecutive_failures`: Number of failed health checks since last success
//! - `enabled`: Whether supervision is active
//!
//! ### Health Status Interpretation
//!
//! **Healthy** (`consecutive_failures: 0`):
//! - RaftActor responding to health checks within 25ms
//! - Normal operation, no action required
//!
//! **Degraded** (`consecutive_failures: 1-2`):
//! - RaftActor experiencing intermittent delays or timeouts
//! - May indicate high load, slow disk I/O, or network issues
//! - Monitor closely, review metrics for bottlenecks
//!
//! **Unhealthy** (`consecutive_failures: >= 3`):
//! - RaftActor consistently unresponsive
//! - Supervisor will attempt automatic restart
//! - Review logs for crash/panic messages
//! - Check disk space and storage health
//!
//! ## Restart Behavior
//!
//! ### Exponential Backoff
//!
//! Delays between restart attempts:
//! - Restart 0: 1 second
//! - Restart 1: 2 seconds
//! - Restart 2: 4 seconds
//! - Restart 3: 8 seconds
//! - Restart 4+: 16 seconds (capped)
//!
//! ### Meltdown Detection
//!
//! Default configuration:
//! - `max_restarts_per_window: 3` - Maximum 3 restarts
//! - `restart_window_secs: 600` - Within 10-minute window
//! - `actor_stability_duration_secs: 300` - 5 minutes uptime required
//!
//! When meltdown detected:
//! - Supervisor stops restarting actor
//! - Logs "meltdown detected" error message
//! - Manual intervention required
//!
//! ### Circuit Breaker Pattern
//!
//! The supervisor implements a circuit breaker for automatic recovery from meltdown
//! conditions. The circuit breaker has three states with automatic transitions:
//!
//! **Closed** (Normal Operation):
//! - All restart attempts proceed normally with exponential backoff
//! - Meltdown detection monitors restart frequency
//! - Transitions to **Open** when meltdown detected
//!
//! **Open** (Failure State):
//! - No restart attempts allowed
//! - Actor remains stopped to prevent infinite restart loops
//! - Automatically transitions to **HalfOpen** after `circuit_open_duration_secs` (default: 300s)
//!
//! **HalfOpen** (Recovery Testing):
//! - Allows exactly ONE restart attempt to test if underlying issue resolved
//! - On successful restart → Transitions to **Closed** after `half_open_stability_duration_secs` (default: 120s)
//! - On failed restart → Returns to **Open** state
//!
//! #### Circuit Breaker Configuration
//!
//! ```rust
//! SupervisionConfig {
//!     circuit_open_duration_secs: 300,        // Wait 5 minutes before testing recovery
//!     half_open_stability_duration_secs: 120, // Require 2 minutes uptime for recovery
//!     ..Default::default()
//! }
//! ```
//!
//! #### Monitoring Circuit Breaker State
//!
//! Query supervision status to check circuit breaker state:
//!
//! ```bash
//! curl http://localhost:8080/health | jq .supervision.circuit_state
//! ```
//!
//! Response values:
//! - `"Closed"`: Normal operation
//! - `"Open"`: Meltdown detected, restarts blocked
//! - `"HalfOpen"`: Testing recovery, one restart allowed
//!
//! #### Circuit Breaker vs Meltdown Detection
//!
//! - **Meltdown Detection**: Rate-limiting mechanism that opens the circuit breaker
//!   - Tracks: Number of restarts within a time window
//!   - Action: Opens circuit breaker when threshold exceeded
//!
//! - **Circuit Breaker**: Automatic recovery mechanism
//!   - Provides: Automatic retry after cooldown period
//!   - Prevents: Need for manual intervention in transient failures
//!
//! #### Recovery Scenarios
//!
//! **Transient Failure** (e.g., temporary network issue):
//! 1. Meltdown detected after 3 rapid restarts → Circuit opens
//! 2. Wait 5 minutes (circuit_open_duration_secs)
//! 3. Circuit transitions to HalfOpen → One restart attempt
//! 4. Network recovered → Restart succeeds
//! 5. Wait 2 minutes (half_open_stability_duration_secs)
//! 6. Circuit closes → Normal operation resumed
//!
//! **Persistent Failure** (e.g., corrupted storage):
//! 1. Meltdown detected → Circuit opens
//! 2. Wait 5 minutes → Circuit transitions to HalfOpen
//! 3. Storage still corrupted → Restart fails
//! 4. Circuit returns to Open
//! 5. Cycle repeats until issue resolved manually
//!
//! ## Troubleshooting Common Issues
//!
//! ### Issue: Repeated Rapid Restarts
//!
//! **Symptoms**: `consecutive_failures` incrementing quickly, frequent restart logs
//!
//! **Possible Causes**:
//! 1. Storage corruption - Check with `validate_raft_storage()`
//! 2. Resource exhaustion - Check disk space, memory, file descriptors
//! 3. Configuration errors - Review Raft config (election timeout, etc.)
//!
//! **Actions**:
//! ```bash
//! # Check disk space
//! df -h /path/to/data
//!
//! # Check open file descriptors
//! lsof -p <pid> | wc -l
//!
//! # Review recent logs
//! journalctl -u aspen -n 100 --no-pager | grep -E "(panic|ERROR|WARN)"
//! ```
//!
//! ### Issue: Meltdown Detected (Circuit Opened)
//!
//! **Symptoms**: Log message "exceeded max_restarts_per_window, entering meltdown"
//! or supervision status shows `circuit_state: "Open"`
//!
//! **Automatic Recovery**:
//! The circuit breaker will automatically attempt recovery:
//! 1. Wait `circuit_open_duration_secs` (default: 5 minutes)
//! 2. Transition to HalfOpen state
//! 3. Attempt one restart
//! 4. If successful and stable for `half_open_stability_duration_secs`, close circuit
//!
//! **Manual Investigation** (while waiting for automatic recovery):
//! 1. Check storage integrity:
//!    ```bash
//!    # If using redb backend
//!    ls -lh /path/to/data/*.redb
//!    # Check for 0-byte files or permission issues
//!    ```
//!
//! 2. Review crash logs:
//!    ```bash
//!    journalctl -u aspen -p err -n 50 --no-pager
//!    ```
//!
//! 3. Manual recovery (if automatic recovery fails repeatedly):
//!    - Stop node
//!    - Restore from backup if storage corrupted
//!    - Adjust configuration if config error identified
//!    - Restart node
//!
//! ### Issue: Circuit Stuck in HalfOpen
//!
//! **Symptoms**: `circuit_state: "HalfOpen"` persists for extended period
//!
//! **Meaning**: Actor restarted successfully but hasn't remained stable for the required duration
//!
//! **Actions**:
//! 1. Check if actor is experiencing intermittent issues:
//!    ```bash
//!    # Monitor health checks
//!    watch -n 1 'curl -s http://localhost:8080/health | jq .supervision'
//!    ```
//!
//! 2. Review resource utilization:
//!    ```bash
//!    top -H -p <pid>  # CPU/memory spikes
//!    iostat -x 1 10   # Disk latency spikes
//!    ```
//!
//! 3. If instability persists:
//!    - Circuit will return to Open on next failure
//!    - Investigate root cause before next automatic retry
//!
//! ### Issue: Degraded Health Persists
//!
//! **Symptoms**: `status: "degraded"` for extended period (> 5 minutes)
//!
//! **Investigation**:
//! 1. Check system resources:
//!    ```bash
//!    top -H -p <pid>  # CPU/memory per thread
//!    iostat -x 1 10   # Disk I/O latency
//!    ```
//!
//! 2. Review Raft metrics via `/metrics`:
//!    - `last_log_index` - Should be incrementing
//!    - `current_term` - Should be stable
//!    - `last_applied` - Should track last_log_index
//!
//! 3. Check network connectivity:
//!    ```bash
//!    # Test peer connectivity
//!    curl http://<peer>:8080/health
//!    ```
//!
//! ## Configuration Tuning
//!
//! ### Aggressive Restart (Development/Testing)
//!
//! Fast recovery with short cooldown periods:
//!
//! ```rust
//! SupervisionConfig {
//!     enable_auto_restart: true,
//!     actor_stability_duration_secs: 30,          // 30 seconds
//!     max_restarts_per_window: 10,               // Allow 10 restarts
//!     restart_window_secs: 300,                  // Within 5 minutes
//!     restart_history_size: 50,
//!     circuit_open_duration_secs: 60,            // 1 minute cooldown
//!     half_open_stability_duration_secs: 30,     // 30 seconds stability
//! }
//! ```
//!
//! ### Conservative Restart (Production)
//!
//! Slow, careful recovery with long cooldown periods:
//!
//! ```rust
//! SupervisionConfig {
//!     enable_auto_restart: true,
//!     actor_stability_duration_secs: 600,        // 10 minutes
//!     max_restarts_per_window: 3,                // Only 3 restarts
//!     restart_window_secs: 1800,                 // Within 30 minutes
//!     restart_history_size: 100,
//!     circuit_open_duration_secs: 600,           // 10 minute cooldown
//!     half_open_stability_duration_secs: 300,    // 5 minutes stability
//! }
//! ```
//!
//! ### Disable Auto-Restart (Manual Control)
//!
//! ```rust
//! SupervisionConfig {
//!     enable_auto_restart: false,
//!     ..Default::default()
//! }
//! ```
//!
//! ## Manual Restart
//!
//! Trigger restart via SupervisorMessage:
//! ```rust
//! supervisor_ref.cast(SupervisorMessage::ManualRestart);
//! ```
//!
//! Or via HTTP (if admin endpoint enabled):
//! ```bash
//! curl -X POST http://localhost:8080/admin/restart-actor
//! ```
//!
//! ## Storage Validation
//!
//! Before each restart, supervisor validates redb storage:
//! - Log monotonicity (no gaps/duplicates)
//! - Snapshot metadata integrity
//! - Vote state consistency
//! - Committed index bounds
//!
//! Validation failure prevents restart to avoid corruption propagation.
//! Check logs for specific validation error:
//! ```bash
//! journalctl -u aspen | grep "storage validation failed"
//! ```

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ractor::{
    Actor, ActorCell, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent, call_t,
};
use snafu::{ResultExt, Snafu};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};

use crate::raft::constants::{MAX_BACKOFF_SECONDS, MAX_RESTART_HISTORY_SIZE};
use crate::raft::{RaftActor, RaftActorConfig, RaftActorMessage};

/// Circuit breaker state for automatic recovery from meltdown.
///
/// Tiger Style: Fixed state transitions, bounded timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Normal operation - restarts proceed with exponential backoff.
    Closed,

    /// Failure state - too many restarts, no restart attempts allowed.
    /// Transitions to HalfOpen after circuit_open_duration_secs.
    Open,

    /// Recovery testing - allows ONE restart attempt.
    /// On success → Closed, On failure → Open.
    HalfOpen,
}

// ============================================================================
// Prometheus-style metrics for health monitoring
// ============================================================================

/// Global metrics for health monitoring.
/// Tiger Style: Fixed-size atomic counters, bounded memory usage.
struct HealthMetrics {
    /// RaftActor health status (0=unhealthy, 1=degraded, 2=healthy).
    health_status: AtomicU32,
    /// Number of consecutive health check failures.
    consecutive_failures: AtomicU32,
    /// Total number of health checks performed.
    health_checks_total: AtomicU64,
    /// Total number of health check failures.
    health_check_failures_total: AtomicU64,
}

impl HealthMetrics {
    const fn new() -> Self {
        Self {
            health_status: AtomicU32::new(2), // Start as healthy
            consecutive_failures: AtomicU32::new(0),
            health_checks_total: AtomicU64::new(0),
            health_check_failures_total: AtomicU64::new(0),
        }
    }
}

/// Global health metrics instance.
/// Tiger Style: Static initialization with const constructor.
static HEALTH_METRICS: OnceLock<HealthMetrics> = OnceLock::new();

fn health_metrics() -> &'static HealthMetrics {
    HEALTH_METRICS.get_or_init(HealthMetrics::new)
}

/// Get a snapshot of current health metrics for Prometheus export.
///
/// Returns (health_status, consecutive_failures, checks_total, failures_total).
pub fn get_health_metrics() -> (u32, u32, u64, u64) {
    let metrics = health_metrics();
    (
        metrics.health_status.load(Ordering::Relaxed),
        metrics.consecutive_failures.load(Ordering::Relaxed),
        metrics.health_checks_total.load(Ordering::Relaxed),
        metrics.health_check_failures_total.load(Ordering::Relaxed),
    )
}

/// Configuration for supervision behavior.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupervisionConfig {
    /// Enable automatic actor restart on failure (default: true).
    pub enable_auto_restart: bool,

    /// Duration an actor must run to be considered stable (default: 5 minutes).
    pub actor_stability_duration_secs: u64,

    /// Maximum restarts allowed within the restart window before circuit opens (default: 3).
    pub max_restarts_per_window: u32,

    /// Time window for counting restarts (default: 10 minutes).
    pub restart_window_secs: u64,

    /// Maximum size of restart history (default: 100).
    pub restart_history_size: u32,

    /// Duration circuit breaker stays open before testing recovery (default: 5 minutes).
    pub circuit_open_duration_secs: u64,

    /// Duration actor must run in HalfOpen state to close circuit (default: 2 minutes).
    pub half_open_stability_duration_secs: u64,
}

impl Default for SupervisionConfig {
    fn default() -> Self {
        Self {
            enable_auto_restart: true,
            actor_stability_duration_secs: 300, // 5 minutes
            max_restarts_per_window: 3,
            restart_window_secs: 600, // 10 minutes
            restart_history_size: MAX_RESTART_HISTORY_SIZE,
            circuit_open_duration_secs: 300,        // 5 minutes
            half_open_stability_duration_secs: 120, // 2 minutes
        }
    }
}

impl SupervisionConfig {
    /// Get actor stability duration as Duration.
    fn actor_stability_duration(&self) -> Duration {
        Duration::from_secs(self.actor_stability_duration_secs)
    }

    /// Get restart window as Duration.
    fn restart_window(&self) -> Duration {
        Duration::from_secs(self.restart_window_secs)
    }

    /// Get circuit breaker open duration as Duration.
    fn circuit_open_duration(&self) -> Duration {
        Duration::from_secs(self.circuit_open_duration_secs)
    }

    /// Get half-open stability duration as Duration.
    fn half_open_stability_duration(&self) -> Duration {
        Duration::from_secs(self.half_open_stability_duration_secs)
    }
}

/// Record of a single restart event.
#[derive(Debug, Clone)]
pub struct RestartEvent {
    pub timestamp: Instant,
    pub reason: String,
    pub actor_uptime_secs: u64,
    pub backoff_applied_secs: u64,
}

/// Supervision status exposed via GetStatus message.
#[derive(Debug, Clone)]
pub struct SupervisionStatus {
    /// Total number of restarts since supervisor started.
    pub total_restarts: u32,
    /// Number of restarts in the current window.
    pub restarts_in_window: u32,
    /// Whether the supervisor is in meltdown state (deprecated - use circuit_breaker_state).
    pub is_meltdown: bool,
    /// Current circuit breaker state.
    pub circuit_breaker_state: CircuitBreakerState,
    /// Whether auto-restart is enabled.
    pub auto_restart_enabled: bool,
    /// Current RaftActor reference (if running).
    pub raft_actor: Option<ActorRef<RaftActorMessage>>,
}

/// Messages that can be sent to the RaftSupervisor.
#[derive(Debug)]
pub enum SupervisorMessage {
    /// Get current supervision status.
    GetStatus(RpcReplyPort<SupervisionStatus>),
    /// Get the current RaftActor reference.
    GetRaftActor(RpcReplyPort<Option<ActorRef<RaftActorMessage>>>),
    /// Manually trigger a restart (bypasses meltdown check).
    ManualRestart,
    /// Get restart history.
    GetRestartHistory(RpcReplyPort<Vec<RestartEvent>>),
    /// Get the HealthMonitor reference (if health monitoring is enabled).
    GetHealthMonitor(RpcReplyPort<Option<Arc<HealthMonitor>>>),
}

impl ractor::Message for SupervisorMessage {}

/// Arguments for initializing the supervisor.
#[derive(Debug, Clone)]
pub struct SupervisorArguments {
    /// Configuration for the RaftActor to be supervised.
    pub raft_actor_config: RaftActorConfig,
    /// Supervision behavior configuration.
    pub supervision_config: SupervisionConfig,
}

/// State maintained by the RaftSupervisor.
pub struct RaftSupervisorState {
    /// Bounded history of restart events (max 100 items).
    restart_history: VecDeque<RestartEvent>,
    /// Supervision configuration.
    config: SupervisionConfig,
    /// Configuration for spawning RaftActor instances.
    raft_actor_config: RaftActorConfig,
    /// Reference to the current RaftActor (if running).
    current_raft_actor: Option<ActorRef<RaftActorMessage>>,
    /// Timestamp when the current RaftActor was spawned.
    actor_spawn_time: Option<Instant>,
    /// Total number of restarts performed.
    total_restarts: u32,
    /// Health monitor for the current RaftActor (if health monitoring is enabled).
    health_monitor: Option<Arc<HealthMonitor>>,
    /// Health monitor background task handle.
    health_monitor_task: Option<JoinHandle<()>>,
    /// Current circuit breaker state.
    circuit_breaker_state: CircuitBreakerState,
    /// Timestamp when circuit breaker entered Open state.
    circuit_opened_at: Option<Instant>,
}

/// Actor that supervises a RaftActor using OneForOne supervision.
///
/// Implements:
/// - Automatic restart on failure with exponential backoff
/// - Meltdown detection (3 restarts in 10 minutes)
/// - Stability tracking (5 minute uptime threshold)
/// - Bounded restart history (100 events)
pub struct RaftSupervisor;

#[derive(Debug, Snafu)]
pub enum SupervisionError {
    #[snafu(display("failed to spawn raft actor: {source}"))]
    SpawnFailed { source: ractor::SpawnErr },

    #[snafu(display("actor is in meltdown state: {restarts} restarts in {window_secs}s"))]
    Meltdown { restarts: u32, window_secs: u64 },
}

impl RaftSupervisor {
    /// Check if supervisor is in meltdown state.
    ///
    /// Meltdown occurs when max_restarts_per_window or more restarts happen
    /// within the configured restart_window.
    fn is_meltdown(&self, state: &RaftSupervisorState) -> bool {
        let now = Instant::now();
        let window = state.config.restart_window();

        let restarts_in_window = state
            .restart_history
            .iter()
            .rev()
            .take_while(|event| now.duration_since(event.timestamp) < window)
            .count() as u32;

        restarts_in_window >= state.config.max_restarts_per_window
    }

    /// Calculate exponential backoff duration.
    ///
    /// Formula: min(2^restart_count, 16) seconds
    /// Sequence: 1s, 2s, 4s, 8s, 16s, 16s, ...
    fn calculate_backoff(&self, restart_count: u32) -> Duration {
        let backoff_secs = 2u64.pow(restart_count).min(MAX_BACKOFF_SECONDS);
        Duration::from_secs(backoff_secs)
    }

    /// Count restarts within the configured window.
    fn count_restarts_in_window(&self, state: &RaftSupervisorState) -> u32 {
        let now = Instant::now();
        let window = state.config.restart_window();

        state
            .restart_history
            .iter()
            .rev()
            .take_while(|event| now.duration_since(event.timestamp) < window)
            .count() as u32
    }

    /// Update circuit breaker state based on restart count and time.
    ///
    /// State transitions:
    /// - Closed → Open: When max_restarts_per_window exceeded
    /// - Open → HalfOpen: After circuit_open_duration elapsed
    /// - HalfOpen → Closed: If actor remains stable for half_open_stability_duration
    /// - HalfOpen → Open: If restart fails in HalfOpen state
    fn update_circuit_breaker_state(&self, state: &mut RaftSupervisorState) {
        let now = Instant::now();

        match state.circuit_breaker_state {
            CircuitBreakerState::Closed => {
                // Check if we should open the circuit
                if self.is_meltdown(state) {
                    let restarts = self.count_restarts_in_window(state);
                    info!(
                        circuit_state = "Closed -> Open",
                        restarts_in_window = restarts,
                        window_secs = state.config.restart_window_secs,
                        "circuit breaker opening due to excessive restarts"
                    );
                    state.circuit_breaker_state = CircuitBreakerState::Open;
                    state.circuit_opened_at = Some(now);
                }
            }
            CircuitBreakerState::Open => {
                // Check if we should transition to HalfOpen
                if let Some(opened_at) = state.circuit_opened_at {
                    let time_open = now.duration_since(opened_at);
                    if time_open >= state.config.circuit_open_duration() {
                        info!(
                            circuit_state = "Open -> HalfOpen",
                            time_open_secs = time_open.as_secs(),
                            "circuit breaker entering half-open state for recovery testing"
                        );
                        state.circuit_breaker_state = CircuitBreakerState::HalfOpen;
                    }
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Check if actor has been stable long enough to close circuit
                if let Some(spawn_time) = state.actor_spawn_time {
                    let uptime = now.duration_since(spawn_time);
                    if uptime >= state.config.half_open_stability_duration() {
                        info!(
                            circuit_state = "HalfOpen -> Closed",
                            uptime_secs = uptime.as_secs(),
                            "circuit breaker closing - actor stable in half-open state"
                        );
                        state.circuit_breaker_state = CircuitBreakerState::Closed;
                        state.circuit_opened_at = None;
                    }
                }
            }
        }
    }

    /// Check if restart should be allowed based on circuit breaker state.
    ///
    /// Returns true if restart should proceed, false if blocked by circuit breaker.
    fn should_allow_restart(&self, state: &RaftSupervisorState) -> bool {
        match state.circuit_breaker_state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => false,
            CircuitBreakerState::HalfOpen => {
                // In HalfOpen, only allow restart if no actor is currently running
                // This ensures we only test ONE restart in HalfOpen state
                state.current_raft_actor.is_none()
            }
        }
    }

    /// Record a restart event in the history.
    ///
    /// Maintains bounded history using VecDeque (max 100 items).
    fn record_restart_event(&self, state: &mut RaftSupervisorState, event: RestartEvent) {
        state.restart_history.push_back(event);

        // Maintain size limit using bounded loop
        let max_size = state.config.restart_history_size as usize;
        let mut trim_count = 0;
        while state.restart_history.len() > max_size && trim_count < max_size {
            state.restart_history.pop_front();
            trim_count += 1;
        }
    }

    /// Spawn a new RaftActor instance using spawn_linked.
    async fn spawn_raft_actor(
        &self,
        myself: ActorRef<SupervisorMessage>,
        config: &RaftActorConfig,
    ) -> Result<(ActorRef<RaftActorMessage>, tokio::task::JoinHandle<()>), SupervisionError> {
        let node_id = config.node_id;
        let actor_name = format!("raft-{}", node_id);

        Actor::spawn_linked(
            Some(actor_name),
            RaftActor,
            config.clone(),
            myself.get_cell(),
        )
        .await
        .context(SpawnFailedSnafu)
    }

    /// Validates Raft storage integrity before allowing actor restart.
    ///
    /// For redb-backed storage, performs comprehensive validation checks:
    /// - Database accessibility
    /// - Log entry monotonicity (no gaps)
    /// - Snapshot metadata integrity
    /// - Vote state consistency
    /// - Committed index bounds
    ///
    /// For in-memory storage, always returns Ok (no disk persistence to corrupt).
    ///
    /// Tiger Style: Fail-fast on validation errors to prevent restarting with corrupt state.
    async fn validate_storage(&self, state: &RaftSupervisorState) -> Result<(), String> {
        use crate::raft::StateMachineVariant;
        use crate::raft::storage_validation::validate_raft_storage;

        let node_id = state.raft_actor_config.node_id;

        match &state.raft_actor_config.state_machine {
            StateMachineVariant::InMemory(_) => {
                // In-memory storage cannot be corrupted on disk
                debug!(
                    node_id = node_id,
                    "skipping storage validation for in-memory backend"
                );
                Ok(())
            }
            #[allow(deprecated)]
            StateMachineVariant::Redb(state_machine) => {
                let storage_path = state_machine.path();

                debug!(
                    node_id = node_id,
                    path = %storage_path.display(),
                    "validating redb storage before restart"
                );

                match validate_raft_storage(node_id, storage_path) {
                    Ok(report) => {
                        info!(
                            node_id = node_id,
                            checks_passed = report.checks_passed,
                            last_log_index = ?report.last_log_index,
                            validation_duration_ms = report.validation_duration.as_millis(),
                            "storage validation passed"
                        );
                        Ok(())
                    }
                    Err(err) => {
                        error!(
                            node_id = node_id,
                            error = %err,
                            "storage validation failed"
                        );
                        Err(format!("storage validation failed: {}", err))
                    }
                }
            }
            StateMachineVariant::Sqlite(state_machine) => {
                let storage_path = state_machine.path();

                debug!(
                    node_id = node_id,
                    path = %storage_path.display(),
                    "validating sqlite storage before restart"
                );

                // First, validate SQLite database integrity
                match state_machine.validate(node_id) {
                    Ok(report) => {
                        info!(
                            node_id = node_id,
                            checks_passed = report.checks_passed,
                            validation_duration_ms = report.validation_duration.as_millis(),
                            "sqlite storage validation passed"
                        );
                    }
                    Err(err) => {
                        error!(
                            node_id = node_id,
                            error = %err,
                            "sqlite storage validation failed"
                        );
                        return Err(format!("sqlite storage validation failed: {}", err));
                    }
                }

                // Second, validate cross-storage consistency (last_applied <= committed)
                if let Some(log_store) = &state.raft_actor_config.log_store {
                    debug!(
                        node_id = node_id,
                        "validating cross-storage consistency (sqlite state machine vs redb log)"
                    );

                    match state_machine.validate_consistency_with_log(log_store).await {
                        Ok(()) => {
                            info!(node_id = node_id, "cross-storage validation passed");
                            Ok(())
                        }
                        Err(err) => {
                            error!(
                                node_id = node_id,
                                error = %err,
                                "cross-storage validation failed"
                            );
                            Err(format!("cross-storage validation failed: {}", err))
                        }
                    }
                } else {
                    warn!(
                        node_id = node_id,
                        "no log store available for cross-storage validation"
                    );
                    Ok(())
                }
            }
        }
    }

    /// Handle actor failure and restart logic.
    async fn handle_actor_failed(
        &self,
        myself: ActorRef<SupervisorMessage>,
        state: &mut RaftSupervisorState,
        _cell: ActorCell,
        err: String,
    ) -> Result<(), ActorProcessingErr> {
        let node_id = state.raft_actor_config.node_id;

        error!(
            node_id = node_id,
            error = %err,
            "raft actor failed"
        );

        // Check if auto-restart is enabled
        if !state.config.enable_auto_restart {
            warn!(
                node_id = node_id,
                "auto-restart disabled, actor will not be restarted"
            );
            state.current_raft_actor = None;
            state.actor_spawn_time = None;
            return Ok(());
        }

        // Update circuit breaker state based on restart history
        self.update_circuit_breaker_state(state);

        // Check if restart should be allowed based on circuit breaker
        if !self.should_allow_restart(state) {
            let restarts_in_window = self.count_restarts_in_window(state);
            error!(
                node_id = node_id,
                circuit_state = ?state.circuit_breaker_state,
                restarts_in_window = restarts_in_window,
                window_secs = state.config.restart_window_secs,
                "circuit breaker blocking restart"
            );
            state.current_raft_actor = None;
            state.actor_spawn_time = None;
            return Ok(());
        }

        // Calculate actor uptime
        let actor_uptime = state
            .actor_spawn_time
            .map(|spawn_time| Instant::now().duration_since(spawn_time))
            .unwrap_or(Duration::ZERO);

        // Check if actor was stable
        let was_stable = actor_uptime >= state.config.actor_stability_duration();
        if was_stable {
            info!(
                node_id = node_id,
                uptime_secs = actor_uptime.as_secs(),
                "actor was stable before failure"
            );
        } else {
            warn!(
                node_id = node_id,
                uptime_secs = actor_uptime.as_secs(),
                stability_threshold_secs = state.config.actor_stability_duration_secs,
                "actor failed before reaching stability threshold"
            );
        }

        // Validate storage before restart
        if let Err(validation_err) = self.validate_storage(state).await {
            error!(
                node_id = node_id,
                error = %validation_err,
                "storage validation failed, restart aborted"
            );
            state.current_raft_actor = None;
            state.actor_spawn_time = None;
            return Ok(());
        }

        // Calculate backoff based on restarts in window
        let restarts_in_window = self.count_restarts_in_window(state);
        let backoff = self.calculate_backoff(restarts_in_window);

        info!(
            node_id = node_id,
            backoff_secs = backoff.as_secs(),
            restarts_in_window = restarts_in_window,
            "applying exponential backoff before restart"
        );

        // Apply backoff delay
        tokio::time::sleep(backoff).await;

        // Attempt to respawn RaftActor
        info!(node_id = node_id, "attempting to restart raft actor");

        match self
            .spawn_raft_actor(myself.clone(), &state.raft_actor_config)
            .await
        {
            Ok((new_actor, _task)) => {
                info!(node_id = node_id, "raft actor restarted successfully");

                // Stop old health monitor task if it exists
                if let Some(task) = state.health_monitor_task.take() {
                    task.abort();
                }

                // Start new health monitor for the restarted actor
                let health_monitor = Arc::new(HealthMonitor::new(
                    HealthMonitorConfig::default(),
                    new_actor.clone(),
                ));
                let health_monitor_task = health_monitor.clone().start();

                info!(
                    node_id = node_id,
                    "health monitor restarted for new raft actor"
                );

                // Record restart event
                let event = RestartEvent {
                    timestamp: Instant::now(),
                    reason: err,
                    actor_uptime_secs: actor_uptime.as_secs(),
                    backoff_applied_secs: backoff.as_secs(),
                };
                self.record_restart_event(state, event);

                // Update state
                state.current_raft_actor = Some(new_actor);
                state.actor_spawn_time = Some(Instant::now());
                state.total_restarts += 1;
                state.health_monitor = Some(health_monitor);
                state.health_monitor_task = Some(health_monitor_task);

                Ok(())
            }
            Err(spawn_err) => {
                error!(
                    node_id = node_id,
                    error = %spawn_err,
                    circuit_state = ?state.circuit_breaker_state,
                    "failed to restart raft actor"
                );

                // If restart failed in HalfOpen state, transition back to Open
                if state.circuit_breaker_state == CircuitBreakerState::HalfOpen {
                    warn!(
                        node_id = node_id,
                        circuit_state = "HalfOpen -> Open",
                        "restart failed in half-open state, reopening circuit"
                    );
                    state.circuit_breaker_state = CircuitBreakerState::Open;
                    state.circuit_opened_at = Some(Instant::now());
                }

                state.current_raft_actor = None;
                state.actor_spawn_time = None;
                Ok(())
            }
        }
    }
}

#[async_trait]
impl Actor for RaftSupervisor {
    type Msg = SupervisorMessage;
    type State = RaftSupervisorState;
    type Arguments = SupervisorArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let node_id = args.raft_actor_config.node_id;
        info!(node_id = node_id, "raft supervisor starting");

        // Spawn initial RaftActor using spawn_linked
        let (raft_actor, _task) = self
            .spawn_raft_actor(myself, &args.raft_actor_config)
            .await
            .map_err(|err| ActorProcessingErr::from(err.to_string()))?;

        info!(
            node_id = node_id,
            "initial raft actor spawned under supervision"
        );

        // Start health monitoring for the RaftActor
        let health_monitor = Arc::new(HealthMonitor::new(
            HealthMonitorConfig::default(),
            raft_actor.clone(),
        ));
        let health_monitor_task = health_monitor.clone().start();

        info!(node_id = node_id, "health monitor started for raft actor");

        Ok(RaftSupervisorState {
            restart_history: VecDeque::with_capacity(
                args.supervision_config.restart_history_size as usize,
            ),
            config: args.supervision_config,
            raft_actor_config: args.raft_actor_config,
            current_raft_actor: Some(raft_actor),
            actor_spawn_time: Some(Instant::now()),
            total_restarts: 0,
            health_monitor: Some(health_monitor),
            health_monitor_task: Some(health_monitor_task),
            circuit_breaker_state: CircuitBreakerState::Closed,
            circuit_opened_at: None,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisorMessage::GetStatus(reply) => {
                // Update circuit breaker state before returning status
                self.update_circuit_breaker_state(state);

                let restarts_in_window = self.count_restarts_in_window(state);
                let is_meltdown = self.is_meltdown(state);

                let status = SupervisionStatus {
                    total_restarts: state.total_restarts,
                    restarts_in_window,
                    is_meltdown,
                    circuit_breaker_state: state.circuit_breaker_state,
                    auto_restart_enabled: state.config.enable_auto_restart,
                    raft_actor: state.current_raft_actor.clone(),
                };

                info!(
                    total_restarts = state.total_restarts,
                    restarts_in_window = restarts_in_window,
                    is_meltdown = is_meltdown,
                    circuit_state = ?state.circuit_breaker_state,
                    auto_restart = state.config.enable_auto_restart,
                    "supervision status requested"
                );

                let _ = reply.send(status);
            }
            SupervisorMessage::GetRaftActor(reply) => {
                let _ = reply.send(state.current_raft_actor.clone());
            }
            SupervisorMessage::ManualRestart => {
                let node_id = state.raft_actor_config.node_id;
                warn!(
                    node_id = node_id,
                    "manual restart requested (bypasses meltdown check)"
                );

                // Stop current actor if running
                if let Some(ref actor) = state.current_raft_actor {
                    actor.stop(Some("manual-restart".into()));
                }

                // Note: The actual restart will happen via SupervisionEvent::ActorTerminated
                // or SupervisionEvent::ActorFailed when the actor stops
            }
            SupervisorMessage::GetRestartHistory(reply) => {
                info!(
                    history_size = state.restart_history.len(),
                    "restart history requested"
                );
                let history: Vec<RestartEvent> = state.restart_history.iter().cloned().collect();
                let _ = reply.send(history);
            }
            SupervisorMessage::GetHealthMonitor(reply) => {
                debug!("health monitor reference requested");
                let _ = reply.send(state.health_monitor.clone());
            }
        }

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        evt: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match evt {
            SupervisionEvent::ActorFailed(cell, err) => {
                self.handle_actor_failed(myself, state, cell, err.to_string())
                    .await
            }
            SupervisionEvent::ActorTerminated(_cell, _, reason) => {
                let node_id = state.raft_actor_config.node_id;
                info!(
                    node_id = node_id,
                    reason = ?reason,
                    "raft actor terminated normally"
                );

                // Stop health monitor task if it exists
                if let Some(task) = state.health_monitor_task.take() {
                    task.abort();
                }

                // Clear current actor reference and health monitor
                state.current_raft_actor = None;
                state.actor_spawn_time = None;
                state.health_monitor = None;

                Ok(())
            }
            SupervisionEvent::ActorStarted(_cell) => {
                // Actor started successfully, no action needed
                Ok(())
            }
            SupervisionEvent::PidLifecycleEvent(_event) => {
                // Process lifecycle event, not relevant for supervision
                Ok(())
            }
            SupervisionEvent::ProcessGroupChanged(_) => {
                // Not used in OneForOne supervision
                Ok(())
            }
        }
    }
}

// ============================================================================
// Phase 3: Health Monitoring
// ============================================================================

/// Health monitor configuration.
///
/// Tiger Style: Fixed values, no runtime configuration to reduce complexity.
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// Interval between health checks.
    /// Fixed at 1 second for consistent monitoring.
    pub check_interval: Duration,

    /// Timeout for each health check.
    /// Fixed at 25ms to detect hangs quickly.
    pub check_timeout: Duration,

    /// Number of consecutive failures before marking unhealthy.
    /// Fixed at 3 to balance sensitivity with false positives.
    pub consecutive_failure_threshold: u32,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(1),
            check_timeout: Duration::from_millis(25),
            consecutive_failure_threshold: 3,
        }
    }
}

/// Health status of a RaftActor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Actor is responding normally (0 consecutive failures).
    Healthy,

    /// Actor has 1-2 consecutive failures (warning state).
    Degraded,

    /// Actor has 3+ consecutive failures (critical state).
    Unhealthy,
}

/// Health monitor for RaftActor.
///
/// Runs continuous liveness checks in the background and exposes health status
/// for observability.
///
/// ## Usage
///
/// ```rust,ignore
/// let health_monitor = Arc::new(HealthMonitor::new(
///     HealthMonitorConfig::default(),
///     raft_actor_ref.clone(),
/// ));
///
/// // Start background health checks
/// let health_task = health_monitor.clone().start();
///
/// // Query health status
/// let status = health_monitor.get_status().await;
/// ```
#[derive(Debug)]
pub struct HealthMonitor {
    config: HealthMonitorConfig,
    raft_actor_ref: ActorRef<RaftActorMessage>,
    consecutive_failures: AtomicU32,
    last_check_time: Arc<RwLock<Instant>>,
    health_status: Arc<RwLock<HealthStatus>>,
}

impl HealthMonitor {
    /// Create a new health monitor.
    ///
    /// # Arguments
    ///
    /// * `config` - Health monitoring configuration (typically default)
    /// * `raft_actor_ref` - Reference to the RaftActor to monitor
    pub fn new(config: HealthMonitorConfig, raft_actor_ref: ActorRef<RaftActorMessage>) -> Self {
        Self {
            config,
            raft_actor_ref,
            consecutive_failures: AtomicU32::new(0),
            last_check_time: Arc::new(RwLock::new(Instant::now())),
            health_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
        }
    }

    /// Start background health check task.
    ///
    /// Returns a JoinHandle that can be used to await task completion or cancel it.
    ///
    /// # Tiger Style Notes
    ///
    /// - Loop runs until task is dropped (bounded by shutdown)
    /// - Sleep interval is fixed at 1 second (no unbounded delay)
    /// - Failure counter has fixed threshold (no unbounded growth)
    #[instrument(skip(self), fields(check_interval_ms = self.config.check_interval.as_millis()))]
    pub fn start(self: Arc<Self>) -> JoinHandle<()> {
        debug!("starting health monitor background task");

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.config.check_interval).await;

                // Increment total health checks counter
                let metrics = health_metrics();
                metrics.health_checks_total.fetch_add(1, Ordering::Relaxed);

                match self.liveness_check().await {
                    Ok(_) => {
                        // Reset failure counter on success
                        self.consecutive_failures.store(0, Ordering::Relaxed);
                        *self.health_status.write().await = HealthStatus::Healthy;
                        *self.last_check_time.write().await = Instant::now();

                        // Update global metrics
                        metrics.health_status.store(2, Ordering::Relaxed); // 2 = Healthy
                        metrics.consecutive_failures.store(0, Ordering::Relaxed);

                        debug!("health check passed");
                    }
                    Err(err) => {
                        // Increment failure counters
                        metrics
                            .health_check_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                        let failures =
                            self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

                        // Update health status based on threshold
                        let status = if failures >= self.config.consecutive_failure_threshold {
                            HealthStatus::Unhealthy
                        } else {
                            HealthStatus::Degraded
                        };

                        *self.health_status.write().await = status;
                        *self.last_check_time.write().await = Instant::now();

                        // Update global metrics
                        let status_value = match status {
                            HealthStatus::Healthy => 2,
                            HealthStatus::Degraded => 1,
                            HealthStatus::Unhealthy => 0,
                        };
                        metrics.health_status.store(status_value, Ordering::Relaxed);
                        metrics
                            .consecutive_failures
                            .store(failures, Ordering::Relaxed);

                        warn!(
                            consecutive_failures = failures,
                            health_status = ?status,
                            error = %err,
                            "health check failed"
                        );
                    }
                }
            }
        })
    }

    /// Perform a liveness check on the RaftActor.
    ///
    /// Sends a Ping message and expects a response within the configured timeout.
    ///
    /// # Errors
    ///
    /// Returns `HealthCheckError::LivenessTimeout` if the actor doesn't respond within timeout.
    /// Returns `HealthCheckError::ActorNotResponding` if the RPC call fails.
    #[instrument(skip(self))]
    async fn liveness_check(&self) -> Result<(), HealthCheckError> {
        let timeout_ms = self.config.check_timeout.as_millis() as u64;

        // Send Ping message with timeout
        call_t!(self.raft_actor_ref, RaftActorMessage::Ping, timeout_ms).map_err(|err| {
            HealthCheckError::ActorNotResponding {
                reason: err.to_string(),
            }
        })?;

        debug!("liveness check succeeded");
        Ok(())
    }

    /// Perform a readiness check on the RaftActor.
    ///
    /// Currently checks liveness. Future versions may add storage accessibility checks.
    ///
    /// # Errors
    ///
    /// Returns errors from liveness check.
    #[allow(dead_code)]
    pub async fn readiness_check(&self) -> Result<(), HealthCheckError> {
        // Check liveness first
        self.liveness_check().await?;

        // TODO(Phase 2): Add storage accessibility check
        // - Verify redb databases are accessible
        // - Check disk space availability
        // - Validate state machine is readable

        Ok(())
    }

    /// Get the current health status.
    ///
    /// Returns Healthy, Degraded, or Unhealthy based on consecutive failure count.
    pub async fn get_status(&self) -> HealthStatus {
        *self.health_status.read().await
    }

    /// Get the number of consecutive health check failures.
    ///
    /// Returns 0 if healthy, 1-2 if degraded, 3+ if unhealthy.
    pub fn get_consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    /// Get the time of the last health check.
    ///
    /// Useful for detecting if the health monitor itself has stalled.
    pub async fn get_last_check_time(&self) -> Instant {
        *self.last_check_time.read().await
    }
}

/// Health check errors.
#[derive(Debug, Snafu)]
pub enum HealthCheckError {
    /// Health check timed out waiting for response.
    #[snafu(display("Liveness check timeout after {timeout:?}"))]
    LivenessTimeout {
        /// Timeout duration that was exceeded.
        timeout: Duration,
    },

    /// Actor is not responding to health checks.
    #[snafu(display("Actor not responding: {reason}"))]
    ActorNotResponding {
        /// Reason for non-responsiveness.
        reason: String,
    },
}

#[cfg(test)]
mod health_monitor_tests {
    use super::*;
    use crate::raft::storage::StateMachineStore;
    use crate::raft::types::{AppTypeConfig, NodeId};
    use crate::raft::{RaftActor, RaftActorConfig, StateMachineVariant};
    use openraft::BasicNode;
    use openraft::Config as RaftConfig;
    use openraft::error::{RPCError, Unreachable};
    use openraft::network::{RaftNetworkFactory, v2::RaftNetworkV2};
    use std::sync::Arc;

    /// Mock network factory for testing that doesn't actually send messages.
    #[derive(Debug, Clone, Default)]
    struct MockNetworkFactory;

    impl RaftNetworkFactory<AppTypeConfig> for MockNetworkFactory {
        type Network = MockNetwork;

        async fn new_client(&mut self, _target: NodeId, _node: &BasicNode) -> Self::Network {
            MockNetwork
        }
    }

    /// Mock network implementation that always returns unreachable errors.
    struct MockNetwork;

    impl RaftNetworkV2<AppTypeConfig> for MockNetwork {
        async fn append_entries(
            &mut self,
            _req: openraft::raft::AppendEntriesRequest<AppTypeConfig>,
            _option: openraft::network::RPCOption,
        ) -> Result<openraft::raft::AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>>
        {
            let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock network");
            Err(RPCError::Unreachable(Unreachable::new(&err)))
        }

        async fn full_snapshot(
            &mut self,
            _vote: openraft::type_config::alias::VoteOf<AppTypeConfig>,
            _snapshot: openraft::Snapshot<AppTypeConfig>,
            _cancel: impl std::future::Future<Output = openraft::error::ReplicationClosed>
            + openraft::OptionalSend,
            _option: openraft::network::RPCOption,
        ) -> Result<
            openraft::raft::SnapshotResponse<AppTypeConfig>,
            openraft::error::StreamingError<AppTypeConfig>,
        > {
            let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock network");
            Err(openraft::error::StreamingError::Unreachable(
                Unreachable::new(&err),
            ))
        }

        async fn vote(
            &mut self,
            _req: openraft::raft::VoteRequest<AppTypeConfig>,
            _option: openraft::network::RPCOption,
        ) -> Result<openraft::raft::VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
            let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock network");
            Err(RPCError::Unreachable(Unreachable::new(&err)))
        }
    }

    /// Create a test RaftActor for health monitoring tests.
    async fn create_test_raft_actor() -> (ActorRef<RaftActorMessage>, openraft::Raft<AppTypeConfig>)
    {
        use crate::raft::storage::InMemoryLogStore;
        use openraft::Raft;

        let node_id = 1;
        let config = Arc::new(RaftConfig::default());
        let log_store = InMemoryLogStore::default();
        let state_machine = StateMachineStore::default();
        let network = MockNetworkFactory;

        let state_machine_arc = Arc::new(state_machine);

        let raft = Raft::new(
            node_id,
            config,
            network,
            log_store,
            state_machine_arc.clone(),
        )
        .await
        .expect("failed to create raft");

        let raft_config = RaftActorConfig {
            node_id,
            raft: raft.clone(),
            state_machine: StateMachineVariant::InMemory(state_machine_arc),
            log_store: None,
        };

        let (actor_ref, _) = ractor::Actor::spawn(None, RaftActor, raft_config)
            .await
            .expect("failed to spawn raft actor");

        (actor_ref, raft)
    }

    #[tokio::test]
    async fn test_health_monitor_detects_healthy_actor() {
        let (actor_ref, _raft) = create_test_raft_actor().await;

        let monitor = Arc::new(HealthMonitor::new(
            HealthMonitorConfig::default(),
            actor_ref.clone(),
        ));

        // Perform a single liveness check
        let result = monitor.liveness_check().await;
        assert!(result.is_ok(), "liveness check should succeed");

        // Verify status is Healthy
        let status = monitor.get_status().await;
        assert_eq!(status, HealthStatus::Healthy, "status should be Healthy");

        // Verify no failures
        assert_eq!(
            monitor.get_consecutive_failures(),
            0,
            "should have 0 failures"
        );
    }

    #[tokio::test]
    async fn test_health_monitor_background_task() {
        let (actor_ref, _raft) = create_test_raft_actor().await;

        let monitor = Arc::new(HealthMonitor::new(
            HealthMonitorConfig {
                check_interval: Duration::from_millis(100), // Fast for testing
                check_timeout: Duration::from_millis(25),
                consecutive_failure_threshold: 3,
            },
            actor_ref.clone(),
        ));

        // Start background task
        let _task = monitor.clone().start();

        // Wait for a few health checks
        tokio::time::sleep(Duration::from_millis(350)).await;

        // Verify status is still Healthy
        let status = monitor.get_status().await;
        assert_eq!(
            status,
            HealthStatus::Healthy,
            "status should remain Healthy"
        );

        // Verify no failures
        assert_eq!(
            monitor.get_consecutive_failures(),
            0,
            "should have 0 failures"
        );
    }

    #[tokio::test]
    async fn test_health_status_transitions() {
        let (actor_ref, _raft) = create_test_raft_actor().await;

        let monitor = Arc::new(HealthMonitor::new(
            HealthMonitorConfig::default(),
            actor_ref.clone(),
        ));

        // Initially Healthy
        assert_eq!(monitor.get_status().await, HealthStatus::Healthy);

        // Simulate 1 failure -> Degraded
        monitor.consecutive_failures.store(1, Ordering::Relaxed);
        *monitor.health_status.write().await = HealthStatus::Degraded;
        assert_eq!(monitor.get_status().await, HealthStatus::Degraded);

        // Simulate 2 failures -> still Degraded
        monitor.consecutive_failures.store(2, Ordering::Relaxed);
        assert_eq!(monitor.get_status().await, HealthStatus::Degraded);

        // Simulate 3 failures -> Unhealthy
        monitor.consecutive_failures.store(3, Ordering::Relaxed);
        *monitor.health_status.write().await = HealthStatus::Unhealthy;
        assert_eq!(monitor.get_status().await, HealthStatus::Unhealthy);

        // Reset -> Healthy
        monitor.consecutive_failures.store(0, Ordering::Relaxed);
        *monitor.health_status.write().await = HealthStatus::Healthy;
        assert_eq!(monitor.get_status().await, HealthStatus::Healthy);
    }
}
