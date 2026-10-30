#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    pub namespace: release_channel_index::model::Namespace,
    pub provider_key: [u8; 32],
    pub claim: release_channel_index::model::Claim,
    pub id: [u8; 32],
    pub revision: u64,
}

/// Authenticated evidence retained for detached receipts, not branch authorization.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub snapshot: release_channel_index::model::Snapshot,
    pub request: release_channel_index::model::Request,
    pub reply: release_channel_index::model::SignedReply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Invalid,
    Unavailable,
    Uncertain,
}

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub detail: String,
    pub reservation: Option<Box<Reservation>>,
}

impl Error {
    pub(super) fn invalid(detail: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Invalid,
            detail: detail.into(),
            reservation: None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for Error {}
