use super::super::CaptureIssue;
use super::super::RootKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureIssue {
    InvalidCore(CaptureIssue),
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateRootObservation(RootKind),
    MissingRootObject(RootKind),
    RootIdentityMismatch(RootKind),
    RootSchemaMismatch(RootKind),
    UnexpectedRootObservation(RootKind),
    DuplicateParentObservation(String),
    ParentEdgeBoundExceeded {
        commit_ref: String,
        actual: usize,
        maximum: usize,
    },
    DuplicateParentEdge {
        commit_ref: String,
        parent_ref: String,
    },
    MissingParentObject(String),
    MissingParentObservation(String),
    UnreachableParentObservation(String),
    ParentCycle(String),
}

impl ClosureIssue {
    /// The stable receipt code for this issue, following [`CaptureIssue::code`]: a closed
    /// kebab-case tag, then `:`-separated typed fields with at most one trailing free-form
    /// string. The edge variant carries two free-form refs, so its first ref is length-prefixed
    /// to keep every code distinct.
    // r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    pub fn code(&self) -> String {
        match self {
            Self::InvalidCore(issue) => format!("invalid-core:{}", issue.code()),
            Self::BoundExceeded { field, actual, maximum } => format!("bound-exceeded:{field}:{actual}:{maximum}"),
            Self::DuplicateRootObservation(kind) => format!("duplicate-root-observation:{}", kind.as_str()),
            Self::MissingRootObject(kind) => format!("missing-root-object:{}", kind.as_str()),
            Self::RootIdentityMismatch(kind) => format!("root-identity-mismatch:{}", kind.as_str()),
            Self::RootSchemaMismatch(kind) => format!("root-schema-mismatch:{}", kind.as_str()),
            Self::UnexpectedRootObservation(kind) => format!("unexpected-root-observation:{}", kind.as_str()),
            Self::DuplicateParentObservation(commit_ref) => format!("duplicate-parent-observation:{commit_ref}"),
            Self::ParentEdgeBoundExceeded {
                commit_ref,
                actual,
                maximum,
            } => format!("parent-edge-bound-exceeded:{actual}:{maximum}:{commit_ref}"),
            Self::DuplicateParentEdge { commit_ref, parent_ref } => {
                format!("duplicate-parent-edge:{}:{commit_ref}:{parent_ref}", commit_ref.len())
            }
            Self::MissingParentObject(commit_ref) => format!("missing-parent-object:{commit_ref}"),
            Self::MissingParentObservation(commit_ref) => format!("missing-parent-observation:{commit_ref}"),
            Self::UnreachableParentObservation(commit_ref) => format!("unreachable-parent-observation:{commit_ref}"),
            Self::ParentCycle(commit_ref) => format!("parent-cycle:{commit_ref}"),
        }
    }
}
