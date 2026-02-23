//! Domain-specific request handlers for Client RPC.

// Native handler crates (require native system access):
//   Blob       → aspen-blob-handler (network I/O, replication, DHT)
//   Cache/SNIX → aspen-nix-handler (snix-castore, protobuf, iroh-blobs)
//   CI         → aspen-ci-handler (cross-cutting orchestration)
//   Cluster    → aspen-cluster-handler (Raft control plane, membership)
//   Core/Lease/Watch → aspen-core-essentials-handler (Raft metrics, streaming)
//   Docs       → aspen-docs-handler (iroh-docs sync, peer federation)
//   Forge      → aspen-forge-handler (git bridge, federation only)
//   Job/Worker → aspen-job-handler (distributed queue orchestration)
//   Secrets    → aspen-secrets-handler (PKI/X.509 crypto only)
//
// Migrated to WASM plugins (native handlers deleted):
//   Automerge        → aspen-automerge-plugin
//   Coordination     → aspen-coordination-plugin
//   DNS              → aspen-dns-plugin
//   Hooks            → aspen-hooks-plugin
//   KV               → aspen-kv-plugin
//   Secrets KV/Transit → aspen-secrets-plugin
//   Service Registry → aspen-service-registry-plugin
//   SQL              → aspen-sql-plugin

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
#[cfg(feature = "jobs")]
pub use aspen_job_handler::JobHandler;
#[cfg(feature = "jobs")]
pub use aspen_job_handler::WorkerHandler;
// KV handler migrated to WASM plugin (aspen-kv-plugin)
// SQL handler migrated to WASM plugin (aspen-sql-plugin)
// Hooks handler migrated to WASM plugin (aspen-hooks-plugin)
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheHandler;
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheMigrationHandler;
#[cfg(feature = "snix")]
pub use aspen_nix_handler::SnixHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "plugins-rpc")]
pub use aspen_wasm_plugin;
