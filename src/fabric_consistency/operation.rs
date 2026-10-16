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
pub enum Request {
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
        mode: super::ReadMode,
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

impl Request {
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
pub struct PortCommandInput {
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
    pub operation: Request,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanDecision {
    Admitted,
    Denied,
}

impl PlanDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortPlan {
    pub plan_ref: String,
    pub request_ref: String,
    pub binding_ref: String,
    pub operation: Request,
    pub decision: PlanDecision,
    pub lifecycle_before: super::GroupLifecycle,
    pub lifecycle_after: super::GroupLifecycle,
    pub diagnostics: Vec<String>,
    pub value: preserves::IOValue,
}

impl PortPlan {
    pub fn admitted(&self) -> bool {
        self.decision == PlanDecision::Admitted
    }
}

pub fn plan(binding: &super::Group, input: PortCommandInput) -> crate::error::Result<PortPlan> {
    super::planner::plan_operation(binding, input)
}
