//! Token management commands for capability-based authorization.
//!
//! All token commands are offline — no cluster connection required.
//! Tokens are created, inspected, and delegated using local cryptographic operations.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_auth::Capability;
use aspen_auth::CapabilityToken;
use aspen_auth::TokenBuilder;
use aspen_auth::generate_root_token;
use clap::Args;
use clap::Subcommand;
use iroh::SecretKey;

use crate::output::Outputable;
use crate::output::print_output;

// =============================================================================
// Command definitions
// =============================================================================

/// Capability token management (offline — no cluster connection needed).
#[derive(Subcommand)]
pub enum TokenCommand {
    /// Generate a new capability token.
    ///
    /// Creates a root token signed by the provided secret key.
    /// If no capabilities are specified, generates a full root token
    /// with Full, ClusterAdmin, and Delegate capabilities.
    Generate(GenerateArgs),

    /// Inspect a capability token.
    ///
    /// Decodes a base64-encoded token and displays its metadata
    /// including issuer, capabilities, expiry, and delegation depth.
    Inspect(InspectArgs),

    /// Delegate a child token from a parent.
    ///
    /// Creates a new token with reduced capabilities derived from
    /// a parent token. The parent must have the Delegate capability.
    Delegate(DelegateArgs),
}

#[derive(Args)]
pub struct GenerateArgs {
    /// Path to Ed25519 secret key file.
    ///
    /// The file should contain a hex-encoded 32-byte Ed25519 secret key.
    #[arg(long, env = "ASPEN_SECRET_KEY_FILE")]
    pub secret_key_file: Option<PathBuf>,

    /// Token lifetime (e.g., 365d, 24h, 30m, 3600s, or bare seconds).
    #[arg(long)]
    pub lifetime: String,

    /// Capabilities to grant. Can be repeated.
    ///
    /// Format: type:prefix (e.g., full:, read:logs/, write:data/, cluster-admin, delegate).
    /// If omitted, generates a full root token.
    #[arg(long = "cap")]
    pub capabilities: Vec<String>,
}

#[derive(Args)]
pub struct InspectArgs {
    /// Base64-encoded capability token to inspect.
    pub token: String,
}

#[derive(Args)]
pub struct DelegateArgs {
    /// Base64-encoded parent token to delegate from.
    #[arg(long)]
    pub parent: String,

    /// Path to Ed25519 secret key file.
    #[arg(long, env = "ASPEN_SECRET_KEY_FILE")]
    pub secret_key_file: Option<PathBuf>,

    /// Child token lifetime (e.g., 24h, 30m, 3600s).
    #[arg(long)]
    pub lifetime: String,

    /// Capabilities for the child token. Can be repeated.
    ///
    /// Must be subsets of the parent's capabilities.
    #[arg(long = "cap")]
    pub capabilities: Vec<String>,
}

// =============================================================================
// Parsing utilities
// =============================================================================

/// Parse a human-readable duration string into a `Duration`.
///
/// Supports suffixes: `d` (days), `h` (hours), `m` (minutes), `s` (seconds).
/// Bare numbers are treated as seconds.
///
/// # Examples
///
/// - `365d` → 365 days
/// - `24h` → 24 hours
/// - `30m` → 30 minutes
/// - `3600s` → 3600 seconds
/// - `3600` → 3600 seconds
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("empty duration string");
    }

    if let Some(num) = s.strip_suffix('d') {
        let days: u64 = num.parse().context("invalid number before 'd' suffix")?;
        Ok(Duration::from_secs(days.checked_mul(86400).context("duration overflow")?))
    } else if let Some(num) = s.strip_suffix('h') {
        let hours: u64 = num.parse().context("invalid number before 'h' suffix")?;
        Ok(Duration::from_secs(hours.checked_mul(3600).context("duration overflow")?))
    } else if let Some(num) = s.strip_suffix('m') {
        let minutes: u64 = num.parse().context("invalid number before 'm' suffix")?;
        Ok(Duration::from_secs(minutes.checked_mul(60).context("duration overflow")?))
    } else if let Some(num) = s.strip_suffix('s') {
        let secs: u64 = num.parse().context("invalid number before 's' suffix")?;
        Ok(Duration::from_secs(secs))
    } else {
        // Bare number = seconds
        let secs: u64 = s.parse().context(
            "invalid duration format. Use a number with optional suffix: 365d, 24h, 30m, 3600s, or bare seconds",
        )?;
        Ok(Duration::from_secs(secs))
    }
}

/// Parse a capability specification string into a `Capability`.
///
/// # Formats
///
/// - `full:` or `full:prefix` → `Full { prefix }`
/// - `read:prefix` → `Read { prefix }`
/// - `write:prefix` → `Write { prefix }`
/// - `delete:prefix` → `Delete { prefix }`
/// - `watch:prefix` → `Watch { prefix }`
/// - `cluster-admin` → `ClusterAdmin`
/// - `delegate` → `Delegate`
/// - `shell:pattern` → `ShellExecute { command_pattern, working_dir: None }`
/// - `secrets-read:mount:prefix` → `SecretsRead { mount, prefix }`
/// - `secrets-write:mount:prefix` → `SecretsWrite { mount, prefix }`
/// - `secrets-delete:mount:prefix` → `SecretsDelete { mount, prefix }`
/// - `secrets-list:mount:prefix` → `SecretsList { mount, prefix }`
/// - `secrets-full:mount:prefix` → `SecretsFull { mount, prefix }`
/// - `secrets-admin` → `SecretsAdmin`
pub fn parse_capability(s: &str) -> Result<Capability> {
    let s = s.trim();

    // No-argument capabilities
    match s {
        "cluster-admin" => return Ok(Capability::ClusterAdmin),
        "delegate" => return Ok(Capability::Delegate),
        "secrets-admin" => return Ok(Capability::SecretsAdmin),
        _ => {}
    }

    // type:value capabilities
    let (cap_type, value) = s.split_once(':').ok_or_else(|| {
        anyhow::anyhow!(
            "invalid capability format '{s}'. Expected type:prefix (e.g., full:, read:logs/, cluster-admin, delegate)"
        )
    })?;

    match cap_type {
        "full" => Ok(Capability::Full {
            prefix: value.to_string(),
        }),
        "read" => Ok(Capability::Read {
            prefix: value.to_string(),
        }),
        "write" => Ok(Capability::Write {
            prefix: value.to_string(),
        }),
        "delete" => Ok(Capability::Delete {
            prefix: value.to_string(),
        }),
        "watch" => Ok(Capability::Watch {
            prefix: value.to_string(),
        }),
        "shell" => Ok(Capability::ShellExecute {
            command_pattern: value.to_string(),
            working_dir: None,
        }),
        "secrets-read" => {
            let (mount, prefix) = parse_mount_prefix(value)?;
            Ok(Capability::SecretsRead { mount, prefix })
        }
        "secrets-write" => {
            let (mount, prefix) = parse_mount_prefix(value)?;
            Ok(Capability::SecretsWrite { mount, prefix })
        }
        "secrets-delete" => {
            let (mount, prefix) = parse_mount_prefix(value)?;
            Ok(Capability::SecretsDelete { mount, prefix })
        }
        "secrets-list" => {
            let (mount, prefix) = parse_mount_prefix(value)?;
            Ok(Capability::SecretsList { mount, prefix })
        }
        "secrets-full" => {
            let (mount, prefix) = parse_mount_prefix(value)?;
            Ok(Capability::SecretsFull { mount, prefix })
        }
        other => anyhow::bail!(
            "unknown capability type '{other}'. Valid types: full, read, write, delete, watch, \
             cluster-admin, delegate, shell, secrets-read, secrets-write, secrets-delete, \
             secrets-list, secrets-full, secrets-admin"
        ),
    }
}

/// Parse `mount:prefix` from a secrets capability value.
fn parse_mount_prefix(value: &str) -> Result<(String, String)> {
    let (mount, prefix) = value
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("secrets capability requires mount:prefix format (e.g., secret/:app/)"))?;
    Ok((mount.to_string(), prefix.to_string()))
}

/// Load a secret key from file path or `ASPEN_SECRET_KEY` environment variable.
///
/// The file should contain a hex-encoded 32-byte Ed25519 secret key (64 hex characters).
/// The environment variable should also be hex-encoded.
pub fn load_secret_key(file_path: Option<&PathBuf>) -> Result<SecretKey> {
    if let Some(path) = file_path {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read secret key file: {}", path.display()))?;
        parse_hex_secret_key(contents.trim()).with_context(|| format!("invalid secret key in file: {}", path.display()))
    } else if let Ok(hex_key) = std::env::var("ASPEN_SECRET_KEY") {
        parse_hex_secret_key(hex_key.trim()).context("invalid ASPEN_SECRET_KEY environment variable")
    } else {
        anyhow::bail!(
            "secret key required. Provide --secret-key-file <path> or set ASPEN_SECRET_KEY environment variable (hex-encoded)"
        )
    }
}

/// Parse a hex-encoded Ed25519 secret key.
fn parse_hex_secret_key(hex_str: &str) -> Result<SecretKey> {
    let bytes = hex::decode(hex_str).context("secret key must be hex-encoded (64 hex characters for 32 bytes)")?;
    if bytes.len() != 32 {
        anyhow::bail!("secret key must be exactly 32 bytes (64 hex characters), got {} bytes", bytes.len());
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);
    Ok(SecretKey::from_bytes(&key_bytes))
}

// =============================================================================
// Output types
// =============================================================================

/// Output for token inspect command.
struct TokenInspectOutput {
    issuer: String,
    audience: String,
    capabilities: Vec<String>,
    issued_at: u64,
    expires_at: u64,
    is_expired: bool,
    delegation_depth: u8,
    has_nonce: bool,
    has_proof: bool,
    version: u8,
}

impl Outputable for TokenInspectOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "issuer": self.issuer,
            "audience": self.audience,
            "capabilities": self.capabilities,
            "issued_at": self.issued_at,
            "expires_at": self.expires_at,
            "is_expired": self.is_expired,
            "delegation_depth": self.delegation_depth,
            "has_nonce": self.has_nonce,
            "has_proof": self.has_proof,
            "version": self.version,
        })
    }

    fn to_human(&self) -> String {
        let issued = format_unix_timestamp(self.issued_at);
        let expires = format_unix_timestamp(self.expires_at);
        let expired_marker = if self.is_expired { " (EXPIRED)" } else { "" };

        let mut out = String::new();
        out.push_str("Token Details\n");
        out.push_str("=============\n");
        out.push_str(&format!("Version:          {}\n", self.version));
        out.push_str(&format!("Issuer:           {}\n", self.issuer));
        out.push_str(&format!("Audience:         {}\n", self.audience));
        out.push_str(&format!("Issued At:        {}\n", issued));
        out.push_str(&format!("Expires At:       {}{}\n", expires, expired_marker));
        out.push_str(&format!("Delegation Depth: {}\n", self.delegation_depth));
        out.push_str(&format!("Has Nonce:        {}\n", self.has_nonce));
        out.push_str(&format!("Has Proof Chain:  {}\n", self.has_proof));
        out.push_str(&format!("Capabilities ({}):\n", self.capabilities.len()));
        for cap in &self.capabilities {
            out.push_str(&format!("  - {}\n", cap));
        }
        out
    }
}

/// Format a Unix timestamp as human-readable UTC string.
fn format_unix_timestamp(secs: u64) -> String {
    match chrono::DateTime::from_timestamp(secs as i64, 0) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => format!("{} (invalid timestamp)", secs),
    }
}

/// Format a capability for display.
fn format_capability(cap: &Capability) -> String {
    match cap {
        Capability::Full { prefix } => {
            if prefix.is_empty() {
                "Full (all keys)".to_string()
            } else {
                format!("Full (prefix: {prefix})")
            }
        }
        Capability::Read { prefix } => format!("Read (prefix: {prefix})"),
        Capability::Write { prefix } => format!("Write (prefix: {prefix})"),
        Capability::Delete { prefix } => format!("Delete (prefix: {prefix})"),
        Capability::Watch { prefix } => format!("Watch (prefix: {prefix})"),
        Capability::ClusterAdmin => "ClusterAdmin".to_string(),
        Capability::Delegate => "Delegate".to_string(),
        Capability::ShellExecute {
            command_pattern,
            working_dir,
        } => match working_dir {
            Some(wd) => format!("ShellExecute (pattern: {command_pattern}, dir: {wd})"),
            None => format!("ShellExecute (pattern: {command_pattern})"),
        },
        Capability::SecretsRead { mount, prefix } => format!("SecretsRead (mount: {mount}, prefix: {prefix})"),
        Capability::SecretsWrite { mount, prefix } => format!("SecretsWrite (mount: {mount}, prefix: {prefix})"),
        Capability::SecretsDelete { mount, prefix } => format!("SecretsDelete (mount: {mount}, prefix: {prefix})"),
        Capability::SecretsList { mount, prefix } => format!("SecretsList (mount: {mount}, prefix: {prefix})"),
        Capability::SecretsFull { mount, prefix } => format!("SecretsFull (mount: {mount}, prefix: {prefix})"),
        Capability::SecretsAdmin => "SecretsAdmin".to_string(),
        Capability::TransitEncrypt { key_prefix } => format!("TransitEncrypt (key_prefix: {key_prefix})"),
        Capability::TransitDecrypt { key_prefix } => format!("TransitDecrypt (key_prefix: {key_prefix})"),
        Capability::TransitSign { key_prefix } => format!("TransitSign (key_prefix: {key_prefix})"),
        Capability::TransitVerify { key_prefix } => format!("TransitVerify (key_prefix: {key_prefix})"),
        Capability::TransitKeyManage { key_prefix } => format!("TransitKeyManage (key_prefix: {key_prefix})"),
        Capability::PkiIssue { role_prefix } => format!("PkiIssue (role: {role_prefix})"),
        Capability::PkiRevoke => "PkiRevoke".to_string(),
        Capability::PkiReadCa => "PkiReadCa".to_string(),
        Capability::PkiManage => "PkiManage".to_string(),
    }
}

// =============================================================================
// Command execution
// =============================================================================

impl TokenCommand {
    /// Execute the token command. Does not require a cluster connection.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            TokenCommand::Generate(args) => run_generate(args, json),
            TokenCommand::Inspect(args) => run_inspect(args, json),
            TokenCommand::Delegate(args) => run_delegate(args, json),
        }
    }
}

/// Generate a new capability token.
fn run_generate(args: GenerateArgs, _json: bool) -> Result<()> {
    let secret_key = load_secret_key(args.secret_key_file.as_ref())?;
    let lifetime = parse_duration(&args.lifetime).context("invalid --lifetime")?;

    let token = if args.capabilities.is_empty() {
        // Default: full root token
        generate_root_token(&secret_key, lifetime).map_err(|e| anyhow::anyhow!("{}", e))?
    } else {
        // Custom capabilities
        let mut builder = TokenBuilder::new(secret_key).with_lifetime(lifetime).with_random_nonce();

        for cap_str in &args.capabilities {
            let cap = parse_capability(cap_str)?;
            builder = builder.with_capability(cap);
        }

        builder.build().map_err(|e| anyhow::anyhow!("{}", e))?
    };

    let b64 = token.to_base64().map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("{}", b64);
    Ok(())
}

/// Inspect a capability token.
fn run_inspect(args: InspectArgs, json: bool) -> Result<()> {
    let token = CapabilityToken::from_base64(&args.token).context("failed to decode token")?;

    let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    let audience = match &token.audience {
        aspen_auth::Audience::Bearer => "Bearer (anyone)".to_string(),
        aspen_auth::Audience::Key(pk) => format!("Key ({})", hex::encode(pk.as_bytes())),
    };

    let output = TokenInspectOutput {
        issuer: hex::encode(token.issuer.as_bytes()),
        audience,
        capabilities: token.capabilities.iter().map(format_capability).collect(),
        issued_at: token.issued_at,
        expires_at: token.expires_at,
        is_expired: now_secs > token.expires_at,
        delegation_depth: token.delegation_depth,
        has_nonce: token.nonce.is_some(),
        has_proof: token.proof.is_some(),
        version: token.version,
    };

    print_output(&output, json);
    Ok(())
}

/// Delegate a child token from a parent.
fn run_delegate(args: DelegateArgs, _json: bool) -> Result<()> {
    let parent = CapabilityToken::from_base64(&args.parent).context("failed to decode parent token")?;
    let secret_key = load_secret_key(args.secret_key_file.as_ref())?;
    let lifetime = parse_duration(&args.lifetime).context("invalid --lifetime")?;

    if args.capabilities.is_empty() {
        anyhow::bail!("at least one --cap is required for delegation");
    }

    let mut builder = TokenBuilder::new(secret_key).delegated_from(parent).with_lifetime(lifetime).with_random_nonce();

    for cap_str in &args.capabilities {
        let cap = parse_capability(cap_str)?;
        builder = builder.with_capability(cap);
    }

    let child = builder.build().map_err(|e| anyhow::anyhow!("{}", e))?;
    let b64 = child.to_base64().map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("{}", b64);
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // parse_duration tests
    // =========================================================================

    #[test]
    fn test_parse_duration_days() {
        let d = parse_duration("365d").unwrap();
        assert_eq!(d, Duration::from_secs(365 * 86400));
    }

    #[test]
    fn test_parse_duration_hours() {
        let d = parse_duration("24h").unwrap();
        assert_eq!(d, Duration::from_secs(24 * 3600));
    }

    #[test]
    fn test_parse_duration_minutes() {
        let d = parse_duration("30m").unwrap();
        assert_eq!(d, Duration::from_secs(30 * 60));
    }

    #[test]
    fn test_parse_duration_seconds_suffix() {
        let d = parse_duration("3600s").unwrap();
        assert_eq!(d, Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_duration_bare_seconds() {
        let d = parse_duration("3600").unwrap();
        assert_eq!(d, Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("10x").is_err());
    }

    // =========================================================================
    // parse_capability tests
    // =========================================================================

    #[test]
    fn test_parse_capability_full_all() {
        let cap = parse_capability("full:").unwrap();
        assert_eq!(cap, Capability::Full { prefix: String::new() });
    }

    #[test]
    fn test_parse_capability_full_prefix() {
        let cap = parse_capability("full:myapp/").unwrap();
        assert_eq!(cap, Capability::Full {
            prefix: "myapp/".to_string()
        });
    }

    #[test]
    fn test_parse_capability_read() {
        let cap = parse_capability("read:logs/").unwrap();
        assert_eq!(cap, Capability::Read {
            prefix: "logs/".to_string()
        });
    }

    #[test]
    fn test_parse_capability_write() {
        let cap = parse_capability("write:data/").unwrap();
        assert_eq!(cap, Capability::Write {
            prefix: "data/".to_string()
        });
    }

    #[test]
    fn test_parse_capability_delete() {
        let cap = parse_capability("delete:tmp/").unwrap();
        assert_eq!(cap, Capability::Delete {
            prefix: "tmp/".to_string()
        });
    }

    #[test]
    fn test_parse_capability_watch() {
        let cap = parse_capability("watch:events/").unwrap();
        assert_eq!(cap, Capability::Watch {
            prefix: "events/".to_string()
        });
    }

    #[test]
    fn test_parse_capability_cluster_admin() {
        let cap = parse_capability("cluster-admin").unwrap();
        assert_eq!(cap, Capability::ClusterAdmin);
    }

    #[test]
    fn test_parse_capability_delegate() {
        let cap = parse_capability("delegate").unwrap();
        assert_eq!(cap, Capability::Delegate);
    }

    #[test]
    fn test_parse_capability_shell() {
        let cap = parse_capability("shell:pg_*").unwrap();
        assert_eq!(cap, Capability::ShellExecute {
            command_pattern: "pg_*".to_string(),
            working_dir: None,
        });
    }

    #[test]
    fn test_parse_capability_secrets_full() {
        let cap = parse_capability("secrets-full:secret/:app/").unwrap();
        assert_eq!(cap, Capability::SecretsFull {
            mount: "secret/".to_string(),
            prefix: "app/".to_string(),
        });
    }

    #[test]
    fn test_parse_capability_secrets_admin() {
        let cap = parse_capability("secrets-admin").unwrap();
        assert_eq!(cap, Capability::SecretsAdmin);
    }

    #[test]
    fn test_parse_capability_invalid_type() {
        let err = parse_capability("unknown:foo").unwrap_err();
        assert!(err.to_string().contains("unknown capability type"));
    }

    #[test]
    fn test_parse_capability_invalid_format() {
        let err = parse_capability("nocolon").unwrap_err();
        assert!(err.to_string().contains("invalid capability format"));
    }

    // =========================================================================
    // load_secret_key tests
    // =========================================================================

    #[test]
    fn test_load_secret_key_missing() {
        // No file, no env var
        unsafe { std::env::remove_var("ASPEN_SECRET_KEY") };
        let err = load_secret_key(None).unwrap_err();
        assert!(err.to_string().contains("secret key required"));
    }

    #[test]
    fn test_load_secret_key_from_env() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        unsafe { std::env::set_var("ASPEN_SECRET_KEY", &hex_key) };
        let loaded = load_secret_key(None).unwrap();
        assert_eq!(loaded.public(), sk.public());
        unsafe { std::env::remove_var("ASPEN_SECRET_KEY") };
    }

    #[test]
    fn test_load_secret_key_from_file() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_secret_key");
        std::fs::write(&tmp, &hex_key).unwrap();
        let loaded = load_secret_key(Some(&tmp)).unwrap();
        assert_eq!(loaded.public(), sk.public());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_load_secret_key_invalid_hex() {
        let tmp = std::env::temp_dir().join("aspen_test_bad_key");
        std::fs::write(&tmp, "not-hex").unwrap();
        let err = load_secret_key(Some(&tmp)).unwrap_err();
        let err_str = format!("{:#}", err);
        assert!(err_str.contains("hex") || err_str.contains("Odd number of digits"), "unexpected error: {err_str}");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_load_secret_key_wrong_length() {
        let tmp = std::env::temp_dir().join("aspen_test_short_key");
        std::fs::write(&tmp, "aabb").unwrap();
        let err = load_secret_key(Some(&tmp)).unwrap_err();
        let err_str = format!("{:#}", err);
        assert!(err_str.contains("32 bytes") || err_str.contains("secret key"), "unexpected error: {err_str}");
        std::fs::remove_file(&tmp).ok();
    }

    // =========================================================================
    // Generate tests
    // =========================================================================

    #[test]
    fn test_generate_default_root_token() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_gen_root");
        std::fs::write(&tmp, &hex_key).unwrap();

        let args = GenerateArgs {
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec![],
        };

        // Run generate — it prints to stdout, but we verify no error
        let result = run_generate(args, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_generate_specific_capabilities() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_gen_caps");
        std::fs::write(&tmp, &hex_key).unwrap();

        let args = GenerateArgs {
            secret_key_file: Some(tmp.clone()),
            lifetime: "24h".to_string(),
            capabilities: vec!["read:logs/".to_string(), "write:data/".to_string()],
        };

        let result = run_generate(args, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_generate_missing_key_error() {
        unsafe { std::env::remove_var("ASPEN_SECRET_KEY") };
        let args = GenerateArgs {
            secret_key_file: None,
            lifetime: "1h".to_string(),
            capabilities: vec![],
        };

        let result = run_generate(args, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("secret key required"));
    }

    #[test]
    fn test_generate_invalid_lifetime() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_gen_badlife");
        std::fs::write(&tmp, &hex_key).unwrap();

        let args = GenerateArgs {
            secret_key_file: Some(tmp.clone()),
            lifetime: "abc".to_string(),
            capabilities: vec![],
        };

        let result = run_generate(args, false);
        assert!(result.is_err());
        std::fs::remove_file(&tmp).ok();
    }

    // =========================================================================
    // Inspect tests
    // =========================================================================

    fn make_test_token(lifetime: Duration, caps: Vec<Capability>) -> String {
        let sk = SecretKey::generate(&mut rand::rng());
        let mut builder = TokenBuilder::new(sk).with_lifetime(lifetime).with_random_nonce();
        for cap in caps {
            builder = builder.with_capability(cap);
        }
        builder.build().unwrap().to_base64().unwrap()
    }

    #[test]
    fn test_inspect_valid_token() {
        let b64 = make_test_token(Duration::from_secs(3600), vec![
            Capability::Full { prefix: String::new() },
            Capability::Delegate,
        ]);

        let args = InspectArgs { token: b64 };
        let result = run_inspect(args, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inspect_json_output() {
        let b64 = make_test_token(Duration::from_secs(3600), vec![Capability::ClusterAdmin]);

        let args = InspectArgs { token: b64 };
        let result = run_inspect(args, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inspect_expired_token() {
        // Create a token that's already expired (lifetime 0 seconds would still be valid
        // at creation time, so we create one and check the output)
        let sk = SecretKey::generate(&mut rand::rng());
        let token = TokenBuilder::new(sk)
            .with_lifetime(Duration::from_secs(1))
            .with_capability(Capability::Delegate)
            .build()
            .unwrap();

        // Manually check expiry logic
        let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Token expires_at is now + 1s, so it's not expired yet.
        // For a true expired-token test, we verify the is_expired logic:
        assert!(!((now_secs) > token.expires_at)); // Not expired right after creation

        // Verify inspect runs without error
        let b64 = token.to_base64().unwrap();
        let result = run_inspect(InspectArgs { token: b64 }, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inspect_invalid_base64() {
        let args = InspectArgs {
            token: "not-valid-base64!!!".to_string(),
        };
        let result = run_inspect(args, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to decode token"));
    }

    // =========================================================================
    // Delegate tests
    // =========================================================================

    #[test]
    fn test_delegate_success() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_delegate_key");
        std::fs::write(&tmp, &hex_key).unwrap();

        let parent = TokenBuilder::new(sk)
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Full { prefix: String::new() })
            .with_capability(Capability::Delegate)
            .with_random_nonce()
            .build()
            .unwrap();
        let parent_b64 = parent.to_base64().unwrap();

        let args = DelegateArgs {
            parent: parent_b64,
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec!["read:logs/".to_string()],
        };

        let result = run_delegate(args, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_delegate_no_delegate_capability() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_delegate_nodel");
        std::fs::write(&tmp, &hex_key).unwrap();

        // Parent WITHOUT Delegate capability
        let parent = TokenBuilder::new(sk)
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Full { prefix: String::new() })
            .with_random_nonce()
            .build()
            .unwrap();
        let parent_b64 = parent.to_base64().unwrap();

        let args = DelegateArgs {
            parent: parent_b64,
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec!["read:logs/".to_string()],
        };

        let result = run_delegate(args, false);
        assert!(result.is_err());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_delegate_capability_escalation() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_delegate_esc");
        std::fs::write(&tmp, &hex_key).unwrap();

        // Parent with only Read
        let parent = TokenBuilder::new(sk)
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Read {
                prefix: "logs/".to_string(),
            })
            .with_capability(Capability::Delegate)
            .with_random_nonce()
            .build()
            .unwrap();
        let parent_b64 = parent.to_base64().unwrap();

        // Try to escalate to Write
        let args = DelegateArgs {
            parent: parent_b64,
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec!["write:data/".to_string()],
        };

        let result = run_delegate(args, false);
        assert!(result.is_err());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_delegate_max_depth() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_delegate_depth");
        std::fs::write(&tmp, &hex_key).unwrap();

        // Build a chain to max depth
        let mut current = TokenBuilder::new(sk.clone())
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Full { prefix: String::new() })
            .with_capability(Capability::Delegate)
            .with_random_nonce()
            .build()
            .unwrap();

        for _ in 0..aspen_auth::constants::MAX_DELEGATION_DEPTH {
            current = TokenBuilder::new(sk.clone())
                .delegated_from(current)
                .with_lifetime(Duration::from_secs(86400))
                .with_capability(Capability::Full { prefix: String::new() })
                .with_capability(Capability::Delegate)
                .with_random_nonce()
                .build()
                .unwrap();
        }

        // This one should fail — we're at max depth
        let parent_b64 = current.to_base64().unwrap();
        let args = DelegateArgs {
            parent: parent_b64,
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec!["read:logs/".to_string()],
        };

        let result = run_delegate(args, false);
        assert!(result.is_err());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_delegate_requires_capabilities() {
        let sk = SecretKey::generate(&mut rand::rng());
        let hex_key = hex::encode(sk.to_bytes());
        let tmp = std::env::temp_dir().join("aspen_test_delegate_nocaps");
        std::fs::write(&tmp, &hex_key).unwrap();

        let parent = TokenBuilder::new(sk)
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Full { prefix: String::new() })
            .with_capability(Capability::Delegate)
            .build()
            .unwrap();

        let args = DelegateArgs {
            parent: parent.to_base64().unwrap(),
            secret_key_file: Some(tmp.clone()),
            lifetime: "1h".to_string(),
            capabilities: vec![], // No caps
        };

        let result = run_delegate(args, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one --cap"));
        std::fs::remove_file(&tmp).ok();
    }

    // =========================================================================
    // Clap parsing tests
    // =========================================================================

    #[test]
    fn test_clap_parse_generate() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from([
            "aspen-cli",
            "token",
            "generate",
            "--secret-key-file",
            "/tmp/key",
            "--lifetime",
            "24h",
            "--cap",
            "read:logs/",
            "--cap",
            "write:data/",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_clap_parse_inspect() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(["aspen-cli", "token", "inspect", "base64tokenstring"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_clap_parse_delegate() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from([
            "aspen-cli",
            "token",
            "delegate",
            "--parent",
            "parentbase64",
            "--secret-key-file",
            "/tmp/key",
            "--lifetime",
            "1h",
            "--cap",
            "read:logs/",
        ]);
        assert!(cli.is_ok());
    }

    // =========================================================================
    // Round-trip tests
    // =========================================================================

    #[test]
    fn test_roundtrip_generate_inspect() {
        let sk = SecretKey::generate(&mut rand::rng());
        let token = TokenBuilder::new(sk.clone())
            .with_lifetime(Duration::from_secs(3600))
            .with_capability(Capability::Read {
                prefix: "logs/".to_string(),
            })
            .with_capability(Capability::Delegate)
            .with_random_nonce()
            .build()
            .unwrap();

        let b64 = token.to_base64().unwrap();
        let decoded = CapabilityToken::from_base64(&b64).unwrap();

        assert_eq!(decoded.issuer, sk.public());
        assert_eq!(decoded.capabilities.len(), 2);
        assert_eq!(decoded.delegation_depth, 0);
        assert!(decoded.nonce.is_some());
    }

    #[test]
    fn test_roundtrip_generate_delegate_inspect() {
        let sk = SecretKey::generate(&mut rand::rng());

        // Root token
        let root = TokenBuilder::new(sk.clone())
            .with_lifetime(Duration::from_secs(86400))
            .with_capability(Capability::Full { prefix: String::new() })
            .with_capability(Capability::Delegate)
            .with_random_nonce()
            .build()
            .unwrap();

        assert_eq!(root.delegation_depth, 0);

        // Delegate child
        let child = TokenBuilder::new(sk.clone())
            .delegated_from(root)
            .with_lifetime(Duration::from_secs(3600))
            .with_capability(Capability::Read {
                prefix: "logs/".to_string(),
            })
            .with_random_nonce()
            .build()
            .unwrap();

        assert_eq!(child.delegation_depth, 1);
        assert_eq!(child.capabilities.len(), 1);
        assert_eq!(child.capabilities[0], Capability::Read {
            prefix: "logs/".to_string()
        });
        assert!(child.proof.is_some());

        // Round-trip through base64
        let b64 = child.to_base64().unwrap();
        let decoded = CapabilityToken::from_base64(&b64).unwrap();
        assert_eq!(decoded.delegation_depth, 1);
        assert_eq!(decoded.capabilities.len(), 1);
    }
}
