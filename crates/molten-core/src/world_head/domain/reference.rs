use super::MAX_WORLD_BRANCH_ID_BYTES;

macro_rules! digest_reference {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, WorldHeadReferenceError> {
                let value = value.into();
                validate_digest_reference(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

digest_reference!(WorldHeadPolicyRef);
digest_reference!(WorldHeadClaimRef);
digest_reference!(WorldHeadStatementRef);
digest_reference!(WorldHeadAuthorityRef);
digest_reference!(WorldHeadCurrentnessRef);
digest_reference!(WorldHeadAuthenticationDecisionRef);
digest_reference!(WorldHeadTransitionRef);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldBranchId(String);

impl WorldBranchId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldHeadReferenceError> {
        let value = value.into();
        validate_branch_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorldBranchId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldHeadReferenceError {
    Empty,
    TooLong,
    InvalidCharacter,
    InvalidDigest,
    UnsafeSegment,
}

impl std::fmt::Display for WorldHeadReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorldHeadReferenceError {}

fn validate_digest_reference(value: &str) -> Result<(), WorldHeadReferenceError> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let Some(digest) = value.strip_prefix(BLAKE3_PREFIX) else {
        return Err(WorldHeadReferenceError::InvalidDigest);
    };
    if digest.len() != BLAKE3_HEX_LENGTH {
        return Err(WorldHeadReferenceError::InvalidDigest);
    }
    if !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(WorldHeadReferenceError::InvalidDigest);
    }
    Ok(())
}

fn validate_branch_id(value: &str) -> Result<(), WorldHeadReferenceError> {
    if value.is_empty() {
        return Err(WorldHeadReferenceError::Empty);
    }
    if value.len() > MAX_WORLD_BRANCH_ID_BYTES {
        return Err(WorldHeadReferenceError::TooLong);
    }
    if value.starts_with('/')
        || value.ends_with('/')
        || value.split('/').any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(WorldHeadReferenceError::UnsafeSegment);
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
    {
        return Err(WorldHeadReferenceError::InvalidCharacter);
    }
    Ok(())
}
