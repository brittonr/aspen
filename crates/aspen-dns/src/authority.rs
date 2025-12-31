//! DNS Authority implementation backed by AspenDnsClient cache.
//!
//! This module provides a hickory-server `Authority` implementation that
//! serves DNS records from the Aspen DNS layer's local cache. It enables
//! the DNS protocol server to respond to queries using records stored
//! in the distributed Aspen cluster.
//!
//! # Architecture
//!
//! ```text
//! AspenDnsClient (local cache)
//!         ↓
//! AspenDnsAuthority (Authority trait)
//!         ↓
//! hickory-server (DNS protocol)
//!         ↓
//! UDP/TCP DNS queries
//! ```

use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use hickory_proto::op::ResponseCode;
use hickory_proto::rr::LowerName;
use hickory_proto::rr::Name;
use hickory_proto::rr::RData;
use hickory_proto::rr::Record;
use hickory_proto::rr::RecordSet;
use hickory_proto::rr::RecordType as HickoryRecordType;
use hickory_proto::rr::rdata::CAA;
use hickory_proto::rr::rdata::MX;
use hickory_proto::rr::rdata::SOA;
use hickory_proto::rr::rdata::SRV;
use hickory_proto::rr::rdata::TXT;
use hickory_server::authority::Authority;
use hickory_server::authority::LookupControlFlow;
use hickory_server::authority::LookupError;
use hickory_server::authority::LookupObject;
use hickory_server::authority::LookupOptions;
use hickory_server::authority::MessageRequest;
use hickory_server::authority::UpdateResult;
use hickory_server::authority::ZoneType;
use hickory_server::server::RequestInfo;
use tracing::debug;
use tracing::trace;
use tracing::warn;

use super::client::AspenDnsClient;
use super::types::DnsRecord;
use super::types::DnsRecordData;
use super::types::RecordType as AspenRecordType;

/// A lookup result containing DNS records.
pub struct AspenLookup {
    records: Arc<RecordSet>,
}

impl AspenLookup {
    fn new(records: RecordSet) -> Self {
        Self {
            records: Arc::new(records),
        }
    }

    fn empty(name: Name, record_type: HickoryRecordType) -> Self {
        Self {
            records: Arc::new(RecordSet::new(name, record_type, 0)),
        }
    }
}

impl LookupObject for AspenLookup {
    fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Record> + Send + 'a> {
        Box::new(self.records.records_without_rrsigs())
    }

    fn take_additionals(&mut self) -> Option<Box<dyn LookupObject>> {
        None
    }
}

/// Aspen DNS Authority - serves DNS records from local cache.
///
/// This implements the hickory-server `Authority` trait to serve DNS queries
/// from the `AspenDnsClient` local cache. It provides:
///
/// - Fast local lookups (no network I/O after initial sync)
/// - Wildcard record support (`*.example.com`)
/// - All standard DNS record types (A, AAAA, MX, TXT, SRV, etc.)
///
/// Tiger Style: Bounded cache, explicit error handling, fail-fast on conversion errors.
pub struct AspenDnsAuthority {
    /// Reference to the DNS client cache.
    client: Arc<AspenDnsClient>,
    /// Zone origin (e.g., "aspen.local.").
    origin: LowerName,
    /// Zone type (always Primary for Aspen).
    zone_type: ZoneType,
}

impl AspenDnsAuthority {
    /// Create a new Aspen DNS Authority for the given zone.
    ///
    /// # Arguments
    ///
    /// * `client` - The DNS client with local cache
    /// * `zone` - The zone name (e.g., "aspen.local")
    pub fn new(client: Arc<AspenDnsClient>, zone: &str) -> io::Result<Self> {
        // Ensure zone ends with a dot for proper DNS name formatting
        let zone_fqdn = if zone.ends_with('.') {
            zone.to_string()
        } else {
            format!("{}.", zone)
        };

        let name = Name::from_utf8(&zone_fqdn).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        Ok(Self {
            client,
            origin: LowerName::new(&name),
            zone_type: ZoneType::Primary,
        })
    }

    /// Convert Aspen RecordType to hickory RecordType.
    #[allow(dead_code)]
    fn to_hickory_record_type(rt: AspenRecordType) -> HickoryRecordType {
        match rt {
            AspenRecordType::A => HickoryRecordType::A,
            AspenRecordType::AAAA => HickoryRecordType::AAAA,
            AspenRecordType::CNAME => HickoryRecordType::CNAME,
            AspenRecordType::MX => HickoryRecordType::MX,
            AspenRecordType::TXT => HickoryRecordType::TXT,
            AspenRecordType::SRV => HickoryRecordType::SRV,
            AspenRecordType::NS => HickoryRecordType::NS,
            AspenRecordType::SOA => HickoryRecordType::SOA,
            AspenRecordType::PTR => HickoryRecordType::PTR,
            AspenRecordType::CAA => HickoryRecordType::CAA,
        }
    }

    /// Convert hickory RecordType to Aspen RecordType.
    fn from_hickory_record_type(rt: HickoryRecordType) -> Option<AspenRecordType> {
        match rt {
            HickoryRecordType::A => Some(AspenRecordType::A),
            HickoryRecordType::AAAA => Some(AspenRecordType::AAAA),
            HickoryRecordType::CNAME => Some(AspenRecordType::CNAME),
            HickoryRecordType::MX => Some(AspenRecordType::MX),
            HickoryRecordType::TXT => Some(AspenRecordType::TXT),
            HickoryRecordType::SRV => Some(AspenRecordType::SRV),
            HickoryRecordType::NS => Some(AspenRecordType::NS),
            HickoryRecordType::SOA => Some(AspenRecordType::SOA),
            HickoryRecordType::PTR => Some(AspenRecordType::PTR),
            HickoryRecordType::CAA => Some(AspenRecordType::CAA),
            _ => None,
        }
    }

    /// Convert an Aspen DnsRecord to hickory Records.
    fn convert_record(&self, record: &DnsRecord) -> Vec<Record<RData>> {
        let name = match Name::from_utf8(format!("{}.", record.domain)) {
            Ok(n) => n,
            Err(e) => {
                warn!(domain = %record.domain, error = %e, "Failed to parse domain name");
                return vec![];
            }
        };

        let ttl = record.ttl_seconds;
        let mut records = Vec::new();

        match &record.data {
            DnsRecordData::A { addresses } => {
                for addr in addresses {
                    let rdata = RData::A((*addr).into());
                    records.push(Record::from_rdata(name.clone(), ttl, rdata));
                }
            }
            DnsRecordData::AAAA { addresses } => {
                for addr in addresses {
                    let rdata = RData::AAAA((*addr).into());
                    records.push(Record::from_rdata(name.clone(), ttl, rdata));
                }
            }
            DnsRecordData::CNAME { target } => {
                if let Ok(target_name) = Name::from_utf8(format!("{}.", target)) {
                    let rdata = RData::CNAME(hickory_proto::rr::rdata::CNAME(target_name));
                    records.push(Record::from_rdata(name, ttl, rdata));
                }
            }
            DnsRecordData::MX { records: mx_records } => {
                for mx in mx_records {
                    if let Ok(exchange) = Name::from_utf8(format!("{}.", mx.exchange)) {
                        let rdata = RData::MX(MX::new(mx.priority, exchange));
                        records.push(Record::from_rdata(name.clone(), ttl, rdata));
                    }
                }
            }
            DnsRecordData::TXT { strings } => {
                // TXT records can have multiple strings in a single record
                let txt_data: Vec<String> = strings.clone();
                let rdata = RData::TXT(TXT::new(txt_data));
                records.push(Record::from_rdata(name, ttl, rdata));
            }
            DnsRecordData::SRV { records: srv_records } => {
                for srv in srv_records {
                    if let Ok(target) = Name::from_utf8(format!("{}.", srv.target)) {
                        let rdata = RData::SRV(SRV::new(srv.priority, srv.weight, srv.port, target));
                        records.push(Record::from_rdata(name.clone(), ttl, rdata));
                    }
                }
            }
            DnsRecordData::NS { nameservers } => {
                for ns in nameservers {
                    if let Ok(ns_name) = Name::from_utf8(format!("{}.", ns)) {
                        let rdata = RData::NS(hickory_proto::rr::rdata::NS(ns_name));
                        records.push(Record::from_rdata(name.clone(), ttl, rdata));
                    }
                }
            }
            DnsRecordData::SOA(soa) => {
                let mname = match Name::from_utf8(format!("{}.", soa.mname)) {
                    Ok(n) => n,
                    Err(_) => return records,
                };
                let rname = match Name::from_utf8(format!("{}.", soa.rname)) {
                    Ok(n) => n,
                    Err(_) => return records,
                };

                let rdata = RData::SOA(SOA::new(
                    mname,
                    rname,
                    soa.serial,
                    i32::try_from(soa.refresh).unwrap_or(i32::MAX),
                    i32::try_from(soa.retry).unwrap_or(i32::MAX),
                    i32::try_from(soa.expire).unwrap_or(i32::MAX),
                    soa.minimum,
                ));
                records.push(Record::from_rdata(name, ttl, rdata));
            }
            DnsRecordData::PTR { target } => {
                if let Ok(target_name) = Name::from_utf8(format!("{}.", target)) {
                    let rdata = RData::PTR(hickory_proto::rr::rdata::PTR(target_name));
                    records.push(Record::from_rdata(name, ttl, rdata));
                }
            }
            DnsRecordData::CAA { records: caa_records } => {
                for caa in caa_records {
                    // CAA value format: [flags] [tag] [value]
                    // Use new_issue for "issue" and "issuewild" tags
                    let issuer_critical = caa.flags > 0;
                    let caa_data = match caa.tag.as_str() {
                        "issue" | "issuewild" => {
                            // For issue/issuewild, value is a domain or ";"
                            if caa.value == ";" || caa.value.is_empty() {
                                CAA::new_issue(issuer_critical, None, vec![])
                            } else if let Ok(domain) = hickory_proto::rr::Name::from_utf8(caa.value.clone()) {
                                CAA::new_issue(issuer_critical, Some(domain), vec![])
                            } else {
                                continue;
                            }
                        }
                        "iodef" => {
                            // For iodef, value is a URL
                            if let Ok(url) = url::Url::parse(&caa.value) {
                                CAA::new_iodef(issuer_critical, url)
                            } else {
                                continue;
                            }
                        }
                        _ => {
                            // Unknown CAA tag, skip
                            continue;
                        }
                    };
                    let rdata = RData::CAA(caa_data);
                    records.push(Record::from_rdata(name.clone(), ttl, rdata));
                }
            }
        }

        records
    }

    /// Perform a lookup in the Aspen DNS cache.
    async fn do_lookup(
        &self,
        name: &LowerName,
        rtype: HickoryRecordType,
    ) -> Result<Box<dyn LookupObject>, LookupError> {
        let aspen_rtype = match Self::from_hickory_record_type(rtype) {
            Some(rt) => rt,
            None => {
                debug!(record_type = ?rtype, "Unsupported record type");
                return Ok(Box::new(AspenLookup::empty(name.clone().into(), rtype)));
            }
        };

        // Convert hickory name to domain string (remove trailing dot)
        let domain = name.to_string();
        let domain = domain.trim_end_matches('.');

        trace!(domain = %domain, record_type = ?aspen_rtype, "Looking up DNS record");

        // Try to resolve from the client cache
        if let Some(record) = self.client.resolve(domain, aspen_rtype).await {
            let hickory_records = self.convert_record(&record);
            if !hickory_records.is_empty() {
                let mut record_set = RecordSet::new(name.clone().into(), rtype, 0);
                for rec in hickory_records {
                    record_set.insert(rec, 0);
                }
                return Ok(Box::new(AspenLookup::new(record_set)));
            }
        }

        // Not found
        debug!(domain = %domain, record_type = ?aspen_rtype, "Record not found in cache");
        Ok(Box::new(AspenLookup::empty(name.clone().into(), rtype)))
    }
}

#[async_trait]
impl Authority for AspenDnsAuthority {
    type Lookup = AspenLookup;

    fn zone_type(&self) -> ZoneType {
        self.zone_type
    }

    fn is_axfr_allowed(&self) -> bool {
        false // Zone transfers not supported
    }

    async fn update(&self, _update: &MessageRequest) -> UpdateResult<bool> {
        // Dynamic updates not supported - records are managed via Aspen API
        Err(ResponseCode::NotImp)
    }

    fn origin(&self) -> &LowerName {
        &self.origin
    }

    async fn lookup(
        &self,
        name: &LowerName,
        rtype: HickoryRecordType,
        _lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        match self.do_lookup(name, rtype).await {
            Ok(lookup) => {
                // Convert Box<dyn LookupObject> back to AspenLookup
                // Since we know we created it, this should be safe
                if lookup.is_empty() {
                    LookupControlFlow::Continue(Ok(AspenLookup::empty(name.clone().into(), rtype)))
                } else {
                    // Collect records and rebuild
                    let mut record_set = RecordSet::new(name.clone().into(), rtype, 0);
                    for rec in lookup.iter() {
                        record_set.insert(rec.clone(), 0);
                    }
                    LookupControlFlow::Continue(Ok(AspenLookup::new(record_set)))
                }
            }
            Err(e) => LookupControlFlow::Continue(Err(e)),
        }
    }

    async fn search(
        &self,
        request_info: RequestInfo<'_>,
        lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        let name = request_info.query.name();
        let rtype = request_info.query.query_type();

        debug!(
            name = %name,
            record_type = ?rtype,
            "DNS search request"
        );

        self.lookup(name, rtype, lookup_options).await
    }

    async fn get_nsec_records(
        &self,
        _name: &LowerName,
        _lookup_options: LookupOptions,
    ) -> LookupControlFlow<Self::Lookup> {
        // DNSSEC not supported
        LookupControlFlow::Continue(Err(LookupError::from(ResponseCode::NotImp)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_type_conversion() {
        // Test round-trip conversion
        let types = [
            AspenRecordType::A,
            AspenRecordType::AAAA,
            AspenRecordType::CNAME,
            AspenRecordType::MX,
            AspenRecordType::TXT,
            AspenRecordType::SRV,
            AspenRecordType::NS,
            AspenRecordType::SOA,
            AspenRecordType::PTR,
            AspenRecordType::CAA,
        ];

        for aspen_type in types {
            let hickory_type = AspenDnsAuthority::to_hickory_record_type(aspen_type);
            let back = AspenDnsAuthority::from_hickory_record_type(hickory_type);
            assert_eq!(Some(aspen_type), back, "Round-trip failed for {:?}", aspen_type);
        }
    }

    #[test]
    fn test_unsupported_record_type() {
        // DNSKEY is not supported by Aspen
        let result = AspenDnsAuthority::from_hickory_record_type(HickoryRecordType::DNSKEY);
        assert!(result.is_none());
    }
}
