//! Canonical BLAKE3 identity and typed content references.
//!
//! A value's identity is the plain BLAKE3 digest of its canonical packed bytes, rendered as
//! `blake3:` followed by exactly 64 lowercase hex digits. Typed derivations that must never collide
//! with a plain value identity use `domain_ref`, which keys BLAKE3 with a derivation context.

/// Name of the identity construction: BLAKE3 over canonical packed Preserves bytes.
pub const HASH_ALGORITHM: &str = "blake3-preserves-packed-v1";
pub const BLAKE3_REF_PREFIX: &str = "blake3:";
pub const BLAKE3_HEX_LEN: usize = 64;

/// Why a string is not a canonical content reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefError {
    MissingPrefix,
    /// The digest is not exactly 64 characters long.
    DigestLength,
    /// The digest contains a character other than `0-9a-f`.
    DigestCharacter,
}

impl core::fmt::Display for RefError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingPrefix => write!(formatter, "content ref must start with {BLAKE3_REF_PREFIX}"),
            Self::DigestLength => {
                write!(formatter, "content ref must be {BLAKE3_REF_PREFIX}<{BLAKE3_HEX_LEN} lowercase hex chars>")
            }
            Self::DigestCharacter => formatter.write_str("content ref must use lowercase hex chars"),
        }
    }
}

/// A validated `blake3:<64 lowercase hex>` content reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentRef(alloc::string::String);

impl ContentRef {
    pub fn parse(value: &str) -> Result<Self, RefError> {
        validate_ref(value)?;
        Ok(Self(alloc::string::String::from(value)))
    }

    /// The reference whose digest is the filename-safe `hex`.
    pub fn from_hex(hex: &str) -> Result<Self, RefError> {
        validate_digest(hex)?;
        let mut reference = alloc::string::String::with_capacity(BLAKE3_REF_PREFIX.len() + hex.len());
        reference.push_str(BLAKE3_REF_PREFIX);
        reference.push_str(hex);
        Ok(Self(reference))
    }

    pub fn from_hash(hash: blake3::Hash) -> Self {
        let mut reference = alloc::string::String::with_capacity(BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_LEN);
        reference.push_str(BLAKE3_REF_PREFIX);
        reference.push_str(hash.to_hex().as_str());
        Self(reference)
    }

    /// The identity of already-canonical bytes.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self::from_hash(blake3::hash(bytes))
    }

    /// The identity of `value`: BLAKE3 over its canonical packed bytes.
    pub fn of_value(value: &crate::model::PackedValue) -> Self {
        Self::of_bytes(&crate::writer::encode_canonical(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 64-digit lowercase hex digest.
    pub fn hex(&self) -> &str {
        &self.0[BLAKE3_REF_PREFIX.len()..]
    }

    pub fn into_string(self) -> alloc::string::String {
        self.0
    }
}

impl core::fmt::Display for ContentRef {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub fn has_ref_prefix(value: &str) -> bool {
    value.starts_with(BLAKE3_REF_PREFIX)
}

pub fn validate_ref(value: &str) -> Result<(), RefError> {
    ref_digest(value).map(|_| ())
}

/// The digest of a canonical reference.
pub fn ref_digest(value: &str) -> Result<&str, RefError> {
    let digest = value.strip_prefix(BLAKE3_REF_PREFIX).ok_or(RefError::MissingPrefix)?;
    validate_digest(digest)?;
    Ok(digest)
}

fn validate_digest(hex: &str) -> Result<(), RefError> {
    if hex.len() != BLAKE3_HEX_LEN {
        return Err(RefError::DigestLength);
    }
    if hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        Ok(())
    } else {
        Err(RefError::DigestCharacter)
    }
}

/// A domain-separated reference: BLAKE3 in key-derivation mode under `context`, over `bytes`.
pub fn domain_ref(context: &str, bytes: &[u8]) -> ContentRef {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(bytes);
    ContentRef::from_hash(hasher.finalize())
}

/// A domain-separated reference over the canonical packed bytes of `value`.
pub fn domain_value_ref(context: &str, value: &crate::model::PackedValue) -> ContentRef {
    domain_ref(context, &crate::writer::encode_canonical(value))
}

pub const NONE_LABEL: &str = "none";
pub const SOME_LABEL: &str = "some";

/// Why a value is not the canonical encoding of an optional reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionalRefError {
    /// Neither `<none>` nor `<some x>`.
    Shape,
    /// `<some x>` where `x` is not a string.
    NotString,
    /// `<some "...">` where the string is not a canonical reference.
    InvalidRef(RefError),
}

impl core::fmt::Display for OptionalRefError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Shape => formatter.write_str("optional ref must be <none> or <some \"blake3:...\">"),
            Self::NotString => formatter.write_str("optional ref <some ...> must hold a string"),
            Self::InvalidRef(error) => write!(formatter, "optional ref is not canonical: {error}"),
        }
    }
}

/// Encodes an optional reference: `<none>` or `<some "text">`. The text is written as given;
/// `decode_optional_ref` validates it.
// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
pub fn optional_ref_value(reference: Option<&str>) -> crate::model::PackedValue {
    match reference {
        None => crate::model::PackedValue::symbol_record(NONE_LABEL, alloc::vec::Vec::new()),
        Some(text) => {
            crate::model::PackedValue::symbol_record(SOME_LABEL, alloc::vec![crate::model::PackedValue::string(text)])
        }
    }
}

/// Decodes the single canonical optional-reference encoding. A bare string is a second encoding of
/// `Some` and is rejected, so one logical value has one canonical identity.
pub fn decode_optional_ref(value: &crate::model::PackedValue) -> Result<Option<ContentRef>, OptionalRefError> {
    if value.symbol_record_fields(NONE_LABEL, 0).is_some() {
        return Ok(None);
    }
    let [inner] = value.symbol_record_fields(SOME_LABEL, 1).ok_or(OptionalRefError::Shape)? else {
        return Err(OptionalRefError::Shape);
    };
    let text = inner.as_str().ok_or(OptionalRefError::NotString)?;
    ContentRef::parse(text).map(Some).map_err(OptionalRefError::InvalidRef)
}
