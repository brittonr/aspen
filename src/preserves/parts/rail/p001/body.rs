pub const HASH_ALGORITHM: &str = molten_preserves_core::identity::HASH_ALGORITHM;

// r[impl molten.runtime_spine.canonical_content_refs.shape]
// r[impl molten.runtime_spine.canonical_content_refs.negative_tests]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentRef(String);

impl ContentRef {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_content_ref(value)?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ContentRef {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ContentRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContentRef {
    type Err = MoltenError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ContentRef {
    type Error = MoltenError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for ContentRef {
    type Error = MoltenError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<molten_preserves_core::identity::ContentRef> for ContentRef {
    fn from(reference: molten_preserves_core::identity::ContentRef) -> Self {
        Self(reference.into_string())
    }
}

impl serde::Serialize for ContentRef {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ContentRef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
pub fn validate_content_ref(value: &str) -> Result<()> {
    molten_preserves_core::identity::validate_ref(value).map_err(|error| identity_bridge::ref_failure(value, error))
}

pub fn content_ref_has_prefix(value: &str) -> bool {
    molten_preserves_core::identity::has_ref_prefix(value)
}

pub fn content_ref_hex(value: &str) -> Result<&str> {
    molten_preserves_core::identity::ref_digest(value).map_err(|error| identity_bridge::ref_failure(value, error))
}

// r[impl molten.runtime_spine.canonical_content_refs.filename_readback]
pub fn content_ref_from_hex(hex: &str) -> Result<String> {
    let reference = format!("{}{hex}", molten_preserves_core::identity::BLAKE3_REF_PREFIX);
    molten_preserves_core::identity::ContentRef::from_hex(hex)
        .map(molten_preserves_core::identity::ContentRef::into_string)
        .map_err(|error| identity_bridge::ref_failure(&reference, error))
}

pub fn parse_text(source: &str) -> Result<IoValue> {
    preserves::read_iovalue_text(source, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn to_text(value: &IoValue) -> Result<String> {
    preserves::write_iovalue_text(value, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

/// Canonical packed bytes of `value`, without annotations, from the kernel's borrowed encoder.
pub fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    stream_bridge::encode_iovalue(value)
}

/// The kernel `PackedValue` of `value`, for shells that hand a value to a `no_std` kernel.
/// Annotations are dropped, as they are for canonical bytes.
pub fn packed_from_iovalue(value: &IoValue) -> Result<molten_preserves_core::model::PackedValue> {
    value_bridge::packed_from_iovalue(value)
}

/// The `IOValue` of a kernel `PackedValue`, for shells that hand a kernel result to `IOValue` callers.
pub fn iovalue_from_packed(value: &molten_preserves_core::model::PackedValue) -> Result<IoValue> {
    value_bridge::iovalue_from_packed(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictCanonicalDecode {
    pub value: IoValue,
    pub canonical_bytes: Vec<u8>,
    pub value_ref: ContentRef,
}

// r[impl molten.preserves_canonical_bytes.strict_decode]
// r[impl molten.preserves_canonical_bytes.noncanonical_denial]
pub fn strict_canonical_decode(bytes: &[u8]) -> Result<StrictCanonicalDecode> {
    let limits = molten_preserves_core::reader::DecodeLimits::IN_MEMORY;
    let packed = molten_preserves_core::reader::decode_canonical(bytes, &limits).map_err(strict_decode_failure)?;
    Ok(StrictCanonicalDecode {
        value: value_bridge::iovalue_from_packed(&packed)?,
        canonical_bytes: bytes.to_vec(),
        value_ref: ContentRef::from(molten_preserves_core::identity::ContentRef::of_bytes(bytes)),
    })
}

/// Keeps the established failure classes: bytes no Preserves reader accepts are `Preserves`
/// errors; parseable bytes that are not the canonical encoding of their value are denials.
fn strict_decode_failure(error: molten_preserves_core::reader::failure::DecodeError) -> MoltenError {
    match error.class() {
        molten_preserves_core::reader::failure::DecodeErrorClass::Malformed => {
            MoltenError::Preserves(format!("malformed packed Preserves input: {error}"))
        }
        molten_preserves_core::reader::failure::DecodeErrorClass::NonCanonical => MoltenError::invalid_harness(format!(
            "strict canonical Preserves decode failed: input bytes differ from canonical re-encoding: {error}"
        )),
        molten_preserves_core::reader::failure::DecodeErrorClass::Limit => {
            MoltenError::invalid_harness(format!("strict canonical Preserves decode failed: {error}"))
        }
    }
}
