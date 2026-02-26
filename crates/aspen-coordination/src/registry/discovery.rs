//! Service discovery operations.

use anyhow::Result;
use aspen_constants::coordination::MAX_SERVICE_DISCOVERY_RESULTS;
use aspen_traits::KeyValueStore;

use super::ServiceRegistry;
use super::types::DiscoveryFilter;
use super::types::ServiceInstance;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Discover all instances of a service.
    ///
    /// Automatically cleans up expired instances during discovery.
    pub async fn discover(&self, service_name: &str, filter: DiscoveryFilter) -> Result<Vec<ServiceInstance>> {
        // Cleanup expired instances first
        let _ = self.cleanup_expired(service_name).await;

        let prefix = verified::service_instances_prefix(service_name);
        let limit = filter.limit.unwrap_or(MAX_SERVICE_DISCOVERY_RESULTS).min(MAX_SERVICE_DISCOVERY_RESULTS);

        let keys = self.scan_keys(&prefix, limit).await?;
        let mut instances = Vec::new();

        for key in keys {
            // Skip the service metadata key (no instance ID suffix)
            if key == verified::services_scan_prefix(service_name) {
                continue;
            }

            if let Some(instance) = self.read_json::<ServiceInstance>(&key).await?
                && !instance.is_expired()
            {
                // Apply filters using pure function
                if !crate::verified::matches_discovery_filter(
                    instance.health_status,
                    &instance.metadata.tags,
                    &instance.metadata.version,
                    filter.healthy_only,
                    &filter.tags,
                    filter.version_prefix.as_deref(),
                ) {
                    continue;
                }

                instances.push(instance);

                if instances.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(instances)
    }

    /// Discover services by name prefix.
    ///
    /// Returns service names matching the prefix.
    pub async fn discover_services(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
        let full_prefix = verified::services_scan_prefix(prefix);
        let limit = limit.min(MAX_SERVICE_DISCOVERY_RESULTS);

        let keys = self.scan_keys(&full_prefix, limit).await?;

        // Extract unique service names from keys
        let mut services = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for key in keys {
            // Key format: __service:{name}:{instance_id}
            if let Some(rest) = key.strip_prefix(verified::SERVICE_PREFIX)
                && let Some(colon_pos) = rest.find(':')
            {
                let service_name = &rest[..colon_pos];
                if seen.insert(service_name.to_string()) {
                    services.push(service_name.to_string());
                }
            }
        }

        Ok(services)
    }

    /// Get a specific service instance.
    pub async fn get_instance(&self, service_name: &str, instance_id: &str) -> Result<Option<ServiceInstance>> {
        let key = Self::instance_key(service_name, instance_id);

        if let Some(instance) = self.read_json::<ServiceInstance>(&key).await? {
            if instance.is_expired() {
                // Lazily delete expired instance
                let _ = self.delete_key(&key).await;
                return Ok(None);
            }
            return Ok(Some(instance));
        }

        Ok(None)
    }
}
