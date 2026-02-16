//! WASM Component Model worker using hyperlight-wasm.
//!
//! Executes WASM Component Model binaries in hardware-isolated sandboxes
//! via hyperlight-wasm, with typed WIT host interfaces for logging,
//! KV store, blob store, and clock access.

use std::sync::Arc;

use aspen_blob::prelude::*;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::vm_executor::types::JobPayload;
use crate::worker::Worker;

/// WASM magic bytes: `\0asm`.
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// Worker that executes WASM Component Model binaries in hyperlight-wasm sandboxes.
pub struct WasmComponentWorker {
    /// Blob store for retrieving WASM components.
    blob_store: Arc<dyn BlobStore>,
}

impl WasmComponentWorker {
    /// Create a new WASM component worker.
    ///
    /// Requires a blob store for retrieving component binaries.
    pub fn new(blob_store: Arc<dyn BlobStore>) -> Result<Self> {
        Ok(Self { blob_store })
    }

    /// Retrieve a WASM component from the blob store.
    ///
    /// Validates the blob hash, expected size, and maximum component size.
    async fn retrieve_component(&self, hash: &str, expected_size: u64) -> Result<Vec<u8>> {
        info!(hash, expected_size, "retrieving WASM component from blob store");

        let blob_hash = hash.parse::<iroh_blobs::Hash>().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("invalid blob hash '{}': {}", hash, e),
        })?;

        let bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to retrieve blob: {}", e),
            })?
            .ok_or_else(|| JobError::VmExecutionFailed {
                reason: format!("blob not found: {}", hash),
            })?;

        // Validate size if provided (0 means skip validation).
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(JobError::VmExecutionFailed {
                reason: format!("blob size mismatch: expected {} bytes, got {} bytes", expected_size, bytes.len()),
            });
        }

        if bytes.len() as u64 > aspen_constants::wasm::MAX_WASM_COMPONENT_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: aspen_constants::wasm::MAX_WASM_COMPONENT_SIZE,
            });
        }

        Ok(bytes.to_vec())
    }

    /// Validate and clamp resource limits for WASM execution.
    ///
    /// Returns `(fuel_limit, memory_limit)` clamped to the configured maximums,
    /// defaulting to the configured defaults when `None` is supplied.
    fn validate_resource_limits(fuel_limit: Option<u64>, memory_limit: Option<u64>) -> (u64, u64) {
        let fuel = fuel_limit
            .unwrap_or(aspen_constants::wasm::DEFAULT_WASM_FUEL_LIMIT)
            .min(aspen_constants::wasm::MAX_WASM_FUEL_LIMIT);

        let memory = memory_limit
            .unwrap_or(aspen_constants::wasm::DEFAULT_WASM_MEMORY_LIMIT)
            .min(aspen_constants::wasm::MAX_WASM_MEMORY_LIMIT);

        debug!(fuel, memory, "validated WASM resource limits");
        (fuel, memory)
    }

    /// Execute a WASM component in a hyperlight-wasm sandbox.
    ///
    /// Validates the component magic bytes and delegates to the sandbox
    /// for isolated execution with the configured resource limits.
    async fn execute_component(
        &self,
        component_bytes: Vec<u8>,
        job: &Job,
        fuel_limit: u64,
        memory_limit: u64,
    ) -> Result<JobResult> {
        info!(
            job_id = %job.id,
            component_size = component_bytes.len(),
            fuel_limit,
            memory_limit,
            "executing WASM component"
        );

        // Validate WASM magic bytes.
        if component_bytes.len() < WASM_MAGIC.len() || component_bytes[..4] != WASM_MAGIC {
            return Err(JobError::WasmComponentInvalid {
                reason: "bytes do not start with WASM magic (\\0asm)".to_string(),
            });
        }

        // TODO: Wire up hyperlight-wasm sandbox execution when the crate stabilizes.
        // The implementation will:
        // 1. Create SandboxConfiguration with fuel_limit and memory_limit
        // 2. Create UninitializedSandbox with GuestBinary::Buffer(&component_bytes)
        // 3. Register host functions via wasm_host::register_host_functions()
        // 4. Evolve to MultiUseSandbox
        // 5. Call guest "execute" export with job input
        Err(JobError::VmExecutionFailed {
            reason: "hyperlight-wasm execution not yet available; sandbox integration pending crate stabilization"
                .to_string(),
        })
    }
}

#[async_trait]
impl Worker for WasmComponentWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let payload: JobPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("failed to parse job payload: {}", e));
            }
        };

        let result = match payload {
            JobPayload::WasmComponent {
                hash,
                size,
                fuel_limit,
                memory_limit,
            } => {
                let component_bytes = match self.retrieve_component(&hash, size).await {
                    Ok(bytes) => bytes,
                    Err(e) => return JobResult::failure(format!("failed to retrieve WASM component: {}", e)),
                };

                let (fuel, memory) = Self::validate_resource_limits(fuel_limit, memory_limit);

                self.execute_component(component_bytes, &job, fuel, memory).await
            }

            other => {
                warn!(job_id = %job.id, payload_type = ?other, "WasmComponentWorker received non-WASM payload");
                Err(JobError::VmExecutionFailed {
                    reason: "WasmComponentWorker only handles WasmComponent payloads".to_string(),
                })
            }
        };

        match result {
            Ok(job_result) => job_result,
            Err(e) => JobResult::failure(format!("WASM execution failed: {}", e)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["wasm_component".to_string()]
    }
}
