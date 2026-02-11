//! Domain-specific request handlers for Client RPC.

// Handler modules
#[cfg(feature = "automerge")]
pub mod automerge;
// Blob handler moved to aspen-blob-handler crate
#[cfg(feature = "ci")]
pub mod cache;
#[cfg(feature = "ci")]
pub mod cache_migration;
// CI handler moved to aspen-ci-handler crate
pub mod cluster;
// Coordination handler moved to aspen-coordination-handler crate
pub mod core;
#[cfg(feature = "dns")]
pub mod dns;
pub mod docs;
// Forge handler moved to aspen-forge-handler crate
pub mod hooks;
pub mod job;
pub mod kv;
pub mod lease;
#[cfg(feature = "pijul")]
pub mod pijul;
// Secrets handler moved to aspen-secrets-handler crate
pub mod service_registry;
pub mod snix;
#[cfg(feature = "sql")]
pub mod sql;
pub mod watch;
pub mod worker;

// Re-export handlers
pub use core::CoreHandler;

#[cfg(feature = "blob")]
pub use aspen_blob_handler::BlobHandler;
#[cfg(feature = "ci")]
pub use aspen_ci_handler::CiHandler;
pub use aspen_coordination_handler::CoordinationHandler;
#[cfg(feature = "forge")]
pub use aspen_forge_handler::ForgeHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsHandler;
#[cfg(feature = "secrets")]
pub use aspen_secrets_handler::SecretsService;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeHandler;
#[cfg(feature = "ci")]
pub use cache::CacheHandler;
#[cfg(feature = "ci")]
pub use cache_migration::CacheMigrationHandler;
pub use cluster::ClusterHandler;
#[cfg(feature = "dns")]
pub use dns::DnsHandler;
pub use docs::DocsHandler;
pub use hooks::HooksHandler;
pub use job::JobHandler;
pub use kv::KvHandler;
pub use lease::LeaseHandler;
#[cfg(feature = "pijul")]
pub use pijul::PijulHandler;
pub use service_registry::ServiceRegistryHandler;
pub use snix::SnixHandler;
#[cfg(feature = "sql")]
pub use sql::SqlHandler;
pub use watch::WatchHandler;
pub use worker::WorkerHandler;
