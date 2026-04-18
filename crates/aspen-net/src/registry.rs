//! Service registry backed by Raft KV.
//!
//! Provides CRUD operations for publishing and discovering named services.
//! All state is stored under `/_sys/net/svc/` and `/_sys/net/node/` KV prefixes.

use std::sync::Arc;

use aspen_traits::KeyValueStore;
use snafu::ResultExt;
use snafu::Snafu;

use crate::constants::MAX_NET_SERVICES;
use crate::constants::MAX_NET_TAGS_PER_SERVICE;
use crate::constants::NET_NODE_PREFIX;
use crate::constants::NET_SVC_PREFIX;
use crate::types::NodeEntry;
use crate::types::ServiceEntry;
use crate::verified::service_name::is_valid_service_name;

/// Errors from service registry operations.
#[derive(Debug, Snafu)]
pub enum RegistryError {
    /// Service name failed validation.
    #[snafu(display("invalid service name '{name}': {reason}"))]
    InvalidName { name: String, reason: String },

    /// Too many tags on the service entry.
    #[snafu(display("service '{name}' has {count} tags, max is {max}"))]
    TooManyTags { name: String, count: u32, max: u32 },

    /// Cluster has reached the maximum number of services.
    #[snafu(display("service limit reached ({max} services)"))]
    ServiceLimitReached { max: u32 },

    /// KV store operation failed.
    #[snafu(display("kv error: {source}"))]
    Kv {
        source: aspen_core::error::KeyValueStoreError,
    },

    /// JSON serialization/deserialization failed.
    #[snafu(display("json error: {source}"))]
    Json { source: serde_json::Error },
}

/// Service registry backed by the distributed KV store.
///
/// Provides publish, unpublish, lookup, list operations for named services.
/// All writes go through Raft consensus via the `KeyValueStore` trait.
pub struct ServiceRegistry<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized> ServiceRegistry<S> {
    /// Create a new registry wrapping the given KV store.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Publish a service to the registry.
    ///
    /// Validates the service name and tag count, then writes to KV.
    /// If a service with the same name exists, it is overwritten.
    /// Also updates the node entry for the service's endpoint.
    pub async fn publish(&self, entry: ServiceEntry) -> Result<(), RegistryError> {
        // Validate name
        if !is_valid_service_name(&entry.name) {
            return Err(RegistryError::InvalidName {
                name: entry.name.clone(),
                reason: "must match [a-z0-9][a-z0-9.-]{0,252}".to_string(),
            });
        }

        // Validate tag count
        let tag_count = entry.tags.len() as u32;
        if tag_count > MAX_NET_TAGS_PER_SERVICE {
            return Err(RegistryError::TooManyTags {
                name: entry.name.clone(),
                count: tag_count,
                max: MAX_NET_TAGS_PER_SERVICE,
            });
        }

        // Check service count limit (only for new services)
        let existing = self.lookup(&entry.name).await?;
        if existing.is_none() {
            let all = self.list(None).await?;
            if all.len() as u32 >= MAX_NET_SERVICES {
                return Err(RegistryError::ServiceLimitReached { max: MAX_NET_SERVICES });
            }
        }

        // Serialize and write
        let key = format!("{NET_SVC_PREFIX}{}", entry.name);
        let value = serde_json::to_string(&entry).context(JsonSnafu)?;
        self.store.write(aspen_kv_types::WriteRequest::set(key, value)).await.context(KvSnafu)?;

        // Update node entry
        self.update_node_entry_add(&entry.endpoint_id, &entry.name).await?;

        Ok(())
    }

    /// Remove a service from the registry.
    ///
    /// Idempotent — succeeds even if the service doesn't exist.
    pub async fn unpublish(&self, name: &str) -> Result<(), RegistryError> {
        // Look up existing entry to get endpoint_id for node update
        let existing = self.lookup(name).await?;

        let key = format!("{NET_SVC_PREFIX}{name}");
        self.store.delete(aspen_kv_types::DeleteRequest::new(key)).await.context(KvSnafu)?;

        // Update node entry if we found an existing service
        if let Some(entry) = existing {
            self.update_node_entry_remove(&entry.endpoint_id, name).await?;
        }

        Ok(())
    }

    /// Look up a service by name.
    ///
    /// Returns `None` if no service with the given name is registered.
    pub async fn lookup(&self, name: &str) -> Result<Option<ServiceEntry>, RegistryError> {
        let key = format!("{NET_SVC_PREFIX}{name}");
        let result = self.store.read(aspen_kv_types::ReadRequest::new(key)).await;

        match result {
            Ok(r) => match r.kv {
                Some(kv) => {
                    let entry: ServiceEntry = serde_json::from_str(&kv.value).context(JsonSnafu)?;
                    Ok(Some(entry))
                }
                None => Ok(None),
            },
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e).context(KvSnafu),
        }
    }

    /// List all registered services, optionally filtered by tag.
    pub async fn list(&self, tag_filter: Option<&str>) -> Result<Vec<ServiceEntry>, RegistryError> {
        let scan_result = self
            .store
            .scan(aspen_kv_types::ScanRequest {
                prefix: NET_SVC_PREFIX.to_string(),
                limit_results: Some(MAX_NET_SERVICES),
                continuation_token: None,
            })
            .await
            .context(KvSnafu)?;

        let mut entries = Vec::with_capacity(scan_result.entries.len());
        for kv in &scan_result.entries {
            let entry: ServiceEntry = serde_json::from_str(&kv.value).context(JsonSnafu)?;
            if let Some(tag) = tag_filter {
                if entry.tags.iter().any(|t| t == tag) {
                    entries.push(entry);
                }
            } else {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// List all registered nodes.
    pub async fn list_nodes(&self) -> Result<Vec<NodeEntry>, RegistryError> {
        let scan_result = self
            .store
            .scan(aspen_kv_types::ScanRequest {
                prefix: NET_NODE_PREFIX.to_string(),
                limit_results: Some(MAX_NET_SERVICES),
                continuation_token: None,
            })
            .await
            .context(KvSnafu)?;

        let mut nodes = Vec::with_capacity(scan_result.entries.len());
        for kv in &scan_result.entries {
            let node: NodeEntry = serde_json::from_str(&kv.value).context(JsonSnafu)?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Add a service name to a node's entry.
    async fn update_node_entry_add(&self, endpoint_id: &str, service_name: &str) -> Result<(), RegistryError> {
        let key = format!("{NET_NODE_PREFIX}{endpoint_id}");
        let result = self.store.read(aspen_kv_types::ReadRequest::new(&key)).await;

        let mut node = match result {
            Ok(r) => match r.kv {
                Some(kv) => serde_json::from_str::<NodeEntry>(&kv.value).context(JsonSnafu)?,
                None => new_node_entry(endpoint_id),
            },
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => new_node_entry(endpoint_id),
            Err(e) => return Err(e).context(KvSnafu),
        };

        if !node.services.contains(&service_name.to_string()) {
            node.services.push(service_name.to_string());
        }

        let value = serde_json::to_string(&node).context(JsonSnafu)?;
        self.store.write(aspen_kv_types::WriteRequest::set(key, value)).await.context(KvSnafu)?;

        Ok(())
    }

    /// Remove a service name from a node's entry.
    async fn update_node_entry_remove(&self, endpoint_id: &str, service_name: &str) -> Result<(), RegistryError> {
        let key = format!("{NET_NODE_PREFIX}{endpoint_id}");
        let result = self.store.read(aspen_kv_types::ReadRequest::new(&key)).await;

        let kv = match result {
            Ok(r) => r.kv,
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => None,
            Err(e) => return Err(e).context(KvSnafu),
        };

        if let Some(kv) = kv {
            let mut node: NodeEntry = serde_json::from_str(&kv.value).context(JsonSnafu)?;
            node.services.retain(|s| s != service_name);

            let value = serde_json::to_string(&node).context(JsonSnafu)?;
            self.store.write(aspen_kv_types::WriteRequest::set(key, value)).await.context(KvSnafu)?;
        }

        Ok(())
    }
}

/// Create a new node entry with defaults.
fn new_node_entry(endpoint_id: &str) -> NodeEntry {
    NodeEntry {
        endpoint_id: endpoint_id.to_string(),
        hostname: gethostname(),
        tags: Vec::new(),
        services: Vec::new(),
        last_seen_ms: 0,
    }
}

/// Get the local hostname, falling back to "unknown".
fn gethostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;

    fn make_registry() -> ServiceRegistry<DeterministicKeyValueStore> {
        let store = DeterministicKeyValueStore::new();
        ServiceRegistry::new(store)
    }

    fn test_entry(name: &str) -> ServiceEntry {
        ServiceEntry {
            name: name.to_string(),
            endpoint_id: "abc123def456".to_string(),
            port: 5432,
            proto: "tcp".to_string(),
            tags: vec!["db".to_string(), "prod".to_string()],
            hostname: None,
            published_at_ms: 1000,
        }
    }

    #[tokio::test]
    async fn publish_and_lookup() {
        let reg = make_registry();
        let entry = test_entry("mydb");
        reg.publish(entry.clone()).await.unwrap();

        let found = reg.lookup("mydb").await.unwrap();
        assert_eq!(found.unwrap(), entry);
    }

    #[tokio::test]
    async fn lookup_nonexistent() {
        let reg = make_registry();
        let found = reg.lookup("missing").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn unpublish_existing() {
        let reg = make_registry();
        reg.publish(test_entry("mydb")).await.unwrap();
        reg.unpublish("mydb").await.unwrap();

        let found = reg.lookup("mydb").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn unpublish_nonexistent_is_ok() {
        let reg = make_registry();
        reg.unpublish("missing").await.unwrap();
    }

    #[tokio::test]
    async fn list_all() {
        let reg = make_registry();
        reg.publish(test_entry("mydb")).await.unwrap();
        reg.publish(test_entry("web")).await.unwrap();
        reg.publish(test_entry("cache")).await.unwrap();

        let all = reg.list(None).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn list_by_tag() {
        let reg = make_registry();
        let mut db_entry = test_entry("mydb");
        db_entry.tags = vec!["db".to_string()];
        reg.publish(db_entry).await.unwrap();

        let mut web_entry = test_entry("web");
        web_entry.tags = vec!["frontend".to_string()];
        reg.publish(web_entry).await.unwrap();

        let db_only = reg.list(Some("db")).await.unwrap();
        assert_eq!(db_only.len(), 1);
        assert_eq!(db_only[0].name, "mydb");
    }

    #[tokio::test]
    async fn duplicate_name_overwrites() {
        let reg = make_registry();
        let mut entry1 = test_entry("mydb");
        entry1.port = 5432;
        reg.publish(entry1).await.unwrap();

        let mut entry2 = test_entry("mydb");
        entry2.port = 5433;
        entry2.published_at_ms = 2000;
        reg.publish(entry2.clone()).await.unwrap();

        let found = reg.lookup("mydb").await.unwrap().unwrap();
        assert_eq!(found.port, 5433);
        assert_eq!(found.published_at_ms, 2000);
    }

    #[tokio::test]
    async fn invalid_name_rejected() {
        let reg = make_registry();
        let mut entry = test_entry("");
        entry.name = String::new();
        assert!(reg.publish(entry).await.is_err());

        let mut entry = test_entry("UPPERCASE");
        entry.name = "UPPERCASE".to_string();
        assert!(reg.publish(entry).await.is_err());
    }

    #[tokio::test]
    async fn too_many_tags_rejected() {
        let reg = make_registry();
        let mut entry = test_entry("mydb");
        entry.tags = (0..33).map(|i| format!("tag{i}")).collect();
        assert!(matches!(reg.publish(entry).await, Err(RegistryError::TooManyTags { .. })));
    }

    #[tokio::test]
    async fn node_entry_updated_on_publish() {
        let reg = make_registry();
        reg.publish(test_entry("mydb")).await.unwrap();
        reg.publish(test_entry("cache")).await.unwrap();

        let nodes = reg.list_nodes().await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].services.contains(&"mydb".to_string()));
        assert!(nodes[0].services.contains(&"cache".to_string()));
    }
}
