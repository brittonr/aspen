//! Types for VM-based job execution.

use serde::{Deserialize, Serialize};

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
}