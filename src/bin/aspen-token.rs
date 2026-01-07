//! CLI tool for managing Aspen capability tokens.
//!
//! Provides commands for generating, delegating, verifying, and inspecting
//! capability tokens for the Aspen distributed system.

use std::fs;
use std::io::Write;
use std::io::{self};
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

        /// Output file (default: stdout).
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

        /// Output file (default: stdout).
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
            output,
        } => delegate_cmd(
            &parent_token,
            &parent_key,
            &capability,
            &lifetime,
            audience.as_deref(),
            output.as_deref(),
            cli.json,
        ),
        Commands::Verify { token, trusted_root } => verify_cmd(&token, &trusted_root, cli.json),
        Commands::Inspect { token } => inspect_cmd(&token, cli.json),
    }
}

fn generate_root_cmd(
    secret_key_hex: Option<String>,
    lifetime_str: &str,
    output_path: Option<&std::path::Path>,
    json: bool,
) -> Result<()> {
    let secret_key = if let Some(hex) = secret_key_hex {
        parse_secret_key(&hex)?
    } else {
        SecretKey::generate(&mut rand::rng())
    };

    let lifetime = parse_duration(lifetime_str)?;
    let token = generate_root_token(&secret_key, lifetime).context("failed to generate root token")?;

    let token_b64 = token.to_base64().context("failed to encode token")?;

    let output = if json {
        serde_json::json!({
            "token": token_b64,
            "issuer": secret_key.public().to_string(),
            "secret_key": hex::encode(secret_key.to_bytes()),
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
             Expires: {} ({})\n\
             \n\
             Keep the secret key secure! It can be used to create more tokens.",
            token_b64,
            secret_key.public(),
            hex::encode(secret_key.to_bytes()),
            format_timestamp(token.expires_at),
            token.expires_at
        )
    };

    write_output(&output, output_path)?;
    Ok(())
}

fn delegate_cmd(
    parent_token_b64: &str,
    parent_key_hex: &str,
    capabilities: &[String],
    lifetime_str: &str,
    audience_hex: Option<&str>,
    output_path: Option<&std::path::Path>,
    json: bool,
) -> Result<()> {
    let parent_token = CapabilityToken::from_base64(parent_token_b64).context("failed to decode parent token")?;
    let issuer_key = parse_secret_key(parent_key_hex)?;
    let lifetime = parse_duration(lifetime_str)?;

    let mut builder = TokenBuilder::new(issuer_key.clone()).delegated_from(parent_token);

    // Parse and add capabilities
    for cap_str in capabilities {
        let cap = parse_capability(cap_str)?;
        builder = builder.with_capability(cap);
    }

    // Set audience
    if let Some(aud_hex) = audience_hex {
        let aud_key = parse_public_key(aud_hex)?;
        builder = builder.for_key(aud_key);
    } else {
        builder = builder.for_audience(Audience::Bearer);
    }

    builder = builder.with_lifetime(lifetime).with_random_nonce();

    let token = builder.build().context("failed to build delegated token")?;
    let token_b64 = token.to_base64().context("failed to encode token")?;

    let output = if json {
        serde_json::json!({
            "token": token_b64,
            "issuer": issuer_key.public().to_string(),
            "audience": format_audience(&token.audience),
            "capabilities": token.capabilities.iter().map(format_capability).collect::<Vec<_>>(),
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
             Expires: {} ({})",
            token_b64,
            issuer_key.public(),
            format_audience(&token.audience),
            token
                .capabilities
                .iter()
                .map(|c| format!("  - {}", format_capability(c)))
                .collect::<Vec<_>>()
                .join("\n"),
            format_timestamp(token.expires_at),
            token.expires_at
        )
    };

    write_output(&output, output_path)?;
    Ok(())
}

fn verify_cmd(token_b64: &str, trusted_roots: &[String], json: bool) -> Result<()> {
    let token = CapabilityToken::from_base64(token_b64).context("failed to decode token")?;

    let mut verifier = TokenVerifier::new();
    for root_hex in trusted_roots {
        let root_key = parse_public_key(root_hex)?;
        verifier = verifier.with_trusted_root(root_key);
    }

    let verification_result = verifier.verify(&token, None);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before UNIX epoch")?.as_secs();

    let is_valid = verification_result.is_ok();
    let error_msg = verification_result.err().map(|e| e.to_string());

    let output = if json {
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
            "expired": token.expires_at < now,
            "has_nonce": token.nonce.is_some(),
            "has_proof": token.proof.is_some(),
        })
        .to_string()
    } else {
        let status = if is_valid { "✓ VALID" } else { "✗ INVALID" };

        let mut output = format!(
            "Token Verification\n\
             ==================\n\
             Status: {}\n",
            status
        );

        if let Some(err) = error_msg {
            output.push_str(&format!("Error: {}\n", err));
        }

        output.push_str(&format!(
            "\nToken Details:\n\
             Issuer: {}\n\
             Audience: {}\n\
             Capabilities:\n{}\n\
             Issued: {} ({})\n\
             Expires: {} ({})\n\
             Expired: {}\n\
             Has Nonce: {}\n\
             Has Proof (delegated): {}",
            token.issuer,
            format_audience(&token.audience),
            token
                .capabilities
                .iter()
                .map(|c| format!("  - {}", format_capability(c)))
                .collect::<Vec<_>>()
                .join("\n"),
            format_timestamp(token.issued_at),
            token.issued_at,
            format_timestamp(token.expires_at),
            token.expires_at,
            token.expires_at < now,
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

fn inspect_cmd(token_b64: &str, json: bool) -> Result<()> {
    let token = CapabilityToken::from_base64(token_b64).context("failed to decode token")?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before UNIX epoch")?.as_secs();

    let output = if json {
        serde_json::json!({
            "version": token.version,
            "issuer": token.issuer.to_string(),
            "audience": format_audience(&token.audience),
            "capabilities": token.capabilities.iter().map(format_capability).collect::<Vec<_>>(),
            "issued_at": token.issued_at,
            "issued_at_iso": format_timestamp(token.issued_at),
            "expires_at": token.expires_at,
            "expires_at_iso": format_timestamp(token.expires_at),
            "expired": token.expires_at < now,
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
                .map(|c| format!("  - {}", format_capability(c)))
                .collect::<Vec<_>>()
                .join("\n"),
            format_timestamp(token.issued_at),
            token.issued_at,
            format_timestamp(token.expires_at),
            token.expires_at,
            token.expires_at < now,
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
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
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
        "m" | "min" | "mins" => value * 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => value * 3600,
        "d" | "day" | "days" => value * 86400,
        "w" | "week" | "weeks" => value * 604800,
        "y" | "year" | "years" => value * 31536000,
        _ => anyhow::bail!("unknown duration unit '{}'. Use: s, m, h, d, w, y", unit),
    };

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
fn parse_capability(s: &str) -> Result<Capability> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();

    match parts[0] {
        "read" => {
            if parts.len() != 2 {
                anyhow::bail!("read capability requires prefix: read:PREFIX");
            }
            Ok(Capability::Read {
                prefix: parts[1].to_string(),
            })
        }
        "write" => {
            if parts.len() != 2 {
                anyhow::bail!("write capability requires prefix: write:PREFIX");
            }
            Ok(Capability::Write {
                prefix: parts[1].to_string(),
            })
        }
        "delete" => {
            if parts.len() != 2 {
                anyhow::bail!("delete capability requires prefix: delete:PREFIX");
            }
            Ok(Capability::Delete {
                prefix: parts[1].to_string(),
            })
        }
        "full" => {
            if parts.len() != 2 {
                anyhow::bail!("full capability requires prefix: full:PREFIX");
            }
            Ok(Capability::Full {
                prefix: parts[1].to_string(),
            })
        }
        "watch" => {
            if parts.len() != 2 {
                anyhow::bail!("watch capability requires prefix: watch:PREFIX");
            }
            Ok(Capability::Watch {
                prefix: parts[1].to_string(),
            })
        }
        "cluster-admin" => Ok(Capability::ClusterAdmin),
        "delegate" => Ok(Capability::Delegate),
        _ => anyhow::bail!(
            "unknown capability type '{}'. Use: read:PREFIX, write:PREFIX, delete:PREFIX, full:PREFIX, watch:PREFIX, cluster-admin, delegate",
            parts[0]
        ),
    }
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
    let dt = DateTime::<Utc>::from_timestamp(unix_secs as i64, 0).unwrap_or(DateTime::UNIX_EPOCH);
    dt.to_rfc3339()
}

/// Write output to a file or stdout.
fn write_output(content: &str, path: Option<&std::path::Path>) -> Result<()> {
    if let Some(p) = path {
        fs::write(p, content).context("failed to write output file")?;
    } else {
        io::stdout().write_all(content.as_bytes()).context("failed to write to stdout")?;
        io::stdout().write_all(b"\n").context("failed to write newline to stdout")?;
    }
    Ok(())
}
