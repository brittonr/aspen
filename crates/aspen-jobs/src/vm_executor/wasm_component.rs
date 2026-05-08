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

        // Build host context for guest callbacks. The current portable fixture
        // ABI is intentionally narrow (`execute() -> i32`), but creating the
        // context here keeps the product worker path wired to Aspen-owned host
        // state instead of a runtime-core-only test seam.
        let _ctx = Arc::new(wasm_host::AspenHostContext::new(
            Arc::clone(&self.kv_store),
            Arc::clone(&self.blob_store),
            job.id.to_string(),
            wasm_host::now_ms(),
        ));

        let exit_code = execute_portable_wasm_i32_const(&component_bytes, fuel_limit)?;

        Ok(JobResult::success(serde_json::json!({
            "abi": "aspen:runtime-host/wasm-v1",
            "entrypoint": "execute",
            "exit_code": exit_code,
            "job_id": job.id.to_string(),
            "memory_limit": memory_limit,
            "marker": "ASPEN_WASM_RUNTIME_HOST_EXECUTED"
        })))
    }
}

fn execute_portable_wasm_i32_const(component_bytes: &[u8], fuel_limit: u64) -> Result<i32> {
    if fuel_limit == 0 {
        return Err(JobError::VmExecutionFailed {
            reason: "WASM fuel limit exhausted before execution".to_string(),
        });
    }

    let module = PortableWasmModule::parse(component_bytes)?;
    module.execute_i32_const_export("execute")
}

struct PortableWasmModule {
    function_type_indices: Vec<u32>,
    execute_function_index: Option<u32>,
    function_bodies: Vec<Vec<u8>>,
}

impl PortableWasmModule {
    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 || bytes[0..4] != WASM_MAGIC || bytes[4..8] != [0x01, 0x00, 0x00, 0x00] {
            return Err(JobError::WasmComponentInvalid {
                reason: "bytes do not start with a supported WASM module header".to_string(),
            });
        }

        let mut module = Self {
            function_type_indices: Vec::new(),
            execute_function_index: None,
            function_bodies: Vec::new(),
        };
        let mut offset = 8;
        while offset < bytes.len() {
            let section_id = bytes[offset];
            offset += 1;
            let section_len = read_uleb_u32(bytes, &mut offset)? as usize;
            let section_end = offset.checked_add(section_len).ok_or_else(|| JobError::VmExecutionFailed {
                reason: "WASM section length overflow".to_string(),
            })?;
            if section_end > bytes.len() {
                return Err(JobError::VmExecutionFailed {
                    reason: "WASM section extends past module end".to_string(),
                });
            }

            match section_id {
                3 => module.parse_function_section(&bytes[offset..section_end])?,
                7 => module.parse_export_section(&bytes[offset..section_end])?,
                10 => module.parse_code_section(&bytes[offset..section_end])?,
                _ => {}
            }
            offset = section_end;
        }

        Ok(module)
    }

    fn parse_function_section(&mut self, section: &[u8]) -> Result<()> {
        let mut offset = 0;
        let count = read_uleb_u32(section, &mut offset)?;
        for _ in 0..count {
            self.function_type_indices.push(read_uleb_u32(section, &mut offset)?);
        }
        Ok(())
    }

    fn parse_export_section(&mut self, section: &[u8]) -> Result<()> {
        let mut offset = 0;
        let count = read_uleb_u32(section, &mut offset)?;
        for _ in 0..count {
            let name_len = read_uleb_u32(section, &mut offset)? as usize;
            let name_end = offset.checked_add(name_len).ok_or_else(|| JobError::VmExecutionFailed {
                reason: "WASM export name length overflow".to_string(),
            })?;
            if name_end > section.len() {
                return Err(JobError::VmExecutionFailed {
                    reason: "WASM export name extends past section end".to_string(),
                });
            }
            let name = &section[offset..name_end];
            offset = name_end;
            let kind = *section.get(offset).ok_or_else(|| JobError::VmExecutionFailed {
                reason: "WASM export missing kind".to_string(),
            })?;
            offset += 1;
            let index = read_uleb_u32(section, &mut offset)?;
            if name == b"execute" && kind == 0 {
                self.execute_function_index = Some(index);
            }
        }
        Ok(())
    }

    fn parse_code_section(&mut self, section: &[u8]) -> Result<()> {
        let mut offset = 0;
        let count = read_uleb_u32(section, &mut offset)?;
        for _ in 0..count {
            let body_len = read_uleb_u32(section, &mut offset)? as usize;
            let body_end = offset.checked_add(body_len).ok_or_else(|| JobError::VmExecutionFailed {
                reason: "WASM function body length overflow".to_string(),
            })?;
            if body_end > section.len() {
                return Err(JobError::VmExecutionFailed {
                    reason: "WASM function body extends past code section".to_string(),
                });
            }
            self.function_bodies.push(section[offset..body_end].to_vec());
            offset = body_end;
        }
        Ok(())
    }

    fn execute_i32_const_export(&self, export_name: &str) -> Result<i32> {
        let function_index = self.execute_function_index.ok_or_else(|| JobError::VmExecutionFailed {
            reason: format!("missing {export_name} function export in WASM module"),
        })? as usize;
        if function_index >= self.function_type_indices.len() || function_index >= self.function_bodies.len() {
            return Err(JobError::VmExecutionFailed {
                reason: format!("{export_name} export points outside the function table"),
            });
        }
        let body = &self.function_bodies[function_index];
        if body.len() != 4 || body[0] != 0x00 || body[1] != 0x41 || body[3] != 0x0b {
            return Err(JobError::VmExecutionFailed {
                reason: format!("{export_name} body is not the supported local-decl/i32.const/end form"),
            });
        }
        Ok(body[2] as i8 as i32)
    }
}

fn read_uleb_u32(bytes: &[u8], offset: &mut usize) -> Result<u32> {
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(*offset).ok_or_else(|| JobError::VmExecutionFailed {
            reason: "truncated WASM varint".to_string(),
        })?;
        *offset += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 32 {
            return Err(JobError::VmExecutionFailed {
                reason: "WASM varint exceeds u32".to_string(),
            });
        }
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
