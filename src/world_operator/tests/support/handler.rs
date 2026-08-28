use molten_core::world_operator::*;

use super::super::super::*;
use super::request::STALE_GENERATION;
use super::request::operation_kinds;
use super::request::reference;
use crate::error::Result;

pub(crate) const EXPECTED_PUBLISHED_RECORDS: usize = 4;

#[derive(Debug)]
pub(crate) struct FixtureHandler {
    pub(crate) kind: WorldOperationKind,
    pub(crate) owner: WorldComponentOwner,
    pub(crate) preview_calls: usize,
    pub(crate) execute_calls: usize,
    execute_state: WorldComponentCompletionState,
    pub(crate) claims_authority: bool,
    pub(crate) sensitive_material_present: bool,
}

impl FixtureHandler {
    pub(crate) fn new(kind: WorldOperationKind) -> Self {
        Self {
            kind,
            owner: kind.owner(),
            preview_calls: 0,
            execute_calls: 0,
            execute_state: WorldComponentCompletionState::Complete,
            claims_authority: false,
            sensitive_material_present: false,
        }
    }

    fn link(
        &self,
        operation: &WorldOperationPlanNode,
        role: WorldReceiptRole,
        state: WorldComponentCompletionState,
    ) -> WorldReceiptLink {
        WorldReceiptLink {
            operation_id: operation.operation_id.clone(),
            kind: operation.kind,
            owner: self.owner,
            role,
            component_ref: reference(&format!(
                "component:{}:{}:{}",
                operation.kind.as_str(),
                role.as_str(),
                state.as_str()
            )),
            state,
            sensitive_material_present: self.sensitive_material_present,
            claims_authority: self.claims_authority,
            claims_deletion_authority: false,
        }
    }
}

impl WorldOperationHandler for FixtureHandler {
    fn kind(&self) -> WorldOperationKind {
        self.kind
    }

    fn owner(&self) -> WorldComponentOwner {
        self.owner
    }

    fn preview(&mut self, _plan: &WorldWorkflowPlan, operation: &WorldOperationPlanNode) -> Result<WorldReceiptLink> {
        self.preview_calls += 1;
        Ok(self.link(operation, WorldReceiptRole::ComponentPlan, WorldComponentCompletionState::Planned))
    }

    fn execute(
        &mut self,
        _plan: &WorldWorkflowPlan,
        operation: &WorldOperationPlanNode,
        admission: Option<&WorldOperationApplyAdmission>,
    ) -> Result<WorldReceiptLink> {
        if operation.kind.is_mutating() {
            assert!(admission.is_some(), "mutating fixture operation requires admission");
        } else {
            assert!(admission.is_none(), "read-only fixture operation has no mutation admission");
        }
        self.execute_calls += 1;
        let role = if self.execute_state == WorldComponentCompletionState::Unknown {
            WorldReceiptRole::Reconciliation
        } else {
            WorldReceiptRole::ComponentReceipt
        };
        Ok(self.link(operation, role, self.execute_state))
    }
}

pub(crate) struct FixtureFacts {
    pub(crate) stale: bool,
    pub(crate) observations: usize,
}

impl WorldOperationCurrentFactsPort for FixtureFacts {
    fn observe_current(
        &mut self,
        plan: &WorldWorkflowPlan,
        operation: &WorldOperationPlanNode,
    ) -> Result<WorldOperationCurrentFacts> {
        self.observations += 1;
        Ok(WorldOperationCurrentFacts {
            plan_ref: plan.plan_ref.clone(),
            operation_id: operation.operation_id.clone(),
            observed_head: plan.expected_head.clone(),
            observed_generation: if self.stale {
                STALE_GENERATION
            } else {
                plan.expected_generation
            },
            policy_ref: plan.policy_ref.clone(),
            authority_observation_ref: plan.authority_observation_ref.clone(),
            profile_ref: operation.profile_ref.clone(),
            profile_status: WorldProfileStatus::Admitted,
        })
    }
}

#[derive(Default)]
pub(crate) struct RecordingPort {
    pub(crate) refs: Vec<String>,
}

impl WorldWorkflowRecordPort for RecordingPort {
    fn publish(&mut self, record: &CanonicalWorldOperatorRecord) -> Result<String> {
        self.refs.push(record.record_ref.clone());
        Ok(record.record_ref.clone())
    }
}

pub(crate) fn fixture_handlers(
    special: Option<(WorldOperationKind, WorldComponentCompletionState)>,
) -> Vec<FixtureHandler> {
    operation_kinds()
        .into_iter()
        .map(|kind| {
            let mut handler = FixtureHandler::new(kind);
            if let Some((special_kind, state)) = special
                && special_kind == kind
            {
                handler.execute_state = state;
            }
            handler
        })
        .collect()
}

pub(crate) fn handler_refs(handlers: &mut [FixtureHandler]) -> Vec<&mut dyn WorldOperationHandler> {
    handlers.iter_mut().map(|handler| handler as &mut dyn WorldOperationHandler).collect()
}
