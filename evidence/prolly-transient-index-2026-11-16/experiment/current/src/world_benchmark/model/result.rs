use super::WorldBenchmarkBounds;
use super::WorldBenchmarkClass;
use super::WorldBenchmarkOperation;
use super::WorldBenchmarkPreparation;
use super::WorldBenchmarkThreshold;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkMetricKind {
    LogicalBytes,
    PhysicalBytesWritten,
    NewObjects,
    ReusedObjects,
    CopiedPages,
    MappedPages,
    TraversedReferences,
    ComparedKeys,
    EmittedConflicts,
    TransferredBytes,
    RetainedObjects,
    PlannedDeletions,
}

impl WorldBenchmarkMetricKind {
    pub const ALL: [Self; super::WORLD_BENCHMARK_METRIC_COUNT] = [
        Self::LogicalBytes,
        Self::PhysicalBytesWritten,
        Self::NewObjects,
        Self::ReusedObjects,
        Self::CopiedPages,
        Self::MappedPages,
        Self::TraversedReferences,
        Self::ComparedKeys,
        Self::EmittedConflicts,
        Self::TransferredBytes,
        Self::RetainedObjects,
        Self::PlannedDeletions,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LogicalBytes => "logical-bytes",
            Self::PhysicalBytesWritten => "physical-bytes-written",
            Self::NewObjects => "new-objects",
            Self::ReusedObjects => "reused-objects",
            Self::CopiedPages => "copied-pages",
            Self::MappedPages => "mapped-pages",
            Self::TraversedReferences => "traversed-references",
            Self::ComparedKeys => "compared-keys",
            Self::EmittedConflicts => "emitted-conflicts",
            Self::TransferredBytes => "transferred-bytes",
            Self::RetainedObjects => "retained-objects",
            Self::PlannedDeletions => "planned-deletions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldBenchmarkMetric {
    pub kind: WorldBenchmarkMetricKind,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkSnapshotBinding {
    pub descriptor_ref: String,
    pub source_revision: String,
    pub completeness_profile: String,
    pub memory_bytes: u64,
    pub closure_members: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkResult {
    pub operation: WorldBenchmarkOperation,
    pub repetition: u32,
    pub adapter_ref: String,
    pub metrics: Vec<WorldBenchmarkMetric>,
    pub duration_nanoseconds: Option<u64>,
    pub peak_memory_bytes: Option<u64>,
    pub snapshot: Option<WorldBenchmarkSnapshotBinding>,
    pub physical_measurement_independent: bool,
}

impl WorldBenchmarkResult {
    pub fn metric(&self, kind: WorldBenchmarkMetricKind) -> Option<u64> {
        self.metrics.iter().find(|metric| metric.kind == kind).map(|metric| metric.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkPreparationObservation {
    pub dataset_ref: String,
    pub source_revision: String,
    pub preparation: WorldBenchmarkPreparation,
    pub prior_objects_available: bool,
    pub preparation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkPlan {
    pub schema: String,
    pub plan_ref: String,
    pub profile_ref: String,
    pub source_revision: String,
    pub dataset_ref: String,
    pub class: WorldBenchmarkClass,
    pub preparation: WorldBenchmarkPreparation,
    pub operations: Vec<WorldBenchmarkOperation>,
    pub repetitions: u32,
    pub adapters: Vec<String>,
    pub hardware_cohort: String,
    pub bounds: WorldBenchmarkBounds,
    pub thresholds: Vec<WorldBenchmarkThreshold>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkThresholdResult {
    pub name: String,
    pub metric: WorldBenchmarkMetricKind,
    pub observed_maximum: u64,
    pub admitted_maximum: u64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkUnsupportedRow {
    pub operation: WorldBenchmarkOperation,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkReceipt {
    pub schema: String,
    pub receipt_ref: String,
    pub plan_ref: String,
    pub consumer_id: String,
    pub profile_ref: String,
    pub source_revision: String,
    pub dataset_ref: String,
    pub preparation: WorldBenchmarkPreparation,
    pub class: WorldBenchmarkClass,
    pub adapters: Vec<String>,
    pub hardware_cohort: String,
    pub bounds: WorldBenchmarkBounds,
    pub results: Vec<WorldBenchmarkResult>,
    pub threshold_results: Vec<WorldBenchmarkThresholdResult>,
    pub unsupported_rows: Vec<WorldBenchmarkUnsupportedRow>,
    pub accepted: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkComparison {
    pub schema: String,
    pub left_receipt_ref: String,
    pub right_receipt_ref: String,
    pub comparable: bool,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<String>,
}
