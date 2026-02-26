//! WASM Component Model worker using hyperlight-wasm.
//!
//! Executes WASM Component Model binaries in hardware-isolated sandboxes
//! via hyperlight-wasm, with typed WIT host interfaces for logging,
//! KV store, blob store, and clock access.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::vm_executor::types::JobPayload;
use crate::vm_executor::wasm_host;
use crate::worker::Worker;

/// WASM magic bytes: `\0asm`.
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// Worker that executes WASM Component Model binaries in hyperlight-wasm sandboxes.
pub struct WasmComponentWorker {
    /// KV store for guest key-value operations.
    kv_store: Arc<dyn KeyValueStore>,
    /// Blob store for retrieving WASM components.
    blob_store: Arc<dyn BlobStore>,
}

impl WasmComponentWorker {
    /// Create a new WASM component worker.
    ///
    /// Requires a KV store for guest operations and a blob store for
    /// retrieving component binaries.
    pub fn new(kv_store: Arc<dyn KeyValueStore>, blob_store: Arc<dyn BlobStore>) -> Result<Self> {
        Ok(Self { kv_store, blob_store })
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

        // Build host context for guest callbacks.
        let ctx = Arc::new(wasm_host::AspenHostContext::new(
            Arc::clone(&self.kv_store),
            Arc::clone(&self.blob_store),
            job.id.to_string(),
            wasm_host::now_ms(),
        ));

        // Build sandbox (requires KVM/hypervisor at runtime).
        // State machine: ProtoWasmSandbox -> WasmSandbox -> LoadedWasmSandbox
        let mut proto =
            hyperlight_wasm::SandboxBuilder::new().with_guest_heap_size(memory_limit).build().map_err(|e| {
                JobError::VmExecutionFailed {
                    reason: format!("failed to create WASM sandbox: {e}"),
                }
            })?;

        // Register host functions (primitive mode) before loading runtime.
        wasm_host::register_host_functions(&mut proto, Arc::clone(&ctx))?;

        let wasm_sb = proto.load_runtime().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to load WASM runtime: {e}"),
        })?;

        let mut loaded =
            wasm_sb.load_module_from_buffer(&component_bytes).map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to load WASM module: {e}"),
            })?;

        // Prepare input bytes from the job payload.
        let input_bytes: Vec<u8> = serde_json::to_vec(&job.spec.payload).unwrap_or_default();

        // Call the guest's "execute" export.
        // TODO: Pass fuel_limit when hyperlight-wasm exposes fuel configuration.
        let _fuel_limit = fuel_limit;
        let output: Vec<u8> =
            loaded.call_guest_function("execute", input_bytes).map_err(|e| JobError::VmExecutionFailed {
                reason: format!("guest execution failed: {e}"),
            })?;

        // Parse output as JSON or wrap raw bytes.
        let result: serde_json::Value = serde_json::from_slice(&output).unwrap_or_else(|_| {
            serde_json::json!({
                "raw_output": String::from_utf8_lossy(&output)
            })
        });

        Ok(JobResult::success(result))
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
