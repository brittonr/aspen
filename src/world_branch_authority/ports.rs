use molten_core::world_branch_authority::CurrentAuthorityFacts;
use molten_core::world_branch_authority::WorldBranchActivationDecision;
use molten_core::world_branch_authority::WorldBranchAuthorityFacts;
use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchRealizationObservation;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentPolicyObservation {
    pub policy_json: String,
    pub policy_ref: String,
    pub generation: u64,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearOwnershipObservation {
    pub capability_ref: String,
    pub generation: u64,
    pub source_active: bool,
    pub destination_active: bool,
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinearTransferOutcome {
    Committed(Box<WorldBranchRealizationObservation>),
    Denied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationOutcome {
    Activated,
    Denied,
    Unknown,
}

impl ActivationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Activated => "activated",
            Self::Denied => "denied",
            Self::Unknown => "unknown",
        }
    }
}

pub trait CurrentBranchPolicyPort {
    fn observe_policy(&mut self) -> Result<CurrentPolicyObservation>;
}

pub trait CurrentBranchAuthorityPort {
    fn observe_authority(&mut self, facts: &WorldBranchAuthorityFacts) -> Result<CurrentAuthorityFacts>;
}

pub trait DestinationGrantPort {
    fn realize_destination_grant(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
    ) -> Result<WorldBranchRealizationObservation>;
}

pub trait LinearAuthorityTransferPort {
    fn observe_ownership(&mut self, plan: &WorldBranchAuthorityPlan) -> Result<LinearOwnershipObservation>;

    fn transfer(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
        expected_generation: u64,
        operation_ref: &str,
    ) -> Result<LinearTransferOutcome>;

    fn reconcile_transfer(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
        operation_ref: &str,
    ) -> Result<Option<WorldBranchRealizationObservation>>;
}

pub trait SimulationAuthorityPort {
    fn bind_simulation(&mut self, plan: &WorldBranchAuthorityPlan) -> Result<WorldBranchRealizationObservation>;
}

pub trait BranchActivationPort {
    fn activate(&mut self, decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome>;

    fn reconcile_activation(&mut self, decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome>;
}

pub trait BranchAuthorityReceiptPort {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_json: &[u8]) -> Result<()>;
}

pub trait WorldBranchAuthorityRuntime:
    CurrentBranchPolicyPort
    + CurrentBranchAuthorityPort
    + DestinationGrantPort
    + LinearAuthorityTransferPort
    + SimulationAuthorityPort
    + BranchActivationPort
    + BranchAuthorityReceiptPort
{
}

impl<T> WorldBranchAuthorityRuntime for T where T: CurrentBranchPolicyPort
        + CurrentBranchAuthorityPort
        + DestinationGrantPort
        + LinearAuthorityTransferPort
        + SimulationAuthorityPort
        + BranchActivationPort
        + BranchAuthorityReceiptPort
{
}
