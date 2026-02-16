//! DNS management commands.
//!
//! Commands for managing DNS records and zones in the distributed cluster.
//! Records are stored with Raft consensus and automatically synced to clients
//! via iroh-docs.
//!
//! # Record Types
//!
//! Supported DNS record types:
//! - `A` - IPv4 addresses
//! - `AAAA` - IPv6 addresses
//! - `CNAME` - Canonical name (alias)
//! - `MX` - Mail exchange
//! - `TXT` - Text records
//! - `SRV` - Service location
//! - `NS` - Nameserver
//! - `SOA` - Start of authority
//! - `PTR` - Pointer (reverse DNS)
//! - `CAA` - Certificate authority authorization
//!
//! # Examples
//!
//! ```bash
//! # Create an A record
//! aspen-cli dns set api.example.com A 192.168.1.1 --ttl 300
//!
//! # Create an MX record
//! aspen-cli dns set example.com MX --data '{"priority":10,"exchange":"mail.example.com"}'
//!
//! # Get a record
//! aspen-cli dns get api.example.com A
//!
//! # Resolve with wildcard fallback
//! aspen-cli dns resolve foo.example.com A
//!
//! # List zones
//! aspen-cli dns zones list
//! ```

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::DnsRecordOutput;
use crate::output::DnsRecordsOutput;
use crate::output::DnsZoneOutput;
use crate::output::DnsZonesOutput;
use crate::output::print_output;
use crate::output::print_success;

/// DNS management operations.
#[derive(Subcommand)]
pub enum DnsCommand {
    /// Create or update a DNS record.
    Set(SetRecordArgs),

    /// Get a DNS record.
    Get(GetRecordArgs),

    /// Get all records for a domain.
    GetAll(GetAllRecordsArgs),

    /// Delete a DNS record.
    Delete(DeleteRecordArgs),

    /// Resolve a domain (with wildcard matching).
    Resolve(ResolveArgs),

    /// Scan DNS records by prefix.
    Scan(ScanRecordsArgs),

    /// Zone management commands.
    #[command(subcommand)]
    Zone(ZoneCommand),
}

/// Zone management subcommands.
#[derive(Subcommand)]
pub enum ZoneCommand {
    /// Create or update a zone.
    Set(SetZoneArgs),

    /// Get zone information.
    Get(GetZoneArgs),

    /// List all zones.
    List,

    /// Delete a zone.
    Delete(DeleteZoneArgs),
}

#[derive(Args)]
pub struct SetRecordArgs {
    /// Domain name (e.g., "api.example.com").
    pub domain: String,

    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,

    /// Record data. Format depends on record type:
    /// - A: IP address (e.g., "192.168.1.1")
    /// - AAAA: IPv6 address (e.g., "2001:db8::1")
    /// - CNAME: Target domain (e.g., "www.example.com")
    /// - TXT: Text string (e.g., "v=spf1 include:_spf.example.com ~all")
    /// - MX: Use --data for JSON (e.g., '{"priority":10,"exchange":"mail.example.com"}')
    /// - SRV: Use --data for JSON (e.g.,
    ///   '{"priority":0,"weight":5,"port":443,"target":"api.example.com"}')
    #[arg(required_unless_present = "data")]
    pub value: Option<String>,

    /// JSON-encoded record data (for complex types like MX, SRV, SOA).
    #[arg(long, short)]
    pub data: Option<String>,

    /// TTL in seconds.
    #[arg(long = "ttl", default_value = "300")]
    pub ttl_secs: u32,
}

#[derive(Args)]
pub struct GetRecordArgs {
    /// Domain name.
    pub domain: String,

    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,
}

#[derive(Args)]
pub struct GetAllRecordsArgs {
    /// Domain name.
    pub domain: String,
}

#[derive(Args)]
pub struct DeleteRecordArgs {
    /// Domain name.
    pub domain: String,

    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,
}

#[derive(Args)]
pub struct ResolveArgs {
    /// Domain name to resolve.
    pub domain: String,

    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,
}

#[derive(Args)]
pub struct ScanRecordsArgs {
    /// Domain prefix to match (empty matches all).
    #[arg(default_value = "")]
    pub prefix: String,

    /// Maximum number of records to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct SetZoneArgs {
    /// Zone name (e.g., "example.com").
    pub name: String,

    /// Default TTL for records in this zone.
    #[arg(long, default_value = "3600")]
    pub default_ttl: u32,

    /// Optional description.
    #[arg(long, short)]
    pub description: Option<String>,

    /// Disable the zone.
    #[arg(long)]
    pub disabled: bool,
}

#[derive(Args)]
pub struct GetZoneArgs {
    /// Zone name.
    pub name: String,
}

#[derive(Args)]
pub struct DeleteZoneArgs {
    /// Zone name.
    pub name: String,

    /// Also delete all records in the zone.
    #[arg(long)]
    pub delete_records: bool,
}

impl DnsCommand {
    /// Execute the DNS command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            DnsCommand::Set(args) => dns_set_record(client, args, json).await,
            DnsCommand::Get(args) => dns_get_record(client, args, json).await,
            DnsCommand::GetAll(args) => dns_get_all_records(client, args, json).await,
            DnsCommand::Delete(args) => dns_delete_record(client, args, json).await,
            DnsCommand::Resolve(args) => dns_resolve(client, args, json).await,
            DnsCommand::Scan(args) => dns_scan_records(client, args, json).await,
            DnsCommand::Zone(cmd) => cmd.run(client, json).await,
        }
    }
}

impl ZoneCommand {
    /// Execute the zone command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            ZoneCommand::Set(args) => dns_set_zone(client, args, json).await,
            ZoneCommand::Get(args) => dns_get_zone(client, args, json).await,
            ZoneCommand::List => dns_list_zones(client, json).await,
            ZoneCommand::Delete(args) => dns_delete_zone(client, args, json).await,
        }
    }
}

/// Build record data JSON from CLI args.
fn build_record_data_json(record_type: &str, value: Option<&str>, data: Option<&str>) -> Result<String> {
    // If explicit JSON data is provided, use it
    if let Some(json_data) = data {
        return Ok(json_data.to_string());
    }

    // Otherwise, build JSON from the simple value
    let value = value.ok_or_else(|| anyhow::anyhow!("value or --data is required"))?;

    // Build type-specific JSON
    let json = match record_type.to_uppercase().as_str() {
        "A" => {
            // Parse as IPv4 address
            let addr: std::net::Ipv4Addr = value.parse().map_err(|e| anyhow::anyhow!("invalid IPv4 address: {}", e))?;
            serde_json::json!({
                "type": "A",
                "addresses": [addr.to_string()]
            })
        }
        "AAAA" => {
            // Parse as IPv6 address
            let addr: std::net::Ipv6Addr = value.parse().map_err(|e| anyhow::anyhow!("invalid IPv6 address: {}", e))?;
            serde_json::json!({
                "type": "AAAA",
                "addresses": [addr.to_string()]
            })
        }
        "CNAME" => {
            serde_json::json!({
                "type": "CNAME",
                "target": value
            })
        }
        "TXT" => {
            serde_json::json!({
                "type": "TXT",
                "strings": [value]
            })
        }
        "PTR" => {
            serde_json::json!({
                "type": "PTR",
                "target": value
            })
        }
        "NS" => {
            serde_json::json!({
                "type": "NS",
                "nameservers": [value]
            })
        }
        "MX" | "SRV" | "SOA" | "CAA" => {
            // These require explicit JSON data
            anyhow::bail!(
                "{} records require --data with JSON format. Example for MX: \
                 --data '{{\"priority\":10,\"exchange\":\"mail.example.com\"}}'",
                record_type
            )
        }
        _ => {
            anyhow::bail!("unsupported record type: {}", record_type)
        }
    };

    Ok(json.to_string())
}

async fn dns_set_record(client: &AspenClient, args: SetRecordArgs, json: bool) -> Result<()> {
    let data_json = build_record_data_json(&args.record_type, args.value.as_deref(), args.data.as_deref())?;

    let response = client
        .send(ClientRpcRequest::DnsSetRecord {
            domain: args.domain.clone(),
            record_type: args.record_type.to_uppercase(),
            ttl_seconds: args.ttl_secs,
            data_json,
        })
        .await?;

    match response {
        ClientRpcResponse::DnsSetRecordResult(result) => {
            if result.success {
                if let Some(record) = result.record {
                    let output = DnsRecordOutput::from_response(record);
                    print_output(&output, json);
                } else {
                    print_success("Record created", json);
                }
            } else {
                anyhow::bail!("failed to set record: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_get_record(client: &AspenClient, args: GetRecordArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsGetRecord {
            domain: args.domain.clone(),
            record_type: args.record_type.to_uppercase(),
        })
        .await?;

    match response {
        ClientRpcResponse::DnsGetRecordResult(result) => {
            if result.success {
                if result.found {
                    if let Some(record) = result.record {
                        let output = DnsRecordOutput::from_response(record);
                        print_output(&output, json);
                    }
                } else if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "found": false,
                            "domain": args.domain,
                            "record_type": args.record_type
                        })
                    );
                } else {
                    println!("Record not found: {} {}", args.domain, args.record_type);
                }
            } else {
                anyhow::bail!("failed to get record: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_get_all_records(client: &AspenClient, args: GetAllRecordsArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsGetRecords {
            domain: args.domain.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::DnsGetRecordsResult(result) => {
            if result.success {
                let output = DnsRecordsOutput::from_responses(result.records);
                print_output(&output, json);
            } else {
                anyhow::bail!("failed to get records: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_delete_record(client: &AspenClient, args: DeleteRecordArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsDeleteRecord {
            domain: args.domain.clone(),
            record_type: args.record_type.to_uppercase(),
        })
        .await?;

    match response {
        ClientRpcResponse::DnsDeleteRecordResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "success",
                            "deleted": result.deleted,
                            "domain": args.domain,
                            "record_type": args.record_type
                        })
                    );
                } else if result.deleted {
                    println!("Deleted {} {} record", args.domain, args.record_type);
                } else {
                    println!("Record not found: {} {}", args.domain, args.record_type);
                }
            } else {
                anyhow::bail!(
                    "failed to delete record: {}",
                    result.error.unwrap_or_else(|| "unknown error".to_string())
                )
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_resolve(client: &AspenClient, args: ResolveArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsResolve {
            domain: args.domain.clone(),
            record_type: args.record_type.to_uppercase(),
        })
        .await?;

    match response {
        ClientRpcResponse::DnsResolveResult(result) => {
            if result.success {
                let output = DnsRecordsOutput::from_responses(result.records);
                print_output(&output, json);
            } else {
                anyhow::bail!("failed to resolve: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_scan_records(client: &AspenClient, args: ScanRecordsArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsScanRecords {
            prefix: args.prefix,
            limit: args.limit,
        })
        .await?;

    match response {
        ClientRpcResponse::DnsScanRecordsResult(result) => {
            if result.success {
                let output = DnsRecordsOutput::from_responses(result.records);
                print_output(&output, json);
            } else {
                anyhow::bail!("failed to scan records: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_set_zone(client: &AspenClient, args: SetZoneArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsSetZone {
            name: args.name.clone(),
            enabled: !args.disabled,
            default_ttl_secs: args.default_ttl,
            description: args.description,
        })
        .await?;

    match response {
        ClientRpcResponse::DnsSetZoneResult(result) => {
            if result.success {
                if let Some(zone) = result.zone {
                    let output = DnsZoneOutput::from_response(zone);
                    print_output(&output, json);
                } else {
                    print_success(&format!("Zone '{}' created", args.name), json);
                }
            } else {
                anyhow::bail!("failed to set zone: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_get_zone(client: &AspenClient, args: GetZoneArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsGetZone {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::DnsGetZoneResult(result) => {
            if result.success {
                if result.found {
                    if let Some(zone) = result.zone {
                        let output = DnsZoneOutput::from_response(zone);
                        print_output(&output, json);
                    }
                } else if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "found": false,
                            "name": args.name
                        })
                    );
                } else {
                    println!("Zone not found: {}", args.name);
                }
            } else {
                anyhow::bail!("failed to get zone: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_list_zones(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::DnsListZones).await?;

    match response {
        ClientRpcResponse::DnsListZonesResult(result) => {
            if result.success {
                let output = DnsZonesOutput::from_responses(result.zones);
                print_output(&output, json);
            } else {
                anyhow::bail!("failed to list zones: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn dns_delete_zone(client: &AspenClient, args: DeleteZoneArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DnsDeleteZone {
            name: args.name.clone(),
            delete_records: args.delete_records,
        })
        .await?;

    match response {
        ClientRpcResponse::DnsDeleteZoneResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "success",
                            "deleted": result.deleted,
                            "records_deleted": result.records_deleted,
                            "name": args.name
                        })
                    );
                } else if result.deleted {
                    if result.records_deleted > 0 {
                        println!("Deleted zone '{}' and {} record(s)", args.name, result.records_deleted);
                    } else {
                        println!("Deleted zone '{}'", args.name);
                    }
                } else {
                    println!("Zone not found: {}", args.name);
                }
            } else {
                anyhow::bail!("failed to delete zone: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}
