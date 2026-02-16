//! Artifact operations: list_artifacts, get_artifact.

use std::collections::HashMap;
#[cfg(feature = "blob")]
use std::str::FromStr;

use aspen_client_api::CiArtifactInfo;
use aspen_client_api::CiGetArtifactResponse;
use aspen_client_api::CiListArtifactsResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use tracing::info;
use tracing::warn;

/// Internal artifact metadata structure stored in KV.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ArtifactMetadata {
    /// Blob hash in the distributed store.
    pub blob_hash: String,
    /// Artifact name (e.g., store path for Nix builds).
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Content type (e.g., "application/x-nix-nar").
    pub content_type: String,
    /// When the artifact was created (ISO 8601).
    pub created_at: String,
    /// Optional run_id for filtering.
    pub run_id: Option<String>,
    /// Additional metadata.
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// Handle CiListArtifacts request.
///
/// Lists artifacts produced by a CI job. Artifacts are stored in the KV store
/// with metadata and blob hashes for the actual content in the blob store.
pub(crate) async fn handle_list_artifacts(
    ctx: &ClientProtocolContext,
    job_id: String,
    run_id: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_core::ScanRequest;

    info!(%job_id, ?run_id, "listing CI artifacts");

    // Scan for artifacts associated with this job
    // Artifact metadata is stored under: _ci:artifacts:{job_id}:{artifact_name}
    let prefix = format!("_ci:artifacts:{}:", job_id);

    let scan_result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix,
            limit: Some(100), // Tiger Style: bounded results
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(job_id = %job_id, error = %e, "failed to scan artifacts");
            return Ok(ClientRpcResponse::CiListArtifactsResult(CiListArtifactsResponse {
                is_success: false,
                artifacts: vec![],
                error: Some(format!("Failed to list artifacts: {}", e)),
            }));
        }
    };

    // Parse artifact metadata from KV entries
    let mut artifacts = Vec::new();
    for entry in entries {
        // Try to parse as artifact metadata
        if let Ok(metadata) = serde_json::from_str::<ArtifactMetadata>(&entry.value) {
            // Filter by run_id if specified
            if let Some(ref filter_run_id) = run_id
                && metadata.run_id.as_deref() != Some(filter_run_id.as_str())
            {
                continue;
            }

            artifacts.push(CiArtifactInfo {
                blob_hash: metadata.blob_hash,
                name: metadata.name,
                size_bytes: metadata.size_bytes,
                content_type: metadata.content_type,
                created_at: metadata.created_at,
                metadata: metadata.extra,
            });
        }
    }

    info!(job_id = %job_id, count = artifacts.len(), "found artifacts");

    Ok(ClientRpcResponse::CiListArtifactsResult(CiListArtifactsResponse {
        is_success: true,
        artifacts,
        error: None,
    }))
}

/// Handle CiGetArtifact request.
///
/// Returns artifact metadata and a blob ticket for downloading.
pub(crate) async fn handle_get_artifact(
    ctx: &ClientProtocolContext,
    blob_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    info!(%blob_hash, "getting CI artifact");

    // Look up artifact metadata by blob hash
    // We scan for any artifact with this blob_hash since we don't know the job_id
    let prefix = "_ci:artifacts:".to_string();

    let scan_result = ctx
        .kv_store
        .scan(aspen_core::ScanRequest {
            prefix,
            limit: Some(1000), // Tiger Style: bounded search
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(blob_hash = %blob_hash, error = %e, "failed to scan for artifact");
            return Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
                is_success: false,
                artifact: None,
                blob_ticket: None,
                error: Some(format!("Failed to find artifact: {}", e)),
            }));
        }
    };

    // Find the artifact with matching blob_hash
    let mut found_artifact = None;
    for entry in entries {
        if let Ok(metadata) = serde_json::from_str::<ArtifactMetadata>(&entry.value)
            && metadata.blob_hash == blob_hash
        {
            found_artifact = Some(CiArtifactInfo {
                blob_hash: metadata.blob_hash,
                name: metadata.name,
                size_bytes: metadata.size_bytes,
                content_type: metadata.content_type,
                created_at: metadata.created_at,
                metadata: metadata.extra,
            });
            break;
        }
    }

    let Some(artifact) = found_artifact else {
        return Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
            is_success: false,
            artifact: None,
            blob_ticket: None,
            error: Some(format!("Artifact not found: {}", blob_hash)),
        }));
    };

    // Generate blob ticket for download
    #[cfg(feature = "blob")]
    let blob_ticket = if let Some(blob_store) = &ctx.blob_store {
        use aspen_blob::prelude::*;
        // Parse blob hash and generate ticket
        match iroh_blobs::Hash::from_str(&blob_hash) {
            Ok(hash) => {
                // Get blob ticket from the blob store
                match blob_store.ticket(&hash).await {
                    Ok(ticket) => Some(ticket.to_string()),
                    Err(e) => {
                        warn!(blob_hash = %blob_hash, error = %e, "failed to generate blob ticket");
                        None
                    }
                }
            }
            Err(e) => {
                warn!(blob_hash = %blob_hash, error = %e, "invalid blob hash format");
                None
            }
        }
    } else {
        None
    };

    #[cfg(not(feature = "blob"))]
    let blob_ticket: Option<String> = None;

    info!(blob_hash = %blob_hash, has_ticket = blob_ticket.is_some(), "artifact found");

    Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
        is_success: true,
        artifact: Some(artifact),
        blob_ticket,
        error: None,
    }))
}
