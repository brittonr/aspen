use preserves::IOValue;

use super::ConsistencyGroupLifecycle;
use super::ConsistencyReadMode;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupOpenMode {
    Create,
    Attach,
}

impl GroupOpenMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Attach => "attach",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationTransition {
    StaticMembershipRefresh {
        next_membership_ref: String,
        next_config_epoch: u64,
    },
    DynamicMembership {
        next_membership_ref: String,
        next_config_epoch: u64,
    },
    JointConsensus {
        next_membership_ref: String,
        next_config_epoch: u64,
    },
}

impl ConfigurationTransition {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::StaticMembershipRefresh { .. } => "static-membership-refresh",
            Self::DynamicMembership { .. } => "dynamic-membership",
            Self::JointConsensus { .. } => "joint-consensus",
        }
    }

    pub fn next_membership_ref(&self) -> &str {
        match self {
            Self::StaticMembershipRefresh {
                next_membership_ref, ..
            }
            | Self::DynamicMembership {
                next_membership_ref, ..
            }
            | Self::JointConsensus {
                next_membership_ref, ..
            } => next_membership_ref,
        }
    }

    pub const fn next_config_epoch(&self) -> u64 {
        match self {
            Self::StaticMembershipRefresh { next_config_epoch, .. }
            | Self::DynamicMembership { next_config_epoch, .. }
            | Self::JointConsensus { next_config_epoch, .. } => *next_config_epoch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyOperation {
    Open {
        mode: GroupOpenMode,
    },
    Propose {
        command_ref: String,
        command_schema_ref: String,
        estimated_command_bytes: u64,
    },
    Read {
        query_ref: String,
        mode: ConsistencyReadMode,
    },
    Snapshot {
        snapshot_policy_ref: String,
    },
    Recover {
        snapshot_ref: String,
        durable_boundary_ref: String,
    },
    Configure {
        transition: ConfigurationTransition,
    },
    Health,
    Drain,
    Status,
    Remove,
}

impl ConsistencyOperation {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Open { .. } => "open",
            Self::Propose { .. } => "propose",
            Self::Read { .. } => "read",
            Self::Snapshot { .. } => "snapshot",
            Self::Recover { .. } => "recover",
            Self::Configure { .. } => "configure",
            Self::Health => "health",
            Self::Drain => "drain",
            Self::Status => "status",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyPortCommandInput {
    pub request_ref: String,
    pub binding_ref: String,
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub application_manifest_ref: String,
    pub engine_algorithm_profile: String,
    pub engine_implementation_profile: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub resource_profile_ref: String,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub observed_in_flight_operations: u32,
    pub operation: ConsistencyOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyPlanDecision {
    Admitted,
    Denied,
}

impl ConsistencyPlanDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyPortPlan {
    pub plan_ref: String,
    pub request_ref: String,
    pub binding_ref: String,
    pub operation: ConsistencyOperation,
    pub decision: ConsistencyPlanDecision,
    pub lifecycle_before: ConsistencyGroupLifecycle,
    pub lifecycle_after: ConsistencyGroupLifecycle,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

impl ConsistencyPortPlan {
    pub fn admitted(&self) -> bool {
        self.decision == ConsistencyPlanDecision::Admitted
    }
}

pub fn plan_consistency_operation(
    binding: &super::ConsistencyGroupBinding,
    input: ConsistencyPortCommandInput,
) -> Result<ConsistencyPortPlan> {
    super::planner::plan_consistency_operation(binding, input)
}
