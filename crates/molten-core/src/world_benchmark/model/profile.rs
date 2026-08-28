use super::WORLD_BENCHMARK_PROFILE_SCHEMA;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkClass {
    Logical,
    OpaqueExactSnapshot,
}

impl WorldBenchmarkClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::OpaqueExactSnapshot => "opaque-exact-snapshot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkPreparation {
    Unknown,
    Cold,
    DeclaredWarm,
}

impl WorldBenchmarkPreparation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Cold => "cold",
            Self::DeclaredWarm => "declared-warm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkDatasetShape {
    Synthetic,
    DownstreamShaped,
}

impl WorldBenchmarkDatasetShape {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::DownstreamShaped => "downstream-shaped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBenchmarkOperation {
    RootBranch,
    FirstMutation,
    RepeatedMutation,
    Diff,
    MergePlan,
    CapsuleExport,
    Replication,
    RetentionPlan,
    SnapshotShare,
}

impl WorldBenchmarkOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RootBranch => "root-branch",
            Self::FirstMutation => "first-mutation",
            Self::RepeatedMutation => "repeated-mutation",
            Self::Diff => "diff",
            Self::MergePlan => "merge-plan",
            Self::CapsuleExport => "capsule-export",
            Self::Replication => "replication",
            Self::RetentionPlan => "retention-plan",
            Self::SnapshotShare => "snapshot-share",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkBounds {
    pub max_operations: u32,
    pub max_repetitions: u32,
    pub max_logical_bytes: u64,
    pub max_physical_bytes: u64,
    pub max_objects: u64,
    pub max_pages: u64,
    pub max_references: u64,
    pub max_keys: u64,
    pub max_conflicts: u64,
    pub max_duration_nanoseconds: u64,
    pub max_peak_memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkDataset {
    pub dataset_ref: String,
    pub source_revision: String,
    pub shape: WorldBenchmarkDatasetShape,
    pub preparation: WorldBenchmarkPreparation,
    pub logical_bytes: u64,
    pub object_count: u64,
    pub preexisting_objects: u64,
    pub changed_objects: u64,
    pub mutation_bytes: u64,
    pub key_count: u64,
    pub page_count: u64,
    pub page_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkThreshold {
    pub name: String,
    pub metric: WorldBenchmarkMetricKind,
    pub maximum: u64,
    pub operation: Option<WorldBenchmarkOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkProfile {
    pub schema: String,
    pub profile_ref: String,
    pub source_revision: String,
    pub dataset_ref: String,
    pub preparation: WorldBenchmarkPreparation,
    pub class: WorldBenchmarkClass,
    pub adapters: Vec<String>,
    pub operations: Vec<WorldBenchmarkOperation>,
    pub bounds: WorldBenchmarkBounds,
    pub repetitions: u32,
    pub hardware_cohort: String,
    pub thresholds: Vec<WorldBenchmarkThreshold>,
}

impl WorldBenchmarkProfile {
    pub fn new(profile_ref: String) -> Self {
        Self {
            schema: WORLD_BENCHMARK_PROFILE_SCHEMA.to_string(),
            profile_ref,
            source_revision: String::new(),
            dataset_ref: String::new(),
            preparation: WorldBenchmarkPreparation::Unknown,
            class: WorldBenchmarkClass::Logical,
            adapters: Vec::new(),
            operations: Vec::new(),
            bounds: WorldBenchmarkBounds {
                max_operations: 0,
                max_repetitions: 0,
                max_logical_bytes: 0,
                max_physical_bytes: 0,
                max_objects: 0,
                max_pages: 0,
                max_references: 0,
                max_keys: 0,
                max_conflicts: 0,
                max_duration_nanoseconds: 0,
                max_peak_memory_bytes: 0,
            },
            repetitions: 0,
            hardware_cohort: String::new(),
            thresholds: Vec::new(),
        }
    }
}

use super::WorldBenchmarkMetricKind;
