mod binding;
mod canonical;
pub mod fastpath;
mod lifecycle;
mod live_service;
mod operation;
mod operator;
mod outcome;
mod planner;
pub mod raft;

pub use binding::ConsistencyGroupBinding;
pub use binding::ConsistencyGroupBindingInput;
pub use binding::canonical_consistency_group_binding;
pub use lifecycle::apply_consistency_outcome;
pub use live_service::plan_live_replica_start_for_host;
pub use operation::ConfigurationTransition;
pub use operation::ConsistencyOperation;
pub use operation::ConsistencyPlanDecision;
pub use operation::ConsistencyPortCommandInput;
pub use operation::ConsistencyPortPlan;
pub use operation::GroupOpenMode;
pub use operation::plan_consistency_operation;
pub use operator::*;
pub use outcome::ConsistencyOutcomeInput;
pub use outcome::ConsistencyOutcomeKind;
pub use outcome::ConsistencyPortOutcome;
pub use outcome::normalize_consistency_outcome;

pub const CONSISTENCY_GROUP_BINDING_SCHEMA: &str = "molten.fabric-consistency.group-binding.v1";
pub const CONSISTENCY_PORT_PLAN_SCHEMA: &str = "molten.fabric-consistency.operation-plan.v1";
pub const CONSISTENCY_PORT_OUTCOME_SCHEMA: &str = "molten.fabric-consistency.operation-outcome.v1";

pub const MAX_CONSISTENCY_POLICY_REFS: usize = 32;
pub const MAX_CONSISTENCY_AUTHORITY_REFS: usize = 16;
pub const MAX_CONSISTENCY_EVIDENCE_REFS: usize = 32;
pub const MAX_CONSISTENCY_NON_CLAIMS: usize = 16;
pub const MAX_CONSISTENCY_DIAGNOSTICS: usize = 16;
pub const MAX_CONSISTENCY_IDENTIFIER_BYTES: usize = 256;
pub const MAX_CONSISTENCY_NON_CLAIM_BYTES: usize = 512;
pub const MAX_CONSISTENCY_COMMAND_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_CONSISTENCY_IN_FLIGHT_OPERATIONS: u32 = 4_096;
pub const INITIAL_CONSISTENCY_EPOCH: u64 = 1;
pub const NEXT_CONSISTENCY_EPOCH_STEP: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyGroupLifecycle {
    Declared,
    Active,
    Draining,
    Removed,
}

impl ConsistencyGroupLifecycle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Removed => "removed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyReadMode {
    LocalStale,
    Linearizable,
    Lease,
}

impl ConsistencyReadMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalStale => "local-stale",
            Self::Linearizable => "linearizable",
            Self::Lease => "lease",
        }
    }
}

#[cfg(test)]
mod tests;
