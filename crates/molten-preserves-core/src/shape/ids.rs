//! Typed stable identifiers and the closed vocabularies of decisions, check statuses and replay
//! classes.

use alloc::borrow::ToOwned;

type String = alloc::string::String;
type ViewError = super::ViewError;

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const CHECK_STATUS_FAIL: &str = "fail";
const CHECK_STATUS_DIAGNOSTIC: &str = "diagnostic";
const REPLAY_CLASS_IDEMPOTENT: &str = "idempotent";
const REPLAY_CLASS_DETERMINISTIC: &str = "deterministic";
const REPLAY_CLASS_EFFECTFUL: &str = "effectful";

/// Admits a non-empty id of ASCII letters, digits, `_`, `.`, `:` and `-`.
pub fn validate_stable_id(value: &str, label: &str) -> Result<(), ViewError> {
    if value.is_empty() {
        return Err(ViewError::EmptyId {
            label: label.to_owned(),
        });
    }
    if value.bytes().all(is_stable_id_byte) {
        return Ok(());
    }
    Err(ViewError::IdCharacters {
        label: label.to_owned(),
        value: value.to_owned(),
    })
}

fn is_stable_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ViewError> {
        Self::labelled(value.as_ref(), "stable id")
    }

    fn labelled(value: &str, label: &str) -> Result<Self, ViewError> {
        validate_stable_id(value, label)?;
        Ok(Self(value.to_owned()))
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
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ViewError> {
        StableId::labelled(value.as_ref(), "schema id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// A stable id that starts with a lowercase ASCII letter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(StableId);

impl OperationId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ViewError> {
        let value = value.as_ref();
        let id = StableId::labelled(value, "operation id")?;
        if value.bytes().next().is_some_and(|first| first.is_ascii_lowercase()) {
            return Ok(Self(id));
        }
        Err(ViewError::OperationIdFirstCharacter {
            value: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(StableId);

impl ProfileId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ViewError> {
        StableId::labelled(value.as_ref(), "profile id").map(Self)
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
    pub fn parse(value: &str) -> Result<Self, ViewError> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            DECISION_DENY => Ok(Self::Deny),
            _ => Err(ViewError::UnsupportedDecision {
                value: value.to_owned(),
            }),
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
    pub fn parse(value: &str) -> Result<Self, ViewError> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            CHECK_STATUS_FAIL => Ok(Self::Fail),
            DECISION_DENY => Ok(Self::Deny),
            CHECK_STATUS_DIAGNOSTIC => Ok(Self::Diagnostic),
            _ => Err(ViewError::UnsupportedCheckStatus {
                value: value.to_owned(),
            }),
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
    pub fn parse(value: &str) -> Result<Self, ViewError> {
        match value {
            REPLAY_CLASS_IDEMPOTENT => Ok(Self::Idempotent),
            REPLAY_CLASS_DETERMINISTIC => Ok(Self::Deterministic),
            REPLAY_CLASS_EFFECTFUL => Ok(Self::Effectful),
            _ => Err(ViewError::UnsupportedReplayClass {
                value: value.to_owned(),
            }),
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
