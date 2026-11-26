//! Shape checks for canonical content refs (`blake3:<64 lowercase hex>`) and peer identifiers.

pub const BLAKE3_REF_PREFIX: &str = "blake3:";
pub const BLAKE3_HEX_LENGTH: usize = 64;
/// Node ids, voter ids, and port implementation profiles are at most this many bytes.
pub const MAX_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentRefIssue {
    MissingPrefix { value: String },
    HexLength { value: String },
    HexCase { value: String },
}

impl core::fmt::Display for ContentRefIssue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingPrefix { value } => {
                write!(formatter, "content ref must start with {BLAKE3_REF_PREFIX}, got {value}")
            }
            Self::HexLength { value } => {
                write!(formatter, "content ref must be {BLAKE3_REF_PREFIX}<64 lowercase hex chars>, got {value}")
            }
            Self::HexCase { value } => write!(formatter, "content ref must use lowercase hex chars, got {value}"),
        }
    }
}

impl core::error::Error for ContentRefIssue {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierIssue {
    Length,
    Characters,
}

impl core::fmt::Display for IdentifierIssue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Length => write!(formatter, "must be non-empty and at most {MAX_IDENTIFIER_BYTES} bytes"),
            Self::Characters => formatter.write_str("contains unsupported characters"),
        }
    }
}

impl core::error::Error for IdentifierIssue {}

/// Check the shape of a canonical content ref.
pub fn check_content_ref(value: &str) -> Result<(), ContentRefIssue> {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(ContentRefIssue::MissingPrefix {
            value: value.to_owned(),
        });
    };
    if hex.len() != BLAKE3_HEX_LENGTH {
        return Err(ContentRefIssue::HexLength {
            value: value.to_owned(),
        });
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(ContentRefIssue::HexCase {
            value: value.to_owned(),
        });
    }
    Ok(())
}

/// A content ref check whose refusal does not name the field.
pub fn validate_content_ref(value: &str) -> crate::denial::Result<()> {
    check_content_ref(value).map_err(|issue| crate::denial::ReplicaError::ContentRef { label: None, issue })
}

/// A content ref check whose refusal names the field as `label`.
pub fn validate_labelled_content_ref(value: &str, label: &'static str) -> crate::denial::Result<()> {
    check_content_ref(value).map_err(|issue| crate::denial::ReplicaError::ContentRef {
        label: Some(label),
        issue,
    })
}

/// Identifiers are 1 to [`MAX_IDENTIFIER_BYTES`] bytes of ASCII alphanumerics, `-`, `_`, `.`, `:`.
pub fn validate_identifier(value: &str, label: &'static str) -> crate::denial::Result<()> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
        return Err(crate::denial::ReplicaError::Identifier {
            label,
            issue: IdentifierIssue::Length,
        });
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(crate::denial::ReplicaError::Identifier {
            label,
            issue: IdentifierIssue::Characters,
        });
    }
    Ok(())
}
