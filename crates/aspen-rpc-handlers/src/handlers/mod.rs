//! Domain-specific request handlers for Client RPC.

// Native handler crates:
//   Blob       → aspen-blob-handler
//   Cache/SNIX → aspen-nix-handler
//   CI         → aspen-ci-handler
//   Cluster    → aspen-cluster-handler
//   Core/Lease/Watch → aspen-core-essentials-handler
//   DNS/SQL    → aspen-query-handler
//   Docs       → aspen-docs-handler
//   Forge      → aspen-forge-handler
//   Hooks      → aspen-hooks-handler
//   Job/Worker → aspen-job-handler
//   KV         → aspen-kv-handler
//   Secrets    → aspen-secrets-handler (PKI + NixCache only)
//
// Migrated to WASM plugins (native handlers deleted):
//   Automerge        → aspen-automerge-plugin
//   Coordination     → aspen-coordination-plugin
//   Secrets KV/Transit → aspen-secrets-plugin
//   Service Registry → aspen-service-registry-plugin

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
// DNS handler migrated to aspen-dns-plugin (WASM)
#[cfg(feature = "sql")]
pub use aspen_query_handler::SqlHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "plugins-rpc")]
pub use aspen_wasm_plugin;
