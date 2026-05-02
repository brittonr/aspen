//! Types for VM-based job execution.

pub use aspen_jobs_core::JobPayload;
use serde::Deserialize;

/// Output from a Nix build operation.
#[derive(Debug, Clone, Deserialize)]
pub struct NixBuildOutput {
    /// Path to the built derivation in the Nix store.
    #[serde(rename = "outputs.out")]
    pub out_path: String,
}
