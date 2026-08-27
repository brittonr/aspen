//! Capability-rooted bundle member access.

use std::io::Read;

const READ_BOUND_SENTINEL_BYTES: usize = 1;

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
    fn read_leaf(&self, leaf: &str, maximum_bytes: usize) -> Result<Vec<u8>, crate::executable_extent::SourceError> {
        validate_leaf(leaf)?;
        let file = self.root.open(leaf).map_err(crate::executable_extent::SourceError::Io)?;
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
