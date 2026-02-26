//! LocalExecutorPayload - job payload definition and validation.

use std::collections::HashMap;

use aspen_core::CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS;
use aspen_core::CI_VM_MAX_EXECUTION_TIMEOUT_MS;
use serde::Deserialize;
use serde::Serialize;

/// Maximum command length.
const MAX_COMMAND_LENGTH: usize = 4096;

/// Maximum argument length.
const MAX_ARG_LENGTH: usize = 4096;
/// Maximum total arguments count.
const MAX_ARGS_COUNT: usize = 256;
/// Maximum environment variable count.
const MAX_ENV_COUNT: usize = 256;
/// Maximum artifact glob patterns.
pub(super) const MAX_ARTIFACTS: usize = 64;

/// Job payload for local executor.
///
/// Compatible with CloudHypervisorPayload for easy migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalExecutorPayload {
    /// CI job name for status tracking.
    #[serde(default)]
    pub job_name: Option<String>,

    /// Command to execute.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory relative to workspace.
    #[serde(default = "default_working_dir")]
    pub working_dir: String,

    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Execution timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Glob patterns for artifacts to collect.
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Source hash for workspace setup (blob store key).
    #[serde(default)]
    pub source_hash: Option<String>,

    /// Checkout directory on the host to copy into workspace.
    #[serde(default)]
    pub checkout_dir: Option<String>,

    /// Flake attribute to prefetch for nix commands.
    #[serde(default)]
    pub flake_attr: Option<String>,
}

fn default_working_dir() -> String {
    ".".to_string()
}

fn default_timeout() -> u64 {
    CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS / 1000
}

impl LocalExecutorPayload {
    /// Validate the payload.
    pub fn validate(&self) -> Result<(), String> {
        if self.command.is_empty() {
            return Err("command cannot be empty".to_string());
        }

        if self.command.len() > MAX_COMMAND_LENGTH {
            return Err(format!("command too long: {} bytes (max: {})", self.command.len(), MAX_COMMAND_LENGTH));
        }

        if self.args.len() > MAX_ARGS_COUNT {
            return Err(format!("too many arguments: {} (max: {})", self.args.len(), MAX_ARGS_COUNT));
        }

        for (i, arg) in self.args.iter().enumerate() {
            if arg.len() > MAX_ARG_LENGTH {
                return Err(format!("argument {} too long: {} bytes (max: {})", i, arg.len(), MAX_ARG_LENGTH));
            }
        }

        if self.env.len() > MAX_ENV_COUNT {
            return Err(format!("too many environment variables: {} (max: {})", self.env.len(), MAX_ENV_COUNT));
        }

        let max_timeout = CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000;
        if self.timeout_secs > max_timeout {
            return Err(format!("timeout too long: {} seconds (max: {})", self.timeout_secs, max_timeout));
        }

        if self.artifacts.len() > MAX_ARTIFACTS {
            return Err(format!("too many artifact patterns: {} (max: {})", self.artifacts.len(), MAX_ARTIFACTS));
        }

        Ok(())
    }
}
