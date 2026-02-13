//! Ref operations (branches, tags).

use aspen_client_api::ClientRpcResponse;

use super::ForgeNodeRef;

pub(crate) async fn handle_get_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.get(&repo_id, &ref_name).await {
        Ok(Some(hash)) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: hash.to_hex().to_string(),
            }),
            previous_hash: None,
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_set_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    // Get previous value for response
    let previous = forge_node.refs.get(&repo_id, &ref_name).await.ok().flatten();

    match forge_node.refs.set(&repo_id, &ref_name, hash).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: hash.to_hex().to_string(),
            }),
            previous_hash: previous.map(|h| h.to_hex().to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_delete_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Get previous value
    let previous = forge_node.refs.get(&repo_id, &ref_name).await.ok().flatten();

    match forge_node.refs.delete(&repo_id, &ref_name).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: previous.is_some(),
            ref_info: None,
            previous_hash: previous.map(|h| h.to_hex().to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_cas_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
    expected: Option<String>,
    new_hash: String,
    signer: Option<String>,
    signature: Option<String>,
    timestamp_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;
    use aspen_forge::refs::DelegateVerifier;
    use aspen_forge::refs::SignedRefUpdate;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let expected_hash = match &expected {
        Some(h) => match blake3::Hash::from_hex(h) {
            Ok(hash) => Some(hash),
            Err(e) => {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Invalid expected hash: {}", e)),
                }));
            }
        },
        None => None,
    };

    let new_hash = match blake3::Hash::from_hex(&new_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid new hash: {}", e)),
            }));
        }
    };

    // For canonical refs (tags, default branch), verify signature if provided
    // If signature is provided, verify it; if not provided for canonical refs,
    // currently we allow it for backwards compatibility during migration
    if let (Some(signer_hex), Some(sig_hex), Some(ts)) = (&signer, &signature, timestamp_ms) {
        // Get repository identity for delegate verification
        let identity = match forge_node.get_repo(&repo_id).await {
            Ok(id) => id,
            Err(e) => {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Failed to get repository: {}", e)),
                }));
            }
        };

        // Check if this is a canonical ref that requires delegate authorization
        if DelegateVerifier::is_canonical_ref(&ref_name, &identity.default_branch) {
            // Parse signer public key
            let signer_bytes = match hex::decode(signer_hex) {
                Ok(b) if b.len() == 32 => b,
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("Invalid signer key: must be 32 bytes hex".to_string()),
                    }));
                }
            };
            let mut signer_arr = [0u8; 32];
            signer_arr.copy_from_slice(&signer_bytes);
            let signer_key = match iroh::PublicKey::from_bytes(&signer_arr) {
                Ok(k) => k,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("Invalid signer key: {}", e)),
                    }));
                }
            };

            // Parse signature
            let sig_bytes = match hex::decode(sig_hex) {
                Ok(b) if b.len() == 64 => b,
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("Invalid signature: must be 64 bytes hex".to_string()),
                    }));
                }
            };
            let mut sig_arr = [0u8; 64];
            sig_arr.copy_from_slice(&sig_bytes);
            let signature = iroh::Signature::from_bytes(&sig_arr);

            // Build SignedRefUpdate for verification
            let signed_update = SignedRefUpdate {
                repo_id,
                ref_name: ref_name.clone(),
                new_hash: *new_hash.as_bytes(),
                old_hash: expected_hash.map(|h| *h.as_bytes()),
                signer: signer_key,
                signature,
                timestamp_ms: ts,
            };

            // Verify the signature and delegate authorization
            if let Err(e) = DelegateVerifier::verify_update(&signed_update, &identity) {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Authorization failed: {}", e)),
                }));
            }
        }
    }

    match forge_node.refs.compare_and_set(&repo_id, &ref_name, expected_hash, new_hash).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: new_hash.to_hex().to_string(),
            }),
            previous_hash: expected,
            error: None,
        })),
        Err(e) => {
            // CAS failure or other error
            Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_list_branches(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.list_branches(&repo_id).await {
        Ok(branches) => {
            let refs: Vec<ForgeRefInfo> = branches
                .iter()
                .map(|(name, hash)| ForgeRefInfo {
                    name: name.clone(),
                    hash: hash.to_hex().to_string(),
                })
                .collect();
            let count = refs.len() as u32;
            Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: true,
                refs,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            success: false,
            refs: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_list_tags(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.list_tags(&repo_id).await {
        Ok(tags) => {
            let refs: Vec<ForgeRefInfo> = tags
                .iter()
                .map(|(name, hash)| ForgeRefInfo {
                    name: name.clone(),
                    hash: hash.to_hex().to_string(),
                })
                .collect();
            let count = refs.len() as u32;
            Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: true,
                refs,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            success: false,
            refs: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}
