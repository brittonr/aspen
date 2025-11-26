//! Validation configuration

use serde::{Deserialize, Serialize};
use super::error::ConfigError;

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Minimum length for job IDs
    pub job_id_min_length: usize,
    /// Maximum length for job IDs
    pub job_id_max_length: usize,
    /// Maximum size for job payloads in bytes
    pub payload_max_size_bytes: usize,
}

impl ValidationConfig {
    /// Load validation configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            job_id_min_length: Self::parse_env("VALIDATION_JOB_ID_MIN_LENGTH")
                .unwrap_or_else(Self::default_job_id_min_length),
            job_id_max_length: Self::parse_env("VALIDATION_JOB_ID_MAX_LENGTH")
                .unwrap_or_else(Self::default_job_id_max_length),
            payload_max_size_bytes: Self::parse_env("VALIDATION_PAYLOAD_MAX_SIZE_BYTES")
                .unwrap_or_else(Self::default_payload_max_size_bytes),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("VALIDATION_JOB_ID_MIN_LENGTH") {
            self.job_id_min_length = val;
        }
        if let Some(val) = Self::parse_env("VALIDATION_JOB_ID_MAX_LENGTH") {
            self.job_id_max_length = val;
        }
        if let Some(val) = Self::parse_env("VALIDATION_PAYLOAD_MAX_SIZE_BYTES") {
            self.payload_max_size_bytes = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_job_id_min_length() -> usize { 1 }
    fn default_job_id_max_length() -> usize { 255 }
    fn default_payload_max_size_bytes() -> usize { 1024 * 1024 } // 1MB
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            job_id_min_length: Self::default_job_id_min_length(),
            job_id_max_length: Self::default_job_id_max_length(),
            payload_max_size_bytes: Self::default_payload_max_size_bytes(),
        }
    }
}
