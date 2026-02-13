//! CI build workers.
//!
//! This module re-exports specialized workers for CI/CD job execution from
//! the dedicated executor crates:
//!
//! - `aspen-ci-executor-shell`: LocalExecutorWorker, ResourceLimiter, common utilities
//! - `aspen-ci-executor-vm`: CloudHypervisorWorker, VmPool, VM management
//! - `aspen-ci-executor-nix`: NixBuildWorker for Nix builds
//!
//! # Feature Flags
//!
//! - `shell-executor`: Enables shell/local executor (default)
//! - `vm-executor`: Enables Cloud Hypervisor VM executor
//! - `nix-executor`: Enables Nix build executor
//! - `nix-cache-proxy`: Enables HTTP/3 cache proxy for Nix substitution

// ============================================================================
// Re-exports from aspen-ci-executor-shell (shell-executor feature)
// ============================================================================

// ============================================================================
// Re-exports from aspen-ci-executor-nix (nix-executor feature)
// ============================================================================
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildPayload;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildWorker;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildWorkerConfig;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::UploadedStorePath;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::UploadedStorePathSnix;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ArtifactCollectionResult;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ArtifactUploadResult;
// Cache proxy re-exports (requires nix-cache-proxy feature)
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::CacheProxy;
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::CacheProxyError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::CollectedArtifact;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ExecutionRequest;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ExecutionResult;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::Executor;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorPayload;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorWorker;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorWorkerConfig;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LogMessage;
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::NIX_CACHE_ALPN;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::NetworkIsolation;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::OutputRef;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimiter;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimiterError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimits;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::UploadedArtifact;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::WorkerUtilError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::collect_artifacts;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::create_limiter;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::create_source_archive;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::seed_workspace_from_blob;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::upload_artifacts_to_blob_store;
// ============================================================================
// Re-exports from aspen-ci-executor-vm (vm-executor feature)
// ============================================================================
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::CloudHypervisorError;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::CloudHypervisorPayload;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::CloudHypervisorWorker;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::CloudHypervisorWorkerConfig;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::ManagedCiVm;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::NetworkMode;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::PoolStatus;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::SharedVm;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::VmApiClient;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::VmPool;
#[cfg(feature = "vm-executor")]
pub use aspen_ci_executor_vm::VmState;
