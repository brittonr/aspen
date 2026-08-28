use super::WorldComponentOwner;
use super::WorldOperationKind;
use super::WorldWorkflowBlocker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReceiptRole {
    ComponentPlan,
    ComponentReceipt,
    Observation,
    Divergence,
    Reconciliation,
}

impl WorldReceiptRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ComponentPlan => "component-plan",
            Self::ComponentReceipt => "component-receipt",
            Self::Observation => "observation",
            Self::Divergence => "divergence",
            Self::Reconciliation => "reconciliation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldComponentCompletionState {
    Planned,
    Complete,
    Blocked,
    Unknown,
}

impl WorldComponentCompletionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Complete => "complete",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReceiptLink {
    pub operation_id: String,
    pub kind: WorldOperationKind,
    pub owner: WorldComponentOwner,
    pub role: WorldReceiptRole,
    pub component_ref: String,
    pub state: WorldComponentCompletionState,
    pub sensitive_material_present: bool,
    pub claims_authority: bool,
    pub claims_deletion_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldWorkflowCompletionState {
    Planned,
    Partial,
    Complete,
    Blocked,
    Unknown,
}

impl WorldWorkflowCompletionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Partial => "partial",
            Self::Complete => "complete",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowReceipt {
    pub schema: &'static str,
    pub receipt_ref: String,
    pub plan_ref: String,
    pub links: Vec<WorldReceiptLink>,
    pub completion: WorldWorkflowCompletionState,
    pub first_blocker: Option<WorldWorkflowBlocker>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowSummary {
    pub schema: &'static str,
    pub summary_ref: String,
    pub plan_ref: String,
    pub completion: WorldWorkflowCompletionState,
    pub operation_count: usize,
    pub linked_receipt_count: usize,
    pub first_blocker: Option<WorldWorkflowBlocker>,
    pub non_claims: Vec<String>,
}
