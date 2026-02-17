//! Types for VM-based job execution.

use serde::Deserialize;
use serde::Serialize;

/// Payload types for VM-executed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobPayload {
    /// Blob-stored binary (ELF, WASM, or other executable format).
    /// All VM binaries MUST be stored in iroh-blobs first.
    BlobBinary {
        /// BLAKE3 hash of the binary (hex string).
        hash: String,
        /// Size of the binary in bytes (for validation).
        size: u64,
        /// Binary format hint (e.g., "elf", "wasm", "unknown").
        format: String,
    },

    /// Build from a Nix flake.
    NixExpression {
        /// Flake URL (e.g., "github:user/repo#package").
        flake_url: String,
        /// Attribute path within the flake (e.g., "jobs.dataProcessor").
        attribute: String,
    },

    /// Build from an inline Nix derivation.
    NixDerivation {
        /// Nix expression as a string.
        content: String,
    },

    /// WASM Component stored in blob store.
    WasmComponent {
        /// BLAKE3 hash of the .wasm component (hex string).
        hash: String,
        /// Size of the component in bytes (for validation).
        size: u64,
        /// Fuel limit for execution (None = default).
        fuel_limit: Option<u64>,
        /// Memory limit in bytes (None = default).
        memory_limit: Option<u64>,
    },

    /// Nanvix workload (JS, Python, or native binary) stored in blob store.
    NanvixWorkload {
        /// BLAKE3 hash of the workload file (hex string).
        hash: String,
        /// Size of the workload in bytes (for validation).
        size: u64,
        /// Workload type: "javascript", "python", or "binary".
        workload_type: String,
    },
}

/// Output from a Nix build operation.
#[derive(Debug, Clone, Deserialize)]
pub struct NixBuildOutput {
    /// Path to the built derivation in the Nix store.
    #[serde(rename = "outputs.out")]
    pub out_path: String,
}

impl JobPayload {
    /// Create a blob-stored binary payload.
    /// The binary must already be uploaded to the blob store.
    pub fn blob_binary(hash: impl Into<String>, size: u64, format: impl Into<String>) -> Self {
        Self::BlobBinary {
            hash: hash.into(),
            size,
            format: format.into(),
        }
    }

    /// Create a Nix flake payload.
    pub fn nix_flake(flake_url: impl Into<String>, attribute: impl Into<String>) -> Self {
        Self::NixExpression {
            flake_url: flake_url.into(),
            attribute: attribute.into(),
        }
    }

    /// Create an inline Nix derivation payload.
    pub fn nix_derivation(content: impl Into<String>) -> Self {
        Self::NixDerivation {
            content: content.into(),
        }
    }

    /// Create a WASM component payload.
    /// The component must already be uploaded to the blob store.
    pub fn wasm_component(hash: impl Into<String>, size: u64) -> Self {
        Self::WasmComponent {
            hash: hash.into(),
            size,
            fuel_limit: None,
            memory_limit: None,
        }
    }

    /// Create a WASM component payload with resource limits.
    pub fn wasm_component_with_limits(
        hash: impl Into<String>,
        size: u64,
        fuel_limit: Option<u64>,
        memory_limit: Option<u64>,
    ) -> Self {
        Self::WasmComponent {
            hash: hash.into(),
            size,
            fuel_limit,
            memory_limit,
        }
    }

    /// Create a Nanvix workload payload.
    /// The workload must already be uploaded to the blob store.
    /// `workload_type` should be "javascript", "python", or "binary".
    pub fn nanvix_workload(hash: impl Into<String>, size: u64, workload_type: impl Into<String>) -> Self {
        Self::NanvixWorkload {
            hash: hash.into(),
            size,
            workload_type: workload_type.into(),
        }
    }
}
