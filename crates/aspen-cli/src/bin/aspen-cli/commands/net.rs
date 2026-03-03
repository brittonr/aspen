//! Net service mesh commands.
//!
//! Commands for publishing, discovering, and listing services in
//! the Aspen service mesh.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Service mesh operations.
#[derive(Subcommand)]
pub enum NetCommand {
    /// Publish a service to the mesh.
    Publish(PublishArgs),

    /// Unpublish a service from the mesh.
    Unpublish(UnpublishArgs),

    /// List services in the mesh.
    Services(ServicesArgs),

    /// Look up a service by name.
    Lookup(LookupArgs),
}

#[derive(Args)]
pub struct PublishArgs {
    /// Service name (e.g., "my-api").
    pub name: String,

    /// Endpoint ID of the publishing node.
    #[arg(long)]
    pub endpoint_id: String,

    /// Port the service listens on.
    #[arg(long)]
    pub port: u16,

    /// Protocol (default: tcp).
    #[arg(long, default_value = "tcp")]
    pub proto: String,

    /// Tags for filtering (repeatable).
    #[arg(long = "tag")]
    pub tags: Vec<String>,
}

#[derive(Args)]
pub struct UnpublishArgs {
    /// Service name to remove.
    pub name: String,
}

#[derive(Args)]
pub struct ServicesArgs {
    /// Filter by tag.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct LookupArgs {
    /// Service name to look up.
    pub name: String,
}

impl NetCommand {
    /// Execute the net command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            NetCommand::Publish(args) => net_publish(client, args, json).await,
            NetCommand::Unpublish(args) => net_unpublish(client, args, json).await,
            NetCommand::Services(args) => net_services(client, args, json).await,
            NetCommand::Lookup(args) => net_lookup(client, args, json).await,
        }
    }
}

// =========================================================================
// Command implementations
// =========================================================================

async fn net_publish(client: &AspenClient, args: PublishArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::NetPublish {
            name: args.name.clone(),
            endpoint_id: args.endpoint_id,
            port: args.port,
            proto: args.proto,
            tags: args.tags,
        })
        .await?;

    match response {
        ClientRpcResponse::NetPublishResult(result) => {
            let output = PublishOutput {
                name: args.name,
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn net_unpublish(client: &AspenClient, args: UnpublishArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::NetUnpublish {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::NetUnpublishResult(result) => {
            let output = UnpublishOutput {
                name: args.name,
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn net_services(client: &AspenClient, args: ServicesArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::NetList { tag_filter: args.tag }).await?;

    match response {
        ClientRpcResponse::NetListResult(result) => {
            if let Some(err) = result.error {
                anyhow::bail!("list failed: {}", err);
            }
            let output = ServicesOutput {
                services: result
                    .services
                    .into_iter()
                    .map(|s| ServiceRow {
                        name: s.name,
                        endpoint_id: s.endpoint_id,
                        port: s.port,
                        proto: s.proto,
                        tags: s.tags,
                    })
                    .collect(),
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn net_lookup(client: &AspenClient, args: LookupArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::NetLookup {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::NetLookupResult(result) => {
            if let Some(err) = result.error {
                anyhow::bail!("lookup failed: {}", err);
            }
            match result.entry {
                Some(entry) => {
                    let output = LookupOutput {
                        name: entry.name,
                        endpoint_id: entry.endpoint_id,
                        port: entry.port,
                        proto: entry.proto,
                        tags: entry.tags,
                        found: true,
                    };
                    print_output(&output, json);
                }
                None => {
                    let output = LookupOutput {
                        name: args.name,
                        endpoint_id: String::new(),
                        port: 0,
                        proto: String::new(),
                        tags: vec![],
                        found: false,
                    };
                    print_output(&output, json);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =========================================================================
// Output types
// =========================================================================

struct PublishOutput {
    name: String,
    is_success: bool,
    error: Option<String>,
}

impl Outputable for PublishOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "success": self.is_success,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Published service '{}'", self.name)
        } else {
            format!("Failed to publish '{}': {}", self.name, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

struct UnpublishOutput {
    name: String,
    is_success: bool,
    error: Option<String>,
}

impl Outputable for UnpublishOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "success": self.is_success,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Unpublished service '{}'", self.name)
        } else {
            format!("Failed to unpublish '{}': {}", self.name, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

struct ServiceRow {
    name: String,
    endpoint_id: String,
    port: u16,
    proto: String,
    tags: Vec<String>,
}

struct ServicesOutput {
    services: Vec<ServiceRow>,
}

impl Outputable for ServicesOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "services": self.services.iter().map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "endpoint_id": s.endpoint_id,
                    "port": s.port,
                    "proto": s.proto,
                    "tags": s.tags,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn to_human(&self) -> String {
        if self.services.is_empty() {
            return "No services found.".to_string();
        }
        let mut lines = vec![format!(
            "{:<24} {:<20} {:>5} {:<5} {}",
            "NAME", "ENDPOINT", "PORT", "PROTO", "TAGS"
        )];
        for s in &self.services {
            lines.push(format!(
                "{:<24} {:<20} {:>5} {:<5} {}",
                s.name,
                &s.endpoint_id[..s.endpoint_id.len().min(18)],
                s.port,
                s.proto,
                s.tags.join(","),
            ));
        }
        lines.join("\n")
    }
}

struct LookupOutput {
    name: String,
    endpoint_id: String,
    port: u16,
    proto: String,
    tags: Vec<String>,
    found: bool,
}

impl Outputable for LookupOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "found": self.found,
            "endpoint_id": self.endpoint_id,
            "port": self.port,
            "proto": self.proto,
            "tags": self.tags,
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!("Service '{}' not found.", self.name);
        }
        format!(
            "Service: {}\n  Endpoint: {}\n  Port: {}\n  Proto: {}\n  Tags: {}",
            self.name,
            self.endpoint_id,
            self.port,
            self.proto,
            if self.tags.is_empty() {
                "(none)".to_string()
            } else {
                self.tags.join(", ")
            }
        )
    }
}
