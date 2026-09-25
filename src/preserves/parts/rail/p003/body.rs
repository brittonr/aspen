
pub fn strict_canonical_decode_with_ref(
    bytes: &[u8],
    expected_ref: &str,
    boundary: &str,
) -> Result<StrictCanonicalDecode> {
    let expected = ContentRef::parse(expected_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{boundary} expected content ref is invalid: {error}"))
    })?;
    let decoded = strict_canonical_decode(bytes)?;
    if decoded.value_ref != expected {
        return Err(MoltenError::invalid_harness(format!(
            "{boundary} strict canonical decode ref mismatch: expected {}, got {}",
            expected, decoded.value_ref
        )));
    }
    Ok(decoded)
}

pub fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    Ok(strict_canonical_decode(bytes)?.value)
}

pub fn canonical_hash(value: &IoValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(content_ref_from_bytes(&bytes))
}

pub fn content_ref_from_bytes(bytes: &[u8]) -> String {
    content_ref_from_blake3_hash(blake3::hash(bytes))
}

pub fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    format!("{BLAKE3_REF_PREFIX}{}", hash.to_hex())
}

pub fn canonical_content_ref(value: &IoValue) -> Result<ContentRef> {
    ContentRef::parse(&canonical_hash(value)?)
}

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const CHECK_STATUS_FAIL: &str = "fail";
const CHECK_STATUS_DIAGNOSTIC: &str = "diagnostic";
const REPLAY_CLASS_IDEMPOTENT: &str = "idempotent";
const REPLAY_CLASS_DETERMINISTIC: &str = "deterministic";
const REPLAY_CLASS_EFFECTFUL: &str = "effectful";
const OPERATION_FIRST_CHAR_LABEL: &str = "operation id first character";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "stable id")?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaId(StableId);

impl SchemaId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "schema id")?;
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(StableId);

impl OperationId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "operation id")?;
        let first = value
            .bytes()
            .next()
            .ok_or_else(|| MoltenError::invalid_harness("operation id cannot be empty"))?;
        if !first.is_ascii_lowercase() {
            return Err(MoltenError::invalid_harness(format!(
                "{OPERATION_FIRST_CHAR_LABEL} must be lowercase ascii, got {value}"
            )));
        }
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(StableId);

impl ProfileId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "profile id")?;
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Deny,
}

impl Decision {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            DECISION_DENY => Ok(Self::Deny),
            _ => Err(MoltenError::invalid_harness(format!("unsupported decision {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => DECISION_PASS,
            Self::Deny => DECISION_DENY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Deny,
    Diagnostic,
}

impl CheckStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            CHECK_STATUS_FAIL => Ok(Self::Fail),
            DECISION_DENY => Ok(Self::Deny),
            CHECK_STATUS_DIAGNOSTIC => Ok(Self::Diagnostic),
            _ => Err(MoltenError::invalid_harness(format!("unsupported check status {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => DECISION_PASS,
            Self::Fail => CHECK_STATUS_FAIL,
            Self::Deny => DECISION_DENY,
            Self::Diagnostic => CHECK_STATUS_DIAGNOSTIC,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayClass {
    Idempotent,
    Deterministic,
    Effectful,
}

impl ReplayClass {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            REPLAY_CLASS_IDEMPOTENT => Ok(Self::Idempotent),
            REPLAY_CLASS_DETERMINISTIC => Ok(Self::Deterministic),
            REPLAY_CLASS_EFFECTFUL => Ok(Self::Effectful),
            _ => Err(MoltenError::invalid_harness(format!("unsupported replay class {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idempotent => REPLAY_CLASS_IDEMPOTENT,
            Self::Deterministic => REPLAY_CLASS_DETERMINISTIC,
            Self::Effectful => REPLAY_CLASS_EFFECTFUL,
        }
    }
}

pub fn validate_stable_id(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} cannot be empty")));
    }
    if value.bytes().all(is_stable_id_byte) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!(
        "{label} must contain only ASCII letters, digits, '_', '.', ':', or '-', got {value}"
    )))
}

fn is_stable_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
}

pub fn symbol(name: &'static str) -> IoValue {
    IoValue::symbol(name)
}

pub fn string(value: impl AsRef<str>) -> IoValue {
    IoValue::new(value.as_ref().to_owned())
}

pub fn u64_value(value: u64) -> IoValue {
    IoValue::new(value)
}

pub fn bool_value(value: bool) -> IoValue {
    IoValue::new(value)
}

pub fn sequence(values: Vec<IoValue>) -> IoValue {
    IoValue::new(values)
}

pub fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    IoValue::record(symbol(label), fields)
}

pub fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    IoValue::from(value.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCheck {
    pub name: String,
    pub status: String,
}

// r[impl molten.preserves_rail_toolkit.parser_builders]
// r[impl molten.preserves_rail_toolkit.negative_shapes]
pub fn simple_record_fields<'a>(
    value: &'a IoValue,
    label: &str,
    arity: u64,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    let expected_arity = crate::bounded::usize_from_u64(arity, "record arity")?;
    value
        .collect_simple_record(label, Some(expected_arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

pub fn required_string_field(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

pub fn required_content_ref(value: &Value<IoValue>, field: &str) -> Result<ContentRef> {
    let reference = required_string_field(value, field)?;
    ContentRef::parse(&reference).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}"))
    })
}

pub fn required_content_ref_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    Ok(required_content_ref(value, field)?.into_string())
}

pub fn optional_content_ref(value: &Value<IoValue>, field: &str) -> Result<Option<ContentRef>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_content_ref(&some[0], field).map(Some);
    }
    required_content_ref(value, field).map(Some)
}
