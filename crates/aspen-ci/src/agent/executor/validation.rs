//! Working directory validation for the CI agent executor.

use std::path::Path;

use super::Executor;
use crate::agent::error;
use crate::agent::error::Result;

impl Executor {
    /// Validate that working directory is safe.
    ///
    /// Canonicalizes the path to resolve symlinks and `..` components
    /// before checking the prefix, preventing traversal attacks like
    /// `/workspace/../../etc/shadow`.
    pub(crate) fn validate_working_dir(&self, path: &Path) -> Result<()> {
        // Check it exists first (canonicalize requires the path to exist)
        if !path.exists() {
            return error::InvalidWorkingDirSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        // Canonicalize to resolve symlinks and .. traversal
        let canonical = path.canonicalize().map_err(|_| error::AgentError::InvalidWorkingDir {
            path: path.display().to_string(),
        })?;

        // Must be under the configured workspace root, or under /tmp/ci-workspace-
        // (the tmpfs fallback used when virtiofs has I/O issues with nix)
        let canonical_workspace = self.workspace_root.canonicalize().unwrap_or_else(|_| self.workspace_root.clone());
        let is_under_workspace = canonical.starts_with(&canonical_workspace);
        let is_tmpfs_workspace = canonical.starts_with("/tmp/ci-workspace-");
        if !is_under_workspace && !is_tmpfs_workspace {
            return error::WorkingDirNotUnderWorkspaceSnafu {
                path: canonical.display().to_string(),
            }
            .fail();
        }

        Ok(())
    }
}
