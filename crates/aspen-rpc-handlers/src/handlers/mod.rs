//! Domain-specific request handlers for Client RPC.
//!
//! Handler types:
//!
//! **Direct RequestHandler** (native, complex integration):
//!   Blob       → aspen-blob-handler (network I/O, replication, DHT)
//!   Cache/SNIX → aspen-nix-handler (snix-castore, protobuf, iroh-blobs)
//!   CI         → aspen-ci-handler (cross-cutting orchestration)
//!   Cluster    → aspen-cluster-handler (Raft control plane, membership)
//!   Core/Lease/Watch → aspen-core-essentials-handler (Raft metrics, streaming)
//!   Forge      → aspen-forge-handler (git bridge, federation only)
//!   Secrets    → aspen-secrets-handler (PKI/X.509 crypto only)
//!
//! **ServiceExecutor → ServiceHandler** (typed, auto-registered):
//!   Docs       → aspen-docs-handler (DocsServiceExecutor, 13 ops)
//!   Job/Worker → aspen-job-handler (JobServiceExecutor, 10 ops)
//!
//! **WASM plugins** (third-party sandboxed):
//!   KV, SQL, Hooks, Coordination, Automerge, ServiceRegistry, DNS, etc.

// Re-export handlers
#[cfg(feature = "blob")]
pub use aspen_blob_handler::BlobHandler;
#[cfg(feature = "ci")]
pub use aspen_ci_handler::CiHandler;
pub use aspen_cluster_handler::ClusterHandler;
pub use aspen_core_essentials_handler::CoreHandler;
pub use aspen_core_essentials_handler::LeaseHandler;
pub use aspen_core_essentials_handler::WatchHandler;
// Docs: ServiceExecutor auto-registered via submit_handler_factory!
#[cfg(feature = "docs")]
pub use aspen_docs_handler::DocsServiceExecutor;
#[cfg(feature = "forge")]
pub use aspen_forge_handler::ForgeHandler;
// Jobs: ServiceExecutor auto-registered via submit_handler_factory!
#[cfg(feature = "jobs")]
pub use aspen_job_handler::JobServiceExecutor;
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
