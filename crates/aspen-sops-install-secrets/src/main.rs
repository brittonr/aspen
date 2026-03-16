//! sops-install-secrets: Rust drop-in replacement for sops-nix's Go binary.
//!
//! Reads the sops-nix manifest JSON, decrypts SOPS files using Aspen Transit
//! (with age fallback), and installs secrets to a ramfs with correct permissions.
//!
//! Usage:
//!   sops-install-secrets [-check-mode=off|manifest|sopsfile] [--ignore-passwd] manifest.json

mod age_keys;
mod decrypt;
mod filesystem;
mod manifest;
mod templates;

use std::fs;
use std::process;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use manifest::Manifest;
use tracing::debug;
use tracing::info;

/// sops-install-secrets: decrypt and install SOPS secrets for NixOS.
#[derive(Parser, Debug)]
#[command(name = "sops-install-secrets", version)]
struct Cli {
    /// Manifest JSON file path.
    manifest: String,

    /// Validation mode: "off" (default), "manifest", or "sopsfile".
    #[arg(long = "check-mode", default_value = "off")]
    check_mode: String,

    /// Don't look up users/groups in /etc/passwd. Everything owned by root:root.
    #[arg(long = "ignore-passwd")]
    ignore_passwd: bool,
}

#[tokio::main]
async fn main() {
    // Go's `flag` package accepts single-dash long flags: `-check-mode=sopsfile`.
    // sops-nix's manifest-for.nix calls us with that syntax. Clap expects `--`.
    // Rewrite args before parsing.
    let args: Vec<String> = std::env::args()
        .map(|a| {
            if let Some(rest) = a.strip_prefix("-check-mode") {
                format!("--check-mode{rest}")
            } else if let Some(rest) = a.strip_prefix("-ignore-passwd") {
                format!("--ignore-passwd{rest}")
            } else {
                a
            }
        })
        .collect();
    let cli = Cli::parse_from(args);

    let filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();

    if let Err(e) = run(cli).await {
        eprintln!("sops-install-secrets: {e:#}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    // Parse manifest
    let manifest_contents =
        fs::read_to_string(&cli.manifest).with_context(|| format!("failed to open manifest '{}'", cli.manifest))?;

    let manifest: Manifest =
        serde_json::from_str(&manifest_contents).with_context(|| "failed to parse manifest JSON")?;

    debug!(secrets = manifest.secrets.len(), templates = manifest.templates.len(), "manifest parsed");

    // Check mode: manifest (structure validation only)
    if cli.check_mode == "manifest" {
        validate_manifest_structure(&manifest)?;
        return Ok(());
    }

    // Check mode: sopsfile (validate SOPS files exist and contain keys)
    if cli.check_mode == "sopsfile" {
        validate_manifest_structure(&manifest)?;
        for secret in &manifest.secrets {
            decrypt::validate_sops_file(secret)?;
        }
        return Ok(());
    }

    // ── Normal mode: decrypt and install ──

    let is_dry = std::env::var("NIXOS_ACTION").ok().is_some_and(|v| v == "dry-activate");

    let ignore_passwd = cli.ignore_passwd || is_dry;

    // Look up keys group
    let keys_gid = if ignore_passwd {
        0
    } else {
        filesystem::lookup_keys_gid()?
    };

    // Mount secrets filesystem
    filesystem::mount_secrets_fs(&manifest.secrets_mount_point, keys_gid, manifest.use_tmpfs)?;

    // Import age keys (SSH-to-age conversion + age key file)
    let age_key_path =
        age_keys::import_age_keys(&manifest, &manifest.secrets_mount_point, manifest.logging.key_import)?;

    // Decrypt all secrets
    let decrypted = decrypt::decrypt_secrets(&manifest, age_key_path.as_deref()).await?;

    // Render templates
    let mut rendered_templates = Vec::new();
    for template in &manifest.templates {
        let source = templates::load_template_source(template)?;
        let rendered = templates::render_template(&source, &manifest.placeholder_by_secret_name, &decrypted);
        rendered_templates.push(rendered);
    }

    // Prepare new generation directory
    let (gen_dir, gen_num) =
        filesystem::prepare_generation(&manifest.secrets_mount_point, &manifest.symlink_path, keys_gid)?;

    // Write secrets
    for secret in &manifest.secrets {
        let value = decrypted
            .get(&secret.name)
            .with_context(|| format!("no decrypted value for secret '{}'", secret.name))?;
        filesystem::write_secret(&gen_dir, secret, value, keys_gid, ignore_passwd)?;
    }

    // Write rendered templates
    for (template, rendered) in manifest.templates.iter().zip(rendered_templates.iter()) {
        filesystem::write_template(&gen_dir, template, rendered, keys_gid, ignore_passwd)?;
    }

    // Detect changes and write restart/reload lists
    filesystem::detect_changes(
        &manifest.symlink_path,
        &gen_dir,
        &manifest.secrets,
        &manifest.templates,
        is_dry,
        manifest.logging.secret_changes,
    )?;

    // Don't update symlinks during dry activation
    if is_dry {
        return Ok(());
    }

    // Atomic symlink update
    filesystem::atomic_symlink(&gen_dir, &manifest.symlink_path)?;

    // Create per-secret and per-template symlinks
    filesystem::create_secret_symlinks(&gen_dir, &manifest.symlink_path, &manifest.secrets, ignore_passwd)?;
    filesystem::create_template_symlinks(&gen_dir, &manifest.templates, ignore_passwd)?;

    // Prune old generations
    filesystem::prune_generations(&manifest.secrets_mount_point, gen_num, manifest.keep_generations)?;

    info!(
        secrets = manifest.secrets.len(),
        templates = manifest.templates.len(),
        generation = gen_num,
        "secrets installed"
    );

    Ok(())
}

/// Validate manifest structure without doing any I/O beyond parsing.
fn validate_manifest_structure(manifest: &Manifest) -> Result<()> {
    for secret in &manifest.secrets {
        // Validate mode is valid octal
        u32::from_str_radix(&secret.mode, 8)
            .with_context(|| format!("invalid mode '{}' for secret '{}'", secret.mode, secret.name))?;

        // Validate format is supported
        if !secret.format.is_supported() {
            anyhow::bail!("secret '{}': format '{}' is not supported; use yaml or json", secret.name, secret.format);
        }
    }

    for template in &manifest.templates {
        u32::from_str_radix(&template.mode, 8)
            .with_context(|| format!("invalid mode '{}' for template '{}'", template.mode, template.name))?;
    }

    Ok(())
}
