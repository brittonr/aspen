//! Types for VM-based job execution.

use serde::{Deserialize, Serialize};

/// Payload types for VM-executed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobPayload {
    /// Pre-built native ELF binary.
    NativeBinary {
        /// The binary content.
        #[serde(with = "base64")]
        binary: Vec<u8>,
    },

    /// WebAssembly module for portable execution.
    WasmModule {
        /// The WASM module bytes.
        #[serde(with = "base64")]
        module: Vec<u8>,
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

/// Base64 encoding/decoding for binary data in JSON.
mod base64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        base64::decode(&s).map_err(serde::de::Error::custom)
    }
}

impl JobPayload {
    /// Create a native binary payload.
    pub fn native_binary(binary: Vec<u8>) -> Self {
        Self::NativeBinary { binary }
    }

    /// Create a WASM module payload.
    pub fn wasm_module(module: Vec<u8>) -> Self {
        Self::WasmModule { module }
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