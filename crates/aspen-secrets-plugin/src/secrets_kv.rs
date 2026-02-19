//! KV v2 secrets engine handlers.
//!
//! Implements versioned key-value secrets with soft/hard delete, CAS, and metadata.
//! All data is stored in the plugin's `__secrets:` KV namespace.

use std::collections::HashMap;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsKvDeleteResultResponse;
use aspen_client_api::SecretsKvListResultResponse;
use aspen_client_api::SecretsKvMetadataResultResponse;
use aspen_client_api::SecretsKvReadResultResponse;
use aspen_client_api::SecretsKvVersionInfo;
use aspen_client_api::SecretsKvVersionMetadata;
use aspen_client_api::SecretsKvWriteResultResponse;
use aspen_wasm_guest_sdk::host;

use crate::kv;
use crate::types::SecretPathMetadata;
use crate::types::VersionedSecret;
use crate::types::secret_data_key;
use crate::types::secret_data_prefix;
use crate::types::secret_list_prefix;
use crate::types::secret_meta_key;
use crate::types::validate_mount;
use crate::types::validate_path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    host::current_time_ms()
}

fn load_metadata(mount: &str, path: &str) -> Result<Option<SecretPathMetadata>, String> {
    let key = secret_meta_key(mount, path);
    match kv::get(&key)? {
        Some(json) => {
            let meta: SecretPathMetadata = serde_json::from_str(&json).map_err(|e| format!("corrupt metadata: {e}"))?;
            Ok(Some(meta))
        }
        None => Ok(None),
    }
}

fn save_metadata(mount: &str, path: &str, meta: &SecretPathMetadata) -> Result<(), String> {
    let key = secret_meta_key(mount, path);
    let json = serde_json::to_string(meta).map_err(|e| format!("serialize metadata: {e}"))?;
    kv::put(&key, &json)
}

fn load_version(mount: &str, path: &str, version: u64) -> Result<Option<VersionedSecret>, String> {
    let key = secret_data_key(mount, path, version);
    match kv::get(&key)? {
        Some(json) => {
            let secret: VersionedSecret =
                serde_json::from_str(&json).map_err(|e| format!("corrupt version data: {e}"))?;
            Ok(Some(secret))
        }
        None => Ok(None),
    }
}

fn save_version(mount: &str, path: &str, secret: &VersionedSecret) -> Result<(), String> {
    let key = secret_data_key(mount, path, secret.version);
    let json = serde_json::to_string(secret).map_err(|e| format!("serialize version: {e}"))?;
    kv::put(&key, &json)
}

fn kv_read_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
        is_success: false,
        data: None,
        metadata: None,
        error: Some(msg),
    })
}

fn kv_write_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::SecretsKvWriteResult(SecretsKvWriteResultResponse {
        is_success: false,
        version: None,
        error: Some(msg),
    })
}

fn kv_delete_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
        is_success: false,
        error: Some(msg),
    })
}

// ---------------------------------------------------------------------------
// KV v2 Handlers
// ---------------------------------------------------------------------------

/// Read a secret, optionally at a specific version.
pub fn handle_kv_read(mount: String, path: String, version: Option<u64>) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_read_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_read_err(e);
    }

    let meta = match load_metadata(&mount, &path) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                is_success: false,
                data: None,
                metadata: None,
                error: Some(format!("Secret not found: {path}")),
            });
        }
        Err(e) => return kv_read_err(e),
    };

    let target_version = version.unwrap_or(meta.current_version);

    match load_version(&mount, &path, target_version) {
        Ok(Some(secret)) => {
            if secret.destroyed {
                return ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                    is_success: false,
                    data: None,
                    metadata: None,
                    error: Some(format!("Version {} of secret '{}' has been destroyed", target_version, path)),
                });
            }
            if secret.deleted {
                return ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                    is_success: false,
                    data: None,
                    metadata: Some(SecretsKvVersionMetadata {
                        version: secret.version,
                        created_time_unix_ms: secret.created_time_ms,
                        deletion_time_unix_ms: secret.deletion_time_ms,
                        was_destroyed: false,
                    }),
                    error: Some(format!("Version {} of secret '{}' has been deleted", target_version, path)),
                });
            }

            ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                is_success: true,
                data: Some(secret.data),
                metadata: Some(SecretsKvVersionMetadata {
                    version: secret.version,
                    created_time_unix_ms: secret.created_time_ms,
                    deletion_time_unix_ms: None,
                    was_destroyed: false,
                }),
                error: None,
            })
        }
        Ok(None) => ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
            is_success: false,
            data: None,
            metadata: None,
            error: Some(format!("Version {} not found for secret: {}", target_version, path)),
        }),
        Err(e) => kv_read_err(e),
    }
}

/// Write a new version of a secret.
pub fn handle_kv_write(
    mount: String,
    path: String,
    data: HashMap<String, String>,
    cas: Option<u64>,
) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_write_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_write_err(e);
    }
    if data.len() > crate::types::MAX_KV_PAIRS_PER_SECRET {
        return kv_write_err(format!(
            "Too many key-value pairs: {} (max {})",
            data.len(),
            crate::types::MAX_KV_PAIRS_PER_SECRET
        ));
    }

    let now = now_ms();

    // Load or create metadata
    let mut meta = match load_metadata(&mount, &path) {
        Ok(Some(m)) => m,
        Ok(None) => SecretPathMetadata::new(now),
        Err(e) => return kv_write_err(e),
    };

    // CAS check
    if let Some(expected) = cas
        && expected != meta.current_version
    {
        return ClientRpcResponse::SecretsKvWriteResult(SecretsKvWriteResultResponse {
            is_success: false,
            version: None,
            error: Some(format!(
                "CAS conflict for secret '{}': expected version {}, found {}",
                path, expected, meta.current_version
            )),
        });
    }

    // Also check if CAS is required by metadata
    if meta.cas_required && cas.is_none() && meta.current_version > 0 {
        return kv_write_err(format!("CAS is required for secret '{}' but no cas value was provided", path));
    }

    // Bump version
    meta.current_version += 1;
    meta.updated_time_ms = now;
    let new_version = meta.current_version;

    let secret = VersionedSecret {
        data,
        version: new_version,
        created_time_ms: now,
        deleted: false,
        destroyed: false,
        deletion_time_ms: None,
    };

    // Save version data
    if let Err(e) = save_version(&mount, &path, &secret) {
        return kv_write_err(e);
    }

    // Save metadata
    if let Err(e) = save_metadata(&mount, &path, &meta) {
        return kv_write_err(e);
    }

    // Garbage collect old versions if over max
    if new_version > meta.max_versions as u64 {
        let oldest_to_keep = new_version - meta.max_versions as u64 + 1;
        for v in 1..oldest_to_keep {
            let key = secret_data_key(&mount, &path, v);
            let _ = kv::delete(&key);
        }
    }

    ClientRpcResponse::SecretsKvWriteResult(SecretsKvWriteResultResponse {
        is_success: true,
        version: Some(new_version),
        error: None,
    })
}

/// Soft-delete specific versions of a secret.
pub fn handle_kv_delete(mount: String, path: String, versions: Vec<u64>) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_delete_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_delete_err(e);
    }

    let now = now_ms();

    for version in &versions {
        match load_version(&mount, &path, *version) {
            Ok(Some(mut secret)) => {
                if !secret.destroyed {
                    secret.deleted = true;
                    secret.deletion_time_ms = Some(now);
                    if let Err(e) = save_version(&mount, &path, &secret) {
                        return kv_delete_err(e);
                    }
                }
            }
            Ok(None) => {
                // Version doesn't exist — skip silently
            }
            Err(e) => return kv_delete_err(e),
        }
    }

    ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
        is_success: true,
        error: None,
    })
}

/// Permanently destroy specific versions of a secret.
pub fn handle_kv_destroy(mount: String, path: String, versions: Vec<u64>) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_delete_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_delete_err(e);
    }

    for version in &versions {
        match load_version(&mount, &path, *version) {
            Ok(Some(mut secret)) => {
                secret.destroyed = true;
                secret.data.clear();
                if let Err(e) = save_version(&mount, &path, &secret) {
                    return kv_delete_err(e);
                }
            }
            Ok(None) => {}
            Err(e) => return kv_delete_err(e),
        }
    }

    ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
        is_success: true,
        error: None,
    })
}

/// Undelete previously soft-deleted versions.
pub fn handle_kv_undelete(mount: String, path: String, versions: Vec<u64>) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_delete_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_delete_err(e);
    }

    for version in &versions {
        match load_version(&mount, &path, *version) {
            Ok(Some(mut secret)) => {
                if secret.deleted && !secret.destroyed {
                    secret.deleted = false;
                    secret.deletion_time_ms = None;
                    if let Err(e) = save_version(&mount, &path, &secret) {
                        return kv_delete_err(e);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return kv_delete_err(e),
        }
    }

    ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
        is_success: true,
        error: None,
    })
}

/// List secrets under a path prefix.
pub fn handle_kv_list(mount: String, path: String) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return ClientRpcResponse::SecretsKvListResult(SecretsKvListResultResponse {
            is_success: false,
            keys: vec![],
            error: Some(e),
        });
    }

    let prefix = secret_list_prefix(&mount, &path);
    match kv::scan(&prefix, 1000) {
        Ok(entries) => {
            let base_prefix = secret_list_prefix(&mount, "");
            let keys: Vec<String> =
                entries.iter().filter_map(|(key, _)| key.strip_prefix(&base_prefix).map(String::from)).collect();

            ClientRpcResponse::SecretsKvListResult(SecretsKvListResultResponse {
                is_success: true,
                keys,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::SecretsKvListResult(SecretsKvListResultResponse {
            is_success: false,
            keys: vec![],
            error: Some(e),
        }),
    }
}

/// Get metadata for a secret path.
pub fn handle_kv_metadata(mount: String, path: String) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
            is_success: false,
            current_version: None,
            max_versions: None,
            cas_required: None,
            created_time_unix_ms: None,
            updated_time_unix_ms: None,
            versions: vec![],
            custom_metadata: None,
            error: Some(e),
        });
    }

    let meta = match load_metadata(&mount, &path) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
                is_success: false,
                current_version: None,
                max_versions: None,
                cas_required: None,
                created_time_unix_ms: None,
                updated_time_unix_ms: None,
                versions: vec![],
                custom_metadata: None,
                error: Some(format!("Secret not found: {path}")),
            });
        }
        Err(e) => {
            return ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
                is_success: false,
                current_version: None,
                max_versions: None,
                cas_required: None,
                created_time_unix_ms: None,
                updated_time_unix_ms: None,
                versions: vec![],
                custom_metadata: None,
                error: Some(e),
            });
        }
    };

    // Collect version info
    let data_prefix = secret_data_prefix(&mount, &path);
    let version_entries = kv::scan(&data_prefix, 1000).unwrap_or_default();

    let mut versions = Vec::new();
    for (_, value) in &version_entries {
        if let Ok(secret) = serde_json::from_str::<VersionedSecret>(value) {
            versions.push(SecretsKvVersionInfo {
                version: secret.version,
                created_time_unix_ms: secret.created_time_ms,
                was_deleted: secret.deleted,
                was_destroyed: secret.destroyed,
            });
        }
    }

    ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
        is_success: true,
        current_version: Some(meta.current_version),
        max_versions: Some(meta.max_versions),
        cas_required: Some(meta.cas_required),
        created_time_unix_ms: Some(meta.created_time_ms),
        updated_time_unix_ms: Some(meta.updated_time_ms),
        versions,
        custom_metadata: meta.custom_metadata,
        error: None,
    })
}

/// Update metadata for a secret path.
pub fn handle_kv_update_metadata(
    mount: String,
    path: String,
    max_versions: Option<u32>,
    cas_required: Option<bool>,
    custom_metadata: Option<HashMap<String, String>>,
) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_delete_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_delete_err(e);
    }

    let mut meta = match load_metadata(&mount, &path) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return kv_delete_err(format!("Secret not found: {path}"));
        }
        Err(e) => return kv_delete_err(e),
    };

    if let Some(mv) = max_versions {
        meta.max_versions = mv;
    }
    if let Some(cr) = cas_required {
        meta.cas_required = cr;
    }
    if let Some(cm) = custom_metadata {
        meta.custom_metadata = Some(cm);
    }
    meta.updated_time_ms = now_ms();

    match save_metadata(&mount, &path, &meta) {
        Ok(()) => ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => kv_delete_err(e),
    }
}

/// Delete a secret and all its versions.
pub fn handle_kv_delete_metadata(mount: String, path: String) -> ClientRpcResponse {
    if let Err(e) = validate_mount(&mount) {
        return kv_delete_err(e);
    }
    if let Err(e) = validate_path(&path) {
        return kv_delete_err(e);
    }

    // Delete metadata
    let meta_key = secret_meta_key(&mount, &path);
    let _ = kv::delete(&meta_key);

    // Delete all versions
    let data_prefix = secret_data_prefix(&mount, &path);
    if let Ok(entries) = kv::scan(&data_prefix, 1000) {
        for (key, _) in entries {
            let _ = kv::delete(&key);
        }
    }

    ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
        is_success: true,
        error: None,
    })
}
