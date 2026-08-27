const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagReferenceError {
    UnsupportedAlgorithm,
    WrongDigestLength,
    InvalidDigestSpelling,
    InvalidDomain,
}

macro_rules! digest_reference {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DagReferenceError> {
                let value = value.into();
                validate_digest(&value)?;
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

digest_reference!(DagNodeRef);
digest_reference!(DagRootRef);
digest_reference!(DagSchemaRef);
digest_reference!(DagContentRef);
digest_reference!(DagPlanRef);
digest_reference!(DagEpochRef);
digest_reference!(DagPolicyRef);
digest_reference!(DagReceiptRef);

impl DagPlanRef {
    pub(super) fn generated(hash: blake3::Hash) -> Self {
        Self(format!("blake3:{hash}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DagPeerId(String);

impl DagPeerId {
    pub fn new(value: impl Into<String>) -> Result<Self, DagReferenceError> {
        let value = value.into();
        if value.is_empty() || value.len() > super::MAX_DAG_DOMAIN_BYTES || value.chars().any(char::is_control) {
            return Err(DagReferenceError::InvalidDomain);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_digest(value: &str) -> Result<(), DagReferenceError> {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return Err(DagReferenceError::UnsupportedAlgorithm);
    };
    if hex.len() != BLAKE3_HEX_LENGTH {
        return Err(DagReferenceError::WrongDigestLength);
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(DagReferenceError::InvalidDigestSpelling);
    }
    Ok(())
}
