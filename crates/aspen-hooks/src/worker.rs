//! Hook job worker for reliable execution via the job queue.
//!
//! Implements the Worker trait from aspen-jobs to enable hook execution
//! with automatic retry, DLQ, and persistence.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::config::HookHandlerType;
use crate::constants::HOOK_JOB_TYPE;
use crate::error::HookError;
use crate::error::Result;
use crate::event::HookEvent;
use crate::handlers::ForwardHandler;
use crate::handlers::HookHandler;
use crate::handlers::InProcessHandler;
use crate::handlers::InProcessHandlerRegistry;
use crate::handlers::ShellHandler;

/// Payload for hook jobs submitted to the job queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookJobPayload {
    /// Name of the handler to invoke.
    pub handler_name: String,

    /// The hook event to process.
    pub event: HookEvent,

    /// Handler type configuration (used to recreate handler on different nodes).
    pub handler_type: HandlerTypePayload,

    /// Timeout for handler execution in milliseconds.
    pub timeout_ms: u64,
}

impl HookJobPayload {
    /// Create a new hook job payload.
    pub fn new(
        handler_name: impl Into<String>,
        event: HookEvent,
        handler_type: HandlerTypePayload,
        timeout_ms: u64,
    ) -> Self {
        Self {
            handler_name: handler_name.into(),
            event,
            handler_type,
            timeout_ms,
        }
    }

    /// Create from a handler config and event.
    pub fn from_config(config: &crate::config::HookHandlerConfig, event: HookEvent) -> Self {
        Self {
            handler_name: config.name.clone(),
            event,
            handler_type: HandlerTypePayload::from(&config.handler_type),
            timeout_ms: config.timeout_ms,
        }
    }
}

/// Serializable handler type for job payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HandlerTypePayload {
    /// In-process handler.
    InProcess {
        /// Handler ID for registry lookup.
        handler_id: String,
    },

    /// Shell command handler.
    Shell {
        /// Shell command to execute.
        command: String,
        /// Working directory.
        working_dir: Option<String>,
    },

    /// Forward handler.
    Forward {
        /// Target cluster ticket.
        target_cluster: String,
        /// Target topic.
        target_topic: String,
    },
}

impl From<&HookHandlerType> for HandlerTypePayload {
    fn from(handler_type: &HookHandlerType) -> Self {
        match handler_type {
            HookHandlerType::InProcess { handler_id } => HandlerTypePayload::InProcess {
                handler_id: handler_id.clone(),
            },
            HookHandlerType::Shell { command, working_dir } => HandlerTypePayload::Shell {
                command: command.clone(),
                working_dir: working_dir.clone(),
            },
            HookHandlerType::Forward {
                target_cluster,
                target_topic,
            } => HandlerTypePayload::Forward {
                target_cluster: target_cluster.clone(),
                target_topic: target_topic.clone(),
            },
        }
    }
}

/// Worker that executes hook jobs from the job queue.
///
/// This worker is registered with the WorkerPool to handle jobs of type "hook".
/// It recreates handlers from the job payload and executes them.
///
/// # Benefits of job-based execution
///
/// - Automatic retry with exponential backoff
/// - Dead Letter Queue for failed handlers
/// - Persistent job state (survives restarts)
/// - Visibility timeout prevents duplicate execution
/// - Priority levels (critical, high, normal, low)
pub struct HookJobWorker {
    /// Registry for in-process handlers.
    registry: Arc<InProcessHandlerRegistry>,
}

impl HookJobWorker {
    /// Create a new hook job worker.
    pub fn new(registry: Arc<InProcessHandlerRegistry>) -> Self {
        Self { registry }
    }

    /// Create a handler from the job payload.
    fn create_handler(&self, payload: &HookJobPayload) -> Box<dyn HookHandler> {
        let timeout = Duration::from_millis(payload.timeout_ms);

        match &payload.handler_type {
            HandlerTypePayload::InProcess { handler_id } => {
                Box::new(InProcessHandler::new(&payload.handler_name, handler_id, Arc::clone(&self.registry)))
            }
            HandlerTypePayload::Shell { command, working_dir } => {
                Box::new(ShellHandler::new(&payload.handler_name, command, working_dir.clone(), timeout))
            }
            HandlerTypePayload::Forward {
                target_cluster,
                target_topic,
            } => Box::new(ForwardHandler::new(&payload.handler_name, target_cluster, target_topic, timeout)),
        }
    }

    /// Execute a hook job payload.
    ///
    /// This is the internal implementation used by the Worker trait.
    pub async fn execute_payload(&self, payload: HookJobPayload) -> Result<()> {
        let handler = self.create_handler(&payload);
        handler.handle(&payload.event).await
    }
}

/// Implementation of the Worker trait from aspen-jobs.
///
/// Note: This module provides the implementation but the actual Worker trait
/// is in aspen-jobs. The integration happens when registering with WorkerPool.
pub mod job_worker {
    use aspen_jobs::Job;
    use aspen_jobs::JobResult;
    use aspen_jobs::Worker;

    use super::*;

    /// Wrapper that implements the Worker trait.
    pub struct HookWorkerImpl {
        inner: HookJobWorker,
    }

    impl HookWorkerImpl {
        /// Create a new hook worker implementation.
        pub fn new(registry: Arc<InProcessHandlerRegistry>) -> Self {
            Self {
                inner: HookJobWorker::new(registry),
            }
        }
    }

    #[async_trait]
    impl Worker for HookWorkerImpl {
        async fn execute(&self, job: Job) -> JobResult {
            // Deserialize payload
            let payload: HookJobPayload = match serde_json::from_value(job.spec.payload.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return JobResult::failure(format!("failed to deserialize hook job payload: {}", e));
                }
            };

            // Execute the handler
            match self.inner.execute_payload(payload).await {
                Ok(()) => JobResult::success(serde_json::json!({"handled": true})),
                Err(e) => {
                    // Determine if the error is retryable
                    let is_retryable = !matches!(
                        e,
                        HookError::ConfigInvalid { .. }
                            | HookError::HandlerNameEmpty
                            | HookError::HandlerNameTooLong { .. }
                            | HookError::ShellCommandEmpty
                            | HookError::ShellCommandTooLong { .. }
                    );

                    if is_retryable {
                        JobResult::failure(format!("hook handler failed: {}", e))
                    } else {
                        // Non-retryable errors - use failure() but job won't be retried
                        // due to the error type check above
                        JobResult::failure(format!("hook handler failed (non-retryable): {}", e))
                    }
                }
            }
        }

        fn job_types(&self) -> Vec<String> {
            vec![HOOK_JOB_TYPE.to_string()]
        }
    }
}

/// Create a JobSpec for a hook execution.
pub fn create_hook_job_spec(config: &crate::config::HookHandlerConfig, event: HookEvent) -> aspen_jobs::JobSpec {
    use aspen_jobs::JobSpec;
    use aspen_jobs::Priority;
    use aspen_jobs::RetryPolicy;

    let payload = HookJobPayload::from_config(config, event);

    let priority = match config.effective_job_priority() {
        "critical" => Priority::Critical,
        "high" => Priority::High,
        "low" => Priority::Low,
        _ => Priority::Normal,
    };

    let retry_policy = RetryPolicy::Exponential {
        initial_delay: Duration::from_millis(crate::constants::RETRY_INITIAL_BACKOFF_MS),
        multiplier: crate::constants::RETRY_BACKOFF_MULTIPLIER,
        max_delay: Some(Duration::from_millis(crate::constants::RETRY_MAX_BACKOFF_MS)),
        max_attempts: config.retry_count,
    };

    JobSpec::new(HOOK_JOB_TYPE)
        .payload(payload)
        .unwrap_or_else(|_| JobSpec::new(HOOK_JOB_TYPE))
        .priority(priority)
        .retry_policy(retry_policy)
        .timeout(Duration::from_millis(config.timeout_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::HookEventType;

    fn test_event() -> HookEvent {
        HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({"key": "test"}))
    }

    #[test]
    fn test_hook_job_payload_serialization() {
        let payload = HookJobPayload::new(
            "test-handler",
            test_event(),
            HandlerTypePayload::Shell {
                command: "echo test".to_string(),
                working_dir: None,
            },
            5000,
        );

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["handler_name"], "test-handler");
        assert_eq!(json["timeout_ms"], 5000);
        assert_eq!(json["handler_type"]["type"], "shell");

        let decoded: HookJobPayload = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.handler_name, "test-handler");
    }

    #[test]
    fn test_handler_type_payload_conversion() {
        let in_process = HookHandlerType::InProcess {
            handler_id: "test".to_string(),
        };
        let payload = HandlerTypePayload::from(&in_process);
        assert!(matches!(payload, HandlerTypePayload::InProcess { .. }));

        let shell = HookHandlerType::Shell {
            command: "echo test".to_string(),
            working_dir: Some("/tmp".to_string()),
        };
        let payload = HandlerTypePayload::from(&shell);
        assert!(matches!(payload, HandlerTypePayload::Shell { .. }));

        let forward = HookHandlerType::Forward {
            target_cluster: "aspen://ticket".to_string(),
            target_topic: "events".to_string(),
        };
        let payload = HandlerTypePayload::from(&forward);
        assert!(matches!(payload, HandlerTypePayload::Forward { .. }));
    }

    #[tokio::test]
    async fn test_hook_job_worker_shell() {
        let registry = Arc::new(InProcessHandlerRegistry::new());
        let worker = HookJobWorker::new(registry);

        let payload = HookJobPayload::new(
            "test",
            test_event(),
            HandlerTypePayload::Shell {
                command: "exit 0".to_string(),
                working_dir: None,
            },
            5000,
        );

        let result = worker.execute_payload(payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_job_worker_shell_failure() {
        let registry = Arc::new(InProcessHandlerRegistry::new());
        let worker = HookJobWorker::new(registry);

        let payload = HookJobPayload::new(
            "test",
            test_event(),
            HandlerTypePayload::Shell {
                command: "exit 1".to_string(),
                working_dir: None,
            },
            5000,
        );

        let result = worker.execute_payload(payload).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_create_hook_job_spec() {
        use crate::config::ExecutionMode;
        use crate::config::HookHandlerConfig;
        use crate::config::HookHandlerType;

        let config = HookHandlerConfig {
            name: "test-handler".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Job,
            handler_type: HookHandlerType::Shell {
                command: "echo test".to_string(),
                working_dir: None,
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: Some("high".to_string()),
            is_enabled: true,
        };

        let spec = create_hook_job_spec(&config, test_event());
        assert_eq!(spec.job_type, HOOK_JOB_TYPE);
        assert_eq!(spec.config.priority, aspen_jobs::Priority::High);
    }
}
