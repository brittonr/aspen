//! DNS integration for the service mesh.
//!
//! Automatically creates/deletes SRV and A records when services are
//! published/unpublished. Uses a loopback address allocator to assign
//! stable `127.0.0.{2..255}` addresses to service names.

#[cfg(feature = "dns")]
mod inner {
    use std::collections::HashMap;
    use std::net::Ipv4Addr;
    use std::sync::Arc;

    use tokio::sync::RwLock;
    use tracing::debug;
    use tracing::warn;

    use crate::constants::MAX_NET_DNS_LOOPBACK_ADDRS;
    use crate::constants::NET_DNS_TTL_SECS;
    use crate::types::ServiceEntry;

    /// Loopback address allocator for DNS A records.
    ///
    /// Assigns addresses from the `127.0.0.{2..255}` range to service names.
    /// Uses LRU eviction when the pool is exhausted.
    pub struct LoopbackAllocator {
        /// service_name → assigned address
        assignments: RwLock<HashMap<String, Ipv4Addr>>,
        /// next address to allocate (2..255)
        next_octet: RwLock<u8>,
        /// LRU order: most recently used at end
        lru: RwLock<Vec<String>>,
    }

    impl LoopbackAllocator {
        /// Create a new loopback allocator.
        pub fn new() -> Self {
            Self {
                assignments: RwLock::new(HashMap::new()),
                next_octet: RwLock::new(2),
                lru: RwLock::new(Vec::new()),
            }
        }

        /// Get or allocate a loopback address for a service name.
        ///
        /// Returns the same address for the same service name (stable mapping).
        /// When the pool is full (254 addresses), evicts the LRU entry.
        pub async fn allocate(&self, service_name: &str) -> Ipv4Addr {
            // Check existing assignment
            {
                let assignments = self.assignments.read().await;
                if let Some(&addr) = assignments.get(service_name) {
                    // Touch LRU
                    let mut lru = self.lru.write().await;
                    if let Some(pos) = lru.iter().position(|s| s == service_name) {
                        lru.remove(pos);
                    }
                    lru.push(service_name.to_string());
                    return addr;
                }
            }

            // Allocate new address
            let mut assignments = self.assignments.write().await;
            let mut next = self.next_octet.write().await;
            let mut lru = self.lru.write().await;

            // Double-check after acquiring write lock
            if let Some(&addr) = assignments.get(service_name) {
                return addr;
            }

            let addr = if assignments.len() < MAX_NET_DNS_LOOPBACK_ADDRS as usize {
                // Pool has space
                let octet = *next;
                *next = next.saturating_add(1);
                Ipv4Addr::new(127, 0, 0, octet)
            } else {
                // Pool full — evict LRU
                if let Some(evicted) = lru.first().cloned() {
                    let evicted_addr = assignments.remove(&evicted).unwrap_or(Ipv4Addr::new(127, 0, 0, 2));
                    lru.remove(0);
                    evicted_addr
                } else {
                    // Shouldn't happen but handle gracefully
                    Ipv4Addr::new(127, 0, 0, 2)
                }
            };

            assignments.insert(service_name.to_string(), addr);
            lru.push(service_name.to_string());
            debug!(service = service_name, addr = %addr, "allocated loopback address");
            addr
        }

        /// Release a loopback address for a service name.
        pub async fn release(&self, service_name: &str) {
            let mut assignments = self.assignments.write().await;
            let mut lru = self.lru.write().await;
            assignments.remove(service_name);
            if let Some(pos) = lru.iter().position(|s| s == service_name) {
                lru.remove(pos);
            }
        }

        /// Get the current address for a service, if any.
        pub async fn get(&self, service_name: &str) -> Option<Ipv4Addr> {
            self.assignments.read().await.get(service_name).copied()
        }
    }

    /// DNS record manager for auto-creating records on publish/unpublish.
    ///
    /// Wraps an `aspen_dns::DnsStore` and a `LoopbackAllocator` to
    /// automatically manage SRV and A records.
    pub struct DnsRecordManager<D: aspen_dns::DnsStore> {
        dns_store: Arc<D>,
        allocator: LoopbackAllocator,
        zone: String,
    }

    impl<D: aspen_dns::DnsStore> DnsRecordManager<D> {
        /// Create a new DNS record manager.
        pub fn new(dns_store: Arc<D>, zone: String) -> Self {
            Self {
                dns_store,
                allocator: LoopbackAllocator::new(),
                zone,
            }
        }

        /// Create DNS records for a published service.
        ///
        /// Creates:
        /// - A record: `{name}.{zone}` → loopback address
        /// - SRV record: `_{proto}.{name}.{zone}` → `{name}.{zone}:{port}`
        pub async fn on_publish(&self, entry: &ServiceEntry) -> Result<(), String> {
            let domain = format!("{}.{}", entry.name, self.zone);
            let srv_domain = format!("_{}.{}.{}", entry.proto, entry.name, self.zone);

            // Allocate loopback address
            let addr = self.allocator.allocate(&entry.name).await;

            // Create A record
            let a_record = aspen_dns::DnsRecord::new(domain.clone(), NET_DNS_TTL_SECS, aspen_dns::DnsRecordData::A {
                addresses: vec![addr],
            });

            if let Err(e) = self.dns_store.set_record(a_record).await {
                warn!(domain = %domain, error = %e, "failed to create A record");
                return Err(format!("failed to create A record: {e}"));
            }

            // Create SRV record
            let srv_record =
                aspen_dns::DnsRecord::new(srv_domain.clone(), NET_DNS_TTL_SECS, aspen_dns::DnsRecordData::SRV {
                    records: vec![aspen_dns::SrvRecord::new(0, 100, entry.port, &domain)],
                });

            if let Err(e) = self.dns_store.set_record(srv_record).await {
                warn!(domain = %srv_domain, error = %e, "failed to create SRV record");
                return Err(format!("failed to create SRV record: {e}"));
            }

            debug!(
                service = entry.name,
                a_domain = domain,
                srv_domain = srv_domain,
                addr = %addr,
                port = entry.port,
                "created DNS records for service"
            );
            Ok(())
        }

        /// Delete DNS records for an unpublished service.
        pub async fn on_unpublish(&self, service_name: &str, proto: &str) -> Result<(), String> {
            let domain = format!("{}.{}", service_name, self.zone);
            let srv_domain = format!("_{}.{}.{}", proto, service_name, self.zone);

            // Delete A record
            if let Err(e) = self.dns_store.delete_record(&domain, aspen_dns::RecordType::A).await {
                warn!(domain = %domain, error = %e, "failed to delete A record");
            }

            // Delete SRV record
            if let Err(e) = self.dns_store.delete_record(&srv_domain, aspen_dns::RecordType::SRV).await {
                warn!(domain = %srv_domain, error = %e, "failed to delete SRV record");
            }

            // Release loopback address
            self.allocator.release(service_name).await;

            debug!(service = service_name, "deleted DNS records for service");
            Ok(())
        }

        /// Get the loopback address assigned to a service.
        pub async fn get_address(&self, service_name: &str) -> Option<Ipv4Addr> {
            self.allocator.get(service_name).await
        }
    }
}

#[cfg(feature = "dns")]
pub use inner::DnsRecordManager;
#[cfg(feature = "dns")]
pub use inner::LoopbackAllocator;

// =========================================================================
// Tests (always available — allocator has no DNS dep for unit tests)
// =========================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    /// Standalone loopback allocator test (no DNS dependency needed).
    #[tokio::test]
    async fn test_loopback_allocator_basic() {
        // Inline a minimal allocator for testing without dns feature
        use tokio::sync::RwLock;

        struct TestAllocator {
            assignments: RwLock<HashMap<String, Ipv4Addr>>,
            next_octet: RwLock<u8>,
        }

        impl TestAllocator {
            fn new() -> Self {
                Self {
                    assignments: RwLock::new(HashMap::new()),
                    next_octet: RwLock::new(2),
                }
            }

            async fn allocate(&self, name: &str) -> Ipv4Addr {
                let assignments = self.assignments.read().await;
                if let Some(&addr) = assignments.get(name) {
                    return addr;
                }
                drop(assignments);

                let mut assignments = self.assignments.write().await;
                if let Some(&addr) = assignments.get(name) {
                    return addr;
                }
                let mut next = self.next_octet.write().await;
                let octet = *next;
                *next = next.saturating_add(1);
                let addr = Ipv4Addr::new(127, 0, 0, octet);
                assignments.insert(name.to_string(), addr);
                addr
            }
        }

        let alloc = TestAllocator::new();

        // First allocation
        let a1 = alloc.allocate("svc-a").await;
        assert_eq!(a1, Ipv4Addr::new(127, 0, 0, 2));

        // Second allocation
        let a2 = alloc.allocate("svc-b").await;
        assert_eq!(a2, Ipv4Addr::new(127, 0, 0, 3));

        // Same name returns same address
        let a1_again = alloc.allocate("svc-a").await;
        assert_eq!(a1_again, Ipv4Addr::new(127, 0, 0, 2));
    }
}
