mod binding;
mod canonical;
pub mod chaoscontrol;
pub mod fastpath;
mod lifecycle;
#[path = "live/service.rs"]
mod live_service;
mod operation;
mod operator;
mod outcome;
mod planner;
pub mod raft;

pub use binding::Group;
pub use binding::GroupInput;
pub use binding::canonical_group;
pub use lifecycle::apply_outcome;
pub use live_service::plan_replica_start_for_host;
pub use operation::ConfigurationTransition;
pub use operation::GroupOpenMode;
pub use operation::PlanDecision;
pub use operation::PortCommandInput;
pub use operation::PortPlan;
pub use operation::Request;
pub use operation::plan;
pub use operator::*;
pub use outcome::Input;
pub use outcome::Kind;
pub use outcome::Port;
pub use outcome::normalize;

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
pub enum GroupLifecycle {
    Declared,
    Active,
    Draining,
    Removed,
}

impl GroupLifecycle {
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
pub enum ReadMode {
    LocalStale,
    Linearizable,
    Lease,
}

impl ReadMode {
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
