//! DNS record resolution and type-specific lookup methods.

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::sync::atomic::Ordering;

use super::super::types::DnsRecord;
use super::super::types::DnsRecordData;
use super::super::types::MxRecord;
use super::super::types::RecordType;
use super::super::types::SrvRecord;
use super::super::validation::wildcard_parent;
use super::AspenDnsClient;

impl AspenDnsClient {
    /// Resolve a DNS record from the local cache.
    ///
    /// This is an instant local lookup - no network I/O.
    /// Supports wildcard matching (tries exact match first, then `*.parent`).
    ///
    /// # Arguments
    /// * `domain` - The domain to resolve (case-insensitive)
    /// * `record_type` - The record type to look up
    ///
    /// # Returns
    /// * `Some(DnsRecord)` - If a matching record exists
    /// * `None` - If no matching record found
    pub async fn resolve(&self, domain: &str, record_type: RecordType) -> Option<DnsRecord> {
        let cache = self.cache.read().await;
        let cache_key = Self::cache_key(domain, record_type);

        // Try exact match first
        if let Some(record) = cache.get(&cache_key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(record.clone());
        }

        // Try wildcard match
        // wildcard_parent("api.example.com") returns "*.example.com"
        if let Some(wildcard_domain) = wildcard_parent(domain) {
            let wildcard_key = Self::cache_key(&wildcard_domain, record_type);
            if let Some(record) = cache.get(&wildcard_key) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(record.clone());
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Look up A records (IPv4 addresses) for a domain.
    ///
    /// Convenience method that returns just the addresses.
    pub async fn lookup_a(&self, domain: &str) -> Option<Vec<Ipv4Addr>> {
        self.resolve(domain, RecordType::A).await.and_then(|r| {
            if let DnsRecordData::A { addresses } = r.data {
                Some(addresses)
            } else {
                None
            }
        })
    }

    /// Look up AAAA records (IPv6 addresses) for a domain.
    ///
    /// Convenience method that returns just the addresses.
    pub async fn lookup_aaaa(&self, domain: &str) -> Option<Vec<Ipv6Addr>> {
        self.resolve(domain, RecordType::AAAA).await.and_then(|r| {
            if let DnsRecordData::AAAA { addresses } = r.data {
                Some(addresses)
            } else {
                None
            }
        })
    }

    /// Look up CNAME record for a domain.
    ///
    /// Convenience method that returns just the target.
    pub async fn lookup_cname(&self, domain: &str) -> Option<String> {
        self.resolve(domain, RecordType::CNAME).await.and_then(|r| {
            if let DnsRecordData::CNAME { target } = r.data {
                Some(target)
            } else {
                None
            }
        })
    }

    /// Look up MX records for a domain.
    ///
    /// Returns records sorted by priority (lowest first).
    pub async fn lookup_mx(&self, domain: &str) -> Vec<MxRecord> {
        self.resolve(domain, RecordType::MX)
            .await
            .map(|r| {
                if let DnsRecordData::MX { mut records } = r.data {
                    records.sort_by_key(|r| r.priority);
                    records
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up TXT records for a domain.
    ///
    /// Returns all TXT strings for the domain.
    pub async fn lookup_txt(&self, domain: &str) -> Vec<String> {
        self.resolve(domain, RecordType::TXT)
            .await
            .map(|r| {
                if let DnsRecordData::TXT { strings } = r.data {
                    strings
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up SRV records for a domain.
    ///
    /// Returns records sorted by priority, then weight (for load balancing).
    pub async fn lookup_srv(&self, domain: &str) -> Vec<SrvRecord> {
        self.resolve(domain, RecordType::SRV)
            .await
            .map(|r| {
                if let DnsRecordData::SRV { mut records } = r.data {
                    records.sort_by_key(|r| (r.priority, std::cmp::Reverse(r.weight)));
                    records
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up NS records for a domain.
    ///
    /// Returns the list of nameservers.
    pub async fn lookup_ns(&self, domain: &str) -> Vec<String> {
        self.resolve(domain, RecordType::NS)
            .await
            .map(|r| {
                if let DnsRecordData::NS { nameservers } = r.data {
                    nameservers
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up PTR record for a domain.
    ///
    /// Used for reverse DNS lookups.
    pub async fn lookup_ptr(&self, domain: &str) -> Option<String> {
        self.resolve(domain, RecordType::PTR).await.and_then(|r| {
            if let DnsRecordData::PTR { target } = r.data {
                Some(target)
            } else {
                None
            }
        })
    }
}
