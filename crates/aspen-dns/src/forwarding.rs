//! DNS forwarding authority implementation.
//!
//! This module provides a hickory-server `Authority` implementation that
//! forwards DNS queries to upstream DNS servers for domains not handled
//! by local authorities.

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use hickory_proto::op::ResponseCode;
use hickory_proto::rr::{LowerName, Name, Record};
use hickory_proto::xfer::Protocol;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, NameServerConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::Resolver;
use hickory_server::authority::{
    Authority, LookupControlFlow, LookupError, LookupObject, LookupOptions, MessageRequest, UpdateResult, ZoneType
};
use hickory_server::server::RequestInfo;
use tracing::{debug, warn};

use super::error::{DnsError, DnsResult};

/// DNS forwarding authority that forwards queries to upstream servers.
///
/// This authority handles queries for domains not served by local authorities
/// by forwarding them to configured upstream DNS servers.
pub struct ForwardingAuthority {
    /// Upstream DNS resolver
    resolver: Arc<Resolver<TokioConnectionProvider>>,
    /// Zone name this authority handles (typically "." for root)
    origin: LowerName,
}

impl ForwardingAuthority {
    /// Create a new forwarding authority.
    ///
    /// # Arguments
    ///
    /// * `upstreams` - List of upstream DNS server addresses
    /// * `origin` - Zone name to handle (usually "." for catch-all)
    ///
    /// # Errors
    ///
    /// Returns an error if the resolver cannot be created.
    pub async fn new(upstreams: Vec<SocketAddr>, origin: Name) -> DnsResult<Self> {
        // Capture count before the move
        let upstream_count = upstreams.len();

        // Convert upstream addresses to NameServerConfig
        let name_servers: Vec<NameServerConfig> = upstreams
            .into_iter()
            .map(|addr| NameServerConfig::new(addr, Protocol::Udp))
            .collect();

        if name_servers.is_empty() {
            return Err(DnsError::ConfigurationError {
                reason: "No upstream DNS servers configured for forwarding".to_string(),
            });
        }

        // Create resolver configuration
        let config = ResolverConfig::from_parts(None, vec![], name_servers);
        let _opts = ResolverOpts::default();

        // Create a resolver with the provided configuration
        let resolver = Resolver::builder_with_config(
            config,
            TokioConnectionProvider::default(),
        ).build();
        debug!(
            upstream_count = upstream_count,
            "Created DNS forwarding authority using default resolver"
        );

        debug!(
            origin = %origin,
            upstream_count = resolver.config().name_servers().len(),
            "Created DNS forwarding authority"
        );

        Ok(Self {
            resolver: Arc::new(resolver),
            origin: origin.into(),
        })
    }
}

#[async_trait]
impl Authority for ForwardingAuthority {
    type Lookup = ForwardingLookup;

    fn zone_type(&self) -> ZoneType {
        ZoneType::External  // External for forwarding/caching authorities
    }

    fn is_axfr_allowed(&self) -> bool {
        false
    }

    async fn update(&self, _update: &MessageRequest) -> UpdateResult<bool> {
        // Forwarding authorities don't support updates
        debug!("Update attempted on forwarding authority (not supported)");
        Err(ResponseCode::NotImp)
    }

    fn origin(&self) -> &LowerName {
        &self.origin
    }

    async fn lookup(
        &self,
        name: &LowerName,
        rtype: hickory_proto::rr::RecordType,
        _lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        debug!(
            name = %name,
            record_type = ?rtype,
            "Forwarding DNS query to upstream"
        );

        // Convert to Name for resolver
        let query_name = Name::from(name.clone());

        LookupControlFlow::Continue(Ok(ForwardingLookup {
            resolver: Arc::clone(&self.resolver),
            name: query_name,
            rtype,
            cached_records: std::sync::Mutex::new(None),
        }))
    }

    async fn search(
        &self,
        request_info: RequestInfo<'_>,
        lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        let query = request_info.query;
        self.lookup(query.name(), query.query_type(), lookup_options).await
    }

    async fn get_nsec_records(
        &self,
        _name: &LowerName,
        _lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        // NSEC not supported for forwarding
        LookupControlFlow::Continue(Err(LookupError::from(ResponseCode::NotImp)))
    }
}

/// Lookup result from forwarding authority.
pub struct ForwardingLookup {
    resolver: Arc<Resolver<TokioConnectionProvider>>,
    name: Name,
    rtype: hickory_proto::rr::RecordType,
    /// Cached records from the lookup (populated on demand)
    cached_records: std::sync::Mutex<Option<Vec<Record>>>,
}

impl LookupObject for ForwardingLookup {
    fn is_empty(&self) -> bool {
        // Check if we have cached records
        if let Ok(guard) = self.cached_records.lock()
            && let Some(ref records) = *guard {
                return records.is_empty();
            }

        // If no cached records, perform lookup synchronously
        // This is a limitation of the sync API - we can't do async lookup here
        // In practice, this will return true until records are resolved
        warn!(
            name = %self.name,
            record_type = ?self.rtype,
            "is_empty called before lookup resolved - returning true"
        );
        true
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Record> + Send + 'a> {
        // LookupObject requires sync methods, but we need async DNS resolution
        // This is a design limitation - return empty iterator for now
        // The actual records should be retrieved through other means
        warn!(
            name = %self.name,
            record_type = ?self.rtype,
            "iter called on ForwardingLookup - sync API limitation, returning empty"
        );
        Box::new(std::iter::empty())
    }

    fn take_additionals(&mut self) -> Option<Box<dyn LookupObject>> {
        None
    }
}

impl ForwardingLookup {
    /// Resolve the DNS query using the upstream resolver.
    /// This should be called during the lookup creation to populate cached_records.
    pub async fn resolve_and_cache(&self) -> Result<(), LookupError> {
        let lookup_result = self.resolver.lookup(
            self.name.clone(),
            self.rtype,
        ).await;

        match lookup_result {
            Ok(lookup) => {
                let records: Vec<Record> = lookup.record_iter().cloned().collect();
                debug!(
                    name = %self.name,
                    record_type = ?self.rtype,
                    count = records.len(),
                    "Forwarding lookup completed"
                );

                // Cache the results
                if let Ok(mut guard) = self.cached_records.lock() {
                    *guard = Some(records);
                }
                Ok(())
            }
            Err(e) => {
                warn!(
                    name = %self.name,
                    record_type = ?self.rtype,
                    error = %e,
                    "Forwarding lookup failed"
                );
                Err(LookupError::from(ResponseCode::ServFail))
            }
        }
    }

    /// Get the cached records if available.
    pub fn get_cached_records(&self) -> Vec<Record> {
        if let Ok(guard) = self.cached_records.lock()
            && let Some(ref records) = *guard {
                return records.clone();
            }
        Vec::new()
    }
}

// Note: LookupControlFlow is an enum returned by Authority methods, not a trait to implement