//! Checked string wrappers that admit only validated actor ids, capabilities and evidence refs.

/// Deserializes a wrapper only through its checked `parse`, so serde input never bypasses
/// validation.
macro_rules! checked_string_deserialize {
    ($wrapper:ident) => {
        impl<'de> serde::Deserialize<'de> for $wrapper {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

// r[impl molten.runtime_spine.admission_and_capability_hardening]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct ActorId(String);

checked_string_deserialize!(ActorId);

impl ActorId {
    pub fn parse(value: impl Into<String>) -> super::Result<Self> {
        let value = value.into();
        super::validate_nonempty_token("actor id", &value, super::MAX_ACTOR_ID_BYTES)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct Capability(String);

checked_string_deserialize!(Capability);

impl Capability {
    pub fn parse(value: impl Into<String>) -> super::Result<Self> {
        let value = value.into();
        super::validate_nonempty_token("capability", &value, super::MAX_CAPABILITY_BYTES)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct EvidenceRef(String);

checked_string_deserialize!(EvidenceRef);

impl EvidenceRef {
    pub fn parse(value: impl Into<String>) -> super::Result<Self> {
        let value = value.into();
        super::validate_content_ref(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
