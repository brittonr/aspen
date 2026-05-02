//! Helper functions for CI handlers.

/// CI config file path within a repository.
#[cfg(all(feature = "forge", feature = "blob"))]
pub const CI_CONFIG_PATH: &[&str] = &[".aspen", "ci.ncl"];

/// Walk a tree recursively to find a file by path components.
///
/// Returns the file content as bytes if found, None if not found.
/// Logs diagnostic info when the file is not found so callers can
/// determine which path component failed.
#[cfg(all(feature = "forge", feature = "blob"))]
pub async fn walk_tree_for_file<B: aspen_blob::BlobStore>(
    git: &aspen_forge::git::GitBlobStore<B>,
    root_tree_hash: &blake3::Hash,
    path: &[&str],
) -> Result<Option<Vec<u8>>, anyhow::Error> {
    use tracing::debug;

    if path.is_empty() {
        return Ok(None);
    }

    let full_path = path.join("/");
    debug!(
        root_tree = %root_tree_hash,
        path = %full_path,
        "tree-walk: starting search"
    );

    let mut current_hash = *root_tree_hash;

    // Walk through each path component
    for (i, part) in path.iter().enumerate() {
        let tree = match git.get_tree(&current_hash).await {
            Ok(t) => t,
            Err(e) => {
                debug!(
                    tree_hash = %current_hash,
                    component = %part,
                    depth = i,
                    error = %e,
                    "tree-walk: failed to load tree object"
                );
                return Err(e.into());
            }
        };

        let entry_names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();

        // Find entry with matching name
        let entry = match tree.entries.iter().find(|e| e.name == *part) {
            Some(e) => e,
            None => {
                debug!(
                    tree_hash = %current_hash,
                    looking_for = %part,
                    depth = i,
                    entry_count = tree.entries.len(),
                    entries = ?entry_names,
                    "tree-walk: component '{}' not found in tree (has {} entries)",
                    part,
                    tree.entries.len()
                );
                return Ok(None);
            }
        };

        if i == path.len() - 1 {
            // Last component - should be a file
            if entry.is_file() {
                let content = git.get_blob(&entry.hash()).await?;
                debug!(
                    path = %full_path,
                    blob_hash = %entry.hash(),
                    size_bytes = content.len(),
                    "tree-walk: found file"
                );
                return Ok(Some(content));
            } else {
                debug!(
                    path = %full_path,
                    entry_hash = %entry.hash(),
                    "tree-walk: expected file at '{}' but found directory",
                    part
                );
                return Ok(None);
            }
        } else {
            // Intermediate component - should be a directory
            if entry.is_directory() {
                debug!(
                    component = %part,
                    subtree_hash = %entry.hash(),
                    depth = i,
                    "tree-walk: descending into subtree"
                );
                current_hash = entry.hash();
            } else {
                debug!(
                    path = %full_path,
                    component = %part,
                    depth = i,
                    "tree-walk: expected directory at '{}' but found file",
                    part
                );
                return Ok(None);
            }
        }
    }

    Ok(None)
}

/// Parse a commit hash from hex string to [u8; 32].
#[cfg(all(feature = "forge", feature = "blob"))]
pub fn parse_commit_hash(hex_str: &str) -> Result<[u8; 32], anyhow::Error> {
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 32 {
        anyhow::bail!("commit hash must be 32 bytes (64 hex chars), got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Convert PipelineStatus to string representation.
pub fn pipeline_status_to_string(status: &aspen_ci::orchestrator::PipelineStatus) -> String {
    status.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use aspen_ci::orchestrator::PipelineStatus;

    use super::pipeline_status_to_string;

    #[test]
    fn pipeline_status_to_string_delegates_to_ci_status() {
        assert_eq!(pipeline_status_to_string(&PipelineStatus::CheckoutFailed), "checkout_failed");
        assert_eq!(pipeline_status_to_string(&PipelineStatus::Success), "success");
    }
}
