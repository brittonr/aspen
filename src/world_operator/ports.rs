use molten_core::world_operator::*;

use super::CanonicalWorldOperatorRecord;
use crate::error::Result;

/// One narrow adapter for one component-owned world operation kind.
///
/// Implementations must delegate semantic decisions to the existing component
/// service. The workflow owns only ordering and receipt linkage.
pub trait WorldOperationHandler {
    fn kind(&self) -> WorldOperationKind;

    fn owner(&self) -> WorldComponentOwner;

    fn preview(&mut self, plan: &WorldWorkflowPlan, operation: &WorldOperationPlanNode) -> Result<WorldReceiptLink>;

    fn execute(
        &mut self,
        plan: &WorldWorkflowPlan,
        operation: &WorldOperationPlanNode,
        admission: Option<&WorldOperationApplyAdmission>,
    ) -> Result<WorldReceiptLink>;
}

/// Supplies a fresh mutable observation immediately before one mutation.
pub trait WorldOperationCurrentFactsPort {
    fn observe_current(
        &mut self,
        plan: &WorldWorkflowPlan,
        operation: &WorldOperationPlanNode,
    ) -> Result<WorldOperationCurrentFacts>;
}

/// Persists a canonical workflow record after the core validates its meaning.
pub trait WorldWorkflowRecordPort {
    fn publish(&mut self, record: &CanonicalWorldOperatorRecord) -> Result<String>;
}
