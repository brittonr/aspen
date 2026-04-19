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

fn main() {
    if let Err(error) = run_main() {
        eprintln!("sops-install-secrets: {error:#}");
        process::exit(1);
    }
}

fn run_main() -> Result<()> {
    let cli = Cli::parse_from(normalize_args());
    init_tracing();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    runtime.block_on(run(cli))
}

fn normalize_args() -> Vec<String> {
    std::env::args()
        .map(|arg| {
            if let Some(rest) = arg.strip_prefix("-check-mode") {
                format!("--check-mode{rest}")
            } else if let Some(rest) = arg.strip_prefix("-ignore-passwd") {
                format!("--ignore-passwd{rest}")
            } else {
                arg
            }
        })
        .collect()
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
}

async fn run(cli: Cli) -> Result<()> {
    let manifest = load_manifest(&cli.manifest)?;
    debug!(secrets = manifest.secrets.len(), templates = manifest.templates.len(), "manifest parsed");

    if handle_check_mode(&manifest, &cli.check_mode)? {
        return Ok(());
    }

    install_manifest(&manifest, cli.ignore_passwd).await
}

fn load_manifest(manifest_path: &str) -> Result<Manifest> {
    let manifest_contents =
        fs::read_to_string(manifest_path).with_context(|| format!("failed to open manifest '{}'", manifest_path))?;
    serde_json::from_str(&manifest_contents).with_context(|| "failed to parse manifest JSON")
}

fn handle_check_mode(manifest: &Manifest, check_mode: &str) -> Result<bool> {
    if check_mode == "manifest" {
        validate_manifest_structure(manifest)?;
        return Ok(true);
    }
    if check_mode == "sopsfile" {
        validate_manifest_structure(manifest)?;
        for secret in &manifest.secrets {
            decrypt::validate_sops_file(secret)?;
        }
        return Ok(true);
    }
    Ok(false)
}

async fn install_manifest(manifest: &Manifest, should_ignore_passwd: bool) -> Result<()> {
    let is_dry = std::env::var("NIXOS_ACTION").ok().is_some_and(|value| value == "dry-activate");
    let install = InstallGenerationOptions {
        keys_gid: lookup_keys_gid(should_ignore_passwd || is_dry)?,
        should_ignore_passwd: should_ignore_passwd || is_dry,
        is_dry,
    };

    filesystem::mount_secrets_fs(&manifest.secrets_mount_point, install.keys_gid, manifest.use_tmpfs)?;
    let age_key_path = age_keys::import_age_keys(manifest, &manifest.secrets_mount_point, manifest.logging.key_import)?;
    let decrypted = decrypt::decrypt_secrets(manifest, age_key_path.as_deref()).await?;
    let rendered_templates = render_templates(manifest, &decrypted)?;
    install_generation(manifest, &decrypted, &rendered_templates, install)?;
    Ok(())
}

fn lookup_keys_gid(should_ignore_passwd: bool) -> Result<u32> {
    if should_ignore_passwd {
        return Ok(0);
    }
    filesystem::lookup_keys_gid()
}

fn render_templates(
    manifest: &Manifest,
    decrypted: &std::collections::HashMap<String, Vec<u8>>,
) -> Result<Vec<String>> {
    let mut rendered_templates = Vec::with_capacity(manifest.templates.len());
    for template in &manifest.templates {
        let source = templates::load_template_source(template)?;
        let rendered = templates::render_template(&source, &manifest.placeholder_by_secret_name, decrypted);
        rendered_templates.push(rendered);
    }
    Ok(rendered_templates)
}

struct InstallGenerationOptions {
    keys_gid: u32,
    should_ignore_passwd: bool,
    is_dry: bool,
}

fn install_generation(
    manifest: &Manifest,
    decrypted: &std::collections::HashMap<String, Vec<u8>>,
    rendered_templates: &[String],
    install: InstallGenerationOptions,
) -> Result<()> {
    let (gen_dir, gen_num) = filesystem::prepare_generation(
        filesystem::GenerationPaths {
            mount_point: &manifest.secrets_mount_point,
            symlink_path: &manifest.symlink_path,
        },
        install.keys_gid,
    )?;

    write_generation_contents(manifest, decrypted, rendered_templates, &gen_dir, &install)?;
    finalize_generation(manifest, &gen_dir, gen_num, install)?;
    Ok(())
}

fn write_generation_contents(
    manifest: &Manifest,
    decrypted: &std::collections::HashMap<String, Vec<u8>>,
    rendered_templates: &[String],
    gen_dir: &std::path::Path,
    install: &InstallGenerationOptions,
) -> Result<()> {
    for secret in &manifest.secrets {
        let value = decrypted
            .get(&secret.name)
            .with_context(|| format!("no decrypted value for secret '{}'", secret.name))?;
        filesystem::write_secret(gen_dir, secret, value, install.keys_gid, install.should_ignore_passwd)?;
    }
    for (template, rendered) in manifest.templates.iter().zip(rendered_templates.iter()) {
        filesystem::write_template(gen_dir, template, rendered, install.keys_gid, install.should_ignore_passwd)?;
    }
    Ok(())
}

fn finalize_generation(
    manifest: &Manifest,
    gen_dir: &std::path::Path,
    gen_num: u64,
    install: InstallGenerationOptions,
) -> Result<()> {
    debug_assert!(!manifest.symlink_path.is_empty());
    debug_assert!(gen_num >= 1);
    filesystem::detect_changes(filesystem::ChangeDetection {
        symlink_path: &manifest.symlink_path,
        gen_dir,
        secrets: &manifest.secrets,
        templates: &manifest.templates,
        is_dry: install.is_dry,
        should_log_changes: manifest.logging.secret_changes,
    })?;
    if install.is_dry {
        return Ok(());
    }

    filesystem::atomic_symlink(gen_dir, &manifest.symlink_path)?;
    filesystem::create_secret_symlinks(
        gen_dir,
        &manifest.symlink_path,
        &manifest.secrets,
        install.should_ignore_passwd,
    )?;
    filesystem::create_template_symlinks(gen_dir, &manifest.templates, install.should_ignore_passwd)?;
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
