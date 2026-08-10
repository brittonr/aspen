use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use super::invalid;

const MAX_NODE_STATE_COMPONENTS: usize = 32;
const MAX_NODE_STATE_PATH_BYTES: usize = 4_096;
const MAX_REASONABLE_NODE_STATE_BOUND: usize = 1_000_000;

const _: () = assert!(MAX_NODE_STATE_COMPONENTS > 0);
const _: () = assert!(MAX_NODE_STATE_COMPONENTS <= MAX_REASONABLE_NODE_STATE_BOUND);
const _: () = assert!(MAX_NODE_STATE_PATH_BYTES > 0);
const _: () = assert!(MAX_NODE_STATE_PATH_BYTES <= MAX_REASONABLE_NODE_STATE_BOUND);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeStatePath {
    relative: PathBuf,
}

impl NodeStatePath {
    pub fn parse(value: &str) -> crate::error::Result<Self> {
        validate_node_state_locator(value)?;
        let mut relative = PathBuf::new();
        let mut component_count = 0usize;
        for component in Path::new(value).components() {
            match component {
                Component::Normal(value) => {
                    component_count = checked_component_count(component_count)?;
                    relative.push(value);
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(invalid(format!("node state path {value} cannot contain parent traversal")));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(invalid(format!("node state path {value} must be relative")));
                }
            }
        }
        if relative.as_os_str().is_empty() {
            return Err(invalid("node state path cannot be empty"));
        }
        Ok(Self { relative })
    }

    pub fn join(&self, suffix: &str) -> crate::error::Result<Self> {
        let suffix = Self::parse(suffix)?;
        let component_count = self
            .relative
            .components()
            .count()
            .checked_add(suffix.relative.components().count())
            .ok_or_else(|| invalid("node state path component count overflow"))?;
        if component_count > MAX_NODE_STATE_COMPONENTS {
            return Err(invalid(format!(
                "node state path component count {component_count} exceeds maximum {MAX_NODE_STATE_COMPONENTS}"
            )));
        }
        let relative = self.relative.join(suffix.relative);
        validate_path_bytes(&relative)?;
        Ok(Self { relative })
    }

    pub fn join_segment(&self, segment: &str) -> crate::error::Result<Self> {
        let suffix = Self::parse(segment)?;
        if suffix.relative.components().count() != 1 {
            return Err(invalid(format!("node state segment {segment} must contain exactly one component")));
        }
        self.join(segment)
    }

    pub fn as_path(&self) -> &Path {
        &self.relative
    }

    pub fn display(&self) -> String {
        self.relative.to_string_lossy().into_owned()
    }

    pub(super) fn into_path_buf(self) -> PathBuf {
        self.relative
    }
}

pub(super) fn join_scope(scope: &Path, suffix: &NodeStatePath) -> crate::error::Result<PathBuf> {
    let component_count = scope
        .components()
        .count()
        .checked_add(suffix.as_path().components().count())
        .ok_or_else(|| invalid("node state namespace view component count overflow"))?;
    if component_count > MAX_NODE_STATE_COMPONENTS {
        return Err(invalid(format!(
            "node state namespace view component count {component_count} exceeds maximum {MAX_NODE_STATE_COMPONENTS}"
        )));
    }
    let joined = scope.join(suffix.as_path());
    validate_path_bytes(&joined)?;
    Ok(joined)
}

fn validate_node_state_locator(value: &str) -> crate::error::Result<()> {
    if value.is_empty() {
        return Err(invalid("node state path cannot be empty"));
    }
    if value.len() > MAX_NODE_STATE_PATH_BYTES {
        return Err(invalid(format!(
            "node state path length {} exceeds maximum {MAX_NODE_STATE_PATH_BYTES}",
            value.len()
        )));
    }
    if has_platform_prefix(value) {
        return Err(invalid(format!("platform-prefixed node state path {value} is not relative authority")));
    }
    if value.contains("://")
        || value.starts_with("iroh:")
        || value.starts_with("http:")
        || value.starts_with("https:")
        || value.starts_with("blake3:")
    {
        return Err(invalid(format!("remote or content locator {value} cannot become node state authority")));
    }
    Ok(())
}

fn validate_path_bytes(path: &Path) -> crate::error::Result<()> {
    let bytes = path.to_string_lossy().len();
    if bytes > MAX_NODE_STATE_PATH_BYTES {
        Err(invalid(format!("node state path length {bytes} exceeds maximum {MAX_NODE_STATE_PATH_BYTES}")))
    } else {
        Ok(())
    }
}

fn has_platform_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    let has_drive_prefix = bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
    has_drive_prefix || value.starts_with("\\\\") || value.contains('\\')
}

fn checked_component_count(count: usize) -> crate::error::Result<usize> {
    let next = count.checked_add(1).ok_or_else(|| invalid("node state path component count overflow"))?;
    if next > MAX_NODE_STATE_COMPONENTS {
        Err(invalid(format!(
            "node state path component count {next} exceeds maximum {MAX_NODE_STATE_COMPONENTS}"
        )))
    } else {
        Ok(next)
    }
}
