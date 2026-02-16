//! Secrets engine operation implementations.
//!
//! Async functions that communicate with the Aspen cluster via the client RPC protocol.

use std::collections::HashMap;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use serde_json::json;

use super::KvDeleteArgs;
use super::KvDestroyArgs;
use super::KvGetArgs;
use super::KvListArgs;
use super::KvMetadataArgs;
use super::KvPutArgs;
use super::KvUndeleteArgs;
use super::NixCacheCreateKeyArgs;
use super::NixCacheDeleteKeyArgs;
use super::NixCacheGetPublicKeyArgs;
use super::NixCacheListKeysArgs;
use super::NixCacheRotateKeyArgs;
use super::PkiCreateRoleArgs;
use super::PkiGenerateRootArgs;
use super::PkiGetRoleArgs;
use super::PkiIssueArgs;
use super::PkiListArgs;
use super::PkiRevokeArgs;
use super::TransitCreateKeyArgs;
use super::TransitDatakeyArgs;
use super::TransitDecryptArgs;
use super::TransitEncryptArgs;
use super::TransitListArgs;
use super::TransitRotateKeyArgs;
use super::TransitSignArgs;
use super::TransitVerifyArgs;
use super::formatting::KvListOutput;
use super::formatting::KvReadOutput;
use super::formatting::KvWriteOutput;
use super::formatting::NixCacheDeleteOutput;
use super::formatting::NixCacheKeyOutput;
use super::formatting::NixCacheListOutput;
use super::formatting::PkiCertificateOutput;
use super::formatting::PkiListOutput;
use super::formatting::SimpleSuccessOutput;
use super::formatting::TransitDecryptOutput;
use super::formatting::TransitEncryptOutput;
use super::formatting::TransitListOutput;
use super::formatting::TransitSignOutput;
use super::formatting::TransitVerifyOutput;
use crate::client::AspenClient;
use crate::output::print_output;

// =============================================================================
// KV Command Implementations
// =============================================================================

pub(crate) async fn kv_get(client: &AspenClient, args: KvGetArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn kv_put(client: &AspenClient, args: KvPutArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn kv_delete(client: &AspenClient, args: KvDeleteArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn kv_destroy(client: &AspenClient, args: KvDestroyArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn kv_undelete(client: &AspenClient, args: KvUndeleteArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn kv_list(client: &AspenClient, args: KvListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvList {
            mount: args.mount,
            path: args.path,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvListResult(result) => {
            let output = KvListOutput {
                is_success: result.success,
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

pub(crate) async fn kv_metadata(client: &AspenClient, args: KvMetadataArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsKvMetadata {
            mount: args.mount,
            path: args.path,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsKvMetadataResult(result) => {
            let output = json!({
                "is_success": result.success,
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

pub(crate) async fn transit_create_key(client: &AspenClient, args: TransitCreateKeyArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn transit_encrypt(client: &AspenClient, args: TransitEncryptArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn transit_decrypt(client: &AspenClient, args: TransitDecryptArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn transit_sign(client: &AspenClient, args: TransitSignArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn transit_verify(client: &AspenClient, args: TransitVerifyArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
                is_valid: result.valid,
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

pub(crate) async fn transit_rotate_key(client: &AspenClient, args: TransitRotateKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsTransitRotateKey {
            mount: args.mount,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsTransitKeyResult(result) => {
            let output = SimpleSuccessOutput {
                is_success: result.success,
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

pub(crate) async fn transit_list_keys(client: &AspenClient, args: TransitListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsTransitListKeys { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsTransitListResult(result) => {
            let output = TransitListOutput {
                is_success: result.success,
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

pub(crate) async fn transit_datakey(client: &AspenClient, args: TransitDatakeyArgs, json: bool) -> Result<()> {
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
                "is_success": result.success,
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

pub(crate) async fn pki_generate_root(client: &AspenClient, args: PkiGenerateRootArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn pki_create_role(client: &AspenClient, args: PkiCreateRoleArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: args.mount,
            name: args.name,
            allowed_domains: args.allowed_domains,
            max_ttl_days: args.max_ttl_days,
            allow_bare_domains: args.allow_bare_domains,
            allow_wildcards: args.allow_wildcards,
            allow_subdomains: args.allow_subdomains,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiRoleResult(result) => {
            let output = SimpleSuccessOutput {
                is_success: result.success,
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

pub(crate) async fn pki_issue(client: &AspenClient, args: PkiIssueArgs, json: bool) -> Result<()> {
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
                is_success: result.success,
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

pub(crate) async fn pki_revoke(client: &AspenClient, args: PkiRevokeArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsPkiRevoke {
            mount: args.mount,
            serial: args.serial,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsPkiRevokeResult(result) => {
            let output = SimpleSuccessOutput {
                is_success: result.success,
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

pub(crate) async fn pki_list_certs(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiListCerts { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiListResult(result) => {
            let output = PkiListOutput {
                is_success: result.success,
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

pub(crate) async fn pki_list_roles(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiListRoles { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiListResult(result) => {
            let output = PkiListOutput {
                is_success: result.success,
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

pub(crate) async fn pki_get_role(client: &AspenClient, args: PkiGetRoleArgs, json: bool) -> Result<()> {
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
                    "is_success": result.success,
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

pub(crate) async fn pki_get_crl(client: &AspenClient, args: PkiListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsPkiGetCrl { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsPkiCrlResult(result) => {
            if json {
                let output = json!({
                    "is_success": result.success,
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

// =============================================================================
// Nix Cache Command Implementations
// =============================================================================

pub(crate) async fn nix_cache_create_key(client: &AspenClient, args: NixCacheCreateKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsNixCacheCreateKey {
            mount: args.mount,
            cache_name: args.cache_name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsNixCacheKeyResult(result) => {
            let output = NixCacheKeyOutput {
                is_success: result.success,
                public_key: result.public_key,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(crate) async fn nix_cache_get_public_key(
    client: &AspenClient,
    args: NixCacheGetPublicKeyArgs,
    json: bool,
) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsNixCacheGetPublicKey {
            mount: args.mount,
            cache_name: args.cache_name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsNixCacheKeyResult(result) => {
            let output = NixCacheKeyOutput {
                is_success: result.success,
                public_key: result.public_key,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(crate) async fn nix_cache_rotate_key(client: &AspenClient, args: NixCacheRotateKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsNixCacheRotateKey {
            mount: args.mount,
            cache_name: args.cache_name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsNixCacheKeyResult(result) => {
            let output = NixCacheKeyOutput {
                is_success: result.success,
                public_key: result.public_key,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(crate) async fn nix_cache_delete_key(client: &AspenClient, args: NixCacheDeleteKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SecretsNixCacheDeleteKey {
            mount: args.mount,
            cache_name: args.cache_name,
        })
        .await?;

    match response {
        ClientRpcResponse::SecretsNixCacheDeleteResult(result) => {
            let output = NixCacheDeleteOutput {
                is_success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(crate) async fn nix_cache_list_keys(client: &AspenClient, args: NixCacheListKeysArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::SecretsNixCacheListKeys { mount: args.mount }).await?;

    match response {
        ClientRpcResponse::SecretsNixCacheListResult(result) => {
            let output = NixCacheListOutput {
                is_success: result.success,
                cache_names: result.cache_names,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
