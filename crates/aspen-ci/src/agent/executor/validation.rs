//! Working directory validation for the CI agent executor.

use std::path::Path;

use super::Executor;
use crate::agent::error;
use crate::agent::error::Result;

impl Executor {
    /// Validate that working directory is safe.
    pub(crate) fn validate_working_dir(&self, path: &Path) -> Result<()> {
        // Must be under the configured workspace root, or under /tmp/ci-workspace-
        // (the tmpfs fallback used when virtiofs has I/O issues with nix)
        let is_under_workspace = path.starts_with(&self.workspace_root);
        let is_tmpfs_workspace = path.starts_with("/tmp/ci-workspace-");
        if !is_under_workspace && !is_tmpfs_workspace {
            return error::WorkingDirNotUnderWorkspaceSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        // Check it exists
        if !path.exists() {
            return error::InvalidWorkingDirSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        Ok(())
    }
}
