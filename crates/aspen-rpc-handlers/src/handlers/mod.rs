//! Domain-specific request handlers for Client RPC.

// Handler modules - remaining inline handlers
// Automerge handler moved to aspen-automerge-handler crate
// Blob handler moved to aspen-blob-handler crate
// Cache handler moved to aspen-cache-handler crate
// CI handler moved to aspen-ci-handler crate
pub mod cluster;
// Coordination handler moved to aspen-coordination-handler crate
pub mod core;
// DNS handler moved to aspen-dns-handler crate
pub mod docs;
// Forge handler moved to aspen-forge-handler crate
// Hooks handler moved to aspen-hooks-handler crate
// Job handler moved to aspen-job-handler crate
pub mod kv;
pub mod lease;
// Pijul handler moved to aspen-pijul-handler crate
// Secrets handler moved to aspen-secrets-handler crate
pub mod service_registry;
// SNIX handler moved to aspen-snix-handler crate
// SQL handler moved to aspen-sql-handler crate
pub mod watch;
// Worker handler moved to aspen-job-handler crate

// Re-export handlers
pub use core::CoreHandler;

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
pub use aspen_coordination_handler::CoordinationHandler;
#[cfg(feature = "dns")]
pub use aspen_dns_handler::DnsHandler;
#[cfg(feature = "forge")]
pub use aspen_forge_handler::ForgeHandler;
#[cfg(feature = "hooks")]
pub use aspen_hooks_handler::HooksHandler;
pub use aspen_job_handler::JobHandler;
pub use aspen_job_handler::WorkerHandler;
#[cfg(feature = "pijul")]
pub use aspen_pijul_handler::PijulHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "snix")]
pub use aspen_snix_handler::SnixHandler;
#[cfg(feature = "sql")]
pub use aspen_sql_handler::SqlHandler;
pub use cluster::ClusterHandler;
pub use docs::DocsHandler;
pub use kv::KvHandler;
pub use lease::LeaseHandler;
pub use service_registry::ServiceRegistryHandler;
pub use watch::WatchHandler;
