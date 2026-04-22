//! Path validation helper functions for the directory layer.

use super::DirectoryError;
use crate::constants::directory::DIR_FIELD_CREATED_AT_MS;
use crate::constants::directory::DIR_FIELD_LAYER;
use crate::constants::directory::DIR_FIELD_PREFIX;
use crate::constants::directory::MAX_DIRECTORY_DEPTH;
use crate::constants::directory::MAX_PATH_COMPONENT_LENGTH_BYTES;

/// Validate a directory path.
pub(super) fn validate_path(path: &[&str]) -> Result<(), DirectoryError> {
    // Path cannot be empty
    if path.is_empty() {
        return Err(DirectoryError::InvalidPath {
            component: String::new(),
            reason: "path cannot be empty".to_string(),
        });
    }

    validate_path_components(path)
}

/// Validate a directory path, allowing empty for list operations.
pub(super) fn validate_path_allow_empty(path: &[&str]) -> Result<(), DirectoryError> {
    if path.is_empty() {
        return Ok(());
    }
    validate_path_components(path)
}

/// Validate path components.
fn validate_path_components(path: &[&str]) -> Result<(), DirectoryError> {
    // Check depth
    if path.len() as u32 > MAX_DIRECTORY_DEPTH {
        return Err(DirectoryError::PathTooDeep {
            depth: path.len() as u32,
            max: MAX_DIRECTORY_DEPTH,
        });
    }

    // Validate each component
    for component in path {
        if component.is_empty() {
            return Err(DirectoryError::InvalidPath {
                component: component.to_string(),
                reason: "path component cannot be empty".to_string(),
            });
        }

        if component.len() as u32 > MAX_PATH_COMPONENT_LENGTH_BYTES {
            return Err(DirectoryError::InvalidPath {
                component: component.to_string(),
                reason: format!(
                    "component length {} exceeds maximum {}",
                    component.len(),
                    MAX_PATH_COMPONENT_LENGTH_BYTES
                ),
            });
        }
    }

    Ok(())
}

/// Extract subdirectory name from a metadata key.
pub(super) fn extract_subdirectory_name(key: &str, prefix: &str, parent_depth: usize) -> Option<String> {
    // Remove the prefix
    let suffix = key.strip_prefix(prefix)?;

    // The suffix should start with a tuple-encoded string
    // We need to find the first path component after the parent path
    // This is a simplified extraction - in production you'd use proper tuple parsing

    // For now, we'll scan for metadata field names and extract the component before them
    let fields = [DIR_FIELD_LAYER, DIR_FIELD_PREFIX, DIR_FIELD_CREATED_AT_MS];

    for field in fields {
        if suffix.contains(field) {
            // Find the component before this field
            // This is imprecise but works for ASCII paths
            let parts: Vec<&str> = suffix.split('\x00').collect();
            if parts.len() > parent_depth {
                // Extract the string content (skip type byte)
                let component = parts.first()?;
                // Remove type prefix (0x02 for string)
                if component.len() > 1 {
                    return Some(component[1..].to_string());
                }
            }
        }
    }

    None
}
