//! CLI tool for managing Aspen capability tokens.
//!
//! Provides commands for generating, delegating, verifying, and inspecting
//! capability tokens for the Aspen distributed system.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self};
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
#[cfg(test)]
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use aspen::auth::Audience;
use aspen::auth::Capability;
use aspen::auth::CapabilityToken;
use aspen::auth::TokenBuilder;
use aspen::auth::TokenVerifier;
use aspen::auth::constants::FEDERATION_PROXY_FACT_KEY;
use aspen::auth::constants::FEDERATION_PROXY_FACT_VALUE;
use aspen::auth::constants::MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS;
use aspen::auth::generate_root_token;
use clap::Parser;
use clap::Subcommand;
use iroh::PublicKey;
use iroh::SecretKey;

/// CLI tool for managing Aspen capability tokens.
#[derive(Parser, Debug)]
#[command(name = "aspen-token")]
#[command(about = "Manage Aspen capability tokens", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output JSON instead of human-readable format.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a root token with full cluster access.
    GenerateRoot {
        /// Ed25519 secret key to sign with (hex-encoded).
        /// If not provided, generates a new keypair.
        #[arg(long)]
        secret_key: Option<String>,

        /// Token lifetime (e.g., "1d", "7d", "365d").
        #[arg(long, default_value = "1d")]
        lifetime: String,

        /// Output file (default: stdout). Files are created/truncated with owner-only permissions.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Create a delegated token from a parent.
    Delegate {
        /// Parent token to delegate from (base64-encoded).
        #[arg(long)]
        parent_token: String,

        /// Issuer secret key for the new token (hex-encoded).
        #[arg(long)]
        parent_key: String,

        /// Capability to grant (repeatable).
        /// Format: read:prefix, write:prefix, delete:prefix, full:prefix, watch:prefix,
        /// cluster-admin, delegate
        #[arg(long, value_name = "CAP")]
        capability: Vec<String>,

        /// Token lifetime (e.g., "1d", "7d", "365d").
        #[arg(long)]
        lifetime: String,

        /// Optional audience public key (hex-encoded).
        /// If not set, creates a bearer token.
        #[arg(long)]
        audience: Option<String>,

        /// Mark this delegated bearer token as an explicit federation proxy token.
        ///
        /// Requires bearer audience, federation-pull/federation-push capabilities,
        /// and lifetime no greater than 15 minutes.
        #[arg(long)]
        federation_proxy: bool,

        /// Output file (default: stdout). Files are created/truncated with owner-only permissions.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Verify a token's signature and show its contents.
    Verify {
        /// Token to verify (base64-encoded).
        #[arg(long)]
        token: String,

        /// Trusted root public key (hex-encoded, repeatable).
        #[arg(long, value_name = "HEX")]
        trusted_root: Vec<String>,
    },

    /// Show token contents without verification.
    Inspect {
        /// Token to inspect (base64-encoded).
        #[arg(long)]
        token: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    debug_assert!(matches!(
        cli.command,
        Commands::GenerateRoot { .. } | Commands::Delegate { .. } | Commands::Verify { .. } | Commands::Inspect { .. }
    ));
    debug_assert!(!cli.json || std::env::args().any(|arg| arg == "--json"));

    match cli.command {
        Commands::GenerateRoot {
            secret_key,
            lifetime,
            output,
        } => generate_root_cmd(secret_key, &lifetime, output.as_deref(), cli.json),
        Commands::Delegate {
            parent_token,
            parent_key,
            capability,
            lifetime,
            audience,
            federation_proxy,
            output,
        } => delegate_cmd(DelegateCmdInput {
            parent_token_b64: &parent_token,
            parent_key_hex: &parent_key,
            capabilities: &capability,
            lifetime_str: &lifetime,
            audience_hex: audience.as_deref(),
            federation_proxy,
            output_path: output.as_deref(),
            is_json_output: cli.json,
        }),
        Commands::Verify { token, trusted_root } => verify_cmd(&token, &trusted_root, cli.json),
        Commands::Inspect { token } => inspect_cmd(&token, cli.json),
    }
}

fn generate_root_cmd(
    secret_key_hex: Option<String>,
    lifetime_str: &str,
    output_path: Option<&std::path::Path>,
    is_json_output: bool,
) -> Result<()> {
    debug_assert!(!lifetime_str.trim().is_empty());
    debug_assert!(secret_key_hex.as_ref().map(|hex| !hex.is_empty()).unwrap_or(true));

    let secret_key = if let Some(hex) = secret_key_hex {
        parse_secret_key(&hex)?
    } else {
        SecretKey::generate(&mut rand::rng())
    };

    let lifetime = parse_duration(lifetime_str)?;
    let token = generate_root_token(&secret_key, lifetime).context("failed to generate root token")?;
    let token_b64 = token.to_base64().context("failed to encode token")?;

    let output = if is_json_output {
        serde_json::json!({
            "token": token_b64,
            "issuer": secret_key.public().to_string(),
            "secret_key": hex::encode(secret_key.to_bytes()),
            "audience": format_audience(&token.audience),
            "expires_at": token.expires_at,
            "expires_at_iso": format_timestamp(token.expires_at),
        })
        .to_string()
    } else {
        format!(
            "Root Token Generated\n\
             ==================\n\
             Token: {}\n\
             Issuer Public Key: {}\n\
             Secret Key: {}\n\
             Audience: {}\n\
             Expires: {} ({})\n\
             \n\
             Keep the secret key secure! It can be used to create more tokens.",
            token_b64,
            secret_key.public(),
            hex::encode(secret_key.to_bytes()),
            format_audience(&token.audience),
            format_timestamp(token.expires_at),
            token.expires_at
        )
    };

    write_output(&output, output_path)?;
    Ok(())
}

struct DelegateCmdInput<'a> {
    parent_token_b64: &'a str,
    parent_key_hex: &'a str,
    capabilities: &'a [String],
    lifetime_str: &'a str,
    audience_hex: Option<&'a str>,
    federation_proxy: bool,
    output_path: Option<&'a std::path::Path>,
    is_json_output: bool,
}

fn delegate_cmd(args: DelegateCmdInput<'_>) -> Result<()> {
    debug_assert!(!args.parent_token_b64.is_empty());
    debug_assert!(!args.parent_key_hex.is_empty());

    let parent_token = CapabilityToken::from_base64(args.parent_token_b64).context("failed to decode parent token")?;
    let issuer_key = parse_secret_key(args.parent_key_hex)?;
    let lifetime = parse_duration(args.lifetime_str)?;
    if args.federation_proxy {
        anyhow::ensure!(
            args.audience_hex.is_none(),
            "federation proxy tokens must use bearer audience; do not pass --audience"
        );
        anyhow::ensure!(
            lifetime.as_secs() <= MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS,
            "federation proxy tokens must have lifetime no greater than {} seconds",
            MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS
        );
    }

    let mut builder = TokenBuilder::new(issuer_key.clone()).delegated_from(parent_token);

    for capability_str in args.capabilities {
        let capability = parse_capability(capability_str)?;
        if args.federation_proxy {
            anyhow::ensure!(
                is_federation_proxy_capability(&capability),
                "federation proxy tokens may only carry federation-pull or federation-push capabilities"
            );
        }
        builder = builder.with_capability(capability);
    }

    if let Some(audience_hex) = args.audience_hex {
        let audience_key = parse_public_key(audience_hex)?;
        builder = builder.for_key(audience_key);
    } else {
        builder = builder.for_audience(Audience::Bearer);
    }

    builder = builder.with_lifetime(lifetime).with_random_nonce();
    if args.federation_proxy {
        builder = builder.with_fact(FEDERATION_PROXY_FACT_KEY, FEDERATION_PROXY_FACT_VALUE);
    }
    let token = builder.build().context("failed to build delegated token")?;
    let token_b64 = token.to_base64().context("failed to encode token")?;

    let output = if args.is_json_output {
        serde_json::json!({
            "token": token_b64,
            "issuer": issuer_key.public().to_string(),
            "audience": format_audience(&token.audience),
            "capabilities": token.capabilities.iter().map(format_capability).collect::<Vec<_>>(),
            "federation_proxy": args.federation_proxy,
            "expires_at": token.expires_at,
            "expires_at_iso": format_timestamp(token.expires_at),
        })
        .to_string()
    } else {
        format!(
            "Delegated Token Created\n\
             ======================\n\
             Token: {}\n\
             Issuer: {}\n\
             Audience: {}\n\
             Capabilities:\n{}\n\
             Federation Proxy: {}\n\
             Expires: {} ({})",
            token_b64,
            issuer_key.public(),
            format_audience(&token.audience),
            token
                .capabilities
                .iter()
                .map(|capability| format!("  - {}", format_capability(capability)))
                .collect::<Vec<_>>()
                .join("\n"),
            if args.federation_proxy { "yes" } else { "no" },
            format_timestamp(token.expires_at),
            token.expires_at
        )
    };

    write_output(&output, args.output_path)?;
    Ok(())
}

fn verify_cmd(token_b64: &str, trusted_roots: &[String], is_json_output: bool) -> Result<()> {
    debug_assert!(!token_b64.is_empty());
    debug_assert!(trusted_roots.iter().all(|root| !root.is_empty()));

    let token = CapabilityToken::from_base64(token_b64).context("failed to decode token")?;
    let mut verifier = TokenVerifier::new();
    for root_hex in trusted_roots {
        let root_key = parse_public_key(root_hex)?;
        verifier = verifier.with_trusted_root(root_key);
    }

    let verification_result = verifier.verify(&token, None);
    let now_secs = current_unix_seconds()?;
    let is_valid = verification_result.is_ok();
    let error_msg = verification_result.err().map(|error| error.to_string());
    let output = if is_json_output {
        serde_json::json!({
            "valid": is_valid,
            "error": error_msg,
            "issuer": token.issuer.to_string(),
            "audience": format_audience(&token.audience),
            "capabilities": token.capabilities.iter().map(format_capability).collect::<Vec<_>>(),
            "issued_at": token.issued_at,
            "issued_at_iso": format_timestamp(token.issued_at),
            "expires_at": token.expires_at,
            "expires_at_iso": format_timestamp(token.expires_at),
            "expired": token.expires_at < now_secs,
            "has_nonce": token.nonce.is_some(),
            "has_proof": token.proof.is_some(),
        })
        .to_string()
    } else {
        let status = if is_valid { "✓ VALID" } else { "✗ INVALID" };
        let mut output = format!("Token Verification\n==================\nStatus: {}\n", status);
        if let Some(error_message) = error_msg {
            output.push_str(&format!("Error: {}\n", error_message));
        }
        output.push_str(&format!(
            "\nToken Details:\nIssuer: {}\nAudience: {}\nCapabilities:\n{}\nIssued: {} ({})\nExpires: {} ({})\nExpired: {}\nHas Nonce: {}\nHas Proof (delegated): {}",
            token.issuer,
            format_audience(&token.audience),
            token.capabilities.iter().map(|capability| format!("  - {}", format_capability(capability))).collect::<Vec<_>>().join("\n"),
            format_timestamp(token.issued_at), token.issued_at,
            format_timestamp(token.expires_at), token.expires_at,
            token.expires_at < now_secs,
            token.nonce.is_some(),
            token.proof.is_some(),
        ));
        output
    };

    write_output(&output, None)?;
    if !is_valid {
        std::process::exit(1);
    }
    Ok(())
}

fn inspect_cmd(token_b64: &str, is_json_output: bool) -> Result<()> {
    debug_assert!(!token_b64.is_empty());
    let token = CapabilityToken::from_base64(token_b64).context("failed to decode token")?;
    let now_secs = current_unix_seconds()?;
    debug_assert!(token.expires_at >= token.issued_at || token.version > 0);

    let output = if is_json_output {
        serde_json::json!({
            "version": token.version,
            "issuer": token.issuer.to_string(),
            "audience": format_audience(&token.audience),
            "capabilities": token.capabilities.iter().map(format_capability).collect::<Vec<_>>(),
            "issued_at": token.issued_at,
            "issued_at_iso": format_timestamp(token.issued_at),
            "expires_at": token.expires_at,
            "expires_at_iso": format_timestamp(token.expires_at),
            "expired": token.expires_at < now_secs,
            "nonce": token.nonce.map(hex::encode),
            "proof": token.proof.map(hex::encode),
            "signature": hex::encode(token.signature),
        })
        .to_string()
    } else {
        format!(
            "Token Contents (Unverified)\n\
             ===========================\n\
             Version: {}\n\
             Issuer: {}\n\
             Audience: {}\n\
             Capabilities:\n{}\n\
             Issued: {} ({})\n\
             Expires: {} ({})\n\
             Expired: {}\n\
             Nonce: {}\n\
             Proof (parent hash): {}\n\
             Signature: {}",
            token.version,
            token.issuer,
            format_audience(&token.audience),
            token
                .capabilities
                .iter()
                .map(|capability| format!("  - {}", format_capability(capability)))
                .collect::<Vec<_>>()
                .join("\n"),
            format_timestamp(token.issued_at),
            token.issued_at,
            format_timestamp(token.expires_at),
            token.expires_at,
            token.expires_at < now_secs,
            token.nonce.map_or("None".to_string(), hex::encode),
            token.proof.map_or("None (root token)".to_string(), hex::encode),
            hex::encode(token.signature)
        )
    };

    write_output(&output, None)?;
    Ok(())
}

// Helper functions

/// Parse a duration string like "1d", "7d", "365d", "1h", "30m".
#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "token CLI owns wall-clock boundary when checking expiry")]
fn current_unix_seconds() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH).context("system time before UNIX epoch")?.as_secs())
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    debug_assert!(!s.contains(char::is_whitespace));
    if s.is_empty() {
        anyhow::bail!("duration cannot be empty");
    }

    let unit_pos = s
        .chars()
        .position(|c| !c.is_ascii_digit())
        .context("duration must have a unit (e.g., '1d', '7h')")?;

    let value_str = &s[..unit_pos];
    let unit = &s[unit_pos..];
    let value: u64 = value_str.parse().context("duration value must be a positive integer")?;

    let seconds = match unit {
        "s" | "sec" | "secs" => value,
        "m" | "min" | "mins" => value.saturating_mul(60),
        "h" | "hr" | "hrs" | "hour" | "hours" => value.saturating_mul(3600),
        "d" | "day" | "days" => value.saturating_mul(86400),
        "w" | "week" | "weeks" => value.saturating_mul(604800),
        "y" | "year" | "years" => value.saturating_mul(31536000),
        _ => anyhow::bail!("unknown duration unit '{}'. Use: s, m, h, d, w, y", unit),
    };

    debug_assert!(seconds >= value || matches!(unit, "s" | "sec" | "secs"));
    Ok(Duration::from_secs(seconds))
}

/// Parse a hex-encoded secret key.
fn parse_secret_key(hex_str: &str) -> Result<SecretKey> {
    let bytes = hex::decode(hex_str).context("secret key must be hex-encoded")?;
    if bytes.len() != 32 {
        anyhow::bail!("secret key must be 32 bytes (64 hex characters)");
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);
    SecretKey::try_from(&key_bytes[..]).context("invalid secret key")
}

/// Parse a hex-encoded public key.
fn parse_public_key(hex_str: &str) -> Result<PublicKey> {
    let bytes = hex::decode(hex_str).context("public key must be hex-encoded")?;
    if bytes.len() != 32 {
        anyhow::bail!("public key must be 32 bytes (64 hex characters)");
    }
    PublicKey::try_from(&bytes[..]).context("invalid public key")
}

/// Parse a capability string like "read:prefix" or "cluster-admin".
///
/// Supported formats:
/// - KV operations: read:PREFIX, write:PREFIX, delete:PREFIX, full:PREFIX, watch:PREFIX
/// - Admin: cluster-admin, delegate
/// - Secrets: secrets-read:MOUNT:PREFIX, secrets-write:MOUNT:PREFIX, secrets-delete:MOUNT:PREFIX,
///   secrets-list:MOUNT:PREFIX, secrets-full:MOUNT:PREFIX, secrets-admin
/// - Transit: transit-encrypt:KEY_PREFIX, transit-decrypt:KEY_PREFIX, transit-sign:KEY_PREFIX,
///   transit-verify:KEY_PREFIX, transit-manage:KEY_PREFIX
/// - PKI: pki-issue:ROLE_PREFIX, pki-revoke, pki-read-ca, pki-manage
struct CapabilityRequirement<'a> {
    capability_name: &'a str,
    usage_suffix: &'a str,
}

fn required_capability_suffix<'a>(parts: &'a [&str], requirement: CapabilityRequirement<'_>) -> Result<&'a str> {
    if parts.len() == 2 {
        Ok(parts[1])
    } else {
        anyhow::bail!(
            "{} capability requires {}: {}:{}",
            requirement.capability_name,
            requirement.usage_suffix,
            requirement.capability_name,
            requirement.usage_suffix
        )
    }
}

fn required_mount_prefix<'a>(parts: &'a [&str], capability_name: &str) -> Result<(&'a str, &'a str)> {
    let rest = required_capability_suffix(parts, CapabilityRequirement {
        capability_name,
        usage_suffix: "MOUNT:PREFIX",
    })?;
    let sub_parts: Vec<&str> = rest.splitn(2, ':').collect();
    if sub_parts.len() == 2 {
        Ok((sub_parts[0], sub_parts[1]))
    } else {
        anyhow::bail!("{} capability requires mount:prefix: {}:MOUNT:PREFIX", capability_name, capability_name)
    }
}

fn parse_basic_capability(parts: &[&str]) -> Result<Option<Capability>> {
    Ok(match parts[0] {
        "read" => Some(Capability::Read {
            prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "read",
                usage_suffix: "PREFIX",
            })?
            .to_string(),
        }),
        "write" => Some(Capability::Write {
            prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "write",
                usage_suffix: "PREFIX",
            })?
            .to_string(),
        }),
        "delete" => Some(Capability::Delete {
            prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "delete",
                usage_suffix: "PREFIX",
            })?
            .to_string(),
        }),
        "full" => Some(Capability::Full {
            prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "full",
                usage_suffix: "PREFIX",
            })?
            .to_string(),
        }),
        "watch" => Some(Capability::Watch {
            prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "watch",
                usage_suffix: "PREFIX",
            })?
            .to_string(),
        }),
        "cluster-admin" => Some(Capability::ClusterAdmin),
        "delegate" => Some(Capability::Delegate),
        _ => None,
    })
}

fn parse_secrets_capability(parts: &[&str]) -> Result<Option<Capability>> {
    Ok(match parts[0] {
        "secrets-read" => {
            let (mount, prefix) = required_mount_prefix(parts, "secrets-read")?;
            Some(Capability::SecretsRead {
                mount: mount.to_string(),
                prefix: prefix.to_string(),
            })
        }
        "secrets-write" => {
            let (mount, prefix) = required_mount_prefix(parts, "secrets-write")?;
            Some(Capability::SecretsWrite {
                mount: mount.to_string(),
                prefix: prefix.to_string(),
            })
        }
        "secrets-delete" => {
            let (mount, prefix) = required_mount_prefix(parts, "secrets-delete")?;
            Some(Capability::SecretsDelete {
                mount: mount.to_string(),
                prefix: prefix.to_string(),
            })
        }
        "secrets-list" => {
            let (mount, prefix) = required_mount_prefix(parts, "secrets-list")?;
            Some(Capability::SecretsList {
                mount: mount.to_string(),
                prefix: prefix.to_string(),
            })
        }
        "secrets-full" => {
            let (mount, prefix) = required_mount_prefix(parts, "secrets-full")?;
            Some(Capability::SecretsFull {
                mount: mount.to_string(),
                prefix: prefix.to_string(),
            })
        }
        "secrets-admin" => Some(Capability::SecretsAdmin),
        _ => None,
    })
}

fn parse_transit_capability(parts: &[&str]) -> Result<Option<Capability>> {
    Ok(match parts[0] {
        "transit-encrypt" => Some(Capability::TransitEncrypt {
            key_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "transit-encrypt",
                usage_suffix: "KEY_PREFIX",
            })?
            .to_string(),
        }),
        "transit-decrypt" => Some(Capability::TransitDecrypt {
            key_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "transit-decrypt",
                usage_suffix: "KEY_PREFIX",
            })?
            .to_string(),
        }),
        "transit-sign" => Some(Capability::TransitSign {
            key_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "transit-sign",
                usage_suffix: "KEY_PREFIX",
            })?
            .to_string(),
        }),
        "transit-verify" => Some(Capability::TransitVerify {
            key_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "transit-verify",
                usage_suffix: "KEY_PREFIX",
            })?
            .to_string(),
        }),
        "transit-manage" => Some(Capability::TransitKeyManage {
            key_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "transit-manage",
                usage_suffix: "KEY_PREFIX",
            })?
            .to_string(),
        }),
        _ => None,
    })
}

fn parse_pki_capability(parts: &[&str]) -> Result<Option<Capability>> {
    Ok(match parts[0] {
        "pki-issue" => Some(Capability::PkiIssue {
            role_prefix: required_capability_suffix(parts, CapabilityRequirement {
                capability_name: "pki-issue",
                usage_suffix: "ROLE_PREFIX",
            })?
            .to_string(),
        }),
        "pki-revoke" => Some(Capability::PkiRevoke),
        "pki-read-ca" => Some(Capability::PkiReadCa),
        "pki-manage" => Some(Capability::PkiManage),
        _ => None,
    })
}

fn parse_federation_capability(parts: &[&str]) -> Option<Capability> {
    let repo_prefix = if parts.len() == 2 { parts[1] } else { "" }.to_string();
    match parts[0] {
        "federation-pull" => Some(Capability::FederationPull { repo_prefix }),
        "federation-push" => Some(Capability::FederationPush { repo_prefix }),
        _ => None,
    }
}

fn parse_snix_capability(parts: &[&str]) -> Option<Capability> {
    let resource_prefix = if parts.len() == 2 { parts[1] } else { "" }.to_string();
    match parts[0] {
        "snix-read" => Some(Capability::SnixRead { resource_prefix }),
        "snix-write" => Some(Capability::SnixWrite { resource_prefix }),
        _ => None,
    }
}

fn is_federation_proxy_capability(capability: &Capability) -> bool {
    matches!(capability, Capability::FederationPull { .. } | Capability::FederationPush { .. })
}

fn parse_capability(s: &str) -> Result<Capability> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    debug_assert!(!parts.is_empty());
    debug_assert!(!parts[0].is_empty());

    if let Some(capability) = parse_basic_capability(&parts)? {
        return Ok(capability);
    }
    if let Some(capability) = parse_secrets_capability(&parts)? {
        return Ok(capability);
    }
    if let Some(capability) = parse_transit_capability(&parts)? {
        return Ok(capability);
    }
    if let Some(capability) = parse_pki_capability(&parts)? {
        return Ok(capability);
    }
    if let Some(capability) = parse_federation_capability(&parts) {
        return Ok(capability);
    }
    if let Some(capability) = parse_snix_capability(&parts) {
        return Ok(capability);
    }

    anyhow::bail!(
        "unknown capability type '{}'. Use: read:PREFIX, write:PREFIX, delete:PREFIX, \
         full:PREFIX, watch:PREFIX, cluster-admin, delegate, secrets-*:MOUNT:PREFIX, \
         transit-*:KEY_PREFIX, pki-*, federation-pull[:PREFIX], federation-push[:PREFIX], \
         snix-read[:RESOURCE_PREFIX], snix-write[:RESOURCE_PREFIX]",
        parts[0]
    )
}

/// Format a capability for display.
fn format_capability(cap: &Capability) -> String {
    match cap {
        Capability::Read { prefix } => format!("read:{}", prefix),
        Capability::Write { prefix } => format!("write:{}", prefix),
        Capability::Delete { prefix } => format!("delete:{}", prefix),
        Capability::Full { prefix } => format!("full:{}", prefix),
        Capability::Watch { prefix } => format!("watch:{}", prefix),
        Capability::ClusterAdmin => "cluster-admin".to_string(),
        Capability::Delegate => "delegate".to_string(),
        Capability::ShellExecute {
            command_pattern,
            working_dir,
        } => match working_dir {
            Some(wd) => format!("shell:{}@{}", command_pattern, wd),
            None => format!("shell:{}", command_pattern),
        },
        // Secrets engine capabilities
        Capability::SecretsRead { mount, prefix } => format!("secrets-read:{}:{}", mount, prefix),
        Capability::SecretsWrite { mount, prefix } => format!("secrets-write:{}:{}", mount, prefix),
        Capability::SecretsDelete { mount, prefix } => format!("secrets-delete:{}:{}", mount, prefix),
        Capability::SecretsList { mount, prefix } => format!("secrets-list:{}:{}", mount, prefix),
        Capability::SecretsFull { mount, prefix } => format!("secrets-full:{}:{}", mount, prefix),
        // Transit engine capabilities
        Capability::TransitEncrypt { key_prefix } => format!("transit-encrypt:{}", key_prefix),
        Capability::TransitDecrypt { key_prefix } => format!("transit-decrypt:{}", key_prefix),
        Capability::TransitSign { key_prefix } => format!("transit-sign:{}", key_prefix),
        Capability::TransitVerify { key_prefix } => format!("transit-verify:{}", key_prefix),
        Capability::TransitKeyManage { key_prefix } => format!("transit-manage:{}", key_prefix),
        // PKI engine capabilities
        Capability::PkiIssue { role_prefix } => format!("pki-issue:{}", role_prefix),
        Capability::PkiRevoke => "pki-revoke".to_string(),
        Capability::PkiReadCa => "pki-read-ca".to_string(),
        Capability::PkiManage => "pki-manage".to_string(),
        // Secrets admin
        Capability::SecretsAdmin => "secrets-admin".to_string(),
        // Net service mesh capabilities
        Capability::NetConnect { service_prefix } => format!("net-connect:{}", service_prefix),
        Capability::NetPublish { service_prefix } => format!("net-publish:{}", service_prefix),
        Capability::NetAdmin => "net-admin".to_string(),
        // Federation sync capabilities
        Capability::FederationPull { repo_prefix } => format!("federation-pull:{}", repo_prefix),
        Capability::FederationPush { repo_prefix } => format!("federation-push:{}", repo_prefix),
        // SNIX store capabilities
        Capability::SnixRead { resource_prefix } => format!("snix-read:{}", resource_prefix),
        Capability::SnixWrite { resource_prefix } => format!("snix-write:{}", resource_prefix),
    }
}

/// Format an audience for display.
fn format_audience(aud: &Audience) -> String {
    match aud {
        Audience::Key(key) => format!("Key({})", key),
        Audience::Bearer => "Bearer (anyone)".to_string(),
    }
}

/// Format a Unix timestamp as ISO 8601.
fn format_timestamp(unix_secs: u64) -> String {
    use chrono::DateTime;
    use chrono::Utc;

    let capped_unix_secs = unix_secs.min(i64::MAX as u64);
    let unix_secs_i64 = i64::try_from(capped_unix_secs).unwrap_or(i64::MAX);
    let dt = DateTime::<Utc>::from_timestamp(unix_secs_i64, 0).unwrap_or(DateTime::UNIX_EPOCH);
    dt.to_rfc3339()
}

fn validate_sensitive_output_file(file: &fs::File) -> Result<()> {
    let metadata = file.metadata().context("failed to inspect output file")?;
    anyhow::ensure!(metadata.file_type().is_file(), "token output path is not a regular file");
    let euid = unsafe { libc::geteuid() };
    anyhow::ensure!(metadata.uid() == euid, "token output file is not owned by the current user");
    Ok(())
}

/// Write sensitive token output to a file with owner-only permissions or stdout.
fn write_output(content: &str, path: Option<&std::path::Path>) -> Result<()> {
    if let Some(p) = path {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(p)
            .context("failed to open output file")?;
        validate_sensitive_output_file(&file).context("refusing unsafe token output file")?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .context("failed to restrict output file permissions before writing")?;
        file.write_all(content.as_bytes()).context("failed to write output file")?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .context("failed to restrict output file permissions")?;
    } else {
        io::stdout().write_all(content.as_bytes()).context("failed to write to stdout")?;
        io::stdout().write_all(b"\n").context("failed to write newline to stdout")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_output_files_are_owner_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output_path = dir.path().join("delegated-token.txt");

        write_output("sensitive", Some(&output_path)).expect("write token output");

        let mode = fs::metadata(&output_path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn token_output_restricts_existing_files_before_rewrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output_path = dir.path().join("existing-token.txt");
        fs::write(&output_path, "old").expect("seed token output");
        fs::set_permissions(&output_path, fs::Permissions::from_mode(0o644)).expect("set permissive mode");

        write_output("new-sensitive", Some(&output_path)).expect("rewrite token output");

        let mode = fs::metadata(&output_path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(fs::read_to_string(&output_path).expect("read token output"), "new-sensitive");
    }

    #[test]
    fn token_output_rejects_symlink_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target_path = dir.path().join("target-token.txt");
        let link_path = dir.path().join("linked-token.txt");
        fs::write(&target_path, "old-target").expect("seed symlink target");
        symlink(&target_path, &link_path).expect("create symlink");

        assert!(write_output("new-sensitive", Some(&link_path)).is_err());
        assert_eq!(fs::read_to_string(&target_path).expect("read symlink target"), "old-target");
    }

    #[test]
    fn token_output_rejects_non_regular_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fifo_path = dir.path().join("token-fifo");
        let fifo_c = std::ffi::CString::new(fifo_path.as_os_str().as_encoded_bytes()).expect("fifo path");
        let rc = unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) };
        assert_eq!(rc, 0, "mkfifo failed");

        assert!(write_output("new-sensitive", Some(&fifo_path)).is_err());
    }

    #[test]
    fn generate_root_output_reports_bearer_audience_in_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output_path = dir.path().join("root-token.json");

        generate_root_cmd(None, "1h", Some(&output_path), true).expect("generate root json");

        let output = fs::read_to_string(output_path).expect("read root json");
        let value: serde_json::Value = serde_json::from_str(&output).expect("parse root json");
        assert_eq!(value["audience"], "Bearer (anyone)");
    }

    #[test]
    fn parse_snix_capabilities() {
        assert_eq!(parse_capability("snix-read:dir:").expect("snix read should parse"), Capability::SnixRead {
            resource_prefix: "dir:".to_string(),
        });
        assert_eq!(parse_capability("snix-write:pathinfo:").expect("snix write should parse"), Capability::SnixWrite {
            resource_prefix: "pathinfo:".to_string(),
        });
    }

    #[test]
    fn delegate_federation_proxy_marks_short_lived_bearer_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output_path = dir.path().join("proxy-token.json");
        let issuer_key = SecretKey::generate(&mut rand::rng());
        let parent = TokenBuilder::new(issuer_key.clone())
            .with_capability(Capability::FederationPull {
                repo_prefix: "forge:".to_string(),
            })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .with_random_nonce()
            .build()
            .expect("parent token");

        delegate_cmd(DelegateCmdInput {
            parent_token_b64: &parent.to_base64().expect("parent b64"),
            parent_key_hex: &hex::encode(issuer_key.to_bytes()),
            capabilities: &["federation-pull:forge:".to_string()],
            lifetime_str: "15m",
            audience_hex: None,
            federation_proxy: true,
            output_path: Some(&output_path),
            is_json_output: true,
        })
        .expect("delegate proxy token");

        let output = fs::read_to_string(output_path).expect("read proxy json");
        let value: serde_json::Value = serde_json::from_str(&output).expect("parse proxy json");
        assert_eq!(value["federation_proxy"], true);
        let token = CapabilityToken::from_base64(value["token"].as_str().expect("token string")).expect("decode token");
        assert!(matches!(token.audience, Audience::Bearer));
        assert!(token.proof.is_some());
        assert_eq!(token.delegation_depth, parent.delegation_depth + 1);
        assert!(token.expires_at.saturating_sub(token.issued_at) <= MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS);
        assert!(token
            .facts
            .iter()
            .any(|(key, value)| key == FEDERATION_PROXY_FACT_KEY && value.as_slice() == FEDERATION_PROXY_FACT_VALUE));
    }

    #[test]
    fn delegate_federation_proxy_rejects_explicit_audience() {
        let issuer_key = SecretKey::generate(&mut rand::rng());
        let parent = TokenBuilder::new(issuer_key.clone())
            .with_capability(Capability::FederationPull {
                repo_prefix: "forge:".to_string(),
            })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .with_random_nonce()
            .build()
            .expect("parent token");

        let audience_key = SecretKey::generate(&mut rand::rng()).public().to_string();
        let result = delegate_cmd(DelegateCmdInput {
            parent_token_b64: &parent.to_base64().expect("parent b64"),
            parent_key_hex: &hex::encode(issuer_key.to_bytes()),
            capabilities: &["federation-pull:forge:".to_string()],
            lifetime_str: "15m",
            audience_hex: Some(&audience_key),
            federation_proxy: true,
            output_path: None,
            is_json_output: true,
        });

        assert!(result.unwrap_err().to_string().contains("must use bearer audience"));
    }

    #[test]
    fn delegate_federation_proxy_rejects_non_federation_capability() {
        let issuer_key = SecretKey::generate(&mut rand::rng());
        let parent = TokenBuilder::new(issuer_key.clone())
            .with_capability(Capability::Full { prefix: "".to_string() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .with_random_nonce()
            .build()
            .expect("parent token");

        let result = delegate_cmd(DelegateCmdInput {
            parent_token_b64: &parent.to_base64().expect("parent b64"),
            parent_key_hex: &hex::encode(issuer_key.to_bytes()),
            capabilities: &["full:".to_string()],
            lifetime_str: "15m",
            audience_hex: None,
            federation_proxy: true,
            output_path: None,
            is_json_output: true,
        });

        assert!(result.unwrap_err().to_string().contains("may only carry federation-pull or federation-push"));
    }

    #[test]
    fn delegate_federation_proxy_rejects_long_lifetime() {
        let issuer_key = SecretKey::generate(&mut rand::rng());
        let parent = TokenBuilder::new(issuer_key.clone())
            .with_capability(Capability::FederationPull {
                repo_prefix: "forge:".to_string(),
            })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .with_random_nonce()
            .build()
            .expect("parent token");

        let result = delegate_cmd(DelegateCmdInput {
            parent_token_b64: &parent.to_base64().expect("parent b64"),
            parent_key_hex: &hex::encode(issuer_key.to_bytes()),
            capabilities: &["federation-pull:forge:".to_string()],
            lifetime_str: "16m",
            audience_hex: None,
            federation_proxy: true,
            output_path: None,
            is_json_output: true,
        });

        assert!(result.unwrap_err().to_string().contains("lifetime no greater than"));
    }

    #[test]
    fn generate_root_output_reports_bearer_audience_for_humans() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output_path = dir.path().join("root-token.txt");

        generate_root_cmd(None, "1h", Some(&output_path), false).expect("generate root text");

        let output = fs::read_to_string(output_path).expect("read root text");
        assert!(output.contains("Audience: Bearer (anyone)"));
    }
}
