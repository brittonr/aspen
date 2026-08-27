use std::fmt;

pub const WORLD_COMMIT_SCHEMA: &str = "molten.world-commit.v1";
pub const WORLD_COMMIT_CAPTURE_RECEIPT_SCHEMA: &str = "molten.world-commit.capture-receipt.v1";
pub const WORLD_COMMIT_CLOSURE_REPORT_SCHEMA: &str = "molten.world-commit.closure-report.v1";
pub const WORLD_COMMIT_RESTORE_PLAN_SCHEMA: &str = "molten.world-commit.restore-plan.v1";
pub const WORLD_COMMIT_DETACHED_EVIDENCE_SCHEMA: &str = "molten.world-commit.detached-evidence.v1";
pub const WORLD_COMMIT_ARTIFACT_AUTH_PROFILE: &str = "molten-world-commit-v1";

pub const MAX_WORLD_COMMIT_PARENTS: usize = 32;
pub const MAX_WORLD_COMMIT_ROOTS: usize = 16;
pub const MAX_WORLD_COMMIT_REVISION_FENCES: usize = 16;
pub const MAX_WORLD_COMMIT_CLOSURE_OBJECTS: usize = 4_096;
pub const MAX_WORLD_COMMIT_CANONICAL_BYTES: usize = 1_048_576;
pub const MAX_WORLD_COMMIT_SOURCE_ID_BYTES: usize = 256;

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RootKind {
    Artifact,
    Schema,
    DurableState,
    Tasks,
    History,
    Effects,
    Scheduler,
    Time,
    Entropy,
    RuntimeProfile,
    Policy,
    AuthorityObservation,
    OpaqueMachineSnapshot,
}

impl RootKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Schema => "schema",
            Self::DurableState => "durable-state",
            Self::Tasks => "tasks",
            Self::History => "history",
            Self::Effects => "effects",
            Self::Scheduler => "scheduler",
            Self::Time => "time",
            Self::Entropy => "entropy",
            Self::RuntimeProfile => "runtime-profile",
            Self::Policy => "policy",
            Self::AuthorityObservation => "authority-observation",
            Self::OpaqueMachineSnapshot => "opaque-machine-snapshot",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldCommitReferenceError> {
        match value {
            "artifact" => Ok(Self::Artifact),
            "schema" => Ok(Self::Schema),
            "durable-state" => Ok(Self::DurableState),
            "tasks" => Ok(Self::Tasks),
            "history" => Ok(Self::History),
            "effects" => Ok(Self::Effects),
            "scheduler" => Ok(Self::Scheduler),
            "time" => Ok(Self::Time),
            "entropy" => Ok(Self::Entropy),
            "runtime-profile" => Ok(Self::RuntimeProfile),
            "policy" => Ok(Self::Policy),
            "authority-observation" => Ok(Self::AuthorityObservation),
            "opaque-machine-snapshot" => Ok(Self::OpaqueMachineSnapshot),
            _ => Err(WorldCommitReferenceError::UnknownRootKind(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldCommitReferenceError {
    UnsupportedAlgorithm,
    WrongDigestLength { actual: usize, expected: usize },
    InvalidDigestSpelling,
    EmptySourceId,
    SourceIdTooLong { actual: usize, maximum: usize },
    InvalidSourceId,
    UnknownRootKind(String),
    UnsupportedVersion(String),
    UnsupportedProfileKind(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CanonicalDigest(String);

impl CanonicalDigest {
    fn new(value: impl Into<String>) -> Result<Self, WorldCommitReferenceError> {
        let value = value.into();
        validate_digest(&value)?;
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! digest_reference {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(CanonicalDigest);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, WorldCommitReferenceError> {
                CanonicalDigest::new(value).map(Self)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

digest_reference!(WorldCommitRef);
digest_reference!(SnapshotProfileRef);
digest_reference!(SnapshotCohortRef);
digest_reference!(ArtifactRootRef);
digest_reference!(SchemaRootRef);
digest_reference!(DurableStateRootRef);
digest_reference!(TaskRootRef);
digest_reference!(HistoryRootRef);
digest_reference!(EffectRootRef);
digest_reference!(SchedulerRootRef);
digest_reference!(TimeRootRef);
digest_reference!(EntropyRootRef);
digest_reference!(RuntimeProfileRootRef);
digest_reference!(PolicyRootRef);
digest_reference!(AuthorityObservationRootRef);
digest_reference!(OpaqueMachineSnapshotRootRef);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldRootRef {
    Artifact(ArtifactRootRef),
    Schema(SchemaRootRef),
    DurableState(DurableStateRootRef),
    Tasks(TaskRootRef),
    History(HistoryRootRef),
    Effects(EffectRootRef),
    Scheduler(SchedulerRootRef),
    Time(TimeRootRef),
    Entropy(EntropyRootRef),
    RuntimeProfile(RuntimeProfileRootRef),
    Policy(PolicyRootRef),
    AuthorityObservation(AuthorityObservationRootRef),
    OpaqueMachineSnapshot(OpaqueMachineSnapshotRootRef),
}

impl WorldRootRef {
    pub const fn kind(&self) -> RootKind {
        match self {
            Self::Artifact(_) => RootKind::Artifact,
            Self::Schema(_) => RootKind::Schema,
            Self::DurableState(_) => RootKind::DurableState,
            Self::Tasks(_) => RootKind::Tasks,
            Self::History(_) => RootKind::History,
            Self::Effects(_) => RootKind::Effects,
            Self::Scheduler(_) => RootKind::Scheduler,
            Self::Time(_) => RootKind::Time,
            Self::Entropy(_) => RootKind::Entropy,
            Self::RuntimeProfile(_) => RootKind::RuntimeProfile,
            Self::Policy(_) => RootKind::Policy,
            Self::AuthorityObservation(_) => RootKind::AuthorityObservation,
            Self::OpaqueMachineSnapshot(_) => RootKind::OpaqueMachineSnapshot,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Artifact(reference) => reference.as_str(),
            Self::Schema(reference) => reference.as_str(),
            Self::DurableState(reference) => reference.as_str(),
            Self::Tasks(reference) => reference.as_str(),
            Self::History(reference) => reference.as_str(),
            Self::Effects(reference) => reference.as_str(),
            Self::Scheduler(reference) => reference.as_str(),
            Self::Time(reference) => reference.as_str(),
            Self::Entropy(reference) => reference.as_str(),
            Self::RuntimeProfile(reference) => reference.as_str(),
            Self::Policy(reference) => reference.as_str(),
            Self::AuthorityObservation(reference) => reference.as_str(),
            Self::OpaqueMachineSnapshot(reference) => reference.as_str(),
        }
    }

    pub fn parse(kind: RootKind, value: impl Into<String>) -> Result<Self, WorldCommitReferenceError> {
        let value = value.into();
        match kind {
            RootKind::Artifact => ArtifactRootRef::new(value).map(Self::Artifact),
            RootKind::Schema => SchemaRootRef::new(value).map(Self::Schema),
            RootKind::DurableState => DurableStateRootRef::new(value).map(Self::DurableState),
            RootKind::Tasks => TaskRootRef::new(value).map(Self::Tasks),
            RootKind::History => HistoryRootRef::new(value).map(Self::History),
            RootKind::Effects => EffectRootRef::new(value).map(Self::Effects),
            RootKind::Scheduler => SchedulerRootRef::new(value).map(Self::Scheduler),
            RootKind::Time => TimeRootRef::new(value).map(Self::Time),
            RootKind::Entropy => EntropyRootRef::new(value).map(Self::Entropy),
            RootKind::RuntimeProfile => RuntimeProfileRootRef::new(value).map(Self::RuntimeProfile),
            RootKind::Policy => PolicyRootRef::new(value).map(Self::Policy),
            RootKind::AuthorityObservation => AuthorityObservationRootRef::new(value).map(Self::AuthorityObservation),
            RootKind::OpaqueMachineSnapshot => {
                OpaqueMachineSnapshotRootRef::new(value).map(Self::OpaqueMachineSnapshot)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldCommitVersion {
    V1,
}

impl WorldCommitVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldCommitReferenceError> {
        match value {
            "v1" => Ok(Self::V1),
            _ => Err(WorldCommitReferenceError::UnsupportedVersion(value.to_string())),
        }
    }
}

pub(crate) fn validate_source_id(value: &str) -> Result<(), WorldCommitReferenceError> {
    if value.is_empty() {
        return Err(WorldCommitReferenceError::EmptySourceId);
    }
    if value.len() > MAX_WORLD_COMMIT_SOURCE_ID_BYTES {
        return Err(WorldCommitReferenceError::SourceIdTooLong {
            actual: value.len(),
            maximum: MAX_WORLD_COMMIT_SOURCE_ID_BYTES,
        });
    }
    let is_valid = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/'));
    if !is_valid {
        return Err(WorldCommitReferenceError::InvalidSourceId);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), WorldCommitReferenceError> {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return Err(WorldCommitReferenceError::UnsupportedAlgorithm);
    };
    if hex.len() != BLAKE3_HEX_LENGTH {
        return Err(WorldCommitReferenceError::WrongDigestLength {
            actual: hex.len(),
            expected: BLAKE3_HEX_LENGTH,
        });
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')) {
        return Err(WorldCommitReferenceError::InvalidDigestSpelling);
    }
    Ok(())
}
