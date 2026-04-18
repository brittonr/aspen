//! Name resolver with local cache.
//!
//! Resolves service names to (endpoint_id, port) pairs. Maintains a local
//! cache that is refreshed periodically from the registry.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use aspen_traits::KeyValueStore;
use snafu::ResultExt;
use snafu::Snafu;
use tokio::sync::RwLock;

use crate::constants::NET_REGISTRY_POLL_INTERVAL_SECS;
use crate::registry::RegistryError;
use crate::registry::ServiceRegistry;
use crate::types::ServiceEntry;

/// Errors from name resolution.
#[derive(Debug, Snafu)]
pub enum ResolverError {
    /// Failed to refresh cache from registry.
    #[snafu(display("registry error: {source}"))]
    Registry { source: RegistryError },
}

/// Cached service entries with TTL tracking.
struct CacheState {
    entries: HashMap<String, ServiceEntry>,
    last_refresh: Option<Instant>,
}

#[inline]
#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "net resolver cache refresh stores monotonic timestamps for staleness checks"
)]
fn monotonic_now() -> Instant {
    Instant::now()
}

/// Name resolver with local cache.
///
/// Resolves service names to (endpoint_id, port) tuples.
/// The cache is refreshed when entries expire (based on poll interval).
pub struct NameResolver<S: KeyValueStore> {
    registry: Arc<ServiceRegistry<S>>,
    cache: RwLock<CacheState>,
}

impl<S: KeyValueStore + 'static> NameResolver<S> {
    /// Create a new resolver backed by the given registry.
    pub fn new(registry: Arc<ServiceRegistry<S>>) -> Self {
        Self {
            registry,
            cache: RwLock::new(CacheState {
                entries: HashMap::new(),
                last_refresh: None,
            }),
        }
    }

    /// Resolve a service name to (endpoint_id, port).
    ///
    /// Strips `.aspen` suffix if present. Checks local cache first,
    /// falling back to a registry refresh if the cache is stale or
    /// the name is not found.
    pub async fn resolve(&self, name: &str) -> Result<Option<(String, u16)>, ResolverError> {
        let name = strip_aspen_suffix(name);

        // Check cache first
        {
            let cache = self.cache.read().await;
            if !is_cache_stale(&cache) {
                if let Some(entry) = cache.entries.get(name) {
                    return Ok(Some((entry.endpoint_id.clone(), entry.port)));
                }
                // Cache is fresh and name not found
                return Ok(None);
            }
        }

        // Cache is stale, refresh
        self.refresh_cache().await?;

        // Try again with fresh cache
        let cache = self.cache.read().await;
        match cache.entries.get(name) {
            Some(entry) => Ok(Some((entry.endpoint_id.clone(), entry.port))),
            None => Ok(None),
        }
    }

    /// Refresh the local cache from the registry.
    pub async fn refresh_cache(&self) -> Result<(), ResolverError> {
        let entries = self.registry.list(None).await.context(RegistrySnafu)?;

        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            map.insert(entry.name.clone(), entry);
        }

        let mut cache = self.cache.write().await;
        cache.entries = map;
        cache.last_refresh = Some(monotonic_now());

        Ok(())
    }
}

/// Check if the cache is stale (expired or never refreshed).
fn is_cache_stale(cache: &CacheState) -> bool {
    match cache.last_refresh {
        None => true,
        Some(last) => last.elapsed().as_secs() >= NET_REGISTRY_POLL_INTERVAL_SECS,
    }
}

/// Strip `.aspen` suffix from a service name if present.
fn strip_aspen_suffix(name: &str) -> &str {
    name.strip_suffix(".aspen").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;
    use crate::registry::ServiceRegistry;
    use crate::types::ServiceEntry;

    fn test_entry(name: &str) -> ServiceEntry {
        ServiceEntry {
            name: name.to_string(),
            endpoint_id: "abc123def456".to_string(),
            port: 5432,
            proto: "tcp".to_string(),
            tags: vec![],
            hostname: None,
            published_at_ms: 1000,
        }
    }

    fn make_resolver() -> (NameResolver<DeterministicKeyValueStore>, Arc<ServiceRegistry<DeterministicKeyValueStore>>) {
        let store = DeterministicKeyValueStore::new();
        let registry = Arc::new(ServiceRegistry::new(store));
        let resolver = NameResolver::new(Arc::clone(&registry));
        (resolver, registry)
    }

    #[tokio::test]
    async fn resolve_known_service() {
        let (resolver, registry) = make_resolver();
        registry.publish(test_entry("mydb")).await.unwrap();

        let result = resolver.resolve("mydb").await.unwrap();
        assert_eq!(result, Some(("abc123def456".to_string(), 5432)));
    }

    #[tokio::test]
    async fn resolve_unknown_service() {
        let (resolver, _registry) = make_resolver();
        let result = resolver.resolve("missing").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn resolve_strips_aspen_suffix() {
        let (resolver, registry) = make_resolver();
        registry.publish(test_entry("mydb")).await.unwrap();

        let result = resolver.resolve("mydb.aspen").await.unwrap();
        assert_eq!(result, Some(("abc123def456".to_string(), 5432)));
    }

    #[tokio::test]
    async fn cache_hit_avoids_refresh() {
        let (resolver, registry) = make_resolver();
        registry.publish(test_entry("mydb")).await.unwrap();

        // First resolve populates cache
        resolver.resolve("mydb").await.unwrap();

        // Second resolve should hit cache
        let result = resolver.resolve("mydb").await.unwrap();
        assert_eq!(result, Some(("abc123def456".to_string(), 5432)));
    }

    #[test]
    fn strip_suffix_works() {
        assert_eq!(strip_aspen_suffix("mydb.aspen"), "mydb");
        assert_eq!(strip_aspen_suffix("mydb"), "mydb");
        assert_eq!(strip_aspen_suffix("multi.dot.aspen"), "multi.dot");
    }
}
