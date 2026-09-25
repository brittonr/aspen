type Failure = crate::error::Failure;
type Result<T> = crate::error::Result<T>;

const MAX_LOCAL_STORE_COMPONENTS: usize = 32;

const _: () = assert!(MAX_LOCAL_STORE_COMPONENTS <= 1_000);

/// Closed storage-directory classification; additions require an explicit mapping.
#[cfg_attr(dylint_lib = "octet", octet::sealed_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Artifact,
    Chunk,
    Retention,
    Dataspace,
    Exchange,
    Ledger,
    Delivery,
    Durable,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Chunk => "chunk",
            Self::Retention => "retention",
            Self::Dataspace => "dataspace",
            Self::Exchange => "exchange",
            Self::Ledger => "ledger",
            Self::Delivery => "delivery",
            Self::Durable => "durable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelativeLocator {
    pub(super) relative: std::path::PathBuf,
}

impl RelativeLocator {
    pub fn parse(input: &str) -> Result<Self> {
        validate_locator(input)?;
        let path = std::path::Path::new(input);
        let mut relative = std::path::PathBuf::new();
        let mut component_count = 0usize;
        for component in path.components() {
            match component {
                std::path::Component::Normal(value) => {
                    component_count = checked_component_count(component_count)?;
                    relative.push(value);
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    return Err(Failure::invalid_harness(format!(
                        "local store path {input} cannot contain parent traversal"
                    )));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(Failure::invalid_harness(format!("local store path {input} must be relative")));
                }
            }
        }
        if relative.as_os_str().is_empty() {
            return Err(Failure::invalid_harness("local store path cannot be empty"));
        }
        Ok(Self { relative })
    }

    pub fn join(&self, suffix: &str) -> Result<Self> {
        let suffix = Self::parse(suffix)?;
        let base_count = self.relative.components().count();
        let suffix_count = suffix.relative.components().count();
        let component_count = base_count
            .checked_add(suffix_count)
            .ok_or_else(|| Failure::invalid_harness("local store path component count overflow"))?;
        if component_count > MAX_LOCAL_STORE_COMPONENTS {
            return Err(Failure::invalid_harness(format!(
                "local store path component count {component_count} exceeds maximum {MAX_LOCAL_STORE_COMPONENTS}"
            )));
        }
        Ok(Self {
            relative: self.relative.join(suffix.relative),
        })
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.relative
    }

    pub fn display(&self) -> String {
        self.relative.to_string_lossy().into_owned()
    }
}

fn validate_locator(input: &str) -> Result<()> {
    if input.is_empty() {
        return Err(Failure::invalid_harness("local store path cannot be empty"));
    }
    let bytes = input.as_bytes();
    let has_drive_prefix = matches!(bytes, [letter, b':', ..] if letter.is_ascii_alphabetic());
    if has_drive_prefix || input.starts_with("\\\\") || input.contains('\\') {
        return Err(Failure::invalid_harness(format!(
            "platform-prefixed local store path {input} is not portable relative authority"
        )));
    }
    if crate::locator::is_remote(input) {
        return Err(Failure::invalid_harness(format!(
            "remote or content locator {input} cannot be used as a local filesystem path"
        )));
    }
    Ok(())
}

fn checked_component_count(count: usize) -> Result<usize> {
    let next = count
        .checked_add(1)
        .ok_or_else(|| Failure::invalid_harness("local store path component count overflow"))?;
    if next > MAX_LOCAL_STORE_COMPONENTS {
        Err(Failure::invalid_harness(format!(
            "local store path component count {next} exceeds maximum {MAX_LOCAL_STORE_COMPONENTS}"
        )))
    } else {
        Ok(next)
    }
}
