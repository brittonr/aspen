//! DNS request handler.
//!
//! Handles all DNS* operations for DNS record management.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DnsDeleteRecordResultResponse;
use aspen_client_api::DnsDeleteZoneResultResponse;
use aspen_client_api::DnsRecordResponse;
use aspen_client_api::DnsRecordResultResponse;
use aspen_client_api::DnsRecordsResultResponse;
use aspen_client_api::DnsZoneResponse;
use aspen_client_api::DnsZoneResultResponse;
use aspen_client_api::DnsZonesResultResponse;
use aspen_dns::AspenDnsStore;
use aspen_dns::DnsRecord;
use aspen_dns::DnsRecordData;
use aspen_dns::DnsStore;
use aspen_dns::MAX_BATCH_SIZE;
use aspen_dns::RecordType;
use aspen_dns::Zone;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Handler for DNS operations.
pub struct DnsHandler;

#[async_trait::async_trait]
impl RequestHandler for DnsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::DnsSetRecord { .. }
                | ClientRpcRequest::DnsGetRecord { .. }
                | ClientRpcRequest::DnsGetRecords { .. }
                | ClientRpcRequest::DnsDeleteRecord { .. }
                | ClientRpcRequest::DnsResolve { .. }
                | ClientRpcRequest::DnsScanRecords { .. }
                | ClientRpcRequest::DnsSetZone { .. }
                | ClientRpcRequest::DnsGetZone { .. }
                | ClientRpcRequest::DnsListZones
                | ClientRpcRequest::DnsDeleteZone { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DnsSetRecord {
                domain,
                record_type,
                ttl_seconds,
                data_json,
            } => handle_dns_set_record(ctx, domain, record_type, ttl_seconds, data_json).await,

            ClientRpcRequest::DnsGetRecord { domain, record_type } => {
                handle_dns_get_record(ctx, domain, record_type).await
            }

            ClientRpcRequest::DnsGetRecords { domain } => handle_dns_get_records(ctx, domain).await,

            ClientRpcRequest::DnsDeleteRecord { domain, record_type } => {
                handle_dns_delete_record(ctx, domain, record_type).await
            }

            ClientRpcRequest::DnsResolve { domain, record_type } => handle_dns_resolve(ctx, domain, record_type).await,

            ClientRpcRequest::DnsScanRecords { prefix, limit } => handle_dns_scan_records(ctx, prefix, limit).await,

            ClientRpcRequest::DnsSetZone {
                name,
                is_enabled,
                default_ttl_secs,
                description,
            } => handle_dns_set_zone(ctx, name, is_enabled, default_ttl_secs, description).await,

            ClientRpcRequest::DnsGetZone { name } => handle_dns_get_zone(ctx, name).await,

            ClientRpcRequest::DnsListZones => handle_dns_list_zones(ctx).await,

            ClientRpcRequest::DnsDeleteZone {
                name,
                should_delete_records,
            } => handle_dns_delete_zone(ctx, name, should_delete_records).await,

            _ => Err(anyhow::anyhow!("request not handled by DnsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "DnsHandler"
    }
}

// ============================================================================
// DNS Record Operations
// ============================================================================

async fn handle_dns_set_record(
    ctx: &ClientProtocolContext,
    domain: String,
    record_type: String,
    ttl_seconds: u32,
    data_json: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse record type
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record type: {}", record_type)),
            }));
        }
    };

    // Parse record data from JSON
    let data: DnsRecordData = match serde_json::from_str(&data_json) {
        Ok(d) => d,
        Err(e) => {
            return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record data JSON: {}", e)),
            }));
        }
    };

    // Verify record type matches data
    if data.record_type() != rtype {
        return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(format!("Record type mismatch: specified {} but data is {}", rtype, data.record_type())),
        }));
    }

    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
    let record = DnsRecord::new(domain, ttl_seconds, data);

    match dns_store.set_record(record.clone()).await {
        Ok(()) => Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: true,
            record: Some(dns_record_to_response(&record)),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_get_record(
    ctx: &ClientProtocolContext,
    domain: String,
    record_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse record type
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                is_success: false,
                was_found: false,
                record: None,
                error: Some(format!("Invalid record type: {}", record_type)),
            }));
        }
    };

    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.get_record(&domain, rtype).await {
        Ok(Some(record)) => Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: true,
            record: Some(dns_record_to_response(&record)),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: true,
            was_found: false,
            record: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
            is_success: false,
            was_found: false,
            record: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_get_records(ctx: &ClientProtocolContext, domain: String) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.get_records(&domain).await {
        Ok(records) => {
            let count = records.len() as u32;
            let record_responses: Vec<DnsRecordResponse> = records.iter().map(dns_record_to_response).collect();

            Ok(ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
                is_success: true,
                records: record_responses,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
            is_success: false,
            records: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_delete_record(
    ctx: &ClientProtocolContext,
    domain: String,
    record_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse record type
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                is_success: false,
                was_deleted: false,
                error: Some(format!("Invalid record type: {}", record_type)),
            }));
        }
    };

    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.delete_record(&domain, rtype).await {
        Ok(was_deleted) => Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
            is_success: true,
            was_deleted,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
            is_success: false,
            was_deleted: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_resolve(
    ctx: &ClientProtocolContext,
    domain: String,
    record_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse record type
    let rtype = match RecordType::from_str_ignore_case(&record_type) {
        Some(rt) => rt,
        None => {
            return Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                is_success: false,
                records: vec![],
                count: 0,
                error: Some(format!("Invalid record type: {}", record_type)),
            }));
        }
    };

    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.resolve(&domain, rtype).await {
        Ok(records) => {
            let count = records.len() as u32;
            let record_responses: Vec<DnsRecordResponse> = records.iter().map(dns_record_to_response).collect();

            Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                is_success: true,
                records: record_responses,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
            is_success: false,
            records: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_scan_records(
    ctx: &ClientProtocolContext,
    prefix: String,
    limit: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    // Cap limit to MAX_BATCH_SIZE
    let capped_limit = limit.min(MAX_BATCH_SIZE);

    match dns_store.scan_records(&prefix, capped_limit).await {
        Ok(records) => {
            let count = records.len() as u32;
            let record_responses: Vec<DnsRecordResponse> = records.iter().map(dns_record_to_response).collect();

            Ok(ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
                is_success: true,
                records: record_responses,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
            is_success: false,
            records: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// DNS Zone Operations
// ============================================================================

async fn handle_dns_set_zone(
    ctx: &ClientProtocolContext,
    name: String,
    enabled: bool,
    default_ttl_secs: u32,
    description: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    // Build zone with optional settings
    let mut zone = Zone::new(&name).with_default_ttl_secs(default_ttl_secs);
    if !enabled {
        zone = zone.disabled();
    }
    if let Some(desc) = description {
        zone = zone.with_description(desc);
    }

    match dns_store.set_zone(zone.clone()).await {
        Ok(()) => Ok(ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: true,
            zone: Some(zone_to_response(&zone)),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
            is_success: false,
            was_found: false,
            zone: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_get_zone(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.get_zone(&name).await {
        Ok(Some(zone)) => Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: true,
            zone: Some(zone_to_response(&zone)),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: true,
            was_found: false,
            zone: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
            is_success: false,
            was_found: false,
            zone: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_list_zones(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.list_zones().await {
        Ok(zones) => {
            let count = zones.len() as u32;
            let zone_responses: Vec<DnsZoneResponse> = zones.iter().map(zone_to_response).collect();

            Ok(ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
                is_success: true,
                zones: zone_responses,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
            is_success: false,
            zones: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_dns_delete_zone(
    ctx: &ClientProtocolContext,
    name: String,
    delete_records: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

    match dns_store.delete_zone(&name, delete_records).await {
        Ok(was_deleted) => Ok(ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
            is_success: true,
            was_deleted,
            records_deleted: 0, // DnsStore doesn't return count; would need enhancement
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
            is_success: false,
            was_deleted: false,
            records_deleted: 0,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn dns_record_to_response(record: &DnsRecord) -> DnsRecordResponse {
    // Serialize record data to JSON. This should never fail for valid DnsRecordData,
    // but we handle the error gracefully by returning an error JSON object.
    let data_json = serde_json::to_string(&record.data).unwrap_or_else(|e| {
        tracing::error!(
            domain = %record.domain,
            record_type = %record.record_type(),
            error = %e,
            "Failed to serialize DNS record data"
        );
        format!(r#"{{"error":"serialization failed: {}"}}"#, e)
    });

    DnsRecordResponse {
        domain: record.domain.clone(),
        record_type: record.record_type().to_string(),
        ttl_seconds: record.ttl_seconds,
        data_json,
        updated_at_ms: record.updated_at_ms,
    }
}

fn zone_to_response(zone: &Zone) -> DnsZoneResponse {
    DnsZoneResponse {
        name: zone.name.clone(),
        is_enabled: zone.is_enabled,
        default_ttl_secs: zone.default_ttl_secs,
        serial: zone.metadata.serial,
        last_modified_ms: zone.metadata.last_modified_ms,
        description: zone.metadata.description.clone(),
    }
}
