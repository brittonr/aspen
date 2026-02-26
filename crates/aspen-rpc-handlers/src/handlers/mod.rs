//! Domain-specific request handlers for Client RPC.
//!
//! Handler types:
//!
//! **Direct RequestHandler** (tightly-coupled control plane):
//!   Cluster    → aspen-cluster-handler (Raft control plane, membership)
//!   Core/Lease/Watch → aspen-core-essentials-handler (Raft metrics, streaming)
//!
//! **ServiceExecutor → ServiceHandler** (typed, auto-registered):
//!   Blob       → aspen-blob-handler (BlobServiceExecutor, 16 ops)
//!   CI         → aspen-ci-handler (CiServiceExecutor, 11 ops)
//!   Docs       → aspen-docs-handler (DocsServiceExecutor, 13 ops)
//!   Forge      → aspen-forge-handler (ForgeServiceExecutor, 15 ops)
//!   Job/Worker → aspen-job-handler (JobServiceExecutor, 10 ops)
//!   Secrets    → aspen-secrets-handler (SecretsServiceExecutor, 15 ops)
//!
//! **WASM plugins** (third-party sandboxed):
//!   KV, SQL, Hooks, Coordination, Automerge, ServiceRegistry, DNS, etc.

// Re-export handlers
#[cfg(feature = "blob")]
pub use aspen_blob_handler::BlobServiceExecutor;
#[cfg(feature = "ci")]
pub use aspen_ci_handler::CiServiceExecutor;
pub use aspen_cluster_handler::ClusterHandler;
pub use aspen_core_essentials_handler::CoreHandler;
pub use aspen_core_essentials_handler::LeaseHandler;
pub use aspen_core_essentials_handler::WatchHandler;
#[cfg(feature = "docs")]
pub use aspen_docs_handler::DocsServiceExecutor;
#[cfg(feature = "forge")]
pub use aspen_forge_handler::ForgeServiceExecutor;
#[cfg(feature = "jobs")]
pub use aspen_job_handler::JobServiceExecutor;
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheHandler;
#[cfg(feature = "ci")]
pub use aspen_nix_handler::CacheMigrationHandler;
#[cfg(feature = "snix")]
pub use aspen_nix_handler::SnixHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsServiceExecutor;
#[cfg(feature = "plugins-rpc")]
pub use aspen_wasm_plugin;
