//! Domain-specific request handlers for Client RPC.

// All handlers have been extracted to separate crates:
// Automerge handler moved to aspen-automerge-handler crate
// Blob handler moved to aspen-blob-handler crate
// Cache handler moved to aspen-cache-handler crate
// CI handler moved to aspen-ci-handler crate
// Cluster handler moved to aspen-cluster-handler crate
// Coordination handler moved to aspen-coordination-handler crate
// Core handler moved to aspen-core-handler crate
// DNS handler moved to aspen-dns-handler crate
// Docs handler moved to aspen-docs-handler crate
// Forge handler moved to aspen-forge-handler crate
// Hooks handler moved to aspen-hooks-handler crate
// Job handler moved to aspen-job-handler crate
// KV handler moved to aspen-kv-handler crate
// Lease handler moved to aspen-lease-handler crate
// Pijul handler moved to aspen-pijul-handler crate
// Secrets handler moved to aspen-secrets-handler crate
// Service registry handler moved to aspen-service-registry-handler crate
// SNIX handler moved to aspen-snix-handler crate
// SQL handler moved to aspen-sql-handler crate
// Watch handler moved to aspen-watch-handler crate
// Worker handler moved to aspen-job-handler crate

// Re-export handlers
#[cfg(feature = "automerge")]
pub use aspen_automerge_handler::AutomergeHandler;
#[cfg(feature = "blob")]
pub use aspen_blob_handler::BlobHandler;
#[cfg(feature = "ci")]
pub use aspen_cache_handler::CacheHandler;
#[cfg(feature = "ci")]
pub use aspen_cache_handler::CacheMigrationHandler;
#[cfg(feature = "ci")]
pub use aspen_ci_handler::CiHandler;
pub use aspen_cluster_handler::ClusterHandler;
pub use aspen_coordination_handler::CoordinationHandler;
pub use aspen_core_handler::CoreHandler;
#[cfg(feature = "dns")]
pub use aspen_dns_handler::DnsHandler;
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
pub use aspen_lease_handler::LeaseHandler;
#[cfg(feature = "pijul")]
pub use aspen_pijul_handler::PijulHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
pub use aspen_service_registry_handler::ServiceRegistryHandler;
#[cfg(feature = "snix")]
pub use aspen_snix_handler::SnixHandler;
#[cfg(feature = "sql")]
pub use aspen_sql_handler::SqlHandler;
pub use aspen_watch_handler::WatchHandler;
