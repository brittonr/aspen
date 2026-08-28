pub const ORACLE_OBSERVATION_SCHEMA: &str = "molten.semantic-state-oracle-observation.v1";
pub const ORACLE_COMPARISON_SCHEMA: &str = "molten.semantic-state-oracle-comparison.v1";
pub const ORACLE_PROJECTION_SCHEMA: &str = "molten.semantic-state-oracle-projection.v1";
pub const REQUIRED_ORACLE_NON_CLAIM_COUNT: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OracleCaseKind {
    HistoryIndependentState,
    DetachedRead,
    BranchIsolation,
    CompareAndAdvance,
    ReaderSafeGarbageCollection,
    ExactFormatReopen,
    SerializationRoundTrip,
    RowIdRejected,
    CustomCollationRejected,
    StaleWriterDenied,
    CompetingWriterClassified,
    MissingSourcePin,
    TamperedStorage,
    WrongFormatRejected,
    MalformedSerialization,
    RemoteDisabled,
    MultiFileWriteUnsupported,
    IdentityOverclaimRejected,
}

impl OracleCaseKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HistoryIndependentState => "history-independent-state",
            Self::DetachedRead => "detached-read",
            Self::BranchIsolation => "branch-isolation",
            Self::CompareAndAdvance => "compare-and-advance",
            Self::ReaderSafeGarbageCollection => "reader-safe-gc",
            Self::ExactFormatReopen => "exact-format-reopen",
            Self::SerializationRoundTrip => "serialization-round-trip",
            Self::RowIdRejected => "rowid-rejected",
            Self::CustomCollationRejected => "custom-collation-rejected",
            Self::StaleWriterDenied => "stale-writer-denied",
            Self::CompetingWriterClassified => "competing-writer-classified",
            Self::MissingSourcePin => "missing-source-pin",
            Self::TamperedStorage => "tampered-storage",
            Self::WrongFormatRejected => "wrong-format-rejected",
            Self::MalformedSerialization => "malformed-serialization",
            Self::RemoteDisabled => "remote-disabled",
            Self::MultiFileWriteUnsupported => "multi-file-write-unsupported",
            Self::IdentityOverclaimRejected => "identity-overclaim-rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleOutcome {
    Applied,
    EqualState,
    ReadOnly,
    StaleSnapshot,
    Busy,
    Rejected,
    FormatRejected,
    Corrupt,
    Unsupported,
}

impl OracleOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::EqualState => "equal-state",
            Self::ReadOnly => "read-only",
            Self::StaleSnapshot => "stale-snapshot",
            Self::Busy => "busy",
            Self::Rejected => "rejected",
            Self::FormatRejected => "format-rejected",
            Self::Corrupt => "corrupt",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticStateRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleObservationInput {
    pub adapter_ref: String,
    pub case: OracleCaseKind,
    pub branch: Option<String>,
    pub rows: Vec<SemanticStateRow>,
    pub outcome: OracleOutcome,
    pub backend_root: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleObservation {
    pub schema: String,
    pub observation_ref: String,
    pub source_revision: String,
    pub build_ref: String,
    pub adapter_ref: String,
    pub case: OracleCaseKind,
    pub branch: Option<String>,
    pub rows: Vec<SemanticStateRow>,
    pub outcome: OracleOutcome,
    pub backend_root: Option<String>,
    pub backend_root_is_global_identity: bool,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<OracleNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OracleNonClaim {
    MoltenCorrectness,
    RootFormatEquality,
    CompleteWorldAtomicity,
    DurableConflictSafety,
    Authority,
    ProductionReadiness,
    ReleaseEligibility,
}

impl OracleNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MoltenCorrectness => "does-not-prove-molten-correctness",
            Self::RootFormatEquality => "does-not-claim-cross-format-root-equality",
            Self::CompleteWorldAtomicity => "does-not-prove-complete-world-atomicity",
            Self::DurableConflictSafety => "does-not-prove-durable-conflict-safety",
            Self::Authority => "does-not-grant-authority",
            Self::ProductionReadiness => "does-not-establish-production-readiness",
            Self::ReleaseEligibility => "does-not-establish-release-eligibility",
        }
    }
}

pub const REQUIRED_ORACLE_NON_CLAIMS: [OracleNonClaim; REQUIRED_ORACLE_NON_CLAIM_COUNT] = [
    OracleNonClaim::MoltenCorrectness,
    OracleNonClaim::RootFormatEquality,
    OracleNonClaim::CompleteWorldAtomicity,
    OracleNonClaim::DurableConflictSafety,
    OracleNonClaim::Authority,
    OracleNonClaim::ProductionReadiness,
    OracleNonClaim::ReleaseEligibility,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleConsumer {
    ProllyPilot,
    WorldBenchmark,
}

impl OracleConsumer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProllyPilot => "pilot-prolly-semantic-state-map",
            Self::WorldBenchmark => "benchmark-world-commit-sharing-and-retention",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonDecision {
    Agreement,
    Divergence,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleComparison {
    pub schema: String,
    pub comparison_ref: String,
    pub expected_ref: String,
    pub observed_ref: String,
    pub decision: ComparisonDecision,
    pub first_divergence: Option<String>,
    pub backend_roots_compared_as_global: bool,
    pub non_claims: Vec<OracleNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleEvidenceProjection {
    pub schema: String,
    pub projection_ref: String,
    pub consumer: OracleConsumer,
    pub source_revision: String,
    pub build_ref: String,
    pub adapter_ref: String,
    pub case: OracleCaseKind,
    pub observation_ref: String,
    pub comparison_ref: String,
    pub decision: ComparisonDecision,
    pub branch: Option<String>,
    pub rows: Vec<SemanticStateRow>,
    pub outcome: OracleOutcome,
    pub backend_root_included: bool,
    pub authority_granted: bool,
    pub correctness_proven: bool,
    pub non_claims: Vec<OracleNonClaim>,
}
