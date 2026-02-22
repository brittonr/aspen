//! Domain-specific request handlers for Client RPC.

// All handlers have been extracted to separate crates:
// Automerge handler migrated to aspen-automerge-plugin (WASM)
// Blob handler moved to aspen-blob-handler crate
// Cache handler moved to aspen-nix-handler crate
// CI handler moved to aspen-ci-handler crate
// Cluster handler moved to aspen-cluster-handler crate
// Coordination handler migrated to aspen-coordination-plugin (WASM)
// Core handler moved to aspen-core-essentials-handler crate
// DNS handler moved to aspen-query-handler crate
// Docs handler moved to aspen-docs-handler crate
// Forge handler moved to aspen-forge-handler crate
// Hooks handler moved to aspen-hooks-handler crate
// Job handler moved to aspen-job-handler crate
// KV handler moved to aspen-kv-handler crate
// Lease handler moved to aspen-core-essentials-handler crate
// Secrets handler moved to aspen-secrets-handler crate
// Service registry handler migrated to aspen-service-registry-plugin (WASM)
// SNIX handler moved to aspen-nix-handler crate
// SQL handler moved to aspen-query-handler crate
// Watch handler moved to aspen-core-essentials-handler crate
// Worker handler moved to aspen-job-handler crate

// Re-export handlers
#[cfg(feature = "blob")]
pub use aspen_blob_handler::BlobHandler;
#[cfg(feature = "ci")]
pub use aspen_ci_handler::CiHandler;
pub use aspen_cluster_handler::ClusterHandler;
pub use aspen_core_essentials_handler::CoreHandler;
pub use aspen_core_essentials_handler::LeaseHandler;
pub use aspen_core_essentials_handler::WatchHandler;
#[cfg(feature = "docs")]
pub use aspen_docs_handler::DocsHandler;
#[cfg(feature = "forge")]
pub use aspen_forge_handler::ForgeHandler;
#[cfg(feature = "hooks")]
pub use aspen_hooks_handler::HooksHandler;
#[cfg(feature = "jobs")]
pub use aspen_job_handler::JobHandler;
#[cfg(feature = "jobs")]
pub use aspen_job_handler::WorkerHandler;
pub use aspen_kv_handler::KvHandler;
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheHandler;
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheMigrationHandler;
#[cfg(feature = "snix")]
pub use aspen_nix_handler::SnixHandler;
#[cfg(feature = "dns")]
pub use aspen_query_handler::DnsHandler;
#[cfg(feature = "sql")]
pub use aspen_query_handler::SqlHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "plugins-rpc")]
pub use aspen_wasm_plugin;
