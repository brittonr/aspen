//! Helper functions for CI handlers.

/// CI config file path within a repository.
#[cfg(all(feature = "forge", feature = "blob"))]
pub(crate) const CI_CONFIG_PATH: &[&str] = &[".aspen", "ci.ncl"];

/// Walk a tree recursively to find a file by path components.
///
/// Returns the file content as bytes if found, None if not found.
#[cfg(all(feature = "forge", feature = "blob"))]
pub(crate) async fn walk_tree_for_file<B: aspen_blob::BlobStore>(
    git: &aspen_forge::git::GitBlobStore<B>,
    root_tree_hash: &blake3::Hash,
    path: &[&str],
) -> Result<Option<Vec<u8>>, anyhow::Error> {
    if path.is_empty() {
        return Ok(None);
    }

    let mut current_hash = *root_tree_hash;

    // Walk through each path component
    for (i, part) in path.iter().enumerate() {
        let tree = git.get_tree(&current_hash).await?;

        // Find entry with matching name
        let entry = match tree.entries.iter().find(|e| e.name == *part) {
            Some(e) => e,
            None => return Ok(None), // Path component not found
        };

        if i == path.len() - 1 {
            // Last component - should be a file
            if entry.is_file() {
                let content = git.get_blob(&entry.hash()).await?;
                return Ok(Some(content));
            } else {
                // Expected file but found directory
                return Ok(None);
            }
        } else {
            // Intermediate component - should be a directory
            if entry.is_directory() {
                current_hash = entry.hash();
            } else {
                // Expected directory but found file
                return Ok(None);
            }
        }
    }

    Ok(None)
}

/// Parse a commit hash from hex string to [u8; 32].
#[cfg(all(feature = "forge", feature = "blob"))]
pub(crate) fn parse_commit_hash(hex_str: &str) -> Result<[u8; 32], anyhow::Error> {
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 32 {
        anyhow::bail!("commit hash must be 32 bytes (64 hex chars), got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Convert PipelineStatus to string representation.
pub(crate) fn pipeline_status_to_string(status: &aspen_ci::orchestrator::PipelineStatus) -> String {
    match status {
        aspen_ci::orchestrator::PipelineStatus::Initializing => "initializing".to_string(),
        aspen_ci::orchestrator::PipelineStatus::CheckingOut => "checking_out".to_string(),
        aspen_ci::orchestrator::PipelineStatus::CheckoutFailed => "checkout_failed".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Pending => "pending".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Running => "running".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Success => "success".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Failed => "failed".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Cancelled => "cancelled".to_string(),
    }
}
