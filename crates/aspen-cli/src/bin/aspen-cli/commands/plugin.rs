//! WASM plugin management commands.
//!
//! Install, list, enable, disable, and remove WASM handler plugins
//! from the cluster. Uses existing KV and blob RPC operations.

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_plugin_api::PLUGIN_KV_PREFIX;
use aspen_plugin_api::PluginManifest;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// WASM plugin operations.
#[derive(Subcommand)]
pub enum PluginCommand {
    /// Install a WASM plugin from a local file.
    Install(InstallArgs),

    /// List installed plugins.
    List,

    /// Show details for a plugin.
    Info(InfoArgs),

    /// Enable a disabled plugin.
    Enable(EnableArgs),

    /// Disable a plugin.
    Disable(DisableArgs),

    /// Remove a plugin.
    Remove(RemoveArgs),
}

#[derive(Args)]
pub struct InstallArgs {
    /// Path to the WASM binary.
    pub wasm_file: PathBuf,

    /// Unique plugin name.
    #[arg(long)]
    pub name: String,

    /// Request types this plugin handles (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub handles: Vec<String>,

    /// Dispatch priority (900-999, lower = earlier).
    #[arg(long, default_value = "950")]
    pub priority: u32,

    /// Semantic version string.
    #[arg(long, default_value = "0.1.0")]
    pub version: String,

    /// Fuel budget override.
    #[arg(long)]
    pub fuel_limit: Option<u64>,

    /// Memory limit override in bytes.
    #[arg(long)]
    pub memory_limit: Option<u64>,
}

#[derive(Args)]
pub struct InfoArgs {
    /// Plugin name.
    pub name: String,
}

#[derive(Args)]
pub struct EnableArgs {
    /// Plugin name.
    pub name: String,
}

#[derive(Args)]
pub struct DisableArgs {
    /// Plugin name.
    pub name: String,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Plugin name.
    pub name: String,
}

// ============================================================================
// Output types
// ============================================================================

struct InstallOutput {
    name: String,
    wasm_hash: String,
}

impl Outputable for InstallOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "installed",
            "name": self.name,
            "wasm_hash": self.wasm_hash
        })
    }

    fn to_human(&self) -> String {
        format!("Installed plugin '{}' (hash: {})", self.name, self.wasm_hash)
    }
}

struct PluginListOutput {
    plugins: Vec<PluginManifest>,
}

impl Outputable for PluginListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.plugins.len(),
            "plugins": self.plugins.iter().map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "enabled": p.enabled,
                    "priority": p.priority,
                    "handles": p.handles
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.plugins.is_empty() {
            return "No plugins installed".to_string();
        }

        let mut output = format!("Plugins ({})\n", self.plugins.len());
        output.push_str("Name                 | Version  | Enabled | Priority | Handles\n");
        output.push_str("---------------------+----------+---------+----------+--------\n");

        for p in &self.plugins {
            let enabled = if p.enabled { "yes" } else { "no" };
            let handles = p.handles.join(",");
            output.push_str(&format!(
                "{:20} | {:8} | {:7} | {:8} | {}\n",
                &p.name[..20.min(p.name.len())],
                &p.version[..8.min(p.version.len())],
                enabled,
                p.priority,
                handles,
            ));
        }

        output
    }
}

struct PluginInfoOutput {
    manifest: PluginManifest,
}

impl Outputable for PluginInfoOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.manifest.name,
            "version": self.manifest.version,
            "wasm_hash": self.manifest.wasm_hash,
            "enabled": self.manifest.enabled,
            "priority": self.manifest.priority,
            "handles": self.manifest.handles,
            "fuel_limit": self.manifest.fuel_limit,
            "memory_limit": self.manifest.memory_limit
        })
    }

    fn to_human(&self) -> String {
        let m = &self.manifest;
        let enabled = if m.enabled { "yes" } else { "no" };
        format!(
            "Plugin: {}\n\
             Version:      {}\n\
             Enabled:      {}\n\
             Priority:     {}\n\
             WASM Hash:    {}\n\
             Handles:      {}\n\
             Fuel Limit:   {}\n\
             Memory Limit: {}",
            m.name,
            m.version,
            enabled,
            m.priority,
            m.wasm_hash,
            m.handles.join(", "),
            m.fuel_limit.map_or("default".to_string(), |v| v.to_string()),
            m.memory_limit.map_or("default".to_string(), |v| v.to_string()),
        )
    }
}

struct ToggleOutput {
    name: String,
    enabled: bool,
}

impl Outputable for ToggleOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "enabled": self.enabled
        })
    }

    fn to_human(&self) -> String {
        let state = if self.enabled { "enabled" } else { "disabled" };
        format!("Plugin '{}' {}", self.name, state)
    }
}

struct RemoveOutput {
    name: String,
}

impl Outputable for RemoveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "removed",
            "name": self.name
        })
    }

    fn to_human(&self) -> String {
        format!("Plugin '{}' removed", self.name)
    }
}

// ============================================================================
// Command dispatch
// ============================================================================

impl PluginCommand {
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PluginCommand::Install(args) => plugin_install(client, args, json).await,
            PluginCommand::List => plugin_list(client, json).await,
            PluginCommand::Info(args) => plugin_info(client, args, json).await,
            PluginCommand::Enable(args) => plugin_toggle(client, &args.name, true, json).await,
            PluginCommand::Disable(args) => plugin_toggle(client, &args.name, false, json).await,
            PluginCommand::Remove(args) => plugin_remove(client, args, json).await,
        }
    }
}

// ============================================================================
// Subcommand implementations
// ============================================================================

async fn plugin_install(client: &AspenClient, args: InstallArgs, json: bool) -> Result<()> {
    // Read WASM binary
    let data =
        std::fs::read(&args.wasm_file).with_context(|| format!("failed to read {}", args.wasm_file.display()))?;

    // Upload to blob store
    let blob_response = client
        .send(ClientRpcRequest::AddBlob {
            data,
            tag: Some(format!("plugin:{}", args.name)),
        })
        .await?;

    let wasm_hash = match blob_response {
        ClientRpcResponse::AddBlobResult(r) if r.is_success => {
            r.hash.ok_or_else(|| anyhow::anyhow!("blob stored but no hash returned"))?
        }
        ClientRpcResponse::AddBlobResult(r) => {
            anyhow::bail!("failed to upload WASM blob: {}", r.error.unwrap_or_default());
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    // Build manifest
    let manifest = PluginManifest {
        name: args.name.clone(),
        version: args.version,
        wasm_hash: wasm_hash.clone(),
        handles: args.handles,
        priority: args.priority.max(900).min(999),
        fuel_limit: args.fuel_limit,
        memory_limit: args.memory_limit,
        enabled: true,
    };

    let manifest_json = serde_json::to_string(&manifest)?;
    let key = format!("{}{}", PLUGIN_KV_PREFIX, args.name);

    // Write manifest to KV store
    let write_response = client
        .send(ClientRpcRequest::WriteKey {
            key,
            value: manifest_json.into_bytes(),
        })
        .await?;

    match write_response {
        ClientRpcResponse::WriteResult(r) if r.is_success => {}
        ClientRpcResponse::WriteResult(r) => {
            anyhow::bail!("failed to write manifest: {}", r.error.unwrap_or_default());
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }

    print_output(
        &InstallOutput {
            name: args.name,
            wasm_hash,
        },
        json,
    );
    Ok(())
}

async fn plugin_list(client: &AspenClient, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ScanKeys {
            prefix: PLUGIN_KV_PREFIX.to_string(),
            limit: Some(100),
            continuation_token: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ScanResult(r) => {
            let mut plugins = Vec::new();
            for entry in r.entries {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&entry.value) {
                    plugins.push(manifest);
                }
            }
            print_output(&PluginListOutput { plugins }, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn plugin_info(client: &AspenClient, args: InfoArgs, json: bool) -> Result<()> {
    let key = format!("{}{}", PLUGIN_KV_PREFIX, args.name);

    let response = client.send(ClientRpcRequest::ReadKey { key }).await?;

    match response {
        ClientRpcResponse::ReadResult(r) if r.was_found => {
            let value = r.value.ok_or_else(|| anyhow::anyhow!("key found but no value"))?;
            let manifest: PluginManifest = serde_json::from_slice(&value).context("failed to parse plugin manifest")?;
            print_output(&PluginInfoOutput { manifest }, json);
            Ok(())
        }
        ClientRpcResponse::ReadResult(_) => {
            anyhow::bail!("plugin '{}' not found", args.name);
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn plugin_toggle(client: &AspenClient, name: &str, enabled: bool, json: bool) -> Result<()> {
    let key = format!("{}{}", PLUGIN_KV_PREFIX, name);

    // Read current manifest
    let response = client.send(ClientRpcRequest::ReadKey { key: key.clone() }).await?;

    let mut manifest = match response {
        ClientRpcResponse::ReadResult(r) if r.was_found => {
            let value = r.value.ok_or_else(|| anyhow::anyhow!("key found but no value"))?;
            serde_json::from_slice::<PluginManifest>(&value).context("failed to parse plugin manifest")?
        }
        ClientRpcResponse::ReadResult(_) => {
            anyhow::bail!("plugin '{}' not found", name);
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    manifest.enabled = enabled;
    let manifest_json = serde_json::to_string(&manifest)?;

    // Write updated manifest
    let write_response = client
        .send(ClientRpcRequest::WriteKey {
            key,
            value: manifest_json.into_bytes(),
        })
        .await?;

    match write_response {
        ClientRpcResponse::WriteResult(r) if r.is_success => {}
        ClientRpcResponse::WriteResult(r) => {
            anyhow::bail!("failed to update manifest: {}", r.error.unwrap_or_default());
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }

    print_output(
        &ToggleOutput {
            name: name.to_string(),
            enabled,
        },
        json,
    );
    Ok(())
}

async fn plugin_remove(client: &AspenClient, args: RemoveArgs, json: bool) -> Result<()> {
    let key = format!("{}{}", PLUGIN_KV_PREFIX, args.name);

    let response = client.send(ClientRpcRequest::DeleteKey { key }).await?;

    match response {
        ClientRpcResponse::DeleteResult(r) if r.was_deleted => {
            print_output(&RemoveOutput { name: args.name }, json);
            Ok(())
        }
        ClientRpcResponse::DeleteResult(_) => {
            anyhow::bail!("plugin '{}' not found", args.name);
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// ============================================================================
// Tests (Deliverable 3c)
// ============================================================================

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::Cli;

    #[test]
    fn plugin_list_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "list"]);
        assert!(result.is_ok(), "plugin list should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_fails_missing_required_args() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "install", "test.wasm"]);
        assert!(result.is_err(), "install without --name and --handles should fail");
    }

    #[test]
    fn plugin_install_succeeds_with_required_args() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--name",
            "my-plugin",
            "--handles",
            "ReadKey",
        ]);
        assert!(result.is_ok(), "install with required args should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_multiple_handles() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--name",
            "my-plugin",
            "--handles",
            "ReadKey,WriteKey",
        ]);
        assert!(result.is_ok(), "install with comma-separated handles should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_enable_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "enable", "my-plugin"]);
        assert!(result.is_ok(), "plugin enable should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_disable_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "disable", "my-plugin"]);
        assert!(result.is_ok(), "plugin disable should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_remove_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "remove", "my-plugin"]);
        assert!(result.is_ok(), "plugin remove should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_info_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "info", "my-plugin"]);
        assert!(result.is_ok(), "plugin info should parse: {:?}", result.err());
    }
}
