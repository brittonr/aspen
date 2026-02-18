//! Repository handlers: create, get, list.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ForgeRepoInfo;
use aspen_client_api::ForgeRepoListResultResponse;
use aspen_client_api::ForgeRepoResultResponse;

use crate::kv;
use crate::signing;
use crate::types::RepoIdentity;
use crate::types::SignedObject;

/// Maximum repos returned by list.
const MAX_LIST_REPOS: u32 = 1000;

// ============================================================================
// KV key helpers
// ============================================================================

fn repo_identity_key(repo_id_hex: &str) -> String {
    format!("forge:repos:{repo_id_hex}:identity")
}

fn repo_name_index_key(name: &str) -> String {
    format!("forge:repos:by-name:{name}")
}

const REPO_IDENTITY_PREFIX: &str = "forge:repos:";
const REPO_IDENTITY_SUFFIX: &str = ":identity";

// ============================================================================
// Helpers
// ============================================================================

fn repo_id_from_name(name: &str) -> String {
    let hash = blake3::hash(name.as_bytes());
    hex::encode(hash.as_bytes())
}

fn read_repo_identity(repo_id_hex: &str) -> Option<SignedObject<RepoIdentity>> {
    let bytes = kv::kv_get(&repo_identity_key(repo_id_hex)).ok()??;
    serde_json::from_slice(&bytes).ok()
}

fn identity_to_info(repo_id_hex: &str, signed: &SignedObject<RepoIdentity>) -> ForgeRepoInfo {
    let identity = &signed.payload;
    ForgeRepoInfo {
        id: repo_id_hex.to_string(),
        name: identity.name.clone(),
        description: identity.description.clone(),
        default_branch: identity.default_branch.clone(),
        delegates: identity.delegates.clone(),
        threshold_delegates: identity.threshold,
        created_at_ms: identity.created_at_ms,
    }
}

// ============================================================================
// Handlers
// ============================================================================

pub fn handle_create_repo(
    name: String,
    description: Option<String>,
    default_branch: Option<String>,
) -> ClientRpcResponse {
    let repo_id_hex = repo_id_from_name(&name);

    // Check for existing repo
    if read_repo_identity(&repo_id_hex).is_some() {
        return ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: false,
            repo: None,
            error: Some(format!("repository '{name}' already exists")),
        });
    }

    let author_hex = kv::public_key();
    let now = kv::hlc_now();

    let identity = RepoIdentity {
        name: name.clone(),
        description,
        default_branch: default_branch.unwrap_or_else(|| "main".to_string()),
        delegates: vec![author_hex],
        threshold: 1,
        created_at_ms: now,
    };

    let signed = match signing::sign_object(&identity) {
        Ok(s) => s,
        Err(_) => {
            return ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: false,
                repo: None,
                error: Some("failed to sign identity: serialization error".to_string()),
            });
        }
    };
    let identity_bytes = match serde_json::to_vec(&signed) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: false,
                repo: None,
                error: Some(format!("failed to serialize identity: {e}")),
            });
        }
    };

    // Write identity
    if let Err(e) = kv::kv_put(&repo_identity_key(&repo_id_hex), &identity_bytes) {
        return ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: false,
            repo: None,
            error: Some(format!("failed to store repo identity: {e}")),
        });
    }

    // Write name index
    if let Err(e) = kv::kv_put(&repo_name_index_key(&name), repo_id_hex.as_bytes()) {
        return ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: false,
            repo: None,
            error: Some(format!("failed to store name index: {e}")),
        });
    }

    let info = identity_to_info(&repo_id_hex, &signed);
    ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
        is_success: true,
        repo: Some(info),
        error: None,
    })
}

pub fn handle_get_repo(repo_id: String) -> ClientRpcResponse {
    match read_repo_identity(&repo_id) {
        Some(signed) => {
            let info = identity_to_info(&repo_id, &signed);
            ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: true,
                repo: Some(info),
                error: None,
            })
        }
        None => ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: true,
            repo: None,
            error: None,
        }),
    }
}

pub fn handle_list_repos(limit: Option<u32>, offset: Option<u32>) -> ClientRpcResponse {
    let scan_limit = limit.unwrap_or(MAX_LIST_REPOS).min(MAX_LIST_REPOS);
    let skip = offset.unwrap_or(0);

    // Scan all repo identity keys
    // Tiger Style: Use saturating_add to avoid overflow
    let entries = kv::kv_scan(REPO_IDENTITY_PREFIX, scan_limit.saturating_add(skip)).unwrap_or_default();

    let mut repos = Vec::new();
    let mut skipped = 0u32;
    let mut collected = 0u32;

    for (key, value) in &entries {
        // Only match keys ending with ":identity"
        if !key.ends_with(REPO_IDENTITY_SUFFIX) {
            continue;
        }

        if skipped < skip {
            skipped = skipped.saturating_add(1);
            continue;
        }

        let signed: SignedObject<RepoIdentity> = match serde_json::from_slice(value) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Extract repo_id from key: "forge:repos:{id}:identity"
        let repo_id_hex = key
            .strip_prefix(REPO_IDENTITY_PREFIX)
            .and_then(|s| s.strip_suffix(REPO_IDENTITY_SUFFIX))
            .unwrap_or("");

        repos.push(identity_to_info(repo_id_hex, &signed));
        collected = collected.saturating_add(1);

        if collected >= scan_limit {
            break;
        }
    }

    // Tiger Style: Use u32::try_from to avoid truncating cast from usize
    let count = u32::try_from(repos.len()).unwrap_or(u32::MAX);
    ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
        is_success: true,
        repos,
        count,
        error: None,
    })
}
