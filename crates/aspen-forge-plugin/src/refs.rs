//! Ref handlers: get, set, delete, CAS, list branches, list tags, delegate key.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ForgeKeyResultResponse;
use aspen_client_api::ForgeRefInfo;
use aspen_client_api::ForgeRefListResultResponse;
use aspen_client_api::ForgeRefResultResponse;

use crate::kv;

/// Maximum refs returned by list.
const MAX_LIST_REFS: u32 = 1000;

// ============================================================================
// KV key helpers
// ============================================================================

fn ref_key(repo_id_hex: &str, ref_name: &str) -> String {
    format!("forge:refs:{repo_id_hex}:{ref_name}")
}

fn branch_prefix(repo_id_hex: &str) -> String {
    format!("forge:refs:{repo_id_hex}:heads/")
}

fn tag_prefix(repo_id_hex: &str) -> String {
    format!("forge:refs:{repo_id_hex}:tags/")
}

/// Extract the ref name from a full KV key by stripping the prefix.
fn extract_ref_name(key: &str, prefix: &str) -> Option<String> {
    key.strip_prefix(prefix).map(|s| s.to_string())
}

// ============================================================================
// Handlers
// ============================================================================

pub fn handle_get_ref(repo_id: String, ref_name: String) -> ClientRpcResponse {
    let key = ref_key(&repo_id, &ref_name);

    match kv::kv_get(&key) {
        Some(bytes) => {
            let hash = String::from_utf8(bytes).unwrap_or_default();
            ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                is_success: true,
                was_found: true,
                ref_info: Some(ForgeRefInfo { name: ref_name, hash }),
                previous_hash: None,
                error: None,
            })
        }
        None => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: true,
            was_found: false,
            ref_info: None,
            previous_hash: None,
            error: None,
        }),
    }
}

pub fn handle_set_ref(
    repo_id: String,
    ref_name: String,
    hash: String,
    _signer: Option<String>,
    _signature: Option<String>,
    _timestamp_ms: Option<u64>,
) -> ClientRpcResponse {
    let key = ref_key(&repo_id, &ref_name);

    // Read previous value for the response
    let previous_hash = kv::kv_get(&key).and_then(|b| String::from_utf8(b).ok());

    match kv::kv_put(&key, hash.as_bytes()) {
        Ok(()) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: true,
            was_found: true,
            ref_info: Some(ForgeRefInfo { name: ref_name, hash }),
            previous_hash,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: false,
            was_found: false,
            ref_info: None,
            previous_hash,
            error: Some(format!("failed to set ref: {e}")),
        }),
    }
}

pub fn handle_delete_ref(repo_id: String, ref_name: String) -> ClientRpcResponse {
    let key = ref_key(&repo_id, &ref_name);

    let previous_hash = kv::kv_get(&key).and_then(|b| String::from_utf8(b).ok());
    let was_found = previous_hash.is_some();

    match kv::kv_delete(&key) {
        Ok(()) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: true,
            was_found,
            ref_info: None,
            previous_hash,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: false,
            was_found,
            ref_info: None,
            previous_hash,
            error: Some(format!("failed to delete ref: {e}")),
        }),
    }
}

pub fn handle_cas_ref(
    repo_id: String,
    ref_name: String,
    expected: Option<String>,
    new_hash: String,
    _signer: Option<String>,
    _signature: Option<String>,
    _timestamp_ms: Option<u64>,
) -> ClientRpcResponse {
    let key = ref_key(&repo_id, &ref_name);

    let expected_bytes = expected.as_deref().unwrap_or("").as_bytes().to_vec();
    let previous_hash = kv::kv_get(&key).and_then(|b| String::from_utf8(b).ok());

    match kv::kv_cas(&key, &expected_bytes, new_hash.as_bytes()) {
        Ok(()) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: true,
            was_found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: new_hash,
            }),
            previous_hash,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: false,
            was_found: previous_hash.is_some(),
            ref_info: None,
            previous_hash,
            error: Some(format!("CAS failed: {e}")),
        }),
    }
}

pub fn handle_list_branches(repo_id: String) -> ClientRpcResponse {
    let prefix = branch_prefix(&repo_id);
    list_refs_with_prefix(&prefix, "heads/")
}

pub fn handle_list_tags(repo_id: String) -> ClientRpcResponse {
    let prefix = tag_prefix(&repo_id);
    list_refs_with_prefix(&prefix, "tags/")
}

fn list_refs_with_prefix(kv_prefix: &str, ref_prefix: &str) -> ClientRpcResponse {
    let entries = kv::kv_scan(kv_prefix, MAX_LIST_REFS);

    let mut refs = Vec::new();
    for (key, value) in &entries {
        let short_name = match extract_ref_name(key, kv_prefix) {
            Some(n) => n,
            None => continue,
        };
        let hash = match String::from_utf8(value.clone()) {
            Ok(h) => h,
            Err(_) => continue,
        };
        refs.push(ForgeRefInfo {
            name: format!("{ref_prefix}{short_name}"),
            hash,
        });
    }

    let count = refs.len() as u32;
    ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
        is_success: true,
        refs,
        count,
        error: None,
    })
}

pub fn handle_get_delegate_key() -> ClientRpcResponse {
    let public_key = kv::public_key();

    ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
        is_success: true,
        public_key: if public_key.is_empty() { None } else { Some(public_key) },
        secret_key: None, // Never expose secret key to clients
        error: None,
    })
}
