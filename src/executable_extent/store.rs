//! Capability-rooted bundle member access.

use std::io::Read;

use cap_fs_ext::OpenOptionsFollowExt;

const READ_BOUND_SENTINEL_BYTES: usize = 1;
/// Members another account could rewrite are never mapped: world-writable bits deny.
#[cfg(unix)]
const WORLD_WRITABLE_BIT: u32 = 0o002;

/// A bundle source rooted in an already authorized directory capability.
pub struct CapabilityBundleSource<'a> {
    root: &'a cap_std::fs::Dir,
}

impl<'a> CapabilityBundleSource<'a> {
    /// Creates a source without opening or discovering a root path.
    #[must_use]
    pub const fn new(root: &'a cap_std::fs::Dir) -> Self {
        Self { root }
    }
}

impl crate::executable_extent::BundleSource for CapabilityBundleSource<'_> {
    // r[impl molten.runtime_spine.admission_and_capability_hardening]
    fn read_leaf(&self, leaf: &str, maximum_bytes: usize) -> Result<Vec<u8>, crate::executable_extent::SourceError> {
        validate_leaf(leaf)?;
        // Open without following a member symlink, then validate the opened handle itself.
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
        let file = self.root.open_with(leaf, &options).map_err(crate::executable_extent::SourceError::Io)?;
        validate_member(&file.metadata().map_err(crate::executable_extent::SourceError::Io)?)?;
        let read_limit = maximum_bytes
            .checked_add(READ_BOUND_SENTINEL_BYTES)
            .ok_or(crate::executable_extent::SourceError::BoundExceeded)?;
        let mut bytes = Vec::with_capacity(maximum_bytes);
        file.take(u64::try_from(read_limit).map_err(|_error| crate::executable_extent::SourceError::BoundExceeded)?)
            .read_to_end(&mut bytes)
            .map_err(crate::executable_extent::SourceError::Io)?;
        if bytes.len() > maximum_bytes {
            return Err(crate::executable_extent::SourceError::BoundExceeded);
        }
        Ok(bytes)
    }
}

fn validate_leaf(leaf: &str) -> Result<(), crate::executable_extent::SourceError> {
    const MAX_LEAF_BYTES: usize = 255;
    let mut components = std::path::Path::new(leaf).components();
    let first = components.next();
    let is_only_normal = matches!(first, Some(std::path::Component::Normal(_))) && components.next().is_none();
    if leaf.is_empty() || leaf.len() > MAX_LEAF_BYTES || !is_only_normal {
        return Err(crate::executable_extent::SourceError::InvalidLeaf);
    }
    Ok(())
}

fn validate_member(metadata: &cap_std::fs::Metadata) -> Result<(), crate::executable_extent::SourceError> {
    if !metadata.is_file() {
        return Err(crate::executable_extent::SourceError::UnsafeMember);
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & WORLD_WRITABLE_BIT != 0 {
            return Err(crate::executable_extent::SourceError::UnsafeMember);
        }
    }
    Ok(())
}
