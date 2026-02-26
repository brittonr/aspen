//! Configuration types for the hook system.
//!
//! Supports configuration via TOML with validation.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::DEFAULT_HANDLER_RETRY_COUNT;
use crate::constants::DEFAULT_HANDLER_TIMEOUT_MS;
use crate::constants::DEFAULT_JOB_PRIORITY;
use crate::constants::MAX_HANDLER_NAME_SIZE;
use crate::constants::MAX_HANDLER_RETRY_COUNT;
use crate::constants::MAX_HANDLER_TIMEOUT_MS;
use crate::constants::MAX_HANDLERS;
use crate::constants::MAX_PATTERN_SIZE;
use crate::constants::MAX_SHELL_COMMAND_SIZE;
use crate::error::ConfigInvalidSnafu;
use crate::error::HandlerNameEmptySnafu;
use crate::error::HandlerNameTooLongSnafu;
use crate::error::InvalidRetryCountSnafu;
use crate::error::InvalidTimeoutSnafu;
use crate::error::PatternTooLongSnafu;
use crate::error::Result;
use crate::error::ShellCommandEmptySnafu;
use crate::error::ShellCommandTooLongSnafu;
use crate::error::TooManyHandlersSnafu;

/// Top-level hooks configuration.
///
/// # Example TOML
///
/// ```toml
/// [hooks]
/// enabled = true
/// publish_topics = ["hooks.>"]
///
/// [[hooks.handlers]]
/// name = "audit-logger"
/// pattern = "hooks.>"
/// execution_mode = "direct"
/// type = "in_process"
/// handler_id = "audit_logger"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HooksConfig {
    /// Whether the hook system is enabled.
    #[serde(default = "default_enabled", rename = "enabled")]
    pub is_enabled: bool,

    /// Topic patterns to publish hook events for.
    ///
    /// If empty, all hook events are published.
    /// Use NATS-style wildcards: `hooks.>` for all, `hooks.kv.*` for KV events.
    #[serde(default)]
    pub publish_topics: Vec<String>,

    /// List of hook handlers.
    #[serde(default)]
    pub handlers: Vec<HookHandlerConfig>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            is_enabled: default_enabled(),
            publish_topics: vec![],
            handlers: vec![],
        }
    }
}

impl HooksConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.handlers.len() > MAX_HANDLERS {
            return TooManyHandlersSnafu {
                count: self.handlers.len() as u32,
                max: MAX_HANDLERS as u32,
            }
            .fail();
        }

        // Validate each handler
        for handler in &self.handlers {
            handler.validate()?;
        }

        // Check for duplicate handler names
        let mut names = std::collections::HashSet::new();
        for handler in &self.handlers {
            if !names.insert(&handler.name) {
                return ConfigInvalidSnafu {
                    message: format!("duplicate handler name: {}", handler.name),
                }
                .fail();
            }
        }

        // Validate publish_topics patterns
        for pattern in &self.publish_topics {
            if pattern.len() > MAX_PATTERN_SIZE {
                return PatternTooLongSnafu {
                    length: pattern.len() as u32,
                    max: MAX_PATTERN_SIZE as u32,
                }
                .fail();
            }
        }

        Ok(())
    }

    /// Get enabled handlers.
    pub fn enabled_handlers(&self) -> impl Iterator<Item = &HookHandlerConfig> {
        self.handlers.iter().filter(|h| h.is_enabled)
    }
}

/// Configuration for a single hook handler.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HookHandlerConfig {
    /// Unique name for this handler.
    pub name: String,

    /// Topic pattern to subscribe to (NATS-style: hooks.>, hooks.kv.*, etc.).
    pub pattern: String,

    /// Execution mode: direct (fast) or job (reliable with retry/DLQ).
    #[serde(default)]
    pub execution_mode: ExecutionMode,

    /// Handler type configuration.
    #[serde(flatten)]
    pub handler_type: HookHandlerType,

    /// Timeout for handler execution in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Number of retry attempts on failure.
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// Job priority (only used when execution_mode = job).
    #[serde(default)]
    pub job_priority: Option<String>,

    /// Whether this handler is enabled.
    #[serde(default = "default_enabled", rename = "enabled")]
    pub is_enabled: bool,
}

impl HookHandlerConfig {
    /// Validate the handler configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate name
        if self.name.is_empty() {
            return HandlerNameEmptySnafu.fail();
        }
        if self.name.len() > MAX_HANDLER_NAME_SIZE {
            return HandlerNameTooLongSnafu {
                length: self.name.len() as u32,
                max: MAX_HANDLER_NAME_SIZE as u32,
            }
            .fail();
        }

        // Validate pattern
        if self.pattern.len() > MAX_PATTERN_SIZE {
            return PatternTooLongSnafu {
                length: self.pattern.len() as u32,
                max: MAX_PATTERN_SIZE as u32,
            }
            .fail();
        }

        // Validate timeout
        if self.timeout_ms > MAX_HANDLER_TIMEOUT_MS {
            return InvalidTimeoutSnafu {
                value: self.timeout_ms,
                max: MAX_HANDLER_TIMEOUT_MS,
            }
            .fail();
        }

        // Validate retry count
        if self.retry_count > MAX_HANDLER_RETRY_COUNT {
            return InvalidRetryCountSnafu {
                value: self.retry_count,
                max: MAX_HANDLER_RETRY_COUNT,
            }
            .fail();
        }

        // Validate handler type specific constraints
        self.handler_type.validate()?;

        Ok(())
    }

    /// Get the effective job priority.
    pub fn effective_job_priority(&self) -> &str {
        self.job_priority.as_deref().unwrap_or(DEFAULT_JOB_PRIORITY)
    }
}

/// Execution mode for handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Direct async execution (fast path, no persistence).
    ///
    /// Handlers are executed immediately when events are received.
    /// No retry or DLQ support. Best for low-latency, fire-and-forget handlers.
    #[default]
    Direct,

    /// Submit as job (reliable path with retry, DLQ, persistence).
    ///
    /// Events are submitted to the job queue for execution by workers.
    /// Provides automatic retry, dead letter queue, and persistence.
    Job,
}

/// Handler type configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookHandlerType {
    /// In-process Rust async callback.
    ///
    /// The fastest option for handlers that don't need external processes.
    /// Handler code must be registered programmatically with the handler_id.
    InProcess {
        /// Unique identifier for the registered handler function.
        handler_id: String,
    },

    /// Execute local shell command.
    ///
    /// Event data is passed via ASPEN_HOOK_EVENT environment variable.
    /// The command is executed with `sh -c`.
    Shell {
        /// Shell command to execute.
        command: String,
        /// Working directory for the command.
        #[serde(default)]
        working_dir: Option<String>,
    },

    /// Forward to another Aspen cluster's pub/sub.
    ///
    /// Useful for cross-cluster event propagation and analytics pipelines.
    Forward {
        /// Remote cluster's connection ticket (aspen://...).
        target_cluster: String,
        /// Topic to publish to on the remote cluster.
        target_topic: String,
    },
}

impl HookHandlerType {
    /// Validate handler type specific constraints.
    pub fn validate(&self) -> Result<()> {
        match self {
            HookHandlerType::InProcess { handler_id } => {
                if handler_id.is_empty() {
                    return ConfigInvalidSnafu {
                        message: "handler_id cannot be empty".to_string(),
                    }
                    .fail();
                }
                if handler_id.len() > MAX_HANDLER_NAME_SIZE {
                    return ConfigInvalidSnafu {
                        message: format!(
                            "handler_id too long: {} bytes (max {})",
                            handler_id.len(),
                            MAX_HANDLER_NAME_SIZE
                        ),
                    }
                    .fail();
                }
            }
            HookHandlerType::Shell { command, .. } => {
                if command.is_empty() {
                    return ShellCommandEmptySnafu.fail();
                }
                if command.len() > MAX_SHELL_COMMAND_SIZE {
                    return ShellCommandTooLongSnafu {
                        length: command.len() as u32,
                        max: MAX_SHELL_COMMAND_SIZE as u32,
                    }
                    .fail();
                }
            }
            HookHandlerType::Forward {
                target_cluster,
                target_topic,
            } => {
                if target_cluster.is_empty() {
                    return ConfigInvalidSnafu {
                        message: "target_cluster cannot be empty".to_string(),
                    }
                    .fail();
                }
                if target_topic.is_empty() {
                    return ConfigInvalidSnafu {
                        message: "target_topic cannot be empty".to_string(),
                    }
                    .fail();
                }
            }
        }
        Ok(())
    }

    /// Get a type name for logging/metrics.
    pub fn type_name(&self) -> &'static str {
        match self {
            HookHandlerType::InProcess { .. } => "in_process",
            HookHandlerType::Shell { .. } => "shell",
            HookHandlerType::Forward { .. } => "forward",
        }
    }
}

// Default value functions
fn default_enabled() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    DEFAULT_HANDLER_TIMEOUT_MS
}

fn default_retry_count() -> u32 {
    DEFAULT_HANDLER_RETRY_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HookTypeError;

    #[test]
    fn test_default_config() {
        let config = HooksConfig::default();
        assert!(config.is_enabled);
        assert!(config.publish_topics.is_empty());
        assert!(config.handlers.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_handler_config_validation() {
        let config = HookHandlerConfig {
            name: "test".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::InProcess {
                handler_id: "test_handler".to_string(),
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_handler_name_validation() {
        // Empty name
        let config = HookHandlerConfig {
            name: "".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::InProcess {
                handler_id: "test".to_string(),
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(matches!(config.validate(), Err(HookTypeError::HandlerNameEmpty)));

        // Name too long
        let config = HookHandlerConfig {
            name: "x".repeat(MAX_HANDLER_NAME_SIZE + 1),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::InProcess {
                handler_id: "test".to_string(),
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(matches!(config.validate(), Err(HookTypeError::HandlerNameTooLong { .. })));
    }

    #[test]
    fn test_timeout_validation() {
        let config = HookHandlerConfig {
            name: "test".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::InProcess {
                handler_id: "test".to_string(),
            },
            timeout_ms: MAX_HANDLER_TIMEOUT_MS + 1,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(matches!(config.validate(), Err(HookTypeError::InvalidTimeout { .. })));
    }

    #[test]
    fn test_shell_handler_validation() {
        // Empty command
        let config = HookHandlerConfig {
            name: "test".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::Shell {
                command: "".to_string(),
                working_dir: None,
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(matches!(config.validate(), Err(HookTypeError::ShellCommandEmpty)));

        // Command too long
        let config = HookHandlerConfig {
            name: "test".to_string(),
            pattern: "hooks.>".to_string(),
            execution_mode: ExecutionMode::Direct,
            handler_type: HookHandlerType::Shell {
                command: "x".repeat(MAX_SHELL_COMMAND_SIZE + 1),
                working_dir: None,
            },
            timeout_ms: 5000,
            retry_count: 3,
            job_priority: None,
            is_enabled: true,
        };
        assert!(matches!(config.validate(), Err(HookTypeError::ShellCommandTooLong { .. })));
    }

    #[test]
    fn test_duplicate_handler_names() {
        let config = HooksConfig {
            is_enabled: true,
            publish_topics: vec![],
            handlers: vec![
                HookHandlerConfig {
                    name: "duplicate".to_string(),
                    pattern: "hooks.>".to_string(),
                    execution_mode: ExecutionMode::Direct,
                    handler_type: HookHandlerType::InProcess {
                        handler_id: "test1".to_string(),
                    },
                    timeout_ms: 5000,
                    retry_count: 3,
                    job_priority: None,
                    is_enabled: true,
                },
                HookHandlerConfig {
                    name: "duplicate".to_string(),
                    pattern: "hooks.kv.*".to_string(),
                    execution_mode: ExecutionMode::Direct,
                    handler_type: HookHandlerType::InProcess {
                        handler_id: "test2".to_string(),
                    },
                    timeout_ms: 5000,
                    retry_count: 3,
                    job_priority: None,
                    is_enabled: true,
                },
            ],
        };
        assert!(matches!(config.validate(), Err(HookTypeError::ConfigInvalid { .. })));
    }

    #[test]
    fn test_execution_mode_serialization() {
        let direct = ExecutionMode::Direct;
        let job = ExecutionMode::Job;

        assert_eq!(serde_json::to_string(&direct).unwrap(), "\"direct\"");
        assert_eq!(serde_json::to_string(&job).unwrap(), "\"job\"");

        assert_eq!(serde_json::from_str::<ExecutionMode>("\"direct\"").unwrap(), ExecutionMode::Direct);
        assert_eq!(serde_json::from_str::<ExecutionMode>("\"job\"").unwrap(), ExecutionMode::Job);
    }

    #[test]
    fn test_handler_type_serialization() {
        let in_process = HookHandlerType::InProcess {
            handler_id: "test".to_string(),
        };
        let json = serde_json::to_value(&in_process).unwrap();
        assert_eq!(json["type"], "in_process");
        assert_eq!(json["handler_id"], "test");

        let shell = HookHandlerType::Shell {
            command: "echo hello".to_string(),
            working_dir: Some("/tmp".to_string()),
        };
        let json = serde_json::to_value(&shell).unwrap();
        assert_eq!(json["type"], "shell");
        assert_eq!(json["command"], "echo hello");
        assert_eq!(json["working_dir"], "/tmp");

        let forward = HookHandlerType::Forward {
            target_cluster: "aspen://ticket".to_string(),
            target_topic: "events.incoming".to_string(),
        };
        let json = serde_json::to_value(&forward).unwrap();
        assert_eq!(json["type"], "forward");
        assert_eq!(json["target_cluster"], "aspen://ticket");
        assert_eq!(json["target_topic"], "events.incoming");
    }
}
