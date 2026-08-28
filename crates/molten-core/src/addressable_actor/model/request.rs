use serde::Deserialize;
use serde::Serialize;

use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorAdmissionFacts {
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub authority_ref: String,
    pub resource_ref: String,
    pub adapter_ref: String,
    pub policy_current: bool,
    pub capability_current: bool,
    pub placement_current: bool,
    pub generation_current: bool,
    pub resources_admitted: bool,
    pub adapter_admitted: bool,
}

impl ActorAdmissionFacts {
    pub const fn all_current(&self) -> bool {
        self.policy_current
            && self.capability_current
            && self.placement_current
            && self.generation_current
            && self.resources_admitted
            && self.adapter_admitted
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WakeReason {
    Message {
        delivery_item_ref: String,
        delivery_token_ref: String,
    },
    Timer {
        timer_ref: String,
    },
    Connection {
        connection_ref: String,
    },
    Operator {
        operator_request_ref: String,
    },
}

impl WakeReason {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Message { .. } => "message",
            Self::Timer { .. } => "timer",
            Self::Connection { .. } => "connection",
            Self::Operator { .. } => "operator",
        }
    }

    pub fn subject_ref(&self) -> &str {
        match self {
            Self::Message { delivery_item_ref, .. } => delivery_item_ref,
            Self::Timer { timer_ref } => timer_ref,
            Self::Connection { connection_ref } => connection_ref,
            Self::Operator { operator_request_ref } => operator_request_ref,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ActorOperation {
    Wake {
        reason: WakeReason,
    },
    StartSucceeded {
        wake_ref: String,
    },
    IdleSleep {
        checkpoint_ref: String,
        pending_mailbox_items: u32,
        unresolved_effects: u32,
    },
    BeginDrain,
    DrainSucceeded {
        checkpoint_ref: String,
        remaining_items: u32,
    },
    Stop,
    Degrade {
        failure_ref: String,
    },
    BeginRecovery {
        checkpoint_ref: String,
    },
    RecoverySucceeded {
        checkpoint_ref: String,
        restored_classes: Vec<SurvivalClass>,
        durable_state_ref: String,
    },
    RecoveryFailed {
        failure_ref: String,
    },
    CompleteDelivery {
        delivery_item_ref: String,
        delivery_token_ref: String,
        semantic_event_ref: String,
        semantic_commit_ref: String,
    },
    RecordUnknownEffect {
        effect_ref: String,
    },
    ResolveUnknownEffect {
        effect_ref: String,
        resolution_ref: String,
        checkpoint_ref: String,
    },
}

impl ActorOperation {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Wake { .. } => "wake",
            Self::StartSucceeded { .. } => "start-succeeded",
            Self::IdleSleep { .. } => "idle-sleep",
            Self::BeginDrain => "begin-drain",
            Self::DrainSucceeded { .. } => "drain-succeeded",
            Self::Stop => "stop",
            Self::Degrade { .. } => "degrade",
            Self::BeginRecovery { .. } => "begin-recovery",
            Self::RecoverySucceeded { .. } => "recovery-succeeded",
            Self::RecoveryFailed { .. } => "recovery-failed",
            Self::CompleteDelivery { .. } => "complete-delivery",
            Self::RecordUnknownEffect { .. } => "record-unknown-effect",
            Self::ResolveUnknownEffect { .. } => "resolve-unknown-effect",
        }
    }

    pub fn primary_ref(&self) -> Option<&str> {
        match self {
            Self::Wake { reason } => Some(reason.subject_ref()),
            Self::StartSucceeded { wake_ref } => Some(wake_ref),
            Self::IdleSleep { checkpoint_ref, .. }
            | Self::DrainSucceeded { checkpoint_ref, .. }
            | Self::BeginRecovery { checkpoint_ref }
            | Self::RecoverySucceeded { checkpoint_ref, .. } => Some(checkpoint_ref),
            Self::BeginDrain | Self::Stop => None,
            Self::Degrade { failure_ref } | Self::RecoveryFailed { failure_ref } => Some(failure_ref),
            Self::CompleteDelivery { delivery_item_ref, .. } => Some(delivery_item_ref),
            Self::RecordUnknownEffect { effect_ref } | Self::ResolveUnknownEffect { effect_ref, .. } => {
                Some(effect_ref)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorRequest {
    pub schema: String,
    pub operation_id: String,
    pub actor_key_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub expected_lifecycle_sequence: u64,
    pub logical_tick: u64,
    pub admission: ActorAdmissionFacts,
    pub operation: ActorOperation,
}
