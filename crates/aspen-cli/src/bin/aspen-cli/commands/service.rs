//! Service registry commands.
//!
//! Commands for service discovery and registration with health checks,
//! load balancing weights, and version filtering.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Service registry operations.
#[derive(Subcommand)]
pub enum ServiceCommand {
    /// Register a service instance.
    Register(RegisterArgs),

    /// Deregister a service instance.
    Deregister(DeregisterArgs),

    /// Discover service instances.
    Discover(DiscoverArgs),

    /// List services by prefix.
    List(ListArgs),

    /// Get a specific service instance.
    Get(GetArgs),

    /// Send heartbeat to renew TTL.
    Heartbeat(HeartbeatArgs),

    /// Update instance health status.
    Health(HealthArgs),

    /// Update instance metadata.
    Update(UpdateArgs),
}

#[derive(Args)]
pub struct RegisterArgs {
    /// Service name.
    pub service_name: String,

    /// Unique instance identifier.
    pub instance_id: String,

    /// Network address (host:port).
    pub address: String,

    /// Version string.
    #[arg(long = "svc-version", default_value = "1.0.0")]
    pub svc_version: String,

    /// Tags for filtering (comma-separated).
    #[arg(long, default_value = "")]
    pub tags: String,

    /// Load balancing weight.
    #[arg(long, default_value = "100")]
    pub weight: u32,

    /// TTL in milliseconds (0 = default).
    #[arg(long = "ttl", default_value = "30000")]
    pub ttl_ms: u64,
}

#[derive(Args)]
pub struct DeregisterArgs {
    /// Service name.
    pub service_name: String,

    /// Instance identifier.
    pub instance_id: String,

    /// Fencing token from registration.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,
}

#[derive(Args)]
pub struct DiscoverArgs {
    /// Service name.
    pub service_name: String,

    /// Only return healthy instances.
    #[arg(long)]
    pub healthy_only: bool,

    /// Filter by tags (comma-separated).
    #[arg(long)]
    pub tags: Option<String>,

    /// Filter by version prefix.
    #[arg(long)]
    pub version_prefix: Option<String>,

    /// Maximum instances to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct ListArgs {
    /// Service name prefix (empty = all).
    #[arg(default_value = "")]
    pub prefix: String,

    /// Maximum services to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct GetArgs {
    /// Service name.
    pub service_name: String,

    /// Instance identifier.
    pub instance_id: String,
}

#[derive(Args)]
pub struct HeartbeatArgs {
    /// Service name.
    pub service_name: String,

    /// Instance identifier.
    pub instance_id: String,

    /// Fencing token from registration.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,
}

/// Health status values.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Unknown => "unknown",
        }
    }
}

#[derive(Args)]
pub struct HealthArgs {
    /// Service name.
    pub service_name: String,

    /// Instance identifier.
    pub instance_id: String,

    /// Fencing token from registration.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,

    /// New health status.
    #[arg(long, value_enum)]
    pub status: HealthStatus,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Service name.
    pub service_name: String,

    /// Instance identifier.
    pub instance_id: String,

    /// Fencing token from registration.
    #[arg(long = "fencing-token")]
    pub fencing_token: u64,

    /// New version (optional).
    #[arg(long = "svc-version")]
    pub svc_version: Option<String>,

    /// New tags (comma-separated, optional).
    #[arg(long)]
    pub tags: Option<String>,

    /// New weight (optional).
    #[arg(long)]
    pub weight: Option<u32>,
}

/// Service register output.
pub struct RegisterOutput {
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for RegisterOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "fencing_token": self.fencing_token,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Registered. Fencing token: {}", self.fencing_token.unwrap_or(0))
        } else {
            format!("Register failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Simple success output for deregister, heartbeat, health, update.
pub struct ServiceSuccessOutput {
    pub operation: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for ServiceSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "OK".to_string()
        } else {
            format!("{} failed: {}", self.operation, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Service discover output.
pub struct DiscoverOutput {
    pub success: bool,
    pub instances: Vec<InstanceInfo>,
    pub error: Option<String>,
}

pub struct InstanceInfo {
    pub instance_id: String,
    pub address: String,
    pub version: String,
    pub health: String,
    pub weight: u32,
}

impl Outputable for DiscoverOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "instances": self.instances.iter().map(|i| {
                serde_json::json!({
                    "instance_id": i.instance_id,
                    "address": i.address,
                    "version": i.version,
                    "health": i.health,
                    "weight": i.weight
                })
            }).collect::<Vec<_>>(),
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.success {
            return format!("Discover failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        if self.instances.is_empty() {
            return "No instances found".to_string();
        }

        let mut output = format!("Instances ({})\n", self.instances.len());
        output.push_str("Instance ID          | Address                | Version  | Health    | Weight\n");
        output.push_str("---------------------+------------------------+----------+-----------+--------\n");

        for inst in &self.instances {
            output.push_str(&format!(
                "{:20} | {:22} | {:8} | {:9} | {:>6}\n",
                &inst.instance_id[..20.min(inst.instance_id.len())],
                &inst.address[..22.min(inst.address.len())],
                &inst.version[..8.min(inst.version.len())],
                &inst.health,
                inst.weight
            ));
        }

        output
    }
}

/// Service list output.
pub struct ServiceListOutput {
    pub success: bool,
    pub services: Vec<String>,
    pub error: Option<String>,
}

impl Outputable for ServiceListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "services": self.services,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.success {
            return format!("List failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        if self.services.is_empty() {
            return "No services found".to_string();
        }

        let mut output = format!("Services ({})\n", self.services.len());
        for svc in &self.services {
            output.push_str(&format!("  {}\n", svc));
        }
        output
    }
}

/// Service get instance output.
pub struct GetInstanceOutput {
    pub success: bool,
    pub found: bool,
    pub instance: Option<InstanceInfo>,
    pub error: Option<String>,
}

impl Outputable for GetInstanceOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "found": self.found,
            "instance": self.instance.as_ref().map(|i| {
                serde_json::json!({
                    "instance_id": i.instance_id,
                    "address": i.address,
                    "version": i.version,
                    "health": i.health,
                    "weight": i.weight
                })
            }),
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.success {
            return format!("Get failed: {}", self.error.as_deref().unwrap_or("unknown error"));
        }

        if !self.found {
            return "Instance not found".to_string();
        }

        match &self.instance {
            Some(i) => {
                format!(
                    "Instance: {}\n  Address: {}\n  Version: {}\n  Health:  {}\n  Weight:  {}",
                    i.instance_id, i.address, i.version, i.health, i.weight
                )
            }
            None => "Instance found but no data".to_string(),
        }
    }
}

impl ServiceCommand {
    /// Execute the service command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            ServiceCommand::Register(args) => service_register(client, args, json).await,
            ServiceCommand::Deregister(args) => service_deregister(client, args, json).await,
            ServiceCommand::Discover(args) => service_discover(client, args, json).await,
            ServiceCommand::List(args) => service_list(client, args, json).await,
            ServiceCommand::Get(args) => service_get(client, args, json).await,
            ServiceCommand::Heartbeat(args) => service_heartbeat(client, args, json).await,
            ServiceCommand::Health(args) => service_health(client, args, json).await,
            ServiceCommand::Update(args) => service_update(client, args, json).await,
        }
    }
}

async fn service_register(client: &AspenClient, args: RegisterArgs, json: bool) -> Result<()> {
    // Convert comma-separated tags to JSON array
    let tags_json = if args.tags.is_empty() {
        "[]".to_string()
    } else {
        let tags: Vec<&str> = args.tags.split(',').map(|t| t.trim()).collect();
        serde_json::to_string(&tags)?
    };

    let response = client
        .send(ClientRpcRequest::ServiceRegister {
            service_name: args.service_name,
            instance_id: args.instance_id,
            address: args.address,
            version: args.svc_version,
            tags: tags_json,
            weight: args.weight,
            custom_metadata: "{}".to_string(),
            ttl_ms: args.ttl_ms,
            lease_id: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceRegisterResult(result) => {
            let output = RegisterOutput {
                success: result.success,
                fencing_token: result.fencing_token,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_deregister(client: &AspenClient, args: DeregisterArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ServiceDeregister {
            service_name: args.service_name,
            instance_id: args.instance_id,
            fencing_token: args.fencing_token,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceDeregisterResult(result) => {
            let output = ServiceSuccessOutput {
                operation: "deregister".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_discover(client: &AspenClient, args: DiscoverArgs, json: bool) -> Result<()> {
    // Convert comma-separated tags to JSON array
    let tags_json = match args.tags {
        Some(ref tags) if !tags.is_empty() => {
            let tags: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
            serde_json::to_string(&tags)?
        }
        _ => "[]".to_string(),
    };

    let response = client
        .send(ClientRpcRequest::ServiceDiscover {
            service_name: args.service_name,
            healthy_only: args.healthy_only,
            tags: tags_json,
            version_prefix: args.version_prefix,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceDiscoverResult(result) => {
            let instances = result
                .instances
                .into_iter()
                .map(|i| InstanceInfo {
                    instance_id: i.instance_id,
                    address: i.address,
                    version: i.version,
                    health: i.health_status,
                    weight: i.weight,
                })
                .collect();

            let output = DiscoverOutput {
                success: result.success,
                instances,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ServiceList {
            prefix: args.prefix,
            limit: args.limit,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceListResult(result) => {
            let output = ServiceListOutput {
                success: result.success,
                services: result.services,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ServiceGetInstance {
            service_name: args.service_name,
            instance_id: args.instance_id,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceGetInstanceResult(result) => {
            let instance = result.instance.map(|i| InstanceInfo {
                instance_id: i.instance_id,
                address: i.address,
                version: i.version,
                health: i.health_status,
                weight: i.weight,
            });

            let output = GetInstanceOutput {
                success: result.success,
                found: result.found,
                instance,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success || !result.found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_heartbeat(client: &AspenClient, args: HeartbeatArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ServiceHeartbeat {
            service_name: args.service_name,
            instance_id: args.instance_id,
            fencing_token: args.fencing_token,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceHeartbeatResult(result) => {
            let output = ServiceSuccessOutput {
                operation: "heartbeat".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_health(client: &AspenClient, args: HealthArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ServiceUpdateHealth {
            service_name: args.service_name,
            instance_id: args.instance_id,
            fencing_token: args.fencing_token,
            status: args.status.as_str().to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceUpdateHealthResult(result) => {
            let output = ServiceSuccessOutput {
                operation: "health".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn service_update(client: &AspenClient, args: UpdateArgs, json: bool) -> Result<()> {
    // Convert comma-separated tags to JSON array if provided
    let tags_json = match args.tags {
        Some(ref tags) if !tags.is_empty() => {
            let tags: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
            Some(serde_json::to_string(&tags)?)
        }
        _ => None,
    };

    let response = client
        .send(ClientRpcRequest::ServiceUpdateMetadata {
            service_name: args.service_name,
            instance_id: args.instance_id,
            fencing_token: args.fencing_token,
            version: args.svc_version,
            tags: tags_json,
            weight: args.weight,
            custom_metadata: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ServiceUpdateMetadataResult(result) => {
            let output = ServiceSuccessOutput {
                operation: "update".to_string(),
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
