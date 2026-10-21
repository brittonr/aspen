// The single mapping table from local service-state vocabularies to the shared
// assertion set. The lifecycle state machine, the system-extension phase
// vocabulary, and the production readiness report each contribute one row per
// local value.

const FLAG_STARTED: u8 = 1 << 0;
const FLAG_READY: u8 = 1 << 1;
const FLAG_FAILED: u8 = 1 << 2;
const FLAG_NORMAL_FINISH: u8 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceStateVocabulary {
    Lifecycle,
    Extension,
    Readiness,
}

impl ServiceStateVocabulary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lifecycle => "lifecycle",
            Self::Extension => "extension",
            Self::Readiness => "readiness",
        }
    }
}

/// One row of the single mapping table from a local vocabulary value to the
/// shared assertion set. A normal-finish row reports `complete` only when the
/// one-shot finished normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceStateMappingRow {
    pub vocabulary: ServiceStateVocabulary,
    pub local_value: &'static str,
    flags: u8,
}

impl ServiceStateMappingRow {
    pub const fn is_started(&self) -> bool {
        self.flags & FLAG_STARTED != 0
    }

    pub const fn is_ready(&self) -> bool {
        self.flags & FLAG_READY != 0
    }

    pub const fn is_failed(&self) -> bool {
        self.flags & FLAG_FAILED != 0
    }

    pub const fn is_normal_finish(&self) -> bool {
        self.flags & FLAG_NORMAL_FINISH != 0
    }
}

const fn row(vocabulary: ServiceStateVocabulary, local_value: &'static str, flags: u8) -> ServiceStateMappingRow {
    ServiceStateMappingRow {
        vocabulary,
        local_value,
        flags,
    }
}

const SERVICE_STATE_MAPPING: &[ServiceStateMappingRow] = &[
    // Lifecycle state machine.
    row(ServiceStateVocabulary::Lifecycle, "declared", 0),
    row(ServiceStateVocabulary::Lifecycle, "spawning", 0),
    row(ServiceStateVocabulary::Lifecycle, "starting", FLAG_STARTED),
    row(ServiceStateVocabulary::Lifecycle, "ready", FLAG_STARTED | FLAG_READY),
    row(ServiceStateVocabulary::Lifecycle, "degraded", FLAG_STARTED),
    row(ServiceStateVocabulary::Lifecycle, "stopping", FLAG_STARTED),
    row(
        ServiceStateVocabulary::Lifecycle,
        "stopped",
        FLAG_STARTED | FLAG_NORMAL_FINISH,
    ),
    row(ServiceStateVocabulary::Lifecycle, "failed", FLAG_FAILED),
    row(ServiceStateVocabulary::Lifecycle, "restarting", FLAG_STARTED),
    row(ServiceStateVocabulary::Lifecycle, "cleaned", 0),
    // System extension phase.
    row(ServiceStateVocabulary::Extension, "absent", 0),
    row(ServiceStateVocabulary::Extension, "installed", 0),
    row(ServiceStateVocabulary::Extension, "admitted", 0),
    row(ServiceStateVocabulary::Extension, "initializing", 0),
    row(ServiceStateVocabulary::Extension, "initialized", 0),
    row(ServiceStateVocabulary::Extension, "starting", FLAG_STARTED),
    row(
        ServiceStateVocabulary::Extension,
        "running",
        FLAG_STARTED | FLAG_READY,
    ),
    row(
        ServiceStateVocabulary::Extension,
        "checkpointing",
        FLAG_STARTED | FLAG_READY,
    ),
    row(ServiceStateVocabulary::Extension, "recovering", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "restarting", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "upgrading", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "rolling-back", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "draining", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "drained", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "shutting-down", FLAG_STARTED),
    row(ServiceStateVocabulary::Extension, "failed", FLAG_FAILED),
    row(ServiceStateVocabulary::Extension, "quarantined", FLAG_FAILED),
    row(
        ServiceStateVocabulary::Extension,
        "stopped",
        FLAG_STARTED | FLAG_NORMAL_FINISH,
    ),
    row(ServiceStateVocabulary::Extension, "removed", 0),
    // Production readiness report.
    row(
        ServiceStateVocabulary::Readiness,
        "ready",
        FLAG_STARTED | FLAG_READY,
    ),
    row(ServiceStateVocabulary::Readiness, "failed", FLAG_FAILED),
];

/// The single mapping table from local vocabulary values to the shared set.
pub fn service_state_mapping() -> &'static [ServiceStateMappingRow] {
    SERVICE_STATE_MAPPING
}

fn mapping_row(vocabulary: ServiceStateVocabulary, local_value: &str) -> Option<&'static ServiceStateMappingRow> {
    SERVICE_STATE_MAPPING
        .iter()
        .find(|row| row.vocabulary == vocabulary && row.local_value == local_value)
}
