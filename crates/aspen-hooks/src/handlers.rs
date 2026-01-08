//! Hook handler trait and implementations.
//!
//! Provides the core abstraction for handling hook events and implementations
//! for in-process, shell, and forward handlers.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::constants::HOOK_EVENT_ENV_VAR;
use crate::constants::HOOK_EVENT_TYPE_ENV_VAR;
use crate::constants::HOOK_TOPIC_ENV_VAR;
use crate::constants::MAX_SHELL_OUTPUT_SIZE;
use crate::constants::SHELL_GRACE_PERIOD;
use crate::error::ExecutionFailedSnafu;
use crate::error::ExecutionTimeoutSnafu;
use crate::error::HookError;
use crate::error::Result;
use crate::error::ShellTerminatedSnafu;
use crate::event::HookEvent;

/// Trait for handling hook events.
///
/// Implementations must be thread-safe and cloneable (via `clone_box`).
#[async_trait]
pub trait HookHandler: Send + Sync {
    /// Handle an event.
    ///
    /// # Errors
    ///
    /// Returns an error if the handler fails to process the event.
    async fn handle(&self, event: &HookEvent) -> Result<()>;

    /// Get the handler name for logging/metrics.
    fn name(&self) -> &str;

    /// Clone the handler into a boxed trait object.
    fn clone_box(&self) -> Box<dyn HookHandler>;
}

impl Clone for Box<dyn HookHandler> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Type alias for async handler functions.
pub type AsyncHandlerFn =
    Arc<dyn Fn(&HookEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> + Send + Sync>;

/// In-process handler that invokes a registered Rust async closure.
///
/// This is the fastest handler type with zero serialization overhead.
///
/// # Example
///
/// ```ignore
/// let registry = InProcessHandlerRegistry::new();
///
/// registry.register("cache_invalidator", |event| {
///     Box::pin(async move {
///         // Handle the event
///         Ok(())
///     })
/// }).await;
///
/// let handler = InProcessHandler::new("my-handler", registry);
/// ```
pub struct InProcessHandler {
    name: String,
    handler_id: String,
    registry: Arc<InProcessHandlerRegistry>,
}

impl InProcessHandler {
    /// Create a new in-process handler.
    pub fn new(
        name: impl Into<String>,
        handler_id: impl Into<String>,
        registry: Arc<InProcessHandlerRegistry>,
    ) -> Self {
        Self {
            name: name.into(),
            handler_id: handler_id.into(),
            registry,
        }
    }
}

#[async_trait]
impl HookHandler for InProcessHandler {
    async fn handle(&self, event: &HookEvent) -> Result<()> {
        let handler = self.registry.get(&self.handler_id).await.ok_or_else(|| HookError::HandlerNotFound {
            name: self.handler_id.clone(),
        })?;

        handler(event).await
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn clone_box(&self) -> Box<dyn HookHandler> {
        Box::new(Self {
            name: self.name.clone(),
            handler_id: self.handler_id.clone(),
            registry: Arc::clone(&self.registry),
        })
    }
}

/// Registry for in-process handler functions.
///
/// Handlers are registered by ID and can be looked up when processing events.
pub struct InProcessHandlerRegistry {
    handlers: RwLock<HashMap<String, AsyncHandlerFn>>,
}

impl Default for InProcessHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessHandlerRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler function.
    ///
    /// The function receives a reference to the event and returns a future
    /// that resolves to a Result.
    pub async fn register<F, Fut>(&self, id: impl Into<String>, handler: F)
    where
        F: Fn(&HookEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler_fn: AsyncHandlerFn = Arc::new(move |event| Box::pin(handler(event)));
        self.handlers.write().await.insert(id.into(), handler_fn);
    }

    /// Get a handler by ID.
    pub async fn get(&self, id: &str) -> Option<AsyncHandlerFn> {
        self.handlers.read().await.get(id).cloned()
    }

    /// Check if a handler is registered.
    pub async fn contains(&self, id: &str) -> bool {
        self.handlers.read().await.contains_key(id)
    }

    /// Remove a handler by ID.
    pub async fn remove(&self, id: &str) -> Option<AsyncHandlerFn> {
        self.handlers.write().await.remove(id)
    }

    /// Get the number of registered handlers.
    pub async fn len(&self) -> usize {
        self.handlers.read().await.len()
    }

    /// Check if the registry is empty.
    pub async fn is_empty(&self) -> bool {
        self.handlers.read().await.is_empty()
    }
}

/// Shell command handler.
///
/// Executes a shell command with the event data passed via environment variables:
/// - `ASPEN_HOOK_EVENT`: JSON-serialized event
/// - `ASPEN_HOOK_TOPIC`: Event topic
/// - `ASPEN_HOOK_EVENT_TYPE`: Event type name
///
/// Commands are executed with `sh -c` for portability.
#[derive(Clone)]
pub struct ShellHandler {
    name: String,
    command: String,
    working_dir: Option<String>,
    timeout: Duration,
}

impl ShellHandler {
    /// Create a new shell handler.
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        working_dir: Option<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            working_dir,
            timeout,
        }
    }
}

#[async_trait]
impl HookHandler for ShellHandler {
    async fn handle(&self, event: &HookEvent) -> Result<()> {
        // Serialize event to JSON for environment variable
        let event_json = serde_json::to_string(event)?;

        // Build command
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        cmd.arg(&self.command);

        // Set environment variables
        cmd.env(HOOK_EVENT_ENV_VAR, &event_json);
        cmd.env(HOOK_TOPIC_ENV_VAR, event.topic());
        cmd.env(HOOK_EVENT_TYPE_ENV_VAR, event.event_type.to_string());

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Create process group for clean termination
        #[cfg(unix)]
        {
            #[allow(unused_imports)]
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| HookError::ShellSpawnFailed { source: e })?;

        // Wait with timeout
        let result = timeout(self.timeout, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(())
                } else {
                    // Read stderr for error message
                    let stderr = if let Some(mut stderr) = child.stderr.take() {
                        let mut buf = vec![0u8; MAX_SHELL_OUTPUT_SIZE];
                        let n = stderr.read(&mut buf).await.unwrap_or(0);
                        buf.truncate(n);
                        String::from_utf8_lossy(&buf).to_string()
                    } else {
                        String::new()
                    };

                    Err(HookError::ShellExecutionFailed {
                        exit_code: status.code().unwrap_or(-1),
                        stderr,
                    })
                }
            }
            Ok(Err(_)) => {
                // Process was terminated by signal
                ShellTerminatedSnafu.fail()
            }
            Err(_) => {
                // Timeout - try graceful shutdown first
                tracing::warn!(
                    handler = %self.name,
                    command = %self.command,
                    "shell handler timed out, sending SIGTERM"
                );

                #[cfg(unix)]
                {
                    use nix::sys::signal::Signal;
                    use nix::unistd::Pid;

                    if let Some(id) = child.id() {
                        // Send SIGTERM to process group
                        let _ = nix::sys::signal::killpg(Pid::from_raw(id as i32), Signal::SIGTERM);

                        // Wait for grace period
                        if timeout(SHELL_GRACE_PERIOD, child.wait()).await.is_err() {
                            // Grace period expired, send SIGKILL
                            tracing::warn!(
                                handler = %self.name,
                                "grace period expired, sending SIGKILL"
                            );
                            let _ = nix::sys::signal::killpg(Pid::from_raw(id as i32), Signal::SIGKILL);
                            let _ = child.wait().await;
                        }
                    }
                }

                #[cfg(not(unix))]
                {
                    let _ = child.kill().await;
                }

                ExecutionTimeoutSnafu {
                    timeout_ms: self.timeout.as_millis() as u64,
                }
                .fail()
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn clone_box(&self) -> Box<dyn HookHandler> {
        Box::new(self.clone())
    }
}

/// Forward handler that forwards events to another cluster.
///
/// Note: This is a placeholder implementation. Full implementation requires
/// integration with the Iroh client for cross-cluster communication.
#[derive(Clone)]
pub struct ForwardHandler {
    name: String,
    target_cluster: String,
    target_topic: String,
    timeout: Duration,
}

impl ForwardHandler {
    /// Create a new forward handler.
    pub fn new(
        name: impl Into<String>,
        target_cluster: impl Into<String>,
        target_topic: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            target_cluster: target_cluster.into(),
            target_topic: target_topic.into(),
            timeout,
        }
    }

    /// Get the target cluster.
    pub fn target_cluster(&self) -> &str {
        &self.target_cluster
    }

    /// Get the target topic.
    pub fn target_topic(&self) -> &str {
        &self.target_topic
    }
}

#[async_trait]
impl HookHandler for ForwardHandler {
    async fn handle(&self, event: &HookEvent) -> Result<()> {
        // TODO: Implement cross-cluster forwarding using Iroh client
        // For now, log the intent and return success
        tracing::debug!(
            handler = %self.name,
            target_cluster = %self.target_cluster,
            target_topic = %self.target_topic,
            event_type = %event.event_type,
            "forwarding event (not yet implemented)"
        );

        // Simulate timeout check
        let _ = self.timeout;

        // Placeholder: In production, this would:
        // 1. Connect to target cluster via Iroh ticket
        // 2. Publish event to target topic
        // 3. Wait for acknowledgment

        ExecutionFailedSnafu {
            message: "forward handler not yet implemented".to_string(),
        }
        .fail()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn clone_box(&self) -> Box<dyn HookHandler> {
        Box::new(self.clone())
    }
}

/// Create a handler from configuration.
pub fn create_handler_from_config(
    config: &crate::config::HookHandlerConfig,
    registry: Arc<InProcessHandlerRegistry>,
) -> Box<dyn HookHandler> {
    let timeout = Duration::from_millis(config.timeout_ms);

    match &config.handler_type {
        crate::config::HookHandlerType::InProcess { handler_id } => {
            Box::new(InProcessHandler::new(&config.name, handler_id, registry))
        }
        crate::config::HookHandlerType::Shell { command, working_dir } => {
            Box::new(ShellHandler::new(&config.name, command, working_dir.clone(), timeout))
        }
        crate::config::HookHandlerType::Forward {
            target_cluster,
            target_topic,
        } => Box::new(ForwardHandler::new(&config.name, target_cluster, target_topic, timeout)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::HookEventType;

    fn test_event() -> HookEvent {
        HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({"key": "test"}))
    }

    #[tokio::test]
    async fn test_in_process_registry() {
        let registry = InProcessHandlerRegistry::new();

        // Register a handler
        registry.register("test", |_event| async { Ok(()) }).await;

        assert!(registry.contains("test").await);
        assert!(!registry.contains("nonexistent").await);
        assert_eq!(registry.len().await, 1);

        // Get and invoke handler
        let handler = registry.get("test").await.unwrap();
        let event = test_event();
        assert!(handler(&event).await.is_ok());

        // Remove handler
        registry.remove("test").await;
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_in_process_handler() {
        let registry = Arc::new(InProcessHandlerRegistry::new());

        // Register a handler that tracks invocations
        let invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked_clone = Arc::clone(&invoked);
        registry
            .register("tracker", move |_event| {
                let invoked = Arc::clone(&invoked_clone);
                async move {
                    invoked.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;

        let handler = InProcessHandler::new("test-handler", "tracker", registry);
        let event = test_event();

        assert!(handler.handle(&event).await.is_ok());
        assert!(invoked.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_in_process_handler_not_found() {
        let registry = Arc::new(InProcessHandlerRegistry::new());
        let handler = InProcessHandler::new("test-handler", "nonexistent", registry);
        let event = test_event();

        let result = handler.handle(&event).await;
        assert!(matches!(result, Err(HookError::HandlerNotFound { .. })));
    }

    #[tokio::test]
    async fn test_shell_handler_success() {
        let handler = ShellHandler::new("test", "exit 0", None, Duration::from_secs(5));
        let event = test_event();

        assert!(handler.handle(&event).await.is_ok());
    }

    #[tokio::test]
    async fn test_shell_handler_failure() {
        let handler = ShellHandler::new("test", "exit 1", None, Duration::from_secs(5));
        let event = test_event();

        let result = handler.handle(&event).await;
        assert!(matches!(result, Err(HookError::ShellExecutionFailed { exit_code: 1, .. })));
    }

    #[tokio::test]
    async fn test_shell_handler_timeout() {
        let handler = ShellHandler::new("test", "sleep 10", None, Duration::from_millis(100));
        let event = test_event();

        let result = handler.handle(&event).await;
        assert!(matches!(result, Err(HookError::ExecutionTimeout { .. })));
    }

    #[tokio::test]
    async fn test_shell_handler_env_vars() {
        // Create a script that echoes environment variables
        let handler = ShellHandler::new(
            "test",
            r#"test -n "$ASPEN_HOOK_EVENT" && test -n "$ASPEN_HOOK_TOPIC" && test -n "$ASPEN_HOOK_EVENT_TYPE""#,
            None,
            Duration::from_secs(5),
        );
        let event = test_event();

        assert!(handler.handle(&event).await.is_ok());
    }

    #[test]
    fn test_handler_clone() {
        let handler: Box<dyn HookHandler> =
            Box::new(ShellHandler::new("test", "echo test", None, Duration::from_secs(5)));
        let cloned = handler.clone_box();
        assert_eq!(cloned.name(), "test");
    }

    #[tokio::test]
    async fn test_forward_handler_not_implemented() {
        let handler = ForwardHandler::new("test", "aspen://target", "events.test", Duration::from_secs(5));
        let event = test_event();

        let result = handler.handle(&event).await;
        assert!(matches!(result, Err(HookError::ExecutionFailed { .. })));
    }
}
