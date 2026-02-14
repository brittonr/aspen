//! Secrets engine commands.
//!
//! Commands for managing Vault-compatible secrets engines:
//! - KV v2: Versioned key-value secrets with soft/hard delete
//! - Transit: Encryption-as-a-service (encrypt, decrypt, sign, verify)
//! - PKI: Certificate authority with role-based issuance

mod formatting;
mod operations;

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;

/// Secrets engine operations.
#[derive(Subcommand)]
pub enum SecretsCommand {
    /// KV v2 secrets engine operations.
    #[command(subcommand)]
    Kv(KvCommand),

    /// Transit secrets engine operations.
    #[command(subcommand)]
    Transit(TransitCommand),

    /// PKI secrets engine operations.
    #[command(subcommand)]
    Pki(PkiCommand),

    /// Nix cache signing key operations.
    #[command(subcommand)]
    NixCache(NixCacheCommand),
}

// =============================================================================
// KV Commands
// =============================================================================

/// KV v2 secrets operations.
#[derive(Subcommand)]
pub enum KvCommand {
    /// Read a secret from the KV store.
    Get(KvGetArgs),

    /// Write a secret to the KV store.
    Put(KvPutArgs),

    /// Delete secret versions (soft delete).
    Delete(KvDeleteArgs),

    /// Permanently destroy secret versions.
    Destroy(KvDestroyArgs),

    /// Restore soft-deleted secret versions.
    Undelete(KvUndeleteArgs),

    /// List secrets under a path prefix.
    List(KvListArgs),

    /// Read secret metadata (all versions).
    Metadata(KvMetadataArgs),
}

#[derive(Args)]
pub struct KvGetArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,

    /// Specific version to read (default: current).
    #[arg(long)]
    pub version: Option<u64>,
}

#[derive(Args)]
pub struct KvPutArgs {
    /// Secret path.
    pub path: String,

    /// Secret data as JSON object (e.g., '{"key":"value"}').
    pub data: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,

    /// Check-and-set version (fail if current version doesn't match).
    #[arg(long)]
    pub cas: Option<u64>,
}

#[derive(Args)]
pub struct KvDeleteArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,

    /// Specific versions to delete (default: current version).
    #[arg(long, value_delimiter = ',')]
    pub versions: Vec<u64>,
}

#[derive(Args)]
pub struct KvDestroyArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,

    /// Versions to permanently destroy.
    #[arg(long, required = true, value_delimiter = ',')]
    pub versions: Vec<u64>,
}

#[derive(Args)]
pub struct KvUndeleteArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,

    /// Versions to restore.
    #[arg(long, required = true, value_delimiter = ',')]
    pub versions: Vec<u64>,
}

#[derive(Args)]
pub struct KvListArgs {
    /// Path prefix to list.
    #[arg(default_value = "")]
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,
}

#[derive(Args)]
pub struct KvMetadataArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "kv").
    #[arg(long, default_value = "kv")]
    pub mount: String,
}

// =============================================================================
// Transit Commands
// =============================================================================

/// Transit secrets engine operations.
#[derive(Subcommand)]
pub enum TransitCommand {
    /// Create a new encryption key.
    CreateKey(TransitCreateKeyArgs),

    /// Encrypt data with a named key.
    Encrypt(TransitEncryptArgs),

    /// Decrypt data with a named key.
    Decrypt(TransitDecryptArgs),

    /// Sign data with a named key.
    Sign(TransitSignArgs),

    /// Verify a signature.
    Verify(TransitVerifyArgs),

    /// Rotate a key to a new version.
    RotateKey(TransitRotateKeyArgs),

    /// List all transit keys.
    ListKeys(TransitListArgs),

    /// Generate a data key for envelope encryption.
    Datakey(TransitDatakeyArgs),
}

#[derive(Args)]
pub struct TransitListArgs {
    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,
}

#[derive(Args)]
pub struct TransitCreateKeyArgs {
    /// Key name.
    pub name: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,

    /// Key type (aes256-gcm, xchacha20-poly1305, ed25519).
    #[arg(long, default_value = "xchacha20-poly1305")]
    pub key_type: String,
}

#[derive(Args)]
pub struct TransitEncryptArgs {
    /// Key name.
    pub name: String,

    /// Plaintext to encrypt (base64-encoded for binary data).
    pub plaintext: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,

    /// Convergent encryption context (base64-encoded).
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Args)]
pub struct TransitDecryptArgs {
    /// Key name.
    pub name: String,

    /// Ciphertext to decrypt (aspen:v...:... format).
    pub ciphertext: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,

    /// Convergent encryption context (base64-encoded).
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Args)]
pub struct TransitSignArgs {
    /// Key name.
    pub name: String,

    /// Data to sign (base64-encoded for binary data).
    pub data: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,
}

#[derive(Args)]
pub struct TransitVerifyArgs {
    /// Key name.
    pub name: String,

    /// Original data (base64-encoded for binary data).
    pub data: String,

    /// Signature to verify (aspen:v...:... format).
    pub signature: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,
}

#[derive(Args)]
pub struct TransitRotateKeyArgs {
    /// Key name.
    pub name: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,
}

#[derive(Args)]
pub struct TransitDatakeyArgs {
    /// Key name.
    pub name: String,

    /// Mount point for the Transit engine (default: "transit").
    #[arg(long, default_value = "transit")]
    pub mount: String,

    /// Key type: "plaintext" (returns plaintext + ciphertext) or "wrapped" (ciphertext only).
    #[arg(long, default_value = "plaintext")]
    pub key_type: String,
}

// =============================================================================
// PKI Commands
// =============================================================================

/// PKI secrets engine operations.
#[derive(Subcommand)]
pub enum PkiCommand {
    /// Generate a root CA certificate.
    GenerateRoot(PkiGenerateRootArgs),

    /// Create a certificate role.
    CreateRole(PkiCreateRoleArgs),

    /// Issue a certificate.
    Issue(PkiIssueArgs),

    /// Revoke a certificate.
    Revoke(PkiRevokeArgs),

    /// List issued certificates.
    ListCerts(PkiListArgs),

    /// List certificate roles.
    ListRoles(PkiListArgs),

    /// Get role configuration.
    GetRole(PkiGetRoleArgs),

    /// Get Certificate Revocation List (CRL).
    GetCrl(PkiListArgs),
}

#[derive(Args)]
pub struct PkiListArgs {
    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,
}

#[derive(Args)]
pub struct PkiGenerateRootArgs {
    /// Common name for the CA certificate.
    pub common_name: String,

    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,

    /// TTL in days (default: 3650 = 10 years).
    #[arg(long)]
    pub ttl_days: Option<u32>,
}

#[derive(Args)]
pub struct PkiCreateRoleArgs {
    /// Role name.
    pub name: String,

    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,

    /// Allowed domains (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub allowed_domains: Vec<String>,

    /// Maximum TTL in days.
    #[arg(long, default_value = "365")]
    pub max_ttl_days: u32,

    /// Allow bare domains (apex domains).
    #[arg(long)]
    pub allow_bare_domains: bool,

    /// Allow wildcard certificates.
    #[arg(long)]
    pub allow_wildcards: bool,

    /// Allow subdomains of allowed domains.
    #[arg(long)]
    pub allow_subdomains: bool,
}

#[derive(Args)]
pub struct PkiIssueArgs {
    /// Role to use for issuance.
    pub role: String,

    /// Common name for the certificate.
    pub common_name: String,

    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,

    /// Alternative names (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub alt_names: Vec<String>,

    /// TTL in days (default: role's TTL).
    #[arg(long)]
    pub ttl_days: Option<u32>,
}

#[derive(Args)]
pub struct PkiRevokeArgs {
    /// Certificate serial number.
    pub serial: String,

    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,
}

#[derive(Args)]
pub struct PkiGetRoleArgs {
    /// Role name.
    pub name: String,

    /// Mount point for the PKI engine (default: "pki").
    #[arg(long, default_value = "pki")]
    pub mount: String,
}

// =============================================================================
// Nix Cache Commands
// =============================================================================

/// Nix cache signing key operations.
#[derive(Subcommand)]
pub enum NixCacheCommand {
    /// Create a signing key for a cache.
    CreateKey(NixCacheCreateKeyArgs),

    /// Get public key for a cache.
    GetPublicKey(NixCacheGetPublicKeyArgs),

    /// Rotate signing key to a new version.
    RotateKey(NixCacheRotateKeyArgs),

    /// Delete a cache signing key.
    DeleteKey(NixCacheDeleteKeyArgs),

    /// List all cache signing keys.
    ListKeys(NixCacheListKeysArgs),
}

#[derive(Args)]
pub struct NixCacheCreateKeyArgs {
    /// Cache name (e.g., "cache.example.com-1").
    pub cache_name: String,

    /// Mount point for the Transit engine (default: "nix-cache").
    #[arg(long, default_value = "nix-cache")]
    pub mount: String,
}

#[derive(Args)]
pub struct NixCacheGetPublicKeyArgs {
    /// Cache name (e.g., "cache.example.com-1").
    pub cache_name: String,

    /// Mount point for the Transit engine (default: "nix-cache").
    #[arg(long, default_value = "nix-cache")]
    pub mount: String,
}

#[derive(Args)]
pub struct NixCacheRotateKeyArgs {
    /// Cache name (e.g., "cache.example.com-1").
    pub cache_name: String,

    /// Mount point for the Transit engine (default: "nix-cache").
    #[arg(long, default_value = "nix-cache")]
    pub mount: String,
}

#[derive(Args)]
pub struct NixCacheDeleteKeyArgs {
    /// Cache name (e.g., "cache.example.com-1").
    pub cache_name: String,

    /// Mount point for the Transit engine (default: "nix-cache").
    #[arg(long, default_value = "nix-cache")]
    pub mount: String,
}

#[derive(Args)]
pub struct NixCacheListKeysArgs {
    /// Mount point for the Transit engine (default: "nix-cache").
    #[arg(long, default_value = "nix-cache")]
    pub mount: String,
}

// =============================================================================
// Command Execution
// =============================================================================

impl SecretsCommand {
    /// Execute the secrets command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            SecretsCommand::Kv(cmd) => cmd.run(client, json).await,
            SecretsCommand::Transit(cmd) => cmd.run(client, json).await,
            SecretsCommand::Pki(cmd) => cmd.run(client, json).await,
            SecretsCommand::NixCache(cmd) => cmd.run(client, json).await,
        }
    }
}

impl KvCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            KvCommand::Get(args) => operations::kv_get(client, args, json).await,
            KvCommand::Put(args) => operations::kv_put(client, args, json).await,
            KvCommand::Delete(args) => operations::kv_delete(client, args, json).await,
            KvCommand::Destroy(args) => operations::kv_destroy(client, args, json).await,
            KvCommand::Undelete(args) => operations::kv_undelete(client, args, json).await,
            KvCommand::List(args) => operations::kv_list(client, args, json).await,
            KvCommand::Metadata(args) => operations::kv_metadata(client, args, json).await,
        }
    }
}

impl TransitCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            TransitCommand::CreateKey(args) => operations::transit_create_key(client, args, json).await,
            TransitCommand::Encrypt(args) => operations::transit_encrypt(client, args, json).await,
            TransitCommand::Decrypt(args) => operations::transit_decrypt(client, args, json).await,
            TransitCommand::Sign(args) => operations::transit_sign(client, args, json).await,
            TransitCommand::Verify(args) => operations::transit_verify(client, args, json).await,
            TransitCommand::RotateKey(args) => operations::transit_rotate_key(client, args, json).await,
            TransitCommand::ListKeys(args) => operations::transit_list_keys(client, args, json).await,
            TransitCommand::Datakey(args) => operations::transit_datakey(client, args, json).await,
        }
    }
}

impl PkiCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PkiCommand::GenerateRoot(args) => operations::pki_generate_root(client, args, json).await,
            PkiCommand::CreateRole(args) => operations::pki_create_role(client, args, json).await,
            PkiCommand::Issue(args) => operations::pki_issue(client, args, json).await,
            PkiCommand::Revoke(args) => operations::pki_revoke(client, args, json).await,
            PkiCommand::ListCerts(args) => operations::pki_list_certs(client, args, json).await,
            PkiCommand::ListRoles(args) => operations::pki_list_roles(client, args, json).await,
            PkiCommand::GetRole(args) => operations::pki_get_role(client, args, json).await,
            PkiCommand::GetCrl(args) => operations::pki_get_crl(client, args, json).await,
        }
    }
}

impl NixCacheCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            NixCacheCommand::CreateKey(args) => operations::nix_cache_create_key(client, args, json).await,
            NixCacheCommand::GetPublicKey(args) => operations::nix_cache_get_public_key(client, args, json).await,
            NixCacheCommand::RotateKey(args) => operations::nix_cache_rotate_key(client, args, json).await,
            NixCacheCommand::DeleteKey(args) => operations::nix_cache_delete_key(client, args, json).await,
            NixCacheCommand::ListKeys(args) => operations::nix_cache_list_keys(client, args, json).await,
        }
    }
}
