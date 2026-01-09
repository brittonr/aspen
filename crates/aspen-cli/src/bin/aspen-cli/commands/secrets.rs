//! Secrets engine commands.
//!
//! Commands for managing Vault-compatible secrets engines:
//! - KV v2: Versioned key-value secrets with soft/hard delete
//! - Transit: Encryption-as-a-service (encrypt, decrypt, sign, verify)
//! - PKI: Certificate authority with role-based issuance

use std::collections::HashMap;

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

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

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
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

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
    pub mount: String,

    /// Check-and-set version (fail if current version doesn't match).
    #[arg(long)]
    pub cas: Option<u64>,
}

#[derive(Args)]
pub struct KvDeleteArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
    pub mount: String,

    /// Specific versions to delete (default: current version).
    #[arg(long, value_delimiter = ',')]
    pub versions: Vec<u64>,
}

#[derive(Args)]
pub struct KvDestroyArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
    pub mount: String,

    /// Versions to permanently destroy.
    #[arg(long, required = true, value_delimiter = ',')]
    pub versions: Vec<u64>,
}

#[derive(Args)]
pub struct KvUndeleteArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
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

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
    pub mount: String,
}

#[derive(Args)]
pub struct KvMetadataArgs {
    /// Secret path.
    pub path: String,

    /// Mount point for the KV engine (default: "secret").
    #[arg(long, default_value = "secret")]
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
// Output Types
// =============================================================================

struct KvReadOutput {
    success: bool,
    data: Option<HashMap<String, String>>,
    version: Option<u64>,
    error: Option<String>,
}

impl Outputable for KvReadOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "data": self.data,
            "version": self.version,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        let mut output = String::new();
        if let Some(version) = self.version {
            output.push_str(&format!("Version: {}\n", version));
        }
        if let Some(data) = &self.data {
            output.push_str("Data:\n");
            for (k, v) in data {
                output.push_str(&format!("  {}: {}\n", k, v));
            }
        }
        output
    }
}

struct KvWriteOutput {
    success: bool,
    version: Option<u64>,
    error: Option<String>,
}

impl Outputable for KvWriteOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "version": self.version,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(version) = self.version {
            format!("Secret written at version {}", version)
        } else {
            "Secret written".to_string()
        }
    }
}

struct KvListOutput {
    success: bool,
    keys: Vec<String>,
    error: Option<String>,
}

impl Outputable for KvListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "keys": self.keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.keys.is_empty() {
            return "No secrets found".to_string();
        }

        let mut output = format!("Keys ({}):\n", self.keys.len());
        for key in &self.keys {
            output.push_str(&format!("  {}\n", key));
        }
        output
    }
}

struct SimpleSuccessOutput {
    success: bool,
    message: String,
    error: Option<String>,
}

impl Outputable for SimpleSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "message": self.message,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            format!("Error: {}", err)
        } else {
            self.message.clone()
        }
    }
}

struct TransitEncryptOutput {
    success: bool,
    ciphertext: Option<String>,
    error: Option<String>,
}

impl Outputable for TransitEncryptOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "ciphertext": self.ciphertext,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(ct) = &self.ciphertext {
            format!("Ciphertext: {}", ct)
        } else {
            "Encryption failed".to_string()
        }
    }
}

struct TransitDecryptOutput {
    success: bool,
    plaintext: Option<String>,
    error: Option<String>,
}

impl Outputable for TransitDecryptOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "plaintext": self.plaintext,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(pt) = &self.plaintext {
            format!("Plaintext: {}", pt)
        } else {
            "Decryption failed".to_string()
        }
    }
}

struct TransitSignOutput {
    success: bool,
    signature: Option<String>,
    error: Option<String>,
}

impl Outputable for TransitSignOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "signature": self.signature,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(sig) = &self.signature {
            format!("Signature: {}", sig)
        } else {
            "Signing failed".to_string()
        }
    }
}

struct TransitVerifyOutput {
    success: bool,
    valid: Option<bool>,
    error: Option<String>,
}

impl Outputable for TransitVerifyOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "valid": self.valid,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        match self.valid {
            Some(true) => "Signature is VALID".to_string(),
            Some(false) => "Signature is INVALID".to_string(),
            None => "Verification failed".to_string(),
        }
    }
}

struct TransitListOutput {
    success: bool,
    keys: Vec<String>,
    error: Option<String>,
}

impl Outputable for TransitListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "keys": self.keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.keys.is_empty() {
            return "No keys found".to_string();
        }

        let mut output = format!("Keys ({}):\n", self.keys.len());
        for key in &self.keys {
            output.push_str(&format!("  {}\n", key));
        }
        output
    }
}

struct PkiCertificateOutput {
    success: bool,
    certificate: Option<String>,
    private_key: Option<String>,
    serial: Option<String>,
    error: Option<String>,
}

impl Outputable for PkiCertificateOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "certificate": self.certificate,
            "private_key": self.private_key,
            "serial": self.serial,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        let mut output = String::new();
        if let Some(serial) = &self.serial {
            output.push_str(&format!("Serial: {}\n\n", serial));
        }
        if let Some(cert) = &self.certificate {
            output.push_str("Certificate:\n");
            output.push_str(cert);
            output.push('\n');
        }
        if let Some(key) = &self.private_key {
            output.push_str("\nPrivate Key:\n");
            output.push_str(key);
            output.push('\n');
        }
        output
    }
}

struct PkiListOutput {
    success: bool,
    items: Vec<String>,
    error: Option<String>,
}

impl Outputable for PkiListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "items": self.items,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.items.is_empty() {
            return "No items found".to_string();
        }

        let mut output = format!("Items ({}):\n", self.items.len());
        for item in &self.items {
            output.push_str(&format!("  {}\n", item));
        }
        output
    }
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
        }
    }
}

impl KvCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            KvCommand::Get(args) => kv_get(client, args, json).await,
            KvCommand::Put(args) => kv_put(client, args, json).await,
            KvCommand::Delete(args) => kv_delete(client, args, json).await,
            KvCommand::Destroy(args) => kv_destroy(client, args, json).await,
            KvCommand::Undelete(args) => kv_undelete(client, args, json).await,
            KvCommand::List(args) => kv_list(client, args, json).await,
            KvCommand::Metadata(args) => kv_metadata(client, args, json).await,
        }
    }
}

impl TransitCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            TransitCommand::CreateKey(args) => transit_create_key(client, args, json).await,
            TransitCommand::Encrypt(args) => transit_encrypt(client, args, json).await,
            TransitCommand::Decrypt(args) => transit_decrypt(client, args, json).await,
            TransitCommand::Sign(args) => transit_sign(client, args, json).await,
            TransitCommand::Verify(args) => transit_verify(client, args, json).await,
            TransitCommand::RotateKey(args) => transit_rotate_key(client, args, json).await,
            TransitCommand::ListKeys(args) => transit_list_keys(client, args, json).await,
            TransitCommand::Datakey(args) => transit_datakey(client, args, json).await,
        }
    }
}

impl PkiCommand {
    async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PkiCommand::GenerateRoot(args) => pki_generate_root(client, args, json).await,
            PkiCommand::CreateRole(args) => pki_create_role(client, args, json).await,
            PkiCommand::Issue(args) => pki_issue(client, args, json).await,
            PkiCommand::Revoke(args) => pki_revoke(client, args, json).await,
            PkiCommand::ListCerts(args) => pki_list_certs(client, args, json).await,
            PkiCommand::ListRoles(args) => pki_list_roles(client, args, json).await,
            PkiCommand::GetRole(args) => pki_get_role(client, args, json).await,
            PkiCommand::GetCrl(args) => pki_get_crl(client, args, json).await,
        }
    }
}

// =============================================================================
// KV Command Implementations
// =============================================================================

async fn kv_get(client: &AspenClient, args: KvGetArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvRead {
            mount: args.mount,
            path: args.path,
            version: args.version,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvReadResult(result) => {
            let output = KvReadOutput {
                success: result.success,
                data: result.data,
                version: result.metadata.map(|m| m.version),
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

async fn kv_put(client: &AspenClient, args: KvPutArgs, json: bool) -> Result<()> {
    // Parse data as JSON
    let data: HashMap<String, String> = serde_json::from_str(&args.data)
        .map_err(|e| anyhow::anyhow!("Invalid data JSON: {}. Expected format: {{\"key\":\"value\"}}", e))?;

    let response = client
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: args.mount,
            path: args.path,
            data,
            cas: args.cas,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvWriteResult(result) => {
            let output = KvWriteOutput {
                success: result.success,
                version: result.version,
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

async fn kv_delete(client: &AspenClient, args: KvDeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvDelete {
            mount: args.mount,
            path: args.path,
            versions: args.versions,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvDeleteResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: "Secret versions deleted".to_string(),
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

async fn kv_destroy(client: &AspenClient, args: KvDestroyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvDestroy {
            mount: args.mount,
            path: args.path,
            versions: args.versions,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvDeleteResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: "Secret versions permanently destroyed".to_string(),
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

async fn kv_undelete(client: &AspenClient, args: KvUndeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvUndelete {
            mount: args.mount,
            path: args.path,
            versions: args.versions,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvDeleteResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: "Secret versions restored".to_string(),
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

async fn kv_list(client: &AspenClient, args: KvListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvList {
            mount: args.mount,
            path: args.path,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvListResult(result) => {
            let output = KvListOutput {
                success: result.success,
                keys: result.keys,
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

async fn kv_metadata(client: &AspenClient, args: KvMetadataArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvMetadata {
            mount: args.mount,
            path: args.path,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvMetadataResult(result) => {
            let output = json!({
                "success": result.success,
                "current_version": result.current_version,
                "max_versions": result.max_versions,
                "cas_required": result.cas_required,
                "versions": result.versions,
                "custom_metadata": result.custom_metadata,
                "error": result.error
            });
            if json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else if let Some(err) = result.error {
                println!("Error: {}", err);
                std::process::exit(1);
            } else {
                println!("Current version: {:?}", result.current_version);
                println!("Max versions: {:?}", result.max_versions);
                println!("CAS required: {:?}", result.cas_required);
                println!("Versions: {}", result.versions.len());
                for v in &result.versions {
                    println!("  - Version {}: deleted={}, destroyed={}", v.version, v.deleted, v.destroyed);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Transit Command Implementations
// =============================================================================

async fn transit_create_key(client: &AspenClient, args: TransitCreateKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitCreateKey {
            mount: args.mount,
            name: args.name,
            key_type: args.key_type,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitKeyResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: format!(
                    "Key '{}' created (type: {}, version: {})",
                    result.name.as_deref().unwrap_or("unknown"),
                    result.key_type.as_deref().unwrap_or("unknown"),
                    result.version.unwrap_or(0)
                ),
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

async fn transit_encrypt(client: &AspenClient, args: TransitEncryptArgs, json: bool) -> Result<()> {
    let context = args
        .context
        .map(|c| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, c))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid base64 context: {}", e))?;

    let response = client
        .send(ClientRpcRequest::SecretsTransitEncrypt {
            mount: args.mount,
            name: args.name,
            plaintext: args.plaintext.into_bytes(),
            context,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitEncryptResult(result) => {
            let output = TransitEncryptOutput {
                success: result.success,
                ciphertext: result.ciphertext,
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

async fn transit_decrypt(client: &AspenClient, args: TransitDecryptArgs, json: bool) -> Result<()> {
    let context = args
        .context
        .map(|c| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, c))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid base64 context: {}", e))?;

    let response = client
        .send(ClientRpcRequest::SecretsTransitDecrypt {
            mount: args.mount,
            name: args.name,
            ciphertext: args.ciphertext,
            context,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitDecryptResult(result) => {
            let plaintext = result.plaintext.map(|p| String::from_utf8_lossy(&p).to_string());
            let output = TransitDecryptOutput {
                success: result.success,
                plaintext,
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

async fn transit_sign(client: &AspenClient, args: TransitSignArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitSign {
            mount: args.mount,
            name: args.name,
            data: args.data.into_bytes(),
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitSignResult(result) => {
            let output = TransitSignOutput {
                success: result.success,
                signature: result.signature,
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

async fn transit_verify(client: &AspenClient, args: TransitVerifyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitVerify {
            mount: args.mount,
            name: args.name,
            data: args.data.into_bytes(),
            signature: args.signature,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitVerifyResult(result) => {
            let output = TransitVerifyOutput {
                success: result.success,
                valid: result.valid,
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

async fn transit_rotate_key(client: &AspenClient, args: TransitRotateKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitRotateKey {
            mount: args.mount,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitKeyResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: format!("Key rotated to version {}", result.version.unwrap_or(0)),
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

async fn transit_list_keys(client: &AspenClient, args: TransitListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsTransitListKeys { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsTransitListResult(result) => {
            let output = TransitListOutput {
                success: result.success,
                keys: result.keys,
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

async fn transit_datakey(client: &AspenClient, args: TransitDatakeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitDatakey {
            mount: args.mount,
            name: args.name,
            key_type: args.key_type,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitDatakeyResult(result) => {
            let output = json!({
                "success": result.success,
                "plaintext": result.plaintext.clone().map(|p| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, p)),
                "ciphertext": result.ciphertext,
                "error": result.error
            });
            if json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else if let Some(err) = result.error {
                println!("Error: {}", err);
                std::process::exit(1);
            } else {
                if let Some(pt) = result.plaintext {
                    println!(
                        "Plaintext (base64): {}",
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pt)
                    );
                }
                if let Some(ct) = result.ciphertext {
                    println!("Ciphertext: {}", ct);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// PKI Command Implementations
// =============================================================================

async fn pki_generate_root(client: &AspenClient, args: PkiGenerateRootArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: args.mount,
            common_name: args.common_name,
            ttl_days: args.ttl_days,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiCertificateResult(result) => {
            let output = PkiCertificateOutput {
                success: result.success,
                certificate: result.certificate,
                private_key: result.private_key,
                serial: result.serial,
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

async fn pki_create_role(client: &AspenClient, args: PkiCreateRoleArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: args.mount,
            name: args.name,
            allowed_domains: args.allowed_domains,
            max_ttl_days: args.max_ttl_days,
            allow_bare_domains: args.allow_bare_domains,
            allow_wildcards: args.allow_wildcards,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiRoleResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: format!(
                    "Role '{}' created",
                    result.role.as_ref().map(|r| r.name.as_str()).unwrap_or("unknown")
                ),
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

async fn pki_issue(client: &AspenClient, args: PkiIssueArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: args.mount,
            role: args.role,
            common_name: args.common_name,
            alt_names: args.alt_names,
            ttl_days: args.ttl_days,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiCertificateResult(result) => {
            let output = PkiCertificateOutput {
                success: result.success,
                certificate: result.certificate,
                private_key: result.private_key,
                serial: result.serial,
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

async fn pki_revoke(client: &AspenClient, args: PkiRevokeArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiRevoke {
            mount: args.mount,
            serial: args.serial,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiRevokeResult(result) => {
            let output = SimpleSuccessOutput {
                success: result.success,
                message: format!("Certificate {} revoked", result.serial.as_deref().unwrap_or("unknown")),
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

async fn pki_list_certs(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiListCerts { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiListResult(result) => {
            let output = PkiListOutput {
                success: result.success,
                items: result.items,
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

async fn pki_list_roles(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiListRoles { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiListResult(result) => {
            let output = PkiListOutput {
                success: result.success,
                items: result.items,
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

async fn pki_get_role(client: &AspenClient, args: PkiGetRoleArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiGetRole {
            mount: args.mount,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiRoleResult(result) => {
            if json {
                let output = json!({
                    "success": result.success,
                    "role": result.role,
                    "error": result.error
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else if let Some(err) = result.error {
                println!("Error: {}", err);
                std::process::exit(1);
            } else if let Some(role) = result.role {
                println!("Role: {}", role.name);
                println!("Allowed domains: {:?}", role.allowed_domains);
                println!("Max TTL: {} days", role.max_ttl_days);
                println!("Allow bare domains: {}", role.allow_bare_domains);
                println!("Allow wildcards: {}", role.allow_wildcards);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn pki_get_crl(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiGetCrl { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiCrlResult(result) => {
            if json {
                let output = json!({
                    "success": result.success,
                    "crl": result.crl,
                    "error": result.error
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else if let Some(err) = result.error {
                println!("Error: {}", err);
                std::process::exit(1);
            } else if let Some(crl) = result.crl {
                println!("{}", crl);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
