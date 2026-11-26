use super::super::RootKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaptureIssue {
    ZeroBound(&'static str),
    BoundAboveHardLimit {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    LogicalProfileHasCohort,
    OpaqueProfileMissingCohort,
    DuplicateParent(String),
    DuplicateRootKind(RootKind),
    DuplicateRootRef(String),
    MissingRequiredRoot(RootKind),
    UnexpectedRoot(RootKind),
    CompletenessMismatch,
    RootDomainMismatch {
        expected: RootKind,
        actual: RootKind,
    },
    RootSchemaNotValidated(RootKind),
    IncompleteInventory(RootKind),
    FenceRootMismatch {
        root: RootKind,
        fence: RootKind,
    },
    DuplicateFenceSource(String),
    MissingRecheck(String),
    UnexpectedRecheck(String),
    DuplicateRecheck(String),
    RevisionDrift {
        source_id: String,
        expected: u64,
        actual: u64,
    },
    RecheckInventoryIncomplete(String),
}

impl CaptureIssue {
    /// The stable receipt code for this issue: a closed kebab-case tag followed by `:`-separated
    /// typed fields.
    ///
    /// Codes are receipt identity material, so they never derive from Rust `Debug` output. Each tag
    /// is unique up to its first `:`, numeric and enumerated fields cannot contain `:`, and the
    /// one free-form string field, when present, is always last, so distinct issues always
    /// produce distinct codes.
    // r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    pub fn code(&self) -> String {
        match self {
            Self::ZeroBound(field) => format!("zero-bound:{field}"),
            Self::BoundAboveHardLimit { field, actual, maximum } => {
                format!("bound-above-hard-limit:{field}:{actual}:{maximum}")
            }
            Self::BoundExceeded { field, actual, maximum } => format!("bound-exceeded:{field}:{actual}:{maximum}"),
            Self::LogicalProfileHasCohort => "logical-profile-has-cohort".to_string(),
            Self::OpaqueProfileMissingCohort => "opaque-profile-missing-cohort".to_string(),
            Self::DuplicateParent(parent) => format!("duplicate-parent:{parent}"),
            Self::DuplicateRootKind(kind) => format!("duplicate-root-kind:{}", kind.as_str()),
            Self::DuplicateRootRef(root) => format!("duplicate-root-ref:{root}"),
            Self::MissingRequiredRoot(kind) => format!("missing-required-root-kind:{}", kind.as_str()),
            Self::UnexpectedRoot(kind) => format!("unexpected-root-kind:{}", kind.as_str()),
            Self::CompletenessMismatch => "completeness-mismatch".to_string(),
            Self::RootDomainMismatch { expected, actual } => {
                format!("root-domain-mismatch:{}:{}", expected.as_str(), actual.as_str())
            }
            Self::RootSchemaNotValidated(kind) => format!("root-schema-not-validated:{}", kind.as_str()),
            Self::IncompleteInventory(kind) => format!("incomplete-inventory:{}", kind.as_str()),
            Self::FenceRootMismatch { root, fence } => {
                format!("fence-root-mismatch:{}:{}", root.as_str(), fence.as_str())
            }
            Self::DuplicateFenceSource(source_id) => format!("duplicate-fence-source:{source_id}"),
            Self::MissingRecheck(source_id) => format!("missing-recheck:{source_id}"),
            Self::UnexpectedRecheck(source_id) => format!("unexpected-recheck:{source_id}"),
            Self::DuplicateRecheck(source_id) => format!("duplicate-recheck:{source_id}"),
            Self::RevisionDrift {
                source_id,
                expected,
                actual,
            } => format!("revision-drift:{expected}:{actual}:{source_id}"),
            Self::RecheckInventoryIncomplete(source_id) => format!("recheck-inventory-incomplete:{source_id}"),
        }
    }
}
