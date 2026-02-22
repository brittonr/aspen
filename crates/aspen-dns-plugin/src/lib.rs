//! WASM guest plugin for DNS record and zone management.
//!
//! Handles 10 DNS operations: records (set, get, get-all, delete, resolve, scan)
//! and zones (set, get, list, delete). All state is stored under the `dns:` KV
//! prefix, matching the native `aspen-dns` wire format exactly.
//!
//! **KV key format:**
//! - Records: `dns:{domain}:{record_type}` (e.g., `dns:example.com:A`)
//! - Zones:   `dns:_zone:{zone_name}` (e.g., `dns:_zone:example.com`)

mod kv;
mod records;
mod zones;

use aspen_client_api::DnsDeleteRecordResultResponse;
use aspen_client_api::DnsDeleteZoneResultResponse;
use aspen_client_api::DnsRecordResponse;
use aspen_client_api::DnsRecordResultResponse;
use aspen_client_api::DnsRecordsResultResponse;
use aspen_client_api::DnsZoneResultResponse;
use aspen_client_api::DnsZonesResultResponse;
use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

use crate::records::DnsRecord;
use crate::records::DnsRecordData;
use crate::records::RecordType;
use crate::zones::Zone;

struct DnsPlugin;

impl AspenPlugin for DnsPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "dns".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "DnsSetRecord".to_string(),
                "DnsGetRecord".to_string(),
                "DnsGetRecords".to_string(),
                "DnsDeleteRecord".to_string(),
                "DnsResolve".to_string(),
                "DnsScanRecords".to_string(),
                "DnsSetZone".to_string(),
                "DnsGetZone".to_string(),
                "DnsListZones".to_string(),
                "DnsDeleteZone".to_string(),
            ],
            priority: 945,
            app_id: Some("dns".to_string()),
            kv_prefixes: vec!["dns:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            // === Record Operations ===
            ClientRpcRequest::DnsSetRecord {
                domain,
                record_type,
                ttl_seconds,
                data_json,
            } => handle_set_record(domain, record_type, ttl_seconds, data_json),

            ClientRpcRequest::DnsGetRecord { domain, record_type } => handle_get_record(domain, record_type),

            ClientRpcRequest::DnsGetRecords { domain } => handle_get_records(domain),

            ClientRpcRequest::DnsDeleteRecord { domain, record_type } => handle_delete_record(domain, record_type),

            ClientRpcRequest::DnsResolve { domain, record_type } => handle_resolve(domain, record_type),

            ClientRpcRequest::DnsScanRecords { prefix, limit } => handle_scan_records(prefix, limit),

            // === Zone Operations ===
            ClientRpcRequest::DnsSetZone {
                name,
                is_enabled,
                default_ttl_secs,
                description,
            } => handle_set_zone(name, is_enabled, default_ttl_secs, description),

            ClientRpcRequest::DnsGetZone { name } => handle_get_zone(name),

            ClientRpcRequest::DnsListZones => handle_list_zones(),

            ClientRpcRequest::DnsDeleteZone {
                name,
                should_delete_records,
            } => handle_delete_zone(name, should_delete_records),

            _ => ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "UNHANDLED_REQUEST".to_string(),
                message: "Request not handled by DNS plugin".to_string(),
            }),
        }
    }
}

register_plugin!(DnsPlugin);

// ============================================================================
// Record Handlers
// ============================================================================

fn handle_set_record(domain: String, record_type: String, ttl_seconds: u32, data_json: String) -> ClientRpcResponse {
    // Parse record type
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record type: {record_type}")),
            });
        }
    };

    // Parse record data from JSON
    let data: DnsRecordData = match serde_json::from_str(&data_json) {
        Ok(d) => d,
        Err(e) => {
            return ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record data JSON: {e}")),
            });
        }
    };

    // Verify record type matches data
    if data.record_type() != rtype {
        return ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(format!("Record type mismatch: specified {} but data is {}", rtype, data.record_type())),
        });
    }

    let record = DnsRecord::new(domain.clone(), ttl_seconds, data);
    let key = kv::record_key(&domain, rtype.as_str());

    match kv::kv_put_json(&key, &record) {
        Ok(()) => ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: true,
            record: Some(record.to_response()),
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(e),
        }),
    }
}

fn handle_get_record(domain: String, record_type: String) -> ClientRpcResponse {
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record type: {record_type}")),
            });
        }
    };

    let key = kv::record_key(&domain, rtype.as_str());

    match kv::kv_get_json::<DnsRecord>(&key) {
        Ok(Some(record)) => ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: true,
            record: Some(record.to_response()),
            error: None,
        }),
        Ok(None) => ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: false,
            record: None,
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(e),
        }),
    }
}

fn handle_get_records(domain: String) -> ClientRpcResponse {
    let prefix = format!("{}{}:", kv::DNS_KEY_PREFIX, domain);

    match kv::kv_scan_raw(&prefix, 100) {
        Ok(entries) => {
            let records: Vec<DnsRecordResponse> =
                entries.iter().filter_map(|(_, v)| DnsRecord::from_json_bytes(v)).map(|r| r.to_response()).collect();
            let count = records.len() as u32;

            ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
                is_success: true,
                records,
                count,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
            is_success: false,
            records: vec![],
            count: 0,
            error: Some(e),
        }),
    }
}

fn handle_delete_record(domain: String, record_type: String) -> ClientRpcResponse {
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                is_success: false,
                was_deleted: false,
                error: Some(format!("Invalid record type: {record_type}")),
            });
        }
    };

    let key = kv::record_key(&domain, rtype.as_str());

    // Check if record exists first
    let exists = match kv::kv_get_json::<DnsRecord>(&key) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            return ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                is_success: false,
                was_deleted: false,
                error: Some(e),
            });
        }
    };

    if !exists {
        return ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
            is_success: true,
            was_deleted: false,
            error: None,
        });
    }

    match kv::kv_delete(&key) {
        Ok(()) => ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
            is_success: true,
            was_deleted: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
            is_success: false,
            was_deleted: false,
            error: Some(e),
        }),
    }
}

fn handle_resolve(domain: String, record_type: String) -> ClientRpcResponse {
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                is_success: false,
                records: vec![],
                count: 0,
                error: Some(format!("Invalid record type: {record_type}")),
            });
        }
    };

    // 1. Try exact match
    let exact_key = kv::record_key(&domain, rtype.as_str());
    match kv::kv_get_json::<DnsRecord>(&exact_key) {
        Ok(Some(mut record)) => {
            record.data.sort_by_priority();
            let records = vec![record.to_response()];
            return ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                is_success: true,
                records,
                count: 1,
                error: None,
            });
        }
        Ok(None) => {} // Fall through to wildcard
        Err(e) => {
            return ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                is_success: false,
                records: vec![],
                count: 0,
                error: Some(e),
            });
        }
    }

    // 2. Try wildcard match (unless already a wildcard domain)
    if !kv::is_wildcard_domain(&domain) {
        if let Some(wildcard_domain) = kv::wildcard_parent(&domain) {
            let wildcard_key = kv::record_key(&wildcard_domain, rtype.as_str());
            match kv::kv_get_json::<DnsRecord>(&wildcard_key) {
                Ok(Some(mut record)) => {
                    record.data.sort_by_priority();
                    let records = vec![record.to_response()];
                    return ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                        is_success: true,
                        records,
                        count: 1,
                        error: None,
                    });
                }
                Ok(None) => {}
                Err(e) => {
                    return ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                        is_success: false,
                        records: vec![],
                        count: 0,
                        error: Some(e),
                    });
                }
            }
        }
    }

    // 3. No match
    ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
        is_success: true,
        records: vec![],
        count: 0,
        error: None,
    })
}

fn handle_scan_records(prefix: String, limit: u32) -> ClientRpcResponse {
    let scan_prefix = format!("{}{}", kv::DNS_KEY_PREFIX, prefix);
    let capped_limit = limit.min(kv::MAX_BATCH_SIZE);

    match kv::kv_scan_raw(&scan_prefix, capped_limit) {
        Ok(entries) => {
            let records: Vec<DnsRecordResponse> =
                entries.iter().filter_map(|(_, v)| DnsRecord::from_json_bytes(v)).map(|r| r.to_response()).collect();
            let count = records.len() as u32;

            ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
                is_success: true,
                records,
                count,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
            is_success: false,
            records: vec![],
            count: 0,
            error: Some(e),
        }),
    }
}

// ============================================================================
// Zone Handlers
// ============================================================================

fn handle_set_zone(
    name: String,
    is_enabled: bool,
    default_ttl_secs: u32,
    description: Option<String>,
) -> ClientRpcResponse {
    // Check zone count limit (read existing zones first)
    match kv::kv_scan_raw(kv::DNS_ZONE_PREFIX, kv::MAX_ZONES) {
        Ok(entries) => {
            let zone_exists =
                entries.iter().any(|(k, _)| k.strip_prefix(kv::DNS_ZONE_PREFIX).map_or(false, |n| n == name));

            if !zone_exists && entries.len() >= kv::MAX_ZONES as usize {
                return ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
                    is_success: false,
                    was_found: false,
                    zone: None,
                    error: Some(format!("Maximum zone count ({}) exceeded", kv::MAX_ZONES)),
                });
            }
        }
        Err(e) => {
            return ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
                is_success: false,
                was_found: false,
                zone: None,
                error: Some(e),
            });
        }
    }

    let zone = Zone::new(name.clone(), is_enabled, default_ttl_secs, description);
    let key = kv::zone_key(&name);

    match kv::kv_put_json(&key, &zone) {
        Ok(()) => ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: true,
            zone: Some(zone.to_response()),
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
            is_success: false,
            was_found: false,
            zone: None,
            error: Some(e),
        }),
    }
}

fn handle_get_zone(name: String) -> ClientRpcResponse {
    let key = kv::zone_key(&name);

    match kv::kv_get_json::<Zone>(&key) {
        Ok(Some(zone)) => ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: true,
            zone: Some(zone.to_response()),
            error: None,
        }),
        Ok(None) => ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: false,
            zone: None,
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: false,
            was_found: false,
            zone: None,
            error: Some(e),
        }),
    }
}

fn handle_list_zones() -> ClientRpcResponse {
    match kv::kv_scan_raw(kv::DNS_ZONE_PREFIX, kv::MAX_ZONES) {
        Ok(entries) => {
            let zones: Vec<_> =
                entries.iter().filter_map(|(_, v)| Zone::from_json_bytes(v)).map(|z| z.to_response()).collect();
            let count = zones.len() as u32;

            ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
                is_success: true,
                zones,
                count,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
            is_success: false,
            zones: vec![],
            count: 0,
            error: Some(e),
        }),
    }
}

fn handle_delete_zone(name: String, delete_records: bool) -> ClientRpcResponse {
    let zone_key = kv::zone_key(&name);

    // Check if zone exists
    let zone_exists = match kv::kv_get_json::<Zone>(&zone_key) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            return ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
                is_success: false,
                was_deleted: false,
                records_deleted: 0,
                error: Some(e),
            });
        }
    };

    if !zone_exists {
        return ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
            is_success: true,
            was_deleted: false,
            records_deleted: 0,
            error: None,
        });
    }

    // Optionally delete all records in this zone
    let mut records_deleted: u32 = 0;
    if delete_records {
        // Scan all DNS records and delete those belonging to this zone
        let mut continuation = true;
        while continuation {
            match kv::kv_scan_raw(kv::DNS_KEY_PREFIX, kv::MAX_BATCH_SIZE) {
                Ok(entries) => {
                    if entries.is_empty() {
                        break;
                    }
                    for (key, _) in &entries {
                        // Parse domain from key: dns:{domain}:{type}
                        if let Some(rest) = key.strip_prefix(kv::DNS_KEY_PREFIX) {
                            if let Some((domain, _)) = rest.rsplit_once(':') {
                                if domain.ends_with(&name) || domain == name {
                                    if kv::kv_delete(key).is_ok() {
                                        records_deleted += 1;
                                    }
                                }
                            }
                        }
                    }
                    // If we got fewer entries than the limit, we're done
                    continuation = entries.len() >= kv::MAX_BATCH_SIZE as usize;
                }
                Err(_) => break,
            }
        }
    }

    // Delete the zone itself
    match kv::kv_delete(&zone_key) {
        Ok(()) => ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
            is_success: true,
            was_deleted: true,
            records_deleted,
            error: None,
        }),
        Err(e) => ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
            is_success: false,
            was_deleted: false,
            records_deleted,
            error: Some(e),
        }),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let info = DnsPlugin::info();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("valid plugin.json");

        assert_eq!(info.name, manifest["name"].as_str().unwrap());
        assert_eq!(info.version, manifest["version"].as_str().unwrap());
        assert_eq!(info.priority, manifest["priority"].as_u64().unwrap() as u32);
        assert_eq!(info.app_id.as_deref(), manifest["app_id"].as_str());
        assert_eq!(info.handles.len(), manifest["handles"].as_array().unwrap().len(), "handle count mismatch");
    }
}
