const MAX_NODE_STATE_COMPONENTS: usize = 32;
const MAX_NODE_STATE_PATH_BYTES: usize = 4_096;
const MAX_REASONABLE_NODE_STATE_BOUND: usize = 1_000_000;

const _: () = assert!(MAX_NODE_STATE_COMPONENTS > 0);
const _: () = assert!(MAX_NODE_STATE_COMPONENTS <= MAX_REASONABLE_NODE_STATE_BOUND);
const _: () = assert!(MAX_NODE_STATE_PATH_BYTES > 0);
const _: () = assert!(MAX_NODE_STATE_PATH_BYTES <= MAX_REASONABLE_NODE_STATE_BOUND);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeStatePath {
    relative: std::path::PathBuf,
}

impl NodeStatePath {
    pub fn parse(value: &str) -> crate::error::Result<Self> {
        validate_node_state_locator(value)?;
        let mut relative = std::path::PathBuf::new();
        let mut component_count = 0usize;
        for component in std::path::Path::new(value).components() {
            match component {
                std::path::Component::Normal(value) => {
                    component_count = checked_component_count(component_count)?;
                    relative.push(value);
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    return Err(super::invalid(format!("node state path {value} cannot contain parent traversal")));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(super::invalid(format!("node state path {value} must be relative")));
                }
            }
        }
        if relative.as_os_str().is_empty() {
            return Err(super::invalid("node state path cannot be empty"));
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
            .ok_or_else(|| super::invalid("node state path component count overflow"))?;
        if component_count > MAX_NODE_STATE_COMPONENTS {
            return Err(super::invalid(format!(
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
            return Err(super::invalid(format!("node state segment {segment} must contain exactly one component")));
        }
        self.join(segment)
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.relative
    }

    pub fn display(&self) -> String {
        self.relative.to_string_lossy().into_owned()
    }

    pub(super) fn into_path_buf(self) -> std::path::PathBuf {
        self.relative
    }
}

pub(super) fn join_scope(scope: &std::path::Path, suffix: &NodeStatePath) -> crate::error::Result<std::path::PathBuf> {
    let component_count = scope
        .components()
        .count()
        .checked_add(suffix.as_path().components().count())
        .ok_or_else(|| super::invalid("node state namespace view component count overflow"))?;
    if component_count > MAX_NODE_STATE_COMPONENTS {
        return Err(super::invalid(format!(
            "node state namespace view component count {component_count} exceeds maximum {MAX_NODE_STATE_COMPONENTS}"
        )));
    }
    let joined = scope.join(suffix.as_path());
    validate_path_bytes(&joined)?;
    Ok(joined)
}

fn validate_node_state_locator(value: &str) -> crate::error::Result<()> {
    if value.is_empty() {
        return Err(super::invalid("node state path cannot be empty"));
    }
    if value.len() > MAX_NODE_STATE_PATH_BYTES {
        return Err(super::invalid(format!(
            "node state path length {} exceeds maximum {MAX_NODE_STATE_PATH_BYTES}",
            value.len()
        )));
    }
    if has_platform_prefix(value) {
        return Err(super::invalid(format!("platform-prefixed node state path {value} is not relative authority")));
    }
    if crate::locator::is_remote(value) {
        return Err(super::invalid(format!("remote or content locator {value} cannot become node state authority")));
    }
    Ok(())
}

fn validate_path_bytes(path: &std::path::Path) -> crate::error::Result<()> {
    let bytes = path.to_string_lossy().len();
    if bytes > MAX_NODE_STATE_PATH_BYTES {
        Err(super::invalid(format!(
            "node state path length {bytes} exceeds maximum {MAX_NODE_STATE_PATH_BYTES}"
        )))
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
    let next = count.checked_add(1).ok_or_else(|| super::invalid("node state path component count overflow"))?;
    if next > MAX_NODE_STATE_COMPONENTS {
        Err(super::invalid(format!(
            "node state path component count {next} exceeds maximum {MAX_NODE_STATE_COMPONENTS}"
        )))
    } else {
        Ok(next)
    }
}
