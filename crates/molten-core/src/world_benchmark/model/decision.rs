use super::WorldBenchmarkReceipt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkExtractionDisposition {
    RetainCurrent,
    OptimizeInPlace,
    EvaluateSharedComponent,
}

impl WorldBenchmarkExtractionDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RetainCurrent => "retain-current",
            Self::OptimizeInPlace => "optimize-in-place",
            Self::EvaluateSharedComponent => "evaluate-shared-component",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkExtractionPolicy {
    pub minimum_accepted_receipts_per_consumer: u32,
    pub minimum_credible_consumers: u32,
    pub require_product_neutral_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkExtractionEvidence {
    pub receipt: WorldBenchmarkReceipt,
    pub owned_adapter: bool,
    pub product_neutral_limit_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkExtractionDecision {
    pub schema: String,
    pub disposition: WorldBenchmarkExtractionDisposition,
    pub accepted_receipt_refs: Vec<String>,
    pub credible_consumers: Vec<String>,
    pub diagnostics: Vec<String>,
    pub creates_repository: bool,
    pub approves_dependency: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkIssue {
    SchemaMismatch,
    InvalidReference(&'static str),
    InvalidRevision,
    StaleRevision,
    UnknownPreparation,
    PreparationDrift,
    HiddenPrepopulation,
    EmptyOperations,
    OperationLimitExceeded,
    RepetitionLimitExceeded,
    EmptyAdapters,
    AdapterLimitExceeded,
    DuplicateAdapter(String),
    InvalidHardwareCohort,
    ThresholdLimitExceeded,
    InvalidThreshold(String),
    DuplicateThreshold(String),
    OperationClassMismatch(&'static str),
    DatasetMismatch,
    DatasetBoundsExceeded(&'static str),
    ResultLimitExceeded,
    UnexpectedOperation(&'static str),
    InvalidRepetition,
    InvalidAdapter(String),
    MissingMetric(&'static str),
    DuplicateMetric(&'static str),
    MetricBoundExceeded(&'static str),
    PhysicalMeasurementCollapsed,
    SnapshotBindingMissing,
    SnapshotBindingUnexpected,
    SnapshotRevisionMismatch,
    SnapshotProfileMismatch,
    SnapshotDescriptorInvalid,
    UnsupportedRowsPresent,
    ReceiptOverclaim,
    ReceiptIdentityMismatch,
    ComparisonClassMismatch,
    ComparisonCohortMismatch,
    ExtractionPolicyInvalid,
    ExtractionEvidenceInvalid,
}
