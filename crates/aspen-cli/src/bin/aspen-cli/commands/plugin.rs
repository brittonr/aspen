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

    /// Reload WASM plugins (hot-reload). Reloads a single plugin by name,
    /// or all plugins if no name is given.
    Reload(ReloadArgs),

    /// Search installed plugins by name, description, or tag.
    Search(SearchArgs),

    /// Show dependency information.
    Deps(DepsArgs),

    /// Validate the dependency graph.
    Check,
}

#[derive(Args)]
pub struct InstallArgs {
    /// Path to the WASM binary.
    pub wasm_file: PathBuf,

    /// Path to a plugin.json manifest file. When provided, name, handles,
    /// priority, and version are read from the manifest. CLI flags override
    /// manifest values.
    #[arg(long)]
    pub manifest: Option<PathBuf>,

    /// Unique plugin name (required unless --manifest is provided).
    #[arg(long)]
    pub name: Option<String>,

    /// Request types this plugin handles (comma-separated; required unless --manifest is provided).
    #[arg(long, value_delimiter = ',')]
    pub handles: Option<Vec<String>>,

    /// Dispatch priority (900-999, lower = earlier).
    #[arg(long)]
    pub priority: Option<u32>,

    /// Semantic version string.
    #[arg(long = "plugin-version")]
    pub plugin_version: Option<String>,

    /// Fuel budget override.
    #[arg(long)]
    pub fuel_limit: Option<u64>,

    /// Memory limit override in bytes.
    #[arg(long)]
    pub memory_limit: Option<u64>,

    /// Application ID for federation discovery (e.g., "forge").
    #[arg(long)]
    pub app_id: Option<String>,

    /// Allowed KV key prefixes (comma-separated). When empty, defaults to `__plugin:{name}:`.
    #[arg(long, value_delimiter = ',')]
    pub kv_prefixes: Option<Vec<String>>,

    /// Plugin description.
    #[arg(long)]
    pub description: Option<String>,

    /// Plugin author.
    #[arg(long)]
    pub author: Option<String>,

    /// Plugin tags (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,

    /// Minimum API version required.
    #[arg(long)]
    pub min_api_version: Option<String>,

    /// Force install even with dependency/version errors.
    #[arg(long)]
    pub force: bool,
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

    /// Force removal even if other plugins depend on this one.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ReloadArgs {
    /// Plugin name to reload. If omitted, all plugins are reloaded.
    pub name: Option<String>,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (substring match on name/description).
    pub query: Option<String>,

    /// Filter by tag.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct DepsArgs {
    /// Plugin name (omit for full dependency graph).
    pub name: Option<String>,

    /// Show reverse dependencies.
    #[arg(long)]
    pub reverse: bool,
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
                    "handles": p.handles,
                    "app_id": p.app_id,
                    "description": p.description,
                    "author": p.author,
                    "tags": p.tags,
                    "dependencies": p.dependencies
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.plugins.is_empty() {
            return "No plugins installed".to_string();
        }

        let mut output = format!("Plugins ({})\n", self.plugins.len());
        output.push_str("Name                 | Version  | Enabled | Priority | Deps | Tags\n");
        output.push_str("---------------------+----------+---------+----------+------+-----\n");

        for p in &self.plugins {
            let enabled = if p.enabled { "yes" } else { "no" };
            let tags = p.tags.join(",");
            let deps_count = p.dependencies.len();
            output.push_str(&format!(
                "{:20} | {:8} | {:7} | {:8} | {:4} | {}\n",
                &p.name[..20.min(p.name.len())],
                &p.version[..8.min(p.version.len())],
                enabled,
                p.priority,
                deps_count,
                tags,
            ));
        }

        output
    }
}

struct PluginInfoOutput {
    manifest: PluginManifest,
    dependents: Vec<String>,
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
            "memory_limit": self.manifest.memory_limit,
            "app_id": self.manifest.app_id,
            "description": self.manifest.description,
            "author": self.manifest.author,
            "tags": self.manifest.tags,
            "min_api_version": self.manifest.min_api_version,
            "dependencies": self.manifest.dependencies,
            "dependents": self.dependents
        })
    }

    fn to_human(&self) -> String {
        let m = &self.manifest;
        let enabled = if m.enabled { "yes" } else { "no" };
        let description = m.description.as_deref().unwrap_or("(none)");
        let author = m.author.as_deref().unwrap_or("(none)");
        let tags = if m.tags.is_empty() {
            "(none)".to_string()
        } else {
            m.tags.join(", ")
        };
        let api_version = m.min_api_version.as_deref().map_or("(none)".to_string(), |v| format!(">= {}", v));
        let deps = if m.dependencies.is_empty() {
            "(none)".to_string()
        } else {
            m.dependencies
                .iter()
                .map(|d| {
                    let opt = if d.optional { " (optional)" } else { "" };
                    let ver = d.min_version.as_deref().map_or("".to_string(), |v| format!(" >= {}", v));
                    format!("{}{}{}", d.name, ver, opt)
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let dependents = if self.dependents.is_empty() {
            "(none)".to_string()
        } else {
            self.dependents.join(", ")
        };

        format!(
            "Plugin: {}\n\
             Version:      {}\n\
             Description:  {}\n\
             Author:       {}\n\
             Tags:         {}\n\
             API Version:  {}\n\
             Dependencies: {}\n\
             Dependents:   {}\n\
             Enabled:      {}\n\
             Priority:     {}\n\
             WASM Hash:    {}\n\
             Handles:      {}\n\
             Fuel Limit:   {}\n\
             Memory Limit: {}\n\
             App ID:       {}",
            m.name,
            m.version,
            description,
            author,
            tags,
            api_version,
            deps,
            dependents,
            enabled,
            m.priority,
            m.wasm_hash,
            m.handles.join(", "),
            m.fuel_limit.map_or("default".to_string(), |v| v.to_string()),
            m.memory_limit.map_or("default".to_string(), |v| v.to_string()),
            m.app_id.as_deref().unwrap_or("none"),
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

struct ReloadOutput {
    is_success: bool,
    plugin_count: u32,
    message: String,
}

impl Outputable for ReloadOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "plugin_count": self.plugin_count,
            "message": self.message
        })
    }

    fn to_human(&self) -> String {
        self.message.clone()
    }
}

struct SearchOutput {
    results: Vec<PluginManifest>,
}

impl Outputable for SearchOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.results.len(),
            "results": self.results.iter().map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "description": p.description,
                    "tags": p.tags,
                    "author": p.author
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.results.is_empty() {
            return "No plugins found".to_string();
        }

        let mut output = format!("Found {} plugin(s)\n", self.results.len());
        for p in &self.results {
            let desc = p.description.as_deref().unwrap_or("(no description)");
            let tags = if p.tags.is_empty() {
                "(no tags)".to_string()
            } else {
                p.tags.join(", ")
            };
            output.push_str(&format!("\n{} ({})\n  {}\n  Tags: {}\n", p.name, p.version, desc, tags));
        }
        output
    }
}

struct DepsOutput {
    message: String,
}

impl Outputable for DepsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "message": self.message
        })
    }

    fn to_human(&self) -> String {
        self.message.clone()
    }
}

struct CheckOutput {
    is_valid: bool,
    errors: Vec<String>,
}

impl Outputable for CheckOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_valid": self.is_valid,
            "errors": self.errors
        })
    }

    fn to_human(&self) -> String {
        if self.is_valid {
            "Dependency graph is valid".to_string()
        } else {
            let mut output = format!("Dependency graph has {} error(s):\n", self.errors.len());
            for err in &self.errors {
                output.push_str(&format!("  - {}\n", err));
            }
            output
        }
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
            PluginCommand::Reload(args) => plugin_reload(client, args, json).await,
            PluginCommand::Search(args) => plugin_search(client, args, json).await,
            PluginCommand::Deps(args) => plugin_deps(client, args, json).await,
            PluginCommand::Check => plugin_check(client, json).await,
        }
    }
}

// ============================================================================
// Subcommand implementations
// ============================================================================

/// Resolved install parameters after merging manifest + CLI flags.
struct ResolvedInstall {
    name: String,
    handles: Vec<String>,
    priority: u32,
    version: String,
    fuel_limit: Option<u64>,
    memory_limit: Option<u64>,
    app_id: Option<String>,
    kv_prefixes: Vec<String>,
    permissions: Option<aspen_plugin_api::PluginPermissions>,
    description: Option<String>,
    author: Option<String>,
    tags: Vec<String>,
    min_api_version: Option<String>,
    dependencies: Vec<aspen_plugin_api::PluginDependency>,
}

fn resolve_install_args(args: &InstallArgs) -> Result<ResolvedInstall> {
    // If --manifest is provided, load it as the base.
    let base: Option<aspen_plugin_api::PluginInfo> = match &args.manifest {
        Some(path) => {
            let bytes = std::fs::read(path).with_context(|| format!("failed to read manifest {}", path.display()))?;
            let info: aspen_plugin_api::PluginInfo =
                serde_json::from_slice(&bytes).with_context(|| format!("invalid manifest {}", path.display()))?;
            Some(info)
        }
        None => None,
    };

    // For fields not in PluginInfo, parse from raw manifest JSON
    let raw: Option<serde_json::Value> = match &args.manifest {
        Some(path) => {
            let bytes = std::fs::read(path)?;
            Some(serde_json::from_slice(&bytes)?)
        }
        None => None,
    };

    // CLI flags override manifest values.
    let name = args
        .name
        .clone()
        .or_else(|| base.as_ref().map(|b| b.name.clone()))
        .ok_or_else(|| anyhow::anyhow!("--name is required when --manifest is not provided"))?;

    let handles = args
        .handles
        .clone()
        .or_else(|| base.as_ref().map(|b| b.handles.clone()))
        .ok_or_else(|| anyhow::anyhow!("--handles is required when --manifest is not provided"))?;

    let priority = args.priority.or_else(|| base.as_ref().map(|b| b.priority)).unwrap_or(950);

    let version = args
        .plugin_version
        .clone()
        .or_else(|| base.as_ref().map(|b| b.version.clone()))
        .unwrap_or_else(|| "0.1.0".to_string());

    let app_id = args.app_id.clone().or_else(|| base.as_ref().and_then(|b| b.app_id.clone()));

    let kv_prefixes = args
        .kv_prefixes
        .clone()
        .or_else(|| base.as_ref().map(|b| b.kv_prefixes.clone()))
        .unwrap_or_default();

    let description = args.description.clone().or_else(|| base.as_ref().and_then(|b| b.description.clone()));

    let author = args.author.clone().or_else(|| raw.as_ref().and_then(|v| v["author"].as_str().map(String::from)));

    let tags = args
        .tags
        .clone()
        .or_else(|| {
            raw.as_ref().and_then(|v| {
                v["tags"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            })
        })
        .unwrap_or_default();

    let min_api_version = args
        .min_api_version
        .clone()
        .or_else(|| raw.as_ref().and_then(|v| v["min_api_version"].as_str().map(String::from)));

    let dependencies: Vec<aspen_plugin_api::PluginDependency> = raw
        .as_ref()
        .and_then(|v| serde_json::from_value(v["dependencies"].clone()).ok())
        .unwrap_or_default();

    Ok(ResolvedInstall {
        name,
        handles,
        priority,
        version,
        fuel_limit: args.fuel_limit,
        memory_limit: args.memory_limit,
        app_id,
        kv_prefixes,
        permissions: base.as_ref().map(|b| b.permissions.clone()),
        description,
        author,
        tags,
        min_api_version,
        dependencies,
    })
}

async fn fetch_installed_manifests(client: &AspenClient) -> Result<Vec<PluginManifest>> {
    let response = client
        .send(ClientRpcRequest::ScanKeys {
            prefix: PLUGIN_KV_PREFIX.to_string(),
            limit: Some(100),
            continuation_token: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ScanResult(r) => {
            let mut manifests = Vec::new();
            for entry in r.entries {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&entry.value) {
                    manifests.push(manifest);
                }
            }
            Ok(manifests)
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn plugin_install(client: &AspenClient, args: InstallArgs, json: bool) -> Result<()> {
    let resolved = resolve_install_args(&args)?;

    // Read WASM binary
    let data =
        std::fs::read(&args.wasm_file).with_context(|| format!("failed to read {}", args.wasm_file.display()))?;

    // Upload to blob store
    let blob_response = client
        .send(ClientRpcRequest::AddBlob {
            data,
            tag: Some(format!("plugin:{}", resolved.name)),
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
        name: resolved.name.clone(),
        version: resolved.version,
        wasm_hash: wasm_hash.clone(),
        handles: resolved.handles,
        priority: resolved.priority.max(900).min(999),
        fuel_limit: resolved.fuel_limit,
        memory_limit: resolved.memory_limit,
        enabled: true,
        app_id: resolved.app_id,
        execution_timeout_secs: None,
        kv_prefixes: resolved.kv_prefixes,
        permissions: resolved.permissions.unwrap_or_default(),
        signature: None,
        description: resolved.description,
        author: resolved.author,
        tags: resolved.tags,
        min_api_version: resolved.min_api_version,
        dependencies: resolved.dependencies,
    };

    // Validate dependencies against installed plugins
    let installed = fetch_installed_manifests(client).await?;
    if let Err(errors) = aspen_plugin_api::resolve::validate_install(&manifest, &installed) {
        for err in &errors {
            eprintln!("Dependency error: {err}");
        }
        if !args.force {
            anyhow::bail!("Install blocked by dependency errors. Use --force to override.");
        }
        eprintln!("Warning: installing despite dependency errors (--force)");
    }

    // Check API version
    if let Err(e) = aspen_plugin_api::resolve::check_api_version(&manifest) {
        if !args.force {
            anyhow::bail!("API version error: {e}. Use --force to override.");
        }
        eprintln!("Warning: {e} (--force)");
    }

    let manifest_json = serde_json::to_string(&manifest)?;
    let key = format!("{}{}", PLUGIN_KV_PREFIX, resolved.name);

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
            name: resolved.name,
            wasm_hash,
        },
        json,
    );
    Ok(())
}

async fn plugin_list(client: &AspenClient, json: bool) -> Result<()> {
    let plugins = fetch_installed_manifests(client).await?;
    print_output(&PluginListOutput { plugins }, json);
    Ok(())
}

async fn plugin_info(client: &AspenClient, args: InfoArgs, json: bool) -> Result<()> {
    let key = format!("{}{}", PLUGIN_KV_PREFIX, args.name);

    let response = client.send(ClientRpcRequest::ReadKey { key }).await?;

    match response {
        ClientRpcResponse::ReadResult(r) if r.was_found => {
            let value = r.value.ok_or_else(|| anyhow::anyhow!("key found but no value"))?;
            let manifest: PluginManifest = serde_json::from_slice(&value).context("failed to parse plugin manifest")?;

            // Fetch all manifests to compute reverse dependencies
            let installed = fetch_installed_manifests(client).await?;
            let dependents = aspen_plugin_api::resolve::reverse_dependents(&args.name, &installed);

            print_output(&PluginInfoOutput { manifest, dependents }, json);
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
    let installed = fetch_installed_manifests(client).await?;
    let dependents = aspen_plugin_api::resolve::reverse_dependents(&args.name, &installed);

    if !dependents.is_empty() && !args.force {
        anyhow::bail!("Cannot remove '{}': required by {}. Use --force to override.", args.name, dependents.join(", "));
    }

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

async fn plugin_reload(client: &AspenClient, args: ReloadArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PluginReload { name: args.name }).await?;

    match response {
        ClientRpcResponse::PluginReloadResult(r) => {
            if !r.is_success {
                if !json {
                    eprintln!("Error: {}", r.error.unwrap_or_default());
                }
            }
            print_output(
                &ReloadOutput {
                    is_success: r.is_success,
                    plugin_count: r.plugin_count,
                    message: r.message,
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn plugin_search(client: &AspenClient, args: SearchArgs, json: bool) -> Result<()> {
    let manifests = fetch_installed_manifests(client).await?;

    let results: Vec<PluginManifest> = manifests
        .into_iter()
        .filter(|m| {
            // Filter by query (substring match on name or description)
            let query_match = args.query.as_ref().map_or(true, |q| {
                let q_lower = q.to_lowercase();
                m.name.to_lowercase().contains(&q_lower)
                    || m.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&q_lower))
            });

            // Filter by tag
            let tag_match = args.tag.as_ref().map_or(true, |t| m.tags.iter().any(|tag| tag == t));

            query_match && tag_match
        })
        .collect();

    print_output(&SearchOutput { results }, json);
    Ok(())
}

async fn plugin_deps(client: &AspenClient, args: DepsArgs, json: bool) -> Result<()> {
    let manifests = fetch_installed_manifests(client).await?;

    let message = if let Some(ref name) = args.name {
        // Show dependencies for a specific plugin
        let manifest = manifests
            .iter()
            .find(|m| m.name == *name)
            .ok_or_else(|| anyhow::anyhow!("plugin '{}' not found", name))?;

        if args.reverse {
            // Show reverse dependencies (what depends on this plugin)
            let dependents = aspen_plugin_api::resolve::reverse_dependents(name, &manifests);
            if dependents.is_empty() {
                format!("No plugins depend on '{}'", name)
            } else {
                format!("Plugins depending on '{}':\n  {}", name, dependents.join("\n  "))
            }
        } else {
            // Show forward dependencies
            if manifest.dependencies.is_empty() {
                format!("Plugin '{}' has no dependencies", name)
            } else {
                let mut output = format!("Dependencies for '{}':\n", name);
                for dep in &manifest.dependencies {
                    let opt = if dep.optional { " (optional)" } else { "" };
                    let ver = dep.min_version.as_deref().map_or("".to_string(), |v| format!(" >= {}", v));
                    let satisfied = manifests.iter().any(|m| {
                        m.name == dep.name
                            && dep.min_version.as_ref().map_or(true, |min| {
                                // Simple version comparison (would need semver crate for proper handling)
                                m.version >= *min
                            })
                    });
                    let status = if satisfied { "✓" } else { "✗" };
                    output.push_str(&format!("  {} {}{}{}\n", status, dep.name, ver, opt));
                }
                output
            }
        }
    } else {
        // Show full dependency graph in topological order
        match aspen_plugin_api::resolve::resolve_load_order(&manifests) {
            Ok(order) => {
                let mut output = "Plugin load order (topological):\n".to_string();
                for (i, manifest) in order.iter().enumerate() {
                    output.push_str(&format!("  {}. {}\n", i + 1, manifest.name));
                }
                output
            }
            Err(errors) => {
                let mut output = "Dependency graph has errors:\n".to_string();
                for err in &errors {
                    output.push_str(&format!("  - {}\n", err));
                }
                output
            }
        }
    };

    print_output(&DepsOutput { message }, json);
    Ok(())
}

async fn plugin_check(client: &AspenClient, json: bool) -> Result<()> {
    let manifests = fetch_installed_manifests(client).await?;

    let result = aspen_plugin_api::resolve::resolve_load_order(&manifests);

    let (is_valid, errors) = match result {
        Ok(_) => (true, Vec::new()),
        Err(errs) => (false, errs.into_iter().map(|e| e.to_string()).collect()),
    };

    print_output(&CheckOutput { is_valid, errors }, json);

    if !is_valid {
        std::process::exit(1);
    }

    Ok(())
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
    fn plugin_install_parses_wasm_only() {
        // Without --name/--handles/--manifest, clap still parses (they are all
        // optional now). Validation happens at runtime in resolve_install_args.
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "install", "test.wasm"]);
        assert!(result.is_ok(), "install with just wasm_file should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_succeeds_with_name_and_handles() {
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
    fn plugin_install_with_manifest_flag() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--manifest",
            "plugin.json",
        ]);
        assert!(result.is_ok(), "install with --manifest should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_manifest_with_overrides() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--manifest",
            "plugin.json",
            "--name",
            "custom-name",
            "--priority",
            "910",
        ]);
        assert!(result.is_ok(), "install with --manifest and overrides should parse: {:?}", result.err());
    }

    #[test]
    fn resolve_requires_name_without_manifest() {
        let args = super::InstallArgs {
            wasm_file: "test.wasm".into(),
            manifest: None,
            name: None,
            handles: Some(vec!["ReadKey".to_string()]),
            priority: None,
            plugin_version: None,
            fuel_limit: None,
            memory_limit: None,
            app_id: None,
            kv_prefixes: None,
            description: None,
            author: None,
            tags: None,
            min_api_version: None,
            force: false,
        };
        let result = super::resolve_install_args(&args);
        assert!(result.is_err(), "should fail without --name or --manifest");
    }

    #[test]
    fn resolve_requires_handles_without_manifest() {
        let args = super::InstallArgs {
            wasm_file: "test.wasm".into(),
            manifest: None,
            name: Some("my-plugin".to_string()),
            handles: None,
            priority: None,
            plugin_version: None,
            fuel_limit: None,
            memory_limit: None,
            app_id: None,
            kv_prefixes: None,
            description: None,
            author: None,
            tags: None,
            min_api_version: None,
            force: false,
        };
        let result = super::resolve_install_args(&args);
        assert!(result.is_err(), "should fail without --handles or --manifest");
    }

    #[test]
    fn resolve_succeeds_with_name_and_handles() {
        let args = super::InstallArgs {
            wasm_file: "test.wasm".into(),
            manifest: None,
            name: Some("my-plugin".to_string()),
            handles: Some(vec!["ReadKey".to_string(), "WriteKey".to_string()]),
            priority: Some(910),
            plugin_version: Some("1.0.0".to_string()),
            fuel_limit: None,
            memory_limit: None,
            app_id: None,
            kv_prefixes: None,
            description: None,
            author: None,
            tags: None,
            min_api_version: None,
            force: false,
        };
        let resolved = super::resolve_install_args(&args).expect("should resolve");
        assert_eq!(resolved.name, "my-plugin");
        assert_eq!(resolved.handles, vec!["ReadKey", "WriteKey"]);
        assert_eq!(resolved.priority, 910);
        assert_eq!(resolved.version, "1.0.0");
    }

    #[test]
    fn resolve_defaults_priority_and_version() {
        let args = super::InstallArgs {
            wasm_file: "test.wasm".into(),
            manifest: None,
            name: Some("my-plugin".to_string()),
            handles: Some(vec!["Ping".to_string()]),
            priority: None,
            plugin_version: None,
            fuel_limit: None,
            memory_limit: None,
            app_id: None,
            kv_prefixes: None,
            description: None,
            author: None,
            tags: None,
            min_api_version: None,
            force: false,
        };
        let resolved = super::resolve_install_args(&args).expect("should resolve");
        assert_eq!(resolved.priority, 950);
        assert_eq!(resolved.version, "0.1.0");
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

    #[test]
    fn plugin_reload_all_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "reload"]);
        assert!(result.is_ok(), "plugin reload should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_reload_single_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "reload", "my-plugin"]);
        assert!(result.is_ok(), "plugin reload with name should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_search_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "search"]);
        assert!(result.is_ok(), "plugin search should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_search_with_query_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "search", "forge"]);
        assert!(result.is_ok(), "plugin search with query should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_search_with_tag_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "search", "--tag", "git"]);
        assert!(result.is_ok(), "plugin search with tag should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_deps_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "deps"]);
        assert!(result.is_ok(), "plugin deps should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_deps_with_name_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "deps", "my-plugin"]);
        assert!(result.is_ok(), "plugin deps with name should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_deps_reverse_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "deps", "my-plugin", "--reverse"]);
        assert!(result.is_ok(), "plugin deps --reverse should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_check_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "check"]);
        assert!(result.is_ok(), "plugin check should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_force_parses() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--name",
            "my-plugin",
            "--handles",
            "ReadKey",
            "--force",
        ]);
        assert!(result.is_ok(), "plugin install --force should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_remove_force_parses() {
        let result = Cli::try_parse_from(["aspen-cli", "plugin", "remove", "my-plugin", "--force"]);
        assert!(result.is_ok(), "plugin remove --force should parse: {:?}", result.err());
    }

    #[test]
    fn plugin_install_with_new_args_parses() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "plugin",
            "install",
            "test.wasm",
            "--name",
            "my-plugin",
            "--handles",
            "ReadKey",
            "--description",
            "Test plugin",
            "--author",
            "Test Author",
            "--tags",
            "test,example",
            "--min-api-version",
            "0.2.0",
        ]);
        assert!(result.is_ok(), "plugin install with new args should parse: {:?}", result.err());
    }
}
